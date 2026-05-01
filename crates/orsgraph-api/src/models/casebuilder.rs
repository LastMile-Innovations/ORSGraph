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
