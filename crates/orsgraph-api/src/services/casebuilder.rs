use crate::error::{ApiError, ApiResult};
use crate::models::casebuilder::*;
use crate::services::neo4j::Neo4jService;
use crate::services::object_store::{
    build_document_object_key, clean_etag, normalize_sha256, ObjectStore, PutOptions, StoredObject,
};
use bytes::Bytes;
use neo4rs::query;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;

#[derive(Clone)]
pub struct CaseBuilderService {
    neo4j: Arc<Neo4jService>,
    object_store: Arc<dyn ObjectStore>,
    upload_ttl_seconds: u64,
    download_ttl_seconds: u64,
    max_upload_bytes: u64,
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

impl CaseBuilderService {
    pub fn new(
        neo4j: Arc<Neo4jService>,
        object_store: Arc<dyn ObjectStore>,
        upload_ttl_seconds: u64,
        download_ttl_seconds: u64,
        max_upload_bytes: u64,
    ) -> Self {
        Self {
            neo4j,
            object_store,
            upload_ttl_seconds,
            download_ttl_seconds,
            max_upload_bytes,
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
            "CREATE INDEX casebuilder_document_matter IF NOT EXISTS FOR (n:CaseDocument) ON (n.matter_id)",
            "CREATE INDEX casebuilder_fact_matter IF NOT EXISTS FOR (n:Fact) ON (n.matter_id)",
            "CREATE INDEX casebuilder_claim_matter IF NOT EXISTS FOR (n:Claim) ON (n.matter_id)",
            "CREATE INDEX casebuilder_draft_matter IF NOT EXISTS FOR (n:Draft) ON (n.matter_id)",
            "CREATE INDEX casebuilder_document_version_matter IF NOT EXISTS FOR (n:DocumentVersion) ON (n.matter_id)",
            "CREATE INDEX casebuilder_ingestion_run_matter IF NOT EXISTS FOR (n:IngestionRun) ON (n.matter_id)",
            "CREATE INDEX casebuilder_source_span_matter IF NOT EXISTS FOR (n:SourceSpan) ON (n.matter_id)",
            "CREATE FULLTEXT INDEX casebuilder_fact_fulltext IF NOT EXISTS FOR (n:Fact) ON EACH [n.text, n.statement]",
            "CREATE FULLTEXT INDEX casebuilder_document_fulltext IF NOT EXISTS FOR (n:CaseDocument) ON EACH [n.filename, n.title, n.summary, n.extracted_text]",
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
                           n:DraftParagraph
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
        let extracted_text = extractable_v0_text(
            &request.filename,
            request.mime_type.as_deref(),
            &request.bytes,
        );
        let is_extractable = extracted_text
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty());
        let processing_status = if is_extractable {
            "processed"
        } else {
            "unsupported"
        };

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
            processing_status: processing_status.to_string(),
            is_exhibit: false,
            exhibit_label: None,
            summary: if is_extractable {
                "Stored privately. Text is ready for deterministic V0 extraction.".to_string()
            } else {
                "Stored privately. Binary parsing/OCR is deferred for V0; keep as queued evidence."
                    .to_string()
            },
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
            extracted_text,
        };

        let provenance = build_original_provenance(matter_id, &document, &stored_object, "stored");
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
            document.processing_status = "failed".to_string();
            document.summary =
                "No extractable text is available for this document in V0.".to_string();
            let ingestion_run = provenance.as_ref().map(|provenance| {
                failed_ingestion_run(
                    &provenance.ingestion_run,
                    "extract_text",
                    "no_extractable_text",
                    "No text content was supplied; OCR and binary parsing are deferred.",
                    false,
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
                status: "failed".to_string(),
                message: "No text content was supplied; OCR and binary parsing are deferred."
                    .to_string(),
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

    pub async fn create_draft(
        &self,
        matter_id: &str,
        request: CreateDraftRequest,
    ) -> ApiResult<CaseDraft> {
        self.require_matter(matter_id).await?;
        let now = now_string();
        let id = generate_id("draft", &request.title);
        let kind = request
            .draft_type
            .unwrap_or_else(|| "complaint".to_string());
        let draft = CaseDraft {
            id: id.clone(),
            draft_id: id,
            matter_id: matter_id.to_string(),
            title: request.title,
            description: request.description.unwrap_or_default(),
            draft_type: kind.clone(),
            kind,
            status: request.status.unwrap_or_else(|| "draft".to_string()),
            created_at: now.clone(),
            updated_at: now,
            word_count: 0,
            sections: Vec::new(),
            paragraphs: Vec::new(),
        };
        self.merge_node(matter_id, draft_spec(), &draft.draft_id, &draft)
            .await
    }

    pub async fn list_drafts(&self, matter_id: &str) -> ApiResult<Vec<CaseDraft>> {
        self.list_nodes(matter_id, draft_spec()).await
    }

    pub async fn get_draft(&self, matter_id: &str, draft_id: &str) -> ApiResult<CaseDraft> {
        self.get_node(matter_id, draft_spec(), draft_id).await
    }

    pub async fn patch_draft(
        &self,
        matter_id: &str,
        draft_id: &str,
        request: PatchDraftRequest,
    ) -> ApiResult<CaseDraft> {
        let mut draft = self.get_draft(matter_id, draft_id).await?;
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
        let draft = self
            .merge_node(matter_id, draft_spec(), draft_id, &draft)
            .await?;
        self.materialize_draft_edges(&draft).await?;
        Ok(draft)
    }

    pub async fn generate_draft(
        &self,
        matter_id: &str,
        draft_id: &str,
    ) -> ApiResult<AiActionResponse<CaseDraft>> {
        let mut draft = self.get_draft(matter_id, draft_id).await?;
        let facts = self.list_facts(matter_id).await?;
        let claims = self.list_claims(matter_id).await?;

        let mut paragraphs = Vec::new();
        paragraphs.push(DraftParagraph {
            paragraph_id: format!("{draft_id}:p:1"),
            index: 1,
            role: "caption".to_string(),
            text: format!("Draft {} for matter {}.", draft.kind, matter_id),
            fact_ids: Vec::new(),
            evidence_ids: Vec::new(),
            authorities: Vec::new(),
            factcheck_status: "needs_authority".to_string(),
            factcheck_note: Some("Caption and court formatting require user review.".to_string()),
        });

        for (index, fact) in facts.iter().take(12).enumerate() {
            paragraphs.push(DraftParagraph {
                paragraph_id: format!("{draft_id}:fact:{}", index + 1),
                index: (index + 2) as u64,
                role: "factual_allegation".to_string(),
                text: fact.statement.clone(),
                fact_ids: vec![fact.fact_id.clone()],
                evidence_ids: fact.source_evidence_ids.clone(),
                authorities: Vec::new(),
                factcheck_status: if fact.source_evidence_ids.is_empty()
                    && fact.source_document_ids.is_empty()
                {
                    "needs_evidence".to_string()
                } else {
                    "supported".to_string()
                },
                factcheck_note: None,
            });
        }

        for claim in &claims {
            paragraphs.push(DraftParagraph {
                paragraph_id: format!("{draft_id}:claim:{}", claim.claim_id),
                index: paragraphs.len() as u64 + 1,
                role: "legal_claim".to_string(),
                text: format!("{}: {}", claim.title, claim.legal_theory),
                fact_ids: claim.fact_ids.clone(),
                evidence_ids: claim.evidence_ids.clone(),
                authorities: claim.authorities.clone(),
                factcheck_status: if claim.authorities.is_empty() {
                    "needs_authority".to_string()
                } else {
                    "supported".to_string()
                },
                factcheck_note: None,
            });
        }

        draft.paragraphs = paragraphs;
        draft.sections = vec![DraftSection {
            section_id: format!("{draft_id}:section:facts"),
            heading: "Factual Allegations".to_string(),
            body: draft
                .paragraphs
                .iter()
                .filter(|p| p.role == "factual_allegation")
                .map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join("\n\n"),
            citations: Vec::new(),
        }];
        draft.word_count = count_words(&draft.paragraphs, &draft.sections);
        draft.updated_at = now_string();
        let draft = self
            .merge_node(matter_id, draft_spec(), draft_id, &draft)
            .await?;
        self.materialize_draft_edges(&draft).await?;

        Ok(AiActionResponse {
            enabled: false,
            mode: "template".to_string(),
            message: "No live drafting provider is configured; generated a deterministic source-linked draft scaffold.".to_string(),
            result: Some(draft),
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
            return Ok(String::from_utf8(bytes.to_vec()).unwrap_or_default());
        }
        if let Some(path) = document.storage_path.as_deref() {
            let bytes = fs::read(path).await.map_err(io_error)?;
            return Ok(String::from_utf8(bytes).unwrap_or_default());
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

fn extractable_v0_text(filename: &str, mime_type: Option<&str>, bytes: &[u8]) -> Option<String> {
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
        return None;
    }
    String::from_utf8(bytes.to_vec()).ok()
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
        chunk_text, failed_ingestion_run, generate_opaque_id, object_blob_id_for_hash,
        propose_facts, sanitize_filename, sha256_hex, slug, SourceContext,
    };
    use crate::models::casebuilder::IngestionRun;
    use crate::services::object_store::build_document_object_key;

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
        };
        let failed =
            failed_ingestion_run(&run, "extract_text", "no_extractable_text", "empty", false);
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.stage, "extract_text");
        assert_eq!(failed.error_code.as_deref(), Some("no_extractable_text"));
        assert!(!failed.retryable);
        assert!(failed.completed_at.is_some());
    }
}
