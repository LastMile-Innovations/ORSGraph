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
  | "other"

export type MatterStatus = "active" | "intake" | "stayed" | "closed" | "appeal"

export type MatterSide = "plaintiff" | "defendant" | "petitioner" | "respondent" | "neutral"
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

export type ProcessingStatus = "queued" | "processing" | "processed" | "failed" | "unsupported"
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
  quote?: string | null
  extraction_method: string
  confidence: number
  review_status: "unreviewed" | "approved" | "rejected" | "unavailable" | string
  unavailable_reason?: string | null
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
  object_blob_id?: string | null
  current_version_id?: string | null
  ingestion_run_ids?: string[]
  source_spans?: SourceSpan[]
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
  date_confidence?: number
  disputed?: boolean
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
  category: "filing" | "service" | "discovery" | "trial" | "appeal" | "agency" | "other"
  kind: "statutory" | "court_order" | "rule" | "agency" | "self"
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

export type AuthorityTargetType = "claim" | "element" | "draft_paragraph"

export interface AuthorityAttachmentResponse {
  matter_id: string
  target_type: AuthorityTargetType
  target_id: string
  authority: { citation: string; canonical_id: string; reason?: string; pinpoint?: string }
  attached: boolean
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
  claims: Claim[] // includes counterclaims and defenses (kind discriminator)
  evidence: CaseEvidence[]
  defenses: CaseDefense[]
  deadlines: Deadline[]
  tasks: CaseTask[]
  drafts: Draft[]
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
