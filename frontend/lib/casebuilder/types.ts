// CaseBuilder type system — "Cursor for law"
// Files → facts → events → evidence → claims/defenses → authorities → drafts.
//
// A Matter is a self-contained graph: documents, facts, events, evidence,
// claims, defenses, deadlines, drafts, and chat history are all keyed off
// the same matter_id and link back to ORSGraph provisions for grounding.

// ===== Matter top-level =====

export type MatterType =
  | "civil"
  | "family"
  | "small_claims"
  | "admin"
  | "criminal"
  | "appeal"
  | "landlord_tenant"
  | "employment"
  | "fact_check"
  | "complaint_analysis"
  | "other"

export type MatterStatus = "active" | "intake" | "stayed" | "closed" | "appeal"

export type MatterSide = "plaintiff" | "defendant" | "petitioner" | "respondent" | "neutral" | "researcher"
export type FactStatus =
  | "proposed"
  | "supported"
  | "alleged"
  | "disputed"
  | "admitted"
  | "denied"
  | "unknown"
  | "contradicted"
  | "needs_evidence"
  | "rejected"
export type EvidenceStrength = "strong" | "moderate" | "weak" | "speculative"
export type ClaimStatus = "candidate" | "asserted" | "dismissed" | "resolved" | "withdrawn"
export type DefenseStatus = "candidate" | "asserted" | "waived" | "rejected"
export type RiskLevel = "low" | "medium" | "high"
export type TaskStatus = "todo" | "in_progress" | "blocked" | "done"
export type TaskPriority = "high" | "med" | "low"
export type ParagraphFactCheck =
  | "supported"
  | "needs_evidence"
  | "needs_authority"
  | "contradicted"
  | "citation_issue"
  | "deadline_warning"
  | "unchecked"

// Lightweight matter card used in the matters list / index.
export interface MatterSummary {
  matter_id: string
  name: string
  shortName?: string
  matter_type: MatterType
  status: MatterStatus
  user_role: MatterSide
  jurisdiction: string
  court: string
  case_number: string | null
  owner_subject?: string | null
  owner_email?: string | null
  owner_name?: string | null
  created_by_subject?: string | null
  created_at: string
  updated_at: string
  document_count: number
  fact_count: number
  evidence_count: number
  claim_count: number
  draft_count: number
  open_task_count: number
  next_deadline: { description: string; due_date: string; days_remaining: number } | null
}

export interface AuthorityRef {
  citation: string
  canonical_id: string
  reason?: string | null
  pinpoint?: string | null
}

export interface CaseGraphNode {
  id: string
  kind: string
  label: string
  subtitle?: string | null
  status?: string | null
  risk?: string | null
  href?: string | null
  metadata: Record<string, string>
}

export interface CaseGraphEdge {
  id: string
  source: string
  target: string
  kind: string
  label: string
  status?: string | null
  metadata: Record<string, string>
}

export interface CaseGraphResponse {
  matter_id: string
  generated_at: string
  modes: string[]
  nodes: CaseGraphNode[]
  edges: CaseGraphEdge[]
  warnings: string[]
}

export interface IssueSuggestion {
  suggestion_id: string
  id: string
  matter_id: string
  issue_type: string
  title: string
  summary: string
  confidence: number
  severity: string
  status: string
  fact_ids: string[]
  evidence_ids: string[]
  document_ids: string[]
  authority_refs: AuthorityRef[]
  recommended_action: string
  mode: string
}

export interface IssueSpotResponse {
  matter_id: string
  generated_at: string
  mode: string
  suggestions: IssueSuggestion[]
  warnings: string[]
}

export interface EvidenceGap {
  gap_id: string
  id: string
  matter_id: string
  target_type: string
  target_id: string
  title: string
  message: string
  severity: string
  status: string
  fact_ids: string[]
  evidence_ids: string[]
}

export interface AuthorityGap {
  gap_id: string
  id: string
  matter_id: string
  target_type: string
  target_id: string
  title: string
  message: string
  severity: string
  status: string
  authority_refs: AuthorityRef[]
}

export interface Contradiction {
  contradiction_id: string
  id: string
  matter_id: string
  title: string
  message: string
  severity: string
  status: string
  fact_ids: string[]
  evidence_ids: string[]
  source_document_ids: string[]
}

export interface WorkProductSentence {
  sentence_id: string
  id: string
  matter_id: string
  work_product_id: string
  block_id: string
  text: string
  index: number
  support_status: string
  fact_ids: string[]
  evidence_ids: string[]
  authority_refs: AuthorityRef[]
  finding_ids: string[]
}

export interface QcSuggestedTask {
  title: string
  status?: TaskStatus
  priority?: TaskPriority
  due_date?: string | null
  assigned_to?: string | null
  related_claim_ids?: string[]
  related_document_ids?: string[]
  related_deadline_id?: string | null
  source?: string
  description?: string | null
}

export interface QcRun {
  qc_run_id: string
  id: string
  matter_id: string
  status: string
  mode: string
  generated_at: string
  evidence_gaps: EvidenceGap[]
  authority_gaps: AuthorityGap[]
  contradictions: Contradiction[]
  fact_findings: CaseFactCheckFinding[]
  citation_findings: CaseCitationCheckFinding[]
  work_product_findings: WorkProductFinding[]
  work_product_sentences: WorkProductSentence[]
  suggested_tasks: QcSuggestedTask[]
  warnings: string[]
}

export interface ExportPackage {
  export_package_id: string
  id: string
  matter_id: string
  format: string
  status: string
  profile: string
  created_at: string
  artifact_count: number
  work_product_ids: string[]
  warnings: string[]
  download_url?: string | null
}

export interface AuditEvent {
  audit_event_id: string
  id: string
  matter_id: string
  event_type: string
  actor: string
  target_type: string
  target_id: string
  summary: string
  created_at: string
  metadata: Record<string, string>
}

// ===== Documents =====

export type DocumentKind =
  | "complaint"
  | "answer"
  | "motion"
  | "order"
  | "contract"
  | "lease"
  | "email"
  | "letter"
  | "notice"
  | "medical"
  | "police"
  | "agency_record"
  | "public_record"
  | "spreadsheet"
  | "photo"
  | "screenshot"
  | "audio_transcript"
  | "receipt"
  | "invoice"
  | "evidence"
  | "exhibit"
  | "other"

export type DocumentType = DocumentKind

export type ProcessingStatus =
  | "queued"
  | "processing"
  | "processed"
  | "review_ready"
  | "failed"
  | "unsupported"
  | "ocr_required"
  | "transcription_deferred"
  | "view_only"
export type StorageStatus = "pending" | "stored" | "metadata_only" | "failed" | "deleted" | string

export interface ObjectBlob {
  object_blob_id: string
  id: string
  sha256?: string | null
  size_bytes: number
  mime_type?: string | null
  storage_provider: "local" | "r2" | string
  storage_bucket?: string | null
  storage_key: string
  etag?: string | null
  storage_class?: string | null
  created_at: string
  retention_state: "active" | "tombstoned" | "deleted" | string
}

export interface DocumentVersion {
  document_version_id: string
  id: string
  matter_id: string
  document_id: string
  object_blob_id: string
  role: "original" | "redacted" | "ocr" | "normalized_text" | "exhibit_bundle" | string
  artifact_kind: "original_upload" | "normalized_text" | "manifest" | "ocr" | "export" | string
  source_version_id?: string | null
  created_by: string
  current: boolean
  created_at: string
  storage_provider: "local" | "r2" | string
  storage_bucket?: string | null
  storage_key: string
  sha256?: string | null
  size_bytes: number
  mime_type?: string | null
}

export interface IngestionRun {
  ingestion_run_id: string
  id: string
  matter_id: string
  document_id: string
  document_version_id?: string | null
  object_blob_id?: string | null
  input_sha256?: string | null
  status: "stored" | "review_ready" | "failed" | "unsupported" | "queued" | string
  stage: string
  mode: "deterministic" | "template" | "live" | "disabled" | string
  started_at: string
  completed_at?: string | null
  error_code?: string | null
  error_message?: string | null
  retryable: boolean
  produced_node_ids: string[]
  produced_object_keys: string[]
  parser_id?: string | null
  parser_version?: string | null
  chunker_version?: string | null
  citation_resolver_version?: string | null
  index_version?: string | null
}

export interface IndexRun {
  index_run_id: string
  id: string
  matter_id: string
  document_id: string
  document_version_id?: string | null
  object_blob_id?: string | null
  ingestion_run_id?: string | null
  status: "review_ready" | "failed" | "queued" | string
  stage: string
  mode: "deterministic" | "template" | "live" | "disabled" | string
  started_at: string
  completed_at?: string | null
  error_code?: string | null
  error_message?: string | null
  retryable: boolean
  parser_id?: string | null
  parser_version?: string | null
  chunker_version?: string | null
  citation_resolver_version?: string | null
  index_version?: string | null
  produced_node_ids: string[]
  produced_object_keys: string[]
  stale: boolean
}

export interface Page {
  page_id: string
  id: string
  matter_id: string
  document_id: string
  document_version_id?: string | null
  object_blob_id?: string | null
  ingestion_run_id?: string | null
  index_run_id?: string | null
  page_number: number
  unit_type: string
  title?: string | null
  text_hash?: string | null
  byte_start?: number | null
  byte_end?: number | null
  char_start?: number | null
  char_end?: number | null
  status: string
}

export interface TextChunk {
  text_chunk_id: string
  id: string
  matter_id: string
  document_id: string
  document_version_id?: string | null
  object_blob_id?: string | null
  page_id?: string | null
  source_span_id?: string | null
  ingestion_run_id?: string | null
  index_run_id?: string | null
  ordinal: number
  page: number
  text_hash: string
  text_excerpt: string
  token_count: number
  unit_type?: string | null
  structure_path?: string | null
  byte_start?: number | null
  byte_end?: number | null
  char_start?: number | null
  char_end?: number | null
  status: string
}

export interface EvidenceSpan {
  evidence_span_id: string
  id: string
  matter_id: string
  document_id: string
  document_version_id?: string | null
  object_blob_id?: string | null
  text_chunk_id?: string | null
  source_span_id?: string | null
  ingestion_run_id?: string | null
  index_run_id?: string | null
  quote_hash: string
  quote_excerpt: string
  byte_start?: number | null
  byte_end?: number | null
  char_start?: number | null
  char_end?: number | null
  review_status: "unreviewed" | "approved" | "rejected" | string
}

export interface EntityMention {
  entity_mention_id: string
  id: string
  matter_id: string
  document_id: string
  text_chunk_id?: string | null
  source_span_id?: string | null
  mention_text: string
  entity_type: string
  confidence: number
  byte_start?: number | null
  byte_end?: number | null
  char_start?: number | null
  char_end?: number | null
  review_status: "unreviewed" | "approved" | "rejected" | string
}

export interface SearchIndexRecord {
  search_index_record_id: string
  id: string
  matter_id: string
  document_id: string
  document_version_id?: string | null
  text_chunk_id?: string | null
  index_run_id?: string | null
  index_name: string
  index_type: string
  index_version: string
  status: string
  stale: boolean
  created_at: string
  indexed_at?: string | null
}

export interface ExtractionArtifactManifest {
  manifest_id: string
  id: string
  matter_id: string
  document_id: string
  document_version_id?: string | null
  object_blob_id?: string | null
  ingestion_run_id?: string | null
  index_run_id?: string | null
  normalized_text_version_id?: string | null
  pages_version_id?: string | null
  manifest_version_id?: string | null
  text_sha256: string
  pages_sha256?: string | null
  manifest_sha256?: string | null
  page_ids: string[]
  text_chunk_ids: string[]
  evidence_span_ids: string[]
  entity_mention_ids: string[]
  search_index_record_ids: string[]
  produced_object_keys: string[]
  created_at: string
}

export interface MatterIndexStatusCount {
  status: string
  count: number
}

export interface MatterIndexFolderSummary {
  folder: string
  count: number
  indexed: number
  pending: number
  failed: number
}

export interface MatterIndexDuplicateGroup {
  file_hash: string
  count: number
  document_ids: string[]
  filenames: string[]
}

export interface MatterIndexUploadBatchSummary {
  upload_batch_id: string
  count: number
  indexed: number
  pending: number
  failed: number
}

export interface MatterIndexSummary {
  matter_id: string
  total_documents: number
  active_documents: number
  archived_documents: number
  indexed_documents: number
  pending_documents: number
  extractable_pending_documents: number
  failed_documents: number
  ocr_required_documents: number
  transcription_deferred_documents: number
  unsupported_documents: number
  processing_status_counts: MatterIndexStatusCount[]
  storage_status_counts: MatterIndexStatusCount[]
  duplicate_groups: MatterIndexDuplicateGroup[]
  folders: MatterIndexFolderSummary[]
  upload_batches: MatterIndexUploadBatchSummary[]
  recent_ingestion_runs: IngestionRun[]
  extractable_pending_document_ids: string[]
}

export interface MatterIndexRunDocumentResult {
  document_id: string
  status: "indexed" | "skipped" | "failed" | string
  extraction_status?: string | null
  message: string
  produced_chunks: number
  produced_facts: number
  produced_timeline_suggestions: number
}

export interface MatterIndexRunResponse {
  matter_id: string
  requested: number
  processed: number
  skipped: number
  failed: number
  produced_timeline_suggestions: number
  results: MatterIndexRunDocumentResult[]
  summary: MatterIndexSummary
}

export interface ComplaintImportProvenance {
  document_id: string
  document_version_id?: string | null
  object_blob_id?: string | null
  ingestion_run_id?: string | null
  source_span_id?: string | null
  parser_id: string
  parser_version: string
  byte_start?: number | null
  byte_end?: number | null
  char_start?: number | null
  char_end?: number | null
}

export interface ComplaintImportResult {
  document_id: string
  complaint_id?: string | null
  status: string
  message: string
  parser_id: string
  likely_complaint: boolean
  complaint?: ComplaintDraft | null
}

export interface ComplaintImportResponse {
  matter_id: string
  mode: string
  imported: ComplaintImportResult[]
  skipped: ComplaintImportResult[]
  warnings: string[]
}

export interface SourceSpan {
  source_span_id: string
  id: string
  matter_id: string
  document_id: string
  document_version_id?: string | null
  object_blob_id?: string | null
  ingestion_run_id?: string | null
  page?: number | null
  chunk_id?: string | null
  byte_start?: number | null
  byte_end?: number | null
  char_start?: number | null
  char_end?: number | null
  time_start_ms?: number | null
  time_end_ms?: number | null
  speaker_label?: string | null
  quote?: string | null
  extraction_method: string
  confidence: number
  review_status: "unreviewed" | "approved" | "rejected" | "unavailable" | string
  unavailable_reason?: string | null
}

export interface DocumentCapability {
  capability: string
  enabled: boolean
  mode: string
  reason?: string | null
}

export interface DocumentPageRange {
  page: number
  x?: number | null
  y?: number | null
  width?: number | null
  height?: number | null
}

export interface DocumentTextRange {
  page?: number | null
  byte_start?: number | null
  byte_end?: number | null
  char_start?: number | null
  char_end?: number | null
  time_start_ms?: number | null
  time_end_ms?: number | null
  speaker_label?: string | null
  quote?: string | null
}

export interface DocumentAnnotation {
  annotation_id: string
  id: string
  matter_id: string
  document_id: string
  document_version_id?: string | null
  annotation_type: "highlight" | "note" | "redaction" | "exhibit_label" | "fact_link" | "citation" | "issue" | string
  status: "active" | "resolved" | "deleted" | string
  label: string
  note?: string | null
  color?: string | null
  page_range?: DocumentPageRange | null
  text_range?: DocumentTextRange | null
  target_type?: string | null
  target_id?: string | null
  created_by: string
  created_at: string
  updated_at: string
}

export interface DocxPackageEntry {
  name: string
  size_bytes: number
  compressed_size_bytes: number
  compression: string
  supported_text_part: boolean
}

export interface DocxPackageManifest {
  document_id: string
  document_version_id?: string | null
  entry_count: number
  text_part_count: number
  editable: boolean
  unsupported_features: string[]
  entries: DocxPackageEntry[]
  text_preview?: string | null
}

export interface DocumentWorkspace {
  matter_id: string
  document: CaseDocument
  current_version?: DocumentVersion | null
  capabilities: DocumentCapability[]
  annotations: DocumentAnnotation[]
  source_spans: SourceSpan[]
  transcriptions: TranscriptionJobResponse[]
  docx_manifest?: DocxPackageManifest | null
  text_content?: string | null
  content_url?: string | null
  warnings: string[]
}

export interface TranscriptionJob {
  transcription_job_id: string
  id: string
  matter_id: string
  document_id: string
  document_version_id?: string | null
  object_blob_id?: string | null
  provider: "assemblyai" | string
  provider_mode: "disabled" | "live" | string
  provider_transcript_id?: string | null
  provider_status?: string | null
  status:
    | "queued"
    | "processing"
    | "review_ready"
    | "processed"
    | "failed"
    | "provider_disabled"
    | string
  review_status: "not_started" | "needs_review" | "approved" | string
  raw_artifact_version_id?: string | null
  normalized_artifact_version_id?: string | null
  redacted_artifact_version_id?: string | null
  redacted_audio_version_id?: string | null
  reviewed_document_version_id?: string | null
  caption_vtt_version_id?: string | null
  caption_srt_version_id?: string | null
  language_code?: string | null
  duration_ms?: number | null
  speaker_count: number
  segment_count: number
  word_count: number
  speakers_expected?: number | null
  speaker_options?: AssemblyAiSpeakerOptions | null
  word_search_terms: string[]
  prompt_preset?: string | null
  prompt?: string | null
  keyterms_prompt: string[]
  remove_audio_tags?: string | null
  redact_pii: boolean
  speech_models: string[]
  retryable: boolean
  error_code?: string | null
  error_message?: string | null
  created_at: string
  updated_at: string
  completed_at?: string | null
  reviewed_at?: string | null
}

export interface TranscriptSegment {
  segment_id: string
  id: string
  matter_id: string
  document_id: string
  transcription_job_id: string
  source_span_id?: string | null
  ordinal: number
  paragraph_ordinal?: number | null
  speaker_label?: string | null
  speaker_name?: string | null
  channel?: string | null
  text: string
  redacted_text?: string | null
  time_start_ms: number
  time_end_ms: number
  confidence: number
  review_status: string
  edited: boolean
  created_at: string
  updated_at: string
}

export interface TranscriptSpeaker {
  speaker_id: string
  id: string
  matter_id: string
  document_id: string
  transcription_job_id: string
  speaker_label: string
  display_name?: string | null
  role?: string | null
  confidence?: number | null
  segment_count: number
  created_at: string
  updated_at: string
}

export interface TranscriptReviewChange {
  review_change_id: string
  id: string
  matter_id: string
  document_id: string
  transcription_job_id: string
  target_type: string
  target_id: string
  field: string
  before?: string | null
  after?: string | null
  created_by: string
  created_at: string
}

export interface TranscriptionJobResponse {
  job: TranscriptionJob
  segments: TranscriptSegment[]
  speakers: TranscriptSpeaker[]
  review_changes: TranscriptReviewChange[]
  raw_artifact_version?: DocumentVersion | null
  normalized_artifact_version?: DocumentVersion | null
  redacted_artifact_version?: DocumentVersion | null
  redacted_audio_version?: DocumentVersion | null
  reviewed_document_version?: DocumentVersion | null
  caption_vtt_version?: DocumentVersion | null
  caption_srt_version?: DocumentVersion | null
  caption_vtt?: string | null
  caption_srt?: string | null
  warnings: string[]
}

export interface AssemblyAiSpeakerOptions {
  min_speakers_expected?: number | null
  max_speakers_expected?: number | null
}

export interface AssemblyAiTranscriptListQuery {
  limit?: number
  status?: "queued" | "processing" | "completed" | "error" | string
  created_on?: string
  before_id?: string
  after_id?: string
  throttled_only?: boolean
}

export interface AssemblyAiTranscriptPageDetails {
  limit: number
  result_count: number
  current_url: string
  prev_url?: string | null
  next_url?: string | null
}

export interface AssemblyAiTranscriptListItem {
  id: string
  resource_url: string
  status: "queued" | "processing" | "completed" | "error" | string
  created: string
  completed?: string | null
  audio_url: string
  error?: string | null
}

export interface AssemblyAiTranscriptListResponse {
  page_details: AssemblyAiTranscriptPageDetails
  transcripts: AssemblyAiTranscriptListItem[]
}

export interface AssemblyAiTranscriptDeleteResponse {
  id: string
  status: string
  deleted: boolean
  provider_response: Record<string, unknown>
}

export interface ExtractedEntity {
  id: string
  type:
    | "person"
    | "org"
    | "date"
    | "money"
    | "address"
    | "location"
    | "statute"
    | "legalCitation"
    | "case"
    | "obligation"
    | "party"
    | "other"
  value: string
  normalized?: string
  confidence: number
  spans: { chunkId: string; start: number; end: number }[]
}

export interface DocumentChunk {
  id: string
  chunk_id?: string
  document_id?: string
  heading?: string
  page: number
  text: string
  tokens: number
  document_version_id?: string | null
  object_blob_id?: string | null
  source_span_id?: string | null
  byte_start?: number | null
  byte_end?: number | null
  char_start?: number | null
  char_end?: number | null
}

export interface DocumentClause {
  id: string
  chunkId: string
  type: "obligation" | "right" | "warranty" | "indemnity" | "term" | "remedy" | "definition" | "notice" | "other"
  label: string
  start: number
  end: number
  confidence: number
  summary: string
  linkedProvisionIds: string[]
}

export interface DocumentIssue {
  id: string
  type: "contradiction" | "missing_signature" | "stale_date" | "low_ocr" | "pii" | "ambiguity" | "other"
  severity: "high" | "med" | "low"
  status: "open" | "resolved" | "ignored"
  label: string
  title?: string
  detail?: string
  chunkId?: string
}

export interface MatterDocument {
  // Primary identity
  id: string
  document_id: string // alias of id for snake-case consumers
  title: string
  filename: string
  kind: DocumentKind
  document_type: DocumentKind // alias of kind
  party?: string
  pages: number
  pageCount: number // alias of pages
  bytes: number
  fileSize: string // human readable, e.g. "2.4 MB"
  dateUploaded: string
  dateFiled?: string
  summary: string
  status: ProcessingStatus
  processing_status: ProcessingStatus // alias of status
  is_exhibit: boolean
  exhibit_label?: string
  facts_extracted: number
  citations_found: number
  contradictions_flagged: number
  entities: ExtractedEntity[]
  chunks: DocumentChunk[]
  clauses: DocumentClause[]
  linkedFacts: ExtractedFact[]
  issues: DocumentIssue[]
  matter_id?: string
  mime_type?: string
  file_hash?: string
  uploaded_at: string
  source?: "user_upload" | "public_records" | "generated" | "system" | string
  confidentiality?: "private" | "filed" | "public" | "sealed" | string
  date_observed?: string | null
  parties_mentioned: string[]
  entities_mentioned: string[]
  linked_claim_ids?: string[]
  folder: string
  storage_provider?: "local" | "r2" | string
  storage_status?: StorageStatus
  storage_bucket?: string | null
  storage_key?: string | null
  content_etag?: string | null
  upload_expires_at?: string | null
  deleted_at?: string | null
  library_path?: string | null
  archived_at?: string | null
  archived_reason?: string | null
  original_relative_path?: string | null
  upload_batch_id?: string | null
  object_blob_id?: string | null
  current_version_id?: string | null
  ingestion_run_ids?: string[]
  source_spans?: SourceSpan[]
  extracted_text?: string | null
}

// ===== Parties =====

export interface Party {
  id: string
  party_id?: string
  matter_id?: string
  name: string
  role: "plaintiff" | "defendant" | "third_party" | "witness" | "judge" | "attorney" | "agency" | "court"
  partyType: "individual" | "entity" | "government" | "court"
  party_type?: "individual" | "entity" | "government" | "court"
  representedBy?: string | null
  represented_by?: string | null
  contactEmail?: string
  contact_email?: string
  contactPhone?: string
  notes?: string
}

export type MatterParty = Party

// ===== Facts =====

export interface FactCitation {
  documentId: string
  chunkId?: string
  page?: number
  quote?: string
  snippet?: string
}

export interface ExtractedFact {
  id: string
  fact_id?: string
  matter_id?: string
  statement: string
  text?: string
  fact_type?: string
  date?: string | null
  status: FactStatus
  confidence: number
  disputed: boolean
  tags: string[]
  sourceDocumentIds: string[]
  source_evidence_ids?: string[]
  contradicted_by_evidence_ids?: string[]
  supports_claim_ids?: string[]
  supports_defense_ids?: string[]
  used_in_draft_ids?: string[]
  party_id?: string
  needs_verification?: boolean
  citations: FactCitation[]
  source_spans?: SourceSpan[]
  notes?: string
}

// ===== Timeline =====

export interface TimelineEvent {
  id: string
  event_id?: string
  matter_id?: string
  date: string
  title: string
  kind: "communication" | "filing" | "service" | "payment" | "notice" | "incident" | "meeting" | "court" | "deadline" | "other"
  category: string
  status?: "complete" | "open" | "missed"
  label?: string
  description?: string
  sourceDocumentId?: string
  source_document_id?: string
  factId?: string
  party_ids?: string[]
  linked_fact_ids?: string[]
  linked_claim_ids?: string[]
  source_span_ids?: string[]
  text_chunk_ids?: string[]
  suggestion_id?: string | null
  agent_run_id?: string | null
  date_confidence?: number
  disputed?: boolean
}

export interface TimelineAgentRun {
  agent_run_id: string
  id: string
  matter_id: string
  subject_type: string
  subject_id?: string | null
  agent_type: string
  scope_type: string
  scope_ids: string[]
  input_hash?: string | null
  pipeline_version: string
  extractor_version: string
  prompt_template_id?: string | null
  provider: string
  model?: string | null
  mode: string
  provider_mode: string
  status: string
  message: string
  produced_suggestion_ids: string[]
  warnings: string[]
  started_at?: string | null
  completed_at?: string | null
  duration_ms?: number | null
  error_code?: string | null
  error_message?: string | null
  deterministic_candidate_count: number
  provider_enriched_count: number
  provider_rejected_count: number
  duplicate_candidate_count: number
  stored_suggestion_count: number
  preserved_review_count: number
  created_at: string
}

export interface TimelineSuggestion {
  suggestion_id: string
  id: string
  matter_id: string
  date: string
  date_text: string
  date_confidence: number
  title: string
  description?: string | null
  kind: TimelineEvent["kind"] | string
  source_type: "document_index" | "document" | "source_span" | "work_product_ast" | "matter_graph" | string
  source_document_id?: string | null
  source_span_ids: string[]
  text_chunk_ids: string[]
  linked_fact_ids: string[]
  linked_claim_ids: string[]
  work_product_id?: string | null
  block_id?: string | null
  agent_run_id?: string | null
  index_run_id?: string | null
  dedupe_key?: string | null
  cluster_id?: string | null
  duplicate_of_suggestion_id?: string | null
  agent_explanation?: string | null
  agent_confidence?: number | null
  status: "suggested" | "approved" | "rejected" | "needs_attention" | string
  warnings: string[]
  approved_event_id?: string | null
  created_at: string
  updated_at: string
}

export interface TimelineSuggestResponse {
  enabled: boolean
  mode: string
  message: string
  suggestions: TimelineSuggestion[]
  agent_run?: TimelineAgentRun | null
  warnings: string[]
}

export interface TimelineSuggestionApprovalResponse {
  suggestion: TimelineSuggestion
  event: TimelineEvent
}

// ===== Claims, Counterclaims, Defenses =====

export type ClaimKind = "claim" | "counterclaim" | "defense"
export type ClaimElementStatus = "supported" | "weak" | "rebutted" | "missing" | "unknown"
export type ClaimRisk = "low" | "medium" | "high"

export interface ClaimElement {
  id: string
  element_id?: string
  title: string
  text?: string
  description: string
  status: ClaimElementStatus
  legalAuthority?: string
  authority?: string
  authorities?: { citation: string; canonical_id: string; reason?: string; pinpoint?: string }[]
  supportingFactIds: string[]
  satisfied?: boolean
  fact_ids?: string[]
  evidence_ids?: string[]
  missing_facts?: string[]
}

export interface Claim {
  id: string
  claim_id?: string
  defense_id?: string
  matter_id?: string
  kind: ClaimKind
  title: string
  name?: string
  cause: string
  count_label?: string
  claim_type?: string
  theory: string
  legal_theory?: string
  against: string
  damages?: Array<{ category: string; amount: string; theory?: string }>
  remedies?: string[]
  risk: ClaimRisk
  risk_level?: RiskLevel
  status: ClaimStatus
  elements: ClaimElement[]
  fact_ids?: string[]
  evidence_ids?: string[]
  defense_ids?: string[]
  authorities?: { citation: string; canonical_id: string; reason?: string }[]
  supportingFactIds: string[]
  counterArguments: { id: string; text?: string; argument?: string; response?: string; severity: "high" | "med" | "low" }[]
}

export interface CaseDefense {
  defense_id: string
  matter_id: string
  name: string
  basis: string
  status: DefenseStatus
  applies_to_claim_ids: string[]
  required_facts: string[]
  fact_ids: string[]
  evidence_ids: string[]
  authorities: { citation: string; canonical_id: string; reason?: string }[]
  viability: RiskLevel
}

export interface CaseEvidence {
  evidence_id: string
  matter_id: string
  document_id: string
  source_span: string
  quote: string
  evidence_type: string
  strength: EvidenceStrength
  confidence: number
  exhibit_label?: string
  supports_fact_ids: string[]
  contradicts_fact_ids: string[]
  source_spans?: SourceSpan[]
}

// ===== Deadlines & Tasks =====

export type DeadlineSeverity = "critical" | "warning" | "info"
export type DeadlineStatus = "open" | "complete" | "missed" | "waived"

export interface DeadlineTask {
  id: string
  label: string
  done: boolean
  assignee?: string
}

export interface Deadline {
  id: string
  deadline_id?: string
  matter_id?: string
  title: string
  description: string
  category: "filing" | "service" | "discovery" | "trial" | "appeal" | "agency" | "case" | "other" | string
  kind: "statutory" | "court_order" | "rule" | "agency" | "self" | "manual" | string
  dueDate: string
  due_date: string // alias
  daysRemaining: number
  days_remaining: number // alias
  severity: DeadlineSeverity
  source: string // human readable source description
  sourceCitation?: string // ORS or rule citation
  source_citation?: string
  sourceCanonicalId?: string
  source_canonical_id?: string
  statuteRef?: string
  computedFrom?: string // event id, e.g. "service of complaint on 2024-03-12"
  triggered_by_event_id?: string
  status: DeadlineStatus
  notes?: string
  owner?: string
  tasks: DeadlineTask[]
}

export interface CaseTask {
  task_id: string
  matter_id: string
  title: string
  status: TaskStatus
  priority: TaskPriority
  due_date: string | null
  assigned_to: string | null
  related_claim_ids: string[]
  related_document_ids: string[]
  related_deadline_id?: string
  source: "user" | "ai_suggestion" | "deadline" | string
  description?: string
}

// ===== Drafts =====

export type DraftStatus = "draft" | "review" | "final" | "filed"
export type DraftKind =
  | "complaint"
  | "answer"
  | "motion"
  | "demand_letter"
  | "public_records_request"
  | "legal_memo"
  | "agency_complaint"
  | "declaration"
  | "exhibit_list"

export interface DraftCitation {
  id: string
  chunkId?: string
  sourceId: string // doc id, fact id, or canonical_id
  sourceKind: "document" | "statute" | "case" | "rule" | "fact"
  page?: number
  shortLabel: string // e.g. "ORS 90.453"
  fullLabel: string // e.g. "ORS 90.453(2) (Habitability — landlord duties)"
  snippet?: string
  verified: boolean
}

export interface DraftSuggestion {
  id: string
  kind: "tighten" | "add_authority" | "add_fact" | "rewrite" | "tone" | "structure" | "factcheck" | "citecheck" | "insert"
  original: string
  proposed: string
  rationale: string
  sources: Array<{ id: string; label: string } | string>
  confidence: number
}

export interface DraftComment {
  id?: string
  author: string
  body: string
  timestamp: string
}

export interface DraftSection {
  id: string
  heading: string
  tone?: "formal" | "neutral" | "persuasive"
  body: string // plain or lightly-formatted text; one paragraph per blank line
  citations: DraftCitation[]
  comments: DraftComment[]
  suggestions: DraftSuggestion[]
}

export interface CiteCheckIssue {
  id: string
  citationId: string
  kind: "missing_authority" | "stale" | "wrong_pinpoint" | "unverified" | "format" | "broken_link"
  severity: "high" | "med" | "low"
  message: string
  title?: string
  detail?: string
  sectionId?: string
}

export interface DraftVersion {
  id: string
  label: string
  timestamp: string
  author: string
  summary: string
}

export interface Draft {
  id: string
  draft_id?: string
  matter_id?: string
  title: string
  description: string
  kind: DraftKind
  draft_type?: DraftKind
  status: DraftStatus
  lastEdited: string
  created_at?: string
  updated_at?: string
  wordCount: number
  word_count?: number
  sections: DraftSection[]
  paragraphs: DraftParagraph[]
  factcheck_summary: Record<ParagraphFactCheck, number>
  citeCheckIssues: CiteCheckIssue[]
  versions: DraftVersion[]
}

export interface DraftParagraph {
  paragraph_id: string
  index: number
  role: string
  heading_level?: number
  text: string
  fact_ids: string[]
  evidence_ids: string[]
  authorities: { citation: string; canonical_id: string; pinpoint?: string }[]
  factcheck_status: ParagraphFactCheck
  factcheck_note?: string
}

// ===== Validation findings and authority search =====

export interface CaseFactCheckFinding {
  finding_id: string
  id?: string
  matter_id: string
  draft_id: string
  paragraph_id?: string | null
  finding_type: string
  severity: "info" | "warning" | "serious" | "blocking" | string
  message: string
  source_fact_ids: string[]
  source_evidence_ids: string[]
  status: "open" | "resolved" | "ignored" | string
}

export interface CaseCitationCheckFinding {
  finding_id: string
  id?: string
  matter_id: string
  draft_id: string
  citation: string
  canonical_id?: string | null
  finding_type: string
  severity: "info" | "warning" | "serious" | "blocking" | string
  message: string
  status: "open" | "resolved" | "ignored" | string
}

export interface CaseAiActionResponse<T> {
  enabled: boolean
  mode: "live" | "template" | "deterministic" | "disabled" | string
  message: string
  result: T | null
}

export interface CaseAuthoritySearchItem {
  id: string
  kind: string
  authority_family?: string | null
  authority_level?: number | null
  authority_tier?: string | null
  source_role?: string | null
  primary_law?: boolean | null
  official_commentary?: boolean | null
  controlling_weight?: number | null
  citation?: string | null
  canonical_id?: string | null
  title?: string | null
  snippet: string
  score: number
  href: string
}

export interface CaseAuthoritySearchResponse {
  matter_id: string
  query: string
  source: string
  results: CaseAuthoritySearchItem[]
  warnings: string[]
}

export type AuthorityTargetType =
  | "matter"
  | "claim"
  | "element"
  | "draft_paragraph"
  | "complaint_count"
  | "complaint_paragraph"
  | "complaint_sentence"
  | "citation_use"
  | "work_product"
  | "work_product_block"
  | "work_product_anchor"

export interface AuthorityAttachmentResponse {
  matter_id: string
  target_type: AuthorityTargetType
  target_id: string
  authority: { citation: string; canonical_id: string; reason?: string; pinpoint?: string }
  attached: boolean
}

// ===== Shared Work Product Editor =====

export type WorkProductType =
  | "complaint"
  | "answer"
  | "motion"
  | "declaration"
  | "affidavit"
  | "memo"
  | "notice"
  | "letter"
  | "exhibit_list"
  | "proposed_order"
  | "custom"

export interface WorkProduct {
  work_product_id: string
  id: string
  matter_id: string
  title: string
  product_type: WorkProductType | string
  status: string
  review_status: string
  setup_stage: string
  source_draft_id?: string | null
  source_complaint_id?: string | null
  created_at: string
  updated_at: string
  profile: WorkProductProfile
  document_ast: WorkProductDocument
  blocks: WorkProductBlock[]
  marks: WorkProductMark[]
  anchors: WorkProductAnchor[]
  findings: WorkProductFinding[]
  artifacts: WorkProductArtifact[]
  history: ChangeSet[]
  ai_commands: WorkProductAiCommandState[]
  formatting_profile: FormattingProfile
  rule_pack: RulePack
}

export interface WorkProductDocument {
  schema_version: string
  document_id: string
  work_product_id: string
  matter_id: string
  draft_id?: string | null
  document_type: string
  product_type: string
  title: string
  metadata: WorkProductMetadata
  blocks: WorkProductBlock[]
  links: WorkProductLink[]
  citations: WorkProductCitationUse[]
  exhibits: WorkProductExhibitReference[]
  rule_findings: WorkProductFinding[]
  tombstones: WorkProductBlock[]
  created_at: string
  updated_at: string
}

export interface WorkProductMetadata {
  work_product_type?: string | null
  document_title?: string | null
  jurisdiction?: string | null
  court?: string | null
  county?: string | null
  case_number?: string | null
  rule_pack_id?: string | null
  template_id?: string | null
  formatting_profile_id?: string | null
  parties?: {
    plaintiffs: string[]
    defendants: string[]
    petitioners: string[]
    respondents: string[]
  } | null
  status: string
  created_at?: string | null
  updated_at?: string | null
  created_by?: string | null
  last_modified_by?: string | null
}

export interface TextRange {
  start_offset: number
  end_offset: number
  quote?: string | null
}

export interface WorkProductLink {
  link_id: string
  source_block_id: string
  source_text_range?: TextRange | null
  target_type: string
  target_id: string
  relation: string
  confidence?: number | null
  created_by: string
  created_at: string
}

export interface WorkProductCitationUse {
  citation_use_id: string
  source_block_id: string
  source_text_range?: TextRange | null
  raw_text: string
  normalized_citation?: string | null
  target_type: string
  target_id?: string | null
  pinpoint?: string | null
  status: string
  resolver_message?: string | null
  created_at: string
}

export interface WorkProductExhibitReference {
  exhibit_reference_id: string
  source_block_id: string
  source_text_range?: TextRange | null
  label: string
  exhibit_id?: string | null
  document_id?: string | null
  page_range?: string | null
  status: string
  created_at: string
}

export interface LegalImpactSummary {
  affected_counts: string[]
  affected_elements: string[]
  affected_facts: string[]
  affected_evidence: string[]
  affected_authorities: string[]
  affected_exhibits: string[]
  support_status_before?: string | null
  support_status_after?: string | null
  qc_warnings_added: string[]
  qc_warnings_resolved: string[]
  blocking_issues_added: string[]
  blocking_issues_resolved: string[]
}

export interface VersionTargetSummary {
  target_type: string
  target_id: string
  label?: string | null
}

export interface VersionChangeSummary {
  text_changes: number
  support_changes: number
  citation_changes: number
  authority_changes: number
  qc_changes: number
  export_changes: number
  ai_changes: number
  targets_changed: VersionTargetSummary[]
  risk_level: string
  user_summary: string
}

export interface ChangeSet {
  change_set_id: string
  id: string
  matter_id: string
  subject_id: string
  branch_id: string
  snapshot_id: string
  parent_snapshot_id?: string | null
  title: string
  summary: string
  reason?: string | null
  actor_type: string
  actor_id?: string | null
  source: string
  created_at: string
  change_ids: string[]
  legal_impact: LegalImpactSummary
}

export interface VersionSnapshot {
  snapshot_id: string
  id: string
  matter_id: string
  subject_type: string
  subject_id: string
  product_type: string
  profile_id: string
  branch_id: string
  sequence_number: number
  title: string
  message?: string | null
  created_at: string
  created_by: string
  actor_id?: string | null
  snapshot_type: string
  parent_snapshot_ids: string[]
  document_hash: string
  support_graph_hash: string
  qc_state_hash: string
  formatting_hash: string
  manifest_hash: string
  manifest_ref?: string | null
  full_state_ref?: string | null
  full_state_inline?: WorkProduct | Record<string, unknown> | null
  summary: VersionChangeSummary
}

export interface SnapshotManifest {
  manifest_id: string
  id: string
  snapshot_id: string
  matter_id: string
  subject_id: string
  manifest_hash: string
  entry_count: number
  storage_ref?: string | null
  created_at: string
}

export interface SnapshotEntityState {
  entity_state_id: string
  id: string
  manifest_id: string
  snapshot_id: string
  matter_id: string
  subject_id: string
  entity_type: string
  entity_id: string
  entity_hash: string
  state_ref?: string | null
  state_inline?: Record<string, unknown> | null
}

export interface VersionChange {
  change_id: string
  id: string
  change_set_id: string
  snapshot_id: string
  matter_id: string
  subject_type: string
  subject_id: string
  branch_id: string
  target_type: string
  target_id: string
  operation: string
  before?: Record<string, unknown> | string | null
  after?: Record<string, unknown> | string | null
  before_hash?: string | null
  after_hash?: string | null
  summary: string
  legal_impact: LegalImpactSummary
  ai_audit_id?: string | null
  created_at: string
  created_by: string
  actor_id?: string | null
}

export interface VersionBranch {
  branch_id: string
  id: string
  matter_id: string
  subject_type: string
  subject_id: string
  name: string
  description?: string | null
  created_from_snapshot_id: string
  current_snapshot_id: string
  branch_type: string
  created_at: string
  updated_at: string
  archived_at?: string | null
}

export interface LegalSupportUse {
  support_use_id: string
  id: string
  matter_id: string
  subject_id: string
  branch_id: string
  target_type: string
  target_id: string
  source_type: string
  source_id: string
  relation: string
  status: string
  quote?: string | null
  pinpoint?: string | null
  confidence?: number | null
  created_snapshot_id: string
  retired_snapshot_id?: string | null
}

export interface AIEditAudit {
  ai_audit_id: string
  id: string
  matter_id: string
  subject_type: string
  subject_id: string
  target_type: string
  target_id: string
  command: string
  prompt_template_id?: string | null
  model?: string | null
  provider_mode: string
  input_fact_ids: string[]
  input_evidence_ids: string[]
  input_authority_ids: string[]
  input_snapshot_id: string
  output_text?: string | null
  inserted_text?: string | null
  user_action: string
  warnings: string[]
  created_at: string
}

export interface CaseHistoryMilestone {
  milestone_id: string
  id: string
  matter_id: string
  subject_id: string
  snapshot_id: string
  label: string
  notes?: string | null
  created_at: string
}

export interface RestoreVersionResponse {
  restored: boolean
  dry_run: boolean
  warnings: string[]
  snapshot_id: string
  change_set?: ChangeSet | null
  result?: WorkProduct | null
}

export interface VersionTextDiff {
  target_type: string
  target_id: string
  title: string
  status: string
  before?: string | null
  after?: string | null
}

export interface VersionLayerDiff {
  layer: string
  target_type: string
  target_id: string
  title: string
  status: string
  before_hash?: string | null
  after_hash?: string | null
  before_summary?: string | null
  after_summary?: string | null
}

export interface CompareVersionsResponse {
  matter_id: string
  subject_id: string
  from_snapshot_id: string
  to_snapshot_id: string
  layers: string[]
  summary: VersionChangeSummary
  text_diffs: VersionTextDiff[]
  layer_diffs: VersionLayerDiff[]
}

export interface WorkProductProfile {
  profile_id: string
  product_type: string
  name: string
  jurisdiction: string
  version: string
  route_slug: string
  required_block_roles: string[]
  optional_block_roles: string[]
  supports_rich_text: boolean
}

export interface WorkProductBlock {
  block_id: string
  id: string
  matter_id: string
  work_product_id: string
  type: string
  block_type: string
  role: string
  title: string
  text: string
  order_index: number
  ordinal: number
  parent_block_id?: string | null
  parent_id?: string | null
  children: WorkProductBlock[]
  links: string[]
  citations: string[]
  exhibits: string[]
  rule_finding_ids: string[]
  paragraph_number?: number | null
  sentence_index?: number | null
  sentence_id?: string | null
  section_kind?: string | null
  count_number?: number | null
  claim_type?: string | null
  defendants: string[]
  requested_relief: string[]
  support_status?: string | null
  created_at: string
  updated_at: string
  fact_ids: string[]
  evidence_ids: string[]
  authorities: { citation: string; canonical_id: string; reason?: string; pinpoint?: string }[]
  mark_ids: string[]
  locked: boolean
  tombstoned: boolean
  deleted_at?: string | null
  source_document_id?: string | null
  source_span_id?: string | null
  created_by?: string | null
  last_modified_by?: string | null
  provenance?: Record<string, string> | null
  review_status: string
  prosemirror_json?: Record<string, unknown> | null
}

export interface AstPatch {
  patch_id: string
  draft_id?: string | null
  work_product_id?: string | null
  base_document_hash?: string | null
  base_snapshot_id?: string | null
  created_by: "user" | "ai" | "system" | string
  reason?: string | null
  operations: AstOperation[]
  created_at: string
}

export type AstOperation =
  | { op: "insert_block"; parent_id?: string | null; after_block_id?: string | null; block: WorkProductBlock }
  | { op: "update_block"; block_id: string; before?: Record<string, unknown> | null; after: Record<string, unknown> }
  | { op: "delete_block"; block_id: string; tombstone?: boolean }
  | { op: "move_block"; block_id: string; parent_id?: string | null; after_block_id?: string | null }
  | { op: "split_block"; block_id: string; offset: number; new_block_id: string }
  | { op: "merge_blocks"; first_block_id: string; second_block_id: string }
  | { op: "renumber_paragraphs" }
  | { op: "add_link"; link: WorkProductLink }
  | { op: "remove_link"; link_id: string }
  | { op: "add_citation"; citation: WorkProductCitationUse }
  | { op: "resolve_citation"; citation_use_id: string; normalized_citation?: string; target_type?: string; target_id?: string; status?: string }
  | { op: "remove_citation"; citation_use_id: string }
  | { op: "add_exhibit_reference"; exhibit: WorkProductExhibitReference }
  | { op: "resolve_exhibit_reference"; exhibit_reference_id: string; exhibit_id?: string; status?: string }
  | { op: "add_rule_finding"; finding: WorkProductFinding }
  | { op: "resolve_rule_finding"; finding_id: string; status: string }
  | { op: "apply_template"; template_id: string }

export interface AstValidationIssue {
  code: string
  message: string
  severity?: string | null
  blocking: boolean
  target_type?: string | null
  target_id?: string | null
}

export interface AstValidationResponse {
  valid: boolean
  errors: AstValidationIssue[]
  warnings: AstValidationIssue[]
}

export interface AstMarkdownResponse {
  markdown: string
  warnings: string[]
}

export interface AstDocumentResponse {
  document_ast: WorkProductDocument
  warnings: string[]
}

export interface AstRenderedResponse {
  html?: string | null
  plain_text?: string | null
  warnings: string[]
}

export interface WorkProductMark {
  mark_id: string
  id: string
  matter_id: string
  work_product_id: string
  block_id: string
  mark_type: string
  from_offset: number
  to_offset: number
  label: string
  target_type: string
  target_id: string
  status: string
}

export interface WorkProductAnchor {
  anchor_id: string
  id: string
  matter_id: string
  work_product_id: string
  block_id: string
  anchor_type: string
  target_type: string
  target_id: string
  relation: string
  citation?: string | null
  canonical_id?: string | null
  pinpoint?: string | null
  quote?: string | null
  status: string
}

export interface WorkProductFinding {
  finding_id: string
  id: string
  matter_id: string
  work_product_id: string
  rule_id: string
  rule_pack_id?: string | null
  source_citation?: string | null
  source_url?: string | null
  category: string
  severity: "info" | "warning" | "serious" | "blocking" | string
  target_type: string
  target_id: string
  message: string
  explanation: string
  suggested_fix: string
  auto_fix_available: boolean
  primary_action: WorkProductAction
  status: "open" | "resolved" | "ignored" | string
  created_at: string
  updated_at: string
}

export interface WorkProductAction {
  action_id: string
  label: string
  action_type: string
  href?: string | null
  target_type: string
  target_id: string
}

export interface WorkProductArtifact {
  artifact_id: string
  id: string
  matter_id: string
  work_product_id: string
  format: string
  profile: string
  mode: string
  status: string
  download_url: string
  page_count: number
  generated_at: string
  warnings: string[]
  content_preview: string
  snapshot_id?: string | null
  artifact_hash?: string | null
  render_profile_hash?: string | null
  qc_status_at_export?: string | null
  changed_since_export?: boolean | null
  immutable?: boolean | null
  object_blob_id?: string | null
  size_bytes?: number | null
  mime_type?: string | null
  storage_status?: string | null
}

export interface WorkProductHistoryEvent {
  event_id: string
  id: string
  matter_id: string
  work_product_id: string
  event_type: string
  target_type: string
  target_id: string
  summary: string
  timestamp: string
}

export interface WorkProductAiCommandState {
  command_id: string
  label: string
  status: string
  mode: string
  description: string
  last_message?: string | null
}

export interface WorkProductPreviewResponse {
  work_product_id: string
  matter_id: string
  html: string
  plain_text: string
  page_count: number
  warnings: string[]
  generated_at: string
  review_label: string
}

// ===== Structured Complaint Editor =====

export interface ComplaintDraft {
  complaint_id: string
  id: string
  matter_id: string
  title: string
  status: string
  review_status: string
  setup_stage: string
  active_profile_id: string
  created_at: string
  updated_at: string
  caption: ComplaintCaption
  parties: ComplaintParty[]
  sections: ComplaintSection[]
  counts: ComplaintCount[]
  paragraphs: PleadingParagraph[]
  relief: ReliefRequest[]
  signature: SignatureBlock
  certificate_of_service?: CertificateOfService | null
  formatting_profile: FormattingProfile
  rule_pack: RulePack
  findings: RuleCheckFinding[]
  export_artifacts: ExportArtifact[]
  history: ComplaintHistoryEvent[]
  next_actions: ComplaintNextAction[]
  ai_commands: ComplaintAiCommandState[]
  filing_packet: FilingPacket
  import_provenance?: ComplaintImportProvenance | null
}

export interface ComplaintCaption {
  court_name: string
  county: string
  case_number?: string | null
  document_title: string
  plaintiff_names: string[]
  defendant_names: string[]
  jury_demand: boolean
  jurisdiction: string
  venue: string
}

export interface ComplaintParty {
  party_id: string
  matter_party_id?: string | null
  name: string
  role: string
  party_type: string
  represented_by?: string | null
}

export interface ComplaintSection {
  section_id: string
  id: string
  matter_id: string
  complaint_id: string
  title: string
  section_type: string
  ordinal: number
  paragraph_ids: string[]
  count_ids: string[]
  review_status: string
}

export interface ComplaintCount {
  count_id: string
  id: string
  matter_id: string
  complaint_id: string
  ordinal: number
  title: string
  claim_id?: string | null
  legal_theory: string
  against_party_ids: string[]
  element_ids: string[]
  fact_ids: string[]
  evidence_ids: string[]
  authorities: { citation: string; canonical_id: string; reason?: string; pinpoint?: string }[]
  relief_ids: string[]
  paragraph_ids: string[]
  incorporation_range?: string | null
  health: string
  weaknesses: string[]
}

export interface PleadingParagraph {
  paragraph_id: string
  id: string
  matter_id: string
  complaint_id: string
  section_id?: string | null
  count_id?: string | null
  number: number
  ordinal: number
  display_number?: string | null
  original_label?: string | null
  source_span_id?: string | null
  import_provenance?: ComplaintImportProvenance | null
  role: string
  text: string
  sentences: PleadingSentence[]
  fact_ids: string[]
  evidence_uses: EvidenceUse[]
  citation_uses: CitationUse[]
  exhibit_references: ExhibitReference[]
  rule_finding_ids: string[]
  locked: boolean
  review_status: string
}

export interface PleadingSentence {
  sentence_id: string
  id: string
  matter_id: string
  complaint_id: string
  paragraph_id: string
  ordinal: number
  text: string
  fact_ids: string[]
  evidence_use_ids: string[]
  citation_use_ids: string[]
  review_status: string
}

export interface CitationUse {
  citation_use_id: string
  id: string
  matter_id: string
  complaint_id: string
  target_type: string
  target_id: string
  citation: string
  canonical_id?: string | null
  pinpoint?: string | null
  quote?: string | null
  status: string
  currentness: string
  scope_warning?: string | null
}

export interface EvidenceUse {
  evidence_use_id: string
  id: string
  matter_id: string
  complaint_id: string
  target_type: string
  target_id: string
  fact_id?: string | null
  evidence_id?: string | null
  document_id?: string | null
  source_span_id?: string | null
  relation: string
  quote?: string | null
  status: string
}

export interface ExhibitReference {
  exhibit_reference_id: string
  id: string
  matter_id: string
  complaint_id: string
  target_type: string
  target_id: string
  exhibit_label: string
  document_id?: string | null
  evidence_id?: string | null
  status: string
}

export interface ReliefRequest {
  relief_id: string
  id: string
  matter_id: string
  complaint_id: string
  category: string
  text: string
  amount?: string | null
  authority_ids: string[]
  supported: boolean
}

export interface SignatureBlock {
  name: string
  bar_number?: string | null
  firm?: string | null
  address: string
  phone: string
  email: string
  signature_date?: string | null
}

export interface CertificateOfService {
  certificate_id: string
  method: string
  served_parties: string[]
  service_date?: string | null
  text: string
  review_status: string
}

export interface FormattingProfile {
  profile_id: string
  name: string
  jurisdiction: string
  line_numbers: boolean
  double_spaced: boolean
  first_page_top_blank_inches: number
  margin_top_inches: number
  margin_bottom_inches: number
  margin_left_inches: number
  margin_right_inches: number
  font_family: string
  font_size_pt: number
}

export interface RulePack {
  rule_pack_id: string
  name: string
  jurisdiction: string
  version: string
  effective_date: string
  rule_profile: RuleProfileSummary
  rules: RuleDefinition[]
}

export interface RuleProfileSummary {
  jurisdiction_id: string
  court_id?: string | null
  court?: string | null
  filing_date?: string | null
  utcr_edition_id?: string | null
  slr_edition_id?: string | null
  active_statewide_order_ids: string[]
  active_local_order_ids: string[]
  active_out_of_cycle_amendment_ids: string[]
  currentness_warnings: string[]
  resolver_endpoint: string
}

export interface RuleDefinition {
  rule_id: string
  source_citation: string
  source_url: string
  severity: string
  target_type: string
  category: string
  message: string
  explanation: string
  suggested_fix: string
  auto_fix_available: boolean
}

export interface RuleCheckFinding {
  finding_id: string
  id: string
  matter_id: string
  complaint_id: string
  rule_id: string
  category: string
  severity: "info" | "warning" | "serious" | "blocking" | string
  target_type: string
  target_id: string
  message: string
  explanation: string
  suggested_fix: string
  primary_action: ComplaintAction
  status: "open" | "resolved" | "ignored" | string
  created_at: string
  updated_at: string
}

export interface ComplaintAction {
  action_id: string
  label: string
  action_type: string
  href?: string | null
  target_type: string
  target_id: string
}

export interface ComplaintNextAction {
  action_id: string
  priority: string
  label: string
  detail: string
  action_type: string
  target_type: string
  target_id: string
  href?: string | null
}

export interface ExportArtifact {
  artifact_id: string
  id: string
  matter_id: string
  complaint_id: string
  format: string
  profile: string
  mode: string
  status: string
  download_url: string
  page_count: number
  generated_at: string
  warnings: string[]
  content_preview: string
  object_blob_id?: string | null
  size_bytes?: number | null
  mime_type?: string | null
  storage_status?: string | null
}

export interface ComplaintHistoryEvent {
  event_id: string
  id: string
  matter_id: string
  complaint_id: string
  event_type: string
  target_type: string
  target_id: string
  summary: string
  timestamp: string
}

export interface ComplaintAiCommandState {
  command_id: string
  label: string
  status: string
  mode: string
  description: string
  last_message?: string | null
}

export interface FilingPacket {
  packet_id: string
  matter_id: string
  complaint_id: string
  status: string
  items: FilingPacketItem[]
  warnings: string[]
}

export interface FilingPacketItem {
  item_id: string
  label: string
  item_type: string
  status: string
  artifact_id?: string | null
  warning?: string | null
}

export interface ComplaintPreviewResponse {
  complaint_id: string
  matter_id: string
  html: string
  plain_text: string
  page_count: number
  warnings: string[]
  generated_at: string
  review_label: string
}

// ===== Ask Matter (chat) =====

export interface MatterChatCitation {
  id: string
  indexLabel?: string
  kind?: "document" | "fact" | "claim" | "statute" | "case" | "rule" | "source"
  refId?: string
  sourceId: string
  sourceKind: "document" | "fact" | "statute" | "case" | "rule"
  shortLabel: string
  fullLabel: string
  title?: string
  page?: number
  snippet?: string
  chunkId?: string
}

export interface MatterChatMessage {
  id: string
  role: "user" | "assistant" | "system"
  content: string
  timestamp: string
  reasoning?: string[]
  confidence?: number
  citations: MatterChatCitation[]
}

export interface ChatThread {
  id: string
  title: string
  preview: string
  date: string
  lastMessageAt?: string
  messageCount: number
}

export interface MatterAskMessage {
  message_id: string
  role: "user" | "assistant" | "system"
  text: string
  timestamp: string
  context_used?: {
    document_ids: string[]
    fact_ids: string[]
    evidence_ids: string[]
    authorities: { citation: string; canonical_id: string }[]
  }
  caveats?: string[]
}

// ===== Milestones (dashboard) =====

export interface Milestone {
  id: string
  title: string
  date: string
  kind: "filed" | "served" | "answered" | "ruled" | "trial" | "settled" | "appealed" | "intake" | "discovery"
  status: "complete" | "active" | "upcoming"
  label?: string
  description?: string
}

// ===== Full Matter =====

export interface Matter extends MatterSummary {
  id: string // alias for matter_id
  title: string // alias for name
  documents: MatterDocument[]
  parties: MatterParty[]
  facts: ExtractedFact[]
  timeline: TimelineEvent[]
  timeline_suggestions: TimelineSuggestion[]
  timeline_agent_runs: TimelineAgentRun[]
  claims: Claim[] // includes counterclaims and defenses (kind discriminator)
  evidence: CaseEvidence[]
  defenses: CaseDefense[]
  deadlines: Deadline[]
  tasks: CaseTask[]
  drafts: Draft[]
  work_products: WorkProduct[]
  fact_check_findings: CaseFactCheckFinding[]
  citation_check_findings: CaseCitationCheckFinding[]
  chatHistory: MatterChatMessage[]
  recentThreads: ChatThread[]
  milestones: Milestone[]
}

// ===== Convenience aliases (legacy / spec compatibility) =====

export type CaseDocument = MatterDocument
export type CaseFact = ExtractedFact
export type CaseClaim = Claim
export type CaseEvent = TimelineEvent
export type CaseDeadline = Deadline
export type CaseDraft = Draft

export interface DocumentExtraction {
  document_id: string
  summary: string
  key_dates: Array<{ date: string; description: string; page?: number }>
  parties: Array<{ name: string; role: string }>
  entities: Array<{ name: string; type: string }>
  possible_facts: Array<{ text: string; confidence: number; suggested_status: FactStatus }>
  possible_claims: Array<{ name: string; rationale: string; viability: RiskLevel }>
  possible_defenses: Array<{ name: string; rationale: string; viability: RiskLevel }>
  citations: Array<{ raw: string; resolved_canonical_id?: string; status: string }>
  contradictions: Array<{ text: string; against_quote?: string }>
  exhibit_notes: string[]
}
