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
  | "exhibit"
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
  | "other"

export type ProcessingStatus = "queued" | "processing" | "processed" | "failed"

export interface ExtractedEntity {
  id: string
  type: "person" | "org" | "date" | "money" | "address" | "statute" | "case" | "other"
  value: string
  normalized?: string
  confidence: number
  spans: { chunkId: string; start: number; end: number }[]
}

export interface DocumentChunk {
  id: string
  heading?: string
  page: number
  text: string
  tokens: number
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
  linkedFacts: string[] // fact ids
  issues: DocumentIssue[]
}

// ===== Parties =====

export interface Party {
  id: string
  name: string
  role: "plaintiff" | "defendant" | "third_party" | "witness" | "judge" | "attorney" | "agency" | "court"
  partyType: "individual" | "entity" | "government" | "court"
  representedBy?: string | null
  contactEmail?: string
  contactPhone?: string
  notes?: string
}

// ===== Facts =====

export interface FactCitation {
  documentId: string
  chunkId?: string
  page?: number
  quote?: string
}

export interface ExtractedFact {
  id: string
  statement: string
  date?: string | null
  confidence: number
  disputed: boolean
  tags: string[]
  sourceDocumentIds: string[]
  citations: FactCitation[]
  notes?: string
}

// ===== Timeline =====

export interface TimelineEvent {
  id: string
  date: string
  title: string
  kind: "communication" | "filing" | "service" | "payment" | "notice" | "incident" | "meeting" | "court" | "deadline" | "other"
  status?: "complete" | "open" | "missed"
  label?: string
  description?: string
  sourceDocumentId?: string
  factId?: string
}

// ===== Claims, Counterclaims, Defenses =====

export type ClaimKind = "claim" | "counterclaim" | "defense"
export type ClaimElementStatus = "supported" | "weak" | "rebutted" | "missing" | "unknown"
export type ClaimRisk = "low" | "medium" | "high"

export interface ClaimElement {
  id: string
  title: string
  description: string
  status: ClaimElementStatus
  legalAuthority?: string
  supportingFactIds: string[]
}

export interface Claim {
  id: string
  kind: ClaimKind
  title: string
  cause: string
  theory: string
  against: string
  damages?: string
  risk?: ClaimRisk
  elements: ClaimElement[]
  counterArguments: { id: string; text: string; severity: "high" | "med" | "low" }[]
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
  sourceCanonicalId?: string
  statuteRef?: string
  computedFrom?: string // event id, e.g. "service of complaint on 2024-03-12"
  status: DeadlineStatus
  owner?: string
  tasks: DeadlineTask[]
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
  kind: "tighten" | "add_authority" | "add_fact" | "rewrite" | "tone" | "structure" | "factcheck" | "citecheck"
  original: string
  proposed: string
  rationale: string
  sources: { id: string; label: string }[]
  confidence: number
}

export interface DraftComment {
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
  title: string
  description: string
  kind: DraftKind
  status: DraftStatus
  lastEdited: string
  wordCount: number
  sections: DraftSection[]
  citeCheckIssues: CiteCheckIssue[]
  versions: DraftVersion[]
}

// ===== Ask Matter (chat) =====

export interface MatterChatCitation {
  id: string
  sourceId: string
  sourceKind: "document" | "fact" | "statute" | "case" | "rule"
  shortLabel: string
  fullLabel: string
  page?: number
  snippet?: string
}

export interface MatterChatMessage {
  id: string
  role: "user" | "assistant" | "system"
  content: string
  timestamp: string
  reasoning?: string
  confidence?: number
  citations: MatterChatCitation[]
}

export interface ChatThread {
  id: string
  title: string
  preview: string
  date: string
  messageCount: number
}

// ===== Milestones (dashboard) =====

export interface Milestone {
  id: string
  title: string
  date: string
  kind: "filed" | "served" | "answered" | "ruled" | "trial" | "settled" | "appealed" | "intake" | "discovery"
  status: "complete" | "active" | "upcoming"
  label?: string
}

// ===== Full Matter =====

export interface Matter extends MatterSummary {
  id: string // alias for matter_id
  title: string // alias for name
  documents: MatterDocument[]
  parties: Party[]
  facts: ExtractedFact[]
  timeline: TimelineEvent[]
  claims: Claim[] // includes counterclaims and defenses (kind discriminator)
  deadlines: Deadline[]
  drafts: Draft[]
  chatHistory: MatterChatMessage[]
  recentThreads: ChatThread[]
  milestones: Milestone[]
}

// ===== Convenience aliases (legacy / spec compatibility) =====

export type CaseDocument = MatterDocument
export type CaseFact = ExtractedFact
export type CaseClaim = Claim
export type CaseDeadline = Deadline
export type CaseDraft = Draft
