use crate::error::{ApiError, ApiResult};
use crate::models::casebuilder::*;
use crate::services::neo4j::Neo4jService;
use crate::services::object_store::{
    build_document_object_key, clean_etag, normalize_sha256, ObjectStore, PutOptions, StoredObject,
};
use bytes::Bytes;
use flate2::read::DeflateDecoder;
use neo4rs::query;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;

#[derive(Clone, Copy)]
struct AstStoragePolicy {
    entity_inline_bytes: u64,
    snapshot_inline_bytes: u64,
    block_inline_bytes: u64,
}

#[derive(Clone)]
pub struct CaseBuilderService {
    neo4j: Arc<Neo4jService>,
    object_store: Arc<dyn ObjectStore>,
    upload_ttl_seconds: u64,
    download_ttl_seconds: u64,
    max_upload_bytes: u64,
    ast_storage_policy: AstStoragePolicy,
}

#[derive(Clone)]
pub struct BinaryUploadRequest {
    pub filename: String,
    pub mime_type: Option<String>,
    pub bytes: Bytes,
    pub document_type: Option<String>,
    pub folder: Option<String>,
    pub confidentiality: Option<String>,
}

#[derive(Clone, Copy)]
struct NodeSpec {
    label: &'static str,
    id_key: &'static str,
    edge: &'static str,
}

#[derive(Clone)]
struct DocumentProvenance {
    object_blob: ObjectBlob,
    document_version: DocumentVersion,
    ingestion_run: IngestionRun,
}

#[derive(Clone)]
struct SourceContext {
    document_version_id: Option<String>,
    object_blob_id: Option<String>,
    ingestion_run_id: Option<String>,
}

#[derive(Clone)]
struct SentenceCandidate {
    text: String,
    byte_start: u64,
    byte_end: u64,
    char_start: u64,
    char_end: u64,
}

#[derive(Clone)]
struct VersionChangeInput {
    target_type: String,
    target_id: String,
    operation: String,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    summary: String,
    legal_impact: LegalImpactSummary,
    ai_audit_id: Option<String>,
}

struct WorkProductHashes {
    document_hash: String,
    support_graph_hash: String,
    qc_state_hash: String,
    formatting_hash: String,
}

struct ComparableLayerItem {
    layer: &'static str,
    target_type: String,
    target_id: String,
    title: String,
    summary: String,
    value: serde_json::Value,
}

const PARSER_REGISTRY_VERSION: &str = "casebuilder-parser-registry-v1";
const CHUNKER_VERSION: &str = "casebuilder-line-chunker-v1";
const CITATION_RESOLVER_VERSION: &str = "casebuilder-citation-resolver-v1";
const CASE_INDEX_VERSION: &str = "casebuilder-case-graph-index-v1";
const ORS_2025_SOURCE_URL: &str = "https://www.oregonlegislature.gov/bills_laws/pages/ors.aspx";
const ORCP_2025_SOURCE_URL: &str = "https://www.oregonlegislature.gov/bills_laws/Pages/ORCP.aspx";
const UTCR_CURRENT_SOURCE_URL: &str = "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf";

static PLEADING_PARAGRAPH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^\s*(?:\*\*)?(?:¶\s*)?([0-9]+[A-Za-z]?)\.\s*(?:\*\*)?\s*(.+?)\s*$"#).unwrap()
});
static CLAIM_HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bCLAIM\s+FOR\s+RELIEF\b").unwrap());
static ORS_CITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\bORS\s+(?:chapters?\s+)?[0-9]{1,3}[A-Z]?(?:\.[0-9]{3,4})?(?:\s*(?:to|through)\s*[0-9]{1,3}[A-Z]?(?:\.[0-9]{3,4})?)?(?:\([^)]+\))*",
    )
    .unwrap()
});
static ORCP_CITATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bORCP\s+[0-9]+[A-Z]?(?:\s*[A-Z])?(?:\([^)]+\))*").unwrap());
static UTCR_CITATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bUTCR\s+[0-9]+(?:\.[0-9]+)?(?:\([^)]+\))*").unwrap());
static SESSION_LAW_CITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:Or(?:egon)?\.?\s+Laws?|Oregon Laws)\s+([0-9]{4}),?\s+ch(?:apter)?\.?\s*([0-9]+)(?:,?\s*(?:§|sec(?:tion)?\.?)\s*([0-9A-Za-z.-]+))?",
    )
    .unwrap()
});
static EXHIBIT_LABEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:PX|EXHIBIT)\s*[- ]?([A-Z]{1,3}|[0-9]{1,4})\b").unwrap());

#[derive(Clone)]
struct ParserOutcome {
    parser_id: String,
    status: String,
    message: String,
    text: Option<String>,
}

#[derive(Clone)]
struct ParsedComplaintParagraph {
    original_label: String,
    text: String,
    section_key: String,
    count_key: Option<String>,
    byte_start: u64,
    byte_end: u64,
    char_start: u64,
    char_end: u64,
}

#[derive(Default)]
struct ParsedComplaintStructure {
    sections: Vec<(String, String, String)>,
    counts: Vec<(String, String)>,
    paragraphs: Vec<ParsedComplaintParagraph>,
}

impl CaseBuilderService {
    pub fn new(
        neo4j: Arc<Neo4jService>,
        object_store: Arc<dyn ObjectStore>,
        upload_ttl_seconds: u64,
        download_ttl_seconds: u64,
        max_upload_bytes: u64,
        ast_entity_inline_bytes: u64,
        ast_snapshot_inline_bytes: u64,
        ast_block_inline_bytes: u64,
    ) -> Self {
        Self {
            neo4j,
            object_store,
            upload_ttl_seconds,
            download_ttl_seconds,
            max_upload_bytes,
            ast_storage_policy: AstStoragePolicy {
                entity_inline_bytes: ast_entity_inline_bytes,
                snapshot_inline_bytes: ast_snapshot_inline_bytes,
                block_inline_bytes: ast_block_inline_bytes,
            },
        }
    }

    pub async fn ensure_indexes(&self) -> ApiResult<()> {
        let statements = [
            "CREATE CONSTRAINT casebuilder_matter_id IF NOT EXISTS FOR (n:Matter) REQUIRE n.matter_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_party_id IF NOT EXISTS FOR (n:Party) REQUIRE n.party_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_document_id IF NOT EXISTS FOR (n:CaseDocument) REQUIRE n.document_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_fact_id IF NOT EXISTS FOR (n:Fact) REQUIRE n.fact_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_timeline_event_id IF NOT EXISTS FOR (n:TimelineEvent) REQUIRE n.event_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_evidence_id IF NOT EXISTS FOR (n:Evidence) REQUIRE n.evidence_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_claim_id IF NOT EXISTS FOR (n:Claim) REQUIRE n.claim_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_defense_id IF NOT EXISTS FOR (n:Defense) REQUIRE n.defense_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_element_id IF NOT EXISTS FOR (n:Element) REQUIRE n.element_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_draft_id IF NOT EXISTS FOR (n:Draft) REQUIRE n.draft_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_draft_paragraph_id IF NOT EXISTS FOR (n:DraftParagraph) REQUIRE n.paragraph_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_deadline_instance_id IF NOT EXISTS FOR (n:DeadlineInstance) REQUIRE n.deadline_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_task_id IF NOT EXISTS FOR (n:Task) REQUIRE n.task_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_fact_check_finding_id IF NOT EXISTS FOR (n:FactCheckFinding) REQUIRE n.finding_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_citation_check_finding_id IF NOT EXISTS FOR (n:CitationCheckFinding) REQUIRE n.finding_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_object_blob_id IF NOT EXISTS FOR (n:ObjectBlob) REQUIRE n.object_blob_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_document_version_id IF NOT EXISTS FOR (n:DocumentVersion) REQUIRE n.document_version_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_ingestion_run_id IF NOT EXISTS FOR (n:IngestionRun) REQUIRE n.ingestion_run_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_source_span_id IF NOT EXISTS FOR (n:SourceSpan) REQUIRE n.source_span_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_complaint_id IF NOT EXISTS FOR (n:ComplaintDraft) REQUIRE n.complaint_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_complaint_section_id IF NOT EXISTS FOR (n:ComplaintSection) REQUIRE n.section_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_complaint_count_id IF NOT EXISTS FOR (n:ComplaintCount) REQUIRE n.count_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_pleading_paragraph_id IF NOT EXISTS FOR (n:PleadingParagraph) REQUIRE n.paragraph_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_pleading_sentence_id IF NOT EXISTS FOR (n:PleadingSentence) REQUIRE n.sentence_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_citation_use_id IF NOT EXISTS FOR (n:CitationUse) REQUIRE n.citation_use_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_evidence_use_id IF NOT EXISTS FOR (n:EvidenceUse) REQUIRE n.evidence_use_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_exhibit_reference_id IF NOT EXISTS FOR (n:ExhibitReference) REQUIRE n.exhibit_reference_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_relief_request_id IF NOT EXISTS FOR (n:ReliefRequest) REQUIRE n.relief_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_rule_check_finding_id IF NOT EXISTS FOR (n:RuleCheckFinding) REQUIRE n.finding_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_export_artifact_id IF NOT EXISTS FOR (n:ExportArtifact) REQUIRE n.artifact_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_complaint_history_event_id IF NOT EXISTS FOR (n:ComplaintHistoryEvent) REQUIRE n.event_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_external_authority_id IF NOT EXISTS FOR (n:ExternalAuthority) REQUIRE n.canonical_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_work_product_id IF NOT EXISTS FOR (n:WorkProduct) REQUIRE n.work_product_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_work_product_block_id IF NOT EXISTS FOR (n:WorkProductBlock) REQUIRE n.block_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_work_product_mark_id IF NOT EXISTS FOR (n:WorkProductMark) REQUIRE n.mark_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_work_product_anchor_id IF NOT EXISTS FOR (n:WorkProductAnchor) REQUIRE n.anchor_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_work_product_finding_id IF NOT EXISTS FOR (n:WorkProductFinding) REQUIRE n.finding_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_work_product_artifact_id IF NOT EXISTS FOR (n:WorkProductArtifact) REQUIRE n.artifact_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_work_product_history_event_id IF NOT EXISTS FOR (n:WorkProductHistoryEvent) REQUIRE n.event_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_change_set_id IF NOT EXISTS FOR (n:ChangeSet) REQUIRE n.change_set_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_version_snapshot_id IF NOT EXISTS FOR (n:VersionSnapshot) REQUIRE n.snapshot_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_snapshot_manifest_id IF NOT EXISTS FOR (n:SnapshotManifest) REQUIRE n.manifest_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_snapshot_entity_state_id IF NOT EXISTS FOR (n:SnapshotEntityState) REQUIRE n.entity_state_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_version_change_id IF NOT EXISTS FOR (n:VersionChange) REQUIRE n.change_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_version_branch_id IF NOT EXISTS FOR (n:VersionBranch) REQUIRE n.branch_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_legal_support_use_id IF NOT EXISTS FOR (n:LegalSupportUse) REQUIRE n.support_use_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_fact_use_id IF NOT EXISTS FOR (n:FactUse) REQUIRE n.support_use_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_authority_use_id IF NOT EXISTS FOR (n:AuthorityUse) REQUIRE n.support_use_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_element_support_id IF NOT EXISTS FOR (n:ElementSupport) REQUIRE n.support_use_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_ai_edit_audit_id IF NOT EXISTS FOR (n:AIEditAudit) REQUIRE n.ai_audit_id IS UNIQUE",
            "CREATE CONSTRAINT casebuilder_milestone_id IF NOT EXISTS FOR (n:Milestone) REQUIRE n.milestone_id IS UNIQUE",
            "CREATE INDEX casebuilder_document_matter IF NOT EXISTS FOR (n:CaseDocument) ON (n.matter_id)",
            "CREATE INDEX casebuilder_fact_matter IF NOT EXISTS FOR (n:Fact) ON (n.matter_id)",
            "CREATE INDEX casebuilder_claim_matter IF NOT EXISTS FOR (n:Claim) ON (n.matter_id)",
            "CREATE INDEX casebuilder_draft_matter IF NOT EXISTS FOR (n:Draft) ON (n.matter_id)",
            "CREATE INDEX casebuilder_document_version_matter IF NOT EXISTS FOR (n:DocumentVersion) ON (n.matter_id)",
            "CREATE INDEX casebuilder_ingestion_run_matter IF NOT EXISTS FOR (n:IngestionRun) ON (n.matter_id)",
            "CREATE INDEX casebuilder_source_span_matter IF NOT EXISTS FOR (n:SourceSpan) ON (n.matter_id)",
            "CREATE INDEX casebuilder_complaint_matter IF NOT EXISTS FOR (n:ComplaintDraft) ON (n.matter_id)",
            "CREATE INDEX casebuilder_complaint_status IF NOT EXISTS FOR (n:ComplaintDraft) ON (n.status)",
            "CREATE INDEX casebuilder_pleading_paragraph_complaint IF NOT EXISTS FOR (n:PleadingParagraph) ON (n.complaint_id)",
            "CREATE INDEX casebuilder_rule_check_finding_complaint IF NOT EXISTS FOR (n:RuleCheckFinding) ON (n.complaint_id, n.status)",
            "CREATE INDEX casebuilder_export_artifact_complaint IF NOT EXISTS FOR (n:ExportArtifact) ON (n.complaint_id, n.status)",
            "CREATE INDEX casebuilder_work_product_matter IF NOT EXISTS FOR (n:WorkProduct) ON (n.matter_id)",
            "CREATE INDEX casebuilder_work_product_type IF NOT EXISTS FOR (n:WorkProduct) ON (n.product_type, n.status)",
            "CREATE INDEX casebuilder_work_product_block_parent IF NOT EXISTS FOR (n:WorkProductBlock) ON (n.work_product_id, n.role)",
            "CREATE INDEX casebuilder_work_product_finding_status IF NOT EXISTS FOR (n:WorkProductFinding) ON (n.work_product_id, n.status)",
            "CREATE INDEX casebuilder_work_product_artifact_status IF NOT EXISTS FOR (n:WorkProductArtifact) ON (n.work_product_id, n.status)",
            "CREATE INDEX casebuilder_change_set_subject IF NOT EXISTS FOR (n:ChangeSet) ON (n.subject_id, n.created_at)",
            "CREATE INDEX casebuilder_version_snapshot_subject IF NOT EXISTS FOR (n:VersionSnapshot) ON (n.subject_id, n.sequence_number)",
            "CREATE INDEX casebuilder_version_branch_subject IF NOT EXISTS FOR (n:VersionBranch) ON (n.subject_id, n.branch_type)",
            "CREATE INDEX casebuilder_version_change_subject IF NOT EXISTS FOR (n:VersionChange) ON (n.subject_id, n.target_type)",
            "CREATE INDEX casebuilder_legal_support_use_subject IF NOT EXISTS FOR (n:LegalSupportUse) ON (n.subject_id, n.target_id)",
            "CREATE INDEX casebuilder_ai_edit_audit_subject IF NOT EXISTS FOR (n:AIEditAudit) ON (n.subject_id, n.created_at)",
            "CREATE FULLTEXT INDEX casebuilder_fact_fulltext IF NOT EXISTS FOR (n:Fact) ON EACH [n.text, n.statement]",
            "CREATE FULLTEXT INDEX casebuilder_document_fulltext IF NOT EXISTS FOR (n:CaseDocument) ON EACH [n.filename, n.title, n.summary, n.extracted_text]",
            "CREATE FULLTEXT INDEX casebuilder_complaint_fulltext IF NOT EXISTS FOR (n:ComplaintDraft|PleadingParagraph|ComplaintCount) ON EACH [n.title, n.text, n.legal_theory]",
            "CREATE FULLTEXT INDEX casebuilder_work_product_fulltext IF NOT EXISTS FOR (n:WorkProduct|WorkProductBlock) ON EACH [n.title, n.text, n.product_type]",
        ];

        for statement in statements {
            self.neo4j.run_rows(query(statement)).await?;
        }

        Ok(())
    }

    pub async fn list_matters(&self) -> ApiResult<Vec<MatterSummary>> {
        let rows = self
            .neo4j
            .run_rows(query(
                "MATCH (m:Matter)
                 OPTIONAL MATCH (m)-[:HAS_DOCUMENT]->(doc:CaseDocument)
                 WITH m, count(DISTINCT doc) AS document_count
                 OPTIONAL MATCH (m)-[:HAS_FACT]->(fact:Fact)
                 WITH m, document_count, count(DISTINCT fact) AS fact_count
                 OPTIONAL MATCH (m)-[:HAS_EVIDENCE]->(evidence:Evidence)
                 WITH m, document_count, fact_count, count(DISTINCT evidence) AS evidence_count
                 OPTIONAL MATCH (m)-[:HAS_CLAIM]->(claim:Claim)
                 WITH m, document_count, fact_count, evidence_count, count(DISTINCT claim) AS claim_count
                 OPTIONAL MATCH (m)-[:HAS_DRAFT]->(draft:Draft)
                 WITH m, document_count, fact_count, evidence_count, claim_count, count(DISTINCT draft) AS draft_count
                 OPTIONAL MATCH (m)-[:HAS_TASK]->(task:Task)
                 WITH m, document_count, fact_count, evidence_count, claim_count, draft_count,
                      count(DISTINCT CASE WHEN task.status <> 'done' THEN task END) AS open_task_count
                 RETURN m.payload AS payload,
                        document_count, fact_count, evidence_count, claim_count, draft_count, open_task_count
                 ORDER BY m.updated_at DESC",
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                let payload = row
                    .get::<String>("payload")
                    .map_err(|error| ApiError::Internal(error.to_string()))?;
                let mut matter = from_payload::<MatterSummary>(&payload)?;
                matter.document_count = row_u64(&row, "document_count");
                matter.fact_count = row_u64(&row, "fact_count");
                matter.evidence_count = row_u64(&row, "evidence_count");
                matter.claim_count = row_u64(&row, "claim_count");
                matter.draft_count = row_u64(&row, "draft_count");
                matter.open_task_count = row_u64(&row, "open_task_count");
                Ok(matter)
            })
            .collect()
    }

    pub async fn create_matter(&self, request: CreateMatterRequest) -> ApiResult<MatterBundle> {
        let now = now_string();
        let matter_id = generate_id("matter", &request.name);
        let matter = MatterSummary {
            matter_id: matter_id.clone(),
            short_name: Some(short_name(&request.name)),
            name: request.name,
            matter_type: request.matter_type.unwrap_or_else(|| "civil".to_string()),
            status: "intake".to_string(),
            user_role: request.user_role.unwrap_or_else(|| "neutral".to_string()),
            jurisdiction: request.jurisdiction.unwrap_or_else(|| "Oregon".to_string()),
            court: request.court.unwrap_or_else(|| "Unassigned".to_string()),
            case_number: request.case_number,
            created_at: now.clone(),
            updated_at: now,
            document_count: 0,
            fact_count: 0,
            evidence_count: 0,
            claim_count: 0,
            draft_count: 0,
            open_task_count: 0,
            next_deadline: None,
        };

        self.merge_matter(&matter).await?;
        self.get_matter(&matter_id).await
    }

    pub async fn get_matter(&self, matter_id: &str) -> ApiResult<MatterBundle> {
        let summary = self.get_matter_summary(matter_id).await?;
        Ok(MatterBundle {
            id: summary.matter_id.clone(),
            title: summary.name.clone(),
            documents: self.list_documents(matter_id).await?,
            parties: self.list_parties(matter_id).await?,
            facts: self.list_facts(matter_id).await?,
            timeline: self.list_timeline(matter_id).await?,
            claims: self.list_claims(matter_id).await?,
            evidence: self.list_evidence(matter_id).await?,
            defenses: self.list_defenses(matter_id).await?,
            deadlines: self.list_deadlines(matter_id).await?,
            tasks: self.list_tasks(matter_id).await?,
            drafts: self.list_drafts(matter_id).await?,
            work_products: self.list_work_products(matter_id).await?,
            fact_check_findings: self.list_fact_check_findings(matter_id, None).await?,
            citation_check_findings: self.list_citation_check_findings(matter_id, None).await?,
            summary,
        })
    }

    pub async fn patch_matter(
        &self,
        matter_id: &str,
        request: PatchMatterRequest,
    ) -> ApiResult<MatterBundle> {
        let mut matter = self.get_matter_summary(matter_id).await?;
        if let Some(value) = request.name {
            matter.name = value;
            matter.short_name = Some(short_name(&matter.name));
        }
        if let Some(value) = request.matter_type {
            matter.matter_type = value;
        }
        if let Some(value) = request.status {
            matter.status = value;
        }
        if let Some(value) = request.user_role {
            matter.user_role = value;
        }
        if let Some(value) = request.jurisdiction {
            matter.jurisdiction = value;
        }
        if let Some(value) = request.court {
            matter.court = value;
        }
        if request.case_number.is_some() {
            matter.case_number = request.case_number;
        }
        matter.updated_at = now_string();
        self.merge_matter(&matter).await?;
        self.get_matter(matter_id).await
    }

    pub async fn delete_matter(&self, matter_id: &str) -> ApiResult<()> {
        self.require_matter(matter_id).await?;
        for document in self.list_documents(matter_id).await.unwrap_or_default() {
            if let Some(key) = document.storage_key {
                if let Err(error) = self.object_store.delete(&key).await {
                    tracing::warn!(
                        matter_id,
                        document_id = document.document_id,
                        "Failed to delete stored matter document object: {}",
                        error
                    );
                }
            }
        }
        self.neo4j
            .run_rows(
                query(
                    "MATCH (m:Matter {matter_id: $matter_id})
                     OPTIONAL MATCH (m)-[*1..2]-(n)
                     WHERE n:Party OR n:CaseDocument OR n:Fact OR n:TimelineEvent OR n:Evidence OR
                           n:Claim OR n:Defense OR n:Element OR n:Draft OR n:DeadlineInstance OR
                           n:Task OR n:FactCheckFinding OR n:CitationCheckFinding OR
                           n:DocumentVersion OR n:IngestionRun OR n:SourceSpan OR n:ExtractedText OR
                           n:DraftParagraph OR n:WorkProduct OR n:WorkProductBlock OR
                           n:WorkProductMark OR n:WorkProductAnchor OR n:WorkProductFinding OR
                           n:WorkProductArtifact OR n:WorkProductHistoryEvent
                     DETACH DELETE n, m",
                )
                .param("matter_id", matter_id),
            )
            .await?;
        Ok(())
    }

    pub async fn create_party(
        &self,
        matter_id: &str,
        request: CreatePartyRequest,
    ) -> ApiResult<CaseParty> {
        self.require_matter(matter_id).await?;
        let id = generate_id("party", &request.name);
        let party = CaseParty {
            id: id.clone(),
            party_id: id,
            matter_id: matter_id.to_string(),
            name: request.name,
            role: request.role.unwrap_or_else(|| "witness".to_string()),
            party_type: request
                .party_type
                .unwrap_or_else(|| "individual".to_string()),
            represented_by: request.represented_by,
            contact_email: request.contact_email,
            contact_phone: request.contact_phone,
            notes: request.notes,
        };
        self.merge_node(matter_id, party_spec(), &party.party_id, &party)
            .await
    }

    pub async fn list_parties(&self, matter_id: &str) -> ApiResult<Vec<CaseParty>> {
        self.list_nodes(matter_id, party_spec()).await
    }

    pub async fn upload_file(
        &self,
        matter_id: &str,
        request: UploadFileRequest,
    ) -> ApiResult<CaseDocument> {
        self.require_matter(matter_id).await?;
        let now = now_string();
        let document_id = generate_opaque_id("doc");
        let title = title_from_filename(&request.filename);
        let bytes = request
            .text
            .as_ref()
            .map(|text| text.len() as u64)
            .or(request.bytes)
            .unwrap_or(0);
        self.ensure_upload_size(bytes)?;
        let object_key = build_document_object_key(&document_id, &request.filename);
        let (stored_object, hash) = if let Some(text) = &request.text {
            let hash = sha256_hex(text.as_bytes());
            let stored = self
                .object_store
                .put_bytes(
                    &object_key,
                    Bytes::copy_from_slice(text.as_bytes()),
                    put_options(request.mime_type.clone(), Some(hash.clone())),
                )
                .await?;
            (Some(stored), Some(hash))
        } else {
            (None, None)
        };
        let storage_status = if stored_object.is_some() {
            "stored"
        } else {
            "metadata_only"
        };

        let mut document = CaseDocument {
            id: document_id.clone(),
            document_id,
            matter_id: matter_id.to_string(),
            filename: request.filename,
            title,
            document_type: request.document_type.unwrap_or_else(|| "other".to_string()),
            mime_type: request.mime_type,
            pages: 1,
            bytes,
            file_hash: hash,
            uploaded_at: now,
            source: "user_upload".to_string(),
            confidentiality: request
                .confidentiality
                .unwrap_or_else(|| "private".to_string()),
            processing_status: if request.text.is_some() {
                "processed".to_string()
            } else {
                "queued".to_string()
            },
            is_exhibit: false,
            exhibit_label: None,
            summary: "Uploaded to CaseBuilder. Run extraction to populate facts and evidence."
                .to_string(),
            date_observed: None,
            parties_mentioned: Vec::new(),
            entities_mentioned: Vec::new(),
            facts_extracted: 0,
            citations_found: 0,
            contradictions_flagged: 0,
            linked_claim_ids: Vec::new(),
            folder: request.folder.unwrap_or_else(|| "Uploads".to_string()),
            storage_path: stored_object
                .as_ref()
                .and_then(|object| object.local_path.clone()),
            storage_provider: self.object_store.provider().to_string(),
            storage_status: storage_status.to_string(),
            storage_bucket: stored_object
                .as_ref()
                .and_then(|object| object.bucket.clone())
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            storage_key: stored_object.as_ref().map(|object| object.key.clone()),
            content_etag: stored_object
                .as_ref()
                .and_then(|object| object.etag.clone()),
            upload_expires_at: None,
            deleted_at: None,
            object_blob_id: None,
            current_version_id: None,
            ingestion_run_ids: Vec::new(),
            source_spans: Vec::new(),
            extracted_text: request.text,
        };

        let provenance = stored_object
            .as_ref()
            .map(|object| build_original_provenance(matter_id, &document, object, "stored"));
        if let Some(provenance) = &provenance {
            apply_document_provenance(&mut document, provenance);
        }

        let document = self
            .merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await?;
        if let Some(provenance) = provenance {
            self.persist_document_provenance(matter_id, &provenance)
                .await?;
        }
        Ok(document)
    }

    pub async fn upload_binary_file(
        &self,
        matter_id: &str,
        request: BinaryUploadRequest,
    ) -> ApiResult<CaseDocument> {
        self.require_matter(matter_id).await?;
        self.ensure_upload_size(request.bytes.len() as u64)?;
        validate_mime_type(request.mime_type.as_deref())?;

        let now = now_string();
        let document_id = generate_opaque_id("doc");
        let object_key = build_document_object_key(&document_id, &request.filename);
        let hash = sha256_hex(&request.bytes);
        let stored_object = self
            .object_store
            .put_bytes(
                &object_key,
                request.bytes.clone(),
                put_options(request.mime_type.clone(), Some(hash.clone())),
            )
            .await?;
        let parser = parse_document_bytes(
            &request.filename,
            request.mime_type.as_deref(),
            &request.bytes,
        );
        let parser_id = parser.parser_id.clone();
        let processing_status = parser.status.clone();

        let mut document = CaseDocument {
            id: document_id.clone(),
            document_id,
            matter_id: matter_id.to_string(),
            filename: request.filename.clone(),
            title: title_from_filename(&request.filename),
            document_type: request.document_type.unwrap_or_else(|| "other".to_string()),
            mime_type: request.mime_type,
            pages: 1,
            bytes: stored_object.content_length,
            file_hash: Some(hash),
            uploaded_at: now,
            source: "user_upload".to_string(),
            confidentiality: request
                .confidentiality
                .unwrap_or_else(|| "private".to_string()),
            processing_status,
            is_exhibit: false,
            exhibit_label: None,
            summary: parser.message,
            date_observed: None,
            parties_mentioned: Vec::new(),
            entities_mentioned: Vec::new(),
            facts_extracted: 0,
            citations_found: 0,
            contradictions_flagged: 0,
            linked_claim_ids: Vec::new(),
            folder: request.folder.unwrap_or_else(|| "Uploads".to_string()),
            storage_path: stored_object.local_path.clone(),
            storage_provider: self.object_store.provider().to_string(),
            storage_status: "stored".to_string(),
            storage_bucket: stored_object
                .bucket
                .clone()
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            storage_key: Some(stored_object.key.clone()),
            content_etag: stored_object.etag.clone(),
            upload_expires_at: None,
            deleted_at: None,
            object_blob_id: None,
            current_version_id: None,
            ingestion_run_ids: Vec::new(),
            source_spans: Vec::new(),
            extracted_text: parser.text,
        };

        let mut provenance =
            build_original_provenance(matter_id, &document, &stored_object, "stored");
        provenance.ingestion_run.parser_id = Some(parser_id);
        apply_document_provenance(&mut document, &provenance);
        let document = self
            .merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await?;
        self.persist_document_provenance(matter_id, &provenance)
            .await?;
        Ok(document)
    }

    pub async fn create_file_upload(
        &self,
        matter_id: &str,
        request: CreateFileUploadRequest,
    ) -> ApiResult<FileUploadResponse> {
        self.require_matter(matter_id).await?;
        if request.bytes == 0 {
            return Err(ApiError::BadRequest(
                "Upload intent bytes must be greater than 0".to_string(),
            ));
        }
        self.ensure_upload_size(request.bytes)?;
        validate_mime_type(request.mime_type.as_deref())?;

        let normalized_hash = match request.sha256.as_deref() {
            Some(value) => Some(normalize_sha256(value).ok_or_else(|| {
                ApiError::BadRequest("sha256 must be a hex SHA-256 digest".to_string())
            })?),
            None => None,
        };
        let now = now_string();
        let document_id = generate_opaque_id("doc");
        let upload_id = upload_id_for_document(&document_id);
        let object_key = build_document_object_key(&document_id, &request.filename);
        let expires_at = timestamp_after(self.upload_ttl_seconds);
        let presigned = self
            .object_store
            .presign_put(
                &object_key,
                put_options(request.mime_type.clone(), normalized_hash.clone()),
                Duration::from_secs(self.upload_ttl_seconds),
            )
            .await?;

        let document = CaseDocument {
            id: document_id.clone(),
            document_id: document_id.clone(),
            matter_id: matter_id.to_string(),
            filename: request.filename.clone(),
            title: title_from_filename(&request.filename),
            document_type: request.document_type.unwrap_or_else(|| "other".to_string()),
            mime_type: request.mime_type,
            pages: 1,
            bytes: request.bytes,
            file_hash: normalized_hash,
            uploaded_at: now,
            source: "user_upload".to_string(),
            confidentiality: request
                .confidentiality
                .unwrap_or_else(|| "private".to_string()),
            processing_status: "queued".to_string(),
            is_exhibit: false,
            exhibit_label: None,
            summary: "Upload pending. Complete the direct R2 upload to queue extraction."
                .to_string(),
            date_observed: None,
            parties_mentioned: Vec::new(),
            entities_mentioned: Vec::new(),
            facts_extracted: 0,
            citations_found: 0,
            contradictions_flagged: 0,
            linked_claim_ids: Vec::new(),
            folder: request.folder.unwrap_or_else(|| "Uploads".to_string()),
            storage_path: None,
            storage_provider: self.object_store.provider().to_string(),
            storage_status: "pending".to_string(),
            storage_bucket: self.object_store.bucket().map(str::to_string),
            storage_key: Some(object_key),
            content_etag: None,
            upload_expires_at: Some(expires_at.clone()),
            deleted_at: None,
            object_blob_id: None,
            current_version_id: None,
            ingestion_run_ids: Vec::new(),
            source_spans: Vec::new(),
            extracted_text: None,
        };
        let document = self
            .merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await?;

        Ok(FileUploadResponse {
            upload_id,
            document_id,
            method: presigned.method,
            url: presigned.url,
            expires_at,
            headers: presigned.headers,
            document,
        })
    }

    pub async fn complete_file_upload(
        &self,
        matter_id: &str,
        upload_id: &str,
        request: CompleteFileUploadRequest,
    ) -> ApiResult<CaseDocument> {
        let mut document = self.get_document(matter_id, &request.document_id).await?;
        if upload_id_for_document(&document.document_id) != upload_id {
            return Err(ApiError::BadRequest(
                "Upload id does not match document".to_string(),
            ));
        }
        if document.storage_status == "deleted" {
            return Err(ApiError::BadRequest(
                "Cannot complete a deleted document upload".to_string(),
            ));
        }
        if let Some(expires_at) = &document.upload_expires_at {
            if parse_timestamp(expires_at).is_some_and(|expires| expires < now_secs()) {
                document.storage_status = "failed".to_string();
                document.summary = "Upload URL expired before completion.".to_string();
                self.merge_node(matter_id, document_spec(), &document.document_id, &document)
                    .await?;
                return Err(ApiError::BadRequest("Upload URL expired".to_string()));
            }
        }
        if let Some(bytes) = request.bytes {
            self.ensure_upload_size(bytes)?;
            if bytes != document.bytes {
                return Err(ApiError::BadRequest(
                    "Completed upload size does not match intent".to_string(),
                ));
            }
        }
        if let Some(sha256) = request.sha256.as_deref() {
            let normalized = normalize_sha256(sha256).ok_or_else(|| {
                ApiError::BadRequest("sha256 must be a hex SHA-256 digest".to_string())
            })?;
            if document
                .file_hash
                .as_deref()
                .is_some_and(|expected| expected != normalized)
            {
                return Err(ApiError::BadRequest(
                    "Completed upload hash does not match intent".to_string(),
                ));
            }
            document.file_hash = Some(normalized);
        }

        let key = document
            .storage_key
            .clone()
            .ok_or_else(|| ApiError::BadRequest("Document has no storage key".to_string()))?;
        let object = self.object_store.head(&key).await?;
        if object.content_length != document.bytes {
            document.storage_status = "failed".to_string();
            document.summary = "Uploaded object size did not match the upload intent.".to_string();
            self.merge_node(matter_id, document_spec(), &document.document_id, &document)
                .await?;
            return Err(ApiError::BadRequest(
                "Uploaded object size did not match intent".to_string(),
            ));
        }
        if let (Some(actual), Some(expected)) = (object.etag.as_deref(), request.etag.as_deref()) {
            if clean_etag(actual) != clean_etag(expected) {
                return Err(ApiError::BadRequest(
                    "Completed upload ETag does not match R2 object".to_string(),
                ));
            }
        }
        if let Some(expected_hash) = document.file_hash.as_deref() {
            if let Some(actual_hash) = object.metadata.get("sha256") {
                if actual_hash != expected_hash {
                    return Err(ApiError::BadRequest(
                        "Completed upload hash metadata does not match intent".to_string(),
                    ));
                }
            }
        }
        if document.file_hash.is_none() {
            document.file_hash = object
                .metadata
                .get("sha256")
                .and_then(|hash| normalize_sha256(hash));
        }

        document.storage_status = "stored".to_string();
        document.storage_bucket = object
            .bucket
            .clone()
            .or_else(|| self.object_store.bucket().map(str::to_string));
        document.content_etag = object.etag.clone();
        document.upload_expires_at = None;
        document.summary =
            "Uploaded to private object storage. Run extraction to populate facts and evidence."
                .to_string();
        let provenance = build_original_provenance(matter_id, &document, &object, "stored");
        apply_document_provenance(&mut document, &provenance);
        let document = self
            .merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await?;
        self.persist_document_provenance(matter_id, &provenance)
            .await?;
        Ok(document)
    }

    pub async fn list_documents(&self, matter_id: &str) -> ApiResult<Vec<CaseDocument>> {
        self.list_nodes(matter_id, document_spec()).await
    }

    pub async fn get_document(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<CaseDocument> {
        self.get_node(matter_id, document_spec(), document_id).await
    }

    pub async fn extract_document(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<DocumentExtractionResponse> {
        let mut document = self.get_document(matter_id, document_id).await?;
        let provenance = self
            .ensure_document_original_provenance(matter_id, &mut document)
            .await?;
        let text = match document.extracted_text.clone() {
            Some(text) if !text.trim().is_empty() => text,
            _ => self.document_bytes_as_text(&document).await?,
        };

        if text.trim().is_empty() {
            let extraction_status = match document.processing_status.as_str() {
                "ocr_required" | "transcription_deferred" | "unsupported" => {
                    document.processing_status.clone()
                }
                _ => "failed".to_string(),
            };
            let error_code = match extraction_status.as_str() {
                "ocr_required" => "ocr_required",
                "transcription_deferred" => "transcription_deferred",
                "unsupported" => "unsupported_file_type",
                _ => "no_extractable_text",
            };
            document.processing_status = extraction_status.clone();
            document.summary = match extraction_status.as_str() {
                "ocr_required" => {
                    "No extractable text is available yet; OCR is required for this document."
                }
                "transcription_deferred" => {
                    "No extractable text is available yet; transcription is deferred for this media file."
                }
                "unsupported" => {
                    "No extractable text is available for this unsupported deterministic V0 file type."
                }
                _ => "No extractable text is available for this document in V0.",
            }
            .to_string();
            let ingestion_run = provenance.as_ref().map(|provenance| {
                failed_ingestion_run(
                    &provenance.ingestion_run,
                    "extract_text",
                    error_code,
                    &document.summary,
                    matches!(
                        extraction_status.as_str(),
                        "ocr_required" | "transcription_deferred"
                    ),
                )
            });
            if let Some(run) = &ingestion_run {
                self.merge_ingestion_run(matter_id, run).await?;
            }
            let document = self
                .merge_node(matter_id, document_spec(), document_id, &document)
                .await?;
            return Ok(DocumentExtractionResponse {
                enabled: true,
                mode: "deterministic".to_string(),
                status: extraction_status,
                message: document.summary.clone(),
                document,
                chunks: Vec::new(),
                proposed_facts: Vec::new(),
                ingestion_run,
                document_version: provenance.map(|provenance| provenance.document_version),
                source_spans: Vec::new(),
            });
        }

        let source_context = source_context_from_provenance(provenance.as_ref());
        let mut chunks = chunk_text(document_id, &text);
        for chunk in &mut chunks {
            chunk.document_version_id = source_context.document_version_id.clone();
            chunk.object_blob_id = source_context.object_blob_id.clone();
            chunk.source_span_id = Some(source_span_id(document_id, "chunk", chunk.page));
        }
        let mut source_spans =
            source_spans_for_chunks(matter_id, document_id, &chunks, &source_context);
        let proposed_facts = propose_facts(matter_id, document_id, &text, &source_context);
        for fact in &proposed_facts {
            source_spans.extend(fact.source_spans.clone());
        }
        document.extracted_text = Some(text.clone());
        document.processing_status = "processed".to_string();
        document.summary = summarize_text(&text);
        document.facts_extracted = proposed_facts.len() as u64;
        document.source_spans = source_spans.clone();
        let document = self
            .merge_node(matter_id, document_spec(), document_id, &document)
            .await?;

        for span in &source_spans {
            self.merge_source_span(matter_id, span).await?;
        }

        for chunk in &chunks {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (d:CaseDocument {document_id: $document_id})
                         MERGE (t:ExtractedText {chunk_id: $chunk_id})
                         SET t.document_id = $document_id,
                             t.matter_id = $matter_id,
                             t.page = $page,
                             t.text = $text,
                             t.document_version_id = $document_version_id,
                             t.object_blob_id = $object_blob_id,
                             t.source_span_id = $source_span_id,
                             t.byte_start = $byte_start,
                             t.byte_end = $byte_end,
                             t.char_start = $char_start,
                             t.char_end = $char_end
                         MERGE (d)-[:HAS_EXTRACTED_TEXT]->(t)
                         WITH d, t
                         OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                         OPTIONAL MATCH (s:SourceSpan {source_span_id: $source_span_id})
                         FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                           MERGE (v)-[:HAS_CHUNK]->(t)
                         )
                         FOREACH (_ IN CASE WHEN s IS NULL THEN [] ELSE [1] END |
                           MERGE (s)-[:QUOTES]->(t)
                         )",
                    )
                    .param("document_id", document_id)
                    .param("matter_id", matter_id)
                    .param("chunk_id", chunk.chunk_id.clone())
                    .param("page", chunk.page as i64)
                    .param("text", chunk.text.clone())
                    .param(
                        "document_version_id",
                        chunk.document_version_id.clone().unwrap_or_default(),
                    )
                    .param("object_blob_id", chunk.object_blob_id.clone().unwrap_or_default())
                    .param("source_span_id", chunk.source_span_id.clone().unwrap_or_default())
                    .param("byte_start", chunk.byte_start.unwrap_or_default() as i64)
                    .param("byte_end", chunk.byte_end.unwrap_or_default() as i64)
                    .param("char_start", chunk.char_start.unwrap_or_default() as i64)
                    .param("char_end", chunk.char_end.unwrap_or_default() as i64),
                )
                .await?;
        }

        let mut stored_facts = Vec::with_capacity(proposed_facts.len());
        for fact in proposed_facts {
            let fact = self
                .merge_node(matter_id, fact_spec(), &fact.fact_id, &fact)
                .await?;
            self.materialize_fact_edges(&fact).await?;
            stored_facts.push(fact);
        }
        let ingestion_run = provenance.as_ref().map(|provenance| {
            completed_ingestion_run(
                &provenance.ingestion_run,
                "review_ready",
                "review_ready",
                produced_node_ids(&chunks, &source_spans, &stored_facts),
            )
        });
        if let Some(run) = &ingestion_run {
            self.merge_ingestion_run(matter_id, run).await?;
        }

        Ok(DocumentExtractionResponse {
            enabled: true,
            mode: "deterministic".to_string(),
            status: "processed".to_string(),
            message: "Extracted text chunks and proposed reviewable facts. AI fact extraction is provider-gated in V0.".to_string(),
            document,
            chunks,
            proposed_facts: stored_facts,
            ingestion_run,
            document_version: provenance.map(|provenance| provenance.document_version),
            source_spans,
        })
    }

    pub async fn create_download_url(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<DownloadUrlResponse> {
        let document = self.get_document(matter_id, document_id).await?;
        if document.storage_status == "deleted" {
            return Err(ApiError::NotFound(format!(
                "Document {document_id} has been deleted"
            )));
        }
        let key = document
            .storage_key
            .as_deref()
            .ok_or_else(|| ApiError::BadRequest("Document has no stored object".to_string()))?;
        let expires_at = timestamp_after(self.download_ttl_seconds);
        let presigned = self
            .object_store
            .presign_get(key, Duration::from_secs(self.download_ttl_seconds))
            .await?;
        Ok(DownloadUrlResponse {
            method: presigned.method,
            url: presigned.url,
            expires_at,
            headers: presigned.headers,
            filename: document.filename,
            mime_type: document.mime_type,
            bytes: document.bytes,
        })
    }

    pub async fn delete_document(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<DeleteDocumentResponse> {
        let mut document = self.get_document(matter_id, document_id).await?;
        if let Some(key) = document.storage_key.clone() {
            self.object_store.delete(&key).await?;
        }
        document.storage_status = "deleted".to_string();
        document.processing_status = "failed".to_string();
        document.summary =
            "Document object deleted; metadata tombstone retained for provenance.".to_string();
        document.deleted_at = Some(now_string());
        document.content_etag = None;
        document.upload_expires_at = None;
        document.extracted_text = None;
        let document = self
            .merge_node(matter_id, document_spec(), document_id, &document)
            .await?;
        Ok(DeleteDocumentResponse {
            deleted: true,
            document,
        })
    }

    pub async fn create_fact(
        &self,
        matter_id: &str,
        request: CreateFactRequest,
    ) -> ApiResult<CaseFact> {
        self.require_matter(matter_id).await?;
        let id = generate_id("fact", &request.statement);
        let fact = CaseFact {
            id: id.clone(),
            fact_id: id,
            matter_id: matter_id.to_string(),
            statement: request.statement.clone(),
            text: request.statement,
            status: request.status.unwrap_or_else(|| "alleged".to_string()),
            confidence: request.confidence.unwrap_or(0.7),
            date: request.date,
            party_id: request.party_id,
            source_document_ids: request.source_document_ids.unwrap_or_default(),
            source_evidence_ids: request.source_evidence_ids.unwrap_or_default(),
            contradicted_by_evidence_ids: Vec::new(),
            supports_claim_ids: Vec::new(),
            supports_defense_ids: Vec::new(),
            used_in_draft_ids: Vec::new(),
            needs_verification: true,
            source_spans: Vec::new(),
            notes: request.notes,
        };
        let fact = self
            .merge_node(matter_id, fact_spec(), &fact.fact_id, &fact)
            .await?;
        self.materialize_fact_edges(&fact).await?;
        Ok(fact)
    }

    pub async fn list_facts(&self, matter_id: &str) -> ApiResult<Vec<CaseFact>> {
        self.list_nodes(matter_id, fact_spec()).await
    }

    pub async fn patch_fact(
        &self,
        matter_id: &str,
        fact_id: &str,
        request: PatchFactRequest,
    ) -> ApiResult<CaseFact> {
        let mut fact = self
            .get_node::<CaseFact>(matter_id, fact_spec(), fact_id)
            .await?;
        if let Some(value) = request.statement {
            fact.statement = value.clone();
            fact.text = value;
        }
        if let Some(value) = request.status {
            fact.status = value;
        }
        if let Some(value) = request.confidence {
            fact.confidence = value;
        }
        if request.date.is_some() {
            fact.date = request.date;
        }
        if request.party_id.is_some() {
            fact.party_id = request.party_id;
        }
        if request.notes.is_some() {
            fact.notes = request.notes;
        }
        self.merge_node(matter_id, fact_spec(), fact_id, &fact)
            .await
    }

    pub async fn approve_fact(&self, matter_id: &str, fact_id: &str) -> ApiResult<CaseFact> {
        let mut fact = self
            .get_node::<CaseFact>(matter_id, fact_spec(), fact_id)
            .await?;
        fact.status = "supported".to_string();
        fact.confidence = fact.confidence.max(0.85);
        fact.needs_verification = false;
        self.merge_node(matter_id, fact_spec(), fact_id, &fact)
            .await
    }

    pub async fn create_timeline_event(
        &self,
        matter_id: &str,
        request: CreateTimelineEventRequest,
    ) -> ApiResult<CaseTimelineEvent> {
        self.require_matter(matter_id).await?;
        let id = generate_id("event", &request.title);
        let event = CaseTimelineEvent {
            id: id.clone(),
            event_id: id,
            matter_id: matter_id.to_string(),
            date: request.date,
            title: request.title,
            description: request.description,
            kind: request.kind.unwrap_or_else(|| "other".to_string()),
            category: "user".to_string(),
            status: "complete".to_string(),
            source_document_id: request.source_document_id,
            party_ids: request.party_ids.unwrap_or_default(),
            linked_fact_ids: request.linked_fact_ids.unwrap_or_default(),
            linked_claim_ids: request.linked_claim_ids.unwrap_or_default(),
            date_confidence: 1.0,
            disputed: false,
        };
        self.merge_node(matter_id, timeline_spec(), &event.event_id, &event)
            .await
    }

    pub async fn list_timeline(&self, matter_id: &str) -> ApiResult<Vec<CaseTimelineEvent>> {
        self.list_nodes(matter_id, timeline_spec()).await
    }

    pub async fn create_evidence(
        &self,
        matter_id: &str,
        request: CreateEvidenceRequest,
    ) -> ApiResult<CaseEvidence> {
        self.require_matter(matter_id).await?;
        let mut document = self.get_document(matter_id, &request.document_id).await?;
        let provenance = self
            .ensure_document_original_provenance(matter_id, &mut document)
            .await?;
        if provenance.is_some() {
            self.merge_node(matter_id, document_spec(), &document.document_id, &document)
                .await?;
        }
        let id = generate_id("evidence", &request.quote);
        let source_context = source_context_from_provenance(provenance.as_ref());
        let source_spans = vec![manual_evidence_source_span(
            matter_id,
            &request.document_id,
            &id,
            request.source_span.as_deref(),
            &request.quote,
            &source_context,
        )];
        let evidence = CaseEvidence {
            id: id.clone(),
            evidence_id: id,
            matter_id: matter_id.to_string(),
            document_id: request.document_id,
            source_span: request
                .source_span
                .unwrap_or_else(|| "document".to_string()),
            quote: request.quote,
            evidence_type: request
                .evidence_type
                .unwrap_or_else(|| "document_text".to_string()),
            strength: request.strength.unwrap_or_else(|| "moderate".to_string()),
            confidence: request.confidence.unwrap_or(0.75),
            exhibit_label: request.exhibit_label,
            supports_fact_ids: request.supports_fact_ids.unwrap_or_default(),
            contradicts_fact_ids: request.contradicts_fact_ids.unwrap_or_default(),
            source_spans,
        };
        for span in &evidence.source_spans {
            self.merge_source_span(matter_id, span).await?;
        }
        let evidence = self
            .merge_node(matter_id, evidence_spec(), &evidence.evidence_id, &evidence)
            .await?;
        for fact_id in &evidence.supports_fact_ids {
            self.sync_fact_evidence_link(matter_id, &evidence.evidence_id, fact_id, "supports")
                .await?;
            self.sync_claim_element_evidence(matter_id, &evidence.evidence_id, fact_id)
                .await?;
        }
        for fact_id in &evidence.contradicts_fact_ids {
            self.sync_fact_evidence_link(matter_id, &evidence.evidence_id, fact_id, "contradicts")
                .await?;
        }
        self.materialize_evidence_edges(&evidence).await?;
        Ok(evidence)
    }

    pub async fn list_evidence(&self, matter_id: &str) -> ApiResult<Vec<CaseEvidence>> {
        self.list_nodes(matter_id, evidence_spec()).await
    }

    pub async fn link_evidence_fact(
        &self,
        matter_id: &str,
        evidence_id: &str,
        request: LinkEvidenceFactRequest,
    ) -> ApiResult<CaseEvidence> {
        let mut evidence = self
            .get_node::<CaseEvidence>(matter_id, evidence_spec(), evidence_id)
            .await?;
        let relation = request.relation.unwrap_or_else(|| "supports".to_string());
        match relation.as_str() {
            "contradicts" => {
                push_unique(&mut evidence.contradicts_fact_ids, request.fact_id.clone())
            }
            _ => push_unique(&mut evidence.supports_fact_ids, request.fact_id.clone()),
        }
        let evidence = self
            .merge_node(matter_id, evidence_spec(), evidence_id, &evidence)
            .await?;
        self.sync_fact_evidence_link(
            matter_id,
            &evidence.evidence_id,
            &request.fact_id,
            &relation,
        )
        .await?;
        self.sync_claim_element_evidence(matter_id, &evidence.evidence_id, &request.fact_id)
            .await?;
        self.materialize_evidence_edges(&evidence).await?;
        Ok(evidence)
    }

    pub async fn create_claim(
        &self,
        matter_id: &str,
        request: CreateClaimRequest,
    ) -> ApiResult<CaseClaim> {
        self.require_matter(matter_id).await?;
        let id = generate_id("claim", &request.title);
        let elements = request
            .elements
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(index, element)| {
                let authority = element.authority;
                let authorities = authority
                    .as_ref()
                    .map(|value| AuthorityRef {
                        citation: value.clone(),
                        canonical_id: value.clone(),
                        reason: None,
                        pinpoint: None,
                    })
                    .into_iter()
                    .collect();
                CaseElement {
                    id: format!("{id}:element:{}", index + 1),
                    element_id: format!("{id}:element:{}", index + 1),
                    matter_id: matter_id.to_string(),
                    text: element.text,
                    authority,
                    authorities,
                    satisfied: false,
                    fact_ids: element.fact_ids.unwrap_or_default(),
                    evidence_ids: element.evidence_ids.unwrap_or_default(),
                    missing_facts: Vec::new(),
                }
            })
            .collect::<Vec<_>>();
        let claim = CaseClaim {
            id: id.clone(),
            claim_id: id,
            matter_id: matter_id.to_string(),
            kind: request.kind.unwrap_or_else(|| "claim".to_string()),
            title: request.title.clone(),
            name: request.title,
            claim_type: request.claim_type.unwrap_or_else(|| "custom".to_string()),
            legal_theory: request.legal_theory.unwrap_or_default(),
            status: request.status.unwrap_or_else(|| "candidate".to_string()),
            risk_level: request.risk_level.unwrap_or_else(|| "medium".to_string()),
            fact_ids: request.fact_ids.unwrap_or_default(),
            evidence_ids: request.evidence_ids.unwrap_or_default(),
            authorities: request.authorities.unwrap_or_default(),
            elements,
        };
        let claim = self
            .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
            .await?;
        self.materialize_claim_edges(&claim).await?;
        Ok(claim)
    }

    pub async fn list_claims(&self, matter_id: &str) -> ApiResult<Vec<CaseClaim>> {
        self.list_nodes(matter_id, claim_spec()).await
    }

    pub async fn map_claim_elements(
        &self,
        matter_id: &str,
        claim_id: &str,
    ) -> ApiResult<CaseClaim> {
        let mut claim = self
            .get_node::<CaseClaim>(matter_id, claim_spec(), claim_id)
            .await?;
        for element in &mut claim.elements {
            element.satisfied = !element.fact_ids.is_empty();
            if element.satisfied {
                element.missing_facts.clear();
            } else if element.missing_facts.is_empty() {
                element
                    .missing_facts
                    .push("No reviewed fact has been linked to this element.".to_string());
            }
        }
        let claim = self
            .merge_node(matter_id, claim_spec(), claim_id, &claim)
            .await?;
        self.materialize_claim_edges(&claim).await?;
        Ok(claim)
    }

    pub async fn create_defense(
        &self,
        matter_id: &str,
        request: CreateDefenseRequest,
    ) -> ApiResult<CaseDefense> {
        self.require_matter(matter_id).await?;
        let id = generate_id("defense", &request.name);
        let defense = CaseDefense {
            id: id.clone(),
            defense_id: id,
            matter_id: matter_id.to_string(),
            name: request.name,
            basis: request.basis.unwrap_or_default(),
            status: request.status.unwrap_or_else(|| "candidate".to_string()),
            applies_to_claim_ids: request.applies_to_claim_ids.unwrap_or_default(),
            required_facts: request.required_facts.unwrap_or_default(),
            fact_ids: request.fact_ids.unwrap_or_default(),
            evidence_ids: request.evidence_ids.unwrap_or_default(),
            authorities: request.authorities.unwrap_or_default(),
            viability: request.viability.unwrap_or_else(|| "medium".to_string()),
        };
        self.merge_node(matter_id, defense_spec(), &defense.defense_id, &defense)
            .await
    }

    pub async fn list_defenses(&self, matter_id: &str) -> ApiResult<Vec<CaseDefense>> {
        self.list_nodes(matter_id, defense_spec()).await
    }

    pub async fn list_deadlines(&self, matter_id: &str) -> ApiResult<Vec<CaseDeadline>> {
        self.list_nodes(matter_id, deadline_spec()).await
    }

    pub async fn list_tasks(&self, matter_id: &str) -> ApiResult<Vec<CaseTask>> {
        self.list_nodes(matter_id, task_spec()).await
    }

    pub async fn list_work_products(&self, matter_id: &str) -> ApiResult<Vec<WorkProduct>> {
        self.require_matter(matter_id).await?;
        self.migrate_legacy_drafts_to_work_products(matter_id)
            .await?;
        self.migrate_complaints_to_work_products(matter_id).await?;
        let mut products = self
            .list_nodes::<WorkProduct>(matter_id, work_product_spec())
            .await?;
        for product in &mut products {
            refresh_work_product_state(product);
        }
        Ok(products)
    }

    pub async fn list_work_products_for_api(
        &self,
        matter_id: &str,
        include_document_ast: bool,
    ) -> ApiResult<Vec<WorkProduct>> {
        let mut products = self.list_work_products(matter_id).await?;
        if !include_document_ast {
            for product in &mut products {
                summarize_work_product_for_list(product);
            }
        }
        Ok(products)
    }

    pub async fn create_work_product(
        &self,
        matter_id: &str,
        request: CreateWorkProductRequest,
    ) -> ApiResult<WorkProduct> {
        let product_type = normalize_work_product_type(&request.product_type)?;
        let title = request
            .title
            .clone()
            .unwrap_or_else(|| format!("{} work product", humanize_product_type(&product_type)));
        let work_product_id = generate_id("work-product", &title);
        self.create_work_product_with_id(matter_id, &work_product_id, request)
            .await
    }

    async fn create_work_product_with_id(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: CreateWorkProductRequest,
    ) -> ApiResult<WorkProduct> {
        let matter = self.get_matter_summary(matter_id).await?;
        let facts = self.list_facts(matter_id).await.unwrap_or_default();
        let claims = self.list_claims(matter_id).await.unwrap_or_default();
        let product_type = normalize_work_product_type(&request.product_type)?;
        let now = now_string();
        let title = request
            .title
            .unwrap_or_else(|| format!("{} work product", humanize_product_type(&product_type)));
        let mut product = default_work_product_from_matter(
            &matter,
            work_product_id,
            &title,
            &product_type,
            &facts,
            &claims,
            &now,
        );
        product.source_draft_id = request.source_draft_id;
        product.source_complaint_id = request.source_complaint_id;
        if let Some(template) = request.template {
            product.history.push(work_product_event(
                matter_id,
                work_product_id,
                "template_selected",
                "work_product",
                work_product_id,
                &format!("Template selected: {template}."),
            ));
        }
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            None,
            &product,
            "editor",
            "auto",
            "Work product created",
            "Shared WorkProduct AST created.",
            vec![VersionChangeInput {
                target_type: "work_product".to_string(),
                target_id: work_product_id.to_string(),
                operation: "create".to_string(),
                before: None,
                after: json_value(&product).ok(),
                summary: "Work product created.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn get_work_product(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<WorkProduct> {
        match self
            .get_node(matter_id, work_product_spec(), work_product_id)
            .await
        {
            Ok(mut product) => {
                refresh_work_product_state(&mut product);
                Ok(product)
            }
            Err(ApiError::NotFound(_)) => {
                self.migrate_legacy_drafts_to_work_products(matter_id)
                    .await?;
                self.migrate_complaints_to_work_products(matter_id).await?;
                let mut product = self
                    .get_node(matter_id, work_product_spec(), work_product_id)
                    .await?;
                refresh_work_product_state(&mut product);
                Ok(product)
            }
            Err(error) => Err(error),
        }
    }

    pub async fn patch_work_product(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: PatchWorkProductRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        if let Some(value) = request.title {
            product.title = value;
        }
        if let Some(value) = request.status {
            product.status = value;
        }
        if let Some(value) = request.review_status {
            product.review_status = value;
        }
        if let Some(value) = request.setup_stage {
            product.setup_stage = value;
        }
        if let Some(value) = request.document_ast {
            product.document_ast = value;
            normalize_work_product_ast(&mut product);
            product.blocks = flatten_work_product_blocks(&product.document_ast.blocks);
            product.marks.clear();
            product.anchors.clear();
        }
        if let Some(value) = request.blocks {
            product.blocks = value;
            rebuild_work_product_ast_from_projection(&mut product);
        }
        if let Some(value) = request.marks {
            product.marks = value;
            rebuild_work_product_ast_from_projection(&mut product);
        }
        if let Some(value) = request.anchors {
            product.anchors = value;
            rebuild_work_product_ast_from_projection(&mut product);
        }
        if let Some(value) = request.formatting_profile {
            product.formatting_profile = value;
        }
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "work_product_updated",
            "work_product",
            work_product_id,
            "Work product metadata or AST was updated.",
        ));
        refresh_work_product_state(&mut product);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "editor",
            "auto",
            "Work product updated",
            "Work product metadata or AST was updated.",
            vec![VersionChangeInput {
                target_type: "work_product".to_string(),
                target_id: work_product_id.to_string(),
                operation: "update".to_string(),
                before: json_value(&before_product).ok(),
                after: json_value(&product).ok(),
                summary: "Work product metadata or AST was updated.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn apply_work_product_ast_patch(
        &self,
        matter_id: &str,
        work_product_id: &str,
        patch: AstPatch,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        normalize_work_product_ast(&mut product);
        let current_document_hash = work_product_hashes(&product)?.document_hash;
        let current_snapshot_id = if patch.base_snapshot_id.is_some() {
            self.latest_work_product_snapshot_id(matter_id, work_product_id)
                .await?
        } else {
            None
        };
        validate_ast_patch_concurrency(
            &patch,
            work_product_id,
            &current_document_hash,
            current_snapshot_id.as_deref(),
        )?;
        self.validate_ast_patch_matter_references(matter_id, &product, &patch)
            .await?;
        for operation in &patch.operations {
            apply_ast_operation(&mut product.document_ast, operation)?;
        }
        normalize_work_product_ast(&mut product);
        let validation = validate_work_product_document(&product);
        if !validation.errors.is_empty() {
            let codes = validation
                .errors
                .iter()
                .map(|issue| issue.code.clone())
                .collect::<Vec<_>>()
                .join(",");
            return Err(ApiError::BadRequest(format!(
                "AST patch failed validation: issue_codes={codes}"
            )));
        }
        product.blocks = flatten_work_product_blocks(&product.document_ast.blocks);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "ast_patch_applied",
            "document_ast",
            &product.document_ast.document_id,
            "AST patch applied.",
        ));
        refresh_work_product_state(&mut product);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "editor",
            "ast_patch",
            "AST patch applied",
            "AST patch applied.",
            vec![VersionChangeInput {
                target_type: "document_ast".to_string(),
                target_id: product.document_ast.document_id.clone(),
                operation: "patch".to_string(),
                before: json_value(&before_product.document_ast).ok(),
                after: json_value(&product.document_ast).ok(),
                summary: "AST patch applied.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn validate_work_product_ast(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<AstValidationResponse> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        Ok(validate_work_product_document(&product))
    }

    pub async fn work_product_ast_to_markdown(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<AstMarkdownResponse> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        Ok(AstMarkdownResponse {
            markdown: work_product_markdown(&product),
            warnings: validate_work_product_document(&product)
                .warnings
                .into_iter()
                .map(|issue| issue.message)
                .collect(),
        })
    }

    pub async fn work_product_ast_from_markdown(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: MarkdownToAstRequest,
    ) -> ApiResult<AstDocumentResponse> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        let (document_ast, warnings) = markdown_to_work_product_ast(&product, &request.markdown);
        Ok(AstDocumentResponse {
            document_ast,
            warnings,
        })
    }

    pub async fn work_product_ast_to_html(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<AstRenderedResponse> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        Ok(AstRenderedResponse {
            html: Some(render_work_product_preview(&product).html),
            plain_text: None,
            warnings: validate_work_product_document(&product)
                .warnings
                .into_iter()
                .map(|issue| issue.message)
                .collect(),
        })
    }

    pub async fn work_product_ast_to_plain_text(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<AstRenderedResponse> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        Ok(AstRenderedResponse {
            html: None,
            plain_text: Some(work_product_plain_text(&product)),
            warnings: validate_work_product_document(&product)
                .warnings
                .into_iter()
                .map(|issue| issue.message)
                .collect(),
        })
    }

    pub async fn create_work_product_block(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: CreateWorkProductBlockRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let role = request.role.unwrap_or_else(|| "custom".to_string());
        let block_id = format!(
            "{work_product_id}:block:{}",
            product.blocks.len().saturating_add(1)
        );
        product.blocks.push(WorkProductBlock {
            id: block_id.clone(),
            block_id: block_id.clone(),
            matter_id: matter_id.to_string(),
            work_product_id: work_product_id.to_string(),
            block_type: request
                .block_type
                .unwrap_or_else(|| "paragraph".to_string()),
            role: role.clone(),
            title: request
                .title
                .unwrap_or_else(|| humanize_product_type(&role)),
            text: request.text,
            ordinal: product.blocks.len() as u64 + 1,
            parent_block_id: request.parent_block_id,
            fact_ids: request.fact_ids.unwrap_or_default(),
            evidence_ids: request.evidence_ids.unwrap_or_default(),
            authorities: request.authorities.unwrap_or_default(),
            mark_ids: Vec::new(),
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: None,
            ..WorkProductBlock::default()
        });
        rebuild_work_product_ast_from_projection(&mut product);
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "block_created",
            "block",
            &block_id,
            "Work product block created.",
        ));
        refresh_work_product_state(&mut product);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        let product = self.save_work_product(matter_id, product).await?;
        let after_block = product
            .blocks
            .iter()
            .find(|block| block.block_id == block_id)
            .cloned();
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "editor",
            "auto",
            "Block created",
            "Work product block created.",
            vec![VersionChangeInput {
                target_type: "block".to_string(),
                target_id: block_id.clone(),
                operation: "create".to_string(),
                before: None,
                after: after_block
                    .as_ref()
                    .and_then(|block| json_value(block).ok()),
                summary: "Work product block created.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn patch_work_product_block(
        &self,
        matter_id: &str,
        work_product_id: &str,
        block_id: &str,
        request: PatchWorkProductBlockRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let before_block = product
            .blocks
            .iter()
            .find(|block| block.block_id == block_id)
            .cloned();
        let block = product
            .blocks
            .iter_mut()
            .find(|block| block.block_id == block_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("Work product block {block_id} not found"))
            })?;
        if block.locked && request.text.is_some() {
            return Err(ApiError::BadRequest(format!(
                "Work product block {block_id} is locked"
            )));
        }
        if let Some(value) = request.block_type {
            block.block_type = value;
        }
        if let Some(value) = request.role {
            block.role = value;
        }
        if let Some(value) = request.title {
            block.title = value;
        }
        if let Some(value) = request.text {
            block.text = value;
        }
        if let Some(value) = request.parent_block_id {
            block.parent_block_id = value;
        }
        if let Some(value) = request.fact_ids {
            block.fact_ids = value;
        }
        if let Some(value) = request.evidence_ids {
            block.evidence_ids = value;
        }
        if let Some(value) = request.authorities {
            block.authorities = value;
        }
        if let Some(value) = request.locked {
            block.locked = value;
        }
        if let Some(value) = request.review_status {
            block.review_status = value;
        }
        if let Some(value) = request.prosemirror_json {
            block.prosemirror_json = value;
        }
        rebuild_work_product_ast_from_projection(&mut product);
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "block_updated",
            "block",
            block_id,
            "Work product block updated.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        let after_block = product
            .blocks
            .iter()
            .find(|block| block.block_id == block_id)
            .cloned();
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "editor",
            "auto",
            "Block updated",
            "Work product block updated.",
            vec![VersionChangeInput {
                target_type: "block".to_string(),
                target_id: block_id.to_string(),
                operation: "update".to_string(),
                before: before_block
                    .as_ref()
                    .and_then(|block| json_value(block).ok()),
                after: after_block
                    .as_ref()
                    .and_then(|block| json_value(block).ok()),
                summary: "Work product block updated.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn link_work_product_support(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: WorkProductLinkRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        self.validate_work_product_link_target(matter_id, &request.target_type, &request.target_id)
            .await?;
        let block_index = product
            .blocks
            .iter()
            .position(|block| block.block_id == request.block_id)
            .ok_or_else(|| ApiError::NotFound("Work product block not found".to_string()))?;
        let anchor_id = format!(
            "{}:anchor:{}",
            request.block_id,
            product.anchors.len().saturating_add(1)
        );
        let anchor_type = request.anchor_type.unwrap_or_else(|| {
            if request.citation.is_some() || request.canonical_id.is_some() {
                "authority".to_string()
            } else {
                request.target_type.clone()
            }
        });
        let relation = request.relation.unwrap_or_else(|| "supports".to_string());
        {
            let block = &mut product.blocks[block_index];
            match request.target_type.as_str() {
                "fact" => push_unique(&mut block.fact_ids, request.target_id.clone()),
                "evidence" | "document" | "source_span" => {
                    push_unique(&mut block.evidence_ids, request.target_id.clone())
                }
                "authority" | "provision" | "legal_text" => push_authority(
                    &mut block.authorities,
                    AuthorityRef {
                        citation: request
                            .citation
                            .clone()
                            .unwrap_or_else(|| request.target_id.clone()),
                        canonical_id: request
                            .canonical_id
                            .clone()
                            .unwrap_or_else(|| request.target_id.clone()),
                        reason: Some(relation.clone()),
                        pinpoint: request.pinpoint.clone(),
                    },
                ),
                _ => {}
            }
            push_unique(&mut block.mark_ids, anchor_id.clone());
        }
        product.anchors.push(WorkProductAnchor {
            id: anchor_id.clone(),
            anchor_id: anchor_id.clone(),
            matter_id: matter_id.to_string(),
            work_product_id: work_product_id.to_string(),
            block_id: request.block_id.clone(),
            anchor_type: anchor_type.clone(),
            target_type: request.target_type,
            target_id: request.target_id,
            relation,
            citation: request.citation,
            canonical_id: request.canonical_id,
            pinpoint: request.pinpoint,
            quote: request.quote,
            status: "needs_review".to_string(),
        });
        product.marks.push(WorkProductMark {
            id: format!("{anchor_id}:mark"),
            mark_id: format!("{anchor_id}:mark"),
            matter_id: matter_id.to_string(),
            work_product_id: work_product_id.to_string(),
            block_id: request.block_id.clone(),
            mark_type: anchor_type,
            from_offset: 0,
            to_offset: 0,
            label: "linked support".to_string(),
            target_type: "anchor".to_string(),
            target_id: anchor_id.clone(),
            status: "derived_from_ast".to_string(),
        });
        rebuild_work_product_ast_from_projection(&mut product);
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "support_linked",
            "anchor",
            &anchor_id,
            "Support or authority anchor linked to a work product block.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        let after_anchor = product
            .anchors
            .iter()
            .find(|anchor| anchor.anchor_id == anchor_id)
            .cloned();
        let mut impact = LegalImpactSummary::default();
        if let Some(anchor) = &after_anchor {
            match anchor.target_type.as_str() {
                "fact" => impact.affected_facts.push(anchor.target_id.clone()),
                "evidence" | "document" | "source_span" => {
                    impact.affected_evidence.push(anchor.target_id.clone())
                }
                "authority" | "provision" | "legal_text" => impact.affected_authorities.push(
                    anchor
                        .canonical_id
                        .clone()
                        .unwrap_or_else(|| anchor.target_id.clone()),
                ),
                _ => {}
            }
        }
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "support_link",
            "auto",
            "Support linked",
            "Support or authority anchor linked to a work product block.",
            vec![VersionChangeInput {
                target_type: "support_use".to_string(),
                target_id: anchor_id.clone(),
                operation: "link".to_string(),
                before: None,
                after: after_anchor
                    .as_ref()
                    .and_then(|anchor| json_value(anchor).ok()),
                summary: "Support or authority anchor linked.".to_string(),
                legal_impact: impact,
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn run_work_product_qc(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<AiActionResponse<Vec<WorkProductFinding>>> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        product.findings = work_product_findings(&product);
        product.document_ast.rule_findings = product.findings.clone();
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "qc_run",
            "work_product",
            work_product_id,
            "Deterministic work-product QC run completed.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        let mut impact = LegalImpactSummary::default();
        impact.qc_warnings_added = product
            .findings
            .iter()
            .filter(|finding| finding.status == "open")
            .map(|finding| finding.finding_id.clone())
            .collect();
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "rule_check",
            "rule_check",
            "QC run completed",
            "Deterministic work-product QC run completed.",
            vec![VersionChangeInput {
                target_type: "qc".to_string(),
                target_id: work_product_id.to_string(),
                operation: "update".to_string(),
                before: json_value(&before_product.findings).ok(),
                after: json_value(&product.findings).ok(),
                summary: "Deterministic work-product QC run completed.".to_string(),
                legal_impact: impact,
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(AiActionResponse {
            enabled: false,
            mode: "deterministic".to_string(),
            message: "No live provider is configured; ran deterministic work-product checks."
                .to_string(),
            result: Some(product.findings),
        })
    }

    pub async fn list_work_product_findings(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<WorkProductFinding>> {
        Ok(self
            .get_work_product(matter_id, work_product_id)
            .await?
            .findings)
    }

    pub async fn patch_work_product_finding(
        &self,
        matter_id: &str,
        work_product_id: &str,
        finding_id: &str,
        request: PatchWorkProductFindingRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let before_finding = product
            .findings
            .iter()
            .find(|finding| finding.finding_id == finding_id)
            .cloned();
        let finding = product
            .findings
            .iter_mut()
            .find(|finding| finding.finding_id == finding_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("Work product finding {finding_id} not found"))
            })?;
        finding.status = request.status;
        finding.updated_at = now_string();
        product.document_ast.rule_findings = product.findings.clone();
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "finding_updated",
            "finding",
            finding_id,
            "Work product finding status changed.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        let after_finding = product
            .findings
            .iter()
            .find(|finding| finding.finding_id == finding_id)
            .cloned();
        let mut impact = LegalImpactSummary::default();
        if let Some(before) = &before_finding {
            if before.status == "open" {
                impact.qc_warnings_resolved.push(before.finding_id.clone());
            }
        }
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "rule_check",
            "auto",
            "Finding updated",
            "Work product finding status changed.",
            vec![VersionChangeInput {
                target_type: "rule_finding".to_string(),
                target_id: finding_id.to_string(),
                operation: "resolve".to_string(),
                before: before_finding
                    .as_ref()
                    .and_then(|finding| json_value(finding).ok()),
                after: after_finding
                    .as_ref()
                    .and_then(|finding| json_value(finding).ok()),
                summary: "Work product finding status changed.".to_string(),
                legal_impact: impact,
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn preview_work_product(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<WorkProductPreviewResponse> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        Ok(render_work_product_preview(&product))
    }

    pub async fn export_work_product(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: ExportWorkProductRequest,
    ) -> ApiResult<WorkProductArtifact> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let format = normalize_export_format(&request.format)?;
        let profile = request.profile.unwrap_or_else(|| "review".to_string());
        let mode = request.mode.unwrap_or_else(|| "review_needed".to_string());
        let generated_at = now_string();
        let artifact_id = format!("{work_product_id}:artifact:{format}:{generated_at}");
        let warnings = work_product_export_warnings(
            &product,
            &format,
            request.include_exhibits.unwrap_or(false),
            request.include_qc_report.unwrap_or(false),
        );
        let export_content = render_work_product_export_content(&product, &format)?;
        let artifact_hash = sha256_hex(export_content.as_bytes());
        let artifact_key = work_product_export_key(
            matter_id,
            work_product_id,
            &artifact_id,
            &artifact_hash,
            &format,
        );
        let artifact_blob = self
            .store_casebuilder_bytes(
                matter_id,
                &artifact_key,
                Bytes::from(export_content.clone()),
                export_mime_type(&format),
            )
            .await?;
        let content_preview = export_content_preview(&export_content);
        let render_profile_hash = sha256_hex(format!("{format}:{profile}:{mode}").as_bytes());
        let next_sequence = self
            .list_work_product_snapshots(matter_id, work_product_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|snapshot| snapshot.sequence_number)
            .max()
            .unwrap_or(0)
            + 1;
        let snapshot_id = format!("{work_product_id}:snapshot:{next_sequence}");
        let qc_status_at_export = work_product_qc_status(&product);
        let artifact = WorkProductArtifact {
            id: artifact_id.clone(),
            artifact_id: artifact_id.clone(),
            matter_id: matter_id.to_string(),
            work_product_id: work_product_id.to_string(),
            format: format.clone(),
            profile,
            mode,
            status: "generated_review_needed".to_string(),
            download_url: format!(
                "/api/v1/matters/{matter_id}/work-products/{work_product_id}/artifacts/{artifact_id}/download"
            ),
            page_count: render_work_product_preview(&product).page_count,
            generated_at,
            warnings,
            content_preview,
            snapshot_id: Some(snapshot_id.clone()),
            artifact_hash: Some(artifact_hash),
            render_profile_hash: Some(render_profile_hash),
            qc_status_at_export: Some(qc_status_at_export),
            changed_since_export: Some(false),
            immutable: Some(true),
            object_blob_id: Some(artifact_blob.object_blob_id.clone()),
            size_bytes: Some(artifact_blob.size_bytes),
            mime_type: artifact_blob.mime_type.clone(),
            storage_status: Some("stored".to_string()),
        };
        product.artifacts.push(artifact.clone());
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "artifact_generated",
            "artifact",
            &artifact_id,
            "Work product export artifact generated for human review.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "export",
            "export",
            "Export generated",
            "Work product export artifact generated for human review.",
            vec![VersionChangeInput {
                target_type: "export".to_string(),
                target_id: artifact_id.clone(),
                operation: "export".to_string(),
                before: None,
                after: json_value(&artifact).ok(),
                summary: "Work product export artifact generated.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(artifact)
    }

    pub async fn get_work_product_artifact(
        &self,
        matter_id: &str,
        work_product_id: &str,
        artifact_id: &str,
    ) -> ApiResult<WorkProductArtifact> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        product
            .artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == artifact_id)
            .ok_or_else(|| ApiError::NotFound("Work product artifact not found".to_string()))
    }

    pub async fn download_work_product_artifact(
        &self,
        matter_id: &str,
        work_product_id: &str,
        artifact_id: &str,
    ) -> ApiResult<WorkProductDownloadResponse> {
        let artifact = self
            .get_work_product_artifact(matter_id, work_product_id, artifact_id)
            .await?;
        let mut headers = BTreeMap::new();
        headers.insert("X-Review-Needed".to_string(), "true".to_string());
        let mut method = "GET".to_string();
        let mut url = artifact.download_url.clone();
        let mut expires_at = now_string();
        let mut bytes = artifact
            .size_bytes
            .unwrap_or_else(|| artifact.content_preview.len() as u64);
        if let Some(object_blob_id) = artifact.object_blob_id.as_deref() {
            let blob = self.get_object_blob(matter_id, object_blob_id).await?;
            bytes = blob.size_bytes;
            expires_at = timestamp_after(self.download_ttl_seconds);
            if self.object_store.provider() == "r2" {
                let presigned = self
                    .object_store
                    .presign_get(
                        &blob.storage_key,
                        Duration::from_secs(self.download_ttl_seconds),
                    )
                    .await?;
                method = presigned.method;
                url = presigned.url;
                headers.extend(presigned.headers);
            }
        }
        Ok(WorkProductDownloadResponse {
            method,
            url,
            expires_at,
            headers,
            filename: safe_work_product_download_filename(&artifact),
            mime_type: Some(export_mime_type(&artifact.format).to_string()),
            bytes,
            artifact,
        })
    }

    pub async fn run_work_product_ai_command(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: WorkProductAiCommandRequest,
    ) -> ApiResult<AiActionResponse<WorkProduct>> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let message = format!(
            "AI command '{}' is in template mode; no provider is configured.",
            request.command
        );
        let target_id = request
            .target_id
            .clone()
            .unwrap_or_else(|| work_product_id.to_string());
        for command in &mut product.ai_commands {
            if command.command_id == request.command {
                command.last_message = Some(message.clone());
            }
        }
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "ai_command_requested",
            request.target_id.as_deref().unwrap_or("work_product"),
            &target_id,
            "AI command requested in provider-free template mode.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        let input_snapshot_id = self
            .list_work_product_snapshots(matter_id, work_product_id)
            .await
            .unwrap_or_default()
            .last()
            .map(|snapshot| snapshot.snapshot_id.clone())
            .unwrap_or_default();
        let ai_audit_id = generate_id(
            "ai-audit",
            &format!("{work_product_id}:{}", request.command),
        );
        let ai_audit = AIEditAudit {
            id: ai_audit_id.clone(),
            ai_audit_id: ai_audit_id.clone(),
            matter_id: matter_id.to_string(),
            subject_type: "work_product".to_string(),
            subject_id: work_product_id.to_string(),
            target_type: request
                .target_id
                .as_ref()
                .map(|_| "block")
                .unwrap_or("work_product")
                .to_string(),
            target_id,
            command: request.command.clone(),
            prompt_template_id: None,
            model: None,
            provider_mode: "template".to_string(),
            input_fact_ids: product
                .blocks
                .iter()
                .flat_map(|block| block.fact_ids.clone())
                .collect(),
            input_evidence_ids: product
                .blocks
                .iter()
                .flat_map(|block| block.evidence_ids.clone())
                .collect(),
            input_authority_ids: product
                .blocks
                .iter()
                .flat_map(|block| {
                    block
                        .authorities
                        .iter()
                        .map(|authority| authority.canonical_id.clone())
                        .collect::<Vec<_>>()
                })
                .collect(),
            input_snapshot_id,
            output_text: None,
            inserted_text: None,
            user_action: "template_recorded".to_string(),
            warnings: vec![message.clone()],
            created_at: now_string(),
        };
        self.merge_node(
            matter_id,
            ai_edit_audit_spec(),
            &ai_audit.ai_audit_id,
            &ai_audit,
        )
        .await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "ai",
            "ai_edit",
            "AI command recorded",
            &message,
            vec![VersionChangeInput {
                target_type: "ai_edit".to_string(),
                target_id: ai_audit.target_id.clone(),
                operation: "create".to_string(),
                before: None,
                after: json_value(&ai_audit).ok(),
                summary: message.clone(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: Some(ai_audit.ai_audit_id.clone()),
            }],
        )
        .await?;
        Ok(AiActionResponse {
            enabled: false,
            mode: "template".to_string(),
            message,
            result: Some(product),
        })
    }

    pub async fn work_product_history(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<ChangeSet>> {
        self.get_work_product(matter_id, work_product_id).await?;
        self.list_work_product_change_sets(matter_id, work_product_id)
            .await
    }

    pub async fn get_work_product_change_set(
        &self,
        matter_id: &str,
        work_product_id: &str,
        change_set_id: &str,
    ) -> ApiResult<ChangeSet> {
        self.get_work_product(matter_id, work_product_id).await?;
        let change_set = self
            .get_node::<ChangeSet>(matter_id, change_set_spec(), change_set_id)
            .await?;
        if change_set.subject_id == work_product_id {
            Ok(change_set)
        } else {
            Err(ApiError::NotFound(format!(
                "Change set {change_set_id} not found"
            )))
        }
    }

    pub async fn list_work_product_snapshots(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<VersionSnapshot>> {
        self.get_work_product(matter_id, work_product_id).await?;
        let mut snapshots = self
            .list_nodes::<VersionSnapshot>(matter_id, version_snapshot_spec())
            .await?
            .into_iter()
            .filter(|snapshot| snapshot.subject_id == work_product_id)
            .collect::<Vec<_>>();
        snapshots.sort_by_key(|snapshot| snapshot.sequence_number);
        Ok(snapshots)
    }

    pub async fn list_work_product_snapshots_for_api(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<VersionSnapshot>> {
        let mut snapshots = self
            .list_work_product_snapshots(matter_id, work_product_id)
            .await?;
        for snapshot in &mut snapshots {
            summarize_version_snapshot_for_list(snapshot);
        }
        Ok(snapshots)
    }

    async fn latest_work_product_snapshot_id(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Option<String>> {
        Ok(latest_snapshot_id(
            &self
                .list_work_product_snapshots(matter_id, work_product_id)
                .await?,
        ))
    }

    pub async fn get_work_product_snapshot(
        &self,
        matter_id: &str,
        work_product_id: &str,
        snapshot_id: &str,
    ) -> ApiResult<VersionSnapshot> {
        self.get_work_product(matter_id, work_product_id).await?;
        let snapshot = self
            .get_node::<VersionSnapshot>(matter_id, version_snapshot_spec(), snapshot_id)
            .await?;
        if snapshot.subject_id == work_product_id {
            let mut snapshot = snapshot;
            self.hydrate_snapshot_full_state(matter_id, &mut snapshot)
                .await?;
            Ok(snapshot)
        } else {
            Err(ApiError::NotFound(format!(
                "Snapshot {snapshot_id} not found"
            )))
        }
    }

    pub async fn create_work_product_snapshot(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: CreateVersionSnapshotRequest,
    ) -> ApiResult<VersionSnapshot> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        let title = request
            .title
            .unwrap_or_else(|| "Manual snapshot".to_string());
        let change_set = self
            .record_work_product_change(
                matter_id,
                None,
                &product,
                "manual_snapshot",
                "manual",
                &title,
                request
                    .message
                    .as_deref()
                    .unwrap_or("Manual Case History snapshot created."),
                vec![VersionChangeInput {
                    target_type: "work_product".to_string(),
                    target_id: work_product_id.to_string(),
                    operation: "snapshot".to_string(),
                    before: None,
                    after: json_value(&product).ok(),
                    summary: "Manual snapshot created.".to_string(),
                    legal_impact: LegalImpactSummary::default(),
                    ai_audit_id: None,
                }],
            )
            .await?;
        self.get_work_product_snapshot(matter_id, work_product_id, &change_set.snapshot_id)
            .await
    }

    pub async fn compare_work_product_snapshots(
        &self,
        matter_id: &str,
        work_product_id: &str,
        from_snapshot_id: &str,
        to_snapshot_id: Option<&str>,
        layers: Vec<String>,
    ) -> ApiResult<CompareVersionsResponse> {
        let current = self.get_work_product(matter_id, work_product_id).await?;
        let from_snapshot = self
            .get_work_product_snapshot(matter_id, work_product_id, from_snapshot_id)
            .await?;
        let from_product = self
            .product_from_snapshot(matter_id, &from_snapshot)
            .await?;
        let (to_id, to_product) = if let Some(snapshot_id) = to_snapshot_id {
            let snapshot = self
                .get_work_product_snapshot(matter_id, work_product_id, snapshot_id)
                .await?;
            (
                snapshot.snapshot_id.clone(),
                self.product_from_snapshot(matter_id, &snapshot).await?,
            )
        } else {
            ("current".to_string(), current)
        };
        let selected_layers = normalize_compare_layers(layers);
        let text_diffs = if selected_layers.iter().any(|layer| layer == "text") {
            diff_work_product_blocks(&from_product, &to_product)
        } else {
            Vec::new()
        };
        let layer_diffs = diff_work_product_layers(&from_product, &to_product, &selected_layers)?;
        let summary = VersionChangeSummary {
            text_changes: text_diffs
                .iter()
                .filter(|diff| diff.status != "unchanged")
                .count() as u64,
            support_changes: layer_change_count(&layer_diffs, "support"),
            citation_changes: layer_change_count(&layer_diffs, "citations"),
            authority_changes: layer_diffs
                .iter()
                .filter(|diff| {
                    diff.layer == "support"
                        && diff.status != "unchanged"
                        && matches!(
                            diff.target_type.as_str(),
                            "legal_authority" | "provision" | "legal_text"
                        )
                })
                .count() as u64,
            qc_changes: layer_change_count(&layer_diffs, "rule_findings"),
            export_changes: layer_change_count(&layer_diffs, "exports"),
            ai_changes: 0,
            targets_changed: text_diffs
                .iter()
                .filter(|diff| diff.status != "unchanged")
                .map(|diff| VersionTargetSummary {
                    target_type: diff.target_type.clone(),
                    target_id: diff.target_id.clone(),
                    label: Some(diff.title.clone()),
                })
                .chain(
                    layer_diffs
                        .iter()
                        .filter(|diff| diff.status != "unchanged")
                        .map(|diff| VersionTargetSummary {
                            target_type: diff.target_type.clone(),
                            target_id: diff.target_id.clone(),
                            label: Some(diff.title.clone()),
                        }),
                )
                .collect(),
            risk_level: "low".to_string(),
            user_summary: "Compared work-product AST layers.".to_string(),
        };
        Ok(CompareVersionsResponse {
            matter_id: matter_id.to_string(),
            subject_id: work_product_id.to_string(),
            from_snapshot_id: from_snapshot.snapshot_id,
            to_snapshot_id: to_id,
            layers: selected_layers,
            summary,
            text_diffs,
            layer_diffs,
        })
    }

    pub async fn restore_work_product_version(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: RestoreVersionRequest,
    ) -> ApiResult<RestoreVersionResponse> {
        let current = self.get_work_product(matter_id, work_product_id).await?;
        let snapshot = self
            .get_work_product_snapshot(matter_id, work_product_id, &request.snapshot_id)
            .await?;
        let snapshot_product = self.product_from_snapshot(matter_id, &snapshot).await?;
        let scope = request.scope.as_str();
        let dry_run = request.dry_run.unwrap_or(false);
        let target_ids = request.target_ids.unwrap_or_default();
        let (mut restored, warnings) =
            restore_work_product_scope(&current, &snapshot_product, scope, &target_ids)?;
        if dry_run {
            return Ok(RestoreVersionResponse {
                restored: false,
                dry_run: true,
                warnings,
                snapshot_id: snapshot.snapshot_id,
                change_set: None,
                result: Some(restored),
            });
        }
        refresh_work_product_state(&mut restored);
        self.validate_work_product_matter_references(matter_id, &restored)
            .await?;
        let restored = self.save_work_product(matter_id, restored).await?;
        self.sync_complaint_projection_from_work_product(matter_id, &restored)
            .await?;
        let change_set = self
            .record_work_product_change(
                matter_id,
                Some(&current),
                &restored,
                "restore",
                "restore",
                "Version restored",
                &format!("Restored {} from snapshot {}.", scope, snapshot.snapshot_id),
                vec![VersionChangeInput {
                    target_type: scope.to_string(),
                    target_id: work_product_id.to_string(),
                    operation: "restore".to_string(),
                    before: json_value(&current).ok(),
                    after: json_value(&restored).ok(),
                    summary: format!("Restored {scope} from Case History snapshot."),
                    legal_impact: LegalImpactSummary::default(),
                    ai_audit_id: None,
                }],
            )
            .await?;
        Ok(RestoreVersionResponse {
            restored: true,
            dry_run: false,
            warnings,
            snapshot_id: snapshot.snapshot_id,
            change_set: Some(change_set),
            result: Some(restored),
        })
    }

    pub async fn work_product_export_history(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<WorkProductArtifact>> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        let current_hashes = work_product_hashes(&product)?;
        let snapshots_by_id = self
            .list_work_product_snapshots(matter_id, work_product_id)
            .await?
            .into_iter()
            .map(|snapshot| (snapshot.snapshot_id.clone(), snapshot))
            .collect::<BTreeMap<_, _>>();
        let mut artifacts = Vec::new();
        for mut artifact in product.artifacts {
            if let Some(snapshot_id) = artifact.snapshot_id.clone() {
                artifact.changed_since_export = snapshots_by_id
                    .get(&snapshot_id)
                    .map(|snapshot| {
                        snapshot.document_hash != current_hashes.document_hash
                            || snapshot.support_graph_hash != current_hashes.support_graph_hash
                            || snapshot.qc_state_hash != current_hashes.qc_state_hash
                            || snapshot.formatting_hash != current_hashes.formatting_hash
                    })
                    .or(Some(true));
            }
            artifacts.push(artifact);
        }
        Ok(artifacts)
    }

    pub async fn work_product_ai_audit(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<AIEditAudit>> {
        self.get_work_product(matter_id, work_product_id).await?;
        let mut audits = self
            .list_nodes::<AIEditAudit>(matter_id, ai_edit_audit_spec())
            .await?
            .into_iter()
            .filter(|audit| audit.subject_id == work_product_id)
            .collect::<Vec<_>>();
        audits.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(audits)
    }

    async fn apply_snapshot_storage_policy(
        &self,
        matter_id: &str,
        product: &WorkProduct,
        snapshot: &mut VersionSnapshot,
        manifest: &mut SnapshotManifest,
        entity_states: &mut [SnapshotEntityState],
    ) -> ApiResult<()> {
        let full_state = json_value(product)?;
        let full_state_bytes = serde_json::to_vec(&full_state)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        if should_inline_payload(
            full_state_bytes.len(),
            self.ast_storage_policy.snapshot_inline_bytes,
        ) {
            snapshot.full_state_inline = Some(full_state);
            snapshot.full_state_ref = None;
        } else {
            let full_state_hash = sha256_hex(&full_state_bytes);
            let key = snapshot_full_state_key(
                matter_id,
                &product.work_product_id,
                &snapshot.snapshot_id,
                &full_state_hash,
            );
            let blob = self
                .store_casebuilder_bytes(
                    matter_id,
                    &key,
                    Bytes::from(full_state_bytes),
                    "application/json; charset=utf-8",
                )
                .await?;
            snapshot.full_state_inline = None;
            snapshot.full_state_ref = Some(blob.object_blob_id);
        }

        let mut offloaded_any_state = false;
        for state in entity_states.iter_mut() {
            let Some(value) = state.state_inline.clone() else {
                continue;
            };
            let bytes = serde_json::to_vec(&value)
                .map_err(|error| ApiError::Internal(error.to_string()))?;
            if should_inline_payload(bytes.len(), self.ast_storage_policy.entity_inline_bytes) {
                continue;
            }
            let key = snapshot_entity_state_key(
                matter_id,
                &product.work_product_id,
                &snapshot.snapshot_id,
                &state.entity_type,
                &state.entity_hash,
            );
            let blob = self
                .store_casebuilder_bytes(
                    matter_id,
                    &key,
                    Bytes::from(bytes),
                    "application/json; charset=utf-8",
                )
                .await?;
            state.state_inline = None;
            state.state_ref = Some(blob.object_blob_id);
            offloaded_any_state = true;
        }

        let manifest_payload = serde_json::json!({
            "manifest": manifest,
            "entity_states": entity_states.iter().map(|state| serde_json::json!({
                "entity_state_id": state.entity_state_id,
                "entity_type": state.entity_type,
                "entity_id": state.entity_id,
                "entity_hash": state.entity_hash,
                "state_ref": state.state_ref,
                "inline": state.state_inline.is_some(),
            })).collect::<Vec<_>>(),
        });
        let manifest_bytes = serde_json::to_vec(&manifest_payload)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        if offloaded_any_state
            || !should_inline_payload(
                manifest_bytes.len(),
                self.ast_storage_policy.entity_inline_bytes,
            )
        {
            let key = snapshot_manifest_key(
                matter_id,
                &product.work_product_id,
                &snapshot.snapshot_id,
                &manifest.manifest_hash,
            );
            let blob = self
                .store_casebuilder_bytes(
                    matter_id,
                    &key,
                    Bytes::from(manifest_bytes),
                    "application/json; charset=utf-8",
                )
                .await?;
            manifest.storage_ref = Some(blob.object_blob_id);
            snapshot.manifest_ref = manifest.storage_ref.clone();
        }
        Ok(())
    }

    async fn product_from_snapshot(
        &self,
        matter_id: &str,
        snapshot: &VersionSnapshot,
    ) -> ApiResult<WorkProduct> {
        if let Some(value) = snapshot.full_state_inline.clone() {
            return serde_json::from_value(value)
                .map_err(|error| ApiError::Internal(error.to_string()));
        }
        let Some(blob_id) = snapshot.full_state_ref.as_deref() else {
            return Err(ApiError::Internal("Snapshot has no state".to_string()));
        };
        self.load_json_blob(matter_id, blob_id).await
    }

    async fn hydrate_snapshot_full_state(
        &self,
        matter_id: &str,
        snapshot: &mut VersionSnapshot,
    ) -> ApiResult<()> {
        if snapshot.full_state_inline.is_some() {
            return Ok(());
        }
        let Some(blob_id) = snapshot.full_state_ref.as_deref() else {
            return Ok(());
        };
        snapshot.full_state_inline = Some(self.load_json_blob(matter_id, blob_id).await?);
        Ok(())
    }

    async fn list_work_product_change_sets(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<ChangeSet>> {
        let mut change_sets = self
            .list_nodes::<ChangeSet>(matter_id, change_set_spec())
            .await?
            .into_iter()
            .filter(|change_set| change_set.subject_id == work_product_id)
            .collect::<Vec<_>>();
        change_sets.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(change_sets)
    }

    async fn record_work_product_change(
        &self,
        matter_id: &str,
        before_product: Option<&WorkProduct>,
        after_product: &WorkProduct,
        source: &str,
        snapshot_type: &str,
        title: &str,
        summary: &str,
        changes: Vec<VersionChangeInput>,
    ) -> ApiResult<ChangeSet> {
        let now = now_string();
        let branch_id = format!("{}:branch:main", after_product.work_product_id);
        let existing_branch = self
            .list_nodes::<VersionBranch>(matter_id, version_branch_spec())
            .await
            .unwrap_or_default()
            .into_iter()
            .find(|branch| {
                branch.subject_id == after_product.work_product_id && branch.branch_type == "main"
            });
        let parent_snapshot_id = existing_branch.as_ref().and_then(|branch| {
            if branch.current_snapshot_id.is_empty() {
                None
            } else {
                Some(branch.current_snapshot_id.clone())
            }
        });
        let sequence_number = self
            .list_work_product_snapshots(matter_id, &after_product.work_product_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|snapshot| snapshot.sequence_number)
            .max()
            .unwrap_or(0)
            + 1;
        let snapshot_id = format!(
            "{}:snapshot:{}",
            after_product.work_product_id, sequence_number
        );
        let change_set_id = format!(
            "{}:changeset:{}",
            after_product.work_product_id, sequence_number
        );
        let mut change_ids = Vec::new();
        let mut recorded_changes = Vec::new();
        for (index, input) in changes.into_iter().enumerate() {
            let change_id = format!("{change_set_id}:change:{}", index + 1);
            let (before_hash, before) = version_change_state_summary(input.before)?;
            let (after_hash, after) = version_change_state_summary(input.after)?;
            change_ids.push(change_id.clone());
            recorded_changes.push(VersionChange {
                id: change_id.clone(),
                change_id,
                change_set_id: change_set_id.clone(),
                snapshot_id: snapshot_id.clone(),
                matter_id: matter_id.to_string(),
                subject_type: "work_product".to_string(),
                subject_id: after_product.work_product_id.clone(),
                branch_id: branch_id.clone(),
                target_type: input.target_type,
                target_id: input.target_id,
                operation: input.operation,
                before_hash,
                after_hash,
                before,
                after,
                summary: input.summary,
                legal_impact: input.legal_impact,
                ai_audit_id: input.ai_audit_id,
                created_at: now.clone(),
                created_by: if source == "ai" { "ai" } else { "system" }.to_string(),
                actor_id: None,
            });
        }
        let legal_impact =
            merge_legal_impacts(recorded_changes.iter().map(|change| &change.legal_impact));
        let version_summary = version_summary_for_changes(summary, &recorded_changes);
        let hashes = work_product_hashes(after_product)?;
        let (mut manifest, mut entity_states) =
            snapshot_manifest_for_product(matter_id, &snapshot_id, after_product, &now)?;
        let mut snapshot = VersionSnapshot {
            id: snapshot_id.clone(),
            snapshot_id: snapshot_id.clone(),
            matter_id: matter_id.to_string(),
            subject_type: "work_product".to_string(),
            subject_id: after_product.work_product_id.clone(),
            product_type: after_product.product_type.clone(),
            profile_id: after_product.profile.profile_id.clone(),
            branch_id: branch_id.clone(),
            sequence_number,
            title: title.to_string(),
            message: Some(summary.to_string()),
            created_at: now.clone(),
            created_by: if source == "ai" { "ai" } else { "system" }.to_string(),
            actor_id: None,
            snapshot_type: snapshot_type.to_string(),
            parent_snapshot_ids: parent_snapshot_id.clone().into_iter().collect(),
            document_hash: hashes.document_hash,
            support_graph_hash: hashes.support_graph_hash,
            qc_state_hash: hashes.qc_state_hash,
            formatting_hash: hashes.formatting_hash,
            manifest_hash: manifest.manifest_hash.clone(),
            manifest_ref: None,
            full_state_ref: None,
            full_state_inline: None,
            summary: version_summary,
        };
        self.apply_snapshot_storage_policy(
            matter_id,
            after_product,
            &mut snapshot,
            &mut manifest,
            &mut entity_states,
        )
        .await?;
        let branch = VersionBranch {
            id: branch_id.clone(),
            branch_id: branch_id.clone(),
            matter_id: matter_id.to_string(),
            subject_type: "work_product".to_string(),
            subject_id: after_product.work_product_id.clone(),
            name: if after_product.product_type == "complaint" {
                "Main Complaint".to_string()
            } else {
                format!(
                    "Main {}",
                    humanize_product_type(&after_product.product_type)
                )
            },
            description: None,
            created_from_snapshot_id: existing_branch
                .as_ref()
                .map(|branch| branch.created_from_snapshot_id.clone())
                .unwrap_or_else(|| snapshot_id.clone()),
            current_snapshot_id: snapshot_id.clone(),
            branch_type: "main".to_string(),
            created_at: existing_branch
                .as_ref()
                .map(|branch| branch.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now.clone(),
            archived_at: None,
        };
        let change_set = ChangeSet {
            id: change_set_id.clone(),
            change_set_id: change_set_id.clone(),
            matter_id: matter_id.to_string(),
            subject_id: after_product.work_product_id.clone(),
            branch_id: branch_id.clone(),
            snapshot_id: snapshot_id.clone(),
            parent_snapshot_id,
            title: title.to_string(),
            summary: summary.to_string(),
            reason: None,
            actor_type: if source == "ai" { "ai" } else { "system" }.to_string(),
            actor_id: None,
            source: source.to_string(),
            created_at: now,
            change_ids,
            legal_impact,
        };

        self.merge_node(matter_id, version_branch_spec(), &branch.branch_id, &branch)
            .await?;
        self.merge_node(
            matter_id,
            version_snapshot_spec(),
            &snapshot.snapshot_id,
            &snapshot,
        )
        .await?;
        self.merge_node(
            matter_id,
            snapshot_manifest_spec(),
            &manifest.manifest_id,
            &manifest,
        )
        .await?;
        for state in &entity_states {
            self.merge_node(
                matter_id,
                snapshot_entity_state_spec(),
                &state.entity_state_id,
                state,
            )
            .await?;
        }
        for change in &recorded_changes {
            self.merge_node(matter_id, version_change_spec(), &change.change_id, change)
                .await?;
        }
        self.merge_node(
            matter_id,
            change_set_spec(),
            &change_set.change_set_id,
            &change_set,
        )
        .await?;
        self.materialize_case_history_edges(
            &after_product.work_product_id,
            &branch,
            &snapshot,
            &manifest,
            &entity_states,
            &recorded_changes,
            &change_set,
        )
        .await?;
        if before_product.is_none() {
            self.materialize_version_subject(after_product).await?;
        }
        Ok(change_set)
    }

    async fn save_work_product(
        &self,
        matter_id: &str,
        mut product: WorkProduct,
    ) -> ApiResult<WorkProduct> {
        product.updated_at = now_string();
        refresh_work_product_state(&mut product);
        self.save_work_product_internal(matter_id, product).await
    }

    async fn save_work_product_internal(
        &self,
        matter_id: &str,
        mut product: WorkProduct,
    ) -> ApiResult<WorkProduct> {
        refresh_work_product_state(&mut product);
        let product = self
            .merge_node(
                matter_id,
                work_product_spec(),
                &product.work_product_id,
                &product,
            )
            .await?;
        self.materialize_work_product_edges(&product).await?;
        for finding in &product.findings {
            self.merge_node(
                matter_id,
                work_product_finding_spec(),
                &finding.finding_id,
                finding,
            )
            .await?;
        }
        for artifact in &product.artifacts {
            self.merge_node(
                matter_id,
                work_product_artifact_spec(),
                &artifact.artifact_id,
                artifact,
            )
            .await?;
            if let Some(object_blob_id) = artifact.object_blob_id.as_deref() {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (a:WorkProductArtifact {artifact_id: $artifact_id})
                             MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                             MERGE (a)-[:STORED_AS]->(b)",
                        )
                        .param("artifact_id", artifact.artifact_id.clone())
                        .param("object_blob_id", object_blob_id.to_string()),
                    )
                    .await?;
            }
        }
        Ok(product)
    }

    async fn migrate_legacy_drafts_to_work_products(&self, matter_id: &str) -> ApiResult<()> {
        let drafts = self
            .list_nodes::<CaseDraft>(matter_id, draft_spec())
            .await
            .unwrap_or_default();
        for draft in drafts {
            if draft.kind == "complaint" || draft.draft_type == "complaint" {
                continue;
            }
            match self
                .get_node::<WorkProduct>(matter_id, work_product_spec(), &draft.draft_id)
                .await
            {
                Ok(_) => continue,
                Err(ApiError::NotFound(_)) => {
                    let product = work_product_from_draft(&draft);
                    self.save_work_product_internal(matter_id, product).await?;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    async fn migrate_complaints_to_work_products(&self, matter_id: &str) -> ApiResult<()> {
        let complaints = self
            .list_nodes::<ComplaintDraft>(matter_id, complaint_spec())
            .await
            .unwrap_or_default();
        for complaint in complaints {
            match self
                .get_node::<WorkProduct>(matter_id, work_product_spec(), &complaint.complaint_id)
                .await
            {
                Ok(_) => continue,
                Err(ApiError::NotFound(_)) => {
                    let product = work_product_from_complaint(&complaint);
                    self.save_work_product_internal(matter_id, product).await?;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub async fn create_draft(
        &self,
        matter_id: &str,
        request: CreateDraftRequest,
    ) -> ApiResult<CaseDraft> {
        let id = generate_id("draft", &request.title);
        let kind = request
            .draft_type
            .clone()
            .unwrap_or_else(|| "legal_memo".to_string());
        if kind == "complaint" {
            return Err(ApiError::BadRequest(
                "Complaint drafting now uses the structured /complaints API, not generic drafts."
                    .to_string(),
            ));
        }
        let product = self
            .create_work_product_with_id(
                matter_id,
                &id,
                CreateWorkProductRequest {
                    title: Some(request.title),
                    product_type: kind,
                    template: None,
                    source_draft_id: Some(id.clone()),
                    source_complaint_id: None,
                },
            )
            .await?;
        let mut draft = work_product_to_draft(&product);
        draft.description = request.description.unwrap_or_default();
        if let Some(status) = request.status {
            draft.status = status;
        }
        Ok(draft)
    }

    pub async fn list_drafts(&self, matter_id: &str) -> ApiResult<Vec<CaseDraft>> {
        Ok(self
            .list_work_products(matter_id)
            .await?
            .into_iter()
            .filter(|product| product.product_type != "complaint")
            .map(|product| work_product_to_draft(&product))
            .collect())
    }

    pub async fn get_draft(&self, matter_id: &str, draft_id: &str) -> ApiResult<CaseDraft> {
        Ok(work_product_to_draft(
            &self.get_work_product(matter_id, draft_id).await?,
        ))
    }

    pub async fn patch_draft(
        &self,
        matter_id: &str,
        draft_id: &str,
        request: PatchDraftRequest,
    ) -> ApiResult<CaseDraft> {
        let mut product = self.get_work_product(matter_id, draft_id).await?;
        let mut draft = work_product_to_draft(&product);
        if let Some(value) = request.title {
            draft.title = value;
        }
        if let Some(value) = request.description {
            draft.description = value;
        }
        if let Some(value) = request.status {
            draft.status = value;
        }
        if let Some(value) = request.sections {
            draft.sections = value;
        }
        if let Some(value) = request.paragraphs {
            draft.paragraphs = value;
        }
        draft.word_count = count_words(&draft.paragraphs, &draft.sections);
        draft.updated_at = now_string();
        product.title = draft.title.clone();
        product.status = draft.status.clone();
        product.blocks = work_product_blocks_from_draft(&draft);
        product.history.push(work_product_event(
            matter_id,
            draft_id,
            "legacy_draft_patch",
            "draft",
            draft_id,
            "Deprecated draft wrapper patched the shared WorkProduct AST.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        Ok(work_product_to_draft(&product))
    }

    pub async fn generate_draft(
        &self,
        matter_id: &str,
        draft_id: &str,
    ) -> ApiResult<AiActionResponse<CaseDraft>> {
        let existing = self.get_work_product(matter_id, draft_id).await?;
        let matter = self.get_matter_summary(matter_id).await?;
        let facts = self.list_facts(matter_id).await?;
        let claims = self.list_claims(matter_id).await?;
        let mut product = default_work_product_from_matter(
            &matter,
            draft_id,
            &existing.title,
            &existing.product_type,
            &facts,
            &claims,
            &now_string(),
        );
        product.created_at = existing.created_at;
        product.source_draft_id = existing.source_draft_id.or(Some(draft_id.to_string()));
        product.history = existing.history;
        product.history.push(work_product_event(
            matter_id,
            draft_id,
            "legacy_draft_generated",
            "draft",
            draft_id,
            "Deprecated draft wrapper regenerated the shared WorkProduct AST.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;

        Ok(AiActionResponse {
            enabled: false,
            mode: "template".to_string(),
            message: "No live drafting provider is configured; generated a deterministic source-linked draft scaffold.".to_string(),
            result: Some(work_product_to_draft(&product)),
        })
    }

    pub async fn fact_check_draft(
        &self,
        matter_id: &str,
        draft_id: &str,
    ) -> ApiResult<AiActionResponse<Vec<FactCheckFinding>>> {
        let draft = self.get_draft(matter_id, draft_id).await?;
        let mut findings = Vec::new();
        for paragraph in &draft.paragraphs {
            if paragraph.factcheck_status == "needs_evidence" {
                let finding_id = generate_id("factcheck", &paragraph.paragraph_id);
                findings.push(FactCheckFinding {
                    id: finding_id.clone(),
                    finding_id,
                    matter_id: matter_id.to_string(),
                    draft_id: draft_id.to_string(),
                    paragraph_id: Some(paragraph.paragraph_id.clone()),
                    finding_type: "unsupported_fact".to_string(),
                    severity: "warning".to_string(),
                    message: "Paragraph has factual text without linked evidence.".to_string(),
                    source_fact_ids: paragraph.fact_ids.clone(),
                    source_evidence_ids: paragraph.evidence_ids.clone(),
                    status: "open".to_string(),
                });
            }
        }
        for finding in &findings {
            self.merge_node(
                matter_id,
                fact_check_finding_spec(),
                &finding.finding_id,
                finding,
            )
            .await?;
        }
        Ok(AiActionResponse {
            enabled: false,
            mode: "deterministic".to_string(),
            message:
                "No live fact-checking provider is configured; ran deterministic support checks."
                    .to_string(),
            result: Some(findings),
        })
    }

    pub async fn citation_check_draft(
        &self,
        matter_id: &str,
        draft_id: &str,
    ) -> ApiResult<AiActionResponse<Vec<CitationCheckFinding>>> {
        let draft = self.get_draft(matter_id, draft_id).await?;
        let mut findings = Vec::new();
        for paragraph in &draft.paragraphs {
            if paragraph.role == "legal_claim" && paragraph.authorities.is_empty() {
                let finding_id = generate_id("citecheck", &paragraph.paragraph_id);
                findings.push(CitationCheckFinding {
                    id: finding_id.clone(),
                    finding_id,
                    matter_id: matter_id.to_string(),
                    draft_id: draft_id.to_string(),
                    citation: String::new(),
                    canonical_id: None,
                    finding_type: "missing_citation".to_string(),
                    severity: "warning".to_string(),
                    message: "Legal claim paragraph has no linked authority.".to_string(),
                    status: "open".to_string(),
                });
            }
        }
        for finding in &findings {
            self.merge_node(
                matter_id,
                citation_check_finding_spec(),
                &finding.finding_id,
                finding,
            )
            .await?;
        }
        Ok(AiActionResponse {
            enabled: false,
            mode: "deterministic".to_string(),
            message:
                "No live citation-checking provider is configured; ran missing-authority checks."
                    .to_string(),
            result: Some(findings),
        })
    }

    pub async fn list_complaints(&self, matter_id: &str) -> ApiResult<Vec<ComplaintDraft>> {
        self.list_nodes(matter_id, complaint_spec()).await
    }

    pub async fn create_complaint(
        &self,
        matter_id: &str,
        request: CreateComplaintRequest,
    ) -> ApiResult<ComplaintDraft> {
        let matter = self.get_matter_summary(matter_id).await?;
        let parties = self.list_parties(matter_id).await.unwrap_or_default();
        let claims = self.list_claims(matter_id).await.unwrap_or_default();
        let facts = self.list_facts(matter_id).await.unwrap_or_default();
        let now = now_string();
        let title = request.title.unwrap_or_else(|| {
            let base = matter.short_name.clone().unwrap_or(matter.name.clone());
            format!("{base} complaint")
        });
        let complaint_id = generate_id("complaint", &title);
        let mut complaint = default_complaint_from_matter(
            &matter,
            &complaint_id,
            &title,
            &parties,
            &claims,
            &facts,
            &now,
        );
        if let Some(source_draft_id) = request.source_draft_id {
            complaint.history.push(complaint_event(
                matter_id,
                &complaint_id,
                "source_draft_linked",
                "draft",
                &source_draft_id,
                "Complaint initialized with a generic draft source.",
            ));
        }
        if let Some(template) = request.template {
            complaint.history.push(complaint_event(
                matter_id,
                &complaint_id,
                "template_selected",
                "complaint",
                &complaint_id,
                &format!("Template selected: {template}."),
            ));
        }
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn import_complaints(
        &self,
        matter_id: &str,
        request: ComplaintImportRequest,
    ) -> ApiResult<ComplaintImportResponse> {
        let mut document_ids = request.document_ids.clone();
        if let Some(document_id) = request.document_id.clone() {
            push_unique(&mut document_ids, document_id);
        }
        if document_ids.is_empty() {
            return Err(ApiError::BadRequest(
                "Complaint import requires at least one document_id".to_string(),
            ));
        }

        let mode = request
            .mode
            .clone()
            .unwrap_or_else(|| "structured_import".to_string());
        let mut imported = Vec::new();
        let mut skipped = Vec::new();
        let mut warnings = Vec::new();

        for document_id in document_ids {
            let title = request.title.clone();
            match self
                .import_complaint_from_document(
                    matter_id,
                    &document_id,
                    ComplaintImportRequest {
                        document_id: Some(document_id.clone()),
                        document_ids: Vec::new(),
                        title,
                        force: request.force,
                        mode: Some(mode.clone()),
                    },
                )
                .await
            {
                Ok(response) => {
                    imported.extend(response.imported);
                    skipped.extend(response.skipped);
                    warnings.extend(response.warnings);
                }
                Err(error) => skipped.push(ComplaintImportResult {
                    document_id,
                    complaint_id: None,
                    status: "failed".to_string(),
                    message: error.to_string(),
                    parser_id: "casebuilder-import-dispatch".to_string(),
                    likely_complaint: false,
                    complaint: None,
                }),
            }
        }

        Ok(ComplaintImportResponse {
            matter_id: matter_id.to_string(),
            mode,
            imported,
            skipped,
            warnings,
        })
    }

    pub async fn import_complaint_from_document(
        &self,
        matter_id: &str,
        document_id: &str,
        request: ComplaintImportRequest,
    ) -> ApiResult<ComplaintImportResponse> {
        let matter = self.get_matter_summary(matter_id).await?;
        let mut document = self.get_document(matter_id, document_id).await?;
        let provenance = self
            .ensure_document_original_provenance(matter_id, &mut document)
            .await?;
        let text = match document.extracted_text.clone() {
            Some(text) if !text.trim().is_empty() => text,
            _ => self.document_bytes_as_text(&document).await?,
        };
        let parser_id = parser_id_for_document(&document);
        let likely_complaint = looks_like_complaint(&document.filename, &text);
        let force = request.force.unwrap_or(false);
        let mode = request
            .mode
            .clone()
            .unwrap_or_else(|| "structured_import".to_string());

        if text.trim().is_empty() {
            return Ok(ComplaintImportResponse {
                matter_id: matter_id.to_string(),
                mode,
                imported: Vec::new(),
                skipped: vec![ComplaintImportResult {
                    document_id: document_id.to_string(),
                    complaint_id: None,
                    status: "no_extractable_text".to_string(),
                    message: "This document has no deterministic text to import yet.".to_string(),
                    parser_id,
                    likely_complaint,
                    complaint: None,
                }],
                warnings: vec!["OCR/transcription or a supported text parser is required before structured complaint import.".to_string()],
            });
        }
        if !likely_complaint && !force {
            return Ok(ComplaintImportResponse {
                matter_id: matter_id.to_string(),
                mode,
                imported: Vec::new(),
                skipped: vec![ComplaintImportResult {
                    document_id: document_id.to_string(),
                    complaint_id: None,
                    status: "not_likely_complaint".to_string(),
                    message: "Document did not meet the complaint-draft detection threshold."
                        .to_string(),
                    parser_id,
                    likely_complaint,
                    complaint: None,
                }],
                warnings: Vec::new(),
            });
        }

        let source_context = source_context_from_provenance(provenance.as_ref());
        let parties = self.list_parties(matter_id).await.unwrap_or_default();
        let claims = self.list_claims(matter_id).await.unwrap_or_default();
        let facts = self.list_facts(matter_id).await.unwrap_or_default();
        let evidence = self.list_evidence(matter_id).await.unwrap_or_default();
        let now = now_string();
        let title = request
            .title
            .clone()
            .unwrap_or_else(|| imported_complaint_title(&document, &text));
        let complaint_id = generate_id("complaint", &format!("{}:{}", document_id, title));
        let mut complaint = build_imported_complaint(
            &matter,
            &document,
            &complaint_id,
            &title,
            &text,
            &parser_id,
            &source_context,
            &parties,
            &claims,
            &facts,
            &evidence,
            &now,
        );
        let manifest_key = format!(
            "casebuilder/documents/{}/artifacts/complaint-import-{}.json",
            sanitize_path_segment(document_id),
            sanitize_path_segment(&complaint_id)
        );
        let manifest = serde_json::json!({
            "matter_id": matter_id,
            "document_id": document_id,
            "complaint_id": complaint_id,
            "parser_id": parser_id,
            "parser_version": PARSER_REGISTRY_VERSION,
            "chunker_version": CHUNKER_VERSION,
            "citation_resolver_version": CITATION_RESOLVER_VERSION,
            "index_version": CASE_INDEX_VERSION,
            "title": title,
            "paragraph_count": complaint.paragraphs.len(),
            "count_count": complaint.counts.len(),
            "citation_count": complaint.paragraphs.iter().map(|p| p.citation_uses.len()).sum::<usize>(),
        });
        let manifest_bytes =
            serde_json::to_vec(&manifest).map_err(|error| ApiError::Internal(error.to_string()))?;
        let manifest_hash = sha256_hex(&manifest_bytes);
        let stored_manifest = self
            .object_store
            .put_bytes(
                &manifest_key,
                Bytes::from(manifest_bytes),
                put_options(
                    Some("application/json".to_string()),
                    Some(manifest_hash.clone()),
                ),
            )
            .await?;

        let mut source_spans = Vec::new();
        for paragraph in &complaint.paragraphs {
            if let Some(provenance) = &paragraph.import_provenance {
                if let Some(source_span_id) = &provenance.source_span_id {
                    source_spans.push(SourceSpan {
                        source_span_id: source_span_id.clone(),
                        id: source_span_id.clone(),
                        matter_id: matter_id.to_string(),
                        document_id: document_id.to_string(),
                        document_version_id: provenance.document_version_id.clone(),
                        object_blob_id: provenance.object_blob_id.clone(),
                        ingestion_run_id: provenance.ingestion_run_id.clone(),
                        page: Some(1),
                        chunk_id: None,
                        byte_start: provenance.byte_start,
                        byte_end: provenance.byte_end,
                        char_start: provenance.char_start,
                        char_end: provenance.char_end,
                        quote: Some(paragraph.text.clone()),
                        extraction_method: "complaint_import_paragraph".to_string(),
                        confidence: 0.82,
                        review_status: "unreviewed".to_string(),
                        unavailable_reason: None,
                    });
                }
            }
        }
        for span in &source_spans {
            self.merge_source_span(matter_id, span).await?;
        }
        document.extracted_text = Some(text.clone());
        document.processing_status = "review_ready".to_string();
        document.summary = format!(
            "Imported as structured complaint {} with {} paragraphs and {} citation uses.",
            complaint.complaint_id,
            complaint.paragraphs.len(),
            complaint
                .paragraphs
                .iter()
                .map(|paragraph| paragraph.citation_uses.len())
                .sum::<usize>()
        );
        document.citations_found = complaint
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.citation_uses.len() as u64)
            .sum();
        document.source_spans = source_spans.clone();
        let document = self
            .merge_node(matter_id, document_spec(), document_id, &document)
            .await?;

        let mut run = provenance
            .as_ref()
            .map(|value| {
                completed_ingestion_run(
                    &value.ingestion_run,
                    "review_ready",
                    "complaint_import",
                    complaint_import_node_ids(&complaint, &source_spans),
                )
            })
            .unwrap_or_else(|| IngestionRun {
                ingestion_run_id: primary_ingestion_run_id(document_id),
                id: primary_ingestion_run_id(document_id),
                matter_id: matter_id.to_string(),
                document_id: document_id.to_string(),
                document_version_id: None,
                object_blob_id: None,
                input_sha256: document.file_hash.clone(),
                status: "review_ready".to_string(),
                stage: "complaint_import".to_string(),
                mode: "deterministic".to_string(),
                started_at: now.clone(),
                completed_at: Some(now_string()),
                error_code: None,
                error_message: None,
                retryable: false,
                produced_node_ids: complaint_import_node_ids(&complaint, &source_spans),
                produced_object_keys: Vec::new(),
                parser_id: Some(parser_id.clone()),
                parser_version: Some(PARSER_REGISTRY_VERSION.to_string()),
                chunker_version: Some(CHUNKER_VERSION.to_string()),
                citation_resolver_version: Some(CITATION_RESOLVER_VERSION.to_string()),
                index_version: Some(CASE_INDEX_VERSION.to_string()),
            });
        run.parser_id = Some(parser_id.clone());
        run.parser_version = Some(PARSER_REGISTRY_VERSION.to_string());
        run.chunker_version = Some(CHUNKER_VERSION.to_string());
        run.citation_resolver_version = Some(CITATION_RESOLVER_VERSION.to_string());
        run.index_version = Some(CASE_INDEX_VERSION.to_string());
        push_unique(&mut run.produced_object_keys, stored_manifest.key.clone());
        self.merge_ingestion_run(matter_id, &run).await?;

        complaint.history.push(complaint_event(
            matter_id,
            &complaint.complaint_id,
            "complaint_imported",
            "document",
            document_id,
            &format!(
                "Structured complaint imported from uploaded document; manifest stored at {}.",
                stored_manifest.key
            ),
        ));
        refresh_complaint_state(&mut complaint);
        let complaint = self.save_complaint(matter_id, complaint).await?;

        Ok(ComplaintImportResponse {
            matter_id: matter_id.to_string(),
            mode,
            imported: vec![ComplaintImportResult {
                document_id: document.document_id,
                complaint_id: Some(complaint.complaint_id.clone()),
                status: "imported".to_string(),
                message: "Structured complaint import completed; human review is required."
                    .to_string(),
                parser_id,
                likely_complaint,
                complaint: Some(complaint),
            }],
            skipped: Vec::new(),
            warnings: Vec::new(),
        })
    }

    pub async fn get_complaint(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<ComplaintDraft> {
        self.get_node(matter_id, complaint_spec(), complaint_id)
            .await
    }

    pub async fn patch_complaint(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: PatchComplaintRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        if let Some(value) = request.title {
            complaint.title = value;
        }
        if let Some(value) = request.status {
            complaint.status = value;
        }
        if let Some(value) = request.review_status {
            complaint.review_status = value;
        }
        if let Some(value) = request.setup_stage {
            complaint.setup_stage = value;
        }
        if let Some(value) = request.caption {
            complaint.caption = value;
        }
        if let Some(value) = request.parties {
            complaint.parties = value;
        }
        if let Some(value) = request.sections {
            complaint.sections = value;
        }
        if let Some(value) = request.counts {
            complaint.counts = value;
        }
        if let Some(value) = request.paragraphs {
            complaint.paragraphs = value;
        }
        if let Some(value) = request.relief {
            complaint.relief = value;
        }
        if let Some(value) = request.signature {
            complaint.signature = value;
        }
        if let Some(value) = request.certificate_of_service {
            complaint.certificate_of_service = value;
        }
        if let Some(value) = request.formatting_profile {
            complaint.formatting_profile = value;
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "complaint_updated",
            "complaint",
            complaint_id,
            "Complaint metadata or AST was updated.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn update_complaint_setup(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: PatchComplaintRequest,
    ) -> ApiResult<ComplaintDraft> {
        self.patch_complaint(matter_id, complaint_id, request).await
    }

    pub async fn create_complaint_section(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: CreateComplaintSectionRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let section_id = format!(
            "{complaint_id}:section:{}",
            sanitize_path_segment(&request.title)
        );
        if !complaint
            .sections
            .iter()
            .any(|section| section.section_id == section_id)
        {
            complaint.sections.push(ComplaintSection {
                id: section_id.clone(),
                section_id: section_id.clone(),
                matter_id: matter_id.to_string(),
                complaint_id: complaint_id.to_string(),
                title: request.title,
                section_type: request.section_type.unwrap_or_else(|| "custom".to_string()),
                ordinal: complaint.sections.len() as u64 + 1,
                paragraph_ids: Vec::new(),
                count_ids: Vec::new(),
                review_status: "needs_review".to_string(),
            });
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "section_created",
            "section",
            &section_id,
            "Complaint section created.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn create_complaint_count(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: CreateComplaintCountRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let count_id = format!("{complaint_id}:count:{}", complaint.counts.len() + 1);
        let fact_ids = request
            .claim_id
            .as_ref()
            .and_then(|claim_id| {
                complaint
                    .counts
                    .iter()
                    .find(|count| count.claim_id.as_ref() == Some(claim_id))
                    .map(|count| count.fact_ids.clone())
            })
            .unwrap_or_default();
        complaint.counts.push(ComplaintCount {
            id: count_id.clone(),
            count_id: count_id.clone(),
            matter_id: matter_id.to_string(),
            complaint_id: complaint_id.to_string(),
            ordinal: complaint.counts.len() as u64 + 1,
            title: request.title.clone(),
            claim_id: request.claim_id,
            legal_theory: request.legal_theory.unwrap_or_default(),
            against_party_ids: request.against_party_ids.unwrap_or_default(),
            element_ids: request.element_ids.unwrap_or_default(),
            fact_ids,
            evidence_ids: Vec::new(),
            authorities: Vec::new(),
            relief_ids: request.relief_ids.unwrap_or_default(),
            paragraph_ids: Vec::new(),
            incorporation_range: Some("1 through preceding paragraph".to_string()),
            health: "needs_review".to_string(),
            weaknesses: Vec::new(),
        });
        let paragraph_text = format!("COUNT {} - {}", complaint.counts.len(), request.title);
        let paragraph_id = format!(
            "{complaint_id}:paragraph:{}",
            complaint.paragraphs.len() + 1
        );
        complaint.paragraphs.push(pleading_paragraph(
            matter_id,
            complaint_id,
            &paragraph_id,
            None,
            Some(count_id.clone()),
            "count_heading",
            &paragraph_text,
            complaint.paragraphs.len() as u64 + 1,
            Vec::new(),
            Vec::new(),
        ));
        if let Some(count) = complaint
            .counts
            .iter_mut()
            .find(|count| count.count_id == count_id)
        {
            count.paragraph_ids.push(paragraph_id.clone());
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "count_created",
            "count",
            &count_id,
            "Complaint count created.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn create_complaint_paragraph(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: CreateComplaintParagraphRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let paragraph_id = format!(
            "{complaint_id}:paragraph:{}",
            complaint.paragraphs.len() + 1
        );
        let paragraph = pleading_paragraph(
            matter_id,
            complaint_id,
            &paragraph_id,
            request.section_id.clone(),
            request.count_id.clone(),
            request.role.as_deref().unwrap_or("factual_allegation"),
            &request.text,
            complaint.paragraphs.len() as u64 + 1,
            request.fact_ids.unwrap_or_default(),
            request.evidence_ids.unwrap_or_default(),
        );
        if let Some(section_id) = &request.section_id {
            if let Some(section) = complaint
                .sections
                .iter_mut()
                .find(|section| &section.section_id == section_id)
            {
                push_unique(&mut section.paragraph_ids, paragraph_id.clone());
            }
        }
        if let Some(count_id) = &request.count_id {
            if let Some(count) = complaint
                .counts
                .iter_mut()
                .find(|count| &count.count_id == count_id)
            {
                push_unique(&mut count.paragraph_ids, paragraph_id.clone());
            }
        }
        complaint.paragraphs.push(paragraph);
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "paragraph_created",
            "paragraph",
            &paragraph_id,
            "Pleading paragraph created.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn patch_complaint_paragraph(
        &self,
        matter_id: &str,
        complaint_id: &str,
        paragraph_id: &str,
        request: PatchComplaintParagraphRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let paragraph = complaint
            .paragraphs
            .iter_mut()
            .find(|paragraph| paragraph.paragraph_id == paragraph_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("Pleading paragraph {paragraph_id} not found"))
            })?;
        if paragraph.locked && request.text.is_some() {
            return Err(ApiError::BadRequest(format!(
                "Pleading paragraph {paragraph_id} is locked"
            )));
        }
        if let Some(value) = request.section_id {
            paragraph.section_id = Some(value);
        }
        if let Some(value) = request.count_id {
            paragraph.count_id = Some(value);
        }
        if let Some(value) = request.role {
            paragraph.role = value;
        }
        if let Some(value) = request.text {
            paragraph.text = value;
            paragraph.sentences = pleading_sentences(
                matter_id,
                complaint_id,
                paragraph_id,
                &paragraph.text,
                &paragraph.fact_ids,
            );
        }
        if let Some(value) = request.fact_ids {
            paragraph.fact_ids = value;
            paragraph.sentences = pleading_sentences(
                matter_id,
                complaint_id,
                paragraph_id,
                &paragraph.text,
                &paragraph.fact_ids,
            );
        }
        if let Some(value) = request.evidence_uses {
            paragraph.evidence_uses = value;
        }
        if let Some(value) = request.citation_uses {
            paragraph.citation_uses = value;
        }
        if let Some(value) = request.exhibit_references {
            paragraph.exhibit_references = value;
        }
        if let Some(value) = request.locked {
            paragraph.locked = value;
        }
        if let Some(value) = request.review_status {
            paragraph.review_status = value;
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "paragraph_updated",
            "paragraph",
            paragraph_id,
            "Pleading paragraph updated.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn renumber_complaint_paragraphs(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        renumber_paragraphs(&mut complaint.paragraphs);
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "paragraphs_renumbered",
            "complaint",
            complaint_id,
            "Pleading paragraphs renumbered without changing stable IDs.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn link_complaint_support(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: ComplaintLinkRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        self.validate_complaint_link_references(matter_id, &request)
            .await?;
        match request.target_type.as_str() {
            "paragraph" | "sentence" => {
                let paragraph_id = if request.target_type == "paragraph" {
                    request.target_id.clone()
                } else {
                    complaint
                        .paragraphs
                        .iter()
                        .find(|paragraph| {
                            paragraph
                                .sentences
                                .iter()
                                .any(|sentence| sentence.sentence_id == request.target_id)
                        })
                        .map(|paragraph| paragraph.paragraph_id.clone())
                        .ok_or_else(|| {
                            ApiError::NotFound(format!(
                                "Complaint target {} not found",
                                request.target_id
                            ))
                        })?
                };
                let paragraph = complaint
                    .paragraphs
                    .iter_mut()
                    .find(|paragraph| paragraph.paragraph_id == paragraph_id)
                    .ok_or_else(|| {
                        ApiError::NotFound(format!("Pleading paragraph {paragraph_id} not found"))
                    })?;
                if let Some(fact_id) = request.fact_id.clone() {
                    push_unique(&mut paragraph.fact_ids, fact_id);
                }
                if request.evidence_id.is_some()
                    || request.document_id.is_some()
                    || request.source_span_id.is_some()
                {
                    let id = format!(
                        "{}:evidence-use:{}",
                        paragraph.paragraph_id,
                        paragraph.evidence_uses.len() + 1
                    );
                    paragraph.evidence_uses.push(EvidenceUse {
                        id: id.clone(),
                        evidence_use_id: id,
                        matter_id: matter_id.to_string(),
                        complaint_id: complaint_id.to_string(),
                        target_type: request.target_type.clone(),
                        target_id: request.target_id.clone(),
                        fact_id: request.fact_id.clone(),
                        evidence_id: request.evidence_id.clone(),
                        document_id: request.document_id.clone(),
                        source_span_id: request.source_span_id.clone(),
                        relation: request
                            .relation
                            .clone()
                            .unwrap_or_else(|| "supports".to_string()),
                        quote: request.quote.clone(),
                        status: "needs_review".to_string(),
                    });
                }
                if let Some(citation) = request.citation.clone() {
                    let id = format!(
                        "{}:citation-use:{}",
                        paragraph.paragraph_id,
                        paragraph.citation_uses.len() + 1
                    );
                    paragraph.citation_uses.push(CitationUse {
                        id: id.clone(),
                        citation_use_id: id,
                        matter_id: matter_id.to_string(),
                        complaint_id: complaint_id.to_string(),
                        target_type: request.target_type.clone(),
                        target_id: request.target_id.clone(),
                        citation,
                        canonical_id: request.canonical_id.clone(),
                        pinpoint: request.pinpoint.clone(),
                        quote: request.quote.clone(),
                        status: if request.canonical_id.is_some() {
                            "resolved".to_string()
                        } else {
                            "unresolved".to_string()
                        },
                        currentness: "needs_review".to_string(),
                        scope_warning: None,
                    });
                }
                if let Some(exhibit_label) = request.exhibit_label.clone() {
                    let id = format!(
                        "{}:exhibit-reference:{}",
                        paragraph.paragraph_id,
                        paragraph.exhibit_references.len() + 1
                    );
                    paragraph.exhibit_references.push(ExhibitReference {
                        id: id.clone(),
                        exhibit_reference_id: id,
                        matter_id: matter_id.to_string(),
                        complaint_id: complaint_id.to_string(),
                        target_type: request.target_type.clone(),
                        target_id: request.target_id.clone(),
                        exhibit_label,
                        document_id: request.document_id.clone(),
                        evidence_id: request.evidence_id.clone(),
                        status: if request.document_id.is_some() || request.evidence_id.is_some() {
                            "linked".to_string()
                        } else {
                            "missing".to_string()
                        },
                    });
                }
                paragraph.sentences = pleading_sentences(
                    matter_id,
                    complaint_id,
                    &paragraph.paragraph_id,
                    &paragraph.text,
                    &paragraph.fact_ids,
                );
            }
            "count" => {
                let count = complaint
                    .counts
                    .iter_mut()
                    .find(|count| count.count_id == request.target_id)
                    .ok_or_else(|| {
                        ApiError::NotFound(format!(
                            "Complaint count {} not found",
                            request.target_id
                        ))
                    })?;
                if let Some(fact_id) = request.fact_id {
                    push_unique(&mut count.fact_ids, fact_id);
                }
                if let Some(evidence_id) = request.evidence_id {
                    push_unique(&mut count.evidence_ids, evidence_id);
                }
                if let Some(citation) = request.citation {
                    push_authority(
                        &mut count.authorities,
                        AuthorityRef {
                            citation,
                            canonical_id: request
                                .canonical_id
                                .unwrap_or_else(|| request.target_id.clone()),
                            reason: request.quote,
                            pinpoint: request.pinpoint,
                        },
                    );
                }
            }
            value => {
                return Err(ApiError::BadRequest(format!(
                    "Unsupported complaint link target_type {value}"
                )));
            }
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "support_linked",
            &request.target_type,
            &request.target_id,
            "Support, authority, citation, or exhibit link added.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn run_complaint_qc(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<AiActionResponse<Vec<RuleCheckFinding>>> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let findings = complaint_rule_findings(&complaint);
        complaint.findings = findings.clone();
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "qc_run",
            "complaint",
            complaint_id,
            "Deterministic complaint QC run completed.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await?;
        Ok(AiActionResponse {
            enabled: false,
            mode: "deterministic".to_string(),
            message: "No live rule provider is configured; ran deterministic Oregon complaint checks. Human review is required.".to_string(),
            result: Some(findings),
        })
    }

    pub async fn list_complaint_findings(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<Vec<RuleCheckFinding>> {
        Ok(self.get_complaint(matter_id, complaint_id).await?.findings)
    }

    pub async fn patch_complaint_finding(
        &self,
        matter_id: &str,
        complaint_id: &str,
        finding_id: &str,
        request: PatchRuleFindingRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let finding = complaint
            .findings
            .iter_mut()
            .find(|finding| finding.finding_id == finding_id)
            .ok_or_else(|| ApiError::NotFound(format!("Rule finding {finding_id} not found")))?;
        finding.status = request.status;
        finding.updated_at = now_string();
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "qc_finding_status_changed",
            "finding",
            finding_id,
            "Complaint QC finding status changed.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn preview_complaint(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<ComplaintPreviewResponse> {
        let complaint = self.get_complaint(matter_id, complaint_id).await?;
        Ok(render_complaint_preview(&complaint))
    }

    pub async fn export_complaint(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: ExportComplaintRequest,
    ) -> ApiResult<ExportArtifact> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let format = request.format.to_ascii_lowercase();
        let supported = [
            "pdf",
            "docx",
            "html",
            "markdown",
            "text",
            "plain_text",
            "json",
        ];
        if !supported.contains(&format.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "Unsupported complaint export format {format}"
            )));
        }
        let profile = request
            .profile
            .unwrap_or_else(|| "clean_filing_copy".to_string());
        let mode = request.mode.unwrap_or_else(|| "review_needed".to_string());
        let rendered = export_complaint_content(
            &complaint,
            &format,
            request.include_exhibits.unwrap_or(true),
            request.include_qc_report.unwrap_or(true),
        )?;
        let artifact_id = format!(
            "{}:artifact:{}:{}",
            complaint_id,
            sanitize_path_segment(&format),
            complaint.export_artifacts.len() + 1
        );
        let warnings = export_warnings(&complaint, &format);
        let artifact = ExportArtifact {
            id: artifact_id.clone(),
            artifact_id: artifact_id.clone(),
            matter_id: matter_id.to_string(),
            complaint_id: complaint_id.to_string(),
            format: format.clone(),
            profile,
            mode,
            status: if matches!(format.as_str(), "pdf" | "docx") {
                "skeleton_review_needed".to_string()
            } else {
                "generated_review_needed".to_string()
            },
            download_url: format!(
                "/api/v1/matters/{}/complaints/{}/artifacts/{}/download",
                matter_id, complaint_id, artifact_id
            ),
            page_count: render_complaint_preview(&complaint).page_count,
            generated_at: now_string(),
            warnings,
            content_preview: rendered,
            object_blob_id: None,
            size_bytes: None,
            mime_type: Some(export_mime_type(&format).to_string()),
            storage_status: Some("legacy_inline".to_string()),
        };
        complaint.export_artifacts.push(artifact.clone());
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "export_generated",
            "export_artifact",
            &artifact_id,
            "Complaint export artifact generated for review.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await?;
        self.merge_node(
            matter_id,
            complaint_artifact_spec(),
            &artifact.artifact_id,
            &artifact,
        )
        .await?;
        Ok(artifact)
    }

    pub async fn get_complaint_artifact(
        &self,
        matter_id: &str,
        complaint_id: &str,
        artifact_id: &str,
    ) -> ApiResult<ExportArtifact> {
        let complaint = self.get_complaint(matter_id, complaint_id).await?;
        complaint
            .export_artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == artifact_id)
            .ok_or_else(|| ApiError::NotFound(format!("Export artifact {artifact_id} not found")))
    }

    pub async fn download_complaint_artifact(
        &self,
        matter_id: &str,
        complaint_id: &str,
        artifact_id: &str,
    ) -> ApiResult<ComplaintDownloadResponse> {
        let artifact = self
            .get_complaint_artifact(matter_id, complaint_id, artifact_id)
            .await?;
        Ok(ComplaintDownloadResponse {
            method: "GET".to_string(),
            url: artifact.download_url.clone(),
            expires_at: timestamp_after(self.download_ttl_seconds),
            headers: BTreeMap::new(),
            filename: format!(
                "{}.{}",
                sanitize_path_segment(&artifact.complaint_id),
                artifact.format
            ),
            mime_type: Some(export_mime_type(&artifact.format).to_string()),
            bytes: artifact.content_preview.as_bytes().len() as u64,
            artifact,
        })
    }

    pub async fn run_complaint_ai_command(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: ComplaintAiCommandRequest,
    ) -> ApiResult<AiActionResponse<ComplaintDraft>> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let target = request
            .target_id
            .unwrap_or_else(|| complaint_id.to_string());
        let command_label = request.command.replace('_', " ");
        let warning = format!(
            "Provider-free template mode recorded command '{command_label}'. No unsupported facts or legal conclusions were inserted."
        );
        for command in &mut complaint.ai_commands {
            if command.command_id == request.command {
                command.last_message = Some(warning.clone());
                command.status = "template_available".to_string();
            }
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "ai_command_template",
            "complaint",
            &target,
            &warning,
        ));
        if request.prompt.is_some() {
            complaint.history.push(complaint_event(
                matter_id,
                complaint_id,
                "ai_prompt_recorded",
                "complaint",
                &target,
                "Prompt recorded for human review.",
            ));
        }
        refresh_complaint_state(&mut complaint);
        let complaint = self.save_complaint(matter_id, complaint).await?;
        Ok(AiActionResponse {
            enabled: false,
            mode: "template".to_string(),
            message: warning,
            result: Some(complaint),
        })
    }

    pub async fn filing_packet(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<FilingPacket> {
        Ok(self
            .get_complaint(matter_id, complaint_id)
            .await?
            .filing_packet)
    }

    async fn save_complaint(
        &self,
        matter_id: &str,
        mut complaint: ComplaintDraft,
    ) -> ApiResult<ComplaintDraft> {
        let before_product = self
            .get_node::<WorkProduct>(matter_id, work_product_spec(), &complaint.complaint_id)
            .await
            .ok();
        complaint.updated_at = now_string();
        let complaint = self
            .merge_node(
                matter_id,
                complaint_spec(),
                &complaint.complaint_id,
                &complaint,
            )
            .await?;
        self.materialize_complaint_edges(&complaint).await?;
        for finding in &complaint.findings {
            self.merge_node(
                matter_id,
                complaint_finding_spec(),
                &finding.finding_id,
                finding,
            )
            .await?;
        }
        for artifact in &complaint.export_artifacts {
            self.merge_node(
                matter_id,
                complaint_artifact_spec(),
                &artifact.artifact_id,
                artifact,
            )
            .await?;
        }
        let product = work_product_from_complaint(&complaint);
        let version_changes = work_product_facade_change_inputs(before_product.as_ref(), &product);
        self.save_work_product_internal(matter_id, product.clone())
            .await?;
        if !version_changes.is_empty() {
            let snapshot_type = if before_product.is_none() {
                "auto"
            } else if version_changes
                .iter()
                .any(|change| change.target_type == "export")
            {
                "export"
            } else if version_changes
                .iter()
                .any(|change| change.target_type == "rule_finding")
            {
                "rule_check"
            } else {
                "auto"
            };
            let title = if before_product.is_none() {
                "Complaint created"
            } else {
                "Complaint updated"
            };
            let change_set = self
                .record_work_product_change(
                    matter_id,
                    before_product.as_ref(),
                    &product,
                    "user",
                    snapshot_type,
                    title,
                    "Complaint facade synchronized to canonical Case History.",
                    version_changes,
                )
                .await?;
            if snapshot_type == "export" {
                let mut locked_product = product;
                let qc_status_at_export = work_product_qc_status(&locked_product);
                let mut changed = false;
                for artifact in &mut locked_product.artifacts {
                    if artifact.snapshot_id.is_none() {
                        artifact.snapshot_id = Some(change_set.snapshot_id.clone());
                        artifact.qc_status_at_export = Some(qc_status_at_export.clone());
                        artifact.changed_since_export = Some(false);
                        artifact.immutable = Some(true);
                        changed = true;
                    }
                }
                if changed {
                    self.save_work_product_internal(matter_id, locked_product)
                        .await?;
                }
            }
        }
        Ok(complaint)
    }

    async fn save_complaint_projection_only(
        &self,
        matter_id: &str,
        mut complaint: ComplaintDraft,
    ) -> ApiResult<ComplaintDraft> {
        complaint.updated_at = now_string();
        let complaint = self
            .merge_node(
                matter_id,
                complaint_spec(),
                &complaint.complaint_id,
                &complaint,
            )
            .await?;
        self.materialize_complaint_edges(&complaint).await?;
        for finding in &complaint.findings {
            self.merge_node(
                matter_id,
                complaint_finding_spec(),
                &finding.finding_id,
                finding,
            )
            .await?;
        }
        for artifact in &complaint.export_artifacts {
            self.merge_node(
                matter_id,
                complaint_artifact_spec(),
                &artifact.artifact_id,
                artifact,
            )
            .await?;
        }
        Ok(complaint)
    }

    async fn sync_complaint_projection_from_work_product(
        &self,
        matter_id: &str,
        product: &WorkProduct,
    ) -> ApiResult<()> {
        if product.product_type != "complaint" {
            return Ok(());
        }
        let mut complaint = match self
            .get_node::<ComplaintDraft>(matter_id, complaint_spec(), &product.work_product_id)
            .await
        {
            Ok(complaint) => complaint,
            Err(ApiError::NotFound(_)) => return Ok(()),
            Err(error) => return Err(error),
        };
        complaint.title = product.title.clone();
        complaint.status = product.status.clone();
        complaint.review_status = product.review_status.clone();
        complaint.setup_stage = product.setup_stage.clone();
        complaint.formatting_profile = product.formatting_profile.clone();
        complaint.rule_pack = product.rule_pack.clone();

        for section in &mut complaint.sections {
            if let Some(block) = product
                .blocks
                .iter()
                .find(|block| block.block_id == section.section_id)
            {
                section.title = block.title.clone();
                section.review_status = block.review_status.clone();
            }
        }
        for count in &mut complaint.counts {
            if let Some(block) = product
                .blocks
                .iter()
                .find(|block| block.block_id == count.count_id)
            {
                count.title = block.title.clone();
                count.legal_theory = block.text.clone();
                count.fact_ids = block.fact_ids.clone();
                count.evidence_ids = block.evidence_ids.clone();
                count.authorities = block.authorities.clone();
                count.health = block.review_status.clone();
            }
        }
        for paragraph in &mut complaint.paragraphs {
            if let Some(block) = product
                .blocks
                .iter()
                .find(|block| block.block_id == paragraph.paragraph_id)
            {
                paragraph.text = block.text.clone();
                paragraph.fact_ids = block.fact_ids.clone();
                paragraph.locked = block.locked;
                paragraph.review_status = block.review_status.clone();
                paragraph.evidence_uses = block
                    .evidence_ids
                    .iter()
                    .enumerate()
                    .map(|(index, evidence_id)| {
                        let id = format!("{}:evidence:{}", paragraph.paragraph_id, index + 1);
                        EvidenceUse {
                            evidence_use_id: id.clone(),
                            id,
                            matter_id: matter_id.to_string(),
                            complaint_id: complaint.complaint_id.clone(),
                            target_type: "paragraph".to_string(),
                            target_id: paragraph.paragraph_id.clone(),
                            fact_id: None,
                            evidence_id: Some(evidence_id.clone()),
                            document_id: None,
                            source_span_id: None,
                            relation: "supports".to_string(),
                            quote: None,
                            status: "linked".to_string(),
                        }
                    })
                    .collect();
                paragraph.citation_uses = block
                    .authorities
                    .iter()
                    .enumerate()
                    .map(|(index, authority)| {
                        let id = format!("{}:citation:{}", paragraph.paragraph_id, index + 1);
                        CitationUse {
                            citation_use_id: id.clone(),
                            id,
                            matter_id: matter_id.to_string(),
                            complaint_id: complaint.complaint_id.clone(),
                            target_type: "paragraph".to_string(),
                            target_id: paragraph.paragraph_id.clone(),
                            citation: authority.citation.clone(),
                            canonical_id: Some(authority.canonical_id.clone()),
                            pinpoint: authority.pinpoint.clone(),
                            quote: None,
                            status: "inserted".to_string(),
                            currentness: "unchecked".to_string(),
                            scope_warning: None,
                        }
                    })
                    .collect();
                paragraph.sentences = pleading_sentences(
                    matter_id,
                    &complaint.complaint_id,
                    &paragraph.paragraph_id,
                    &paragraph.text,
                    &paragraph.fact_ids,
                );
            }
        }
        complaint.findings = product
            .findings
            .iter()
            .map(|finding| RuleCheckFinding {
                id: finding.finding_id.clone(),
                finding_id: finding.finding_id.clone(),
                matter_id: finding.matter_id.clone(),
                complaint_id: complaint.complaint_id.clone(),
                rule_id: finding.rule_id.clone(),
                category: finding.category.clone(),
                severity: finding.severity.clone(),
                target_type: finding.target_type.clone(),
                target_id: finding.target_id.clone(),
                message: finding.message.clone(),
                explanation: finding.explanation.clone(),
                suggested_fix: finding.suggested_fix.clone(),
                primary_action: ComplaintAction {
                    action_id: finding.primary_action.action_id.clone(),
                    label: finding.primary_action.label.clone(),
                    action_type: finding.primary_action.action_type.clone(),
                    href: finding.primary_action.href.clone(),
                    target_type: finding.primary_action.target_type.clone(),
                    target_id: finding.primary_action.target_id.clone(),
                },
                status: finding.status.clone(),
                created_at: finding.created_at.clone(),
                updated_at: finding.updated_at.clone(),
            })
            .collect();
        refresh_complaint_state(&mut complaint);
        self.save_complaint_projection_only(matter_id, complaint)
            .await?;
        Ok(())
    }

    pub async fn attach_authority(
        &self,
        matter_id: &str,
        request: AuthorityAttachmentRequest,
    ) -> ApiResult<AuthorityAttachmentResponse> {
        self.require_matter(matter_id).await?;
        let authority = AuthorityRef {
            citation: request.citation,
            canonical_id: request.canonical_id,
            reason: request.reason,
            pinpoint: request.pinpoint,
        };
        match request.target_type.as_str() {
            "claim" => {
                let mut claim = self
                    .get_node::<CaseClaim>(matter_id, claim_spec(), &request.target_id)
                    .await?;
                push_authority(&mut claim.authorities, authority.clone());
                let claim = self
                    .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
                    .await?;
                self.materialize_claim_edges(&claim).await?;
            }
            "element" => {
                let mut claim = self
                    .claim_for_element(matter_id, &request.target_id)
                    .await?;
                let element = claim
                    .elements
                    .iter_mut()
                    .find(|element| {
                        element.element_id == request.target_id || element.id == request.target_id
                    })
                    .ok_or_else(|| {
                        ApiError::NotFound(format!("Element {} not found", request.target_id))
                    })?;
                push_authority(&mut element.authorities, authority.clone());
                if element.authority.is_none() {
                    element.authority = Some(authority.citation.clone());
                }
                let claim = self
                    .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
                    .await?;
                self.materialize_claim_edges(&claim).await?;
            }
            "draft_paragraph" => {
                let mut draft = self
                    .draft_for_paragraph(matter_id, &request.target_id)
                    .await?;
                let paragraph = draft
                    .paragraphs
                    .iter_mut()
                    .find(|paragraph| paragraph.paragraph_id == request.target_id)
                    .ok_or_else(|| {
                        ApiError::NotFound(format!(
                            "Draft paragraph {} not found",
                            request.target_id
                        ))
                    })?;
                push_authority(&mut paragraph.authorities, authority.clone());
                let draft = self
                    .merge_node(matter_id, draft_spec(), &draft.draft_id, &draft)
                    .await?;
                self.materialize_draft_edges(&draft).await?;
            }
            value => {
                return Err(ApiError::BadRequest(format!(
                    "Unsupported authority target_type {value}"
                )));
            }
        }

        Ok(AuthorityAttachmentResponse {
            matter_id: matter_id.to_string(),
            target_type: request.target_type,
            target_id: request.target_id,
            authority,
            attached: true,
        })
    }

    pub async fn detach_authority(
        &self,
        matter_id: &str,
        request: AuthorityAttachmentRequest,
    ) -> ApiResult<AuthorityAttachmentResponse> {
        self.require_matter(matter_id).await?;
        let authority = AuthorityRef {
            citation: request.citation,
            canonical_id: request.canonical_id,
            reason: request.reason,
            pinpoint: request.pinpoint,
        };
        match request.target_type.as_str() {
            "claim" => {
                let mut claim = self
                    .get_node::<CaseClaim>(matter_id, claim_spec(), &request.target_id)
                    .await?;
                remove_authority(&mut claim.authorities, &authority);
                let claim = self
                    .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
                    .await?;
                self.detach_authority_edge("Claim", "claim_id", &claim.claim_id, &authority)
                    .await?;
                self.materialize_claim_edges(&claim).await?;
            }
            "element" => {
                let mut claim = self
                    .claim_for_element(matter_id, &request.target_id)
                    .await?;
                let element = claim
                    .elements
                    .iter_mut()
                    .find(|element| {
                        element.element_id == request.target_id || element.id == request.target_id
                    })
                    .ok_or_else(|| {
                        ApiError::NotFound(format!("Element {} not found", request.target_id))
                    })?;
                remove_authority(&mut element.authorities, &authority);
                if element.authority.as_deref() == Some(authority.citation.as_str()) {
                    element.authority = element
                        .authorities
                        .first()
                        .map(|item| item.citation.clone());
                }
                let element_id = element.element_id.clone();
                let claim = self
                    .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
                    .await?;
                self.detach_authority_edge("Element", "element_id", &element_id, &authority)
                    .await?;
                self.materialize_claim_edges(&claim).await?;
            }
            "draft_paragraph" => {
                let mut draft = self
                    .draft_for_paragraph(matter_id, &request.target_id)
                    .await?;
                if let Some(paragraph) = draft
                    .paragraphs
                    .iter_mut()
                    .find(|paragraph| paragraph.paragraph_id == request.target_id)
                {
                    remove_authority(&mut paragraph.authorities, &authority);
                }
                let draft = self
                    .merge_node(matter_id, draft_spec(), &draft.draft_id, &draft)
                    .await?;
                self.detach_authority_edge(
                    "DraftParagraph",
                    "paragraph_id",
                    &request.target_id,
                    &authority,
                )
                .await?;
                self.materialize_draft_edges(&draft).await?;
            }
            value => {
                return Err(ApiError::BadRequest(format!(
                    "Unsupported authority target_type {value}"
                )));
            }
        }

        Ok(AuthorityAttachmentResponse {
            matter_id: matter_id.to_string(),
            target_type: request.target_type,
            target_id: request.target_id,
            authority,
            attached: false,
        })
    }

    async fn list_fact_check_findings(
        &self,
        matter_id: &str,
        draft_id: Option<&str>,
    ) -> ApiResult<Vec<FactCheckFinding>> {
        let mut findings: Vec<FactCheckFinding> = self
            .list_nodes(matter_id, fact_check_finding_spec())
            .await?;
        if let Some(draft_id) = draft_id {
            findings.retain(|finding| finding.draft_id == draft_id);
        }
        Ok(findings)
    }

    async fn list_citation_check_findings(
        &self,
        matter_id: &str,
        draft_id: Option<&str>,
    ) -> ApiResult<Vec<CitationCheckFinding>> {
        let mut findings: Vec<CitationCheckFinding> = self
            .list_nodes(matter_id, citation_check_finding_spec())
            .await?;
        if let Some(draft_id) = draft_id {
            findings.retain(|finding| finding.draft_id == draft_id);
        }
        Ok(findings)
    }

    async fn claim_for_element(&self, matter_id: &str, element_id: &str) -> ApiResult<CaseClaim> {
        self.list_claims(matter_id)
            .await?
            .into_iter()
            .find(|claim| {
                claim
                    .elements
                    .iter()
                    .any(|element| element.element_id == element_id || element.id == element_id)
            })
            .ok_or_else(|| ApiError::NotFound(format!("Element {element_id} not found")))
    }

    async fn draft_for_paragraph(
        &self,
        matter_id: &str,
        paragraph_id: &str,
    ) -> ApiResult<CaseDraft> {
        self.list_drafts(matter_id)
            .await?
            .into_iter()
            .find(|draft| {
                draft
                    .paragraphs
                    .iter()
                    .any(|paragraph| paragraph.paragraph_id == paragraph_id)
            })
            .ok_or_else(|| ApiError::NotFound(format!("Draft paragraph {paragraph_id} not found")))
    }

    async fn merge_matter(&self, matter: &MatterSummary) -> ApiResult<MatterSummary> {
        let payload = to_payload(matter)?;
        self.neo4j
            .run_rows(
                query(
                    "MERGE (m:Matter {matter_id: $matter_id})
                     SET m.payload = $payload,
                         m.name = $name,
                         m.status = $status,
                         m.matter_type = $matter_type,
                         m.updated_at = $updated_at
                     RETURN m.payload AS payload",
                )
                .param("matter_id", matter.matter_id.clone())
                .param("payload", payload)
                .param("name", matter.name.clone())
                .param("status", matter.status.clone())
                .param("matter_type", matter.matter_type.clone())
                .param("updated_at", matter.updated_at.clone()),
            )
            .await?;
        Ok(matter.clone())
    }

    async fn get_matter_summary(&self, matter_id: &str) -> ApiResult<MatterSummary> {
        let rows = self
            .neo4j
            .run_rows(
                query("MATCH (m:Matter {matter_id: $matter_id}) RETURN m.payload AS payload")
                    .param("matter_id", matter_id),
            )
            .await?;
        let payload = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
            .ok_or_else(|| ApiError::NotFound(format!("Matter {matter_id} not found")))?;
        from_payload(&payload)
    }

    async fn require_matter(&self, matter_id: &str) -> ApiResult<()> {
        self.get_matter_summary(matter_id).await.map(|_| ())
    }

    async fn merge_node<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        matter_id: &str,
        spec: NodeSpec,
        id: &str,
        value: &T,
    ) -> ApiResult<T> {
        let payload = to_payload(value)?;
        let statement = format!(
            "MATCH (m:Matter {{matter_id: $matter_id}})
             MERGE (n:{label} {{{id_key}: $id}})
             SET n.payload = $payload,
                 n.matter_id = $matter_id,
                 n.{id_key} = $id
             MERGE (m)-[:{edge}]->(n)
             RETURN n.payload AS payload",
            label = spec.label,
            id_key = spec.id_key,
            edge = spec.edge,
        );
        let rows = self
            .neo4j
            .run_rows(
                query(&statement)
                    .param("matter_id", matter_id)
                    .param("id", id)
                    .param("payload", payload),
            )
            .await?;
        let payload = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
            .ok_or_else(|| ApiError::Internal("CaseBuilder write returned no row".to_string()))?;
        from_payload(&payload)
    }

    async fn get_node<T: serde::de::DeserializeOwned>(
        &self,
        matter_id: &str,
        spec: NodeSpec,
        id: &str,
    ) -> ApiResult<T> {
        let statement = format!(
            "MATCH (:Matter {{matter_id: $matter_id}})-[:{edge}]->(n:{label} {{{id_key}: $id}})
             RETURN n.payload AS payload",
            label = spec.label,
            id_key = spec.id_key,
            edge = spec.edge,
        );
        let rows = self
            .neo4j
            .run_rows(
                query(&statement)
                    .param("matter_id", matter_id)
                    .param("id", id),
            )
            .await?;
        let payload = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
            .ok_or_else(|| ApiError::NotFound(format!("{} {id} not found", spec.label)))?;
        from_payload(&payload)
    }

    async fn list_nodes<T: serde::de::DeserializeOwned>(
        &self,
        matter_id: &str,
        spec: NodeSpec,
    ) -> ApiResult<Vec<T>> {
        let statement = format!(
            "MATCH (:Matter {{matter_id: $matter_id}})-[:{edge}]->(n:{label})
             RETURN n.payload AS payload
             ORDER BY coalesce(n.uploaded_at, n.updated_at, n.created_at, n.{id_key})",
            label = spec.label,
            id_key = spec.id_key,
            edge = spec.edge,
        );
        let rows = self
            .neo4j
            .run_rows(query(&statement).param("matter_id", matter_id))
            .await?;
        rows.into_iter()
            .map(|row| {
                let payload = row
                    .get::<String>("payload")
                    .map_err(|error| ApiError::Internal(error.to_string()))?;
                from_payload(&payload)
            })
            .collect()
    }

    async fn persist_document_provenance(
        &self,
        matter_id: &str,
        provenance: &DocumentProvenance,
    ) -> ApiResult<()> {
        self.merge_object_blob(matter_id, &provenance.object_blob)
            .await?;
        self.merge_document_version(matter_id, &provenance.document_version)
            .await?;
        self.merge_ingestion_run(matter_id, &provenance.ingestion_run)
            .await?;
        Ok(())
    }

    async fn ensure_document_original_provenance(
        &self,
        matter_id: &str,
        document: &mut CaseDocument,
    ) -> ApiResult<Option<DocumentProvenance>> {
        let Some(key) = document.storage_key.clone() else {
            return Ok(None);
        };
        if document.storage_status == "deleted" {
            return Ok(None);
        }
        let object = StoredObject {
            bucket: document
                .storage_bucket
                .clone()
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            key,
            content_length: document.bytes,
            etag: document.content_etag.clone(),
            content_type: document.mime_type.clone(),
            metadata: document
                .file_hash
                .as_ref()
                .map(|hash| BTreeMap::from([("sha256".to_string(), hash.clone())]))
                .unwrap_or_default(),
            local_path: document.storage_path.clone(),
        };
        let provenance = build_original_provenance(matter_id, document, &object, "stored");
        apply_document_provenance(document, &provenance);
        self.persist_document_provenance(matter_id, &provenance)
            .await?;
        Ok(Some(provenance))
    }

    async fn merge_object_blob(&self, matter_id: &str, blob: &ObjectBlob) -> ApiResult<ObjectBlob> {
        let payload = to_payload(blob)?;
        let rows = self
            .neo4j
            .run_rows(
                query(
                    "MATCH (m:Matter {matter_id: $matter_id})
                     MERGE (b:ObjectBlob {object_blob_id: $object_blob_id})
                     ON CREATE SET b.created_at = $created_at
                     SET b.payload = $payload,
                         b.object_blob_id = $object_blob_id,
                         b.sha256 = $sha256,
                         b.storage_provider = $storage_provider,
                         b.storage_bucket = $storage_bucket,
                         b.storage_key = $storage_key,
                         b.size_bytes = $size_bytes,
                         b.retention_state = $retention_state
                     MERGE (m)-[:USES_OBJECT_BLOB]->(b)
                     RETURN b.payload AS payload",
                )
                .param("matter_id", matter_id)
                .param("object_blob_id", blob.object_blob_id.clone())
                .param("payload", payload)
                .param("created_at", blob.created_at.clone())
                .param("sha256", blob.sha256.clone().unwrap_or_default())
                .param("storage_provider", blob.storage_provider.clone())
                .param(
                    "storage_bucket",
                    blob.storage_bucket.clone().unwrap_or_default(),
                )
                .param("storage_key", blob.storage_key.clone())
                .param("size_bytes", blob.size_bytes as i64)
                .param("retention_state", blob.retention_state.clone()),
            )
            .await?;
        let payload = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
            .ok_or_else(|| ApiError::Internal("ObjectBlob write returned no row".to_string()))?;
        from_payload(&payload)
    }

    async fn get_object_blob(
        &self,
        matter_id: &str,
        object_blob_id: &str,
    ) -> ApiResult<ObjectBlob> {
        let rows = self
            .neo4j
            .run_rows(
                query(
                    "MATCH (:Matter {matter_id: $matter_id})-[:USES_OBJECT_BLOB]->(b:ObjectBlob {object_blob_id: $object_blob_id})
                     RETURN b.payload AS payload",
                )
                .param("matter_id", matter_id)
                .param("object_blob_id", object_blob_id),
            )
            .await?;
        let payload = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
            .ok_or_else(|| ApiError::NotFound("ObjectBlob not found".to_string()))?;
        from_payload(&payload)
    }

    async fn store_casebuilder_bytes(
        &self,
        matter_id: &str,
        key: &str,
        bytes: Bytes,
        content_type: &str,
    ) -> ApiResult<ObjectBlob> {
        let sha256 = sha256_hex(&bytes);
        let stored = self
            .object_store
            .put_bytes(
                key,
                bytes.clone(),
                put_options(Some(content_type.to_string()), Some(sha256.clone())),
            )
            .await?;
        let now = now_string();
        let blob = ObjectBlob {
            object_blob_id: object_blob_id_for_hash(&sha256),
            id: object_blob_id_for_hash(&sha256),
            sha256: Some(sha256),
            size_bytes: stored.content_length,
            mime_type: stored.content_type.clone(),
            storage_provider: self.object_store.provider().to_string(),
            storage_bucket: stored
                .bucket
                .clone()
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            storage_key: stored.key,
            etag: stored.etag,
            storage_class: None,
            created_at: now,
            retention_state: "active".to_string(),
        };
        self.merge_object_blob(matter_id, &blob).await
    }

    async fn load_json_blob<T: serde::de::DeserializeOwned>(
        &self,
        matter_id: &str,
        object_blob_id: &str,
    ) -> ApiResult<T> {
        let blob = self.get_object_blob(matter_id, object_blob_id).await?;
        let bytes = self.object_store.get_bytes(&blob.storage_key).await?;
        serde_json::from_slice(&bytes).map_err(|error| ApiError::Internal(error.to_string()))
    }

    async fn validate_ast_patch_matter_references(
        &self,
        matter_id: &str,
        product: &WorkProduct,
        patch: &AstPatch,
    ) -> ApiResult<()> {
        for operation in &patch.operations {
            match operation {
                AstOperation::InsertBlock { block, .. } => {
                    self.validate_ast_block_payload_references(matter_id, product, block)
                        .await?;
                }
                AstOperation::AddLink { link } => {
                    self.validate_work_product_link_target(
                        matter_id,
                        &link.target_type,
                        &link.target_id,
                    )
                    .await?;
                }
                AstOperation::AddCitation { citation } => {
                    self.validate_citation_target_reference(
                        matter_id,
                        citation.target_type.as_str(),
                        citation.target_id.as_deref(),
                    )
                    .await?;
                }
                AstOperation::ResolveCitation {
                    target_type,
                    target_id,
                    ..
                } => {
                    if let Some(target_id) = target_id.as_deref() {
                        self.validate_citation_target_reference(
                            matter_id,
                            target_type.as_deref().unwrap_or("legal_authority"),
                            Some(target_id),
                        )
                        .await?;
                    }
                }
                AstOperation::AddExhibitReference { exhibit } => {
                    self.validate_exhibit_reference_targets(matter_id, exhibit)
                        .await?;
                }
                AstOperation::ResolveExhibitReference { exhibit_id, .. } => {
                    if let Some(exhibit_id) = exhibit_id.as_deref() {
                        self.require_evidence_or_document(matter_id, exhibit_id)
                            .await?;
                    }
                }
                AstOperation::AddRuleFinding { finding } => {
                    if finding.matter_id != matter_id {
                        return Err(ApiError::BadRequest(
                            "AST rule finding matter does not match route matter.".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn validate_work_product_matter_references(
        &self,
        matter_id: &str,
        product: &WorkProduct,
    ) -> ApiResult<()> {
        if product.matter_id != matter_id || product.document_ast.matter_id != matter_id {
            return Err(ApiError::BadRequest(
                "Work product matter does not match route matter.".to_string(),
            ));
        }
        let blocks = flatten_work_product_blocks(&product.document_ast.blocks);
        let block_ids = blocks
            .iter()
            .map(|block| block.block_id.clone())
            .collect::<HashSet<_>>();
        for block in &blocks {
            if block.matter_id != matter_id || block.work_product_id != product.work_product_id {
                return Err(ApiError::BadRequest(
                    "AST block ownership does not match work product.".to_string(),
                ));
            }
            for fact_id in &block.fact_ids {
                self.require_fact(matter_id, fact_id).await?;
            }
            for evidence_id in &block.evidence_ids {
                self.require_evidence_document_or_span(matter_id, evidence_id)
                    .await?;
            }
        }
        for link in &product.document_ast.links {
            if !block_ids.contains(&link.source_block_id) {
                return Err(ApiError::NotFound(
                    "AST link source block not found".to_string(),
                ));
            }
            self.validate_work_product_link_target(matter_id, &link.target_type, &link.target_id)
                .await?;
        }
        for citation in &product.document_ast.citations {
            if !block_ids.contains(&citation.source_block_id) {
                return Err(ApiError::NotFound(
                    "AST citation source block not found".to_string(),
                ));
            }
            self.validate_citation_target_reference(
                matter_id,
                &citation.target_type,
                citation.target_id.as_deref(),
            )
            .await?;
        }
        for exhibit in &product.document_ast.exhibits {
            if !block_ids.contains(&exhibit.source_block_id) {
                return Err(ApiError::NotFound(
                    "AST exhibit source block not found".to_string(),
                ));
            }
            self.validate_exhibit_reference_targets(matter_id, exhibit)
                .await?;
        }
        for finding in &product.document_ast.rule_findings {
            if finding.matter_id != matter_id || finding.work_product_id != product.work_product_id
            {
                return Err(ApiError::BadRequest(
                    "AST rule finding ownership does not match work product.".to_string(),
                ));
            }
            if matches!(
                finding.target_type.as_str(),
                "block" | "paragraph" | "section"
            ) && !block_ids.contains(&finding.target_id)
            {
                return Err(ApiError::NotFound(
                    "AST rule finding target block not found".to_string(),
                ));
            }
        }
        Ok(())
    }

    async fn validate_ast_block_payload_references(
        &self,
        matter_id: &str,
        product: &WorkProduct,
        block: &WorkProductBlock,
    ) -> ApiResult<()> {
        let mut stack = vec![block];
        while let Some(block) = stack.pop() {
            if (!block.matter_id.is_empty() && block.matter_id != matter_id)
                || (!block.work_product_id.is_empty()
                    && block.work_product_id != product.work_product_id)
            {
                return Err(ApiError::BadRequest(
                    "AST block ownership does not match work product.".to_string(),
                ));
            }
            for fact_id in &block.fact_ids {
                self.require_fact(matter_id, fact_id).await?;
            }
            for evidence_id in &block.evidence_ids {
                self.require_evidence_document_or_span(matter_id, evidence_id)
                    .await?;
            }
            for child in &block.children {
                stack.push(child);
            }
        }
        Ok(())
    }

    async fn validate_complaint_link_references(
        &self,
        matter_id: &str,
        request: &ComplaintLinkRequest,
    ) -> ApiResult<()> {
        if let Some(fact_id) = request.fact_id.as_deref() {
            self.require_fact(matter_id, fact_id).await?;
        }
        if let Some(evidence_id) = request.evidence_id.as_deref() {
            self.require_evidence(matter_id, evidence_id).await?;
        }
        if let Some(document_id) = request.document_id.as_deref() {
            self.require_document(matter_id, document_id).await?;
        }
        if let Some(source_span_id) = request.source_span_id.as_deref() {
            self.require_source_span(matter_id, source_span_id).await?;
        }
        Ok(())
    }

    async fn validate_work_product_link_target(
        &self,
        matter_id: &str,
        target_type: &str,
        target_id: &str,
    ) -> ApiResult<()> {
        match target_type {
            "fact" => self.require_fact(matter_id, target_id).await,
            "evidence" => self.require_evidence(matter_id, target_id).await,
            "document" | "case_document" => self.require_document(matter_id, target_id).await,
            "source_span" | "text_span" | "document_page" => {
                self.require_source_span(matter_id, target_id).await
            }
            "exhibit" => {
                self.require_evidence_or_document(matter_id, target_id)
                    .await
            }
            "authority"
            | "legal_authority"
            | "provision"
            | "legal_text_identity"
            | "legal_text" => Ok(()),
            _ => Err(ApiError::BadRequest(
                "Unsupported support target_type.".to_string(),
            )),
        }
    }

    async fn validate_citation_target_reference(
        &self,
        matter_id: &str,
        target_type: &str,
        target_id: Option<&str>,
    ) -> ApiResult<()> {
        let Some(target_id) = target_id else {
            return Ok(());
        };
        match target_type {
            "fact" => self.require_fact(matter_id, target_id).await,
            "evidence" => self.require_evidence(matter_id, target_id).await,
            "document" | "case_document" => self.require_document(matter_id, target_id).await,
            "source_span" | "text_span" | "document_page" => {
                self.require_source_span(matter_id, target_id).await
            }
            _ => Ok(()),
        }
    }

    async fn validate_exhibit_reference_targets(
        &self,
        matter_id: &str,
        exhibit: &WorkProductExhibitReference,
    ) -> ApiResult<()> {
        if let Some(document_id) = exhibit.document_id.as_deref() {
            self.require_document(matter_id, document_id).await?;
        }
        if let Some(exhibit_id) = exhibit.exhibit_id.as_deref() {
            self.require_evidence_or_document(matter_id, exhibit_id)
                .await?;
        }
        Ok(())
    }

    async fn require_fact(&self, matter_id: &str, fact_id: &str) -> ApiResult<()> {
        self.get_node::<CaseFact>(matter_id, fact_spec(), fact_id)
            .await
            .map(|_| ())
            .map_err(|error| matter_reference_error(error, "fact"))
    }

    async fn require_evidence(&self, matter_id: &str, evidence_id: &str) -> ApiResult<()> {
        self.get_node::<CaseEvidence>(matter_id, evidence_spec(), evidence_id)
            .await
            .map(|_| ())
            .map_err(|error| matter_reference_error(error, "evidence"))
    }

    async fn require_document(&self, matter_id: &str, document_id: &str) -> ApiResult<()> {
        self.get_node::<CaseDocument>(matter_id, document_spec(), document_id)
            .await
            .map(|_| ())
            .map_err(|error| matter_reference_error(error, "document"))
    }

    async fn require_source_span(&self, matter_id: &str, source_span_id: &str) -> ApiResult<()> {
        self.get_node::<SourceSpan>(matter_id, source_span_spec(), source_span_id)
            .await
            .map(|_| ())
            .map_err(|error| matter_reference_error(error, "source_span"))
    }

    async fn require_evidence_or_document(
        &self,
        matter_id: &str,
        target_id: &str,
    ) -> ApiResult<()> {
        match self.require_evidence(matter_id, target_id).await {
            Ok(()) => Ok(()),
            Err(ApiError::NotFound(_)) => self.require_document(matter_id, target_id).await,
            Err(error) => Err(error),
        }
    }

    async fn require_evidence_document_or_span(
        &self,
        matter_id: &str,
        target_id: &str,
    ) -> ApiResult<()> {
        match self.require_evidence(matter_id, target_id).await {
            Ok(()) => Ok(()),
            Err(ApiError::NotFound(_)) => match self.require_document(matter_id, target_id).await {
                Ok(()) => Ok(()),
                Err(ApiError::NotFound(_)) => self.require_source_span(matter_id, target_id).await,
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        }
    }

    async fn merge_document_version(
        &self,
        matter_id: &str,
        version: &DocumentVersion,
    ) -> ApiResult<DocumentVersion> {
        let version = self
            .merge_node(
                matter_id,
                document_version_spec(),
                &version.document_version_id,
                version,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     MERGE (d)-[:HAS_VERSION]->(v)
                     MERGE (v)-[:VERSION_OF]->(d)
                     MERGE (v)-[:STORED_AS]->(b)
                     MERGE (v)-[:DERIVED_FROM]->(b)",
                )
                .param("document_id", version.document_id.clone())
                .param("document_version_id", version.document_version_id.clone())
                .param("object_blob_id", version.object_blob_id.clone()),
            )
            .await?;
        Ok(version)
    }

    async fn merge_ingestion_run(
        &self,
        matter_id: &str,
        run: &IngestionRun,
    ) -> ApiResult<IngestionRun> {
        let run = self
            .merge_node(matter_id, ingestion_run_spec(), &run.ingestion_run_id, run)
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (r:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     MERGE (d)-[:HAS_INGESTION_RUN]->(r)
                     WITH d, r
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:INDEXED]->(v)
                     )
                     FOREACH (_ IN CASE WHEN b IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:DERIVED_FROM]->(b)
                     )",
                )
                .param("document_id", run.document_id.clone())
                .param("ingestion_run_id", run.ingestion_run_id.clone())
                .param(
                    "document_version_id",
                    run.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "object_blob_id",
                    run.object_blob_id.clone().unwrap_or_default(),
                ),
            )
            .await?;
        Ok(run)
    }

    async fn merge_source_span(&self, matter_id: &str, span: &SourceSpan) -> ApiResult<SourceSpan> {
        let span = self
            .merge_node(matter_id, source_span_spec(), &span.source_span_id, span)
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (s:SourceSpan {source_span_id: $source_span_id})
                     MERGE (d)-[:HAS_SOURCE_SPAN]->(s)
                     WITH d, s
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     OPTIONAL MATCH (r:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (s)-[:FROM_VERSION]->(v)
                     )
                     FOREACH (_ IN CASE WHEN b IS NULL THEN [] ELSE [1] END |
                       MERGE (s)-[:DERIVED_FROM]->(b)
                     )
                     FOREACH (_ IN CASE WHEN r IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:PRODUCED]->(s)
                     )",
                )
                .param("document_id", span.document_id.clone())
                .param("source_span_id", span.source_span_id.clone())
                .param(
                    "document_version_id",
                    span.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "object_blob_id",
                    span.object_blob_id.clone().unwrap_or_default(),
                )
                .param(
                    "ingestion_run_id",
                    span.ingestion_run_id.clone().unwrap_or_default(),
                ),
            )
            .await?;
        Ok(span)
    }

    async fn materialize_fact_edges(&self, fact: &CaseFact) -> ApiResult<()> {
        for document_id in &fact.source_document_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (f:Fact {fact_id: $fact_id})
                         MATCH (d:CaseDocument {document_id: $document_id})
                         MERGE (f)-[:SUPPORTED_BY]->(d)
                         MERGE (d)-[:SUPPORTS]->(f)",
                    )
                    .param("fact_id", fact.fact_id.clone())
                    .param("document_id", document_id.clone()),
                )
                .await?;
        }
        for evidence_id in &fact.source_evidence_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (f:Fact {fact_id: $fact_id})
                         MATCH (e:Evidence {evidence_id: $evidence_id})
                         MERGE (f)-[:SUPPORTED_BY]->(e)
                         MERGE (e)-[:SUPPORTS]->(f)",
                    )
                    .param("fact_id", fact.fact_id.clone())
                    .param("evidence_id", evidence_id.clone()),
                )
                .await?;
        }
        for span in &fact.source_spans {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (f:Fact {fact_id: $fact_id})
                         MATCH (s:SourceSpan {source_span_id: $source_span_id})
                         MERGE (f)-[:SUPPORTED_BY]->(s)
                         MERGE (s)-[:SUPPORTS]->(f)",
                    )
                    .param("fact_id", fact.fact_id.clone())
                    .param("source_span_id", span.source_span_id.clone()),
                )
                .await?;
        }
        Ok(())
    }

    async fn materialize_evidence_edges(&self, evidence: &CaseEvidence) -> ApiResult<()> {
        self.neo4j
            .run_rows(
                query(
                    "MATCH (e:Evidence {evidence_id: $evidence_id})
                     MATCH (d:CaseDocument {document_id: $document_id})
                     MERGE (e)-[:DERIVED_FROM]->(d)",
                )
                .param("evidence_id", evidence.evidence_id.clone())
                .param("document_id", evidence.document_id.clone()),
            )
            .await?;
        for fact_id in &evidence.supports_fact_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:Evidence {evidence_id: $evidence_id})
                         MATCH (f:Fact {fact_id: $fact_id})
                         MERGE (e)-[:SUPPORTS]->(f)
                         MERGE (f)-[:SUPPORTED_BY]->(e)",
                    )
                    .param("evidence_id", evidence.evidence_id.clone())
                    .param("fact_id", fact_id.clone()),
                )
                .await?;
        }
        for fact_id in &evidence.contradicts_fact_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:Evidence {evidence_id: $evidence_id})
                         MATCH (f:Fact {fact_id: $fact_id})
                         MERGE (e)-[:CONTRADICTS]->(f)
                         MERGE (f)-[:CONTRADICTED_BY]->(e)",
                    )
                    .param("evidence_id", evidence.evidence_id.clone())
                    .param("fact_id", fact_id.clone()),
                )
                .await?;
        }
        for span in &evidence.source_spans {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:Evidence {evidence_id: $evidence_id})
                         MATCH (s:SourceSpan {source_span_id: $source_span_id})
                         MERGE (e)-[:QUOTES]->(s)
                         MERGE (s)-[:SUPPORTS]->(e)",
                    )
                    .param("evidence_id", evidence.evidence_id.clone())
                    .param("source_span_id", span.source_span_id.clone()),
                )
                .await?;
        }
        Ok(())
    }

    async fn materialize_claim_edges(&self, claim: &CaseClaim) -> ApiResult<()> {
        for fact_id in &claim.fact_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:Claim {claim_id: $claim_id})
                         MATCH (f:Fact {fact_id: $fact_id})
                         MERGE (f)-[:SATISFIES_ELEMENT]->(c)
                         MERGE (c)-[:SUPPORTED_BY_FACT]->(f)",
                    )
                    .param("claim_id", claim.claim_id.clone())
                    .param("fact_id", fact_id.clone()),
                )
                .await?;
        }
        for evidence_id in &claim.evidence_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:Claim {claim_id: $claim_id})
                         MATCH (e:Evidence {evidence_id: $evidence_id})
                         MERGE (c)-[:SUPPORTED_BY_EVIDENCE]->(e)",
                    )
                    .param("claim_id", claim.claim_id.clone())
                    .param("evidence_id", evidence_id.clone()),
                )
                .await?;
        }
        for authority in &claim.authorities {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:Claim {claim_id: $claim_id})
                         OPTIONAL MATCH (p:Provision {canonical_id: $canonical_id})
                         OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $canonical_id})
                         WITH c, coalesce(p, i) AS authority
                         FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                           MERGE (c)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                         )",
                    )
                    .param("claim_id", claim.claim_id.clone())
                    .param("canonical_id", authority.canonical_id.clone()),
                )
                .await?;
        }
        for element in &claim.elements {
            let element_payload = to_payload(element)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:Claim {claim_id: $claim_id})
                         MERGE (e:Element {element_id: $element_id})
                         SET e.payload = $payload,
                             e.matter_id = $matter_id,
                             e.element_id = $element_id,
                             e.text = $text
                         MERGE (c)-[:HAS_ELEMENT]->(e)",
                    )
                    .param("claim_id", claim.claim_id.clone())
                    .param("matter_id", claim.matter_id.clone())
                    .param("element_id", element.element_id.clone())
                    .param("text", element.text.clone())
                    .param("payload", element_payload),
                )
                .await?;
            for fact_id in &element.fact_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (e:Element {element_id: $element_id})
                             MATCH (f:Fact {fact_id: $fact_id})
                             MERGE (f)-[:SATISFIES_ELEMENT]->(e)
                             MERGE (e)-[:SUPPORTED_BY_FACT]->(f)",
                        )
                        .param("element_id", element.element_id.clone())
                        .param("fact_id", fact_id.clone()),
                    )
                    .await?;
            }
            for evidence_id in &element.evidence_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (e:Element {element_id: $element_id})
                             MATCH (ev:Evidence {evidence_id: $evidence_id})
                             MERGE (e)-[:SUPPORTED_BY_EVIDENCE]->(ev)",
                        )
                        .param("element_id", element.element_id.clone())
                        .param("evidence_id", evidence_id.clone()),
                    )
                    .await?;
            }
            let mut authorities = element.authorities.clone();
            if let Some(value) = &element.authority {
                push_authority(
                    &mut authorities,
                    AuthorityRef {
                        citation: value.clone(),
                        canonical_id: value.clone(),
                        reason: None,
                        pinpoint: None,
                    },
                );
            }
            for authority in authorities {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (e:Element {element_id: $element_id})
                             OPTIONAL MATCH (p:Provision {canonical_id: $canonical_id})
                             OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $canonical_id})
                             WITH e, coalesce(p, i) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (e)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                             )",
                        )
                        .param("element_id", element.element_id.clone())
                        .param("canonical_id", authority.canonical_id),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    async fn materialize_draft_edges(&self, draft: &CaseDraft) -> ApiResult<()> {
        for paragraph in &draft.paragraphs {
            let payload = to_payload(paragraph)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (d:Draft {draft_id: $draft_id})
                         MERGE (p:DraftParagraph {paragraph_id: $paragraph_id})
                         SET p.payload = $payload,
                             p.matter_id = $matter_id,
                             p.draft_id = $draft_id,
                             p.paragraph_id = $paragraph_id,
                             p.role = $role
                         MERGE (d)-[:HAS_PARAGRAPH]->(p)",
                    )
                    .param("draft_id", draft.draft_id.clone())
                    .param("matter_id", draft.matter_id.clone())
                    .param("paragraph_id", paragraph.paragraph_id.clone())
                    .param("role", paragraph.role.clone())
                    .param("payload", payload),
                )
                .await?;
            for authority in &paragraph.authorities {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:DraftParagraph {paragraph_id: $paragraph_id})
                             OPTIONAL MATCH (provision:Provision {canonical_id: $canonical_id})
                             OPTIONAL MATCH (identity:LegalTextIdentity {canonical_id: $canonical_id})
                             WITH p, coalesce(provision, identity) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (p)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                             )",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("canonical_id", authority.canonical_id.clone()),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    async fn materialize_work_product_edges(&self, product: &WorkProduct) -> ApiResult<()> {
        for block in &product.blocks {
            let payload = work_product_block_graph_payload(
                block,
                self.ast_storage_policy.block_inline_bytes,
            )?;
            let text_excerpt =
                block_text_excerpt(&block.text, self.ast_storage_policy.block_inline_bytes);
            let text_hash = sha256_hex(block.text.as_bytes());
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (w:WorkProduct {work_product_id: $work_product_id})
                         MERGE (b:WorkProductBlock {block_id: $block_id})
                         SET b.payload = $payload,
                             b.matter_id = $matter_id,
                             b.work_product_id = $work_product_id,
                             b.block_id = $block_id,
                             b.role = $role,
                             b.title = $title,
                             b.text = $text,
                             b.text_hash = $text_hash,
                             b.text_size_bytes = $text_size_bytes,
                             b.ordinal = $ordinal
                         MERGE (w)-[:HAS_BLOCK]->(b)
                         WITH b
                         OPTIONAL MATCH (parent:WorkProductBlock {block_id: $parent_block_id})
                         FOREACH (_ IN CASE WHEN parent IS NULL THEN [] ELSE [1] END |
                           MERGE (parent)-[:HAS_CHILD_BLOCK]->(b)
                         )",
                    )
                    .param("work_product_id", product.work_product_id.clone())
                    .param("matter_id", product.matter_id.clone())
                    .param("block_id", block.block_id.clone())
                    .param("role", block.role.clone())
                    .param("title", block.title.clone())
                    .param("text", text_excerpt)
                    .param("text_hash", text_hash)
                    .param("text_size_bytes", block.text.len() as i64)
                    .param("ordinal", block.ordinal as i64)
                    .param(
                        "parent_block_id",
                        block.parent_block_id.clone().unwrap_or_default(),
                    )
                    .param("payload", payload),
                )
                .await?;
            for fact_id in &block.fact_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                             MATCH (f:Fact {fact_id: $fact_id, matter_id: $matter_id})
                             MERGE (b)-[:SUPPORTED_BY_FACT]->(f)",
                        )
                        .param("block_id", block.block_id.clone())
                        .param("matter_id", product.matter_id.clone())
                        .param("fact_id", fact_id.clone()),
                    )
                    .await?;
            }
            for evidence_id in &block.evidence_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                             OPTIONAL MATCH (e:Evidence {evidence_id: $evidence_id, matter_id: $matter_id})
                             OPTIONAL MATCH (d:CaseDocument {document_id: $evidence_id, matter_id: $matter_id})
                             OPTIONAL MATCH (s:SourceSpan {source_span_id: $evidence_id, matter_id: $matter_id})
                             WITH b, coalesce(e, d, s) AS support
                             FOREACH (_ IN CASE WHEN support IS NULL THEN [] ELSE [1] END |
                               MERGE (b)-[:SUPPORTED_BY_EVIDENCE]->(support)
                             )",
                        )
                        .param("block_id", block.block_id.clone())
                        .param("matter_id", product.matter_id.clone())
                        .param("evidence_id", evidence_id.clone()),
                    )
                    .await?;
            }
            for authority in &block.authorities {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                             OPTIONAL MATCH (p:Provision {canonical_id: $canonical_id})
                             OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $canonical_id})
                             WITH b, coalesce(p, i) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (b)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                             )",
                        )
                        .param("block_id", block.block_id.clone())
                        .param("matter_id", product.matter_id.clone())
                        .param("canonical_id", authority.canonical_id.clone()),
                    )
                    .await?;
            }
        }

        for link in &product.document_ast.links {
            match link.target_type.as_str() {
                "fact" => {
                    self.neo4j
                        .run_rows(
                            query(
                                "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                                 MATCH (f:Fact {fact_id: $target_id, matter_id: $matter_id})
                                 MERGE (b)-[:SUPPORTED_BY_FACT]->(f)",
                            )
                            .param("block_id", link.source_block_id.clone())
                            .param("matter_id", product.matter_id.clone())
                            .param("target_id", link.target_id.clone()),
                        )
                        .await?;
                }
                "evidence" | "case_document" | "document_page" | "text_span" | "source_span" => {
                    self.neo4j
                        .run_rows(
                            query(
                                "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (e:Evidence {evidence_id: $target_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (d:CaseDocument {document_id: $target_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (s:SourceSpan {source_span_id: $target_id, matter_id: $matter_id})
                                 WITH b, coalesce(e, d, s) AS support
                                 FOREACH (_ IN CASE WHEN support IS NULL THEN [] ELSE [1] END |
                                   MERGE (b)-[:SUPPORTED_BY_EVIDENCE]->(support)
                                 )",
                            )
                            .param("block_id", link.source_block_id.clone())
                            .param("matter_id", product.matter_id.clone())
                            .param("target_id", link.target_id.clone()),
                        )
                        .await?;
                }
                "legal_authority" | "provision" | "legal_text_identity" | "legal_text" => {
                    self.neo4j
                        .run_rows(
                            query(
                                "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (p:Provision {canonical_id: $target_id})
                                 OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $target_id})
                                 WITH b, coalesce(p, i) AS authority
                                 FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                                   MERGE (b)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                                 )",
                            )
                            .param("block_id", link.source_block_id.clone())
                            .param("matter_id", product.matter_id.clone())
                            .param("target_id", link.target_id.clone()),
                        )
                        .await?;
                }
                "exhibit" => {
                    self.neo4j
                        .run_rows(
                            query(
                                "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (e:Evidence {evidence_id: $target_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (d:CaseDocument {document_id: $target_id, matter_id: $matter_id})
                                 WITH b, coalesce(e, d) AS exhibit
                                 FOREACH (_ IN CASE WHEN exhibit IS NULL THEN [] ELSE [1] END |
                                   MERGE (b)-[:REFERENCES_EXHIBIT]->(exhibit)
                                 )",
                            )
                            .param("block_id", link.source_block_id.clone())
                            .param("matter_id", product.matter_id.clone())
                            .param("target_id", link.target_id.clone()),
                        )
                        .await?;
                }
                _ => {}
            }
        }

        for citation in &product.document_ast.citations {
            if let Some(target_id) = &citation.target_id {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                             OPTIONAL MATCH (p:Provision {canonical_id: $target_id})
                             OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $target_id})
                             WITH b, coalesce(p, i) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (b)-[:CITES]->(authority)
                             )",
                        )
                        .param("block_id", citation.source_block_id.clone())
                        .param("matter_id", product.matter_id.clone())
                        .param("target_id", target_id.clone()),
                    )
                    .await?;
            }
        }

        for anchor in &product.anchors {
            let payload = to_payload(anchor)?;
            let support_use = legal_support_use_from_anchor(product, anchor);
            let support_payload = to_payload(&support_use)?;
            let support_label = support_use_label(&support_use.source_type);
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (w:WorkProduct {work_product_id: $work_product_id, matter_id: $matter_id})
                         MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                         MERGE (a:WorkProductAnchor {anchor_id: $anchor_id})
                         SET a.payload = $payload,
                             a.matter_id = $matter_id,
                             a.work_product_id = $work_product_id,
                             a.anchor_id = $anchor_id,
                             a.anchor_type = $anchor_type,
                             a.target_type = $target_type,
                             a.target_id = $target_id,
                             a.relation = $relation,
                             a.status = $status
                         MERGE (w)-[:HAS_ANCHOR]->(a)
                         MERGE (b)-[:HAS_ANCHOR]->(a)
                         WITH a
                         OPTIONAL MATCH (f:Fact {fact_id: $target_id, matter_id: $matter_id})
                         OPTIONAL MATCH (e:Evidence {evidence_id: $target_id, matter_id: $matter_id})
                         OPTIONAL MATCH (d:CaseDocument {document_id: $target_id, matter_id: $matter_id})
                         OPTIONAL MATCH (p:Provision {canonical_id: $canonical_id})
                         OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $canonical_id})
                         WITH a, coalesce(f, e, d, p, i) AS target
                         FOREACH (_ IN CASE WHEN target IS NULL THEN [] ELSE [1] END |
                           MERGE (a)-[:RESOLVES_TO]->(target)
                         )",
                    )
                    .param("work_product_id", product.work_product_id.clone())
                    .param("matter_id", product.matter_id.clone())
                    .param("block_id", anchor.block_id.clone())
                    .param("anchor_id", anchor.anchor_id.clone())
                    .param("anchor_type", anchor.anchor_type.clone())
                    .param("target_type", anchor.target_type.clone())
                    .param("target_id", anchor.target_id.clone())
                    .param(
                        "canonical_id",
                        anchor.canonical_id.clone().unwrap_or_default(),
                    )
                    .param("relation", anchor.relation.clone())
                    .param("status", anchor.status.clone())
                    .param("payload", payload),
                )
                .await?;
            let support_statement = format!(
                "MATCH (w:WorkProduct {{work_product_id: $work_product_id, matter_id: $matter_id}})
                 MATCH (b:WorkProductBlock {{block_id: $block_id, matter_id: $matter_id}})
                 MERGE (u:LegalSupportUse:{support_label} {{support_use_id: $support_use_id}})
                 SET u.payload = $payload,
                     u.matter_id = $matter_id,
                     u.subject_id = $work_product_id,
                     u.branch_id = $branch_id,
                     u.target_type = $target_type,
                     u.target_id = $target_id,
                     u.source_type = $source_type,
                     u.source_id = $source_id,
                     u.relation = $relation,
                     u.status = $status
                 MERGE (w)-[:HAS_SUPPORT_USE]->(u)
                 MERGE (b)-[:HAS_SUPPORT_USE]->(u)",
            );
            self.neo4j
                .run_rows(
                    query(&support_statement)
                        .param("work_product_id", product.work_product_id.clone())
                        .param("block_id", anchor.block_id.clone())
                        .param("matter_id", product.matter_id.clone())
                        .param("branch_id", support_use.branch_id.clone())
                        .param("support_use_id", support_use.support_use_id.clone())
                        .param("target_type", support_use.target_type.clone())
                        .param("target_id", support_use.target_id.clone())
                        .param("source_type", support_use.source_type.clone())
                        .param("source_id", support_use.source_id.clone())
                        .param("relation", support_use.relation.clone())
                        .param("status", support_use.status.clone())
                        .param("payload", support_payload),
                )
                .await?;
            self.materialize_support_use_target(&support_use).await?;
        }

        for mark in &product.marks {
            let payload = to_payload(mark)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                         MERGE (m:WorkProductMark {mark_id: $mark_id})
                         SET m.payload = $payload,
                             m.matter_id = $matter_id,
                             m.work_product_id = $work_product_id,
                             m.mark_id = $mark_id,
                             m.mark_type = $mark_type,
                             m.target_type = $target_type,
                             m.target_id = $target_id,
                             m.status = $status
                         MERGE (b)-[:HAS_MARK]->(m)",
                    )
                    .param("block_id", mark.block_id.clone())
                    .param("matter_id", product.matter_id.clone())
                    .param("work_product_id", product.work_product_id.clone())
                    .param("mark_id", mark.mark_id.clone())
                    .param("mark_type", mark.mark_type.clone())
                    .param("target_type", mark.target_type.clone())
                    .param("target_id", mark.target_id.clone())
                    .param("status", mark.status.clone())
                    .param("payload", payload),
                )
                .await?;
        }

        for finding in &product.findings {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (w:WorkProduct {work_product_id: $work_product_id, matter_id: $matter_id})
                         MERGE (f:WorkProductFinding {finding_id: $finding_id})
                         SET f.matter_id = $matter_id,
                             f.work_product_id = $work_product_id,
                             f.finding_id = $finding_id,
                             f.status = $status,
                             f.severity = $severity,
                             f.category = $category
                         MERGE (w)-[:HAS_FINDING]->(f)",
                    )
                    .param("work_product_id", product.work_product_id.clone())
                    .param("matter_id", product.matter_id.clone())
                    .param("finding_id", finding.finding_id.clone())
                    .param("status", finding.status.clone())
                    .param("severity", finding.severity.clone())
                    .param("category", finding.category.clone()),
                )
                .await?;
        }

        Ok(())
    }

    async fn materialize_case_history_edges(
        &self,
        work_product_id: &str,
        branch: &VersionBranch,
        snapshot: &VersionSnapshot,
        manifest: &SnapshotManifest,
        entity_states: &[SnapshotEntityState],
        changes: &[VersionChange],
        change_set: &ChangeSet,
    ) -> ApiResult<()> {
        self.neo4j
            .run_rows(
                query(
                    "MATCH (w:WorkProduct {work_product_id: $work_product_id})
                     MATCH (b:VersionBranch {branch_id: $branch_id})
                     MATCH (s:VersionSnapshot {snapshot_id: $snapshot_id})
                     MATCH (cs:ChangeSet {change_set_id: $change_set_id})
                     MERGE (w)-[:HAS_BRANCH]->(b)
                     MERGE (b)-[:HAS_SNAPSHOT]->(s)
                     MERGE (b)-[:CURRENT_SNAPSHOT]->(s)
                     MERGE (s)-[:CREATED_BY_CHANGESET]->(cs)",
                )
                .param("work_product_id", work_product_id)
                .param("branch_id", branch.branch_id.clone())
                .param("snapshot_id", snapshot.snapshot_id.clone())
                .param("change_set_id", change_set.change_set_id.clone()),
            )
            .await?;
        for parent_id in &snapshot.parent_snapshot_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (s:VersionSnapshot {snapshot_id: $snapshot_id})
                         MATCH (p:VersionSnapshot {snapshot_id: $parent_id})
                         MERGE (s)-[:HAS_PARENT]->(p)",
                    )
                    .param("snapshot_id", snapshot.snapshot_id.clone())
                    .param("parent_id", parent_id.clone()),
                )
                .await?;
        }
        self.neo4j
            .run_rows(
                query(
                    "MATCH (s:VersionSnapshot {snapshot_id: $snapshot_id})
                     MATCH (m:SnapshotManifest {manifest_id: $manifest_id})
                     MERGE (s)-[:HAS_MANIFEST]->(m)",
                )
                .param("snapshot_id", snapshot.snapshot_id.clone())
                .param("manifest_id", manifest.manifest_id.clone()),
            )
            .await?;
        for object_blob_id in [
            snapshot.full_state_ref.as_ref(),
            snapshot.manifest_ref.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (s:VersionSnapshot {snapshot_id: $snapshot_id})
                         MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                         MERGE (s)-[:STORED_AS]->(b)",
                    )
                    .param("snapshot_id", snapshot.snapshot_id.clone())
                    .param("object_blob_id", object_blob_id.clone()),
                )
                .await?;
        }
        for state in entity_states {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (m:SnapshotManifest {manifest_id: $manifest_id})
                         MATCH (es:SnapshotEntityState {entity_state_id: $entity_state_id})
                         MERGE (m)-[:HAS_ENTITY_STATE]->(es)",
                    )
                    .param("manifest_id", manifest.manifest_id.clone())
                    .param("entity_state_id", state.entity_state_id.clone()),
                )
                .await?;
            if let Some(object_blob_id) = state.state_ref.as_ref() {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (es:SnapshotEntityState {entity_state_id: $entity_state_id})
                             MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                             MERGE (es)-[:STORED_AS]->(b)",
                        )
                        .param("entity_state_id", state.entity_state_id.clone())
                        .param("object_blob_id", object_blob_id.clone()),
                    )
                    .await?;
            }
        }
        for change in changes {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (cs:ChangeSet {change_set_id: $change_set_id})
                         MATCH (c:VersionChange {change_id: $change_id})
                         MERGE (cs)-[:HAS_CHANGE]->(c)",
                    )
                    .param("change_set_id", change_set.change_set_id.clone())
                    .param("change_id", change.change_id.clone()),
                )
                .await?;
            if let Some(ai_audit_id) = &change.ai_audit_id {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (c:VersionChange {change_id: $change_id})
                             MATCH (a:AIEditAudit {ai_audit_id: $ai_audit_id})
                             MERGE (c)-[:AI_AUDIT]->(a)",
                        )
                        .param("change_id", change.change_id.clone())
                        .param("ai_audit_id", ai_audit_id.clone()),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    async fn materialize_version_subject(&self, product: &WorkProduct) -> ApiResult<()> {
        self.neo4j
            .run_rows(
                query(
                    "MATCH (m:Matter {matter_id: $matter_id})
                     MATCH (w:WorkProduct {work_product_id: $work_product_id})
                     MERGE (m)-[:HAS_VERSION_SUBJECT]->(w)",
                )
                .param("matter_id", product.matter_id.clone())
                .param("work_product_id", product.work_product_id.clone()),
            )
            .await?;
        Ok(())
    }

    async fn materialize_support_use_target(&self, support_use: &LegalSupportUse) -> ApiResult<()> {
        match support_use.source_type.as_str() {
            "fact" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (u:LegalSupportUse {support_use_id: $support_use_id, matter_id: $matter_id})
                             MATCH (f:Fact {fact_id: $source_id, matter_id: $matter_id})
                             MERGE (u)-[:USES_FACT]->(f)",
                        )
                        .param("support_use_id", support_use.support_use_id.clone())
                        .param("matter_id", support_use.matter_id.clone())
                        .param("source_id", support_use.source_id.clone()),
                    )
                    .await?;
            }
            "evidence" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (u:LegalSupportUse {support_use_id: $support_use_id, matter_id: $matter_id})
                             MATCH (e:Evidence {evidence_id: $source_id, matter_id: $matter_id})
                             MERGE (u)-[:USES_EVIDENCE]->(e)",
                        )
                        .param("support_use_id", support_use.support_use_id.clone())
                        .param("matter_id", support_use.matter_id.clone())
                        .param("source_id", support_use.source_id.clone()),
                    )
                    .await?;
            }
            "source_span" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (u:LegalSupportUse {support_use_id: $support_use_id, matter_id: $matter_id})
                             MATCH (s:SourceSpan {source_span_id: $source_id, matter_id: $matter_id})
                             MERGE (u)-[:USES_SPAN]->(s)",
                        )
                        .param("support_use_id", support_use.support_use_id.clone())
                        .param("matter_id", support_use.matter_id.clone())
                        .param("source_id", support_use.source_id.clone()),
                    )
                    .await?;
            }
            "authority" | "provision" | "citation" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (u:LegalSupportUse {support_use_id: $support_use_id, matter_id: $matter_id})
                             OPTIONAL MATCH (p:Provision {canonical_id: $source_id})
                             OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $source_id})
                             WITH u, coalesce(p, i) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (u)-[:USES_AUTHORITY]->(authority)
                             )",
                        )
                        .param("support_use_id", support_use.support_use_id.clone())
                        .param("matter_id", support_use.matter_id.clone())
                        .param("source_id", support_use.source_id.clone()),
                    )
                    .await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn materialize_complaint_edges(&self, complaint: &ComplaintDraft) -> ApiResult<()> {
        for section in &complaint.sections {
            let payload = to_payload(section)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:ComplaintDraft {complaint_id: $complaint_id})
                         MERGE (s:ComplaintSection {section_id: $section_id})
                         SET s.payload = $payload,
                             s.matter_id = $matter_id,
                             s.complaint_id = $complaint_id,
                             s.section_id = $section_id,
                             s.title = $title
                         MERGE (c)-[:HAS_SECTION]->(s)",
                    )
                    .param("complaint_id", complaint.complaint_id.clone())
                    .param("matter_id", complaint.matter_id.clone())
                    .param("section_id", section.section_id.clone())
                    .param("title", section.title.clone())
                    .param("payload", payload),
                )
                .await?;
        }

        for count in &complaint.counts {
            let payload = to_payload(count)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:ComplaintDraft {complaint_id: $complaint_id})
                         MERGE (n:ComplaintCount {count_id: $count_id})
                         SET n.payload = $payload,
                             n.matter_id = $matter_id,
                             n.complaint_id = $complaint_id,
                             n.count_id = $count_id,
                             n.title = $title,
                             n.legal_theory = $legal_theory
                         MERGE (c)-[:HAS_COUNT]->(n)
                         WITH n
                         OPTIONAL MATCH (claim:Claim {claim_id: $claim_id})
                         FOREACH (_ IN CASE WHEN claim IS NULL THEN [] ELSE [1] END |
                           MERGE (n)-[:IMPLEMENTS_CLAIM]->(claim)
                         )",
                    )
                    .param("complaint_id", complaint.complaint_id.clone())
                    .param("matter_id", complaint.matter_id.clone())
                    .param("count_id", count.count_id.clone())
                    .param("title", count.title.clone())
                    .param("legal_theory", count.legal_theory.clone())
                    .param("claim_id", count.claim_id.clone().unwrap_or_default())
                    .param("payload", payload),
                )
                .await?;
            for fact_id in &count.fact_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (n:ComplaintCount {count_id: $count_id})
                             MATCH (f:Fact {fact_id: $fact_id})
                             MERGE (n)-[:SUPPORTED_BY_FACT]->(f)",
                        )
                        .param("count_id", count.count_id.clone())
                        .param("fact_id", fact_id.clone()),
                    )
                    .await?;
            }
            for evidence_id in &count.evidence_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (n:ComplaintCount {count_id: $count_id})
                             MATCH (e:Evidence {evidence_id: $evidence_id})
                             MERGE (n)-[:SUPPORTED_BY_EVIDENCE]->(e)",
                        )
                        .param("count_id", count.count_id.clone())
                        .param("evidence_id", evidence_id.clone()),
                    )
                    .await?;
            }
            for authority in &count.authorities {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (n:ComplaintCount {count_id: $count_id})
                             OPTIONAL MATCH (p:Provision {canonical_id: $canonical_id})
                             OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $canonical_id})
                             WITH n, coalesce(p, i) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (n)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                             )",
                        )
                        .param("count_id", count.count_id.clone())
                        .param("canonical_id", authority.canonical_id.clone()),
                    )
                    .await?;
            }
        }

        for paragraph in &complaint.paragraphs {
            let payload = to_payload(paragraph)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:ComplaintDraft {complaint_id: $complaint_id})
                         MERGE (p:PleadingParagraph {paragraph_id: $paragraph_id})
                         SET p.payload = $payload,
                             p.matter_id = $matter_id,
                             p.complaint_id = $complaint_id,
                             p.paragraph_id = $paragraph_id,
                             p.number = $number,
                             p.display_number = $display_number,
                             p.original_label = $original_label,
                             p.source_span_id = $source_span_id,
                             p.role = $role,
                             p.text = $text
                         MERGE (c)-[:HAS_PARAGRAPH]->(p)
                         WITH p
                         OPTIONAL MATCH (s:ComplaintSection {section_id: $section_id})
                         OPTIONAL MATCH (n:ComplaintCount {count_id: $count_id})
                         OPTIONAL MATCH (source_span:SourceSpan {source_span_id: $source_span_id})
                         FOREACH (_ IN CASE WHEN s IS NULL THEN [] ELSE [1] END |
                           MERGE (s)-[:HAS_PARAGRAPH]->(p)
                         )
                         FOREACH (_ IN CASE WHEN n IS NULL THEN [] ELSE [1] END |
                           MERGE (n)-[:HAS_PARAGRAPH]->(p)
                         )
                         FOREACH (_ IN CASE WHEN source_span IS NULL THEN [] ELSE [1] END |
                           MERGE (p)-[:DERIVED_FROM]->(source_span)
                         )",
                    )
                    .param("complaint_id", complaint.complaint_id.clone())
                    .param("matter_id", complaint.matter_id.clone())
                    .param("paragraph_id", paragraph.paragraph_id.clone())
                    .param("number", paragraph.number as i64)
                    .param(
                        "display_number",
                        paragraph
                            .display_number
                            .clone()
                            .unwrap_or_else(|| paragraph.number.to_string()),
                    )
                    .param(
                        "original_label",
                        paragraph.original_label.clone().unwrap_or_default(),
                    )
                    .param(
                        "source_span_id",
                        paragraph.source_span_id.clone().unwrap_or_default(),
                    )
                    .param("role", paragraph.role.clone())
                    .param("text", paragraph.text.clone())
                    .param(
                        "section_id",
                        paragraph.section_id.clone().unwrap_or_default(),
                    )
                    .param("count_id", paragraph.count_id.clone().unwrap_or_default())
                    .param("payload", payload),
                )
                .await?;
            for sentence in &paragraph.sentences {
                let payload = to_payload(sentence)?;
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:PleadingParagraph {paragraph_id: $paragraph_id})
                             MERGE (s:PleadingSentence {sentence_id: $sentence_id})
                             SET s.payload = $payload,
                                 s.matter_id = $matter_id,
                                 s.complaint_id = $complaint_id,
                                 s.paragraph_id = $paragraph_id,
                                 s.sentence_id = $sentence_id,
                                 s.text = $text
                             MERGE (p)-[:HAS_SENTENCE]->(s)",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("matter_id", complaint.matter_id.clone())
                        .param("complaint_id", complaint.complaint_id.clone())
                        .param("sentence_id", sentence.sentence_id.clone())
                        .param("text", sentence.text.clone())
                        .param("payload", payload),
                    )
                    .await?;
            }
            for fact_id in &paragraph.fact_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:PleadingParagraph {paragraph_id: $paragraph_id})
                             MATCH (f:Fact {fact_id: $fact_id})
                             MERGE (p)-[:SUPPORTED_BY_FACT]->(f)",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("fact_id", fact_id.clone()),
                    )
                    .await?;
            }
            for evidence_use in &paragraph.evidence_uses {
                let payload = to_payload(evidence_use)?;
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:PleadingParagraph {paragraph_id: $paragraph_id})
                             MERGE (u:EvidenceUse {evidence_use_id: $evidence_use_id})
                             SET u.payload = $payload,
                                 u.matter_id = $matter_id,
                                 u.complaint_id = $complaint_id,
                                 u.evidence_use_id = $evidence_use_id,
                                 u.relation = $relation
                             MERGE (p)-[:HAS_EVIDENCE_USE]->(u)
                             WITH u
                             OPTIONAL MATCH (e:Evidence {evidence_id: $evidence_id})
                             OPTIONAL MATCH (d:CaseDocument {document_id: $document_id})
                             OPTIONAL MATCH (span:SourceSpan {source_span_id: $source_span_id})
                             FOREACH (_ IN CASE WHEN e IS NULL THEN [] ELSE [1] END |
                               MERGE (u)-[:USES_EVIDENCE]->(e)
                             )
                             FOREACH (_ IN CASE WHEN d IS NULL THEN [] ELSE [1] END |
                               MERGE (u)-[:USES_DOCUMENT]->(d)
                             )
                             FOREACH (_ IN CASE WHEN span IS NULL THEN [] ELSE [1] END |
                               MERGE (u)-[:USES_SPAN]->(span)
                             )",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("matter_id", complaint.matter_id.clone())
                        .param("complaint_id", complaint.complaint_id.clone())
                        .param("evidence_use_id", evidence_use.evidence_use_id.clone())
                        .param("relation", evidence_use.relation.clone())
                        .param(
                            "evidence_id",
                            evidence_use.evidence_id.clone().unwrap_or_default(),
                        )
                        .param(
                            "document_id",
                            evidence_use.document_id.clone().unwrap_or_default(),
                        )
                        .param(
                            "source_span_id",
                            evidence_use.source_span_id.clone().unwrap_or_default(),
                        )
                        .param("payload", payload),
                    )
                    .await?;
            }
            for citation_use in &paragraph.citation_uses {
                let payload = to_payload(citation_use)?;
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:PleadingParagraph {paragraph_id: $paragraph_id})
                             MERGE (u:CitationUse {citation_use_id: $citation_use_id})
                             SET u.payload = $payload,
                                 u.matter_id = $matter_id,
                                 u.complaint_id = $complaint_id,
                                 u.citation_use_id = $citation_use_id,
                                 u.citation = $citation,
                                 u.status = $status
                             MERGE (p)-[:HAS_CITATION_USE]->(u)
                             WITH u
                             OPTIONAL MATCH (provision:Provision {canonical_id: $canonical_id})
                             OPTIONAL MATCH (identity:LegalTextIdentity {canonical_id: $canonical_id})
                             WITH u, coalesce(provision, identity) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (u)-[:RESOLVES_TO]->(authority)
                             )
                             WITH u, authority
                             FOREACH (_ IN CASE WHEN authority IS NULL AND $canonical_id <> '' AND $status <> 'unresolved' THEN [1] ELSE [] END |
                               MERGE (external:ExternalAuthority {canonical_id: $canonical_id})
	                               SET external.canonical_id = $canonical_id,
	                                   external.citation = $citation,
	                                   external.authority_type = $authority_type,
	                                   external.source = 'source_backed_external_rule',
	                                   external.source_url = $source_url,
	                                   external.edition = $authority_edition,
	                                   external.currentness = $currentness
	                               MERGE (u)-[:RESOLVES_TO]->(external)
	                             )",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("matter_id", complaint.matter_id.clone())
                        .param("complaint_id", complaint.complaint_id.clone())
	                        .param("citation_use_id", citation_use.citation_use_id.clone())
	                        .param("citation", citation_use.citation.clone())
	                        .param("status", citation_use.status.clone())
	                        .param("currentness", citation_use.currentness.clone())
	                        .param("authority_type", authority_type_for_citation(&citation_use.citation))
	                        .param("source_url", authority_source_url_for_citation(&citation_use.citation))
	                        .param(
	                            "authority_edition",
	                            authority_edition_for_citation(&citation_use.citation),
	                        )
	                        .param("canonical_id", citation_use.canonical_id.clone().unwrap_or_default())
	                        .param("payload", payload),
	                )
                    .await?;
            }
            for exhibit_reference in &paragraph.exhibit_references {
                let payload = to_payload(exhibit_reference)?;
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:PleadingParagraph {paragraph_id: $paragraph_id})
                             MERGE (x:ExhibitReference {exhibit_reference_id: $exhibit_reference_id})
                             SET x.payload = $payload,
                                 x.matter_id = $matter_id,
                                 x.complaint_id = $complaint_id,
                                 x.exhibit_reference_id = $exhibit_reference_id,
                                 x.exhibit_label = $exhibit_label,
                                 x.status = $status
                             MERGE (p)-[:HAS_EXHIBIT_REFERENCE]->(x)
                             WITH x
                             OPTIONAL MATCH (d:CaseDocument {document_id: $document_id})
                             OPTIONAL MATCH (e:Evidence {evidence_id: $evidence_id})
                             FOREACH (_ IN CASE WHEN d IS NULL THEN [] ELSE [1] END |
                               MERGE (x)-[:REFERENCES_DOCUMENT]->(d)
                             )
                             FOREACH (_ IN CASE WHEN e IS NULL THEN [] ELSE [1] END |
                               MERGE (x)-[:REFERENCES_EVIDENCE]->(e)
                             )",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("matter_id", complaint.matter_id.clone())
                        .param("complaint_id", complaint.complaint_id.clone())
                        .param("exhibit_reference_id", exhibit_reference.exhibit_reference_id.clone())
                        .param("exhibit_label", exhibit_reference.exhibit_label.clone())
                        .param("status", exhibit_reference.status.clone())
                        .param("document_id", exhibit_reference.document_id.clone().unwrap_or_default())
                        .param("evidence_id", exhibit_reference.evidence_id.clone().unwrap_or_default())
                        .param("payload", payload),
                    )
                    .await?;
            }
        }

        for relief in &complaint.relief {
            let payload = to_payload(relief)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:ComplaintDraft {complaint_id: $complaint_id})
                         MERGE (r:ReliefRequest {relief_id: $relief_id})
                         SET r.payload = $payload,
                             r.matter_id = $matter_id,
                             r.complaint_id = $complaint_id,
                             r.relief_id = $relief_id,
                             r.category = $category,
                             r.text = $text
                         MERGE (c)-[:REQUESTS_RELIEF]->(r)",
                    )
                    .param("complaint_id", complaint.complaint_id.clone())
                    .param("matter_id", complaint.matter_id.clone())
                    .param("relief_id", relief.relief_id.clone())
                    .param("category", relief.category.clone())
                    .param("text", relief.text.clone())
                    .param("payload", payload),
                )
                .await?;
        }

        for finding in &complaint.findings {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:ComplaintDraft {complaint_id: $complaint_id})
                         MERGE (f:RuleCheckFinding {finding_id: $finding_id})
                         SET f.matter_id = $matter_id,
                             f.complaint_id = $complaint_id,
                             f.finding_id = $finding_id,
                             f.status = $status,
                             f.severity = $severity,
                             f.category = $category
                         MERGE (c)-[:HAS_RULE_FINDING]->(f)",
                    )
                    .param("complaint_id", complaint.complaint_id.clone())
                    .param("matter_id", complaint.matter_id.clone())
                    .param("finding_id", finding.finding_id.clone())
                    .param("status", finding.status.clone())
                    .param("severity", finding.severity.clone())
                    .param("category", finding.category.clone()),
                )
                .await?;
        }

        Ok(())
    }

    async fn sync_fact_evidence_link(
        &self,
        matter_id: &str,
        evidence_id: &str,
        fact_id: &str,
        relation: &str,
    ) -> ApiResult<()> {
        let mut fact = self
            .get_node::<CaseFact>(matter_id, fact_spec(), fact_id)
            .await?;
        if relation == "contradicts" {
            push_unique(
                &mut fact.contradicted_by_evidence_ids,
                evidence_id.to_string(),
            );
            fact.status = "contradicted".to_string();
            fact.needs_verification = true;
        } else {
            push_unique(&mut fact.source_evidence_ids, evidence_id.to_string());
            if matches!(
                fact.status.as_str(),
                "proposed" | "alleged" | "needs_evidence"
            ) {
                fact.status = "supported".to_string();
            }
            fact.confidence = fact.confidence.max(0.8);
            fact.needs_verification = false;
        }
        let fact = self
            .merge_node(matter_id, fact_spec(), fact_id, &fact)
            .await?;
        self.materialize_fact_edges(&fact).await
    }

    async fn sync_claim_element_evidence(
        &self,
        matter_id: &str,
        evidence_id: &str,
        fact_id: &str,
    ) -> ApiResult<()> {
        for mut claim in self.list_claims(matter_id).await? {
            let mut changed = false;
            for element in &mut claim.elements {
                if element.fact_ids.contains(&fact_id.to_string()) {
                    push_unique(&mut element.evidence_ids, evidence_id.to_string());
                    element.satisfied = true;
                    changed = true;
                }
            }
            if claim.fact_ids.contains(&fact_id.to_string()) {
                push_unique(&mut claim.evidence_ids, evidence_id.to_string());
                changed = true;
            }
            if changed {
                let claim = self
                    .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
                    .await?;
                self.materialize_claim_edges(&claim).await?;
            }
        }
        Ok(())
    }

    async fn detach_authority_edge(
        &self,
        label: &str,
        id_key: &str,
        id: &str,
        authority: &AuthorityRef,
    ) -> ApiResult<()> {
        let statement = format!(
            "MATCH (n:{label} {{{id_key}: $id}})-[r:SUPPORTED_BY_AUTHORITY]->(authority)
             WHERE authority.canonical_id = $canonical_id
             DELETE r",
            label = label,
            id_key = id_key,
        );
        self.neo4j
            .run_rows(
                query(&statement)
                    .param("id", id.to_string())
                    .param("canonical_id", authority.canonical_id.clone()),
            )
            .await?;
        Ok(())
    }

    async fn document_bytes_as_text(&self, document: &CaseDocument) -> ApiResult<String> {
        if document.storage_status == "deleted" {
            return Ok(String::new());
        }
        if let Some(key) = document.storage_key.as_deref() {
            let bytes = self.object_store.get_bytes(key).await?;
            return Ok(parse_document_bytes(
                &document.filename,
                document.mime_type.as_deref(),
                &bytes,
            )
            .text
            .unwrap_or_default());
        }
        if let Some(path) = document.storage_path.as_deref() {
            let bytes = fs::read(path).await.map_err(io_error)?;
            return Ok(parse_document_bytes(
                &document.filename,
                document.mime_type.as_deref(),
                &bytes,
            )
            .text
            .unwrap_or_default());
        }
        Ok(String::new())
    }

    fn ensure_upload_size(&self, bytes: u64) -> ApiResult<()> {
        if bytes > self.max_upload_bytes {
            Err(ApiError::BadRequest(format!(
                "Upload is {bytes} bytes; maximum is {} bytes",
                self.max_upload_bytes
            )))
        } else {
            Ok(())
        }
    }
}

fn party_spec() -> NodeSpec {
    NodeSpec {
        label: "Party",
        id_key: "party_id",
        edge: "HAS_PARTY",
    }
}
fn document_spec() -> NodeSpec {
    NodeSpec {
        label: "CaseDocument",
        id_key: "document_id",
        edge: "HAS_DOCUMENT",
    }
}
fn fact_spec() -> NodeSpec {
    NodeSpec {
        label: "Fact",
        id_key: "fact_id",
        edge: "HAS_FACT",
    }
}
fn timeline_spec() -> NodeSpec {
    NodeSpec {
        label: "TimelineEvent",
        id_key: "event_id",
        edge: "HAS_EVENT",
    }
}
fn evidence_spec() -> NodeSpec {
    NodeSpec {
        label: "Evidence",
        id_key: "evidence_id",
        edge: "HAS_EVIDENCE",
    }
}
fn claim_spec() -> NodeSpec {
    NodeSpec {
        label: "Claim",
        id_key: "claim_id",
        edge: "HAS_CLAIM",
    }
}
fn defense_spec() -> NodeSpec {
    NodeSpec {
        label: "Defense",
        id_key: "defense_id",
        edge: "HAS_DEFENSE",
    }
}
fn deadline_spec() -> NodeSpec {
    NodeSpec {
        label: "DeadlineInstance",
        id_key: "deadline_id",
        edge: "HAS_DEADLINE",
    }
}
fn task_spec() -> NodeSpec {
    NodeSpec {
        label: "Task",
        id_key: "task_id",
        edge: "HAS_TASK",
    }
}
fn draft_spec() -> NodeSpec {
    NodeSpec {
        label: "Draft",
        id_key: "draft_id",
        edge: "HAS_DRAFT",
    }
}
fn fact_check_finding_spec() -> NodeSpec {
    NodeSpec {
        label: "FactCheckFinding",
        id_key: "finding_id",
        edge: "HAS_FACT_CHECK_FINDING",
    }
}
fn citation_check_finding_spec() -> NodeSpec {
    NodeSpec {
        label: "CitationCheckFinding",
        id_key: "finding_id",
        edge: "HAS_CITATION_CHECK_FINDING",
    }
}
fn document_version_spec() -> NodeSpec {
    NodeSpec {
        label: "DocumentVersion",
        id_key: "document_version_id",
        edge: "HAS_DOCUMENT_VERSION",
    }
}
fn ingestion_run_spec() -> NodeSpec {
    NodeSpec {
        label: "IngestionRun",
        id_key: "ingestion_run_id",
        edge: "HAS_INGESTION_RUN",
    }
}
fn source_span_spec() -> NodeSpec {
    NodeSpec {
        label: "SourceSpan",
        id_key: "source_span_id",
        edge: "HAS_SOURCE_SPAN",
    }
}
fn complaint_spec() -> NodeSpec {
    NodeSpec {
        label: "ComplaintDraft",
        id_key: "complaint_id",
        edge: "HAS_COMPLAINT",
    }
}
fn complaint_finding_spec() -> NodeSpec {
    NodeSpec {
        label: "RuleCheckFinding",
        id_key: "finding_id",
        edge: "HAS_RULE_CHECK_FINDING",
    }
}
fn complaint_artifact_spec() -> NodeSpec {
    NodeSpec {
        label: "ExportArtifact",
        id_key: "artifact_id",
        edge: "HAS_EXPORT_ARTIFACT",
    }
}
fn work_product_spec() -> NodeSpec {
    NodeSpec {
        label: "WorkProduct",
        id_key: "work_product_id",
        edge: "HAS_WORK_PRODUCT",
    }
}
fn work_product_finding_spec() -> NodeSpec {
    NodeSpec {
        label: "WorkProductFinding",
        id_key: "finding_id",
        edge: "HAS_WORK_PRODUCT_FINDING",
    }
}
fn work_product_artifact_spec() -> NodeSpec {
    NodeSpec {
        label: "WorkProductArtifact",
        id_key: "artifact_id",
        edge: "HAS_WORK_PRODUCT_ARTIFACT",
    }
}
fn change_set_spec() -> NodeSpec {
    NodeSpec {
        label: "ChangeSet",
        id_key: "change_set_id",
        edge: "HAS_CHANGE_SET",
    }
}
fn version_snapshot_spec() -> NodeSpec {
    NodeSpec {
        label: "VersionSnapshot",
        id_key: "snapshot_id",
        edge: "HAS_VERSION_SNAPSHOT",
    }
}
fn snapshot_manifest_spec() -> NodeSpec {
    NodeSpec {
        label: "SnapshotManifest",
        id_key: "manifest_id",
        edge: "HAS_SNAPSHOT_MANIFEST",
    }
}
fn snapshot_entity_state_spec() -> NodeSpec {
    NodeSpec {
        label: "SnapshotEntityState",
        id_key: "entity_state_id",
        edge: "HAS_SNAPSHOT_ENTITY_STATE",
    }
}
fn version_change_spec() -> NodeSpec {
    NodeSpec {
        label: "VersionChange",
        id_key: "change_id",
        edge: "HAS_VERSION_CHANGE",
    }
}
fn version_branch_spec() -> NodeSpec {
    NodeSpec {
        label: "VersionBranch",
        id_key: "branch_id",
        edge: "HAS_VERSION_BRANCH",
    }
}
fn legal_support_use_spec() -> NodeSpec {
    NodeSpec {
        label: "LegalSupportUse",
        id_key: "support_use_id",
        edge: "HAS_LEGAL_SUPPORT_USE",
    }
}
fn ai_edit_audit_spec() -> NodeSpec {
    NodeSpec {
        label: "AIEditAudit",
        id_key: "ai_audit_id",
        edge: "HAS_AI_EDIT_AUDIT",
    }
}

fn matter_reference_error(error: ApiError, target_type: &str) -> ApiError {
    match error {
        ApiError::NotFound(_) => {
            ApiError::NotFound(format!("Matter-owned {target_type} reference not found"))
        }
        other => other,
    }
}

fn normalize_work_product_type(value: &str) -> ApiResult<String> {
    let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
    let supported = [
        "complaint",
        "motion",
        "answer",
        "declaration",
        "brief",
        "demand_letter",
        "legal_memo",
        "exhibit_list",
        "notice",
        "proposed_order",
    ];
    if supported.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(ApiError::BadRequest(format!(
            "Unsupported work product type {value}"
        )))
    }
}

fn humanize_product_type(value: &str) -> String {
    value
        .replace('_', " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn default_work_product_from_matter(
    matter: &MatterSummary,
    work_product_id: &str,
    title: &str,
    product_type: &str,
    facts: &[CaseFact],
    claims: &[CaseClaim],
    now: &str,
) -> WorkProduct {
    let blocks = match product_type {
        "complaint" => complaint_work_product_blocks(matter, work_product_id, facts, claims),
        "motion" => motion_blocks(matter, work_product_id, facts, claims),
        "answer" => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "responses",
                    "Responses to allegations",
                    "Build the admit, deny, and lack-knowledge responses from the allegation grid.",
                ),
                (
                    "affirmative_defenses",
                    "Affirmative defenses",
                    "Add defenses, supporting facts, evidence, and authority.",
                ),
                (
                    "counterclaims",
                    "Counterclaims",
                    "Add any counterclaims or mark this section intentionally omitted.",
                ),
                ("prayer", "Prayer for relief", "State the requested relief."),
            ],
        ),
        "declaration" => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "declarant",
                    "Declarant identity",
                    "Identify the declarant and basis of personal knowledge.",
                ),
                (
                    "facts",
                    "Declaration facts",
                    "Add numbered factual statements supported by evidence.",
                ),
                (
                    "signature",
                    "Signature",
                    "Add date, place, signature, and review required language.",
                ),
            ],
        ),
        "exhibit_list" => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "exhibits",
                    "Exhibits",
                    "List exhibits with stable labels and source documents.",
                ),
                (
                    "foundation",
                    "Foundation notes",
                    "Track authentication and admissibility notes for each exhibit.",
                ),
            ],
        ),
        _ => profile_blocks(
            &matter.matter_id,
            work_product_id,
            &[
                (
                    "summary",
                    "Summary",
                    "Describe the purpose of this work product.",
                ),
                (
                    "facts",
                    "Relevant facts",
                    "Link facts and evidence before relying on this text.",
                ),
                (
                    "analysis",
                    "Analysis",
                    "Add source-backed legal analysis or argument.",
                ),
                (
                    "conclusion",
                    "Conclusion",
                    "Add requested next step or relief.",
                ),
            ],
        ),
    };
    let mut product = WorkProduct {
        id: work_product_id.to_string(),
        work_product_id: work_product_id.to_string(),
        matter_id: matter.matter_id.clone(),
        title: title.to_string(),
        product_type: product_type.to_string(),
        status: "draft".to_string(),
        review_status: "needs_human_review".to_string(),
        setup_stage: "guided_setup".to_string(),
        source_draft_id: None,
        source_complaint_id: None,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        profile: work_product_profile(product_type),
        document_ast: WorkProductDocument::default(),
        blocks,
        marks: Vec::new(),
        anchors: Vec::new(),
        findings: Vec::new(),
        artifacts: Vec::new(),
        history: vec![work_product_event(
            &matter.matter_id,
            work_product_id,
            "work_product_created",
            "work_product",
            work_product_id,
            "Shared WorkProduct AST created.",
        )],
        ai_commands: default_work_product_ai_commands(product_type),
        formatting_profile: default_work_product_formatting_profile(product_type),
        rule_pack: work_product_rule_pack(product_type),
    };
    apply_matter_rule_profile(
        &mut product.rule_pack,
        matter,
        now.get(0..10).unwrap_or(now),
        product_type,
    );
    refresh_work_product_state(&mut product);
    product
}

fn complaint_work_product_blocks(
    matter: &MatterSummary,
    work_product_id: &str,
    facts: &[CaseFact],
    claims: &[CaseClaim],
) -> Vec<WorkProductBlock> {
    let fact_text = if facts.is_empty() {
        "Add numbered, evidence-supported factual allegations.".to_string()
    } else {
        facts
            .iter()
            .take(8)
            .map(|fact| fact.statement.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let count_text = if claims.is_empty() {
        "Add each claim for relief with elements, supporting facts, evidence, and authority."
            .to_string()
    } else {
        claims
            .iter()
            .filter(|claim| claim.kind != "defense")
            .take(4)
            .map(|claim| format!("{}: {}", claim.title, claim.legal_theory))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let rows = [
        (
            "caption",
            "Caption",
            format!("{} · {}", matter.court, matter.name),
        ),
        (
            "jurisdiction_venue",
            "Jurisdiction and venue",
            format!(
                "Jurisdiction and venue are alleged in {}.",
                matter.jurisdiction
            ),
        ),
        ("factual_paragraph", "Factual allegations", fact_text),
        ("count", "Claims for relief", count_text),
        (
            "prayer_for_relief",
            "Prayer for relief",
            "Plaintiff requests relief according to proof after human review.".to_string(),
        ),
        (
            "signature_block",
            "Signature block",
            "Complete signature, contact, and certification information before filing.".to_string(),
        ),
    ];
    rows.iter()
        .enumerate()
        .map(|(index, (role, title, text))| WorkProductBlock {
            id: format!("{work_product_id}:block:{}", index + 1),
            block_id: format!("{work_product_id}:block:{}", index + 1),
            matter_id: matter.matter_id.clone(),
            work_product_id: work_product_id.to_string(),
            block_type: if *role == "caption" {
                "caption".to_string()
            } else {
                "section".to_string()
            },
            role: role.to_string(),
            title: title.to_string(),
            text: text.clone(),
            ordinal: index as u64 + 1,
            parent_block_id: None,
            fact_ids: if *role == "factual_paragraph" {
                facts
                    .iter()
                    .take(8)
                    .map(|fact| fact.fact_id.clone())
                    .collect()
            } else {
                Vec::new()
            },
            evidence_ids: Vec::new(),
            authorities: if *role == "count" {
                claims
                    .iter()
                    .filter(|claim| claim.kind != "defense")
                    .flat_map(|claim| claim.authorities.clone())
                    .collect()
            } else {
                Vec::new()
            },
            mark_ids: Vec::new(),
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: Some(prosemirror_doc_for_text(text)),
            ..WorkProductBlock::default()
        })
        .collect()
}

fn motion_blocks(
    matter: &MatterSummary,
    work_product_id: &str,
    facts: &[CaseFact],
    claims: &[CaseClaim],
) -> Vec<WorkProductBlock> {
    let fact_text = if facts.is_empty() {
        "Add record-supported facts before relying on the motion.".to_string()
    } else {
        facts
            .iter()
            .take(6)
            .map(|fact| fact.statement.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let argument_text = if claims.is_empty() {
        "Add the controlling legal standard, authority, and argument for the order requested."
            .to_string()
    } else {
        claims
            .iter()
            .take(4)
            .map(|claim| format!("{}: {}", claim.title, claim.legal_theory))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let rows = [
        (
            "notice_motion",
            "Notice and motion",
            format!(
                "Movant will ask {} for an order granting the relief requested below.",
                matter.court
            ),
        ),
        (
            "relief_requested",
            "Relief requested",
            "State the exact order or relief sought with particularity.".to_string(),
        ),
        ("factual_background", "Factual background", fact_text),
        (
            "legal_standard",
            "Legal standard",
            "Add controlling authority and explain the applicable standard.".to_string(),
        ),
        ("argument", "Argument", argument_text),
        (
            "conclusion",
            "Conclusion",
            "For these reasons, the motion should be granted after human review.".to_string(),
        ),
        (
            "proposed_order",
            "Proposed order placeholder",
            "Prepare or attach a proposed order if required by the court or local practice."
                .to_string(),
        ),
    ];
    rows.iter()
        .enumerate()
        .map(|(index, (role, title, text))| WorkProductBlock {
            id: format!("{work_product_id}:block:{}", index + 1),
            block_id: format!("{work_product_id}:block:{}", index + 1),
            matter_id: matter.matter_id.clone(),
            work_product_id: work_product_id.to_string(),
            block_type: if *role == "notice_motion" {
                "heading".to_string()
            } else {
                "section".to_string()
            },
            role: role.to_string(),
            title: title.to_string(),
            text: text.clone(),
            ordinal: index as u64 + 1,
            parent_block_id: None,
            fact_ids: if *role == "factual_background" {
                facts
                    .iter()
                    .take(6)
                    .map(|fact| fact.fact_id.clone())
                    .collect()
            } else {
                Vec::new()
            },
            evidence_ids: Vec::new(),
            authorities: if *role == "argument" {
                claims
                    .iter()
                    .flat_map(|claim| claim.authorities.clone())
                    .collect()
            } else {
                Vec::new()
            },
            mark_ids: Vec::new(),
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: Some(prosemirror_doc_for_text(text)),
            ..WorkProductBlock::default()
        })
        .collect()
}

fn profile_blocks(
    matter_id: &str,
    work_product_id: &str,
    rows: &[(&str, &str, &str)],
) -> Vec<WorkProductBlock> {
    rows.iter()
        .enumerate()
        .map(|(index, (role, title, text))| WorkProductBlock {
            id: format!("{work_product_id}:block:{}", index + 1),
            block_id: format!("{work_product_id}:block:{}", index + 1),
            matter_id: matter_id.to_string(),
            work_product_id: work_product_id.to_string(),
            block_type: "section".to_string(),
            role: role.to_string(),
            title: title.to_string(),
            text: text.to_string(),
            ordinal: index as u64 + 1,
            parent_block_id: None,
            fact_ids: Vec::new(),
            evidence_ids: Vec::new(),
            authorities: Vec::new(),
            mark_ids: Vec::new(),
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: Some(prosemirror_doc_for_text(text)),
            ..WorkProductBlock::default()
        })
        .collect()
}

fn prosemirror_doc_for_text(text: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "doc",
        "content": text
            .split("\n\n")
            .filter(|part| !part.trim().is_empty())
            .map(|part| serde_json::json!({
                "type": "paragraph",
                "content": [{ "type": "text", "text": part.trim() }]
            }))
            .collect::<Vec<_>>()
    })
}

fn work_product_profile(product_type: &str) -> WorkProductProfile {
    let (name, required, optional) = match product_type {
        "complaint" => (
            "Structured Complaint",
            vec![
                "caption",
                "jurisdiction",
                "facts",
                "count",
                "relief",
                "signature",
            ],
            vec!["certificate", "exhibits"],
        ),
        "motion" => (
            "Oregon Civil Motion",
            vec![
                "notice_motion",
                "relief_requested",
                "legal_standard",
                "argument",
                "conclusion",
            ],
            vec![
                "factual_background",
                "conferral_certificate",
                "proposed_order",
            ],
        ),
        "answer" => (
            "Structured Answer",
            vec!["responses", "affirmative_defenses", "prayer"],
            vec!["counterclaims"],
        ),
        "declaration" => (
            "Declaration",
            vec!["declarant", "facts", "signature"],
            vec!["exhibits"],
        ),
        _ => (
            "Structured Work Product",
            vec!["summary", "facts", "analysis", "conclusion"],
            vec!["exhibits", "certificate"],
        ),
    };
    WorkProductProfile {
        profile_id: format!("work-product-{product_type}-v1"),
        product_type: product_type.to_string(),
        name: name.to_string(),
        jurisdiction: "Oregon".to_string(),
        version: "provider-free-seed-2026-05-01".to_string(),
        route_slug: product_type.replace('_', "-"),
        required_block_roles: required.into_iter().map(str::to_string).collect(),
        optional_block_roles: optional.into_iter().map(str::to_string).collect(),
        supports_rich_text: true,
    }
}

fn default_work_product_formatting_profile(product_type: &str) -> FormattingProfile {
    let mut profile = default_formatting_profile();
    profile.profile_id = format!("oregon-circuit-civil-{product_type}");
    profile.name = format!(
        "Oregon Circuit Civil {}",
        humanize_product_type(product_type)
    );
    profile
}

fn work_product_rule_pack(product_type: &str) -> RulePack {
    if product_type == "motion" {
        return oregon_civil_motion_rule_pack();
    }
    if product_type == "complaint" {
        return oregon_civil_complaint_rule_pack();
    }
    RulePack {
        rule_pack_id: format!("oregon-circuit-civil-{product_type}-baseline"),
        name: format!(
            "Oregon Circuit Civil {} - baseline",
            humanize_product_type(product_type)
        ),
        jurisdiction: "Oregon".to_string(),
        version: "provider-free-seed-2026-05-01".to_string(),
        effective_date: "2025-08-01".to_string(),
        rule_profile: default_rule_profile_summary(),
        rules: vec![rule_definition(
            "work-product-review-needed",
            "Human review",
            "internal://casebuilder/rules/work-product-review-needed",
            "warning",
            "work_product",
            "review",
            "Work product requires human review.",
            "Generated or structured work product is not legal advice and is not filing-ready.",
            "Review the full document, authorities, evidence links, and formatting before use.",
            false,
        )],
    }
}

fn default_rule_profile_summary() -> RuleProfileSummary {
    RuleProfileSummary {
        jurisdiction_id: "or:state".to_string(),
        court_id: None,
        court: None,
        filing_date: None,
        utcr_edition_id: Some("or:utcr@2025".to_string()),
        slr_edition_id: None,
        active_statewide_order_ids: Vec::new(),
        active_local_order_ids: Vec::new(),
        active_out_of_cycle_amendment_ids: Vec::new(),
        currentness_warnings: vec![
            "Resolve filing-date rule context through /api/v1/rules/applicable before filing or export."
                .to_string(),
        ],
        resolver_endpoint:
            "/api/v1/rules/applicable?jurisdiction=Linn&date=YYYY-MM-DD&type=complaint"
                .to_string(),
    }
}

fn apply_matter_rule_profile(
    rule_pack: &mut RulePack,
    matter: &MatterSummary,
    filing_date: &str,
    product_type: &str,
) {
    let jurisdiction_id = casebuilder_jurisdiction_id(matter);
    let court_id = casebuilder_court_id(matter, &jurisdiction_id);
    rule_pack.rule_profile.jurisdiction_id = jurisdiction_id.clone();
    rule_pack.rule_profile.court_id = court_id;
    rule_pack.rule_profile.court = if matter.court.is_empty() {
        None
    } else {
        Some(matter.court.clone())
    };
    rule_pack.rule_profile.filing_date = Some(filing_date.to_string());
    rule_pack.rule_profile.resolver_endpoint = format!(
        "/api/v1/rules/applicable?jurisdiction={}&date={}&type={}",
        jurisdiction_id, filing_date, product_type
    );
}

fn casebuilder_jurisdiction_id(matter: &MatterSummary) -> String {
    let jurisdiction = matter.jurisdiction.trim();
    if jurisdiction.is_empty() || jurisdiction.eq_ignore_ascii_case("oregon") {
        return county_from_court(&matter.court)
            .map(|county| format!("or:{}", slug_casebuilder_id(&county)))
            .unwrap_or_else(|| "or:state".to_string());
    }
    let normalized = jurisdiction.to_ascii_lowercase();
    if normalized.starts_with("or:") {
        return normalized;
    }
    if normalized == "statewide" {
        return "or:state".to_string();
    }
    let county_source = if normalized.ends_with(" county") {
        normalized.trim_end_matches(" county").trim().to_string()
    } else if let Some(county) = county_from_court(&matter.court) {
        county
    } else {
        normalized
    };
    format!("or:{}", slug_casebuilder_id(&county_source))
}

fn casebuilder_court_id(matter: &MatterSummary, jurisdiction_id: &str) -> Option<String> {
    if jurisdiction_id == "or:state" {
        return None;
    }
    let court = matter.court.trim().to_ascii_lowercase();
    if court.starts_with("or:") {
        Some(court)
    } else if court.contains("circuit court") || court.is_empty() {
        Some(format!("{jurisdiction_id}:circuit_court"))
    } else {
        Some(format!("{jurisdiction_id}:{}", slug_casebuilder_id(&court)))
    }
}

fn county_from_court(court: &str) -> Option<String> {
    let lower = court.to_ascii_lowercase();
    lower
        .split(" county")
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != lower)
        .map(str::to_string)
}

fn slug_casebuilder_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn oregon_civil_motion_rule_pack() -> RulePack {
    RulePack {
        rule_pack_id: "oregon-circuit-civil-motion-orcp-utcr".to_string(),
        name: "Oregon Circuit Civil Motion - ORCP + UTCR".to_string(),
        jurisdiction: "Oregon".to_string(),
        version: "provider-free-seed-2026-05-01".to_string(),
        effective_date: "2025-08-01".to_string(),
        rule_profile: default_rule_profile_summary(),
        rules: vec![
            rule_definition("orcp-14-motion-writing-grounds-relief", "ORCP 14 A", "https://oregon.public.law/rules-of-civil-procedure/orcp-14-motions/", "blocking", "work_product", "rules", "Motion must state grounds and relief sought.", "ORCP 14 A requires written motions to state grounds with particularity and set forth requested relief.", "Complete the relief requested and argument blocks.", false),
            rule_definition("orcp-14-motion-form", "ORCP 14 B", "https://oregon.public.law/rules-of-civil-procedure/orcp-14-motions/", "serious", "formatting", "formatting", "Motion form requires caption, signing, and other form review.", "ORCP 14 B applies pleading form rules, including signing requirements, to motions and other papers.", "Review caption, signature, and document form before export.", false),
            rule_definition("orcp-17-motion-signature", "ORCP 17", "https://oregon.public.law/rules-of-civil-procedure/orcp-17-signing-of-pleadings-motions-and-other-papers-sanctions/", "serious", "signature", "rules", "Motion signature and certification require review.", "ORCP 17 governs signing and certification obligations for pleadings, motions, and other papers.", "Complete and review the signature/certification block.", false),
            rule_definition("utcr-5-010-conferral", "UTCR 5.010", "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf", "warning", "work_product", "rules", "Motion may need conferral certificate.", "UTCR 5.010 describes conferral and certificate requirements for specified civil motions.", "Add a conferral certificate or mark why it is not required.", false),
            rule_definition("utcr-5-020-authorities", "UTCR 5.020", "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf", "warning", "authority", "authority", "Motion authorities require review.", "UTCR 5.020 covers authorities in motions and related requirements.", "Link controlling authority in the legal-standard and argument blocks.", false),
            rule_definition("utcr-2-010-document-form", "UTCR 2.010", "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf", "serious", "formatting", "formatting", "Motion document form requires review.", "UTCR 2.010 applies document form requirements to pleadings and motions.", "Use the Oregon court-paper formatting profile before export.", true),
        ],
    }
}

fn default_work_product_ai_commands(product_type: &str) -> Vec<WorkProductAiCommandState> {
    let common = [
        ("tighten", "Make concise"),
        ("find_missing_evidence", "Find missing evidence"),
        ("find_missing_authority", "Find missing authority"),
        ("citation_check", "Check citations"),
        ("export_check", "Check export readiness"),
    ];
    let profile = if product_type == "motion" {
        vec![
            ("draft_motion_argument", "Draft motion argument"),
            ("draft_legal_standard", "Draft legal standard"),
            ("draft_proposed_order", "Draft proposed order"),
        ]
    } else {
        vec![("draft_section", "Draft section")]
    };
    profile
        .into_iter()
        .chain(common)
        .map(|(command_id, label)| WorkProductAiCommandState {
            command_id: command_id.to_string(),
            label: label.to_string(),
            status: "disabled".to_string(),
            mode: "template".to_string(),
            description: "Provider-free template state; no live AI provider is configured."
                .to_string(),
            last_message: None,
        })
        .collect()
}

fn work_product_event(
    matter_id: &str,
    work_product_id: &str,
    event_type: &str,
    target_type: &str,
    target_id: &str,
    summary: &str,
) -> WorkProductHistoryEvent {
    let event_id = generate_id(
        "work-product-event",
        &format!("{work_product_id}:{event_type}"),
    );
    WorkProductHistoryEvent {
        id: event_id.clone(),
        event_id,
        matter_id: matter_id.to_string(),
        work_product_id: work_product_id.to_string(),
        event_type: event_type.to_string(),
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        summary: summary.to_string(),
        timestamp: now_string(),
    }
}

fn refresh_work_product_state(product: &mut WorkProduct) {
    if product.document_ast.blocks.is_empty() {
        rebuild_work_product_ast_from_projection(product);
    } else {
        normalize_work_product_ast(product);
        product.blocks = flatten_work_product_blocks(&product.document_ast.blocks);
    }
    product.blocks.sort_by_key(|block| block.ordinal);
    for (index, block) in product.blocks.iter_mut().enumerate() {
        block.ordinal = index as u64 + 1;
        block.updated_at = product.updated_at.clone();
        block
            .prosemirror_json
            .get_or_insert_with(|| prosemirror_doc_for_text(&block.text));
    }
    normalize_work_product_ast(product);
    product.review_status = if product
        .findings
        .iter()
        .any(|finding| finding.status == "open" && finding.severity == "blocking")
    {
        "blocked".to_string()
    } else if product
        .findings
        .iter()
        .any(|finding| finding.status == "open")
    {
        "needs_human_review".to_string()
    } else {
        "ready_for_review".to_string()
    };
}

fn summarize_work_product_for_list(product: &mut WorkProduct) {
    product.blocks.clear();
    product.marks.clear();
    product.anchors.clear();
    product.document_ast.blocks.clear();
    product.document_ast.links.clear();
    product.document_ast.citations.clear();
    product.document_ast.exhibits.clear();
    product.document_ast.rule_findings.clear();
    for artifact in &mut product.artifacts {
        artifact.content_preview = export_content_preview(&artifact.content_preview);
    }
}

fn summarize_version_snapshot_for_list(snapshot: &mut VersionSnapshot) {
    snapshot.full_state_inline = None;
}

fn latest_snapshot_id(snapshots: &[VersionSnapshot]) -> Option<String> {
    snapshots
        .iter()
        .max_by_key(|snapshot| snapshot.sequence_number)
        .map(|snapshot| snapshot.snapshot_id.clone())
}

fn validate_ast_patch_concurrency(
    patch: &AstPatch,
    route_work_product_id: &str,
    current_document_hash: &str,
    current_snapshot_id: Option<&str>,
) -> ApiResult<()> {
    if let Some(patch_work_product_id) = patch.work_product_id.as_deref() {
        if patch_work_product_id != route_work_product_id {
            return Err(ApiError::BadRequest(
                "AST patch work_product_id does not match route target.".to_string(),
            ));
        }
    }
    if patch.base_document_hash.is_none() && patch.base_snapshot_id.is_none() {
        return Err(ApiError::BadRequest(
            "AST patch requires base_document_hash or base_snapshot_id.".to_string(),
        ));
    }
    if let Some(base_hash) = patch.base_document_hash.as_deref() {
        if base_hash != current_document_hash {
            return Err(ApiError::Conflict(format!(
                "AST patch conflict: conflict_field=base_document_hash base_document_hash={base_hash} current_document_hash={current_document_hash}."
            )));
        }
    }
    if let Some(base_snapshot_id) = patch.base_snapshot_id.as_deref() {
        if current_snapshot_id != Some(base_snapshot_id) {
            return Err(ApiError::Conflict(
                "AST patch conflict: conflict_field=base_snapshot_id.".to_string(),
            ));
        }
    }
    Ok(())
}

fn rebuild_work_product_ast_from_projection(product: &mut WorkProduct) {
    let blocks = product.blocks.clone();
    product.document_ast = work_product_document_from_projection(product, blocks);
}

fn normalize_work_product_ast(product: &mut WorkProduct) {
    let now = if product.updated_at.is_empty() {
        now_string()
    } else {
        product.updated_at.clone()
    };
    product.document_ast.schema_version = if product.document_ast.schema_version.is_empty() {
        default_work_product_schema_version()
    } else {
        product.document_ast.schema_version.clone()
    };
    product.document_ast.document_id = if product.document_ast.document_id.is_empty() {
        format!("{}:document", product.work_product_id)
    } else {
        product.document_ast.document_id.clone()
    };
    product.document_ast.work_product_id = product.work_product_id.clone();
    product.document_ast.matter_id = product.matter_id.clone();
    product.document_ast.product_type = product.product_type.clone();
    product.document_ast.title = product.title.clone();
    product.document_ast.metadata.status = product.status.clone();
    product.document_ast.metadata.rule_pack_id = Some(product.rule_pack.rule_pack_id.clone());
    product.document_ast.metadata.formatting_profile_id =
        Some(product.formatting_profile.profile_id.clone());
    if product.document_ast.created_at.is_empty() {
        product.document_ast.created_at = product.created_at.clone();
    }
    product.document_ast.updated_at = now.clone();
    if product.document_ast.rule_findings.is_empty() && !product.findings.is_empty() {
        product.document_ast.rule_findings = product.findings.clone();
    }
    product.findings = product.document_ast.rule_findings.clone();
    for block in &mut product.document_ast.blocks {
        normalize_ast_block(block, &product.matter_id, &product.work_product_id, &now);
    }
}

fn normalize_ast_block(
    block: &mut WorkProductBlock,
    matter_id: &str,
    work_product_id: &str,
    now: &str,
) {
    if block.id.is_empty() {
        block.id = block.block_id.clone();
    }
    block.matter_id = matter_id.to_string();
    block.work_product_id = work_product_id.to_string();
    if block.created_at.is_empty() {
        block.created_at = now.to_string();
    }
    block.updated_at = now.to_string();
    if block.review_status.is_empty() {
        block.review_status = "needs_review".to_string();
    }
    if block.block_type.is_empty() {
        block.block_type = "paragraph".to_string();
    }
    for child in &mut block.children {
        child.parent_block_id = Some(block.block_id.clone());
        normalize_ast_block(child, matter_id, work_product_id, now);
    }
}

fn work_product_document_from_projection(
    product: &WorkProduct,
    blocks: Vec<WorkProductBlock>,
) -> WorkProductDocument {
    let now = if product.updated_at.is_empty() {
        product.created_at.clone()
    } else {
        product.updated_at.clone()
    };
    let mut flat_blocks = blocks
        .into_iter()
        .enumerate()
        .map(|(index, mut block)| {
            if block.id.is_empty() {
                block.id = block.block_id.clone();
            }
            if block.created_at.is_empty() {
                block.created_at = product.created_at.clone();
            }
            block.updated_at = now.clone();
            block.ordinal = if block.ordinal == 0 {
                index as u64 + 1
            } else {
                block.ordinal
            };
            block.children.clear();
            block.links.clear();
            block.citations.clear();
            block.exhibits.clear();
            block.rule_finding_ids = product
                .findings
                .iter()
                .filter(|finding| finding.target_id == block.block_id)
                .map(|finding| finding.finding_id.clone())
                .collect();
            block
        })
        .collect::<Vec<_>>();
    flat_blocks.sort_by_key(|block| block.ordinal);

    let mut links = Vec::new();
    let mut citations = Vec::new();
    let exhibits = Vec::new();
    for block in &mut flat_blocks {
        for fact_id in &block.fact_ids {
            let link_id = format!(
                "{}:link:fact:{}",
                block.block_id,
                sanitize_path_segment(fact_id)
            );
            block.links.push(link_id.clone());
            links.push(WorkProductLink {
                link_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                target_type: "fact".to_string(),
                target_id: fact_id.clone(),
                relation: "supports".to_string(),
                confidence: None,
                created_by: "system".to_string(),
                created_at: now.clone(),
            });
        }
        for evidence_id in &block.evidence_ids {
            let link_id = format!(
                "{}:link:evidence:{}",
                block.block_id,
                sanitize_path_segment(evidence_id)
            );
            block.links.push(link_id.clone());
            links.push(WorkProductLink {
                link_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                target_type: "evidence".to_string(),
                target_id: evidence_id.clone(),
                relation: "supports".to_string(),
                confidence: None,
                created_by: "system".to_string(),
                created_at: now.clone(),
            });
        }
        for authority in &block.authorities {
            let link_id = format!(
                "{}:link:authority:{}",
                block.block_id,
                sanitize_path_segment(&authority.canonical_id)
            );
            let citation_use_id = format!(
                "{}:citation:{}",
                block.block_id,
                sanitize_path_segment(&authority.citation)
            );
            block.links.push(link_id.clone());
            block.citations.push(citation_use_id.clone());
            links.push(WorkProductLink {
                link_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                target_type: "legal_authority".to_string(),
                target_id: authority.canonical_id.clone(),
                relation: "cites".to_string(),
                confidence: None,
                created_by: "system".to_string(),
                created_at: now.clone(),
            });
            citations.push(WorkProductCitationUse {
                citation_use_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                raw_text: authority.citation.clone(),
                normalized_citation: Some(authority.citation.clone()),
                target_type: "provision".to_string(),
                target_id: Some(authority.canonical_id.clone()),
                pinpoint: authority.pinpoint.clone(),
                status: "resolved".to_string(),
                resolver_message: authority.reason.clone(),
                created_at: now.clone(),
            });
        }
    }

    for anchor in &product.anchors {
        let link_id = anchor.anchor_id.clone();
        links.push(WorkProductLink {
            link_id: link_id.clone(),
            source_block_id: anchor.block_id.clone(),
            source_text_range: anchor.quote.as_ref().map(|quote| TextRange {
                start_offset: 0,
                end_offset: quote.chars().count() as u64,
                quote: Some(quote.clone()),
            }),
            target_type: anchor.target_type.clone(),
            target_id: anchor.target_id.clone(),
            relation: anchor.relation.clone(),
            confidence: None,
            created_by: "user".to_string(),
            created_at: now.clone(),
        });
        if let Some(block) = flat_blocks
            .iter_mut()
            .find(|block| block.block_id == anchor.block_id)
        {
            push_unique(&mut block.links, link_id.clone());
            if anchor.anchor_type == "authority" || anchor.citation.is_some() {
                let citation_use_id = format!("{link_id}:citation");
                push_unique(&mut block.citations, citation_use_id.clone());
                citations.push(WorkProductCitationUse {
                    citation_use_id,
                    source_block_id: anchor.block_id.clone(),
                    source_text_range: None,
                    raw_text: anchor
                        .citation
                        .clone()
                        .unwrap_or_else(|| anchor.target_id.clone()),
                    normalized_citation: anchor.citation.clone(),
                    target_type: "provision".to_string(),
                    target_id: anchor
                        .canonical_id
                        .clone()
                        .or_else(|| Some(anchor.target_id.clone())),
                    pinpoint: anchor.pinpoint.clone(),
                    status: if anchor.status == "resolved" {
                        "resolved".to_string()
                    } else {
                        "needs_review".to_string()
                    },
                    resolver_message: None,
                    created_at: now.clone(),
                });
            }
        }
    }

    WorkProductDocument {
        schema_version: default_work_product_schema_version(),
        document_id: format!("{}:document", product.work_product_id),
        work_product_id: product.work_product_id.clone(),
        matter_id: product.matter_id.clone(),
        product_type: product.product_type.clone(),
        title: product.title.clone(),
        metadata: WorkProductMetadata {
            jurisdiction: Some(product.profile.jurisdiction.clone()),
            court: None,
            county: None,
            case_number: None,
            rule_pack_id: Some(product.rule_pack.rule_pack_id.clone()),
            template_id: None,
            formatting_profile_id: Some(product.formatting_profile.profile_id.clone()),
            parties: None,
            status: product.status.clone(),
        },
        blocks: build_work_product_block_tree(&flat_blocks),
        links,
        citations,
        exhibits,
        rule_findings: product.findings.clone(),
        created_at: product.created_at.clone(),
        updated_at: now,
    }
}

fn build_work_product_block_tree(flat_blocks: &[WorkProductBlock]) -> Vec<WorkProductBlock> {
    let ids = flat_blocks
        .iter()
        .map(|block| block.block_id.clone())
        .collect::<HashSet<_>>();
    flat_blocks
        .iter()
        .filter(|block| {
            block
                .parent_block_id
                .as_ref()
                .map(|parent_id| !ids.contains(parent_id))
                .unwrap_or(true)
        })
        .cloned()
        .map(|mut block| {
            attach_work_product_children(&mut block, flat_blocks);
            block
        })
        .collect()
}

fn attach_work_product_children(block: &mut WorkProductBlock, flat_blocks: &[WorkProductBlock]) {
    block.children = flat_blocks
        .iter()
        .filter(|candidate| candidate.parent_block_id.as_deref() == Some(&block.block_id))
        .cloned()
        .map(|mut child| {
            attach_work_product_children(&mut child, flat_blocks);
            child
        })
        .collect();
}

fn flatten_work_product_blocks(blocks: &[WorkProductBlock]) -> Vec<WorkProductBlock> {
    let mut flattened = Vec::new();
    for block in blocks {
        flatten_work_product_block(block, &mut flattened);
    }
    flattened.sort_by_key(|block| block.ordinal);
    flattened
}

fn flatten_work_product_block(block: &WorkProductBlock, flattened: &mut Vec<WorkProductBlock>) {
    let mut current = block.clone();
    current.children.clear();
    flattened.push(current);
    for child in &block.children {
        flatten_work_product_block(child, flattened);
    }
}

fn block_text_excerpt(text: &str, inline_limit: u64) -> String {
    if should_inline_payload(text.len(), inline_limit) {
        return text.to_string();
    }
    const GRAPH_EXCERPT_CHARS: usize = 4096;
    text.chars().take(GRAPH_EXCERPT_CHARS).collect()
}

fn work_product_block_graph_payload(
    block: &WorkProductBlock,
    inline_limit: u64,
) -> ApiResult<String> {
    let mut payload = json_value(block)?;
    if !should_inline_payload(block.text.len(), inline_limit) {
        if let Some(object) = payload.as_object_mut() {
            let excerpt = block_text_excerpt(&block.text, inline_limit);
            object.insert(
                "text".to_string(),
                serde_json::Value::String(excerpt.clone()),
            );
            object.insert(
                "text_excerpt".to_string(),
                serde_json::Value::String(excerpt),
            );
            object.insert(
                "text_hash".to_string(),
                serde_json::Value::String(sha256_hex(block.text.as_bytes())),
            );
            object.insert(
                "text_size_bytes".to_string(),
                serde_json::Value::Number(serde_json::Number::from(block.text.len() as u64)),
            );
            object.insert(
                "text_storage_status".to_string(),
                serde_json::Value::String("graph_excerpt".to_string()),
            );
        }
    }
    to_payload(&payload)
}

fn apply_ast_operation(
    document: &mut WorkProductDocument,
    operation: &AstOperation,
) -> ApiResult<()> {
    match operation {
        AstOperation::InsertBlock {
            parent_id,
            after_block_id,
            block,
        } => insert_ast_block(
            &mut document.blocks,
            parent_id.as_deref(),
            after_block_id.as_deref(),
            block.clone(),
        ),
        AstOperation::UpdateBlock {
            block_id, after, ..
        } => {
            let block = find_ast_block_mut(&mut document.blocks, block_id)
                .ok_or_else(|| ApiError::NotFound(format!("AST block {block_id} not found")))?;
            merge_json_patch_into_block(block, after)
        }
        AstOperation::DeleteBlock { block_id, .. } => {
            delete_ast_block(&mut document.blocks, block_id)
                .map(|_| ())
                .ok_or_else(|| ApiError::NotFound(format!("AST block {block_id} not found")))
        }
        AstOperation::MoveBlock {
            block_id,
            parent_id,
            after_block_id,
        } => {
            let block = delete_ast_block(&mut document.blocks, block_id)
                .ok_or_else(|| ApiError::NotFound(format!("AST block {block_id} not found")))?;
            insert_ast_block(
                &mut document.blocks,
                parent_id.as_deref(),
                after_block_id.as_deref(),
                block,
            )
        }
        AstOperation::SplitBlock {
            block_id,
            offset,
            new_block_id,
        } => split_ast_block(&mut document.blocks, block_id, *offset, new_block_id),
        AstOperation::MergeBlocks {
            first_block_id,
            second_block_id,
        } => merge_ast_blocks(&mut document.blocks, first_block_id, second_block_id),
        AstOperation::RenumberParagraphs => {
            let mut next = 1;
            renumber_ast_paragraphs(&mut document.blocks, &mut next);
            Ok(())
        }
        AstOperation::AddCitation { citation } => {
            let citation_id = citation.citation_use_id.clone();
            let block = find_ast_block_mut(&mut document.blocks, &citation.source_block_id)
                .ok_or_else(|| {
                    ApiError::NotFound("AST citation source block not found".to_string())
                })?;
            push_unique(&mut block.citations, citation_id.clone());
            document
                .citations
                .retain(|item| item.citation_use_id != citation_id);
            document.citations.push(citation.clone());
            Ok(())
        }
        AstOperation::ResolveCitation {
            citation_use_id,
            normalized_citation,
            target_type,
            target_id,
            status,
        } => {
            let citation = document
                .citations
                .iter_mut()
                .find(|item| item.citation_use_id == *citation_use_id)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("CitationUse {citation_use_id} not found"))
                })?;
            if let Some(value) = normalized_citation {
                citation.normalized_citation = Some(value.clone());
            }
            if let Some(value) = target_type {
                citation.target_type = value.clone();
            }
            if let Some(value) = target_id {
                citation.target_id = Some(value.clone());
            }
            if let Some(value) = status {
                citation.status = value.clone();
            }
            Ok(())
        }
        AstOperation::RemoveCitation { citation_use_id } => {
            document
                .citations
                .retain(|item| item.citation_use_id != *citation_use_id);
            remove_block_ref(&mut document.blocks, citation_use_id, "citation");
            Ok(())
        }
        AstOperation::AddLink { link } => {
            let link_id = link.link_id.clone();
            let block = find_ast_block_mut(&mut document.blocks, &link.source_block_id)
                .ok_or_else(|| ApiError::NotFound("AST link source block not found".to_string()))?;
            push_unique(&mut block.links, link_id.clone());
            document.links.retain(|item| item.link_id != link_id);
            document.links.push(link.clone());
            Ok(())
        }
        AstOperation::RemoveLink { link_id } => {
            document.links.retain(|item| item.link_id != *link_id);
            remove_block_ref(&mut document.blocks, link_id, "link");
            Ok(())
        }
        AstOperation::AddExhibitReference { exhibit } => {
            let exhibit_id = exhibit.exhibit_reference_id.clone();
            let block = find_ast_block_mut(&mut document.blocks, &exhibit.source_block_id)
                .ok_or_else(|| {
                    ApiError::NotFound("AST exhibit source block not found".to_string())
                })?;
            push_unique(&mut block.exhibits, exhibit_id.clone());
            document
                .exhibits
                .retain(|item| item.exhibit_reference_id != exhibit_id);
            document.exhibits.push(exhibit.clone());
            Ok(())
        }
        AstOperation::ResolveExhibitReference {
            exhibit_reference_id,
            exhibit_id,
            status,
        } => {
            let exhibit = document
                .exhibits
                .iter_mut()
                .find(|item| item.exhibit_reference_id == *exhibit_reference_id)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("ExhibitReference {exhibit_reference_id} not found"))
                })?;
            if let Some(value) = exhibit_id {
                exhibit.exhibit_id = Some(value.clone());
            }
            if let Some(value) = status {
                exhibit.status = value.clone();
            }
            Ok(())
        }
        AstOperation::AddRuleFinding { finding } => {
            let finding_id = finding.finding_id.clone();
            let block =
                find_ast_block_mut(&mut document.blocks, &finding.target_id).ok_or_else(|| {
                    ApiError::NotFound("AST rule finding target block not found".to_string())
                })?;
            push_unique(&mut block.rule_finding_ids, finding_id.clone());
            document
                .rule_findings
                .retain(|item| item.finding_id != finding_id);
            document.rule_findings.push(finding.clone());
            Ok(())
        }
        AstOperation::ResolveRuleFinding { finding_id, status } => {
            let finding = document
                .rule_findings
                .iter_mut()
                .find(|item| item.finding_id == *finding_id)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("Rule finding {finding_id} not found"))
                })?;
            finding.status = status.clone();
            finding.updated_at = now_string();
            Ok(())
        }
        AstOperation::ApplyTemplate { template_id } => {
            document.metadata.template_id = Some(template_id.clone());
            Ok(())
        }
    }
}

fn insert_ast_block(
    blocks: &mut Vec<WorkProductBlock>,
    parent_id: Option<&str>,
    after_block_id: Option<&str>,
    mut block: WorkProductBlock,
) -> ApiResult<()> {
    block.parent_block_id = parent_id.map(str::to_string);
    let target_blocks = if let Some(parent_id) = parent_id {
        &mut find_ast_block_mut(blocks, parent_id)
            .ok_or_else(|| ApiError::NotFound(format!("Parent block {parent_id} not found")))?
            .children
    } else {
        blocks
    };
    let insert_index = after_block_id
        .and_then(|after_id| {
            target_blocks
                .iter()
                .position(|candidate| candidate.block_id == after_id)
                .map(|index| index + 1)
        })
        .unwrap_or_else(|| target_blocks.len());
    target_blocks.insert(insert_index, block);
    for (index, block) in target_blocks.iter_mut().enumerate() {
        block.ordinal = index as u64 + 1;
    }
    Ok(())
}

fn find_ast_block_mut<'a>(
    blocks: &'a mut [WorkProductBlock],
    block_id: &str,
) -> Option<&'a mut WorkProductBlock> {
    for block in blocks {
        if block.block_id == block_id {
            return Some(block);
        }
        if let Some(found) = find_ast_block_mut(&mut block.children, block_id) {
            return Some(found);
        }
    }
    None
}

fn delete_ast_block(
    blocks: &mut Vec<WorkProductBlock>,
    block_id: &str,
) -> Option<WorkProductBlock> {
    if let Some(index) = blocks.iter().position(|block| block.block_id == block_id) {
        return Some(blocks.remove(index));
    }
    for block in blocks {
        if let Some(deleted) = delete_ast_block(&mut block.children, block_id) {
            return Some(deleted);
        }
    }
    None
}

fn merge_json_patch_into_block(
    block: &mut WorkProductBlock,
    patch: &serde_json::Value,
) -> ApiResult<()> {
    let mut value = json_value(block)?;
    merge_json_objects(&mut value, patch);
    let mut updated: WorkProductBlock =
        serde_json::from_value(value).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    if updated.block_id.is_empty() {
        updated.block_id = block.block_id.clone();
    }
    *block = updated;
    Ok(())
}

fn merge_json_objects(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(base), serde_json::Value::Object(patch)) => {
            for (key, value) in patch {
                if value.is_null() {
                    base.remove(key);
                } else {
                    merge_json_objects(base.entry(key).or_insert(serde_json::Value::Null), value);
                }
            }
        }
        (base, patch) => *base = patch.clone(),
    }
}

fn split_ast_block(
    blocks: &mut Vec<WorkProductBlock>,
    block_id: &str,
    offset: u64,
    new_block_id: &str,
) -> ApiResult<()> {
    let (parent_id, new_block) = {
        let block = find_ast_block_mut(blocks, block_id)
            .ok_or_else(|| ApiError::NotFound(format!("AST block {block_id} not found")))?;
        let split_at = offset.min(block.text.chars().count() as u64) as usize;
        let left = block.text.chars().take(split_at).collect::<String>();
        let right = block.text.chars().skip(split_at).collect::<String>();
        block.text = left.trim_end().to_string();
        let mut new_block = block.clone();
        new_block.block_id = new_block_id.to_string();
        new_block.id = new_block_id.to_string();
        new_block.text = right.trim_start().to_string();
        new_block.ordinal = block.ordinal + 1;
        (block.parent_block_id.clone(), new_block)
    };
    insert_ast_block(blocks, parent_id.as_deref(), Some(block_id), new_block)
}

fn merge_ast_blocks(
    blocks: &mut Vec<WorkProductBlock>,
    first_block_id: &str,
    second_block_id: &str,
) -> ApiResult<()> {
    let second = delete_ast_block(blocks, second_block_id)
        .ok_or_else(|| ApiError::NotFound(format!("AST block {second_block_id} not found")))?;
    let first = find_ast_block_mut(blocks, first_block_id)
        .ok_or_else(|| ApiError::NotFound(format!("AST block {first_block_id} not found")))?;
    if !first.text.is_empty() && !second.text.is_empty() {
        first.text.push_str("\n\n");
    }
    first.text.push_str(&second.text);
    for id in second.links {
        push_unique(&mut first.links, id);
    }
    for id in second.citations {
        push_unique(&mut first.citations, id);
    }
    for id in second.exhibits {
        push_unique(&mut first.exhibits, id);
    }
    for id in second.rule_finding_ids {
        push_unique(&mut first.rule_finding_ids, id);
    }
    Ok(())
}

fn renumber_ast_paragraphs(blocks: &mut [WorkProductBlock], next: &mut u64) {
    for block in blocks {
        if matches!(
            block.block_type.as_str(),
            "numbered_paragraph" | "paragraph"
        ) && matches!(
            block.role.as_str(),
            "factual_allegation"
                | "legal_allegation"
                | "jurisdiction"
                | "venue"
                | "claim_element"
                | "relief"
                | "fact"
        ) {
            block.paragraph_number = Some(*next);
            *next += 1;
        }
        renumber_ast_paragraphs(&mut block.children, next);
    }
}

fn remove_block_ref(blocks: &mut [WorkProductBlock], id: &str, ref_kind: &str) {
    for block in blocks {
        match ref_kind {
            "citation" => block.citations.retain(|value| value != id),
            "link" => block.links.retain(|value| value != id),
            "exhibit" => block.exhibits.retain(|value| value != id),
            _ => {}
        }
        remove_block_ref(&mut block.children, id, ref_kind);
    }
}

fn validate_work_product_document(product: &WorkProduct) -> AstValidationResponse {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let document = &product.document_ast;
    if document.schema_version.trim().is_empty() {
        errors.push(ast_issue(
            "missing_schema_version",
            "WorkProduct AST is missing schema_version.",
            Some("document"),
            Some(&document.document_id),
        ));
    }
    let mut seen = HashSet::new();
    let mut parent_ids = HashSet::new();
    validate_ast_blocks(
        &document.blocks,
        None,
        &mut seen,
        &mut parent_ids,
        &mut errors,
        &mut warnings,
    );
    for parent_id in parent_ids {
        if !seen.contains(&parent_id) {
            errors.push(ast_issue(
                "missing_parent",
                &format!("Parent block {parent_id} does not exist."),
                Some("block"),
                Some(&parent_id),
            ));
        }
    }
    if product.product_type == "complaint" {
        let flat = flatten_work_product_blocks(&document.blocks);
        if !flat
            .iter()
            .any(|block| block.block_type == "caption" || block.role == "caption")
        {
            warnings.push(ast_issue(
                "complaint_caption_missing",
                "Complaint AST should include a caption block.",
                Some("document"),
                Some(&document.document_id),
            ));
        }
        if !flat
            .iter()
            .any(|block| matches!(block.role.as_str(), "relief" | "prayer_for_relief"))
        {
            warnings.push(ast_issue(
                "complaint_relief_missing",
                "Complaint AST should include demand/prayer for relief.",
                Some("document"),
                Some(&document.document_id),
            ));
        }
    }
    let link_ids = document
        .links
        .iter()
        .map(|link| link.link_id.clone())
        .collect::<HashSet<_>>();
    let citation_ids = document
        .citations
        .iter()
        .map(|citation| citation.citation_use_id.clone())
        .collect::<HashSet<_>>();
    let exhibit_ids = document
        .exhibits
        .iter()
        .map(|exhibit| exhibit.exhibit_reference_id.clone())
        .collect::<HashSet<_>>();
    for block in flatten_work_product_blocks(&document.blocks) {
        for link_id in &block.links {
            if !link_ids.contains(link_id) {
                errors.push(ast_issue(
                    "broken_block_link",
                    &format!(
                        "Block {} references missing link {link_id}.",
                        block.block_id
                    ),
                    Some("block"),
                    Some(&block.block_id),
                ));
            }
        }
        for citation_id in &block.citations {
            if !citation_ids.contains(citation_id) {
                errors.push(ast_issue(
                    "broken_block_citation",
                    &format!(
                        "Block {} references missing citation {citation_id}.",
                        block.block_id
                    ),
                    Some("block"),
                    Some(&block.block_id),
                ));
            }
        }
        for exhibit_id in &block.exhibits {
            if !exhibit_ids.contains(exhibit_id) {
                errors.push(ast_issue(
                    "broken_block_exhibit",
                    &format!(
                        "Block {} references missing exhibit {exhibit_id}.",
                        block.block_id
                    ),
                    Some("block"),
                    Some(&block.block_id),
                ));
            }
        }
    }
    for citation in &document.citations {
        if matches!(
            citation.status.as_str(),
            "unresolved" | "ambiguous" | "stale" | "currentness_warning" | "needs_review"
        ) {
            warnings.push(ast_issue(
                "citation_needs_review",
                &format!("Citation '{}' needs review.", citation.raw_text),
                Some("citation"),
                Some(&citation.citation_use_id),
            ));
        }
    }
    for exhibit in &document.exhibits {
        if exhibit.status != "attached" {
            warnings.push(ast_issue(
                "exhibit_needs_review",
                &format!("Exhibit reference '{}' is not attached.", exhibit.label),
                Some("exhibit"),
                Some(&exhibit.exhibit_reference_id),
            ));
        }
    }
    AstValidationResponse {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

fn validate_ast_blocks(
    blocks: &[WorkProductBlock],
    parent_id: Option<&str>,
    seen: &mut HashSet<String>,
    parent_ids: &mut HashSet<String>,
    errors: &mut Vec<AstValidationIssue>,
    warnings: &mut Vec<AstValidationIssue>,
) {
    let mut order_indexes = HashSet::new();
    for block in blocks {
        if block.block_id.trim().is_empty() {
            errors.push(ast_issue(
                "missing_block_id",
                "AST block is missing block_id.",
                Some("block"),
                None,
            ));
        } else if !seen.insert(block.block_id.clone()) {
            errors.push(ast_issue(
                "duplicate_block_id",
                &format!("Duplicate block id {}.", block.block_id),
                Some("block"),
                Some(&block.block_id),
            ));
        }
        if block.block_type.trim().is_empty() {
            errors.push(ast_issue(
                "missing_block_type",
                &format!("Block {} is missing type.", block.block_id),
                Some("block"),
                Some(&block.block_id),
            ));
        }
        if block.ordinal == 0 || !order_indexes.insert(block.ordinal) {
            warnings.push(ast_issue(
                "order_index_review",
                &format!(
                    "Block {} has a duplicate or zero order_index.",
                    block.block_id
                ),
                Some("block"),
                Some(&block.block_id),
            ));
        }
        if let Some(parent_id) = parent_id.or(block.parent_block_id.as_deref()) {
            parent_ids.insert(parent_id.to_string());
            if parent_id == block.block_id {
                errors.push(ast_issue(
                    "block_cycle",
                    &format!("Block {} cannot be its own parent.", block.block_id),
                    Some("block"),
                    Some(&block.block_id),
                ));
            }
        }
        validate_ast_blocks(
            &block.children,
            Some(&block.block_id),
            seen,
            parent_ids,
            errors,
            warnings,
        );
    }
}

fn ast_issue(
    code: &str,
    message: &str,
    target_type: Option<&str>,
    target_id: Option<&str>,
) -> AstValidationIssue {
    AstValidationIssue {
        code: code.to_string(),
        message: message.to_string(),
        target_type: target_type.map(str::to_string),
        target_id: target_id.map(str::to_string),
    }
}

fn markdown_to_work_product_ast(
    product: &WorkProduct,
    markdown: &str,
) -> (WorkProductDocument, Vec<String>) {
    let mut blocks = Vec::new();
    let mut warnings = Vec::new();
    let mut pending = Vec::new();
    let mut ordinal = 1_u64;
    let mut in_frontmatter = false;
    let mut frontmatter_seen = false;

    for raw_line in markdown.lines() {
        let line = raw_line.trim_end();
        if line.trim() == "---" && !frontmatter_seen {
            in_frontmatter = !in_frontmatter;
            if !in_frontmatter {
                frontmatter_seen = true;
            }
            continue;
        }
        if in_frontmatter || line.trim_start().starts_with("<!--") {
            continue;
        }
        if line.trim().is_empty() {
            flush_markdown_paragraph(product, &mut blocks, &mut pending, &mut ordinal);
            continue;
        }
        if let Some((level, heading)) = markdown_heading(line) {
            flush_markdown_paragraph(product, &mut blocks, &mut pending, &mut ordinal);
            let is_count = heading.to_uppercase().starts_with("COUNT ");
            blocks.push(WorkProductBlock {
                id: format!("{}:block:{}", product.work_product_id, ordinal),
                block_id: format!("{}:block:{}", product.work_product_id, ordinal),
                matter_id: product.matter_id.clone(),
                work_product_id: product.work_product_id.clone(),
                block_type: if is_count { "count" } else { "heading" }.to_string(),
                role: if is_count { "count" } else { "heading" }.to_string(),
                title: heading.to_string(),
                text: heading.to_string(),
                ordinal,
                section_kind: if is_count {
                    None
                } else {
                    Some(format!("level_{level}"))
                },
                count_number: if is_count {
                    roman_or_number_after_count(heading)
                } else {
                    None
                },
                review_status: "needs_review".to_string(),
                prosemirror_json: Some(prosemirror_doc_for_text(heading)),
                ..WorkProductBlock::default()
            });
            ordinal += 1;
            continue;
        }
        if let Some((number, text)) = markdown_numbered_paragraph(line) {
            flush_markdown_paragraph(product, &mut blocks, &mut pending, &mut ordinal);
            blocks.push(WorkProductBlock {
                id: format!("{}:block:{}", product.work_product_id, ordinal),
                block_id: format!("{}:block:{}", product.work_product_id, ordinal),
                matter_id: product.matter_id.clone(),
                work_product_id: product.work_product_id.clone(),
                block_type: "numbered_paragraph".to_string(),
                role: "factual_allegation".to_string(),
                title: format!("Paragraph {number}"),
                text: text.to_string(),
                ordinal,
                paragraph_number: Some(number),
                review_status: "needs_review".to_string(),
                prosemirror_json: Some(prosemirror_doc_for_text(text)),
                ..WorkProductBlock::default()
            });
            ordinal += 1;
            continue;
        }
        pending.push(line.trim().to_string());
    }
    flush_markdown_paragraph(product, &mut blocks, &mut pending, &mut ordinal);

    if blocks.is_empty() {
        warnings.push(
            "Markdown did not contain recognizable blocks; created an empty AST.".to_string(),
        );
    }
    let document = work_product_document_from_projection(product, blocks);
    (document, warnings)
}

fn flush_markdown_paragraph(
    product: &WorkProduct,
    blocks: &mut Vec<WorkProductBlock>,
    pending: &mut Vec<String>,
    ordinal: &mut u64,
) {
    if pending.is_empty() {
        return;
    }
    let text = pending.join(" ");
    blocks.push(WorkProductBlock {
        id: format!("{}:block:{}", product.work_product_id, *ordinal),
        block_id: format!("{}:block:{}", product.work_product_id, *ordinal),
        matter_id: product.matter_id.clone(),
        work_product_id: product.work_product_id.clone(),
        block_type: "paragraph".to_string(),
        role: "custom".to_string(),
        title: format!("Paragraph {}", *ordinal),
        text: text.clone(),
        ordinal: *ordinal,
        review_status: "needs_review".to_string(),
        prosemirror_json: Some(prosemirror_doc_for_text(&text)),
        ..WorkProductBlock::default()
    });
    *ordinal += 1;
    pending.clear();
}

fn markdown_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|value| *value == '#').count();
    if (1..=4).contains(&level) && trimmed.chars().nth(level) == Some(' ') {
        Some((level, trimmed[level + 1..].trim()))
    } else {
        None
    }
}

fn markdown_numbered_paragraph(line: &str) -> Option<(u64, &str)> {
    let trimmed = line.trim_start();
    let dot = trimmed.find('.')?;
    let number = trimmed[..dot].parse::<u64>().ok()?;
    let text = trimmed[dot + 1..].trim();
    if text.is_empty() {
        None
    } else {
        Some((number, text))
    }
}

fn roman_or_number_after_count(text: &str) -> Option<u64> {
    let rest = text.trim_start_matches(|c: char| c != ' ').trim();
    let token = rest.split_whitespace().next().unwrap_or_default();
    token.parse::<u64>().ok().or_else(|| {
        match token
            .trim_matches(|c: char| !c.is_ascii_alphabetic())
            .to_uppercase()
            .as_str()
        {
            "I" => Some(1),
            "II" => Some(2),
            "III" => Some(3),
            "IV" => Some(4),
            "V" => Some(5),
            "VI" => Some(6),
            _ => None,
        }
    })
}

fn work_product_findings(product: &WorkProduct) -> Vec<WorkProductFinding> {
    let now = now_string();
    let mut findings = Vec::new();
    for role in &product.profile.required_block_roles {
        let missing = product
            .blocks
            .iter()
            .find(|block| &block.role == role)
            .map(|block| block.text.trim().is_empty())
            .unwrap_or(true);
        if missing {
            findings.push(work_product_finding(
                product,
                &format!("required-block-{role}"),
                "structure",
                "blocking",
                "block",
                role,
                &format!("{} block is required.", humanize_product_type(role)),
                "The active work-product profile requires this block before preview/export can be trusted.",
                "Add the missing block or complete its text.",
                &now,
            ));
        }
    }
    if product.product_type == "motion" {
        let has_relief = product.blocks.iter().any(|block| {
            block.role == "relief_requested" && block.text.split_whitespace().count() > 8
        });
        if !has_relief {
            findings.push(work_product_finding(
                product,
                "orcp-14-motion-writing-grounds-relief",
                "rules",
                "blocking",
                "block",
                "relief_requested",
                "Motion relief must be stated with particularity.",
                "ORCP 14 A requires the motion to set forth the relief or order sought.",
                "Complete the relief requested block.",
                &now,
            ));
        }
        let has_authority = product.blocks.iter().any(|block| {
            matches!(block.role.as_str(), "legal_standard" | "argument")
                && !block.authorities.is_empty()
        }) || product
            .anchors
            .iter()
            .any(|anchor| anchor.anchor_type == "authority");
        if !has_authority {
            findings.push(work_product_finding(
                product,
                "utcr-5-020-authorities",
                "authority",
                "warning",
                "work_product",
                &product.work_product_id,
                "Motion has no linked authority.",
                "UTCR 5.020 and motion practice require human review of authorities.",
                "Link controlling authority in the legal-standard or argument block.",
                &now,
            ));
        }
        let has_conferral = product
            .blocks
            .iter()
            .any(|block| block.role == "conferral_certificate");
        if !has_conferral {
            findings.push(work_product_finding(
                product,
                "utcr-5-010-conferral",
                "rules",
                "warning",
                "work_product",
                &product.work_product_id,
                "Conferral requirement needs review.",
                "Some civil motions require conferral and a certificate under UTCR 5.010.",
                "Add a conferral certificate block or mark why it is not required.",
                &now,
            ));
        }
    }
    if !product.formatting_profile.double_spaced || !product.formatting_profile.line_numbers {
        findings.push(work_product_finding(
            product,
            "utcr-2-010-document-form",
            "formatting",
            "serious",
            "formatting",
            &product.formatting_profile.profile_id,
            "Document formatting requires review.",
            "UTCR 2.010 applies form requirements to motions and other court documents.",
            "Use court-paper formatting before export.",
            &now,
        ));
    }
    findings
}

fn work_product_finding(
    product: &WorkProduct,
    rule_id: &str,
    category: &str,
    severity: &str,
    target_type: &str,
    target_id: &str,
    message: &str,
    explanation: &str,
    suggested_fix: &str,
    now: &str,
) -> WorkProductFinding {
    let finding_id = format!(
        "{}:finding:{}:{}",
        product.work_product_id,
        sanitize_path_segment(rule_id),
        sanitize_path_segment(target_id)
    );
    WorkProductFinding {
        id: finding_id.clone(),
        finding_id,
        matter_id: product.matter_id.clone(),
        work_product_id: product.work_product_id.clone(),
        rule_id: rule_id.to_string(),
        category: category.to_string(),
        severity: severity.to_string(),
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        message: message.to_string(),
        explanation: explanation.to_string(),
        suggested_fix: suggested_fix.to_string(),
        primary_action: WorkProductAction {
            action_id: format!("action:{}", sanitize_path_segment(rule_id)),
            label: suggested_fix.to_string(),
            action_type: "open_editor".to_string(),
            href: None,
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
        },
        status: "open".to_string(),
        created_at: now.to_string(),
        updated_at: now.to_string(),
    }
}

fn render_work_product_preview(product: &WorkProduct) -> WorkProductPreviewResponse {
    let mut html = String::new();
    html.push_str("<article class=\"work-product-preview\">");
    html.push_str(&format!(
        "<header><p>{}</p><h1>{}</h1><p class=\"review\">Review needed - not legal advice or filing-ready.</p></header>",
        escape_html(&product.profile.name),
        escape_html(&product.title)
    ));
    for block in &product.blocks {
        html.push_str(&format!(
            "<section data-block-id=\"{}\"><h2>{}</h2><p>{}</p></section>",
            escape_html(&block.block_id),
            escape_html(&block.title),
            escape_html(&block.text).replace('\n', "<br />")
        ));
    }
    html.push_str("</article>");
    let plain_text = work_product_plain_text(product);
    WorkProductPreviewResponse {
        work_product_id: product.work_product_id.clone(),
        matter_id: product.matter_id.clone(),
        html,
        plain_text,
        page_count: ((count_work_product_words(product) / 450) + 1).max(1),
        warnings: work_product_export_warnings(product, "preview", false, true),
        generated_at: now_string(),
        review_label: "Review needed; not legal advice or filing-ready.".to_string(),
    }
}

fn work_product_plain_text(product: &WorkProduct) -> String {
    let mut lines = vec![product.title.clone()];
    for block in &product.blocks {
        lines.push(String::new());
        lines.push(block.title.clone());
        lines.push(block.text.clone());
    }
    lines.push(String::new());
    lines.push("Review needed; not legal advice or filing-ready.".to_string());
    lines.join("\n")
}

fn render_work_product_export_content(product: &WorkProduct, format: &str) -> ApiResult<String> {
    Ok(match format {
        "html" => render_work_product_preview(product).html,
        "json" => to_payload(product)?,
        "markdown" => work_product_markdown(product),
        "text" | "plain_text" => work_product_plain_text(product),
        "pdf" | "docx" => format!(
            "{}\n\nPDF/DOCX renderer placeholder. Review needed.\n\n{}",
            product.title,
            work_product_plain_text(product)
        ),
        _ => work_product_plain_text(product),
    })
}

fn export_content_preview(content: &str) -> String {
    const PREVIEW_CHARS: usize = 16 * 1024;
    content.chars().take(PREVIEW_CHARS).collect()
}

fn work_product_markdown(product: &WorkProduct) -> String {
    let mut lines = vec![format!("# {}", product.title)];
    for block in &product.blocks {
        lines.push(format!("\n## {}", block.title));
        lines.push(block.text.clone());
    }
    lines.push("\n> Review needed; not legal advice or filing-ready.".to_string());
    lines.join("\n")
}

fn work_product_export_warnings(
    product: &WorkProduct,
    format: &str,
    include_exhibits: bool,
    include_qc_report: bool,
) -> Vec<String> {
    let mut warnings =
        vec!["Review needed; generated checks and exports are not legal advice.".to_string()];
    if product
        .findings
        .iter()
        .any(|finding| finding.status == "open")
    {
        warnings.push("Open QC findings remain.".to_string());
    }
    if product.product_type == "motion" && !include_qc_report {
        warnings.push("Motion export excludes the QC report.".to_string());
    }
    if include_exhibits
        && product
            .anchors
            .iter()
            .all(|anchor| anchor.target_type != "evidence")
    {
        warnings.push("No exhibit or evidence anchors are currently linked.".to_string());
    }
    if matches!(format, "pdf" | "docx") {
        warnings.push(
            "PDF/DOCX output is a deterministic skeleton until the dedicated renderer is enabled."
                .to_string(),
        );
    }
    warnings
}

fn work_product_qc_status(product: &WorkProduct) -> String {
    if product
        .findings
        .iter()
        .any(|finding| finding.status == "open" && finding.severity == "blocking")
    {
        "blocking".to_string()
    } else if product
        .findings
        .iter()
        .any(|finding| finding.status == "open" && finding.severity == "serious")
    {
        "serious".to_string()
    } else if product
        .findings
        .iter()
        .any(|finding| finding.status == "open")
    {
        "warning".to_string()
    } else if product.findings.is_empty() {
        "not_run".to_string()
    } else {
        "clear".to_string()
    }
}

fn normalize_export_format(value: &str) -> ApiResult<String> {
    let format = value.to_ascii_lowercase();
    let supported = [
        "pdf",
        "docx",
        "html",
        "markdown",
        "text",
        "plain_text",
        "json",
    ];
    if supported.contains(&format.as_str()) {
        Ok(format)
    } else {
        Err(ApiError::BadRequest(format!(
            "Unsupported work product export format {format}"
        )))
    }
}

fn count_work_product_words(product: &WorkProduct) -> u64 {
    product
        .blocks
        .iter()
        .map(|block| block.text.split_whitespace().count() as u64)
        .sum()
}

fn work_product_to_draft(product: &WorkProduct) -> CaseDraft {
    let sections = product
        .blocks
        .iter()
        .map(|block| DraftSection {
            section_id: block.block_id.clone(),
            heading: block.title.clone(),
            body: block.text.clone(),
            citations: block.authorities.clone(),
        })
        .collect::<Vec<_>>();
    let paragraphs = product
        .blocks
        .iter()
        .map(|block| DraftParagraph {
            paragraph_id: block.block_id.clone(),
            index: block.ordinal,
            role: block.role.clone(),
            text: block.text.clone(),
            fact_ids: block.fact_ids.clone(),
            evidence_ids: block.evidence_ids.clone(),
            authorities: block.authorities.clone(),
            factcheck_status: if block.fact_ids.is_empty() && block.evidence_ids.is_empty() {
                "needs_evidence".to_string()
            } else if block.authorities.is_empty()
                && matches!(
                    block.role.as_str(),
                    "legal_standard" | "argument" | "analysis"
                )
            {
                "needs_authority".to_string()
            } else {
                "supported".to_string()
            },
            factcheck_note: None,
        })
        .collect::<Vec<_>>();
    CaseDraft {
        id: product.work_product_id.clone(),
        draft_id: product.work_product_id.clone(),
        matter_id: product.matter_id.clone(),
        title: product.title.clone(),
        description: format!(
            "Migrated shared {} work product.",
            humanize_product_type(&product.product_type)
        ),
        draft_type: product.product_type.clone(),
        kind: product.product_type.clone(),
        status: product.status.clone(),
        created_at: product.created_at.clone(),
        updated_at: product.updated_at.clone(),
        word_count: count_work_product_words(product),
        sections,
        paragraphs,
    }
}

fn work_product_from_draft(draft: &CaseDraft) -> WorkProduct {
    let mut product = WorkProduct {
        id: draft.draft_id.clone(),
        work_product_id: draft.draft_id.clone(),
        matter_id: draft.matter_id.clone(),
        title: draft.title.clone(),
        product_type: draft.kind.clone(),
        status: draft.status.clone(),
        review_status: "needs_human_review".to_string(),
        setup_stage: "migrated_from_draft".to_string(),
        source_draft_id: Some(draft.draft_id.clone()),
        source_complaint_id: None,
        created_at: draft.created_at.clone(),
        updated_at: draft.updated_at.clone(),
        profile: work_product_profile(&draft.kind),
        document_ast: WorkProductDocument::default(),
        blocks: work_product_blocks_from_draft(draft),
        marks: Vec::new(),
        anchors: Vec::new(),
        findings: Vec::new(),
        artifacts: Vec::new(),
        history: vec![work_product_event(
            &draft.matter_id,
            &draft.draft_id,
            "migrated_from_draft",
            "draft",
            &draft.draft_id,
            "Legacy Draft node migrated into the shared WorkProduct AST.",
        )],
        ai_commands: default_work_product_ai_commands(&draft.kind),
        formatting_profile: default_work_product_formatting_profile(&draft.kind),
        rule_pack: work_product_rule_pack(&draft.kind),
    };
    refresh_work_product_state(&mut product);
    product
}

fn work_product_blocks_from_draft(draft: &CaseDraft) -> Vec<WorkProductBlock> {
    let section_blocks =
        draft
            .sections
            .iter()
            .enumerate()
            .map(|(index, section)| WorkProductBlock {
                id: section.section_id.clone(),
                block_id: section.section_id.clone(),
                matter_id: draft.matter_id.clone(),
                work_product_id: draft.draft_id.clone(),
                block_type: "section".to_string(),
                role: slug(&section.heading).replace('-', "_"),
                title: section.heading.clone(),
                text: section.body.clone(),
                ordinal: index as u64 + 1,
                parent_block_id: None,
                fact_ids: Vec::new(),
                evidence_ids: Vec::new(),
                authorities: section.citations.clone(),
                mark_ids: Vec::new(),
                locked: false,
                review_status: "needs_review".to_string(),
                prosemirror_json: Some(prosemirror_doc_for_text(&section.body)),
                ..WorkProductBlock::default()
            });
    let offset = draft.sections.len() as u64;
    let paragraph_blocks = draft
        .paragraphs
        .iter()
        .enumerate()
        .map(|(index, paragraph)| WorkProductBlock {
            id: paragraph.paragraph_id.clone(),
            block_id: paragraph.paragraph_id.clone(),
            matter_id: draft.matter_id.clone(),
            work_product_id: draft.draft_id.clone(),
            block_type: "paragraph".to_string(),
            role: paragraph.role.clone(),
            title: humanize_product_type(&paragraph.role),
            text: paragraph.text.clone(),
            ordinal: offset + index as u64 + 1,
            parent_block_id: None,
            fact_ids: paragraph.fact_ids.clone(),
            evidence_ids: paragraph.evidence_ids.clone(),
            authorities: paragraph.authorities.clone(),
            mark_ids: Vec::new(),
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: Some(prosemirror_doc_for_text(&paragraph.text)),
            ..WorkProductBlock::default()
        });
    section_blocks.chain(paragraph_blocks).collect()
}

fn work_product_from_complaint(complaint: &ComplaintDraft) -> WorkProduct {
    let mut blocks = Vec::new();
    blocks.push(WorkProductBlock {
        id: format!("{}:block:caption", complaint.complaint_id),
        block_id: format!("{}:block:caption", complaint.complaint_id),
        matter_id: complaint.matter_id.clone(),
        work_product_id: complaint.complaint_id.clone(),
        block_type: "caption".to_string(),
        role: "caption".to_string(),
        title: "Caption".to_string(),
        text: format!(
            "{}\n{} v. {}",
            complaint.caption.document_title,
            complaint.caption.plaintiff_names.join(", "),
            complaint.caption.defendant_names.join(", ")
        ),
        ordinal: 1,
        parent_block_id: None,
        fact_ids: Vec::new(),
        evidence_ids: Vec::new(),
        authorities: Vec::new(),
        mark_ids: Vec::new(),
        locked: false,
        review_status: complaint.review_status.clone(),
        prosemirror_json: None,
        ..WorkProductBlock::default()
    });
    for section in &complaint.sections {
        blocks.push(WorkProductBlock {
            id: section.section_id.clone(),
            block_id: section.section_id.clone(),
            matter_id: complaint.matter_id.clone(),
            work_product_id: complaint.complaint_id.clone(),
            block_type: "section".to_string(),
            role: section.section_type.clone(),
            title: section.title.clone(),
            text: String::new(),
            ordinal: blocks.len() as u64 + 1,
            parent_block_id: None,
            fact_ids: Vec::new(),
            evidence_ids: Vec::new(),
            authorities: Vec::new(),
            mark_ids: Vec::new(),
            locked: false,
            review_status: section.review_status.clone(),
            prosemirror_json: None,
            ..WorkProductBlock::default()
        });
    }
    for count in &complaint.counts {
        blocks.push(WorkProductBlock {
            id: count.count_id.clone(),
            block_id: count.count_id.clone(),
            matter_id: complaint.matter_id.clone(),
            work_product_id: complaint.complaint_id.clone(),
            block_type: "count".to_string(),
            role: "count".to_string(),
            title: count.title.clone(),
            text: count.legal_theory.clone(),
            ordinal: blocks.len() as u64 + 1,
            parent_block_id: None,
            fact_ids: count.fact_ids.clone(),
            evidence_ids: count.evidence_ids.clone(),
            authorities: count.authorities.clone(),
            mark_ids: Vec::new(),
            locked: false,
            review_status: count.health.clone(),
            prosemirror_json: None,
            ..WorkProductBlock::default()
        });
    }
    for paragraph in &complaint.paragraphs {
        let evidence_ids = paragraph
            .evidence_uses
            .iter()
            .filter_map(|use_ref| {
                use_ref
                    .evidence_id
                    .clone()
                    .or_else(|| use_ref.document_id.clone())
            })
            .collect::<Vec<_>>();
        let authorities = paragraph
            .citation_uses
            .iter()
            .filter_map(|citation| {
                citation
                    .canonical_id
                    .clone()
                    .map(|canonical_id| AuthorityRef {
                        citation: citation.citation.clone(),
                        canonical_id,
                        reason: Some("Complaint citation use".to_string()),
                        pinpoint: citation.pinpoint.clone(),
                    })
            })
            .collect::<Vec<_>>();
        blocks.push(WorkProductBlock {
            id: paragraph.paragraph_id.clone(),
            block_id: paragraph.paragraph_id.clone(),
            matter_id: complaint.matter_id.clone(),
            work_product_id: complaint.complaint_id.clone(),
            block_type: "paragraph".to_string(),
            role: paragraph.role.clone(),
            title: format!("Paragraph {}", paragraph.number),
            text: paragraph.text.clone(),
            ordinal: blocks.len() as u64 + 1,
            parent_block_id: paragraph
                .section_id
                .clone()
                .or_else(|| paragraph.count_id.clone()),
            fact_ids: paragraph.fact_ids.clone(),
            evidence_ids,
            authorities,
            mark_ids: Vec::new(),
            locked: paragraph.locked,
            review_status: paragraph.review_status.clone(),
            prosemirror_json: Some(prosemirror_doc_for_text(&paragraph.text)),
            ..WorkProductBlock::default()
        });
    }
    let mut product = WorkProduct {
        id: complaint.complaint_id.clone(),
        work_product_id: complaint.complaint_id.clone(),
        matter_id: complaint.matter_id.clone(),
        title: complaint.title.clone(),
        product_type: "complaint".to_string(),
        status: complaint.status.clone(),
        review_status: complaint.review_status.clone(),
        setup_stage: complaint.setup_stage.clone(),
        source_draft_id: None,
        source_complaint_id: Some(complaint.complaint_id.clone()),
        created_at: complaint.created_at.clone(),
        updated_at: complaint.updated_at.clone(),
        profile: work_product_profile("complaint"),
        document_ast: WorkProductDocument::default(),
        blocks,
        marks: Vec::new(),
        anchors: Vec::new(),
        findings: complaint
            .findings
            .iter()
            .map(|finding| WorkProductFinding {
                id: finding.finding_id.clone(),
                finding_id: finding.finding_id.clone(),
                matter_id: finding.matter_id.clone(),
                work_product_id: complaint.complaint_id.clone(),
                rule_id: finding.rule_id.clone(),
                category: finding.category.clone(),
                severity: finding.severity.clone(),
                target_type: finding.target_type.clone(),
                target_id: finding.target_id.clone(),
                message: finding.message.clone(),
                explanation: finding.explanation.clone(),
                suggested_fix: finding.suggested_fix.clone(),
                primary_action: WorkProductAction {
                    action_id: finding.primary_action.action_id.clone(),
                    label: finding.primary_action.label.clone(),
                    action_type: finding.primary_action.action_type.clone(),
                    href: finding.primary_action.href.clone(),
                    target_type: finding.primary_action.target_type.clone(),
                    target_id: finding.primary_action.target_id.clone(),
                },
                status: finding.status.clone(),
                created_at: finding.created_at.clone(),
                updated_at: finding.updated_at.clone(),
            })
            .collect(),
        artifacts: complaint
            .export_artifacts
            .iter()
            .map(|artifact| WorkProductArtifact {
                id: artifact.artifact_id.clone(),
                artifact_id: artifact.artifact_id.clone(),
                matter_id: artifact.matter_id.clone(),
                work_product_id: complaint.complaint_id.clone(),
                format: artifact.format.clone(),
                profile: artifact.profile.clone(),
                mode: artifact.mode.clone(),
                status: artifact.status.clone(),
                download_url: artifact.download_url.clone(),
                page_count: artifact.page_count,
                generated_at: artifact.generated_at.clone(),
                warnings: artifact.warnings.clone(),
                content_preview: artifact.content_preview.clone(),
                snapshot_id: None,
                artifact_hash: Some(sha256_hex(artifact.content_preview.as_bytes())),
                render_profile_hash: Some(sha256_hex(
                    format!("{}:{}:{}", artifact.format, artifact.profile, artifact.mode)
                        .as_bytes(),
                )),
                qc_status_at_export: None,
                changed_since_export: Some(false),
                immutable: Some(true),
                object_blob_id: None,
                size_bytes: Some(artifact.content_preview.len() as u64),
                mime_type: Some(export_mime_type(&artifact.format).to_string()),
                storage_status: Some("legacy_inline".to_string()),
            })
            .collect(),
        history: complaint
            .history
            .iter()
            .map(|event| WorkProductHistoryEvent {
                id: event.event_id.clone(),
                event_id: event.event_id.clone(),
                matter_id: event.matter_id.clone(),
                work_product_id: complaint.complaint_id.clone(),
                event_type: event.event_type.clone(),
                target_type: event.target_type.clone(),
                target_id: event.target_id.clone(),
                summary: event.summary.clone(),
                timestamp: event.timestamp.clone(),
            })
            .collect(),
        ai_commands: default_work_product_ai_commands("complaint"),
        formatting_profile: complaint.formatting_profile.clone(),
        rule_pack: complaint.rule_pack.clone(),
    };
    refresh_work_product_state(&mut product);
    product
}

fn default_complaint_from_matter(
    matter: &MatterSummary,
    complaint_id: &str,
    title: &str,
    parties: &[CaseParty],
    claims: &[CaseClaim],
    facts: &[CaseFact],
    now: &str,
) -> ComplaintDraft {
    let matter_id = matter.matter_id.clone();
    let complaint_parties = if parties.is_empty() {
        vec![
            ComplaintParty {
                party_id: format!("{complaint_id}:party:plaintiff"),
                matter_party_id: None,
                name: "Plaintiff".to_string(),
                role: "plaintiff".to_string(),
                party_type: "individual".to_string(),
                represented_by: None,
            },
            ComplaintParty {
                party_id: format!("{complaint_id}:party:defendant"),
                matter_party_id: None,
                name: "Defendant".to_string(),
                role: "defendant".to_string(),
                party_type: "entity".to_string(),
                represented_by: None,
            },
        ]
    } else {
        parties
            .iter()
            .map(|party| ComplaintParty {
                party_id: format!(
                    "{complaint_id}:party:{}",
                    sanitize_path_segment(&party.party_id)
                ),
                matter_party_id: Some(party.party_id.clone()),
                name: party.name.clone(),
                role: party.role.clone(),
                party_type: party.party_type.clone(),
                represented_by: party.represented_by.clone(),
            })
            .collect()
    };
    let plaintiff_names = role_names(&complaint_parties, &["plaintiff", "petitioner"]);
    let defendant_names = role_names(&complaint_parties, &["defendant", "respondent"]);
    let sections = vec![
        complaint_section(
            &matter_id,
            complaint_id,
            "jurisdiction",
            "Jurisdiction and Venue",
            1,
        ),
        complaint_section(&matter_id, complaint_id, "facts", "Factual Allegations", 2),
        complaint_section(&matter_id, complaint_id, "counts", "Claims for Relief", 3),
        complaint_section(&matter_id, complaint_id, "relief", "Prayer for Relief", 4),
    ];
    let mut paragraphs = Vec::new();
    paragraphs.push(pleading_paragraph(
        &matter_id,
        complaint_id,
        &format!("{complaint_id}:paragraph:1"),
        Some(sections[0].section_id.clone()),
        None,
        "jurisdiction_venue",
        &format!(
            "This action is brought in {} and venue is proper in {}.",
            empty_as_review_needed(&matter.court),
            empty_as_review_needed(&matter.jurisdiction)
        ),
        1,
        Vec::new(),
        Vec::new(),
    ));
    if facts.is_empty() {
        paragraphs.push(pleading_paragraph(
            &matter_id,
            complaint_id,
            &format!("{complaint_id}:paragraph:2"),
            Some(sections[1].section_id.clone()),
            None,
            "factual_allegation",
            "Plaintiff alleges the following ultimate facts after human review and evidentiary support are added.",
            2,
            Vec::new(),
            Vec::new(),
        ));
    } else {
        for (index, fact) in facts.iter().take(12).enumerate() {
            paragraphs.push(pleading_paragraph(
                &matter_id,
                complaint_id,
                &format!("{complaint_id}:paragraph:{}", index + 2),
                Some(sections[1].section_id.clone()),
                None,
                "factual_allegation",
                &fact.statement,
                index as u64 + 2,
                vec![fact.fact_id.clone()],
                fact.source_evidence_ids.clone(),
            ));
        }
    }
    let mut counts = Vec::new();
    let complaint_claims = claims
        .iter()
        .filter(|claim| claim.kind != "defense")
        .take(8)
        .collect::<Vec<_>>();
    for claim in complaint_claims {
        let count_id = format!("{complaint_id}:count:{}", counts.len() + 1);
        let paragraph_id = format!("{complaint_id}:paragraph:{}", paragraphs.len() + 1);
        paragraphs.push(pleading_paragraph(
            &matter_id,
            complaint_id,
            &paragraph_id,
            Some(sections[2].section_id.clone()),
            Some(count_id.clone()),
            "count_heading",
            &format!("COUNT {} - {}", counts.len() + 1, claim.title),
            paragraphs.len() as u64 + 1,
            claim.fact_ids.clone(),
            claim.evidence_ids.clone(),
        ));
        counts.push(ComplaintCount {
            id: count_id.clone(),
            count_id,
            matter_id: matter_id.clone(),
            complaint_id: complaint_id.to_string(),
            ordinal: counts.len() as u64 + 1,
            title: claim.title.clone(),
            claim_id: Some(claim.claim_id.clone()),
            legal_theory: claim.legal_theory.clone(),
            against_party_ids: Vec::new(),
            element_ids: claim
                .elements
                .iter()
                .map(|element| element.element_id.clone())
                .collect(),
            fact_ids: claim.fact_ids.clone(),
            evidence_ids: claim.evidence_ids.clone(),
            authorities: claim.authorities.clone(),
            relief_ids: Vec::new(),
            paragraph_ids: vec![paragraph_id],
            incorporation_range: Some("1 through preceding paragraph".to_string()),
            health: "needs_review".to_string(),
            weaknesses: Vec::new(),
        });
    }
    let relief_id = format!("{complaint_id}:relief:general");
    let relief = vec![ReliefRequest {
        id: relief_id.clone(),
        relief_id,
        matter_id: matter_id.clone(),
        complaint_id: complaint_id.to_string(),
        category: "general".to_string(),
        text: "Plaintiff requests relief determined by the court after human review, including any damages, costs, fees, or equitable relief supported by law and evidence.".to_string(),
        amount: None,
        authority_ids: Vec::new(),
        supported: false,
    }];
    let mut complaint = ComplaintDraft {
        complaint_id: complaint_id.to_string(),
        id: complaint_id.to_string(),
        matter_id: matter_id.clone(),
        title: title.to_string(),
        status: "draft".to_string(),
        review_status: "needs_human_review".to_string(),
        setup_stage: "guided_setup".to_string(),
        active_profile_id: "oregon-circuit-civil-complaint".to_string(),
        created_at: now.to_string(),
        updated_at: now.to_string(),
        caption: ComplaintCaption {
            court_name: matter.court.clone(),
            county: infer_county(&matter.court),
            case_number: matter.case_number.clone(),
            document_title: "Complaint".to_string(),
            plaintiff_names,
            defendant_names,
            jury_demand: false,
            jurisdiction: matter.jurisdiction.clone(),
            venue: matter.court.clone(),
        },
        parties: complaint_parties,
        sections,
        counts,
        paragraphs,
        relief,
        signature: SignatureBlock {
            name: String::new(),
            bar_number: None,
            firm: None,
            address: String::new(),
            phone: String::new(),
            email: String::new(),
            signature_date: None,
        },
        certificate_of_service: Some(CertificateOfService {
            certificate_id: format!("{complaint_id}:certificate:service"),
            method: "review_needed".to_string(),
            served_parties: Vec::new(),
            service_date: None,
            text: "Certificate of service requires human review before filing or service."
                .to_string(),
            review_status: "needs_review".to_string(),
        }),
        formatting_profile: default_formatting_profile(),
        rule_pack: oregon_civil_complaint_rule_pack(),
        findings: Vec::new(),
        export_artifacts: Vec::new(),
        history: vec![complaint_event(
            &matter_id,
            complaint_id,
            "complaint_created",
            "complaint",
            complaint_id,
            "Structured complaint AST created.",
        )],
        next_actions: Vec::new(),
        ai_commands: default_ai_commands(),
        filing_packet: FilingPacket {
            packet_id: format!("{complaint_id}:packet:filing"),
            matter_id: matter_id.clone(),
            complaint_id: complaint_id.to_string(),
            status: "review_needed".to_string(),
            items: Vec::new(),
            warnings: Vec::new(),
        },
        import_provenance: None,
    };
    apply_matter_rule_profile(
        &mut complaint.rule_pack,
        matter,
        now.get(0..10).unwrap_or(now),
        "complaint",
    );
    refresh_complaint_state(&mut complaint);
    complaint
}

fn build_imported_complaint(
    matter: &MatterSummary,
    document: &CaseDocument,
    complaint_id: &str,
    title: &str,
    text: &str,
    parser_id: &str,
    context: &SourceContext,
    parties: &[CaseParty],
    claims: &[CaseClaim],
    facts: &[CaseFact],
    evidence: &[CaseEvidence],
    now: &str,
) -> ComplaintDraft {
    let mut complaint =
        default_complaint_from_matter(matter, complaint_id, title, parties, claims, &[], now);
    complaint.status = "imported_draft".to_string();
    complaint.setup_stage = "editor".to_string();
    complaint.caption = imported_caption(matter, text, parties);
    complaint.import_provenance = Some(ComplaintImportProvenance {
        document_id: document.document_id.clone(),
        document_version_id: context.document_version_id.clone(),
        object_blob_id: context.object_blob_id.clone(),
        ingestion_run_id: context.ingestion_run_id.clone(),
        source_span_id: None,
        parser_id: parser_id.to_string(),
        parser_version: PARSER_REGISTRY_VERSION.to_string(),
        byte_start: Some(0),
        byte_end: Some(text.len() as u64),
        char_start: Some(0),
        char_end: Some(text.chars().count() as u64),
    });

    let parsed = parse_complaint_structure(text);
    let section_rows = if parsed.sections.is_empty() {
        vec![(
            "imported".to_string(),
            "Imported Draft".to_string(),
            "imported".to_string(),
        )]
    } else {
        parsed.sections
    };
    complaint.sections = section_rows
        .iter()
        .enumerate()
        .map(|(index, (key, title, section_type))| ComplaintSection {
            id: format!("{complaint_id}:section:{}", sanitize_path_segment(key)),
            section_id: format!("{complaint_id}:section:{}", sanitize_path_segment(key)),
            matter_id: matter.matter_id.clone(),
            complaint_id: complaint_id.to_string(),
            title: title.clone(),
            section_type: section_type.clone(),
            ordinal: index as u64 + 1,
            paragraph_ids: Vec::new(),
            count_ids: Vec::new(),
            review_status: "needs_review".to_string(),
        })
        .collect();

    complaint.counts = parsed
        .counts
        .iter()
        .enumerate()
        .map(|(index, (count_key, title))| {
            let count_id = format!("{complaint_id}:count:{}", sanitize_path_segment(count_key));
            ComplaintCount {
                id: count_id.clone(),
                count_id,
                matter_id: matter.matter_id.clone(),
                complaint_id: complaint_id.to_string(),
                ordinal: index as u64 + 1,
                title: title.clone(),
                claim_id: match_claim_for_count(title, claims),
                legal_theory: title.clone(),
                against_party_ids: Vec::new(),
                element_ids: Vec::new(),
                fact_ids: Vec::new(),
                evidence_ids: Vec::new(),
                authorities: Vec::new(),
                relief_ids: Vec::new(),
                paragraph_ids: Vec::new(),
                incorporation_range: Some("1 through preceding paragraph".to_string()),
                health: "needs_review".to_string(),
                weaknesses: Vec::new(),
            }
        })
        .collect();

    let imported_paragraphs = if parsed.paragraphs.is_empty() {
        vec![ParsedComplaintParagraph {
            original_label: "1".to_string(),
            text: summarize_text(text),
            section_key: complaint
                .sections
                .first()
                .map(|section| section.section_type.clone())
                .unwrap_or_else(|| "imported".to_string()),
            count_key: None,
            byte_start: 0,
            byte_end: text.len() as u64,
            char_start: 0,
            char_end: text.chars().count() as u64,
        }]
    } else {
        parsed.paragraphs
    };
    complaint.paragraphs = imported_paragraphs
        .into_iter()
        .enumerate()
        .map(|(index, parsed)| {
            imported_pleading_paragraph(
                matter,
                complaint_id,
                document,
                parser_id,
                context,
                &complaint.sections,
                &complaint.counts,
                facts,
                evidence,
                index as u64 + 1,
                parsed,
            )
        })
        .collect();

    for count in &mut complaint.counts {
        count.paragraph_ids = complaint
            .paragraphs
            .iter()
            .filter(|paragraph| paragraph.count_id.as_deref() == Some(count.count_id.as_str()))
            .map(|paragraph| paragraph.paragraph_id.clone())
            .collect();
        let mut authorities = Vec::new();
        for paragraph in complaint
            .paragraphs
            .iter()
            .filter(|paragraph| paragraph.count_id.as_deref() == Some(count.count_id.as_str()))
        {
            for citation in &paragraph.citation_uses {
                if let Some(canonical_id) = &citation.canonical_id {
                    push_authority(
                        &mut authorities,
                        AuthorityRef {
                            citation: citation.citation.clone(),
                            canonical_id: canonical_id.clone(),
                            reason: Some("Imported citation from complaint count.".to_string()),
                            pinpoint: citation.pinpoint.clone(),
                        },
                    );
                }
            }
        }
        count.authorities = authorities;
    }

    complaint.relief = imported_relief_requests(matter, complaint_id, &complaint.paragraphs);
    complaint.history = vec![complaint_event(
        &matter.matter_id,
        complaint_id,
        "complaint_import_started",
        "document",
        &document.document_id,
        "Structured complaint AST created from uploaded draft.",
    )];
    complaint.findings = complaint_rule_findings(&complaint);
    refresh_complaint_state(&mut complaint);
    complaint
}

fn imported_pleading_paragraph(
    matter: &MatterSummary,
    complaint_id: &str,
    document: &CaseDocument,
    parser_id: &str,
    context: &SourceContext,
    sections: &[ComplaintSection],
    counts: &[ComplaintCount],
    facts: &[CaseFact],
    evidence: &[CaseEvidence],
    ordinal: u64,
    parsed: ParsedComplaintParagraph,
) -> PleadingParagraph {
    let paragraph_id = format!("{complaint_id}:paragraph:{ordinal}");
    let section_id = sections
        .iter()
        .find(|section| {
            section
                .section_id
                .ends_with(&sanitize_path_segment(&parsed.section_key))
        })
        .or_else(|| sections.first())
        .map(|section| section.section_id.clone());
    let count_id = parsed.count_key.as_ref().and_then(|count_key| {
        let target = format!("{complaint_id}:count:{}", sanitize_path_segment(count_key));
        counts
            .iter()
            .find(|count| count.count_id == target)
            .map(|count| count.count_id.clone())
    });
    let role = role_for_imported_paragraph(section_id.as_deref(), sections, count_id.as_deref());
    let (fact_ids, evidence_uses) = exact_support_for_paragraph(
        matter,
        complaint_id,
        &paragraph_id,
        &parsed.text,
        facts,
        evidence,
    );
    let mut paragraph = pleading_paragraph(
        &matter.matter_id,
        complaint_id,
        &paragraph_id,
        section_id,
        count_id,
        &role,
        &parsed.text,
        ordinal,
        fact_ids,
        Vec::new(),
    );
    let source_span_id = source_span_id(&document.document_id, "pleading-paragraph", ordinal);
    paragraph.display_number = Some(parsed.original_label.clone());
    paragraph.original_label = Some(parsed.original_label);
    paragraph.source_span_id = Some(source_span_id.clone());
    paragraph.import_provenance = Some(ComplaintImportProvenance {
        document_id: document.document_id.clone(),
        document_version_id: context.document_version_id.clone(),
        object_blob_id: context.object_blob_id.clone(),
        ingestion_run_id: context.ingestion_run_id.clone(),
        source_span_id: Some(source_span_id),
        parser_id: parser_id.to_string(),
        parser_version: PARSER_REGISTRY_VERSION.to_string(),
        byte_start: Some(parsed.byte_start),
        byte_end: Some(parsed.byte_end),
        char_start: Some(parsed.char_start),
        char_end: Some(parsed.char_end),
    });
    paragraph.citation_uses = citation_uses_for_text(
        &matter.matter_id,
        complaint_id,
        &paragraph_id,
        "paragraph",
        &parsed.text,
    );
    paragraph.exhibit_references =
        exhibit_references_for_text(&matter.matter_id, complaint_id, &paragraph_id, &parsed.text);
    paragraph.evidence_uses = evidence_uses;
    paragraph.review_status = "imported_needs_review".to_string();
    paragraph
}

fn parse_complaint_structure(text: &str) -> ParsedComplaintStructure {
    let mut structure = ParsedComplaintStructure::default();
    ensure_import_section(
        &mut structure.sections,
        "imported",
        "Imported Draft",
        "imported",
    );
    let mut current_section_key = "imported".to_string();
    let mut current_count_key: Option<String> = None;
    let mut current: Option<ParsedComplaintParagraph> = None;
    let mut cursor = 0usize;

    for raw_line in text.split_inclusive('\n') {
        let line_start = cursor;
        let line_end = cursor + raw_line.len();
        cursor = line_end;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            if let Some(paragraph) = current.as_mut() {
                paragraph.text.push('\n');
                paragraph.byte_end = line_end as u64;
                paragraph.char_end = text[..line_end].chars().count() as u64;
            }
            continue;
        }

        if let Some(heading) = imported_heading(trimmed) {
            finish_import_paragraph(&mut structure.paragraphs, current.take());
            if CLAIM_HEADING_RE.is_match(&heading) {
                ensure_import_section(
                    &mut structure.sections,
                    "claims",
                    "Claims for Relief",
                    "counts",
                );
                current_section_key = "claims".to_string();
                let count_key = format!("count-{}", structure.counts.len() + 1);
                structure.counts.push((count_key.clone(), heading));
                current_count_key = Some(count_key);
            } else {
                let key = sanitize_path_segment(&heading.to_ascii_lowercase());
                let section_type = section_type_for_heading(&heading);
                ensure_import_section(&mut structure.sections, &key, &heading, &section_type);
                current_section_key = key;
                current_count_key = None;
            }
            continue;
        }

        if let Some(caps) = PLEADING_PARAGRAPH_RE.captures(trimmed) {
            finish_import_paragraph(&mut structure.paragraphs, current.take());
            let label = caps.get(1).map(|m| m.as_str()).unwrap_or("0").to_string();
            let body = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim();
            current = Some(ParsedComplaintParagraph {
                original_label: label,
                text: body.to_string(),
                section_key: current_section_key.clone(),
                count_key: current_count_key.clone(),
                byte_start: line_start as u64,
                byte_end: line_end as u64,
                char_start: text[..line_start].chars().count() as u64,
                char_end: text[..line_end].chars().count() as u64,
            });
        } else if let Some(paragraph) = current.as_mut() {
            if !paragraph.text.ends_with('\n') && !paragraph.text.is_empty() {
                paragraph.text.push(' ');
            }
            paragraph.text.push_str(trimmed);
            paragraph.byte_end = line_end as u64;
            paragraph.char_end = text[..line_end].chars().count() as u64;
        }
    }
    finish_import_paragraph(&mut structure.paragraphs, current);
    structure
}

fn finish_import_paragraph(
    paragraphs: &mut Vec<ParsedComplaintParagraph>,
    paragraph: Option<ParsedComplaintParagraph>,
) {
    if let Some(mut paragraph) = paragraph {
        paragraph.text = paragraph
            .text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !paragraph.text.is_empty() {
            paragraphs.push(paragraph);
        }
    }
}

fn ensure_import_section(
    sections: &mut Vec<(String, String, String)>,
    key: &str,
    title: &str,
    section_type: &str,
) {
    if !sections.iter().any(|(existing, _, _)| existing == key) {
        sections.push((key.to_string(), title.to_string(), section_type.to_string()));
    }
}

fn imported_heading(line: &str) -> Option<String> {
    let stripped = line.trim().trim_matches('*').trim();
    let markdown = stripped.strip_prefix('#').map(|_| {
        stripped
            .trim_start_matches('#')
            .trim()
            .trim_matches('*')
            .trim()
            .to_string()
    });
    if let Some(value) = markdown.filter(|value| !value.is_empty()) {
        return Some(value);
    }
    if CLAIM_HEADING_RE.is_match(stripped) {
        return Some(stripped.to_string());
    }
    None
}

fn section_type_for_heading(heading: &str) -> String {
    let value = heading.to_ascii_lowercase();
    if value.contains("caption") {
        "caption".to_string()
    } else if value.contains("jurisdiction") || value.contains("venue") {
        "jurisdiction".to_string()
    } else if value.contains("parties") {
        "parties".to_string()
    } else if value.contains("fact") || value.contains("allegation") {
        "facts".to_string()
    } else if value.contains("prayer") || value.contains("relief") {
        "relief".to_string()
    } else if value.contains("exhibit") {
        "exhibits".to_string()
    } else {
        "custom".to_string()
    }
}

fn role_for_imported_paragraph(
    section_id: Option<&str>,
    sections: &[ComplaintSection],
    count_id: Option<&str>,
) -> String {
    if count_id.is_some() {
        return "count_allegation".to_string();
    }
    let section_type = section_id
        .and_then(|id| sections.iter().find(|section| section.section_id == id))
        .map(|section| section.section_type.as_str())
        .unwrap_or("imported");
    match section_type {
        "jurisdiction" => "jurisdiction_venue",
        "relief" => "relief",
        "caption" | "parties" | "exhibits" => "procedural_notice",
        _ => "factual_allegation",
    }
    .to_string()
}

fn imported_caption(matter: &MatterSummary, text: &str, parties: &[CaseParty]) -> ComplaintCaption {
    let court_name = text
        .lines()
        .map(str::trim)
        .find(|line| line.to_ascii_uppercase().contains("CIRCUIT COURT"))
        .map(clean_caption_line)
        .unwrap_or_else(|| matter.court.clone());
    let county = text
        .lines()
        .find_map(|line| {
            let upper = line.to_ascii_uppercase();
            upper
                .find("COUNTY OF ")
                .map(|idx| clean_caption_line(&line[idx + "COUNTY OF ".len()..]))
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| infer_county(&matter.court));
    let plaintiffs = role_names_from_parties(parties, &["plaintiff", "petitioner"])
        .or_else(|| caption_names_before(text, "Plaintiffs,"))
        .unwrap_or_else(|| vec!["Plaintiff".to_string()]);
    let defendants = role_names_from_parties(parties, &["defendant", "respondent"])
        .or_else(|| caption_names_before(text, "Defendants."))
        .unwrap_or_else(|| vec!["Defendant".to_string()]);
    ComplaintCaption {
        court_name,
        county: county.clone(),
        case_number: matter.case_number.clone(),
        document_title: "Complaint".to_string(),
        plaintiff_names: plaintiffs,
        defendant_names: defendants,
        jury_demand: text.to_ascii_uppercase().contains("JURY TRIAL"),
        jurisdiction: matter.jurisdiction.clone(),
        venue: if county.is_empty() {
            matter.court.clone()
        } else {
            format!("{county} County")
        },
    }
}

fn role_names_from_parties(parties: &[CaseParty], roles: &[&str]) -> Option<Vec<String>> {
    let names = parties
        .iter()
        .filter(|party| {
            roles
                .iter()
                .any(|role| party.role.eq_ignore_ascii_case(role))
        })
        .map(|party| party.name.clone())
        .collect::<Vec<_>>();
    if names.is_empty() {
        None
    } else {
        Some(names)
    }
}

fn caption_names_before(text: &str, marker: &str) -> Option<Vec<String>> {
    let lines = text.lines().collect::<Vec<_>>();
    let marker_index = lines.iter().position(|line| {
        line.to_ascii_lowercase()
            .contains(&marker.to_ascii_lowercase())
    })?;
    let mut names = Vec::new();
    for line in lines[..marker_index].iter().rev().take(8).rev() {
        let cleaned = clean_caption_line(line);
        if cleaned.is_empty()
            || cleaned.contains("COURT")
            || cleaned.contains("COUNTY")
            || cleaned.eq_ignore_ascii_case("v.")
        {
            continue;
        }
        if cleaned
            .chars()
            .filter(|ch| ch.is_ascii_alphabetic())
            .count()
            >= 3
        {
            names.push(cleaned.trim_matches(',').to_string());
        }
    }
    if names.is_empty() {
        None
    } else {
        Some(names)
    }
}

fn clean_caption_line(value: &str) -> String {
    value
        .trim()
        .trim_matches('*')
        .trim_matches(',')
        .trim()
        .to_string()
}

fn imported_complaint_title(document: &CaseDocument, text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| line.to_ascii_uppercase().contains("COMPLAINT"))
        .map(|line| {
            line.trim_start_matches('#')
                .trim()
                .trim_matches('*')
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .unwrap_or_else(|| format!("{} structured complaint", document.title))
}

fn looks_like_complaint(filename: &str, text: &str) -> bool {
    let lower_name = filename.to_ascii_lowercase();
    let upper = text.to_ascii_uppercase();
    let numbered = PLEADING_PARAGRAPH_RE.find_iter(text).take(12).count();
    let signals = [
        lower_name.contains("complaint"),
        upper.contains("CIRCUIT COURT"),
        upper.contains("COMPLAINT"),
        upper.contains("CLAIM FOR RELIEF"),
        upper.contains("PRAYER FOR RELIEF"),
        upper.contains("PLAINTIFF"),
        upper.contains("DEFENDANT"),
        ORS_CITATION_RE.is_match(text)
            || ORCP_CITATION_RE.is_match(text)
            || UTCR_CITATION_RE.is_match(text),
        numbered >= 3,
    ];
    signals.iter().filter(|signal| **signal).count() >= 3
}

fn parser_id_for_document(document: &CaseDocument) -> String {
    let filename = document.filename.to_ascii_lowercase();
    let mime = document
        .mime_type
        .clone()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if filename.ends_with(".md") || filename.ends_with(".markdown") {
        "casebuilder-markdown-v1"
    } else if filename.ends_with(".html") || filename.ends_with(".htm") || mime == "text/html" {
        "casebuilder-html-text-v1"
    } else if filename.ends_with(".csv") || mime == "text/csv" {
        "casebuilder-csv-text-v1"
    } else if filename.ends_with(".pdf") || mime == "application/pdf" {
        "casebuilder-pdf-embedded-text-v1"
    } else if filename.ends_with(".docx") {
        "casebuilder-docx-text-v1"
    } else {
        "casebuilder-plain-text-v1"
    }
    .to_string()
}

fn exact_support_for_paragraph(
    matter: &MatterSummary,
    complaint_id: &str,
    paragraph_id: &str,
    text: &str,
    facts: &[CaseFact],
    evidence: &[CaseEvidence],
) -> (Vec<String>, Vec<EvidenceUse>) {
    let normalized = normalize_for_match(text);
    let mut fact_ids = Vec::new();
    for fact in facts {
        let candidate = normalize_for_match(&fact.statement);
        if !candidate.is_empty()
            && (normalized.contains(&candidate) || candidate.contains(&normalized))
        {
            push_unique(&mut fact_ids, fact.fact_id.clone());
        }
    }
    let mut evidence_uses = Vec::new();
    for item in evidence {
        let quote = normalize_for_match(&item.quote);
        if quote.len() >= 24 && (normalized.contains(&quote) || quote.contains(&normalized)) {
            let id = format!("{paragraph_id}:evidence-use:{}", evidence_uses.len() + 1);
            evidence_uses.push(EvidenceUse {
                id: id.clone(),
                evidence_use_id: id,
                matter_id: matter.matter_id.clone(),
                complaint_id: complaint_id.to_string(),
                target_type: "paragraph".to_string(),
                target_id: paragraph_id.to_string(),
                fact_id: fact_ids.first().cloned(),
                evidence_id: Some(item.evidence_id.clone()),
                document_id: Some(item.document_id.clone()),
                source_span_id: item
                    .source_spans
                    .first()
                    .map(|span| span.source_span_id.clone()),
                relation: "supports".to_string(),
                quote: Some(item.quote.clone()),
                status: "exact_match_needs_review".to_string(),
            });
        }
    }
    (fact_ids, evidence_uses)
}

fn normalize_for_match(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn citation_uses_for_text(
    matter_id: &str,
    complaint_id: &str,
    target_id: &str,
    target_type: &str,
    text: &str,
) -> Vec<CitationUse> {
    let mut citations = Vec::new();
    for citation in ORS_CITATION_RE
        .find_iter(text)
        .chain(ORCP_CITATION_RE.find_iter(text))
        .chain(UTCR_CITATION_RE.find_iter(text))
        .chain(SESSION_LAW_CITATION_RE.find_iter(text))
        .map(|m| m.as_str().trim().trim_end_matches('.').to_string())
    {
        if citations
            .iter()
            .any(|existing: &CitationUse| existing.citation.eq_ignore_ascii_case(&citation))
        {
            continue;
        }
        let canonical_id = canonical_id_for_citation(&citation);
        let is_external = is_external_authority_citation(&citation);
        let currentness = authority_currentness_for_citation(&citation);
        let scope_warning = if is_external {
            Some("External rule or session-law authority is source-backed but not yet part of the full ORSGraph provision corpus.".to_string())
        } else {
            None
        };
        let id = format!("{target_id}:citation-use:{}", citations.len() + 1);
        citations.push(CitationUse {
            id: id.clone(),
            citation_use_id: id,
            matter_id: matter_id.to_string(),
            complaint_id: complaint_id.to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            citation,
            pinpoint: None,
            quote: None,
            status: if canonical_id.is_some() {
                if is_external {
                    "resolved_external".to_string()
                } else {
                    "resolved".to_string()
                }
            } else {
                "unresolved".to_string()
            },
            currentness,
            scope_warning,
            canonical_id,
        });
    }
    citations
}

fn canonical_id_for_citation(citation: &str) -> Option<String> {
    let normalized = citation.split_whitespace().collect::<Vec<_>>().join(" ");
    let upper = normalized.to_ascii_uppercase();
    if upper.starts_with("ORS CHAPTER") || upper.starts_with("ORS CHAPTERS") {
        let chapter = normalized
            .split_whitespace()
            .find(|part| part.chars().any(|ch| ch.is_ascii_digit()))?
            .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        return Some(format!("or:ors:chapter:{chapter}"));
    }
    if upper.starts_with("ORS ") {
        let section = normalized
            .split_whitespace()
            .nth(1)?
            .split('(')
            .next()
            .unwrap_or_default()
            .trim_end_matches(',');
        if section.contains('.') {
            return Some(format!("or:ors:{section}"));
        }
    }
    if upper.starts_with("ORCP ") {
        return Some(format!(
            "or:orcp:{}",
            sanitize_path_segment(&normalized[5..].trim().to_ascii_lowercase())
        ));
    }
    if upper.starts_with("UTCR ") {
        let rule = normalized[5..]
            .trim()
            .split('(')
            .next()
            .unwrap_or_default()
            .trim_end_matches(',');
        return Some(format!(
            "or:utcr:{}",
            sanitize_path_segment(&rule.to_ascii_lowercase())
        ));
    }
    if let Some(caps) = SESSION_LAW_CITATION_RE.captures(&normalized) {
        let year = caps.get(1)?.as_str();
        let chapter = caps.get(2)?.as_str();
        let mut canonical = format!("or:session-law:{year}:ch:{chapter}");
        if let Some(section) = caps.get(3) {
            canonical.push_str(":sec:");
            canonical.push_str(&sanitize_path_segment(
                &section.as_str().trim().to_ascii_lowercase(),
            ));
        }
        return Some(canonical);
    }
    None
}

fn is_external_authority_citation(citation: &str) -> bool {
    let upper = citation.to_ascii_uppercase();
    upper.starts_with("ORCP")
        || upper.starts_with("OR LAWS")
        || upper.starts_with("OR. LAWS")
        || upper.starts_with("OREGON LAWS")
        || upper.starts_with("OREGON LAW")
}

fn authority_type_for_citation(citation: &str) -> &'static str {
    let upper = citation.to_ascii_uppercase();
    if upper.starts_with("ORS ") {
        "ors"
    } else if upper.starts_with("ORCP") {
        "orcp"
    } else if upper.starts_with("UTCR") {
        "utcr"
    } else if is_external_authority_citation(citation) {
        "session_law"
    } else {
        "unknown"
    }
}

fn authority_source_url_for_citation(citation: &str) -> &'static str {
    match authority_type_for_citation(citation) {
        "ors" => ORS_2025_SOURCE_URL,
        "orcp" => ORCP_2025_SOURCE_URL,
        "utcr" => UTCR_CURRENT_SOURCE_URL,
        "session_law" => ORS_2025_SOURCE_URL,
        _ => ORS_2025_SOURCE_URL,
    }
}

fn authority_edition_for_citation(citation: &str) -> &'static str {
    match authority_type_for_citation(citation) {
        "ors" => "2025 ORS",
        "orcp" => "2025 ORCP",
        "utcr" => "Current UTCR",
        "session_law" => "Oregon session law source-backed",
        _ => "Source-backed authority",
    }
}

fn authority_currentness_for_citation(citation: &str) -> String {
    match authority_type_for_citation(citation) {
        "ors" => "2025_ors_needs_review",
        "orcp" => "2025_orcp_needs_review",
        "utcr" => "current_utcr_needs_review",
        "session_law" => "source_backed_needs_review",
        _ => "source_backed_needs_review",
    }
    .to_string()
}

fn exhibit_references_for_text(
    matter_id: &str,
    complaint_id: &str,
    paragraph_id: &str,
    text: &str,
) -> Vec<ExhibitReference> {
    let mut out = Vec::new();
    for caps in EXHIBIT_LABEL_RE.captures_iter(text) {
        let Some(label) = caps.get(0).map(|m| m.as_str().trim().to_string()) else {
            continue;
        };
        if out
            .iter()
            .any(|existing: &ExhibitReference| existing.exhibit_label.eq_ignore_ascii_case(&label))
        {
            continue;
        }
        let id = format!("{paragraph_id}:exhibit-reference:{}", out.len() + 1);
        out.push(ExhibitReference {
            id: id.clone(),
            exhibit_reference_id: id,
            matter_id: matter_id.to_string(),
            complaint_id: complaint_id.to_string(),
            target_type: "paragraph".to_string(),
            target_id: paragraph_id.to_string(),
            exhibit_label: label,
            document_id: None,
            evidence_id: None,
            status: "missing".to_string(),
        });
    }
    out
}

fn imported_relief_requests(
    matter: &MatterSummary,
    complaint_id: &str,
    paragraphs: &[PleadingParagraph],
) -> Vec<ReliefRequest> {
    let mut relief = paragraphs
        .iter()
        .filter(|paragraph| paragraph.role == "relief")
        .take(12)
        .enumerate()
        .map(|(index, paragraph)| ReliefRequest {
            id: format!("{complaint_id}:relief:{}", index + 1),
            relief_id: format!("{complaint_id}:relief:{}", index + 1),
            matter_id: matter.matter_id.clone(),
            complaint_id: complaint_id.to_string(),
            category: "imported".to_string(),
            text: paragraph.text.clone(),
            amount: None,
            authority_ids: paragraph
                .citation_uses
                .iter()
                .filter_map(|citation| citation.canonical_id.clone())
                .collect(),
            supported: false,
        })
        .collect::<Vec<_>>();
    if relief.is_empty() {
        relief.push(ReliefRequest {
            id: format!("{complaint_id}:relief:general"),
            relief_id: format!("{complaint_id}:relief:general"),
            matter_id: matter.matter_id.clone(),
            complaint_id: complaint_id.to_string(),
            category: "general".to_string(),
            text: "Imported complaint contains no clearly parsed prayer paragraph; relief requires human review.".to_string(),
            amount: None,
            authority_ids: Vec::new(),
            supported: false,
        });
    }
    relief
}

fn match_claim_for_count(title: &str, claims: &[CaseClaim]) -> Option<String> {
    let normalized = normalize_for_match(title);
    claims
        .iter()
        .find(|claim| {
            let claim_text =
                normalize_for_match(&format!("{} {}", claim.title, claim.legal_theory));
            !claim_text.is_empty()
                && (normalized.contains(&claim_text) || claim_text.contains(&normalized))
        })
        .map(|claim| claim.claim_id.clone())
}

fn complaint_import_node_ids(complaint: &ComplaintDraft, spans: &[SourceSpan]) -> Vec<String> {
    let mut ids = vec![complaint.complaint_id.clone()];
    ids.extend(
        complaint
            .sections
            .iter()
            .map(|section| section.section_id.clone()),
    );
    ids.extend(complaint.counts.iter().map(|count| count.count_id.clone()));
    ids.extend(
        complaint
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.paragraph_id.clone()),
    );
    ids.extend(spans.iter().map(|span| span.source_span_id.clone()));
    ids
}

fn complaint_section(
    matter_id: &str,
    complaint_id: &str,
    section_type: &str,
    title: &str,
    ordinal: u64,
) -> ComplaintSection {
    let section_id = format!("{complaint_id}:section:{section_type}");
    ComplaintSection {
        id: section_id.clone(),
        section_id,
        matter_id: matter_id.to_string(),
        complaint_id: complaint_id.to_string(),
        title: title.to_string(),
        section_type: section_type.to_string(),
        ordinal,
        paragraph_ids: Vec::new(),
        count_ids: Vec::new(),
        review_status: "needs_review".to_string(),
    }
}

fn pleading_paragraph(
    matter_id: &str,
    complaint_id: &str,
    paragraph_id: &str,
    section_id: Option<String>,
    count_id: Option<String>,
    role: &str,
    text: &str,
    ordinal: u64,
    fact_ids: Vec<String>,
    evidence_ids: Vec<String>,
) -> PleadingParagraph {
    let evidence_uses = evidence_ids
        .iter()
        .enumerate()
        .map(|(index, evidence_id)| {
            let id = format!("{paragraph_id}:evidence-use:{}", index + 1);
            EvidenceUse {
                id: id.clone(),
                evidence_use_id: id,
                matter_id: matter_id.to_string(),
                complaint_id: complaint_id.to_string(),
                target_type: "paragraph".to_string(),
                target_id: paragraph_id.to_string(),
                fact_id: fact_ids.first().cloned(),
                evidence_id: Some(evidence_id.clone()),
                document_id: None,
                source_span_id: None,
                relation: "supports".to_string(),
                quote: None,
                status: "needs_review".to_string(),
            }
        })
        .collect::<Vec<_>>();
    PleadingParagraph {
        paragraph_id: paragraph_id.to_string(),
        id: paragraph_id.to_string(),
        matter_id: matter_id.to_string(),
        complaint_id: complaint_id.to_string(),
        section_id,
        count_id,
        number: ordinal,
        ordinal,
        display_number: Some(ordinal.to_string()),
        original_label: None,
        source_span_id: None,
        import_provenance: None,
        role: role.to_string(),
        text: text.to_string(),
        sentences: pleading_sentences(matter_id, complaint_id, paragraph_id, text, &fact_ids),
        fact_ids,
        evidence_uses,
        citation_uses: Vec::new(),
        exhibit_references: Vec::new(),
        rule_finding_ids: Vec::new(),
        locked: false,
        review_status: "needs_review".to_string(),
    }
}

fn pleading_sentences(
    matter_id: &str,
    complaint_id: &str,
    paragraph_id: &str,
    text: &str,
    fact_ids: &[String],
) -> Vec<PleadingSentence> {
    let parts = text
        .split_terminator(['.', '?', '!'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let sentences = if parts.is_empty() {
        vec![text.trim()]
    } else {
        parts
    };
    sentences
        .into_iter()
        .enumerate()
        .map(|(index, sentence)| {
            let id = format!("{paragraph_id}:sentence:{}", index + 1);
            PleadingSentence {
                sentence_id: id.clone(),
                id,
                matter_id: matter_id.to_string(),
                complaint_id: complaint_id.to_string(),
                paragraph_id: paragraph_id.to_string(),
                ordinal: index as u64 + 1,
                text: sentence.to_string(),
                fact_ids: fact_ids.to_vec(),
                evidence_use_ids: Vec::new(),
                citation_use_ids: Vec::new(),
                review_status: "needs_review".to_string(),
            }
        })
        .collect()
}

fn refresh_complaint_state(complaint: &mut ComplaintDraft) {
    renumber_paragraphs(&mut complaint.paragraphs);
    for section in &mut complaint.sections {
        section.paragraph_ids = complaint
            .paragraphs
            .iter()
            .filter(|paragraph| {
                paragraph.section_id.as_deref() == Some(section.section_id.as_str())
            })
            .map(|paragraph| paragraph.paragraph_id.clone())
            .collect();
        section.count_ids = complaint
            .counts
            .iter()
            .filter(|count| {
                count.paragraph_ids.iter().any(|paragraph_id| {
                    section
                        .paragraph_ids
                        .iter()
                        .any(|section_paragraph_id| section_paragraph_id == paragraph_id)
                })
            })
            .map(|count| count.count_id.clone())
            .collect();
    }
    for count in &mut complaint.counts {
        let mut weaknesses = Vec::new();
        if count.fact_ids.is_empty() {
            weaknesses.push("missing fact support".to_string());
        }
        if count.evidence_ids.is_empty() {
            weaknesses.push("missing evidence support".to_string());
        }
        if count.authorities.is_empty() {
            weaknesses.push("missing authority".to_string());
        }
        if count.relief_ids.is_empty() {
            weaknesses.push("relief not mapped".to_string());
        }
        count.health = if weaknesses.is_empty() {
            "supported_needs_review".to_string()
        } else {
            "needs_work".to_string()
        };
        count.weaknesses = weaknesses;
    }
    let open_findings = complaint
        .findings
        .iter()
        .filter(|finding| finding.status == "open")
        .cloned()
        .collect::<Vec<_>>();
    complaint.next_actions = derive_next_actions(complaint, &open_findings);
    complaint.filing_packet = derive_filing_packet(complaint);
}

fn renumber_paragraphs(paragraphs: &mut [PleadingParagraph]) {
    paragraphs.sort_by_key(|paragraph| paragraph.ordinal);
    for (index, paragraph) in paragraphs.iter_mut().enumerate() {
        paragraph.number = index as u64 + 1;
        paragraph.ordinal = index as u64 + 1;
        if paragraph.original_label.is_none() {
            paragraph.display_number = Some(paragraph.number.to_string());
        }
    }
}

fn complaint_rule_findings(complaint: &ComplaintDraft) -> Vec<RuleCheckFinding> {
    let now = now_string();
    let mut findings = Vec::new();
    let mut add = |rule_id: &str,
                   category: &str,
                   severity: &str,
                   target_type: &str,
                   target_id: &str,
                   message: &str,
                   explanation: &str,
                   suggested_fix: &str,
                   action_label: &str,
                   action_type: &str| {
        findings.push(RuleCheckFinding {
            finding_id: format!(
                "finding:{}:{}:{}",
                sanitize_path_segment(&complaint.complaint_id),
                sanitize_path_segment(rule_id),
                sanitize_path_segment(target_id)
            ),
            id: format!(
                "finding:{}:{}:{}",
                sanitize_path_segment(&complaint.complaint_id),
                sanitize_path_segment(rule_id),
                sanitize_path_segment(target_id)
            ),
            matter_id: complaint.matter_id.clone(),
            complaint_id: complaint.complaint_id.clone(),
            rule_id: rule_id.to_string(),
            category: category.to_string(),
            severity: severity.to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            message: message.to_string(),
            explanation: explanation.to_string(),
            suggested_fix: suggested_fix.to_string(),
            primary_action: ComplaintAction {
                action_id: format!("action:{rule_id}:{}", sanitize_path_segment(target_id)),
                label: action_label.to_string(),
                action_type: action_type.to_string(),
                href: None,
                target_type: target_type.to_string(),
                target_id: target_id.to_string(),
            },
            status: "open".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    };

    if complaint.caption.court_name.trim().is_empty()
        || complaint.caption.court_name == "Unassigned"
    {
        add(
            "orcp-16-caption-court",
            "rules",
            "blocking",
            "caption",
            &complaint.complaint_id,
            "Caption is missing the court name.",
            "ORCP 16 requires a caption setting forth the name of the court.",
            "Add the court name to the caption.",
            "Complete caption",
            "edit_caption",
        );
    }
    if complaint.caption.plaintiff_names.is_empty() || complaint.caption.defendant_names.is_empty()
    {
        add(
            "orcp-16-complaint-title-parties",
            "rules",
            "blocking",
            "caption",
            &complaint.complaint_id,
            "Complaint title must identify all parties.",
            "ORCP 16 requires the complaint title to include the names of all parties.",
            "Confirm plaintiff and defendant names from the matter parties.",
            "Review parties",
            "edit_parties",
        );
    }
    if complaint.counts.is_empty() {
        add(
            "orcp-16-separate-counts",
            "structure",
            "blocking",
            "complaint",
            &complaint.complaint_id,
            "No separately stated count exists.",
            "ORCP 16 requires separate claims or defenses to be separately stated.",
            "Create at least one count linked to a claim or custom legal theory.",
            "Create count",
            "create_count",
        );
    }
    if complaint.paragraphs.is_empty() {
        add(
            "orcp-16-numbered-paragraphs",
            "structure",
            "blocking",
            "complaint",
            &complaint.complaint_id,
            "No numbered pleading paragraphs exist.",
            "ORCP 16 requires plain and concise statements in consecutively numbered paragraphs.",
            "Add numbered pleading paragraphs.",
            "Add paragraph",
            "create_paragraph",
        );
    }
    for (index, paragraph) in complaint.paragraphs.iter().enumerate() {
        if paragraph.number != index as u64 + 1 {
            add(
                "orcp-16-consecutive-numbering",
                "structure",
                "serious",
                "paragraph",
                &paragraph.paragraph_id,
                "Paragraph numbering is not consecutive.",
                "ORCP 16 expects paragraphs to be consecutively numbered throughout the pleading.",
                "Run renumbering to restore consecutive Arabic numerals.",
                "Renumber paragraphs",
                "renumber",
            );
        }
        if paragraph.role == "factual_allegation"
            && paragraph.fact_ids.is_empty()
            && paragraph.evidence_uses.is_empty()
        {
            add(
                "cb-support-factual-allegation",
                "evidence",
                "warning",
                "paragraph",
                &paragraph.paragraph_id,
                "Factual allegation has no linked fact or evidence.",
                "Complaint Editor flags unsupported factual allegations for human review.",
                "Link a fact, evidence record, document span, or mark the paragraph reviewed.",
                "Link evidence",
                "link_evidence",
            );
        }
        if paragraph.role == "factual_allegation" && paragraph.text.split_whitespace().count() > 90
        {
            add(
                "orcp-18-plain-concise-ultimate-facts",
                "rules",
                "warning",
                "paragraph",
                &paragraph.paragraph_id,
                "Factual paragraph may be too long for plain and concise pleading.",
                "ORCP 18 calls for a plain and concise statement of ultimate facts without unnecessary repetition.",
                "Split or tighten the paragraph after human review.",
                "Split paragraph",
                "split_paragraph",
            );
        }
        for citation in &paragraph.citation_uses {
            if citation.status != "resolved" {
                add(
                    "cb-citation-unresolved",
                    "citations",
                    "warning",
                    "citation",
                    &citation.citation_use_id,
                    "Citation is unresolved or needs review.",
                    "Citation uses should be linked to ORSGraph authority when possible and reviewed for currentness.",
                    "Resolve the citation against ORSGraph authority.",
                    "Resolve citation",
                    "resolve_citation",
                );
            }
        }
        for exhibit in &paragraph.exhibit_references {
            if exhibit.status == "missing" {
                add(
                    "cb-exhibit-missing",
                    "exhibits",
                    "warning",
                    "exhibit",
                    &exhibit.exhibit_reference_id,
                    "Exhibit reference is not linked to an exhibit document or evidence record.",
                    "Exhibit labels should remain stable and link to the supporting record.",
                    "Attach the referenced exhibit.",
                    "Attach exhibit",
                    "attach_exhibit",
                );
            }
        }
    }
    if complaint.relief.is_empty()
        || complaint
            .relief
            .iter()
            .all(|relief| relief.text.trim().is_empty())
    {
        add(
            "orcp-18-demand-relief",
            "relief",
            "blocking",
            "relief",
            &complaint.complaint_id,
            "Demand for relief is missing.",
            "ORCP 18 requires a demand of the relief claimed, including amounts when money or damages are demanded.",
            "Add requested relief and any amount after human review.",
            "Add relief",
            "edit_relief",
        );
    }
    if complaint.signature.name.trim().is_empty()
        || complaint.signature.address.trim().is_empty()
        || complaint.signature.email.trim().is_empty()
    {
        add(
            "orcp-17-signature-contact",
            "rules",
            "serious",
            "signature",
            &complaint.complaint_id,
            "Signature/contact block is incomplete.",
            "Pleadings require a human-reviewed signature and contact block before filing or service.",
            "Complete the signature block.",
            "Complete signature",
            "edit_signature",
        );
    }
    if !complaint.formatting_profile.double_spaced {
        add(
            "utcr-2-010-double-spacing",
            "formatting",
            "serious",
            "formatting",
            &complaint.formatting_profile.profile_id,
            "Formatting profile is not double-spaced.",
            "UTCR 2.010 provides that pleadings must be double-spaced unless another rule applies.",
            "Set the complaint formatting profile to double-spaced.",
            "Fix formatting",
            "edit_formatting",
        );
    }
    if !complaint.formatting_profile.line_numbers {
        add(
            "utcr-2-010-numbered-lines",
            "formatting",
            "serious",
            "formatting",
            &complaint.formatting_profile.profile_id,
            "Formatting profile does not include numbered lines.",
            "UTCR 2.010 provides that pleadings must be prepared with numbered lines unless another rule applies.",
            "Enable numbered lines.",
            "Fix formatting",
            "edit_formatting",
        );
    }
    if complaint.formatting_profile.first_page_top_blank_inches < 2.0 {
        add(
            "utcr-2-010-first-page-blank",
            "formatting",
            "serious",
            "formatting",
            &complaint.formatting_profile.profile_id,
            "First-page top blank area is less than two inches.",
            "UTCR 2.010 calls for two inches blank at the top of the first page of pleadings or similar documents.",
            "Set the first-page top blank area to two inches.",
            "Fix formatting",
            "edit_formatting",
        );
    }
    if complaint.formatting_profile.margin_left_inches < 1.0
        || complaint.formatting_profile.margin_right_inches < 1.0
    {
        add(
            "utcr-2-010-side-margins",
            "formatting",
            "serious",
            "formatting",
            &complaint.formatting_profile.profile_id,
            "Side margins are less than one inch.",
            "UTCR 2.010 calls for one-inch side margins except where a different form applies.",
            "Set side margins to at least one inch.",
            "Fix formatting",
            "edit_formatting",
        );
    }
    findings
}

fn derive_next_actions(
    complaint: &ComplaintDraft,
    open_findings: &[RuleCheckFinding],
) -> Vec<ComplaintNextAction> {
    let mut actions = Vec::new();
    for finding in open_findings.iter().take(5) {
        actions.push(ComplaintNextAction {
            action_id: format!("next:{}", finding.finding_id),
            priority: finding.severity.clone(),
            label: finding.primary_action.label.clone(),
            detail: finding.message.clone(),
            action_type: finding.primary_action.action_type.clone(),
            target_type: finding.target_type.clone(),
            target_id: finding.target_id.clone(),
            href: finding.primary_action.href.clone(),
        });
    }
    if actions.is_empty() {
        if complaint.paragraphs.iter().any(|paragraph| {
            paragraph.role == "factual_allegation"
                && paragraph.fact_ids.is_empty()
                && paragraph.evidence_uses.is_empty()
        }) {
            actions.push(ComplaintNextAction {
                action_id: "next:link-evidence".to_string(),
                priority: "warning".to_string(),
                label: "Link evidence".to_string(),
                detail: "Some factual allegations still need support links.".to_string(),
                action_type: "link_evidence".to_string(),
                target_type: "complaint".to_string(),
                target_id: complaint.complaint_id.clone(),
                href: None,
            });
        } else {
            actions.push(ComplaintNextAction {
                action_id: "next:run-qc".to_string(),
                priority: "info".to_string(),
                label: "Run QC".to_string(),
                detail: "Run deterministic complaint QC before preview or export.".to_string(),
                action_type: "run_qc".to_string(),
                target_type: "complaint".to_string(),
                target_id: complaint.complaint_id.clone(),
                href: None,
            });
        }
    }
    actions
}

fn derive_filing_packet(complaint: &ComplaintDraft) -> FilingPacket {
    let mut items = Vec::new();
    items.push(FilingPacketItem {
        item_id: format!("{}:packet:item:complaint", complaint.complaint_id),
        label: "Complaint".to_string(),
        item_type: "complaint_pdf".to_string(),
        status: if complaint
            .export_artifacts
            .iter()
            .any(|artifact| artifact.format == "pdf")
        {
            "generated_review_needed".to_string()
        } else {
            "missing".to_string()
        },
        artifact_id: complaint
            .export_artifacts
            .iter()
            .find(|artifact| artifact.format == "pdf")
            .map(|artifact| artifact.artifact_id.clone()),
        warning: Some("Review before filing; no e-filing is performed.".to_string()),
    });
    items.push(FilingPacketItem {
        item_id: format!("{}:packet:item:certificate", complaint.complaint_id),
        label: "Certificate of service".to_string(),
        item_type: "certificate".to_string(),
        status: complaint
            .certificate_of_service
            .as_ref()
            .map(|certificate| certificate.review_status.clone())
            .unwrap_or_else(|| "missing".to_string()),
        artifact_id: None,
        warning: Some("Service details require human review.".to_string()),
    });
    for exhibit in complaint
        .paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.exhibit_references.iter())
    {
        items.push(FilingPacketItem {
            item_id: format!(
                "{}:packet:item:{}",
                complaint.complaint_id, exhibit.exhibit_label
            ),
            label: format!("Exhibit {}", exhibit.exhibit_label),
            item_type: "exhibit".to_string(),
            status: exhibit.status.clone(),
            artifact_id: None,
            warning: if exhibit.status == "missing" {
                Some("Referenced exhibit is not attached.".to_string())
            } else {
                None
            },
        });
    }
    let warnings = items
        .iter()
        .filter_map(|item| item.warning.clone())
        .collect::<Vec<_>>();
    FilingPacket {
        packet_id: format!("{}:packet:filing", complaint.complaint_id),
        matter_id: complaint.matter_id.clone(),
        complaint_id: complaint.complaint_id.clone(),
        status: "review_needed".to_string(),
        items,
        warnings,
    }
}

fn render_complaint_preview(complaint: &ComplaintDraft) -> ComplaintPreviewResponse {
    let mut html = String::new();
    html.push_str("<article class=\"court-paper review-needed\">");
    html.push_str("<header class=\"caption\">");
    html.push_str(&format!(
        "<p>{}</p>",
        escape_html(&complaint.caption.court_name)
    ));
    html.push_str(&format!(
        "<h1>{}</h1>",
        escape_html(&complaint.caption.document_title)
    ));
    html.push_str(&format!(
        "<p>{} v. {}</p>",
        escape_html(&complaint.caption.plaintiff_names.join(", ")),
        escape_html(&complaint.caption.defendant_names.join(", "))
    ));
    html.push_str("</header>");
    for paragraph in &complaint.paragraphs {
        html.push_str(&format!(
            "<p id=\"{}\"><span class=\"line-number\">{}</span> {}</p>",
            escape_html(&paragraph.paragraph_id),
            paragraph.number,
            escape_html(&paragraph.text)
        ));
    }
    html.push_str("<section class=\"signature\">");
    html.push_str(&format!(
        "<p>{}</p><p>{}</p><p>{}</p>",
        escape_html(&complaint.signature.name),
        escape_html(&complaint.signature.address),
        escape_html(&complaint.signature.email)
    ));
    html.push_str("</section>");
    html.push_str("</article>");
    let plain_text = complaint_plain_text(complaint);
    let page_count = (plain_text.lines().count() as u64 / 28).max(1);
    ComplaintPreviewResponse {
        complaint_id: complaint.complaint_id.clone(),
        matter_id: complaint.matter_id.clone(),
        html,
        plain_text,
        page_count,
        warnings: export_warnings(complaint, "preview"),
        generated_at: now_string(),
        review_label: "Review needed; not legal advice or filing-ready status.".to_string(),
    }
}

fn export_complaint_content(
    complaint: &ComplaintDraft,
    format: &str,
    include_exhibits: bool,
    include_qc_report: bool,
) -> ApiResult<String> {
    let mut text = match format {
        "html" => render_complaint_preview(complaint).html,
        "json" => to_payload(complaint)?,
        "markdown" => complaint_markdown(complaint),
        "text" | "plain_text" => complaint_plain_text(complaint),
        "pdf" => format!(
            "PDF skeleton for human review\n\n{}",
            complaint_plain_text(complaint)
        ),
        "docx" => format!(
            "DOCX skeleton for human review\n\n{}",
            complaint_plain_text(complaint)
        ),
        _ => complaint_plain_text(complaint),
    };
    if include_exhibits {
        let exhibits = complaint
            .paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.exhibit_references.iter())
            .map(|exhibit| format!("Exhibit {} - {}", exhibit.exhibit_label, exhibit.status))
            .collect::<Vec<_>>();
        if !exhibits.is_empty() {
            text.push_str("\n\nEXHIBITS\n");
            text.push_str(&exhibits.join("\n"));
        }
    }
    if include_qc_report {
        text.push_str("\n\nQC REVIEW ITEMS\n");
        if complaint.findings.is_empty() {
            text.push_str("No persisted findings. Run complaint QC before relying on this export.");
        } else {
            for finding in &complaint.findings {
                text.push_str(&format!(
                    "\n- [{}] {}: {}",
                    finding.severity, finding.status, finding.message
                ));
            }
        }
    }
    Ok(text)
}

fn complaint_plain_text(complaint: &ComplaintDraft) -> String {
    let mut lines = Vec::new();
    lines.push(complaint.caption.court_name.clone());
    lines.push(format!(
        "{} v. {}",
        complaint.caption.plaintiff_names.join(", "),
        complaint.caption.defendant_names.join(", ")
    ));
    lines.push(complaint.caption.document_title.clone());
    lines.push(String::new());
    for paragraph in &complaint.paragraphs {
        lines.push(format!("{}. {}", paragraph.number, paragraph.text));
    }
    lines.push(String::new());
    lines.push("PRAYER FOR RELIEF".to_string());
    for relief in &complaint.relief {
        lines.push(format!("- {}", relief.text));
    }
    lines.push(String::new());
    lines.push("Review needed; not legal advice or filing-ready status.".to_string());
    lines.join("\n")
}

fn complaint_markdown(complaint: &ComplaintDraft) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# {}", complaint.caption.document_title));
    lines.push(format!(
        "**{} v. {}**",
        complaint.caption.plaintiff_names.join(", "),
        complaint.caption.defendant_names.join(", ")
    ));
    for section in &complaint.sections {
        lines.push(format!("\n## {}", section.title));
        for paragraph in complaint.paragraphs.iter().filter(|paragraph| {
            paragraph.section_id.as_deref() == Some(section.section_id.as_str())
        }) {
            lines.push(format!("{}. {}", paragraph.number, paragraph.text));
        }
    }
    lines.push("\n> Review needed; not legal advice or filing-ready status.".to_string());
    lines.join("\n")
}

fn export_warnings(complaint: &ComplaintDraft, format: &str) -> Vec<String> {
    let mut warnings =
        vec!["Review needed; generated checks and exports are not legal advice.".to_string()];
    if complaint
        .findings
        .iter()
        .any(|finding| finding.status == "open")
    {
        warnings.push("Open QC findings remain.".to_string());
    }
    if matches!(format, "pdf" | "docx") {
        warnings.push(
            "PDF/DOCX output is a deterministic skeleton until the dedicated renderer is enabled."
                .to_string(),
        );
    }
    warnings
}

fn export_mime_type(format: &str) -> &'static str {
    match format {
        "html" => "text/html",
        "json" => "application/json",
        "markdown" => "text/markdown",
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => "text/plain",
    }
}

fn default_formatting_profile() -> FormattingProfile {
    FormattingProfile {
        profile_id: "oregon-circuit-civil-complaint".to_string(),
        name: "Oregon Circuit Civil Complaint".to_string(),
        jurisdiction: "Oregon".to_string(),
        line_numbers: true,
        double_spaced: true,
        first_page_top_blank_inches: 2.0,
        margin_top_inches: 1.0,
        margin_bottom_inches: 1.0,
        margin_left_inches: 1.0,
        margin_right_inches: 1.0,
        font_family: "Times New Roman".to_string(),
        font_size_pt: 12,
    }
}

fn oregon_civil_complaint_rule_pack() -> RulePack {
    RulePack {
        rule_pack_id: "oregon-circuit-civil-complaint-orcp-utcr".to_string(),
        name: "Oregon Circuit Civil Complaint - ORCP + UTCR".to_string(),
        jurisdiction: "Oregon".to_string(),
        version: "provider-free-seed-2026-05-01".to_string(),
        effective_date: "2024-08-01".to_string(),
        rule_profile: default_rule_profile_summary(),
        rules: vec![
            rule_definition("orcp-16-caption-court", "ORCP 16 A", "https://oregon.public.law/rules-of-civil-procedure/orcp-16-form-of-pleadings/", "blocking", "caption", "rules", "Caption must include court name.", "ORCP 16 describes caption requirements.", "Complete the court name.", false),
            rule_definition("orcp-16-complaint-title-parties", "ORCP 16 A", "https://oregon.public.law/rules-of-civil-procedure/orcp-16-form-of-pleadings/", "blocking", "caption", "rules", "Complaint title must include all parties.", "ORCP 16 distinguishes complaint title requirements from later pleadings.", "Confirm all party names.", false),
            rule_definition("orcp-16-numbered-paragraphs", "ORCP 16 C", "https://oregon.public.law/rules-of-civil-procedure/orcp-16-form-of-pleadings/", "blocking", "paragraph", "structure", "Paragraphs must be consecutively numbered.", "ORCP 16 calls for consecutively numbered paragraphs.", "Run renumbering.", true),
            rule_definition("orcp-16-separate-counts", "ORCP 16 C", "https://oregon.public.law/rules-of-civil-procedure/orcp-16-form-of-pleadings/", "blocking", "count", "structure", "Separate claims must be separately stated.", "ORCP 16 requires separate claims or defenses to be separately stated.", "Create separate counts.", false),
            rule_definition("orcp-18-plain-concise-ultimate-facts", "ORCP 18 A", "https://oregon.public.law/rules-of-civil-procedure/orcp-18-claims-for-relief/", "warning", "paragraph", "rules", "Claims should plead plain and concise ultimate facts.", "ORCP 18 calls for a plain and concise statement of ultimate facts.", "Tighten or split long factual allegations.", false),
            rule_definition("orcp-18-demand-relief", "ORCP 18 B", "https://oregon.public.law/rules-of-civil-procedure/orcp-18-claims-for-relief/", "blocking", "relief", "relief", "Demand for relief is required.", "ORCP 18 requires a demand for relief and amount when money or damages are demanded.", "Add requested relief.", false),
            rule_definition("orcp-17-signature-contact", "ORCP 17", "https://oregon.public.law/rules-of-civil-procedure/orcp-17-signing-of-pleadings-motions-and-other-papers-sanctions/", "serious", "signature", "rules", "Signature and contact block require review.", "Pleadings must be signed by a responsible person subject to Rule 17 obligations.", "Complete signature details.", false),
            rule_definition("utcr-2-010-double-spacing", "UTCR 2.010(4)(a)", "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf", "serious", "formatting", "formatting", "Pleadings should be double-spaced.", "UTCR 2.010 includes spacing standards for pleadings.", "Enable double spacing.", true),
            rule_definition("utcr-2-010-numbered-lines", "UTCR 2.010(4)(a)", "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf", "serious", "formatting", "formatting", "Pleadings should have numbered lines.", "UTCR 2.010 includes numbered-line standards for pleadings.", "Enable numbered lines.", true),
            rule_definition("utcr-2-010-first-page-blank", "UTCR 2.010(4)(c)", "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf", "serious", "formatting", "formatting", "First page top blank area should be two inches.", "UTCR 2.010 includes a first-page blank area standard.", "Set first-page blank area to two inches.", true),
            rule_definition("utcr-2-010-side-margins", "UTCR 2.010(4)(d)", "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf", "serious", "formatting", "formatting", "Side margins should be at least one inch.", "UTCR 2.010 includes side-margin standards.", "Set one-inch side margins.", true),
        ],
    }
}

fn rule_definition(
    rule_id: &str,
    source_citation: &str,
    source_url: &str,
    severity: &str,
    target_type: &str,
    category: &str,
    message: &str,
    explanation: &str,
    suggested_fix: &str,
    auto_fix_available: bool,
) -> RuleDefinition {
    RuleDefinition {
        rule_id: rule_id.to_string(),
        source_citation: source_citation.to_string(),
        source_url: source_url.to_string(),
        severity: severity.to_string(),
        target_type: target_type.to_string(),
        category: category.to_string(),
        message: message.to_string(),
        explanation: explanation.to_string(),
        suggested_fix: suggested_fix.to_string(),
        auto_fix_available,
    }
}

fn default_ai_commands() -> Vec<ComplaintAiCommandState> {
    [
        ("draft_factual_background", "Draft factual background"),
        ("draft_count", "Draft count"),
        ("rewrite_ultimate_facts", "Rewrite as ultimate facts"),
        ("split_paragraph", "Split paragraph"),
        ("make_concise", "Make concise"),
        ("find_missing_evidence", "Find missing evidence"),
        ("find_missing_authority", "Find missing authority"),
        ("generate_prayer", "Generate prayer"),
        ("generate_exhibit_list", "Generate exhibit list"),
        ("generate_certificate", "Generate certificate"),
        ("fact_check", "Fact-check"),
        ("citation_check", "Citation-check"),
    ]
    .into_iter()
    .map(|(command_id, label)| ComplaintAiCommandState {
        command_id: command_id.to_string(),
        label: label.to_string(),
        status: "template_available".to_string(),
        mode: "provider_free".to_string(),
        description: "Provider-free command state; no unsupported text is inserted automatically."
            .to_string(),
        last_message: None,
    })
    .collect()
}

fn complaint_event(
    matter_id: &str,
    complaint_id: &str,
    event_type: &str,
    target_type: &str,
    target_id: &str,
    summary: &str,
) -> ComplaintHistoryEvent {
    let id = format!(
        "event:{}:{}:{}",
        sanitize_path_segment(complaint_id),
        sanitize_path_segment(event_type),
        now_secs()
    );
    ComplaintHistoryEvent {
        id: id.clone(),
        event_id: id,
        matter_id: matter_id.to_string(),
        complaint_id: complaint_id.to_string(),
        event_type: event_type.to_string(),
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        summary: summary.to_string(),
        timestamp: now_string(),
    }
}

fn json_value<T: serde::Serialize>(value: &T) -> ApiResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|error| ApiError::Internal(error.to_string()))
}

fn json_hash(value: &serde_json::Value) -> ApiResult<String> {
    let payload =
        serde_json::to_string(value).map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(sha256_hex(payload.as_bytes()))
}

fn hash_json<T: serde::Serialize>(value: &T) -> ApiResult<String> {
    json_hash(&json_value(value)?)
}

fn version_change_state_summary(
    value: Option<serde_json::Value>,
) -> ApiResult<(Option<String>, Option<serde_json::Value>)> {
    let Some(value) = value else {
        return Ok((None, None));
    };
    let state_hash = json_hash(&value)?;
    let size_bytes = serde_json::to_vec(&value)
        .map_err(|error| ApiError::Internal(error.to_string()))?
        .len() as u64;
    Ok((
        Some(state_hash.clone()),
        Some(serde_json::json!({
            "state_hash": state_hash,
            "size_bytes": size_bytes,
            "state_storage": "version_snapshot",
            "inline_payload": false,
        })),
    ))
}

fn work_product_hashes(product: &WorkProduct) -> ApiResult<WorkProductHashes> {
    let document_state = serde_json::json!({
        "title": product.title,
        "product_type": product.product_type,
        "profile": product.profile,
        "document_ast": product.document_ast,
    });
    let support_state = serde_json::json!({
        "blocks": flatten_work_product_blocks(&product.document_ast.blocks).iter().map(|block| serde_json::json!({
            "block_id": block.block_id,
            "links": block.links,
            "citations": block.citations,
            "exhibits": block.exhibits,
        })).collect::<Vec<_>>(),
        "links": product.document_ast.links,
        "citations": product.document_ast.citations,
        "exhibits": product.document_ast.exhibits,
    });
    let qc_state = serde_json::json!({
        "findings": product.document_ast.rule_findings,
        "review_status": product.review_status,
    });
    Ok(WorkProductHashes {
        document_hash: json_hash(&document_state)?,
        support_graph_hash: json_hash(&support_state)?,
        qc_state_hash: json_hash(&qc_state)?,
        formatting_hash: hash_json(&product.formatting_profile)?,
    })
}

fn snapshot_manifest_for_product(
    matter_id: &str,
    snapshot_id: &str,
    product: &WorkProduct,
    created_at: &str,
) -> ApiResult<(SnapshotManifest, Vec<SnapshotEntityState>)> {
    let manifest_id = format!("{snapshot_id}:manifest");
    let mut states = Vec::new();
    push_entity_state(
        &mut states,
        &manifest_id,
        snapshot_id,
        matter_id,
        &product.work_product_id,
        "work_product",
        &product.work_product_id,
        json_value(product)?,
    )?;
    push_entity_state(
        &mut states,
        &manifest_id,
        snapshot_id,
        matter_id,
        &product.work_product_id,
        "document_ast",
        &product.document_ast.document_id,
        json_value(&product.document_ast)?,
    )?;
    for block in flatten_work_product_blocks(&product.document_ast.blocks) {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "block",
            &block.block_id,
            json_value(&block)?,
        )?;
    }
    for link in &product.document_ast.links {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "support_use",
            &link.link_id,
            json_value(link)?,
        )?;
    }
    for citation in &product.document_ast.citations {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "citation",
            &citation.citation_use_id,
            json_value(citation)?,
        )?;
    }
    for exhibit in &product.document_ast.exhibits {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "exhibit_reference",
            &exhibit.exhibit_reference_id,
            json_value(exhibit)?,
        )?;
    }
    for finding in &product.document_ast.rule_findings {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "rule_finding",
            &finding.finding_id,
            json_value(finding)?,
        )?;
    }
    for artifact in &product.artifacts {
        push_entity_state(
            &mut states,
            &manifest_id,
            snapshot_id,
            matter_id,
            &product.work_product_id,
            "export_artifact",
            &artifact.artifact_id,
            json_value(artifact)?,
        )?;
    }
    let manifest_hash = snapshot_manifest_hash_for_states(&states)?;
    Ok((
        SnapshotManifest {
            id: manifest_id.clone(),
            manifest_id,
            snapshot_id: snapshot_id.to_string(),
            matter_id: matter_id.to_string(),
            subject_id: product.work_product_id.clone(),
            manifest_hash,
            entry_count: states.len() as u64,
            storage_ref: None,
            created_at: created_at.to_string(),
        },
        states,
    ))
}

fn snapshot_manifest_hash_for_states(states: &[SnapshotEntityState]) -> ApiResult<String> {
    hash_json(
        &states
            .iter()
            .map(|state| (&state.entity_type, &state.entity_id, &state.entity_hash))
            .collect::<Vec<_>>(),
    )
}

fn push_entity_state(
    states: &mut Vec<SnapshotEntityState>,
    manifest_id: &str,
    snapshot_id: &str,
    matter_id: &str,
    subject_id: &str,
    entity_type: &str,
    entity_id: &str,
    state: serde_json::Value,
) -> ApiResult<()> {
    let entity_hash = json_hash(&state)?;
    let entity_state_id = format!(
        "{snapshot_id}:state:{}:{}",
        sanitize_path_segment(entity_type),
        sanitize_path_segment(entity_id)
    );
    states.push(SnapshotEntityState {
        id: entity_state_id.clone(),
        entity_state_id,
        manifest_id: manifest_id.to_string(),
        snapshot_id: snapshot_id.to_string(),
        matter_id: matter_id.to_string(),
        subject_id: subject_id.to_string(),
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        entity_hash,
        state_ref: None,
        state_inline: Some(state),
    });
    Ok(())
}

fn diff_work_product_blocks(from: &WorkProduct, to: &WorkProduct) -> Vec<VersionTextDiff> {
    let mut diffs = Vec::new();
    let from_blocks = flatten_work_product_blocks(&from.document_ast.blocks);
    let to_blocks = flatten_work_product_blocks(&to.document_ast.blocks);
    for before in &from_blocks {
        match to_blocks
            .iter()
            .find(|block| block.block_id == before.block_id)
        {
            Some(after) => {
                let status = if before.text == after.text && before.title == after.title {
                    "unchanged"
                } else {
                    "modified"
                };
                diffs.push(VersionTextDiff {
                    target_type: "block".to_string(),
                    target_id: before.block_id.clone(),
                    title: after.title.clone(),
                    status: status.to_string(),
                    before: Some(before.text.clone()),
                    after: Some(after.text.clone()),
                });
            }
            None => diffs.push(VersionTextDiff {
                target_type: "block".to_string(),
                target_id: before.block_id.clone(),
                title: before.title.clone(),
                status: "removed".to_string(),
                before: Some(before.text.clone()),
                after: None,
            }),
        }
    }
    for after in &to_blocks {
        if !from_blocks
            .iter()
            .any(|block| block.block_id == after.block_id)
        {
            diffs.push(VersionTextDiff {
                target_type: "block".to_string(),
                target_id: after.block_id.clone(),
                title: after.title.clone(),
                status: "added".to_string(),
                before: None,
                after: Some(after.text.clone()),
            });
        }
    }
    diffs
}

fn normalize_compare_layers(layers: Vec<String>) -> Vec<String> {
    let defaults = [
        "text",
        "support",
        "citations",
        "exhibits",
        "rule_findings",
        "formatting",
        "exports",
    ];
    let requested = if layers.is_empty() {
        defaults.iter().map(|layer| layer.to_string()).collect()
    } else {
        layers
    };
    let mut normalized = Vec::new();
    for layer in requested {
        let layer = layer.trim().to_ascii_lowercase();
        let expanded = match layer.as_str() {
            "all" | "legal" | "ast" => defaults.to_vec(),
            "text" | "blocks" => vec!["text"],
            "support" | "links" | "support_links" => vec!["support"],
            "citation" | "citations" => vec!["citations"],
            "exhibit" | "exhibits" => vec!["exhibits"],
            "rule_finding" | "rule_findings" | "qc" | "findings" | "rules" => {
                vec!["rule_findings"]
            }
            "format" | "formatting" => vec!["formatting"],
            "export" | "exports" | "artifacts" => vec!["exports"],
            _ => Vec::new(),
        };
        for value in expanded {
            if !normalized.iter().any(|existing| existing == value) {
                normalized.push(value.to_string());
            }
        }
    }
    if normalized.is_empty() {
        normalized = defaults.iter().map(|layer| layer.to_string()).collect();
    }
    normalized
}

fn diff_work_product_layers(
    from: &WorkProduct,
    to: &WorkProduct,
    layers: &[String],
) -> ApiResult<Vec<VersionLayerDiff>> {
    let mut diffs = Vec::new();
    if layers.iter().any(|layer| layer == "support") {
        diffs.extend(diff_layer_items(
            support_layer_items(from),
            support_layer_items(to),
        )?);
    }
    if layers.iter().any(|layer| layer == "citations") {
        diffs.extend(diff_layer_items(
            citation_layer_items(from),
            citation_layer_items(to),
        )?);
    }
    if layers.iter().any(|layer| layer == "exhibits") {
        diffs.extend(diff_layer_items(
            exhibit_layer_items(from),
            exhibit_layer_items(to),
        )?);
    }
    if layers.iter().any(|layer| layer == "rule_findings") {
        diffs.extend(diff_layer_items(
            rule_finding_layer_items(from),
            rule_finding_layer_items(to),
        )?);
    }
    if layers.iter().any(|layer| layer == "formatting") {
        diffs.extend(diff_layer_items(
            formatting_layer_items(from)?,
            formatting_layer_items(to)?,
        )?);
    }
    if layers.iter().any(|layer| layer == "exports") {
        diffs.extend(diff_layer_items(
            export_layer_items(from),
            export_layer_items(to),
        )?);
    }
    Ok(diffs)
}

fn diff_layer_items(
    from: Vec<ComparableLayerItem>,
    to: Vec<ComparableLayerItem>,
) -> ApiResult<Vec<VersionLayerDiff>> {
    let from_map = from
        .into_iter()
        .map(|item| (item.target_id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let to_map = to
        .into_iter()
        .map(|item| (item.target_id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let mut diffs = Vec::new();
    for (target_id, before) in &from_map {
        match to_map.get(target_id) {
            Some(after) => {
                let before_hash = json_hash(&before.value)?;
                let after_hash = json_hash(&after.value)?;
                if before_hash != after_hash {
                    diffs.push(VersionLayerDiff {
                        layer: before.layer.to_string(),
                        target_type: after.target_type.to_string(),
                        target_id: target_id.clone(),
                        title: after.title.clone(),
                        status: "modified".to_string(),
                        before_hash: Some(before_hash),
                        after_hash: Some(after_hash),
                        before_summary: Some(before.summary.clone()),
                        after_summary: Some(after.summary.clone()),
                    });
                }
            }
            None => diffs.push(VersionLayerDiff {
                layer: before.layer.to_string(),
                target_type: before.target_type.to_string(),
                target_id: target_id.clone(),
                title: before.title.clone(),
                status: "removed".to_string(),
                before_hash: Some(json_hash(&before.value)?),
                after_hash: None,
                before_summary: Some(before.summary.clone()),
                after_summary: None,
            }),
        }
    }
    for (target_id, after) in &to_map {
        if !from_map.contains_key(target_id) {
            diffs.push(VersionLayerDiff {
                layer: after.layer.to_string(),
                target_type: after.target_type.to_string(),
                target_id: target_id.clone(),
                title: after.title.clone(),
                status: "added".to_string(),
                before_hash: None,
                after_hash: Some(json_hash(&after.value)?),
                before_summary: None,
                after_summary: Some(after.summary.clone()),
            });
        }
    }
    Ok(diffs)
}

fn support_layer_items(product: &WorkProduct) -> Vec<ComparableLayerItem> {
    product
        .document_ast
        .links
        .iter()
        .map(|link| ComparableLayerItem {
            layer: "support",
            target_type: match link.target_type.as_str() {
                "authority" | "legal_authority" | "provision" | "legal_text" => "legal_authority",
                value => value,
            }
            .to_string(),
            target_id: link.link_id.clone(),
            title: "Support link".to_string(),
            summary: format!(
                "{} {} on {}",
                link.relation, link.target_type, link.source_block_id
            ),
            value: serde_json::json!({
                "source_block_id": link.source_block_id,
                "target_type": link.target_type,
                "target_id": link.target_id,
                "relation": link.relation,
                "confidence": link.confidence,
            }),
        })
        .collect()
}

fn citation_layer_items(product: &WorkProduct) -> Vec<ComparableLayerItem> {
    product
        .document_ast
        .citations
        .iter()
        .map(|citation| ComparableLayerItem {
            layer: "citations",
            target_type: "citation".to_string(),
            target_id: citation.citation_use_id.clone(),
            title: "Citation use".to_string(),
            summary: format!(
                "{} citation on {}",
                citation.status, citation.source_block_id
            ),
            value: serde_json::json!({
                "source_block_id": citation.source_block_id,
                "normalized_citation": citation.normalized_citation,
                "target_type": citation.target_type,
                "target_id": citation.target_id,
                "pinpoint": citation.pinpoint,
                "status": citation.status,
            }),
        })
        .collect()
}

fn exhibit_layer_items(product: &WorkProduct) -> Vec<ComparableLayerItem> {
    product
        .document_ast
        .exhibits
        .iter()
        .map(|exhibit| ComparableLayerItem {
            layer: "exhibits",
            target_type: "exhibit_reference".to_string(),
            target_id: exhibit.exhibit_reference_id.clone(),
            title: "Exhibit reference".to_string(),
            summary: format!("{} exhibit on {}", exhibit.status, exhibit.source_block_id),
            value: serde_json::json!({
                "source_block_id": exhibit.source_block_id,
                "exhibit_id": exhibit.exhibit_id,
                "document_id": exhibit.document_id,
                "page_range": exhibit.page_range,
                "status": exhibit.status,
            }),
        })
        .collect()
}

fn rule_finding_layer_items(product: &WorkProduct) -> Vec<ComparableLayerItem> {
    product
        .document_ast
        .rule_findings
        .iter()
        .map(|finding| ComparableLayerItem {
            layer: "rule_findings",
            target_type: "rule_finding".to_string(),
            target_id: finding.finding_id.clone(),
            title: "Rule finding".to_string(),
            summary: format!(
                "{} {} finding on {}",
                finding.status, finding.severity, finding.target_type
            ),
            value: serde_json::json!({
                "rule_id": finding.rule_id,
                "category": finding.category,
                "severity": finding.severity,
                "target_type": finding.target_type,
                "target_id": finding.target_id,
                "status": finding.status,
            }),
        })
        .collect()
}

fn formatting_layer_items(product: &WorkProduct) -> ApiResult<Vec<ComparableLayerItem>> {
    Ok(vec![ComparableLayerItem {
        layer: "formatting",
        target_type: "formatting_profile".to_string(),
        target_id: product.formatting_profile.profile_id.clone(),
        title: "Formatting profile".to_string(),
        summary: format!(
            "Formatting profile {}",
            product.formatting_profile.profile_id
        ),
        value: json_value(&product.formatting_profile)?,
    }])
}

fn export_layer_items(product: &WorkProduct) -> Vec<ComparableLayerItem> {
    product
        .artifacts
        .iter()
        .map(|artifact| ComparableLayerItem {
            layer: "exports",
            target_type: "export_artifact".to_string(),
            target_id: artifact.artifact_id.clone(),
            title: "Export artifact".to_string(),
            summary: format!("{} export {}", artifact.format, artifact.status),
            value: serde_json::json!({
                "format": artifact.format,
                "profile": artifact.profile,
                "mode": artifact.mode,
                "status": artifact.status,
                "snapshot_id": artifact.snapshot_id,
                "artifact_hash": artifact.artifact_hash,
                "render_profile_hash": artifact.render_profile_hash,
                "object_blob_id": artifact.object_blob_id,
                "size_bytes": artifact.size_bytes,
                "storage_status": artifact.storage_status,
            }),
        })
        .collect()
}

fn layer_change_count(diffs: &[VersionLayerDiff], layer: &str) -> u64 {
    diffs
        .iter()
        .filter(|diff| diff.layer == layer && diff.status != "unchanged")
        .count() as u64
}

fn restore_work_product_scope(
    current: &WorkProduct,
    snapshot: &WorkProduct,
    scope: &str,
    target_ids: &[String],
) -> ApiResult<(WorkProduct, Vec<String>)> {
    let normalized_scope = scope.trim().to_ascii_lowercase();
    let mut restored = current.clone();
    let mut warnings = Vec::new();
    if !target_ids.is_empty() {
        warnings.push(
            "Targeted restore will only apply matching targets in the selected scope.".to_string(),
        );
    }
    match normalized_scope.as_str() {
        "all" | "work_product" | "complaint" => {
            warnings.push(
                "This will replace the current work product with the selected snapshot."
                    .to_string(),
            );
            restored = snapshot.clone();
        }
        "text" | "blocks" | "block" | "paragraph" => {
            restore_blocks_from_snapshot(&mut restored, snapshot, target_ids, &mut warnings)?;
        }
        "metadata" | "document_metadata" => {
            restored.title = snapshot.title.clone();
            restored.status = snapshot.status.clone();
            restored.review_status = snapshot.review_status.clone();
            restored.setup_stage = snapshot.setup_stage.clone();
            restored.document_ast.metadata = snapshot.document_ast.metadata.clone();
            restored.document_ast.title = snapshot.document_ast.title.clone();
        }
        "support" | "links" | "support_links" => {
            restore_links_from_snapshot(&mut restored, snapshot, target_ids);
        }
        "citations" | "citation" => {
            restore_citations_from_snapshot(&mut restored, snapshot, target_ids);
        }
        "exhibits" | "exhibit" => {
            restore_exhibits_from_snapshot(&mut restored, snapshot, target_ids);
        }
        "rule_findings" | "rule_finding" | "qc" => {
            restore_rule_findings_from_snapshot(&mut restored, snapshot, target_ids);
        }
        "formatting" | "format" => {
            restored.formatting_profile = snapshot.formatting_profile.clone();
        }
        "exports" | "export" | "export_state" | "artifacts" => {
            merge_export_state_from_snapshot(&mut restored, snapshot);
        }
        _ => {
            return Err(ApiError::BadRequest(
                "Unsupported restore scope.".to_string(),
            ));
        }
    }
    refresh_work_product_state(&mut restored);
    Ok((restored, warnings))
}

fn restore_blocks_from_snapshot(
    restored: &mut WorkProduct,
    snapshot: &WorkProduct,
    target_ids: &[String],
    warnings: &mut Vec<String>,
) -> ApiResult<()> {
    if target_ids.is_empty() {
        restored.document_ast.blocks = snapshot.document_ast.blocks.clone();
        return Ok(());
    }
    for target_id in target_ids {
        let snapshot_block = find_ast_block(&snapshot.document_ast.blocks, target_id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound("Snapshot block not found".to_string()))?;
        let current_block = find_ast_block(&restored.document_ast.blocks, target_id).cloned();
        match replace_ast_block(&mut restored.document_ast.blocks, &snapshot_block) {
            true => {
                if let Some(current_block) = current_block.as_ref() {
                    if current_block.evidence_ids.len() > snapshot_block.evidence_ids.len()
                        || current_block.links.len() > snapshot_block.links.len()
                    {
                        warnings.push(
                            "Restoring a block may remove current support references on that block."
                                .to_string(),
                        );
                    }
                }
            }
            false => restored.document_ast.blocks.push(snapshot_block),
        }
    }
    Ok(())
}

fn restore_links_from_snapshot(
    restored: &mut WorkProduct,
    snapshot: &WorkProduct,
    target_ids: &[String],
) {
    if target_ids.is_empty() {
        restored.document_ast.links = snapshot.document_ast.links.clone();
        copy_block_refs(
            &mut restored.document_ast.blocks,
            &snapshot.document_ast.blocks,
            "links",
            None,
        );
        return;
    }
    restored
        .document_ast
        .links
        .retain(|link| !target_ids.contains(&link.source_block_id));
    restored.document_ast.links.extend(
        snapshot
            .document_ast
            .links
            .iter()
            .filter(|link| target_ids.contains(&link.source_block_id))
            .cloned(),
    );
    copy_block_refs(
        &mut restored.document_ast.blocks,
        &snapshot.document_ast.blocks,
        "links",
        Some(target_ids),
    );
}

fn restore_citations_from_snapshot(
    restored: &mut WorkProduct,
    snapshot: &WorkProduct,
    target_ids: &[String],
) {
    if target_ids.is_empty() {
        restored.document_ast.citations = snapshot.document_ast.citations.clone();
        copy_block_refs(
            &mut restored.document_ast.blocks,
            &snapshot.document_ast.blocks,
            "citations",
            None,
        );
        return;
    }
    restored
        .document_ast
        .citations
        .retain(|citation| !target_ids.contains(&citation.source_block_id));
    restored.document_ast.citations.extend(
        snapshot
            .document_ast
            .citations
            .iter()
            .filter(|citation| target_ids.contains(&citation.source_block_id))
            .cloned(),
    );
    copy_block_refs(
        &mut restored.document_ast.blocks,
        &snapshot.document_ast.blocks,
        "citations",
        Some(target_ids),
    );
}

fn restore_exhibits_from_snapshot(
    restored: &mut WorkProduct,
    snapshot: &WorkProduct,
    target_ids: &[String],
) {
    if target_ids.is_empty() {
        restored.document_ast.exhibits = snapshot.document_ast.exhibits.clone();
        copy_block_refs(
            &mut restored.document_ast.blocks,
            &snapshot.document_ast.blocks,
            "exhibits",
            None,
        );
        return;
    }
    restored
        .document_ast
        .exhibits
        .retain(|exhibit| !target_ids.contains(&exhibit.source_block_id));
    restored.document_ast.exhibits.extend(
        snapshot
            .document_ast
            .exhibits
            .iter()
            .filter(|exhibit| target_ids.contains(&exhibit.source_block_id))
            .cloned(),
    );
    copy_block_refs(
        &mut restored.document_ast.blocks,
        &snapshot.document_ast.blocks,
        "exhibits",
        Some(target_ids),
    );
}

fn restore_rule_findings_from_snapshot(
    restored: &mut WorkProduct,
    snapshot: &WorkProduct,
    target_ids: &[String],
) {
    if target_ids.is_empty() {
        restored.document_ast.rule_findings = snapshot.document_ast.rule_findings.clone();
        restored.findings = snapshot.findings.clone();
        copy_block_refs(
            &mut restored.document_ast.blocks,
            &snapshot.document_ast.blocks,
            "rule_findings",
            None,
        );
        return;
    }
    restored
        .document_ast
        .rule_findings
        .retain(|finding| !target_ids.contains(&finding.target_id));
    restored.document_ast.rule_findings.extend(
        snapshot
            .document_ast
            .rule_findings
            .iter()
            .filter(|finding| target_ids.contains(&finding.target_id))
            .cloned(),
    );
    restored.findings = restored.document_ast.rule_findings.clone();
    copy_block_refs(
        &mut restored.document_ast.blocks,
        &snapshot.document_ast.blocks,
        "rule_findings",
        Some(target_ids),
    );
}

fn merge_export_state_from_snapshot(restored: &mut WorkProduct, snapshot: &WorkProduct) {
    for artifact in &snapshot.artifacts {
        if !restored
            .artifacts
            .iter()
            .any(|current| current.artifact_id == artifact.artifact_id)
        {
            restored.artifacts.push(artifact.clone());
        }
    }
}

fn find_ast_block<'a>(
    blocks: &'a [WorkProductBlock],
    block_id: &str,
) -> Option<&'a WorkProductBlock> {
    for block in blocks {
        if block.block_id == block_id {
            return Some(block);
        }
        if let Some(found) = find_ast_block(&block.children, block_id) {
            return Some(found);
        }
    }
    None
}

fn replace_ast_block(blocks: &mut [WorkProductBlock], replacement: &WorkProductBlock) -> bool {
    for block in blocks {
        if block.block_id == replacement.block_id {
            *block = replacement.clone();
            return true;
        }
        if replace_ast_block(&mut block.children, replacement) {
            return true;
        }
    }
    false
}

fn copy_block_refs(
    blocks: &mut [WorkProductBlock],
    snapshot_blocks: &[WorkProductBlock],
    ref_kind: &str,
    only_block_ids: Option<&[String]>,
) {
    for block in blocks {
        if only_block_ids
            .map(|ids| ids.contains(&block.block_id))
            .unwrap_or(true)
        {
            if let Some(snapshot_block) = find_ast_block(snapshot_blocks, &block.block_id) {
                match ref_kind {
                    "links" => block.links = snapshot_block.links.clone(),
                    "citations" => block.citations = snapshot_block.citations.clone(),
                    "exhibits" => block.exhibits = snapshot_block.exhibits.clone(),
                    "rule_findings" => {
                        block.rule_finding_ids = snapshot_block.rule_finding_ids.clone()
                    }
                    _ => {}
                }
            }
        }
        copy_block_refs(
            &mut block.children,
            snapshot_blocks,
            ref_kind,
            only_block_ids,
        );
    }
}

fn work_product_facade_change_inputs(
    before: Option<&WorkProduct>,
    after: &WorkProduct,
) -> Vec<VersionChangeInput> {
    let Some(before) = before else {
        return vec![VersionChangeInput {
            target_type: "work_product".to_string(),
            target_id: after.work_product_id.clone(),
            operation: "create".to_string(),
            before: None,
            after: json_value(after).ok(),
            summary: "Complaint-profile work product created.".to_string(),
            legal_impact: LegalImpactSummary::default(),
            ai_audit_id: None,
        }];
    };

    let mut changes = Vec::new();
    if before.title != after.title
        || before.status != after.status
        || before.review_status != after.review_status
        || before.setup_stage != after.setup_stage
        || before.formatting_profile.profile_id != after.formatting_profile.profile_id
    {
        changes.push(VersionChangeInput {
            target_type: "work_product".to_string(),
            target_id: after.work_product_id.clone(),
            operation: "update".to_string(),
            before: json_value(before).ok(),
            after: json_value(after).ok(),
            summary: "Complaint metadata changed.".to_string(),
            legal_impact: LegalImpactSummary::default(),
            ai_audit_id: None,
        });
    }

    for after_block in &after.blocks {
        let target_type = if after_block.block_type == "paragraph" {
            "paragraph"
        } else {
            "block"
        };
        match before
            .blocks
            .iter()
            .find(|block| block.block_id == after_block.block_id)
        {
            Some(before_block) => {
                if before_block.text != after_block.text
                    || before_block.title != after_block.title
                    || before_block.ordinal != after_block.ordinal
                    || before_block.parent_block_id != after_block.parent_block_id
                    || before_block.fact_ids != after_block.fact_ids
                    || before_block.evidence_ids != after_block.evidence_ids
                    || json_value(&before_block.authorities).ok()
                        != json_value(&after_block.authorities).ok()
                    || before_block.review_status != after_block.review_status
                {
                    let before_authority_ids = before_block
                        .authorities
                        .iter()
                        .map(|authority| authority.canonical_id.clone())
                        .collect::<Vec<_>>();
                    let after_authority_ids = after_block
                        .authorities
                        .iter()
                        .map(|authority| authority.canonical_id.clone())
                        .collect::<Vec<_>>();
                    changes.push(VersionChangeInput {
                        target_type: target_type.to_string(),
                        target_id: after_block.block_id.clone(),
                        operation: "update".to_string(),
                        before: json_value(before_block).ok(),
                        after: json_value(after_block).ok(),
                        summary: format!("{} changed.", after_block.title),
                        legal_impact: LegalImpactSummary {
                            affected_counts: Vec::new(),
                            affected_elements: Vec::new(),
                            affected_facts: union_string_ids(
                                &before_block.fact_ids,
                                &after_block.fact_ids,
                            ),
                            affected_evidence: union_string_ids(
                                &before_block.evidence_ids,
                                &after_block.evidence_ids,
                            ),
                            affected_authorities: union_string_ids(
                                &before_authority_ids,
                                &after_authority_ids,
                            ),
                            affected_exhibits: Vec::new(),
                            support_status_before: None,
                            support_status_after: None,
                            qc_warnings_added: Vec::new(),
                            qc_warnings_resolved: Vec::new(),
                            blocking_issues_added: Vec::new(),
                            blocking_issues_resolved: Vec::new(),
                        },
                        ai_audit_id: None,
                    });
                }
            }
            None => changes.push(VersionChangeInput {
                target_type: target_type.to_string(),
                target_id: after_block.block_id.clone(),
                operation: "create".to_string(),
                before: None,
                after: json_value(after_block).ok(),
                summary: format!("{} created.", after_block.title),
                legal_impact: LegalImpactSummary {
                    affected_counts: Vec::new(),
                    affected_elements: Vec::new(),
                    affected_facts: after_block.fact_ids.clone(),
                    affected_evidence: after_block.evidence_ids.clone(),
                    affected_authorities: after_block
                        .authorities
                        .iter()
                        .map(|authority| authority.canonical_id.clone())
                        .collect(),
                    affected_exhibits: Vec::new(),
                    support_status_before: None,
                    support_status_after: None,
                    qc_warnings_added: Vec::new(),
                    qc_warnings_resolved: Vec::new(),
                    blocking_issues_added: Vec::new(),
                    blocking_issues_resolved: Vec::new(),
                },
                ai_audit_id: None,
            }),
        }
    }
    for before_block in &before.blocks {
        if !after
            .blocks
            .iter()
            .any(|block| block.block_id == before_block.block_id)
        {
            changes.push(VersionChangeInput {
                target_type: if before_block.block_type == "paragraph" {
                    "paragraph"
                } else {
                    "block"
                }
                .to_string(),
                target_id: before_block.block_id.clone(),
                operation: "delete".to_string(),
                before: json_value(before_block).ok(),
                after: None,
                summary: format!("{} removed.", before_block.title),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            });
        }
    }

    if json_value(&before.findings).ok() != json_value(&after.findings).ok() {
        let before_open = before
            .findings
            .iter()
            .filter(|finding| finding.status == "open")
            .map(|finding| finding.finding_id.clone())
            .collect::<Vec<_>>();
        let after_open = after
            .findings
            .iter()
            .filter(|finding| finding.status == "open")
            .map(|finding| finding.finding_id.clone())
            .collect::<Vec<_>>();
        changes.push(VersionChangeInput {
            target_type: "rule_finding".to_string(),
            target_id: after.work_product_id.clone(),
            operation: "update".to_string(),
            before: json_value(&before.findings).ok(),
            after: json_value(&after.findings).ok(),
            summary: "QC findings changed.".to_string(),
            legal_impact: LegalImpactSummary {
                qc_warnings_added: after_open
                    .iter()
                    .filter(|id| !before_open.contains(id))
                    .cloned()
                    .collect(),
                qc_warnings_resolved: before_open
                    .iter()
                    .filter(|id| !after_open.contains(id))
                    .cloned()
                    .collect(),
                ..LegalImpactSummary::default()
            },
            ai_audit_id: None,
        });
    }

    for artifact in &after.artifacts {
        if !before
            .artifacts
            .iter()
            .any(|item| item.artifact_id == artifact.artifact_id)
        {
            changes.push(VersionChangeInput {
                target_type: "export".to_string(),
                target_id: artifact.artifact_id.clone(),
                operation: "create".to_string(),
                before: None,
                after: json_value(artifact).ok(),
                summary: format!("{} export generated.", artifact.format.to_uppercase()),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            });
        }
    }

    changes
}

fn union_string_ids(left: &[String], right: &[String]) -> Vec<String> {
    let mut values = left.to_vec();
    for value in right {
        if !values.contains(value) {
            values.push(value.clone());
        }
    }
    values
}

fn legal_support_use_from_anchor(
    product: &WorkProduct,
    anchor: &WorkProductAnchor,
) -> LegalSupportUse {
    let source_type = match anchor.target_type.as_str() {
        "fact" => "fact",
        "evidence" => "evidence",
        "document" => "document",
        "source_span" => "source_span",
        "authority" | "provision" | "legal_text" => "authority",
        value => value,
    }
    .to_string();
    let source_id = anchor
        .canonical_id
        .clone()
        .filter(|_| matches!(source_type.as_str(), "authority" | "provision" | "citation"))
        .unwrap_or_else(|| anchor.target_id.clone());
    LegalSupportUse {
        id: anchor.anchor_id.clone(),
        support_use_id: anchor.anchor_id.clone(),
        matter_id: product.matter_id.clone(),
        subject_id: product.work_product_id.clone(),
        branch_id: format!("{}:branch:main", product.work_product_id),
        target_type: "block".to_string(),
        target_id: anchor.block_id.clone(),
        source_type,
        source_id,
        relation: anchor.relation.clone(),
        status: anchor.status.clone(),
        quote: anchor.quote.clone(),
        pinpoint: anchor.pinpoint.clone(),
        confidence: None,
        created_snapshot_id: String::new(),
        retired_snapshot_id: None,
    }
}

fn support_use_label(source_type: &str) -> &'static str {
    match source_type {
        "fact" => "FactUse",
        "authority" | "provision" | "citation" => "AuthorityUse",
        "element" => "ElementSupport",
        _ => "LegalSupportUse",
    }
}

fn version_summary_for_changes(summary: &str, changes: &[VersionChange]) -> VersionChangeSummary {
    VersionChangeSummary {
        text_changes: changes
            .iter()
            .filter(|change| {
                matches!(
                    change.target_type.as_str(),
                    "block" | "paragraph" | "work_product"
                )
            })
            .count() as u64,
        support_changes: changes
            .iter()
            .filter(|change| {
                change.target_type.contains("support") || change.target_type.contains("link")
            })
            .count() as u64,
        citation_changes: changes
            .iter()
            .filter(|change| change.target_type.contains("citation"))
            .count() as u64,
        authority_changes: changes
            .iter()
            .filter(|change| change.target_type.contains("authority"))
            .count() as u64,
        qc_changes: changes
            .iter()
            .filter(|change| {
                change.target_type.contains("finding") || change.target_type.contains("qc")
            })
            .count() as u64,
        export_changes: changes
            .iter()
            .filter(|change| change.target_type == "export" || change.target_type == "artifact")
            .count() as u64,
        ai_changes: changes
            .iter()
            .filter(|change| change.ai_audit_id.is_some() || change.target_type == "ai_edit")
            .count() as u64,
        targets_changed: changes
            .iter()
            .map(|change| VersionTargetSummary {
                target_type: change.target_type.clone(),
                target_id: change.target_id.clone(),
                label: Some(change.summary.clone()),
            })
            .collect(),
        risk_level: if changes
            .iter()
            .any(|change| !change.legal_impact.blocking_issues_added.is_empty())
        {
            "high"
        } else {
            "low"
        }
        .to_string(),
        user_summary: summary.to_string(),
    }
}

fn merge_legal_impacts<'a>(
    impacts: impl Iterator<Item = &'a LegalImpactSummary>,
) -> LegalImpactSummary {
    let mut merged = LegalImpactSummary::default();
    for impact in impacts {
        merged
            .affected_counts
            .extend(impact.affected_counts.clone());
        merged
            .affected_elements
            .extend(impact.affected_elements.clone());
        merged.affected_facts.extend(impact.affected_facts.clone());
        merged
            .affected_evidence
            .extend(impact.affected_evidence.clone());
        merged
            .affected_authorities
            .extend(impact.affected_authorities.clone());
        merged
            .affected_exhibits
            .extend(impact.affected_exhibits.clone());
        merged
            .qc_warnings_added
            .extend(impact.qc_warnings_added.clone());
        merged
            .qc_warnings_resolved
            .extend(impact.qc_warnings_resolved.clone());
        merged
            .blocking_issues_added
            .extend(impact.blocking_issues_added.clone());
        merged
            .blocking_issues_resolved
            .extend(impact.blocking_issues_resolved.clone());
    }
    merged
}

fn role_names(parties: &[ComplaintParty], roles: &[&str]) -> Vec<String> {
    let values = parties
        .iter()
        .filter(|party| {
            roles
                .iter()
                .any(|role| party.role.eq_ignore_ascii_case(role))
        })
        .map(|party| party.name.clone())
        .collect::<Vec<_>>();
    if values.is_empty() {
        roles
            .first()
            .map(|role| vec![title_case(role)])
            .unwrap_or_default()
    } else {
        values
    }
}

fn infer_county(court: &str) -> String {
    court
        .split_whitespace()
        .take_while(|part| !part.eq_ignore_ascii_case("county"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn empty_as_review_needed(value: &str) -> String {
    if value.trim().is_empty() {
        "[review needed]".to_string()
    } else {
        value.to_string()
    }
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn put_options(mime_type: Option<String>, sha256: Option<String>) -> PutOptions {
    let mut metadata = BTreeMap::new();
    if let Some(sha256) = sha256 {
        metadata.insert("sha256".to_string(), sha256);
    }
    PutOptions {
        content_type: mime_type,
        metadata,
    }
}

fn build_original_provenance(
    matter_id: &str,
    document: &CaseDocument,
    object: &StoredObject,
    status: &str,
) -> DocumentProvenance {
    let now = now_string();
    let blob = ObjectBlob {
        object_blob_id: object_blob_id_for_document(document),
        id: object_blob_id_for_document(document),
        sha256: document.file_hash.clone(),
        size_bytes: object.content_length,
        mime_type: document
            .mime_type
            .clone()
            .or_else(|| object.content_type.clone()),
        storage_provider: document.storage_provider.clone(),
        storage_bucket: object
            .bucket
            .clone()
            .or_else(|| document.storage_bucket.clone()),
        storage_key: object.key.clone(),
        etag: object
            .etag
            .clone()
            .or_else(|| document.content_etag.clone()),
        storage_class: None,
        created_at: now.clone(),
        retention_state: "active".to_string(),
    };
    let version_id = original_version_id(&document.document_id);
    let version = DocumentVersion {
        document_version_id: version_id.clone(),
        id: version_id.clone(),
        matter_id: matter_id.to_string(),
        document_id: document.document_id.clone(),
        object_blob_id: blob.object_blob_id.clone(),
        role: "original".to_string(),
        artifact_kind: "original_upload".to_string(),
        source_version_id: None,
        created_by: "casebuilder_upload".to_string(),
        current: true,
        created_at: now.clone(),
        storage_provider: document.storage_provider.clone(),
        storage_bucket: blob.storage_bucket.clone(),
        storage_key: object.key.clone(),
        sha256: document.file_hash.clone(),
        size_bytes: object.content_length,
        mime_type: document
            .mime_type
            .clone()
            .or_else(|| object.content_type.clone()),
    };
    let run_id = primary_ingestion_run_id(&document.document_id);
    let run = IngestionRun {
        ingestion_run_id: run_id.clone(),
        id: run_id,
        matter_id: matter_id.to_string(),
        document_id: document.document_id.clone(),
        document_version_id: Some(version.document_version_id.clone()),
        object_blob_id: Some(blob.object_blob_id.clone()),
        input_sha256: document.file_hash.clone(),
        status: status.to_string(),
        stage: status.to_string(),
        mode: "deterministic".to_string(),
        started_at: now,
        completed_at: None,
        error_code: None,
        error_message: None,
        retryable: false,
        produced_node_ids: Vec::new(),
        produced_object_keys: vec![object.key.clone()],
        parser_id: Some("casebuilder-parser-registry".to_string()),
        parser_version: Some(PARSER_REGISTRY_VERSION.to_string()),
        chunker_version: Some(CHUNKER_VERSION.to_string()),
        citation_resolver_version: Some(CITATION_RESOLVER_VERSION.to_string()),
        index_version: Some(CASE_INDEX_VERSION.to_string()),
    };
    DocumentProvenance {
        object_blob: blob,
        document_version: version,
        ingestion_run: run,
    }
}

fn apply_document_provenance(document: &mut CaseDocument, provenance: &DocumentProvenance) {
    document.object_blob_id = Some(provenance.object_blob.object_blob_id.clone());
    document.current_version_id = Some(provenance.document_version.document_version_id.clone());
    push_unique(
        &mut document.ingestion_run_ids,
        provenance.ingestion_run.ingestion_run_id.clone(),
    );
}

fn source_context_from_provenance(provenance: Option<&DocumentProvenance>) -> SourceContext {
    SourceContext {
        document_version_id: provenance
            .map(|value| value.document_version.document_version_id.clone()),
        object_blob_id: provenance.map(|value| value.object_blob.object_blob_id.clone()),
        ingestion_run_id: provenance.map(|value| value.ingestion_run.ingestion_run_id.clone()),
    }
}

fn object_blob_id_for_document(document: &CaseDocument) -> String {
    if let Some(sha256) = document.file_hash.as_deref() {
        return object_blob_id_for_hash(sha256);
    }
    let seed = format!(
        "{}:{}:{}",
        document.storage_provider,
        document.storage_bucket.clone().unwrap_or_default(),
        document.storage_key.clone().unwrap_or_default()
    );
    format!("blob:object:{}", hex_prefix(seed.as_bytes(), 24))
}

fn object_blob_id_for_hash(sha256: &str) -> String {
    let raw = sha256
        .trim()
        .strip_prefix("sha256:")
        .unwrap_or(sha256.trim());
    format!("blob:sha256:{}", raw.to_ascii_lowercase())
}

fn should_inline_payload(byte_len: usize, limit: u64) -> bool {
    byte_len as u64 <= limit
}

fn storage_hash_segment(hash: &str) -> String {
    sanitize_path_segment(hash.trim().strip_prefix("sha256:").unwrap_or(hash.trim()))
}

fn work_product_storage_prefix(matter_id: &str, work_product_id: &str) -> String {
    format!(
        "casebuilder/matters/{}/work-products/{}",
        hex_prefix(matter_id.as_bytes(), 24),
        hex_prefix(work_product_id.as_bytes(), 24)
    )
}

fn snapshot_full_state_key(
    matter_id: &str,
    work_product_id: &str,
    snapshot_id: &str,
    state_hash: &str,
) -> String {
    format!(
        "{}/snapshots/{}/full-state.{}.json",
        work_product_storage_prefix(matter_id, work_product_id),
        hex_prefix(snapshot_id.as_bytes(), 24),
        storage_hash_segment(state_hash)
    )
}

fn snapshot_manifest_key(
    matter_id: &str,
    work_product_id: &str,
    snapshot_id: &str,
    manifest_hash: &str,
) -> String {
    format!(
        "{}/snapshots/{}/manifest.{}.json",
        work_product_storage_prefix(matter_id, work_product_id),
        hex_prefix(snapshot_id.as_bytes(), 24),
        storage_hash_segment(manifest_hash)
    )
}

fn snapshot_entity_state_key(
    matter_id: &str,
    work_product_id: &str,
    snapshot_id: &str,
    entity_type: &str,
    entity_hash: &str,
) -> String {
    format!(
        "{}/snapshots/{}/states/{}/{}.json",
        work_product_storage_prefix(matter_id, work_product_id),
        hex_prefix(snapshot_id.as_bytes(), 24),
        sanitize_path_segment(entity_type),
        storage_hash_segment(entity_hash)
    )
}

fn work_product_export_key(
    matter_id: &str,
    work_product_id: &str,
    artifact_id: &str,
    artifact_hash: &str,
    ext: &str,
) -> String {
    format!(
        "{}/exports/{}/{}.{}",
        work_product_storage_prefix(matter_id, work_product_id),
        hex_prefix(artifact_id.as_bytes(), 24),
        storage_hash_segment(artifact_hash),
        sanitize_path_segment(ext)
    )
}

fn safe_work_product_download_filename(artifact: &WorkProductArtifact) -> String {
    let seed = artifact
        .artifact_hash
        .as_deref()
        .unwrap_or(artifact.artifact_id.as_str());
    format!(
        "work-product-export-{}.{}",
        hex_prefix(seed.as_bytes(), 16),
        sanitize_path_segment(&artifact.format)
    )
}

fn original_version_id(document_id: &str) -> String {
    format!("version:{}:original", sanitize_path_segment(document_id))
}

fn primary_ingestion_run_id(document_id: &str) -> String {
    format!("ingestion:{}:primary", sanitize_path_segment(document_id))
}

fn source_span_id(document_id: &str, kind: &str, index: u64) -> String {
    format!(
        "span:{}:{}:{}",
        sanitize_path_segment(document_id),
        sanitize_path_segment(kind),
        index
    )
}

fn source_spans_for_chunks(
    matter_id: &str,
    document_id: &str,
    chunks: &[ExtractedTextChunk],
    context: &SourceContext,
) -> Vec<SourceSpan> {
    chunks
        .iter()
        .map(|chunk| SourceSpan {
            source_span_id: chunk
                .source_span_id
                .clone()
                .unwrap_or_else(|| source_span_id(document_id, "chunk", chunk.page)),
            id: chunk
                .source_span_id
                .clone()
                .unwrap_or_else(|| source_span_id(document_id, "chunk", chunk.page)),
            matter_id: matter_id.to_string(),
            document_id: document_id.to_string(),
            document_version_id: context.document_version_id.clone(),
            object_blob_id: context.object_blob_id.clone(),
            ingestion_run_id: context.ingestion_run_id.clone(),
            page: Some(chunk.page),
            chunk_id: Some(chunk.chunk_id.clone()),
            byte_start: chunk.byte_start,
            byte_end: chunk.byte_end,
            char_start: chunk.char_start,
            char_end: chunk.char_end,
            quote: Some(chunk.text.clone()),
            extraction_method: "deterministic_text_chunk".to_string(),
            confidence: 1.0,
            review_status: "unreviewed".to_string(),
            unavailable_reason: None,
        })
        .collect()
}

fn source_span_for_sentence(
    matter_id: &str,
    document_id: &str,
    index: u64,
    sentence: &SentenceCandidate,
    context: &SourceContext,
) -> SourceSpan {
    let id = source_span_id(document_id, "fact", index);
    SourceSpan {
        source_span_id: id.clone(),
        id,
        matter_id: matter_id.to_string(),
        document_id: document_id.to_string(),
        document_version_id: context.document_version_id.clone(),
        object_blob_id: context.object_blob_id.clone(),
        ingestion_run_id: context.ingestion_run_id.clone(),
        page: Some(1),
        chunk_id: None,
        byte_start: Some(sentence.byte_start),
        byte_end: Some(sentence.byte_end),
        char_start: Some(sentence.char_start),
        char_end: Some(sentence.char_end),
        quote: Some(sentence.text.clone()),
        extraction_method: "deterministic_sentence".to_string(),
        confidence: 0.55,
        review_status: "unreviewed".to_string(),
        unavailable_reason: None,
    }
}

fn manual_evidence_source_span(
    matter_id: &str,
    document_id: &str,
    evidence_id: &str,
    source_span: Option<&str>,
    quote: &str,
    context: &SourceContext,
) -> SourceSpan {
    let id = format!("span:{}:evidence", sanitize_path_segment(evidence_id));
    SourceSpan {
        source_span_id: id.clone(),
        id,
        matter_id: matter_id.to_string(),
        document_id: document_id.to_string(),
        document_version_id: context.document_version_id.clone(),
        object_blob_id: context.object_blob_id.clone(),
        ingestion_run_id: context.ingestion_run_id.clone(),
        page: None,
        chunk_id: source_span.map(str::to_string),
        byte_start: None,
        byte_end: None,
        char_start: None,
        char_end: None,
        quote: Some(quote.to_string()),
        extraction_method: "manual_evidence_quote".to_string(),
        confidence: 0.75,
        review_status: "unreviewed".to_string(),
        unavailable_reason: None,
    }
}

fn failed_ingestion_run(
    run: &IngestionRun,
    stage: &str,
    error_code: &str,
    error_message: &str,
    retryable: bool,
) -> IngestionRun {
    let mut next = run.clone();
    next.status = "failed".to_string();
    next.stage = stage.to_string();
    next.completed_at = Some(now_string());
    next.error_code = Some(error_code.to_string());
    next.error_message = Some(error_message.to_string());
    next.retryable = retryable;
    next
}

fn completed_ingestion_run(
    run: &IngestionRun,
    status: &str,
    stage: &str,
    produced_node_ids: Vec<String>,
) -> IngestionRun {
    let mut next = run.clone();
    next.status = status.to_string();
    next.stage = stage.to_string();
    next.completed_at = Some(now_string());
    next.error_code = None;
    next.error_message = None;
    next.retryable = false;
    next.produced_node_ids = produced_node_ids;
    next
}

fn produced_node_ids(
    chunks: &[ExtractedTextChunk],
    spans: &[SourceSpan],
    facts: &[CaseFact],
) -> Vec<String> {
    let mut ids = Vec::new();
    for chunk in chunks {
        push_unique(&mut ids, chunk.chunk_id.clone());
    }
    for span in spans {
        push_unique(&mut ids, span.source_span_id.clone());
    }
    for fact in facts {
        push_unique(&mut ids, fact.fact_id.clone());
    }
    ids
}

fn upload_id_for_document(document_id: &str) -> String {
    format!("upload:{}", sanitize_path_segment(document_id))
}

fn timestamp_after(seconds: u64) -> String {
    (now_secs() + seconds).to_string()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn parse_timestamp(value: &str) -> Option<u64> {
    value.parse().ok()
}

fn validate_mime_type(mime_type: Option<&str>) -> ApiResult<()> {
    let Some(mime_type) = mime_type else {
        return Ok(());
    };
    let allowed = [
        "text/",
        "image/",
        "audio/",
        "video/",
        "application/pdf",
        "application/json",
        "application/octet-stream",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.",
        "application/vnd.ms-excel",
        "application/vnd.ms-powerpoint",
        "application/zip",
    ];
    if allowed.iter().any(|prefix| mime_type.starts_with(prefix)) {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "Unsupported upload MIME type {mime_type}"
        )))
    }
}

fn to_payload<T: serde::Serialize>(value: &T) -> ApiResult<String> {
    serde_json::to_string(value).map_err(|error| ApiError::Internal(error.to_string()))
}

fn from_payload<T: serde::de::DeserializeOwned>(payload: &str) -> ApiResult<T> {
    serde_json::from_str(payload).map_err(|error| ApiError::Internal(error.to_string()))
}

fn row_u64(row: &neo4rs::Row, key: &str) -> u64 {
    row.get::<i64>(key).ok().unwrap_or(0).max(0) as u64
}

fn now_string() -> String {
    now_secs().to_string()
}

fn generate_id(prefix: &str, seed: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("{prefix}:{}:{millis}", slug(seed))
}

fn generate_opaque_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let seed = format!("{prefix}:{nanos}");
    format!("{prefix}:{}", hex_prefix(seed.as_bytes(), 26))
}

fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(chars);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
        if out.len() >= chars {
            break;
        }
    }
    out.truncate(chars);
    out
}

fn slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "item".to_string()
    } else {
        slug.chars().take(48).collect()
    }
}

fn short_name(name: &str) -> String {
    name.split(" v. ").next().unwrap_or(name).trim().to_string()
}

fn title_from_filename(filename: &str) -> String {
    Path::new(filename)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(filename)
        .replace(['_', '-'], " ")
}

fn sanitize_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
fn sanitize_filename(value: &str) -> String {
    let candidate = Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("upload.txt");
    sanitize_path_segment(candidate)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    format!("sha256:{out}")
}

fn parse_document_bytes(filename: &str, mime_type: Option<&str>, bytes: &[u8]) -> ParserOutcome {
    let lower = filename.to_ascii_lowercase();
    let mime = mime_type.unwrap_or_default().to_ascii_lowercase();
    if is_image_file(&lower, &mime) {
        return ParserOutcome {
            parser_id: "casebuilder-image-ocr-deferred".to_string(),
            status: "ocr_required".to_string(),
            message: "Stored privately. OCR is required before text indexing.".to_string(),
            text: None,
        };
    }
    if is_audio_video_file(&lower, &mime) {
        return ParserOutcome {
            parser_id: "casebuilder-media-transcription-deferred".to_string(),
            status: "transcription_deferred".to_string(),
            message: "Stored privately. Transcription is deferred for this media file.".to_string(),
            text: None,
        };
    }
    if lower.ends_with(".pdf") || mime == "application/pdf" || bytes.starts_with(b"%PDF") {
        let text = extract_pdf_embedded_text(bytes);
        return ParserOutcome {
            parser_id: "casebuilder-pdf-embedded-text-v1".to_string(),
            status: if text
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "processed".to_string()
            } else {
                "ocr_required".to_string()
            },
            message: if text
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "Stored privately. Embedded PDF text is ready for deterministic extraction."
                    .to_string()
            } else {
                "Stored privately. No embedded PDF text was detected; OCR is required.".to_string()
            },
            text,
        };
    }
    if lower.ends_with(".docx")
        || mime == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    {
        let text = extract_docx_like_text(bytes);
        return ParserOutcome {
            parser_id: "casebuilder-docx-text-v1".to_string(),
            status: if text
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "processed".to_string()
            } else {
                "unsupported".to_string()
            },
            message: if text
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "Stored privately. DOCX XML text is ready for deterministic extraction.".to_string()
            } else {
                "Stored privately. Compressed DOCX parsing is deferred until the document parser worker is enabled."
                    .to_string()
            },
            text,
        };
    }
    let lower = filename.to_ascii_lowercase();
    let text_like_mime = mime_type
        .map(|value| {
            value.starts_with("text/")
                || matches!(
                    value,
                    "application/json" | "application/xml" | "application/x-ndjson"
                )
        })
        .unwrap_or(false);
    let text_like_extension = lower.ends_with(".txt")
        || lower.ends_with(".md")
        || lower.ends_with(".markdown")
        || lower.ends_with(".csv")
        || lower.ends_with(".html")
        || lower.ends_with(".htm")
        || lower.ends_with(".json")
        || lower.ends_with(".log");
    if !(text_like_mime || text_like_extension) {
        return ParserOutcome {
            parser_id: "casebuilder-binary-unsupported-v1".to_string(),
            status: "unsupported".to_string(),
            message: "Stored privately. This file type is not parseable in the deterministic V0 importer.".to_string(),
            text: None,
        };
    }
    let text = String::from_utf8(bytes.to_vec())
        .ok()
        .map(|text| {
            if lower.ends_with(".html") || lower.ends_with(".htm") || mime_type == Some("text/html")
            {
                strip_html_text(&text)
            } else {
                text
            }
        })
        .filter(|text| !text.trim().is_empty());
    ParserOutcome {
        parser_id: if lower.ends_with(".md") || lower.ends_with(".markdown") {
            "casebuilder-markdown-v1".to_string()
        } else if lower.ends_with(".csv") || mime_type == Some("text/csv") {
            "casebuilder-csv-text-v1".to_string()
        } else if lower.ends_with(".html")
            || lower.ends_with(".htm")
            || mime_type == Some("text/html")
        {
            "casebuilder-html-text-v1".to_string()
        } else {
            "casebuilder-plain-text-v1".to_string()
        },
        status: if text.is_some() {
            "processed"
        } else {
            "failed"
        }
        .to_string(),
        message: if text.is_some() {
            "Stored privately. Text is ready for deterministic V0 extraction.".to_string()
        } else {
            "Stored privately, but the text parser could not decode this file as UTF-8.".to_string()
        },
        text,
    }
}

fn is_image_file(lower: &str, mime: &str) -> bool {
    mime.starts_with("image/")
        || [
            ".png", ".jpg", ".jpeg", ".gif", ".webp", ".heic", ".tif", ".tiff",
        ]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn is_audio_video_file(lower: &str, mime: &str) -> bool {
    mime.starts_with("audio/")
        || mime.starts_with("video/")
        || [".mp3", ".m4a", ".wav", ".mp4", ".mov", ".m4v", ".webm"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

fn strip_html_text(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut entity = String::new();
    let mut in_entity = false;
    for ch in text.chars() {
        match ch {
            '<' if !in_entity => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            '&' if !in_tag => {
                in_entity = true;
                entity.clear();
            }
            ';' if in_entity => {
                in_entity = false;
                out.push(match entity.as_str() {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    "nbsp" => ' ',
                    _ => ' ',
                });
            }
            _ if in_tag => {}
            _ if in_entity => entity.push(ch),
            _ => out.push(ch),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_pdf_embedded_text(bytes: &[u8]) -> Option<String> {
    let raw = String::from_utf8_lossy(bytes);
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_literal = false;
    let mut escaped = false;
    for ch in raw.chars() {
        if in_literal {
            if escaped {
                current.push(match ch {
                    'n' | 'r' | 't' => ' ',
                    value => value,
                });
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == ')' {
                let cleaned = current.split_whitespace().collect::<Vec<_>>().join(" ");
                if cleaned
                    .chars()
                    .filter(|value| value.is_ascii_alphabetic())
                    .count()
                    >= 3
                {
                    values.push(cleaned);
                }
                current.clear();
                in_literal = false;
            } else {
                current.push(ch);
            }
        } else if ch == '(' {
            in_literal = true;
        }
    }
    let text = values.join("\n");
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_docx_like_text(bytes: &[u8]) -> Option<String> {
    extract_docx_zip_text(bytes).or_else(|| {
        let raw = String::from_utf8(bytes.to_vec()).ok()?;
        extract_docx_xml_text(&raw)
    })
}

fn extract_docx_zip_text(bytes: &[u8]) -> Option<String> {
    let eocd = find_zip_eocd(bytes)?;
    let entry_count = le_u16(bytes, eocd + 10)? as usize;
    let mut cursor = le_u32(bytes, eocd + 16)? as usize;
    let mut parts = Vec::new();

    for _ in 0..entry_count {
        if cursor + 46 > bytes.len() || le_u32(bytes, cursor)? != 0x0201_4b50 {
            break;
        }
        let compression = le_u16(bytes, cursor + 10)?;
        let compressed_size = le_u32(bytes, cursor + 20)? as usize;
        let name_len = le_u16(bytes, cursor + 28)? as usize;
        let extra_len = le_u16(bytes, cursor + 30)? as usize;
        let comment_len = le_u16(bytes, cursor + 32)? as usize;
        let local_header_offset = le_u32(bytes, cursor + 42)? as usize;
        let name_start = cursor + 46;
        let name_end = name_start.checked_add(name_len)?;
        let next = name_end.checked_add(extra_len)?.checked_add(comment_len)?;
        if next > bytes.len() {
            break;
        }

        let name = std::str::from_utf8(&bytes[name_start..name_end]).ok()?;
        if is_docx_text_part(name) {
            let entry = read_zip_entry(bytes, local_header_offset, compression, compressed_size)?;
            let raw = String::from_utf8(entry).ok()?;
            if let Some(text) = extract_docx_xml_text(&raw) {
                parts.push(text);
            }
        }
        cursor = next;
    }

    let text = parts.join("\n");
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_docx_xml_text(raw: &str) -> Option<String> {
    if !raw.contains("<w:t") && !raw.contains("<text") {
        return None;
    }
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(start_rel) = raw[cursor..].find('>') {
        let tag_end = cursor + start_rel + 1;
        let tag_start = raw[..tag_end].rfind('<').unwrap_or(cursor);
        let tag = &raw[tag_start..tag_end];
        if tag.starts_with("<w:t") || tag.starts_with("<text") {
            if let Some(end_rel) = raw[tag_end..].find("</") {
                out.push_str(&decode_xml_text(&raw[tag_end..tag_end + end_rel]));
                out.push(' ');
                cursor = tag_end + end_rel;
                continue;
            }
        }
        cursor = tag_end;
    }
    let text = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn is_docx_text_part(name: &str) -> bool {
    name == "word/document.xml"
        || name == "word/footnotes.xml"
        || name == "word/endnotes.xml"
        || (name.starts_with("word/header") && name.ends_with(".xml"))
        || (name.starts_with("word/footer") && name.ends_with(".xml"))
}

fn read_zip_entry(
    bytes: &[u8],
    local_header_offset: usize,
    compression: u16,
    compressed_size: usize,
) -> Option<Vec<u8>> {
    if local_header_offset + 30 > bytes.len() || le_u32(bytes, local_header_offset)? != 0x0403_4b50
    {
        return None;
    }
    let name_len = le_u16(bytes, local_header_offset + 26)? as usize;
    let extra_len = le_u16(bytes, local_header_offset + 28)? as usize;
    let data_start = local_header_offset
        .checked_add(30)?
        .checked_add(name_len)?
        .checked_add(extra_len)?;
    let data_end = data_start.checked_add(compressed_size)?;
    if data_end > bytes.len() {
        return None;
    }
    let payload = &bytes[data_start..data_end];
    match compression {
        0 => Some(payload.to_vec()),
        8 => {
            let mut decoder = DeflateDecoder::new(Cursor::new(payload));
            let mut out = Vec::new();
            decoder.read_to_end(&mut out).ok()?;
            Some(out)
        }
        _ => None,
    }
}

fn find_zip_eocd(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    let earliest = bytes.len().saturating_sub(22 + 65_535);
    (earliest..=bytes.len() - 22)
        .rev()
        .find(|offset| le_u32(bytes, *offset) == Some(0x0605_4b50))
}

fn le_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

fn le_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn decode_xml_text(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn summarize_text(text: &str) -> String {
    let summary = text
        .split_whitespace()
        .take(60)
        .collect::<Vec<_>>()
        .join(" ");
    if summary.len() < text.len() {
        format!("{summary}...")
    } else {
        summary
    }
}

fn chunk_text(document_id: &str, text: &str) -> Vec<ExtractedTextChunk> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_start = 0usize;
    let mut current_end = 0usize;
    let mut cursor = 0usize;
    let mut index = 1;
    for line in text.split_inclusive('\n') {
        if current.len() + line.len() > 1800 && !current.is_empty() {
            let (chunk_text, byte_start, byte_end, char_start, char_end) =
                trim_offsets(text, &current, current_start, current_end);
            chunks.push(ExtractedTextChunk {
                chunk_id: format!("chunk:{document_id}:{index}"),
                document_id: document_id.to_string(),
                page: index,
                text: chunk_text,
                document_version_id: None,
                object_blob_id: None,
                source_span_id: None,
                byte_start: Some(byte_start),
                byte_end: Some(byte_end),
                char_start: Some(char_start),
                char_end: Some(char_end),
            });
            current.clear();
            index += 1;
        }
        if current.is_empty() {
            current_start = cursor;
        }
        current.push_str(line);
        cursor += line.len();
        current_end = cursor;
    }
    if !current.trim().is_empty() {
        let (chunk_text, byte_start, byte_end, char_start, char_end) =
            trim_offsets(text, &current, current_start, current_end);
        chunks.push(ExtractedTextChunk {
            chunk_id: format!("chunk:{document_id}:{index}"),
            document_id: document_id.to_string(),
            page: index,
            text: chunk_text,
            document_version_id: None,
            object_blob_id: None,
            source_span_id: None,
            byte_start: Some(byte_start),
            byte_end: Some(byte_end),
            char_start: Some(char_start),
            char_end: Some(char_end),
        });
    }
    chunks
}

fn trim_offsets(
    text: &str,
    current: &str,
    start: usize,
    end: usize,
) -> (String, u64, u64, u64, u64) {
    let leading = current.len() - current.trim_start().len();
    let trailing = current.len() - current.trim_end().len();
    let byte_start = start + leading;
    let byte_end = end.saturating_sub(trailing);
    let chunk_text = text
        .get(byte_start..byte_end)
        .unwrap_or_else(|| current.trim())
        .to_string();
    (
        chunk_text,
        byte_start as u64,
        byte_end as u64,
        text[..byte_start].chars().count() as u64,
        text[..byte_end].chars().count() as u64,
    )
}

fn propose_facts(
    matter_id: &str,
    document_id: &str,
    text: &str,
    context: &SourceContext,
) -> Vec<CaseFact> {
    sentence_candidates_with_offsets(text)
        .into_iter()
        .take(24)
        .enumerate()
        .map(|(index, sentence)| {
            let ordinal = index as u64 + 1;
            let fact_id = format!("fact:{}:{}", sanitize_path_segment(document_id), ordinal);
            let source_span =
                source_span_for_sentence(matter_id, document_id, ordinal, &sentence, context);
            CaseFact {
                id: fact_id.clone(),
                fact_id,
                matter_id: matter_id.to_string(),
                statement: sentence.text.clone(),
                text: sentence.text,
                status: "proposed".to_string(),
                confidence: 0.55,
                date: None,
                party_id: None,
                source_document_ids: vec![document_id.to_string()],
                source_evidence_ids: Vec::new(),
                contradicted_by_evidence_ids: Vec::new(),
                supports_claim_ids: Vec::new(),
                supports_defense_ids: Vec::new(),
                used_in_draft_ids: Vec::new(),
                needs_verification: true,
                source_spans: vec![source_span],
                notes: Some(
                    "Deterministic V0 extraction from document text; user review required."
                        .to_string(),
                ),
            }
        })
        .collect()
}

fn sentence_candidates_with_offsets(text: &str) -> Vec<SentenceCandidate> {
    let mut candidates = Vec::new();
    let mut current = String::new();
    let mut sentence_start = 0usize;
    let mut cursor = 0usize;

    for ch in text.chars() {
        if current.is_empty() {
            sentence_start = cursor;
        }
        current.push(ch);
        cursor += ch.len_utf8();
        if matches!(ch, '.' | '?' | '!' | '\n') {
            push_sentence_candidate(&mut candidates, text, &current, sentence_start, cursor);
            current.clear();
        }
    }
    push_sentence_candidate(&mut candidates, text, &current, sentence_start, cursor);
    candidates
}

fn push_sentence_candidate(
    candidates: &mut Vec<SentenceCandidate>,
    full_text: &str,
    sentence: &str,
    start: usize,
    end: usize,
) {
    let leading = sentence.len() - sentence.trim_start().len();
    let trailing = sentence.len() - sentence.trim_end().len();
    let byte_start = start + leading;
    let byte_end = end.saturating_sub(trailing);
    let cleaned = full_text
        .get(byte_start..byte_end)
        .unwrap_or_else(|| sentence.trim())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.len() < 35 || cleaned.len() > 400 {
        return;
    }
    if !cleaned.chars().any(|ch| ch.is_ascii_alphabetic()) {
        return;
    }
    if cleaned.ends_with(':') {
        return;
    }
    if !candidates.iter().any(|existing| existing.text == cleaned) {
        candidates.push(SentenceCandidate {
            text: cleaned,
            byte_start: byte_start as u64,
            byte_end: byte_end as u64,
            char_start: full_text[..byte_start].chars().count() as u64,
            char_end: full_text[..byte_end].chars().count() as u64,
        });
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn push_authority(values: &mut Vec<AuthorityRef>, value: AuthorityRef) {
    if !values
        .iter()
        .any(|existing| same_authority(existing, &value))
    {
        values.push(value);
    }
}

fn remove_authority(values: &mut Vec<AuthorityRef>, value: &AuthorityRef) {
    values.retain(|existing| !same_authority(existing, value));
}

fn same_authority(left: &AuthorityRef, right: &AuthorityRef) -> bool {
    if !left.canonical_id.is_empty() && !right.canonical_id.is_empty() {
        left.canonical_id == right.canonical_id
    } else {
        left.citation == right.citation
    }
}

fn count_words(paragraphs: &[DraftParagraph], sections: &[DraftSection]) -> u64 {
    paragraphs
        .iter()
        .map(|paragraph| paragraph.text.split_whitespace().count() as u64)
        .sum::<u64>()
        + sections
            .iter()
            .map(|section| section.body.split_whitespace().count() as u64)
            .sum::<u64>()
}

fn io_error(error: std::io::Error) -> ApiError {
    ApiError::Internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        apply_ast_operation, canonical_id_for_citation, chunk_text, citation_uses_for_text,
        default_formatting_profile, diff_work_product_layers, failed_ingestion_run,
        generate_opaque_id, looks_like_complaint, markdown_to_work_product_ast,
        normalize_compare_layers, object_blob_id_for_hash, oregon_civil_complaint_rule_pack,
        parse_complaint_structure, parse_document_bytes, propose_facts, prosemirror_doc_for_text,
        refresh_work_product_state, restore_work_product_scope,
        safe_work_product_download_filename, sanitize_filename, sha256_hex, should_inline_payload,
        slug, snapshot_entity_state_key, snapshot_full_state_key, snapshot_manifest_for_product,
        snapshot_manifest_hash_for_states, snapshot_manifest_key,
        summarize_version_snapshot_for_list, summarize_work_product_for_list,
        validate_ast_patch_concurrency, validate_work_product_document,
        version_change_state_summary, work_product_block_graph_payload, work_product_export_key,
        work_product_finding, work_product_hashes, work_product_profile, SourceContext,
        CASE_INDEX_VERSION, CHUNKER_VERSION, CITATION_RESOLVER_VERSION, PARSER_REGISTRY_VERSION,
    };
    use crate::error::ApiError;
    use crate::models::casebuilder::{
        AstOperation, AstPatch, IngestionRun, VersionChangeSummary, VersionSnapshot, WorkProduct,
        WorkProductAction, WorkProductArtifact, WorkProductBlock, WorkProductCitationUse,
        WorkProductDocument, WorkProductDownloadResponse, WorkProductExhibitReference,
        WorkProductFinding, WorkProductLink,
    };
    use crate::services::object_store::build_document_object_key;
    use std::collections::BTreeMap;

    #[test]
    fn sanitizes_file_names_to_local_paths() {
        assert_eq!(
            sanitize_filename("../secret motion.txt"),
            "secret_motion.txt"
        );
        assert_eq!(sanitize_filename("Lease 4B.pdf"), "Lease_4B.pdf");
    }

    #[test]
    fn hashes_uploaded_content_with_sha256_prefix() {
        let hash = sha256_hex(b"case text");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), "sha256:".len() + 64);
    }

    #[test]
    fn work_product_hashes_are_stable_and_layered() {
        let base = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let same = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let text_edit =
            test_work_product("Plaintiff timely paid rent.", Vec::new(), Vec::new(), None);
        let support_edit = test_work_product(
            "Plaintiff paid rent.",
            vec!["fact:rent".to_string()],
            vec!["evidence:receipt".to_string()],
            None,
        );
        let qc_edit = test_work_product(
            "Plaintiff paid rent.",
            Vec::new(),
            Vec::new(),
            Some("Unsupported allegation"),
        );
        let mut format_edit = base.clone();
        format_edit.formatting_profile.double_spaced =
            !format_edit.formatting_profile.double_spaced;

        let base_hashes = work_product_hashes(&base).unwrap();
        let same_hashes = work_product_hashes(&same).unwrap();
        assert_eq!(base_hashes.document_hash, same_hashes.document_hash);
        assert_eq!(
            base_hashes.support_graph_hash,
            same_hashes.support_graph_hash
        );
        assert_eq!(base_hashes.qc_state_hash, same_hashes.qc_state_hash);
        assert_eq!(base_hashes.formatting_hash, same_hashes.formatting_hash);

        assert_ne!(
            base_hashes.document_hash,
            work_product_hashes(&text_edit).unwrap().document_hash
        );
        assert_ne!(
            base_hashes.support_graph_hash,
            work_product_hashes(&support_edit)
                .unwrap()
                .support_graph_hash
        );
        assert_ne!(
            base_hashes.qc_state_hash,
            work_product_hashes(&qc_edit).unwrap().qc_state_hash
        );
        assert_ne!(
            base_hashes.formatting_hash,
            work_product_hashes(&format_edit).unwrap().formatting_hash
        );
    }

    #[test]
    fn work_product_ast_validation_rejects_duplicate_block_ids() {
        let mut product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let duplicate = product.document_ast.blocks[0].clone();
        product.document_ast.blocks.push(duplicate);

        let validation = validate_work_product_document(&product);

        assert!(!validation.valid);
        assert!(validation
            .errors
            .iter()
            .any(|issue| issue.code == "duplicate_block_id"));
    }

    #[test]
    fn ast_patch_updates_block_text_in_place() {
        let mut product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let block_id = product.document_ast.blocks[0].block_id.clone();

        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::UpdateBlock {
                block_id: block_id.clone(),
                before: None,
                after: serde_json::json!({
                    "text": "Plaintiff timely paid rent.",
                    "title": "Paragraph 1"
                }),
            },
        )
        .expect("patch applies");

        assert_eq!(product.document_ast.blocks[0].block_id, block_id);
        assert_eq!(
            product.document_ast.blocks[0].text,
            "Plaintiff timely paid rent."
        );
    }

    #[test]
    fn ast_rule_finding_patch_survives_projection_refresh() {
        let mut product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);
        let block_id = product.document_ast.blocks[0].block_id.clone();
        let finding = work_product_finding(
            &product,
            "support-required",
            "support",
            "warning",
            "paragraph",
            &block_id,
            "Paragraph needs support.",
            "Factual allegations should point to support.",
            "Link support.",
            "2026-01-01T00:00:00Z",
        );
        let finding_id = finding.finding_id.clone();

        apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddRuleFinding { finding },
        )
        .expect("finding patch applies");
        refresh_work_product_state(&mut product);

        assert!(product
            .document_ast
            .rule_findings
            .iter()
            .any(|finding| finding.finding_id == finding_id));
        assert!(product
            .findings
            .iter()
            .any(|finding| finding.finding_id == finding_id));
    }

    #[test]
    fn markdown_conversion_creates_ast_blocks() {
        let product = test_work_product("Plaintiff paid rent.", Vec::new(), Vec::new(), None);

        let (document, warnings) = markdown_to_work_product_ast(
            &product,
            "## COUNT I - Breach of Contract\n\n1. Plaintiff paid rent.\n\n2. Defendant refused repairs.",
        );

        assert!(warnings.is_empty());
        assert_eq!(document.blocks.len(), 3);
        assert_eq!(document.blocks[0].block_type, "count");
        assert_eq!(document.blocks[1].block_type, "numbered_paragraph");
        assert_eq!(document.blocks[1].paragraph_number, Some(1));
    }

    fn test_work_product(
        text: &str,
        fact_ids: Vec<String>,
        evidence_ids: Vec<String>,
        qc_message: Option<&str>,
    ) -> WorkProduct {
        let block_id = "work-product:test:block:1".to_string();
        let mut product = WorkProduct {
            id: "work-product:test".to_string(),
            work_product_id: "work-product:test".to_string(),
            matter_id: "matter:test".to_string(),
            title: "Test complaint".to_string(),
            product_type: "complaint".to_string(),
            status: "draft".to_string(),
            review_status: "needs_human_review".to_string(),
            setup_stage: "editor".to_string(),
            source_draft_id: None,
            source_complaint_id: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            profile: work_product_profile("complaint"),
            document_ast: WorkProductDocument::default(),
            blocks: vec![WorkProductBlock {
                id: block_id.clone(),
                block_id: block_id.clone(),
                matter_id: "matter:test".to_string(),
                work_product_id: "work-product:test".to_string(),
                block_type: "paragraph".to_string(),
                role: "factual_allegation".to_string(),
                title: "Paragraph 1".to_string(),
                text: text.to_string(),
                ordinal: 1,
                parent_block_id: None,
                fact_ids,
                evidence_ids,
                authorities: Vec::new(),
                mark_ids: Vec::new(),
                locked: false,
                review_status: "needs_review".to_string(),
                prosemirror_json: Some(prosemirror_doc_for_text(text)),
                ..WorkProductBlock::default()
            }],
            marks: Vec::new(),
            anchors: Vec::new(),
            findings: qc_message
                .map(|message| {
                    vec![WorkProductFinding {
                        id: "finding:test".to_string(),
                        finding_id: "finding:test".to_string(),
                        matter_id: "matter:test".to_string(),
                        work_product_id: "work-product:test".to_string(),
                        rule_id: "support:required".to_string(),
                        category: "support".to_string(),
                        severity: "warning".to_string(),
                        target_type: "paragraph".to_string(),
                        target_id: block_id,
                        message: message.to_string(),
                        explanation: message.to_string(),
                        suggested_fix: "Link support.".to_string(),
                        primary_action: WorkProductAction {
                            action_id: "action:test".to_string(),
                            label: "Link support".to_string(),
                            action_type: "link_support".to_string(),
                            href: None,
                            target_type: "paragraph".to_string(),
                            target_id: "work-product:test:block:1".to_string(),
                        },
                        status: "open".to_string(),
                        created_at: "2026-01-01T00:00:00Z".to_string(),
                        updated_at: "2026-01-01T00:00:00Z".to_string(),
                    }]
                })
                .unwrap_or_default(),
            artifacts: Vec::new(),
            history: Vec::new(),
            ai_commands: Vec::new(),
            formatting_profile: default_formatting_profile(),
            rule_pack: oregon_civil_complaint_rule_pack(),
        };
        refresh_work_product_state(&mut product);
        product
    }

    fn test_version_snapshot(full_state_inline: Option<serde_json::Value>) -> VersionSnapshot {
        VersionSnapshot {
            id: "work-product:test:snapshot:1".to_string(),
            snapshot_id: "work-product:test:snapshot:1".to_string(),
            matter_id: "matter:test".to_string(),
            subject_type: "work_product".to_string(),
            subject_id: "work-product:test".to_string(),
            product_type: "complaint".to_string(),
            profile_id: "complaint".to_string(),
            branch_id: "work-product:test:branch:main".to_string(),
            sequence_number: 1,
            title: "Snapshot 1".to_string(),
            message: Some("Snapshot".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            created_by: "system".to_string(),
            actor_id: None,
            snapshot_type: "auto".to_string(),
            parent_snapshot_ids: Vec::new(),
            document_hash: "sha256:document".to_string(),
            support_graph_hash: "sha256:support".to_string(),
            qc_state_hash: "sha256:qc".to_string(),
            formatting_hash: "sha256:formatting".to_string(),
            manifest_hash: "sha256:manifest".to_string(),
            manifest_ref: None,
            full_state_ref: None,
            full_state_inline,
            summary: VersionChangeSummary {
                text_changes: 0,
                support_changes: 0,
                citation_changes: 0,
                authority_changes: 0,
                qc_changes: 0,
                export_changes: 0,
                ai_changes: 0,
                targets_changed: Vec::new(),
                risk_level: "low".to_string(),
                user_summary: "Snapshot".to_string(),
            },
        }
    }

    fn test_work_product_artifact(artifact_id: &str, artifact_hash: &str) -> WorkProductArtifact {
        WorkProductArtifact {
            artifact_id: artifact_id.to_string(),
            id: artifact_id.to_string(),
            matter_id: "matter:test".to_string(),
            work_product_id: "work-product:test".to_string(),
            format: "html".to_string(),
            profile: "review".to_string(),
            mode: "review_needed".to_string(),
            status: "generated_review_needed".to_string(),
            download_url: "/api/v1/download".to_string(),
            page_count: 1,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            warnings: vec!["review needed".to_string()],
            content_preview: "bounded preview".to_string(),
            snapshot_id: Some("snapshot:1".to_string()),
            artifact_hash: Some(artifact_hash.to_string()),
            render_profile_hash: Some("sha256:render".to_string()),
            qc_status_at_export: Some("needs_review".to_string()),
            changed_since_export: Some(false),
            immutable: Some(true),
            object_blob_id: Some("blob:sha256:artifact".to_string()),
            size_bytes: Some(42),
            mime_type: Some("text/html".to_string()),
            storage_status: Some("stored".to_string()),
        }
    }

    #[test]
    fn chunks_text_without_empty_chunks() {
        let chunks = chunk_text("doc:1", "first\nsecond");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "first\nsecond");
        assert_eq!(chunks[0].byte_start, Some(0));
        assert_eq!(
            chunks[0].char_end,
            Some("first\nsecond".chars().count() as u64)
        );
    }

    #[test]
    fn slug_has_stable_prefix_safe_shape() {
        assert_eq!(
            slug("Smith v. ABC Property Management"),
            "smith-v-abc-property-management"
        );
    }

    #[test]
    fn duplicate_hashes_share_object_blob_identity() {
        let first = object_blob_id_for_hash(
            "sha256:ABCDEFabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234",
        );
        let second = object_blob_id_for_hash(
            "abcdefabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234",
        );
        assert_eq!(first, second);
        assert!(first.starts_with("blob:sha256:"));
    }

    #[test]
    fn new_document_object_keys_do_not_include_raw_filenames() {
        let document_id = generate_opaque_id("doc");
        let key = build_document_object_key(&document_id, "../Private Tenant Notice.pdf");
        assert!(!document_id.contains("tenant"));
        assert!(!document_id.contains("notice"));
        assert!(!key.contains("Private"));
        assert!(!key.contains("Tenant"));
        assert!(key.ends_with("/original.pdf"));
    }

    #[test]
    fn ast_storage_policy_inlines_only_within_threshold() {
        assert!(should_inline_payload(64 * 1024, 64 * 1024));
        assert!(!should_inline_payload(64 * 1024 + 1, 64 * 1024));
    }

    #[test]
    fn ast_artifact_object_keys_are_hash_scoped() {
        let matter_id = "matter:Smith v. ABC Property:123";
        let work_product_id = "work-product:Private Motion:456";
        let snapshot_id = "work-product:Private Motion:456:snapshot:1";
        let key = snapshot_full_state_key(
            matter_id,
            work_product_id,
            snapshot_id,
            "sha256:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd",
        );
        assert!(key.starts_with("casebuilder/matters/"));
        assert!(!key.contains("Smith"));
        assert!(!key.contains("Private"));
        assert!(!key.contains("Motion"));
        assert!(key.ends_with(".json"));

        let manifest_key = snapshot_manifest_key(
            matter_id,
            work_product_id,
            snapshot_id,
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert!(!manifest_key.contains("Smith"));
        assert!(!manifest_key.contains("Private"));
        assert!(manifest_key.ends_with(".json"));

        let state_key = snapshot_entity_state_key(
            matter_id,
            work_product_id,
            snapshot_id,
            "document_ast",
            "sha256:0123456789abcdef",
        );
        assert!(state_key.contains("/states/document_ast/0123456789abcdef.json"));

        let export_key = work_product_export_key(
            matter_id,
            work_product_id,
            "artifact:Secret Draft:1",
            "sha256:fedcba",
            "html",
        );
        assert!(!export_key.contains("Secret"));
        assert!(export_key.ends_with("/fedcba.html"));
    }

    #[test]
    fn snapshot_manifest_hash_survives_state_object_offload() {
        let product = test_work_product(
            "Confidential allegation that belongs only in snapshot state.",
            Vec::new(),
            Vec::new(),
            None,
        );
        let (mut manifest, mut states) = snapshot_manifest_for_product(
            "matter:test",
            "work-product:test:snapshot:1",
            &product,
            "2026-01-01T00:00:00Z",
        )
        .expect("manifest");
        let before_hash = manifest.manifest_hash.clone();

        for state in &mut states {
            state.state_inline = None;
            state.state_ref = Some(object_blob_id_for_hash(&state.entity_hash));
        }
        manifest.storage_ref = Some(object_blob_id_for_hash(&before_hash));

        assert_eq!(
            before_hash,
            snapshot_manifest_hash_for_states(&states).expect("hash")
        );
        assert!(states.iter().all(|state| state
            .state_ref
            .as_deref()
            .is_some_and(|value| value.starts_with("blob:sha256:"))));
    }

    #[test]
    fn version_change_payloads_store_hash_summaries_not_document_text() {
        let value = serde_json::json!({
            "document_ast": {
                "blocks": [{
                    "block_id": "block:1",
                    "text": "Secret merits analysis should not be duplicated in VersionChange."
                }]
            }
        });

        let (hash, summary) = version_change_state_summary(Some(value)).expect("summary");
        let summary_text = serde_json::to_string(&summary).expect("json");

        assert!(hash
            .as_deref()
            .is_some_and(|value| value.starts_with("sha256:")));
        assert!(summary_text.contains("\"state_storage\":\"version_snapshot\""));
        assert!(summary_text.contains("\"inline_payload\":false"));
        assert!(!summary_text.contains("Secret merits analysis"));
    }

    #[test]
    fn stale_ast_patch_base_document_hash_is_rejected_without_text() {
        let patch = AstPatch {
            patch_id: "patch:test".to_string(),
            draft_id: None,
            work_product_id: Some("work-product:test".to_string()),
            base_document_hash: Some("sha256:stale".to_string()),
            base_snapshot_id: None,
            created_by: "user".to_string(),
            reason: Some("Secret factual edit".to_string()),
            operations: Vec::new(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let error = validate_ast_patch_concurrency(
            &patch,
            "work-product:test",
            "sha256:current",
            Some("work-product:test:snapshot:2"),
        )
        .expect_err("stale hash is rejected");

        match error {
            ApiError::Conflict(message) => {
                assert!(!message.contains("patch_id=patch:test"));
                assert!(message.contains("base_document_hash=sha256:stale"));
                assert!(message.contains("current_document_hash=sha256:current"));
                assert!(!message.contains("Secret factual edit"));
            }
            other => panic!("expected conflict, got {other:?}"),
        }
    }

    #[test]
    fn ast_patch_reference_errors_do_not_echo_legal_text_or_ids() {
        let mut product = test_work_product(
            "Secret factual edit should stay out of error messages.",
            Vec::new(),
            Vec::new(),
            None,
        );
        let error = apply_ast_operation(
            &mut product.document_ast,
            &AstOperation::AddLink {
                link: WorkProductLink {
                    link_id: "link:1".to_string(),
                    source_block_id: "block:Secret Missing Block".to_string(),
                    source_text_range: None,
                    target_type: "fact".to_string(),
                    target_id: "fact:Secret Tenant Payment".to_string(),
                    relation: "supports".to_string(),
                    confidence: None,
                    created_by: "user".to_string(),
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                },
            },
        )
        .expect_err("missing source block is rejected");

        match error {
            ApiError::NotFound(message) => {
                assert_eq!(message, "AST link source block not found");
                assert!(!message.contains("Secret"));
                assert!(!message.contains("Tenant"));
                assert!(!message.contains("Payment"));
            }
            other => panic!("expected not found, got {other:?}"),
        }
    }

    #[test]
    fn compare_layers_cover_legal_ast_without_copying_sensitive_payloads() {
        let base = test_work_product(
            "Confidential base allegation should not appear in layer diffs.",
            Vec::new(),
            Vec::new(),
            None,
        );
        let mut changed = base.clone();
        let block_id = changed.document_ast.blocks[0].block_id.clone();
        changed.document_ast.links.push(WorkProductLink {
            link_id: "link:1".to_string(),
            source_block_id: block_id.clone(),
            source_text_range: None,
            target_type: "fact".to_string(),
            target_id: "fact:secret-payment-detail".to_string(),
            relation: "supports".to_string(),
            confidence: Some(0.9),
            created_by: "user".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        });
        changed.document_ast.blocks[0]
            .links
            .push("link:1".to_string());
        changed.document_ast.citations.push(WorkProductCitationUse {
            citation_use_id: "citation:1".to_string(),
            source_block_id: block_id.clone(),
            source_text_range: None,
            raw_text: "SECRET RAW CITATION TEXT".to_string(),
            normalized_citation: Some("ORS 90.320".to_string()),
            target_type: "provision".to_string(),
            target_id: Some("or:ors:90.320".to_string()),
            pinpoint: Some("SECRET PINPOINT".to_string()),
            status: "resolved".to_string(),
            resolver_message: Some("SECRET RESOLVER MESSAGE".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        });
        changed.document_ast.blocks[0]
            .citations
            .push("citation:1".to_string());
        changed
            .document_ast
            .exhibits
            .push(WorkProductExhibitReference {
                exhibit_reference_id: "exhibit:1".to_string(),
                source_block_id: block_id.clone(),
                source_text_range: None,
                label: "SECRET EXHIBIT LABEL".to_string(),
                exhibit_id: Some("evidence:secret-exhibit".to_string()),
                document_id: None,
                page_range: Some("1-2".to_string()),
                status: "resolved".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            });
        changed.document_ast.blocks[0]
            .exhibits
            .push("exhibit:1".to_string());
        let finding = work_product_finding(
            &changed,
            "support-required",
            "support",
            "warning",
            "paragraph",
            &block_id,
            "SECRET FINDING MESSAGE",
            "SECRET FINDING EXPLANATION",
            "SECRET FINDING FIX",
            "2026-01-01T00:00:00Z",
        );
        changed.document_ast.rule_findings.push(finding.clone());
        changed.findings.push(finding);
        changed.formatting_profile.double_spaced = !changed.formatting_profile.double_spaced;
        changed.artifacts.push(WorkProductArtifact {
            artifact_id: "artifact:1".to_string(),
            id: "artifact:1".to_string(),
            matter_id: changed.matter_id.clone(),
            work_product_id: changed.work_product_id.clone(),
            format: "html".to_string(),
            profile: "review".to_string(),
            mode: "review_needed".to_string(),
            status: "generated_review_needed".to_string(),
            download_url: "/api/v1/download".to_string(),
            page_count: 1,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            warnings: vec!["review needed".to_string()],
            content_preview: "SECRET EXPORT PREVIEW".to_string(),
            snapshot_id: Some("snapshot:1".to_string()),
            artifact_hash: Some("sha256:artifact".to_string()),
            render_profile_hash: Some("sha256:render".to_string()),
            qc_status_at_export: Some("needs_review".to_string()),
            changed_since_export: Some(false),
            immutable: Some(true),
            object_blob_id: Some("blob:sha256:artifact".to_string()),
            size_bytes: Some(512),
            mime_type: Some("text/html".to_string()),
            storage_status: Some("stored".to_string()),
        });

        let diffs = diff_work_product_layers(
            &base,
            &changed,
            &normalize_compare_layers(vec!["all".to_string()]),
        )
        .expect("layer diff");
        for layer in [
            "support",
            "citations",
            "exhibits",
            "rule_findings",
            "formatting",
            "exports",
        ] {
            assert!(
                diffs.iter().any(|diff| diff.layer == layer),
                "missing {layer} diff"
            );
        }
        let text = serde_json::to_string(&diffs).expect("diffs serialize");
        for forbidden in [
            "SECRET RAW CITATION TEXT",
            "SECRET EXHIBIT LABEL",
            "SECRET FINDING MESSAGE",
            "SECRET EXPORT PREVIEW",
            "Confidential base allegation",
        ] {
            assert!(!text.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[test]
    fn scoped_restore_block_preserves_unrelated_current_ast_edits() {
        let mut snapshot = test_work_product("Snapshot block one.", Vec::new(), Vec::new(), None);
        let mut current = snapshot.clone();
        let target_id = snapshot.document_ast.blocks[0].block_id.clone();
        let mut second_snapshot = snapshot.document_ast.blocks[0].clone();
        second_snapshot.block_id = "work-product:test:block:2".to_string();
        second_snapshot.id = second_snapshot.block_id.clone();
        second_snapshot.text = "Snapshot block two.".to_string();
        second_snapshot.ordinal = 2;
        snapshot.document_ast.blocks.push(second_snapshot.clone());
        refresh_work_product_state(&mut snapshot);

        current.document_ast.blocks[0].text = "Current block one edit.".to_string();
        let mut second_current = second_snapshot;
        second_current.text = "Current unrelated block two edit.".to_string();
        current.document_ast.blocks.push(second_current);
        refresh_work_product_state(&mut current);

        let (restored, warnings) =
            restore_work_product_scope(&current, &snapshot, "block", &[target_id.clone()])
                .expect("restore block");

        assert!(warnings
            .iter()
            .any(|warning| warning.contains("Targeted restore")));
        assert_eq!(restored.document_ast.blocks[0].text, "Snapshot block one.");
        assert_eq!(
            restored.document_ast.blocks[1].text,
            "Current unrelated block two edit."
        );
    }

    #[test]
    fn scoped_restore_export_state_merges_without_deleting_newer_artifacts() {
        let mut snapshot = test_work_product("Snapshot.", Vec::new(), Vec::new(), None);
        let mut current = snapshot.clone();
        snapshot.artifacts.push(test_work_product_artifact(
            "artifact:snapshot",
            "sha256:snapshot",
        ));
        current.artifacts.push(test_work_product_artifact(
            "artifact:current",
            "sha256:current",
        ));

        let (restored, _warnings) =
            restore_work_product_scope(&current, &snapshot, "export_state", &[])
                .expect("restore export state");

        assert!(restored
            .artifacts
            .iter()
            .any(|artifact| artifact.artifact_id == "artifact:snapshot"));
        assert!(restored
            .artifacts
            .iter()
            .any(|artifact| artifact.artifact_id == "artifact:current"));
    }

    #[test]
    fn work_product_download_response_omits_storage_keys_and_safe_filename_omits_titles() {
        let artifact = test_work_product_artifact("artifact:opaque", "sha256:artifact");
        let response = WorkProductDownloadResponse {
            method: "GET".to_string(),
            url: "/api/v1/matters/matter/work-products/work-product/artifacts/artifact/download"
                .to_string(),
            expires_at: "2".to_string(),
            headers: BTreeMap::new(),
            filename: safe_work_product_download_filename(&artifact),
            mime_type: Some("text/html".to_string()),
            bytes: 42,
            artifact,
        };
        let text = serde_json::to_string(&response).expect("response serializes");
        assert!(!text.contains("storage_key"));
        assert!(!text.contains("casebuilder/matters/private/raw-key"));

        let sensitive_artifact = test_work_product_artifact(
            "artifact:Secret Draft About Tenant Payment",
            "sha256:artifact",
        );
        let filename = safe_work_product_download_filename(&sensitive_artifact);
        assert!(!filename.contains("Secret"));
        assert!(!filename.contains("Tenant"));
        assert!(!filename.contains("Payment"));
        assert!(filename.starts_with("work-product-export-"));
    }

    #[test]
    fn work_product_list_summary_omits_ast_payload_by_default() {
        let mut summary = test_work_product(
            "Long legal document body should not ride along in list responses.",
            Vec::new(),
            Vec::new(),
            None,
        );

        summarize_work_product_for_list(&mut summary);

        assert!(summary.blocks.is_empty());
        assert!(summary.document_ast.blocks.is_empty());
        assert!(summary.document_ast.links.is_empty());
        assert_eq!(summary.document_ast.title, "Test complaint");
    }

    #[test]
    fn large_work_product_list_summary_omits_large_ast_by_default() {
        let mut summary = test_work_product(
            "Large legal document body should not ride along in list responses.",
            Vec::new(),
            Vec::new(),
            None,
        );
        let template = summary.document_ast.blocks[0].clone();
        for index in 2..=750 {
            let mut block = template.clone();
            block.block_id = format!("work-product:test:block:{index}");
            block.id = block.block_id.clone();
            block.ordinal = index;
            block.text = format!("Large confidential paragraph {index}.");
            summary.document_ast.blocks.push(block);
        }
        refresh_work_product_state(&mut summary);

        summarize_work_product_for_list(&mut summary);

        assert!(summary.blocks.is_empty());
        assert!(summary.document_ast.blocks.is_empty());
        assert_eq!(summary.document_ast.title, "Test complaint");
    }

    #[test]
    fn snapshot_list_summary_omits_inline_full_state() {
        let mut snapshot = test_version_snapshot(Some(serde_json::json!({
            "document_ast": {
                "blocks": [{
                    "text": "Snapshot detail text should not appear in snapshot lists."
                }]
            }
        })));
        snapshot.full_state_ref = Some("blob:sha256:abcdef".to_string());

        summarize_version_snapshot_for_list(&mut snapshot);

        assert!(snapshot.full_state_inline.is_none());
        assert_eq!(
            snapshot.full_state_ref.as_deref(),
            Some("blob:sha256:abcdef")
        );
    }

    #[test]
    fn oversized_block_graph_payload_keeps_excerpt_and_hash() {
        let product = test_work_product(&"x".repeat(80), Vec::new(), Vec::new(), None);
        let block = &product.blocks[0];
        let payload = work_product_block_graph_payload(block, 8).expect("payload");
        let value: serde_json::Value = serde_json::from_str(&payload).expect("json payload");
        assert_eq!(value["text_storage_status"], "graph_excerpt");
        assert_eq!(value["text_size_bytes"], 80);
        assert!(value["text_hash"]
            .as_str()
            .expect("hash")
            .starts_with("sha256:"));
    }

    #[test]
    fn proposed_facts_get_source_spans() {
        let context = SourceContext {
            document_version_id: Some("version:doc_opaque:original".to_string()),
            object_blob_id: Some("blob:sha256:abc".to_string()),
            ingestion_run_id: Some("ingestion:doc_opaque:primary".to_string()),
        };
        let facts = propose_facts(
            "matter:test",
            "doc:opaque",
            "The tenant paid rent on March 1, 2024, and the landlord accepted the payment without objection.",
            &context,
        );
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].source_spans.len(), 1);
        assert_eq!(
            facts[0].source_spans[0].document_version_id.as_deref(),
            Some("version:doc_opaque:original")
        );
        assert_eq!(
            facts[0].source_spans[0].quote.as_deref(),
            Some("The tenant paid rent on March 1, 2024, and the landlord accepted the payment without objection.")
        );
    }

    #[test]
    fn failed_extraction_marks_ingestion_run_failed() {
        let run = IngestionRun {
            ingestion_run_id: "ingestion:doc:primary".to_string(),
            id: "ingestion:doc:primary".to_string(),
            matter_id: "matter:test".to_string(),
            document_id: "doc:test".to_string(),
            document_version_id: Some("version:doc:original".to_string()),
            object_blob_id: Some("blob:sha256:abc".to_string()),
            input_sha256: Some("sha256:abc".to_string()),
            status: "stored".to_string(),
            stage: "stored".to_string(),
            mode: "deterministic".to_string(),
            started_at: "1".to_string(),
            completed_at: None,
            error_code: None,
            error_message: None,
            retryable: false,
            produced_node_ids: Vec::new(),
            produced_object_keys: Vec::new(),
            parser_id: Some("casebuilder-parser-registry".to_string()),
            parser_version: Some(PARSER_REGISTRY_VERSION.to_string()),
            chunker_version: Some(CHUNKER_VERSION.to_string()),
            citation_resolver_version: Some(CITATION_RESOLVER_VERSION.to_string()),
            index_version: Some(CASE_INDEX_VERSION.to_string()),
        };
        let failed =
            failed_ingestion_run(&run, "extract_text", "no_extractable_text", "empty", false);
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.stage, "extract_text");
        assert_eq!(failed.error_code.as_deref(), Some("no_extractable_text"));
        assert!(!failed.retryable);
        assert!(failed.completed_at.is_some());
    }

    #[test]
    fn parser_registry_extracts_supported_text_and_flags_deferred_media() {
        let markdown = parse_document_bytes(
            "complaint.md",
            Some("text/markdown"),
            b"# Complaint\n1. Plaintiff cites ORS 90.100.",
        );
        assert_eq!(markdown.status, "processed");
        assert!(markdown.text.unwrap().contains("Plaintiff cites"));

        let html = parse_document_bytes(
            "notice.html",
            Some("text/html"),
            b"<main><p>Tenant paid &amp; landlord accepted.</p></main>",
        );
        assert_eq!(
            html.text.as_deref(),
            Some("Tenant paid & landlord accepted.")
        );

        let docx = stored_zip_document_xml(
	            br#"<w:document><w:body><w:p><w:r><w:t>Complaint paragraph from DOCX.</w:t></w:r></w:p></w:body></w:document>"#,
	        );
        let docx = parse_document_bytes(
            "complaint.docx",
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
            &docx,
        );
        assert_eq!(docx.status, "processed");
        assert!(docx
            .text
            .unwrap()
            .contains("Complaint paragraph from DOCX."));

        let pdf = parse_document_bytes(
            "embedded.pdf",
            Some("application/pdf"),
            b"%PDF-1.4\nBT (Plaintiff paid rent on March 1, 2026.) Tj ET",
        );
        assert_eq!(pdf.status, "processed");
        assert!(pdf.text.unwrap().contains("Plaintiff paid rent"));

        let image = parse_document_bytes("scan.png", Some("image/png"), b"not text");
        assert_eq!(image.status, "ocr_required");
        assert!(image.text.is_none());

        let media = parse_document_bytes("hearing.mp4", Some("video/mp4"), b"not text");
        assert_eq!(media.status, "transcription_deferred");
    }

    fn stored_zip_document_xml(xml: &[u8]) -> Vec<u8> {
        let name = b"word/document.xml";
        let mut zip = Vec::new();
        push_u32(&mut zip, 0x0403_4b50);
        push_u16(&mut zip, 20);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u32(&mut zip, 0);
        push_u32(&mut zip, xml.len() as u32);
        push_u32(&mut zip, xml.len() as u32);
        push_u16(&mut zip, name.len() as u16);
        push_u16(&mut zip, 0);
        zip.extend_from_slice(name);
        zip.extend_from_slice(xml);

        let central_directory_offset = zip.len();
        push_u32(&mut zip, 0x0201_4b50);
        push_u16(&mut zip, 20);
        push_u16(&mut zip, 20);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u32(&mut zip, 0);
        push_u32(&mut zip, xml.len() as u32);
        push_u32(&mut zip, xml.len() as u32);
        push_u16(&mut zip, name.len() as u16);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u32(&mut zip, 0);
        push_u32(&mut zip, 0);
        zip.extend_from_slice(name);

        let central_directory_size = zip.len() - central_directory_offset;
        push_u32(&mut zip, 0x0605_4b50);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 1);
        push_u16(&mut zip, 1);
        push_u32(&mut zip, central_directory_size as u32);
        push_u32(&mut zip, central_directory_offset as u32);
        push_u16(&mut zip, 0);
        zip
    }

    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn complaint_import_parser_preserves_labels_counts_and_citations() {
        let text = r#"
# PAYNTER v. BLUE OX RV PARK - MASTER CIVIL COMPLAINT
## CAPTION
**IN THE CIRCUIT COURT OF THE STATE OF OREGON**
**FOR THE COUNTY OF LINN**

## I. FACTUAL ALLEGATIONS
1. Plaintiffs began occupying Space #076 on October 27, 2025.
10A. Plaintiff Debra Paynter is an elderly person under ORS 124.100(1)(a).

### FIRST CLAIM FOR RELIEF - RETALIATION
42. Defendants retaliated in violation of ORS 90.385 and ORCP 16 D.

## PRAYER FOR RELIEF
43. Plaintiffs request injunctive relief under UTCR 2.010.
"#;
        assert!(looks_like_complaint("master_complaint.md", text));
        let parsed = parse_complaint_structure(text);
        assert_eq!(parsed.paragraphs.len(), 4);
        assert_eq!(parsed.paragraphs[1].original_label, "10A");
        assert_eq!(parsed.counts.len(), 1);

        let citations = citation_uses_for_text(
            "matter:test",
            "complaint:test",
            "complaint:test:paragraph:2",
            "paragraph",
            &parsed.paragraphs[1].text,
        );
        assert_eq!(citations[0].canonical_id.as_deref(), Some("or:ors:124.100"));

        let rule_citations = citation_uses_for_text(
            "matter:test",
            "complaint:test",
            "complaint:test:paragraph:4",
            "paragraph",
            &parsed.paragraphs[3].text,
        );
        assert_eq!(rule_citations[0].status, "resolved");
        assert_eq!(
            rule_citations[0].canonical_id.as_deref(),
            Some("or:utcr:2.010")
        );
    }

    #[test]
    fn citation_canonical_ids_cover_ors_orcp_and_utcr() {
        assert_eq!(
            canonical_id_for_citation("ORS 90.385"),
            Some("or:ors:90.385".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("ORCP 16 D"),
            Some("or:orcp:16_d".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("UTCR 2.010(4)"),
            Some("or:utcr:2.010".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("Or Laws 2023, ch 13, § 4"),
            Some("or:session-law:2023:ch:13:sec:4".to_string())
        );
    }
}
