use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatterSummary {
    pub matter_id: String,
    pub name: String,
    pub short_name: Option<String>,
    pub matter_type: String,
    pub status: String,
    pub user_role: String,
    pub jurisdiction: String,
    pub court: String,
    pub case_number: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub document_count: u64,
    pub fact_count: u64,
    pub evidence_count: u64,
    pub claim_count: u64,
    pub draft_count: u64,
    pub open_task_count: u64,
    pub next_deadline: Option<NextDeadline>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NextDeadline {
    pub description: String,
    pub due_date: String,
    pub days_remaining: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatterBundle {
    #[serde(flatten)]
    pub summary: MatterSummary,
    pub id: String,
    pub title: String,
    pub documents: Vec<CaseDocument>,
    pub parties: Vec<CaseParty>,
    pub facts: Vec<CaseFact>,
    pub timeline: Vec<CaseTimelineEvent>,
    pub claims: Vec<CaseClaim>,
    pub evidence: Vec<CaseEvidence>,
    pub defenses: Vec<CaseDefense>,
    pub deadlines: Vec<CaseDeadline>,
    pub tasks: Vec<CaseTask>,
    pub drafts: Vec<CaseDraft>,
    pub work_products: Vec<WorkProduct>,
    pub fact_check_findings: Vec<FactCheckFinding>,
    pub citation_check_findings: Vec<CitationCheckFinding>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMatterRequest {
    pub name: String,
    pub matter_type: Option<String>,
    pub user_role: Option<String>,
    pub jurisdiction: Option<String>,
    pub court: Option<String>,
    pub case_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PatchMatterRequest {
    pub name: Option<String>,
    pub matter_type: Option<String>,
    pub status: Option<String>,
    pub user_role: Option<String>,
    pub jurisdiction: Option<String>,
    pub court: Option<String>,
    pub case_number: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseParty {
    pub party_id: String,
    pub id: String,
    pub matter_id: String,
    pub name: String,
    pub role: String,
    pub party_type: String,
    pub represented_by: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePartyRequest {
    pub name: String,
    pub role: Option<String>,
    pub party_type: Option<String>,
    pub represented_by: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseDocument {
    pub document_id: String,
    pub id: String,
    pub matter_id: String,
    pub filename: String,
    pub title: String,
    pub document_type: String,
    pub mime_type: Option<String>,
    pub pages: u64,
    pub bytes: u64,
    pub file_hash: Option<String>,
    pub uploaded_at: String,
    pub source: String,
    pub confidentiality: String,
    pub processing_status: String,
    pub is_exhibit: bool,
    pub exhibit_label: Option<String>,
    pub summary: String,
    pub date_observed: Option<String>,
    pub parties_mentioned: Vec<String>,
    pub entities_mentioned: Vec<String>,
    pub facts_extracted: u64,
    pub citations_found: u64,
    pub contradictions_flagged: u64,
    pub linked_claim_ids: Vec<String>,
    pub folder: String,
    pub storage_path: Option<String>,
    #[serde(default = "default_storage_provider")]
    pub storage_provider: String,
    #[serde(default = "default_storage_status")]
    pub storage_status: String,
    #[serde(default)]
    pub storage_bucket: Option<String>,
    #[serde(default)]
    pub storage_key: Option<String>,
    #[serde(default)]
    pub content_etag: Option<String>,
    #[serde(default)]
    pub upload_expires_at: Option<String>,
    #[serde(default)]
    pub deleted_at: Option<String>,
    #[serde(default)]
    pub object_blob_id: Option<String>,
    #[serde(default)]
    pub current_version_id: Option<String>,
    #[serde(default)]
    pub ingestion_run_ids: Vec<String>,
    #[serde(default)]
    pub source_spans: Vec<SourceSpan>,
    pub extracted_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObjectBlob {
    pub object_blob_id: String,
    pub id: String,
    pub sha256: Option<String>,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub storage_provider: String,
    pub storage_bucket: Option<String>,
    pub storage_key: String,
    pub etag: Option<String>,
    pub storage_class: Option<String>,
    pub created_at: String,
    pub retention_state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentVersion {
    pub document_version_id: String,
    pub id: String,
    pub matter_id: String,
    pub document_id: String,
    pub object_blob_id: String,
    pub role: String,
    pub artifact_kind: String,
    pub source_version_id: Option<String>,
    pub created_by: String,
    pub current: bool,
    pub created_at: String,
    pub storage_provider: String,
    pub storage_bucket: Option<String>,
    pub storage_key: String,
    pub sha256: Option<String>,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IngestionRun {
    pub ingestion_run_id: String,
    pub id: String,
    pub matter_id: String,
    pub document_id: String,
    pub document_version_id: Option<String>,
    pub object_blob_id: Option<String>,
    pub input_sha256: Option<String>,
    pub status: String,
    pub stage: String,
    pub mode: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub retryable: bool,
    pub produced_node_ids: Vec<String>,
    pub produced_object_keys: Vec<String>,
    #[serde(default)]
    pub parser_id: Option<String>,
    #[serde(default)]
    pub parser_version: Option<String>,
    #[serde(default)]
    pub chunker_version: Option<String>,
    #[serde(default)]
    pub citation_resolver_version: Option<String>,
    #[serde(default)]
    pub index_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceSpan {
    pub source_span_id: String,
    pub id: String,
    pub matter_id: String,
    pub document_id: String,
    pub document_version_id: Option<String>,
    pub object_blob_id: Option<String>,
    pub ingestion_run_id: Option<String>,
    pub page: Option<u64>,
    pub chunk_id: Option<String>,
    pub byte_start: Option<u64>,
    pub byte_end: Option<u64>,
    pub char_start: Option<u64>,
    pub char_end: Option<u64>,
    pub quote: Option<String>,
    pub extraction_method: String,
    pub confidence: f32,
    pub review_status: String,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UploadFileRequest {
    pub filename: String,
    pub mime_type: Option<String>,
    pub bytes: Option<u64>,
    pub document_type: Option<String>,
    pub folder: Option<String>,
    pub confidentiality: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFileUploadRequest {
    pub filename: String,
    pub mime_type: Option<String>,
    pub bytes: u64,
    pub document_type: Option<String>,
    pub folder: Option<String>,
    pub confidentiality: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteFileUploadRequest {
    pub document_id: String,
    pub etag: Option<String>,
    pub bytes: Option<u64>,
    pub sha256: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileUploadResponse {
    pub upload_id: String,
    pub document_id: String,
    pub method: String,
    pub url: String,
    pub expires_at: String,
    pub headers: BTreeMap<String, String>,
    pub document: CaseDocument,
}

#[derive(Debug, Serialize)]
pub struct DownloadUrlResponse {
    pub method: String,
    pub url: String,
    pub expires_at: String,
    pub headers: BTreeMap<String, String>,
    pub filename: String,
    pub mime_type: Option<String>,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct DeleteDocumentResponse {
    pub deleted: bool,
    pub document: CaseDocument,
}

#[derive(Debug, Serialize)]
pub struct DocumentExtractionResponse {
    pub enabled: bool,
    pub mode: String,
    pub status: String,
    pub message: String,
    pub document: CaseDocument,
    pub chunks: Vec<ExtractedTextChunk>,
    pub proposed_facts: Vec<CaseFact>,
    pub ingestion_run: Option<IngestionRun>,
    pub document_version: Option<DocumentVersion>,
    pub source_spans: Vec<SourceSpan>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplaintImportProvenance {
    pub document_id: String,
    pub document_version_id: Option<String>,
    pub object_blob_id: Option<String>,
    pub ingestion_run_id: Option<String>,
    pub source_span_id: Option<String>,
    pub parser_id: String,
    pub parser_version: String,
    pub byte_start: Option<u64>,
    pub byte_end: Option<u64>,
    pub char_start: Option<u64>,
    pub char_end: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ComplaintImportRequest {
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub document_ids: Vec<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub force: Option<bool>,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ComplaintImportResponse {
    pub matter_id: String,
    pub mode: String,
    pub imported: Vec<ComplaintImportResult>,
    pub skipped: Vec<ComplaintImportResult>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ComplaintImportResult {
    pub document_id: String,
    pub complaint_id: Option<String>,
    pub status: String,
    pub message: String,
    pub parser_id: String,
    pub likely_complaint: bool,
    pub complaint: Option<ComplaintDraft>,
}

fn default_storage_provider() -> String {
    "local".to_string()
}

fn default_storage_status() -> String {
    "stored".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtractedTextChunk {
    pub chunk_id: String,
    pub document_id: String,
    pub page: u64,
    pub text: String,
    #[serde(default)]
    pub document_version_id: Option<String>,
    #[serde(default)]
    pub object_blob_id: Option<String>,
    #[serde(default)]
    pub source_span_id: Option<String>,
    #[serde(default)]
    pub byte_start: Option<u64>,
    #[serde(default)]
    pub byte_end: Option<u64>,
    #[serde(default)]
    pub char_start: Option<u64>,
    #[serde(default)]
    pub char_end: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseFact {
    pub fact_id: String,
    pub id: String,
    pub matter_id: String,
    pub statement: String,
    pub text: String,
    pub status: String,
    pub confidence: f32,
    pub date: Option<String>,
    pub party_id: Option<String>,
    pub source_document_ids: Vec<String>,
    pub source_evidence_ids: Vec<String>,
    pub contradicted_by_evidence_ids: Vec<String>,
    pub supports_claim_ids: Vec<String>,
    pub supports_defense_ids: Vec<String>,
    pub used_in_draft_ids: Vec<String>,
    pub needs_verification: bool,
    #[serde(default)]
    pub source_spans: Vec<SourceSpan>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFactRequest {
    pub statement: String,
    pub status: Option<String>,
    pub confidence: Option<f32>,
    pub date: Option<String>,
    pub party_id: Option<String>,
    pub source_document_ids: Option<Vec<String>>,
    pub source_evidence_ids: Option<Vec<String>>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PatchFactRequest {
    pub statement: Option<String>,
    pub status: Option<String>,
    pub confidence: Option<f32>,
    pub date: Option<String>,
    pub party_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseTimelineEvent {
    pub event_id: String,
    pub id: String,
    pub matter_id: String,
    pub date: String,
    pub title: String,
    pub description: Option<String>,
    pub kind: String,
    pub category: String,
    pub status: String,
    pub source_document_id: Option<String>,
    pub party_ids: Vec<String>,
    pub linked_fact_ids: Vec<String>,
    pub linked_claim_ids: Vec<String>,
    pub date_confidence: f32,
    pub disputed: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateTimelineEventRequest {
    pub date: String,
    pub title: String,
    pub description: Option<String>,
    pub kind: Option<String>,
    pub source_document_id: Option<String>,
    pub party_ids: Option<Vec<String>>,
    pub linked_fact_ids: Option<Vec<String>>,
    pub linked_claim_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseEvidence {
    pub evidence_id: String,
    pub id: String,
    pub matter_id: String,
    pub document_id: String,
    pub source_span: String,
    pub quote: String,
    pub evidence_type: String,
    pub strength: String,
    pub confidence: f32,
    pub exhibit_label: Option<String>,
    pub supports_fact_ids: Vec<String>,
    pub contradicts_fact_ids: Vec<String>,
    #[serde(default)]
    pub source_spans: Vec<SourceSpan>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEvidenceRequest {
    pub document_id: String,
    pub source_span: Option<String>,
    pub quote: String,
    pub evidence_type: Option<String>,
    pub strength: Option<String>,
    pub confidence: Option<f32>,
    pub exhibit_label: Option<String>,
    pub supports_fact_ids: Option<Vec<String>>,
    pub contradicts_fact_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct LinkEvidenceFactRequest {
    pub fact_id: String,
    pub relation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseClaim {
    pub claim_id: String,
    pub id: String,
    pub matter_id: String,
    pub kind: String,
    pub title: String,
    pub name: String,
    pub claim_type: String,
    pub legal_theory: String,
    pub status: String,
    pub risk_level: String,
    pub fact_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub authorities: Vec<AuthorityRef>,
    pub elements: Vec<CaseElement>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseDefense {
    pub defense_id: String,
    pub id: String,
    pub matter_id: String,
    pub name: String,
    pub basis: String,
    pub status: String,
    pub applies_to_claim_ids: Vec<String>,
    pub required_facts: Vec<String>,
    pub fact_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub authorities: Vec<AuthorityRef>,
    pub viability: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseElement {
    pub element_id: String,
    pub id: String,
    pub matter_id: String,
    pub text: String,
    pub authority: Option<String>,
    #[serde(default)]
    pub authorities: Vec<AuthorityRef>,
    pub satisfied: bool,
    pub fact_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub missing_facts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorityRef {
    pub citation: String,
    pub canonical_id: String,
    pub reason: Option<String>,
    pub pinpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateClaimRequest {
    pub kind: Option<String>,
    pub title: String,
    pub claim_type: Option<String>,
    pub legal_theory: Option<String>,
    pub status: Option<String>,
    pub risk_level: Option<String>,
    pub fact_ids: Option<Vec<String>>,
    pub evidence_ids: Option<Vec<String>>,
    pub authorities: Option<Vec<AuthorityRef>>,
    pub elements: Option<Vec<CreateElementRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDefenseRequest {
    pub name: String,
    pub basis: Option<String>,
    pub status: Option<String>,
    pub applies_to_claim_ids: Option<Vec<String>>,
    pub required_facts: Option<Vec<String>>,
    pub fact_ids: Option<Vec<String>>,
    pub evidence_ids: Option<Vec<String>>,
    pub authorities: Option<Vec<AuthorityRef>>,
    pub viability: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateElementRequest {
    pub text: String,
    pub authority: Option<String>,
    pub fact_ids: Option<Vec<String>>,
    pub evidence_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseDeadline {
    pub deadline_id: String,
    pub id: String,
    pub matter_id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub kind: String,
    pub due_date: String,
    pub days_remaining: i64,
    pub severity: String,
    pub source: String,
    pub source_citation: Option<String>,
    pub source_canonical_id: Option<String>,
    pub triggered_by_event_id: Option<String>,
    pub status: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseTask {
    pub task_id: String,
    pub id: String,
    pub matter_id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub assigned_to: Option<String>,
    pub related_claim_ids: Vec<String>,
    pub related_document_ids: Vec<String>,
    pub related_deadline_id: Option<String>,
    pub source: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseDraft {
    pub draft_id: String,
    pub id: String,
    pub matter_id: String,
    pub title: String,
    pub description: String,
    pub draft_type: String,
    pub kind: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub word_count: u64,
    pub sections: Vec<DraftSection>,
    pub paragraphs: Vec<DraftParagraph>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DraftSection {
    pub section_id: String,
    pub heading: String,
    pub body: String,
    pub citations: Vec<AuthorityRef>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DraftParagraph {
    pub paragraph_id: String,
    pub index: u64,
    pub role: String,
    pub text: String,
    pub fact_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub authorities: Vec<AuthorityRef>,
    pub factcheck_status: String,
    pub factcheck_note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDraftRequest {
    pub title: String,
    pub draft_type: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PatchDraftRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub sections: Option<Vec<DraftSection>>,
    pub paragraphs: Option<Vec<DraftParagraph>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FactCheckFinding {
    pub finding_id: String,
    pub id: String,
    pub matter_id: String,
    pub draft_id: String,
    pub paragraph_id: Option<String>,
    pub finding_type: String,
    pub severity: String,
    pub message: String,
    pub source_fact_ids: Vec<String>,
    pub source_evidence_ids: Vec<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CitationCheckFinding {
    pub finding_id: String,
    pub id: String,
    pub matter_id: String,
    pub draft_id: String,
    pub citation: String,
    pub canonical_id: Option<String>,
    pub finding_type: String,
    pub severity: String,
    pub message: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkProduct {
    pub work_product_id: String,
    pub id: String,
    pub matter_id: String,
    pub title: String,
    pub product_type: String,
    pub status: String,
    pub review_status: String,
    pub setup_stage: String,
    pub source_draft_id: Option<String>,
    pub source_complaint_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub profile: WorkProductProfile,
    pub blocks: Vec<WorkProductBlock>,
    pub marks: Vec<WorkProductMark>,
    pub anchors: Vec<WorkProductAnchor>,
    pub findings: Vec<WorkProductFinding>,
    pub artifacts: Vec<WorkProductArtifact>,
    pub history: Vec<WorkProductHistoryEvent>,
    pub ai_commands: Vec<WorkProductAiCommandState>,
    pub formatting_profile: FormattingProfile,
    pub rule_pack: RulePack,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionSubject {
    pub subject_id: String,
    pub matter_id: String,
    pub subject_type: String,
    pub product_type: String,
    pub profile_id: String,
    pub title: String,
    pub current_branch_id: String,
    pub current_snapshot_id: String,
    pub review_status: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LegalImpactSummary {
    pub affected_counts: Vec<String>,
    pub affected_elements: Vec<String>,
    pub affected_facts: Vec<String>,
    pub affected_evidence: Vec<String>,
    pub affected_authorities: Vec<String>,
    pub affected_exhibits: Vec<String>,
    pub support_status_before: Option<String>,
    pub support_status_after: Option<String>,
    pub qc_warnings_added: Vec<String>,
    pub qc_warnings_resolved: Vec<String>,
    pub blocking_issues_added: Vec<String>,
    pub blocking_issues_resolved: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionTargetSummary {
    pub target_type: String,
    pub target_id: String,
    pub label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionChangeSummary {
    pub text_changes: u64,
    pub support_changes: u64,
    pub citation_changes: u64,
    pub authority_changes: u64,
    pub qc_changes: u64,
    pub export_changes: u64,
    pub ai_changes: u64,
    pub targets_changed: Vec<VersionTargetSummary>,
    pub risk_level: String,
    pub user_summary: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChangeSet {
    pub change_set_id: String,
    pub id: String,
    pub matter_id: String,
    pub subject_id: String,
    pub branch_id: String,
    pub snapshot_id: String,
    pub parent_snapshot_id: Option<String>,
    pub title: String,
    pub summary: String,
    pub reason: Option<String>,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub source: String,
    pub created_at: String,
    pub change_ids: Vec<String>,
    pub legal_impact: LegalImpactSummary,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionSnapshot {
    pub snapshot_id: String,
    pub id: String,
    pub matter_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub product_type: String,
    pub profile_id: String,
    pub branch_id: String,
    pub sequence_number: u64,
    pub title: String,
    pub message: Option<String>,
    pub created_at: String,
    pub created_by: String,
    pub actor_id: Option<String>,
    pub snapshot_type: String,
    pub parent_snapshot_ids: Vec<String>,
    pub document_hash: String,
    pub support_graph_hash: String,
    pub qc_state_hash: String,
    pub formatting_hash: String,
    pub manifest_hash: String,
    pub manifest_ref: Option<String>,
    pub full_state_ref: Option<String>,
    pub full_state_inline: Option<serde_json::Value>,
    pub summary: VersionChangeSummary,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotManifest {
    pub manifest_id: String,
    pub id: String,
    pub snapshot_id: String,
    pub matter_id: String,
    pub subject_id: String,
    pub manifest_hash: String,
    pub entry_count: u64,
    pub storage_ref: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotEntityState {
    pub entity_state_id: String,
    pub id: String,
    pub manifest_id: String,
    pub snapshot_id: String,
    pub matter_id: String,
    pub subject_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub entity_hash: String,
    pub state_ref: Option<String>,
    pub state_inline: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionChange {
    pub change_id: String,
    pub id: String,
    pub change_set_id: String,
    pub snapshot_id: String,
    pub matter_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub branch_id: String,
    pub target_type: String,
    pub target_id: String,
    pub operation: String,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
    pub summary: String,
    pub legal_impact: LegalImpactSummary,
    pub ai_audit_id: Option<String>,
    pub created_at: String,
    pub created_by: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionBranch {
    pub branch_id: String,
    pub id: String,
    pub matter_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_from_snapshot_id: String,
    pub current_snapshot_id: String,
    pub branch_type: String,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LegalSupportUse {
    pub support_use_id: String,
    pub id: String,
    pub matter_id: String,
    pub subject_id: String,
    pub branch_id: String,
    pub target_type: String,
    pub target_id: String,
    pub source_type: String,
    pub source_id: String,
    pub relation: String,
    pub status: String,
    pub quote: Option<String>,
    pub pinpoint: Option<String>,
    pub confidence: Option<f32>,
    pub created_snapshot_id: String,
    pub retired_snapshot_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIEditAudit {
    pub ai_audit_id: String,
    pub id: String,
    pub matter_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub target_type: String,
    pub target_id: String,
    pub command: String,
    pub prompt_template_id: Option<String>,
    pub model: Option<String>,
    pub provider_mode: String,
    pub input_fact_ids: Vec<String>,
    pub input_evidence_ids: Vec<String>,
    pub input_authority_ids: Vec<String>,
    pub input_snapshot_id: String,
    pub output_text: Option<String>,
    pub inserted_text: Option<String>,
    pub user_action: String,
    pub warnings: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Milestone {
    pub milestone_id: String,
    pub id: String,
    pub matter_id: String,
    pub subject_id: String,
    pub snapshot_id: String,
    pub label: String,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkProductProfile {
    pub profile_id: String,
    pub product_type: String,
    pub name: String,
    pub jurisdiction: String,
    pub version: String,
    pub route_slug: String,
    pub required_block_roles: Vec<String>,
    pub optional_block_roles: Vec<String>,
    pub supports_rich_text: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkProductBlock {
    pub block_id: String,
    pub id: String,
    pub matter_id: String,
    pub work_product_id: String,
    pub block_type: String,
    pub role: String,
    pub title: String,
    pub text: String,
    pub ordinal: u64,
    pub parent_block_id: Option<String>,
    pub fact_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub authorities: Vec<AuthorityRef>,
    pub mark_ids: Vec<String>,
    pub locked: bool,
    pub review_status: String,
    pub prosemirror_json: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkProductMark {
    pub mark_id: String,
    pub id: String,
    pub matter_id: String,
    pub work_product_id: String,
    pub block_id: String,
    pub mark_type: String,
    pub from_offset: u64,
    pub to_offset: u64,
    pub label: String,
    pub target_type: String,
    pub target_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkProductAnchor {
    pub anchor_id: String,
    pub id: String,
    pub matter_id: String,
    pub work_product_id: String,
    pub block_id: String,
    pub anchor_type: String,
    pub target_type: String,
    pub target_id: String,
    pub relation: String,
    pub citation: Option<String>,
    pub canonical_id: Option<String>,
    pub pinpoint: Option<String>,
    pub quote: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkProductFinding {
    pub finding_id: String,
    pub id: String,
    pub matter_id: String,
    pub work_product_id: String,
    pub rule_id: String,
    pub category: String,
    pub severity: String,
    pub target_type: String,
    pub target_id: String,
    pub message: String,
    pub explanation: String,
    pub suggested_fix: String,
    pub primary_action: WorkProductAction,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkProductAction {
    pub action_id: String,
    pub label: String,
    pub action_type: String,
    pub href: Option<String>,
    pub target_type: String,
    pub target_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkProductArtifact {
    pub artifact_id: String,
    pub id: String,
    pub matter_id: String,
    pub work_product_id: String,
    pub format: String,
    pub profile: String,
    pub mode: String,
    pub status: String,
    pub download_url: String,
    pub page_count: u64,
    pub generated_at: String,
    pub warnings: Vec<String>,
    pub content_preview: String,
    #[serde(default)]
    pub snapshot_id: Option<String>,
    #[serde(default)]
    pub artifact_hash: Option<String>,
    #[serde(default)]
    pub render_profile_hash: Option<String>,
    #[serde(default)]
    pub qc_status_at_export: Option<String>,
    #[serde(default)]
    pub changed_since_export: Option<bool>,
    #[serde(default)]
    pub immutable: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkProductHistoryEvent {
    pub event_id: String,
    pub id: String,
    pub matter_id: String,
    pub work_product_id: String,
    pub event_type: String,
    pub target_type: String,
    pub target_id: String,
    pub summary: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkProductAiCommandState {
    pub command_id: String,
    pub label: String,
    pub status: String,
    pub mode: String,
    pub description: String,
    pub last_message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkProductRequest {
    pub title: Option<String>,
    pub product_type: String,
    pub template: Option<String>,
    pub source_draft_id: Option<String>,
    pub source_complaint_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PatchWorkProductRequest {
    pub title: Option<String>,
    pub status: Option<String>,
    pub review_status: Option<String>,
    pub setup_stage: Option<String>,
    pub blocks: Option<Vec<WorkProductBlock>>,
    pub marks: Option<Vec<WorkProductMark>>,
    pub anchors: Option<Vec<WorkProductAnchor>>,
    pub formatting_profile: Option<FormattingProfile>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkProductBlockRequest {
    pub block_type: Option<String>,
    pub role: Option<String>,
    pub title: Option<String>,
    pub text: String,
    pub parent_block_id: Option<String>,
    pub fact_ids: Option<Vec<String>>,
    pub evidence_ids: Option<Vec<String>>,
    pub authorities: Option<Vec<AuthorityRef>>,
}

#[derive(Debug, Deserialize)]
pub struct PatchWorkProductBlockRequest {
    pub block_type: Option<String>,
    pub role: Option<String>,
    pub title: Option<String>,
    pub text: Option<String>,
    pub parent_block_id: Option<Option<String>>,
    pub fact_ids: Option<Vec<String>>,
    pub evidence_ids: Option<Vec<String>>,
    pub authorities: Option<Vec<AuthorityRef>>,
    pub locked: Option<bool>,
    pub review_status: Option<String>,
    pub prosemirror_json: Option<Option<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct WorkProductLinkRequest {
    pub block_id: String,
    pub anchor_type: Option<String>,
    pub relation: Option<String>,
    pub target_type: String,
    pub target_id: String,
    pub citation: Option<String>,
    pub canonical_id: Option<String>,
    pub pinpoint: Option<String>,
    pub quote: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PatchWorkProductFindingRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ExportWorkProductRequest {
    pub format: String,
    pub profile: Option<String>,
    pub mode: Option<String>,
    pub include_exhibits: Option<bool>,
    pub include_qc_report: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct WorkProductAiCommandRequest {
    pub command: String,
    pub target_id: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkProductPreviewResponse {
    pub work_product_id: String,
    pub matter_id: String,
    pub html: String,
    pub plain_text: String,
    pub page_count: u64,
    pub warnings: Vec<String>,
    pub generated_at: String,
    pub review_label: String,
}

#[derive(Debug, Serialize)]
pub struct WorkProductDownloadResponse {
    pub method: String,
    pub url: String,
    pub expires_at: String,
    pub headers: BTreeMap<String, String>,
    pub filename: String,
    pub mime_type: Option<String>,
    pub bytes: u64,
    pub artifact: WorkProductArtifact,
}

#[derive(Debug, Deserialize)]
pub struct CreateVersionSnapshotRequest {
    pub title: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RestoreVersionRequest {
    pub snapshot_id: String,
    pub scope: String,
    pub target_ids: Option<Vec<String>>,
    pub mode: Option<String>,
    pub branch_id: Option<String>,
    pub dry_run: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RestoreVersionResponse {
    pub restored: bool,
    pub dry_run: bool,
    pub warnings: Vec<String>,
    pub snapshot_id: String,
    pub change_set: Option<ChangeSet>,
    pub result: Option<WorkProduct>,
}

#[derive(Debug, Serialize)]
pub struct VersionTextDiff {
    pub target_type: String,
    pub target_id: String,
    pub title: String,
    pub status: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CompareVersionsResponse {
    pub matter_id: String,
    pub subject_id: String,
    pub from_snapshot_id: String,
    pub to_snapshot_id: String,
    pub layers: Vec<String>,
    pub summary: VersionChangeSummary,
    pub text_diffs: Vec<VersionTextDiff>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplaintDraft {
    pub complaint_id: String,
    pub id: String,
    pub matter_id: String,
    pub title: String,
    pub status: String,
    pub review_status: String,
    pub setup_stage: String,
    pub active_profile_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub caption: ComplaintCaption,
    pub parties: Vec<ComplaintParty>,
    pub sections: Vec<ComplaintSection>,
    pub counts: Vec<ComplaintCount>,
    pub paragraphs: Vec<PleadingParagraph>,
    pub relief: Vec<ReliefRequest>,
    pub signature: SignatureBlock,
    pub certificate_of_service: Option<CertificateOfService>,
    pub formatting_profile: FormattingProfile,
    pub rule_pack: RulePack,
    pub findings: Vec<RuleCheckFinding>,
    pub export_artifacts: Vec<ExportArtifact>,
    pub history: Vec<ComplaintHistoryEvent>,
    pub next_actions: Vec<ComplaintNextAction>,
    pub ai_commands: Vec<ComplaintAiCommandState>,
    pub filing_packet: FilingPacket,
    #[serde(default)]
    pub import_provenance: Option<ComplaintImportProvenance>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplaintCaption {
    pub court_name: String,
    pub county: String,
    pub case_number: Option<String>,
    pub document_title: String,
    pub plaintiff_names: Vec<String>,
    pub defendant_names: Vec<String>,
    pub jury_demand: bool,
    pub jurisdiction: String,
    pub venue: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplaintParty {
    pub party_id: String,
    pub matter_party_id: Option<String>,
    pub name: String,
    pub role: String,
    pub party_type: String,
    pub represented_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplaintSection {
    pub section_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub title: String,
    pub section_type: String,
    pub ordinal: u64,
    pub paragraph_ids: Vec<String>,
    pub count_ids: Vec<String>,
    pub review_status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplaintCount {
    pub count_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub ordinal: u64,
    pub title: String,
    pub claim_id: Option<String>,
    pub legal_theory: String,
    pub against_party_ids: Vec<String>,
    pub element_ids: Vec<String>,
    pub fact_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub authorities: Vec<AuthorityRef>,
    pub relief_ids: Vec<String>,
    pub paragraph_ids: Vec<String>,
    pub incorporation_range: Option<String>,
    pub health: String,
    pub weaknesses: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PleadingParagraph {
    pub paragraph_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub section_id: Option<String>,
    pub count_id: Option<String>,
    pub number: u64,
    pub ordinal: u64,
    #[serde(default)]
    pub display_number: Option<String>,
    #[serde(default)]
    pub original_label: Option<String>,
    #[serde(default)]
    pub source_span_id: Option<String>,
    #[serde(default)]
    pub import_provenance: Option<ComplaintImportProvenance>,
    pub role: String,
    pub text: String,
    pub sentences: Vec<PleadingSentence>,
    pub fact_ids: Vec<String>,
    pub evidence_uses: Vec<EvidenceUse>,
    pub citation_uses: Vec<CitationUse>,
    pub exhibit_references: Vec<ExhibitReference>,
    pub rule_finding_ids: Vec<String>,
    pub locked: bool,
    pub review_status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PleadingSentence {
    pub sentence_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub paragraph_id: String,
    pub ordinal: u64,
    pub text: String,
    pub fact_ids: Vec<String>,
    pub evidence_use_ids: Vec<String>,
    pub citation_use_ids: Vec<String>,
    pub review_status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CitationUse {
    pub citation_use_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub target_type: String,
    pub target_id: String,
    pub citation: String,
    pub canonical_id: Option<String>,
    pub pinpoint: Option<String>,
    pub quote: Option<String>,
    pub status: String,
    pub currentness: String,
    pub scope_warning: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvidenceUse {
    pub evidence_use_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub target_type: String,
    pub target_id: String,
    pub fact_id: Option<String>,
    pub evidence_id: Option<String>,
    pub document_id: Option<String>,
    pub source_span_id: Option<String>,
    pub relation: String,
    pub quote: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExhibitReference {
    pub exhibit_reference_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub target_type: String,
    pub target_id: String,
    pub exhibit_label: String,
    pub document_id: Option<String>,
    pub evidence_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReliefRequest {
    pub relief_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub category: String,
    pub text: String,
    pub amount: Option<String>,
    pub authority_ids: Vec<String>,
    pub supported: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignatureBlock {
    pub name: String,
    pub bar_number: Option<String>,
    pub firm: Option<String>,
    pub address: String,
    pub phone: String,
    pub email: String,
    pub signature_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CertificateOfService {
    pub certificate_id: String,
    pub method: String,
    pub served_parties: Vec<String>,
    pub service_date: Option<String>,
    pub text: String,
    pub review_status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormattingProfile {
    pub profile_id: String,
    pub name: String,
    pub jurisdiction: String,
    pub line_numbers: bool,
    pub double_spaced: bool,
    pub first_page_top_blank_inches: f32,
    pub margin_top_inches: f32,
    pub margin_bottom_inches: f32,
    pub margin_left_inches: f32,
    pub margin_right_inches: f32,
    pub font_family: String,
    pub font_size_pt: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RulePack {
    pub rule_pack_id: String,
    pub name: String,
    pub jurisdiction: String,
    pub version: String,
    pub effective_date: String,
    pub rules: Vec<RuleDefinition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleDefinition {
    pub rule_id: String,
    pub source_citation: String,
    pub source_url: String,
    pub severity: String,
    pub target_type: String,
    pub category: String,
    pub message: String,
    pub explanation: String,
    pub suggested_fix: String,
    pub auto_fix_available: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleCheckFinding {
    pub finding_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub rule_id: String,
    pub category: String,
    pub severity: String,
    pub target_type: String,
    pub target_id: String,
    pub message: String,
    pub explanation: String,
    pub suggested_fix: String,
    pub primary_action: ComplaintAction,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplaintAction {
    pub action_id: String,
    pub label: String,
    pub action_type: String,
    pub href: Option<String>,
    pub target_type: String,
    pub target_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplaintNextAction {
    pub action_id: String,
    pub priority: String,
    pub label: String,
    pub detail: String,
    pub action_type: String,
    pub target_type: String,
    pub target_id: String,
    pub href: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportArtifact {
    pub artifact_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub format: String,
    pub profile: String,
    pub mode: String,
    pub status: String,
    pub download_url: String,
    pub page_count: u64,
    pub generated_at: String,
    pub warnings: Vec<String>,
    pub content_preview: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplaintHistoryEvent {
    pub event_id: String,
    pub id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub event_type: String,
    pub target_type: String,
    pub target_id: String,
    pub summary: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplaintAiCommandState {
    pub command_id: String,
    pub label: String,
    pub status: String,
    pub mode: String,
    pub description: String,
    pub last_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FilingPacket {
    pub packet_id: String,
    pub matter_id: String,
    pub complaint_id: String,
    pub status: String,
    pub items: Vec<FilingPacketItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FilingPacketItem {
    pub item_id: String,
    pub label: String,
    pub item_type: String,
    pub status: String,
    pub artifact_id: Option<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateComplaintRequest {
    pub title: Option<String>,
    pub template: Option<String>,
    pub source_draft_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PatchComplaintRequest {
    pub title: Option<String>,
    pub status: Option<String>,
    pub review_status: Option<String>,
    pub setup_stage: Option<String>,
    pub caption: Option<ComplaintCaption>,
    pub parties: Option<Vec<ComplaintParty>>,
    pub sections: Option<Vec<ComplaintSection>>,
    pub counts: Option<Vec<ComplaintCount>>,
    pub paragraphs: Option<Vec<PleadingParagraph>>,
    pub relief: Option<Vec<ReliefRequest>>,
    pub signature: Option<SignatureBlock>,
    pub certificate_of_service: Option<Option<CertificateOfService>>,
    pub formatting_profile: Option<FormattingProfile>,
}

#[derive(Debug, Deserialize)]
pub struct CreateComplaintSectionRequest {
    pub title: String,
    pub section_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateComplaintCountRequest {
    pub title: String,
    pub claim_id: Option<String>,
    pub legal_theory: Option<String>,
    pub against_party_ids: Option<Vec<String>>,
    pub element_ids: Option<Vec<String>>,
    pub relief_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateComplaintParagraphRequest {
    pub section_id: Option<String>,
    pub count_id: Option<String>,
    pub role: Option<String>,
    pub text: String,
    pub fact_ids: Option<Vec<String>>,
    pub evidence_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct PatchComplaintParagraphRequest {
    pub section_id: Option<String>,
    pub count_id: Option<String>,
    pub role: Option<String>,
    pub text: Option<String>,
    pub fact_ids: Option<Vec<String>>,
    pub evidence_uses: Option<Vec<EvidenceUse>>,
    pub citation_uses: Option<Vec<CitationUse>>,
    pub exhibit_references: Option<Vec<ExhibitReference>>,
    pub locked: Option<bool>,
    pub review_status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ComplaintLinkRequest {
    pub target_type: String,
    pub target_id: String,
    pub relation: Option<String>,
    pub fact_id: Option<String>,
    pub evidence_id: Option<String>,
    pub document_id: Option<String>,
    pub source_span_id: Option<String>,
    pub citation: Option<String>,
    pub canonical_id: Option<String>,
    pub pinpoint: Option<String>,
    pub quote: Option<String>,
    pub exhibit_label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PatchRuleFindingRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ExportComplaintRequest {
    pub format: String,
    pub profile: Option<String>,
    pub mode: Option<String>,
    pub include_exhibits: Option<bool>,
    pub include_qc_report: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ComplaintAiCommandRequest {
    pub command: String,
    pub target_id: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ComplaintPreviewResponse {
    pub complaint_id: String,
    pub matter_id: String,
    pub html: String,
    pub plain_text: String,
    pub page_count: u64,
    pub warnings: Vec<String>,
    pub generated_at: String,
    pub review_label: String,
}

#[derive(Debug, Serialize)]
pub struct ComplaintDownloadResponse {
    pub method: String,
    pub url: String,
    pub expires_at: String,
    pub headers: BTreeMap<String, String>,
    pub filename: String,
    pub mime_type: Option<String>,
    pub bytes: u64,
    pub artifact: ExportArtifact,
}

#[derive(Debug, Serialize)]
pub struct AiActionResponse<T: Serialize> {
    pub enabled: bool,
    pub mode: String,
    pub message: String,
    pub result: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct AuthoritySearchQuery {
    pub q: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct AuthorityRecommendRequest {
    pub text: String,
    pub claim_id: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct AuthorityAttachmentRequest {
    pub target_type: String,
    pub target_id: String,
    pub citation: String,
    pub canonical_id: String,
    pub reason: Option<String>,
    pub pinpoint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthorityAttachmentResponse {
    pub matter_id: String,
    pub target_type: String,
    pub target_id: String,
    pub authority: AuthorityRef,
    pub attached: bool,
}

#[derive(Debug, Serialize)]
pub struct AuthoritySearchResponse {
    pub matter_id: String,
    pub query: String,
    pub source: String,
    pub results: Vec<AuthoritySearchItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthoritySearchItem {
    pub id: String,
    pub kind: String,
    pub citation: Option<String>,
    pub canonical_id: Option<String>,
    pub title: Option<String>,
    pub snippet: String,
    pub score: f32,
    pub href: String,
}
