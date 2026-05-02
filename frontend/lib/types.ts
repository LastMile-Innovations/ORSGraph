// ORSGraph type system — shaped to the JSON specs in the product architecture doc.

export type QCStatus = "pass" | "warning" | "fail"
export type LegalStatus = "active" | "repealed" | "renumbered" | "amended"
export type AuthorityLevel = "constitution" | "statute" | "rule" | "regulation" | "official_commentary" | "case" | "agency_guidance" | "secondary"

export type ChunkType =
  | "full_statute"
  | "contextual_provision"
  | "definition_block"
  | "exception_block"
  | "deadline_block"
  | "penalty_block"
  | "citation_context"

export type ProvisionSignal = "definition" | "exception" | "deadline" | "penalty" | "citation"

export type ProvisionType = "section" | "subsection" | "paragraph" | "subparagraph" | "clause"

export interface StatuteIdentity {
  canonical_id: string // e.g. "or:ors:3.010"
  citation: string // e.g. "ORS 3.010"
  title: string
  jurisdiction: string
  corpus: string
  authority_family?: string
  authority_level?: number
  authority_tier?: string
  source_role?: string
  primary_law?: boolean
  official_commentary?: boolean
  controlling_weight?: number
  chapter: string
  status: LegalStatus
  edition: number
}

export interface SourceDocument {
  source_id: string
  url: string
  retrieved_at: string
  raw_hash: string
  normalized_hash: string
  edition_year: number
  parser_profile: string
  parser_warnings: string[]
}

export interface LegalTextVersion {
  version_id: string
  effective_date: string
  end_date: string | null
  is_current: boolean
  text: string
  source_documents: string[]
}

export interface Provision {
  provision_id: string
  display_citation: string // e.g. "ORS 3.010(1)(a)"
  provision_type: ProvisionType
  parent_id: string | null
  text: string
  text_preview: string
  signals: ProvisionSignal[]
  cites_count: number
  cited_by_count: number
  chunk_count: number
  qc_status: QCStatus
  status: LegalStatus
  children?: Provision[]
}

export interface CitationMention {
  mention_id: string
  raw_text: string
  resolved_target: string | null
  resolved_citation: string | null
  source_provision_id: string
  qc_status: QCStatus
}

export interface InboundCitation {
  source_canonical_id: string
  source_citation: string
  source_title: string
  source_provision: string
  context_snippet: string
}

export interface OutboundCitation {
  target_canonical_id: string | null
  target_citation: string
  context_snippet: string
  source_provision: string
  resolved: boolean
}

export interface Chunk {
  chunk_id: string
  chunk_type: ChunkType
  source_kind: "statute" | "provision"
  source_id: string
  text: string
  embedding_policy: "primary" | "secondary" | "none"
  answer_policy: "preferred" | "supporting" | "context_only"
  search_weight: number
  embedded: boolean
  parser_confidence: number
}

export interface Definition {
  definition_id: string
  term: string
  text: string
  source_provision: string
  scope: string
}

export interface Exception {
  exception_id: string
  text: string
  applies_to_provision: string
  source_provision: string
}

export interface Deadline {
  deadline_id: string
  description: string
  duration: string
  trigger: string
  source_provision: string
}

export interface Penalty {
  penalty_id: string
  description: string
  category: "civil" | "criminal" | "administrative"
  source_provision: string
}

export interface QCNote {
  note_id: string
  level: "info" | "warning" | "fail"
  category: string
  message: string
  related_id: string | null
}

export interface QCSummary {
  status: QCStatus
  passed_checks: number
  total_checks: number
  notes: QCNote[]
}

export interface StatuteSummaryCounts {
  provision_count: number
  citation_counts: {
    outbound: number
    inbound: number
  }
  semantic_counts: {
    obligations: number
    exceptions: number
    deadlines: number
    penalties: number
    definitions: number
  }
}

export interface StatutePageResponse {
  identity: StatuteIdentity
  current_version: LegalTextVersion
  versions: LegalTextVersion[]
  provisions: Provision[]
  chunks: Chunk[]
  definitions: Definition[]
  exceptions: Exception[]
  deadlines: Deadline[]
  penalties: Penalty[]
  outbound_citations: OutboundCitation[]
  inbound_citations: InboundCitation[]
  source_documents: SourceDocument[]
  qc: QCSummary
  summary_counts?: StatuteSummaryCounts
  source_notes?: string[]
}

export interface ProvisionInspectorData {
  parent_statute: {
    canonical_id: string
    citation: string
    title: string
    chapter: string
    status: LegalStatus
    edition: number
  }
  provision: Provision
  ancestors: { provision_id: string; citation: string }[]
  children: Provision[]
  siblings: { provision_id: string; citation: string }[]
  chunks: Chunk[]
  outbound_citations: OutboundCitation[]
  inbound_citations: InboundCitation[]
  definitions: Definition[]
  exceptions: Exception[]
  deadlines: Deadline[]
  qc_notes: QCNote[]
}

export interface GraphInfo {
  canonical_id?: string
  version_id?: string
  provision_id?: string
  connected_node_count?: number
  citation_count?: number
  cited_by_count?: number
}

export interface ScoreBreakdown {
  exact?: number
  keyword?: number
  vector?: number
  rerank?: number
  graph?: number
  authority?: number
  expansion?: number
  penalties?: number
}

export interface RerankInfo {
  enabled: boolean
  model?: string
  candidate_count?: number
  returned_count?: number
  total_tokens?: number
}

export interface RetrievalInfo {
  exact_candidates: number
  fulltext_candidates: number
  vector_candidates: number
  filtered_candidates?: number
  capped_candidates?: number
  graph_expanded_candidates: number
  reranked_candidates: number
}

export interface EmbeddingsInfo {
  enabled: boolean
  model?: string
  profile?: string
  dimension?: number
}

export interface SearchResult {
  id?: string
  result_type?: string
  kind?: string
  authority_family?: string
  authority_type?: string
  authority_level?: number
  authority_tier?: string
  jurisdiction_id?: string
  source_role?: string
  primary_law?: boolean
  official_commentary?: boolean
  controlling_weight?: number
  corpus_id?: string
  citation?: string
  title?: string
  chapter?: string
  status?: string
  snippet: string
  score: number
  vector_score?: number
  fulltext_score?: number
  graph_score?: number
  rerank_score?: number
  pre_rerank_score?: number
  rank_source?: "exact" | "parent" | "keyword" | "keyword-fallback" | "graph-expanded" | "vector" | "graph" | "rerank"
  score_breakdown?: ScoreBreakdown
  semantic_types?: string[]
  source_backed?: boolean
  qc_warnings?: string[]
  href?: string
  source_provision?: string
  edition_year?: number
  cited_by_count?: number
  cites_count?: number
  qc_status?: QCStatus
  source_id?: string
  matched_chunk_type?: ChunkType
  source?: {
    source_document_id?: string
    provision_id?: string
    version_id?: string
    chunk_id?: string
    source_note_id?: string
  }
  graph?: GraphInfo
}

export type DirectMatchType = "exact_provision" | "exact_statute" | "parent_statute" | "none"

export interface QueryCitation {
  raw: string
  normalized: string
  base: string
  chapter: string
  section: string
  subsections: string[]
  parent?: string
}

export interface QueryCitationRange {
  raw: string
  start: string
  end: string
  chapter: string
}

export interface QueryExpansionTerm {
  term: string
  normalized_term?: string | null
  kind: string
  source_id?: string | null
  source_citation?: string | null
  score: number
}

export interface SearchTimingInfo {
  total_ms: number
  retrieval_ms: number
  graph_ms: number
  rerank_ms: number
}

export interface SearchAnalysis {
  normalized_query: string
  intent: string
  citations: QueryCitation[]
  ranges: QueryCitationRange[]
  inferred_chapter?: string | null
  residual_text?: string | null
  expansion_terms: QueryExpansionTerm[]
  expansion_count: number
  applied_filters: string[]
  timings: SearchTimingInfo
}

export interface SearchResponse {
  query: string
  mode: string
  total: number
  limit?: number
  offset?: number
  results: SearchResult[]
  facets?: SearchFacets
  warnings?: string[]
  analysis: SearchAnalysis
  retrieval?: RetrievalInfo
  embeddings?: EmbeddingsInfo
  rerank?: RerankInfo
}

export interface SearchFacets {
  kinds: Record<string, number>
  chapters: Record<string, number>
  statuses: Record<string, number>
  semantic_types: Record<string, number>
  source_backed: {
    true: number
    false: number
  }
  qc_warnings: Record<string, number>
}

export interface SuggestResult {
  label: string
  kind: string
  href: string
  citation?: string | null
  canonical_id?: string | null
  match_type: DirectMatchType
  score: number
}

export interface DirectOpenResponse {
  matched: boolean
  match_type: DirectMatchType
  normalized_query: string
  citation: string
  canonical_id: string
  href: string
  parent?: {
    citation: string
    canonical_id: string
    href: string
  } | null
}

export interface AskAnswer {
  question: string
  short_answer: string
  controlling_law: { citation: string; canonical_id: string; reason: string }[]
  relevant_provisions: { citation: string; provision_id: string; text_preview: string }[]
  definitions: { term: string; text: string; source: string }[]
  exceptions: { text: string; source: string }[]
  deadlines: { description: string; duration: string; source: string }[]
  citations: string[]
  caveats: string[]
  retrieved_chunks: { chunk_id: string; chunk_type: ChunkType; score: number; preview: string }[]
  qc_notes: string[]
}

export interface GraphNode {
  id: string
  label: string
  type: "Statute" | "Provision" | "CitationMention" | "Chapter" | "Definition" | "Exception" | "Deadline" | "Penalty" | "Source"
  status?: LegalStatus
  qc_status?: QCStatus
}

export interface GraphEdge {
  id: string
  source: string
  target: string
  type:
    | "CITES"
    | "MENTIONS_CITATION"
    | "RESOLVES_TO"
    | "HAS_VERSION"
    | "CONTAINS"
    | "DERIVED_FROM"
    | "DEFINES"
    | "EXCEPTION_TO"
    | "HAS_DEADLINE"
    | "ANNOTATES"
    | "INTERPRETS"
    | "HAS_COMMENTARY"
}

export interface QCRunSummary {
  run_id: string
  ran_at: string
  duration_ms: number
  status: QCStatus
  total_checks: number
  passed: number
  warnings: number
  failures: number
  panels: QCPanel[]
}

export interface QCPanel {
  panel_id: string
  title: string
  category:
    | "source"
    | "parse"
    | "chunk"
    | "citation"
    | "graph"
    | "embedding"
  status: QCStatus
  count: number
  description: string
  rows: QCRow[]
}

export interface QCRow {
  id: string
  citation: string
  message: string
  level: "info" | "warning" | "fail"
}

export interface CorpusStatus {
  editionYear: number
  source: string
  lastUpdated?: string
  lastQcRun?: string
  qcStatus: QCStatus | "unknown"
  counts: {
    sections: number
    versions: number
    provisions: number
    retrievalChunks: number
    citationMentions: number
    citesEdges: number
    semanticNodes: number
    sourceNotes: number
    amendments: number
    sessionLaws: number
    neo4jNodes: number
    neo4jRelationships: number
  }
  citations: {
    total: number
    resolved: number
    unresolved: number
    citesEdges: number
    coveragePercent: number
  }
  embeddings: {
    model?: string
    profile?: string
    embedded: number
    totalEligible: number
    coveragePercent: number
    status: "not_started" | "partial" | "complete" | "error" | "unknown"
  }
}

export interface SystemHealth {
  api: "connected" | "mock" | "offline" | "unknown"
  neo4j: "connected" | "offline" | "unknown"
  qc: QCStatus | "unknown"
  graphMaterialization: "complete" | "partial" | "failed" | "unknown"
  embeddings: "not_started" | "partial" | "complete" | "error" | "unknown"
  rerank: "enabled" | "disabled" | "missing_key" | "unknown"
  lastSeededAt?: string
  lastCheckedAt?: string
}

export interface HomeAction {
  title: string
  description: string
  href: string
  icon: string
  variant?: "primary" | "default"
  badges?: string[]
  status?: "ready" | "coming_soon" | "internal" | "warning"
}

export interface GraphInsightCard {
  title: string
  value: string
  subtitle?: string
  href?: string
  state?: "ok" | "warning" | "error" | "unknown"
}

export interface FeaturedStatute {
  citation: string
  title: string
  chapter: string
  href: string
  status: LegalStatus | "unknown"
  semanticTypes: string[]
  citedByCount?: number
  sourceBacked?: boolean
}

export interface BuildInfo {
  appVersion: string
  apiVersion?: string
  graphEdition?: string
  environment: "development" | "staging" | "production"
}

export interface HomePageData {
  corpus: CorpusStatus
  health: SystemHealth
  actions: HomeAction[]
  insights: GraphInsightCard[]
  featuredStatutes: FeaturedStatute[]
  build: BuildInfo
}

// ===== Sources index =====

export interface SourceIndexEntry extends SourceDocument {
  title: string
  jurisdiction: string
  scope: string // e.g. "ORS Chapter 3"
  byte_size: number
  ingestion_status: "ingested" | "queued" | "failed"
}

// ===== Fact-Check =====

export type FactCheckStatus =
  | "supported"
  | "partially_supported"
  | "unsupported"
  | "contradicted"
  | "wrong_citation"
  | "stale_law"
  | "needs_source"

export interface FactCheckFinding {
  finding_id: string
  paragraph_id: string
  paragraph_index: number
  claim: string
  status: FactCheckStatus
  confidence: number
  explanation: string
  suggested_fix: string | null
  sources: {
    citation: string
    canonical_id: string | null
    quote: string | null
    edition_year: number
    status: LegalStatus
  }[]
}

export interface FactCheckDocument {
  document_id: string
  title: string
  doc_type: "complaint" | "answer" | "motion" | "brief" | "memo" | "demand_letter" | "ai_generated"
  word_count: number
  uploaded_at: string
  paragraphs: {
    paragraph_id: string
    index: number
    text: string
  }[]
}

export interface FactCheckReport {
  document: FactCheckDocument
  findings: FactCheckFinding[]
  summary: {
    total: number
    supported: number
    partial: number
    unsupported: number
    contradicted: number
    wrong_citation: number
    stale_law: number
    needs_source: number
  }
  citation_table: {
    raw_citation: string
    resolved_citation: string | null
    canonical_id: string | null
    edition_year: number | null
    status: LegalStatus | "unresolved"
    qc_status: QCStatus
    occurrences: number[] // paragraph indices
  }[]
}

// ===== Complaint Analyzer =====

export type ResponseType =
  | "admit"
  | "deny"
  | "deny_in_part"
  | "lack_knowledge"
  | "legal_conclusion"
  | "needs_review"

export type RiskLevel = "high" | "medium" | "low"

export interface ComplaintParty {
  party_id: string
  name: string
  role: "plaintiff" | "defendant" | "third_party"
  type: "individual" | "entity" | "government"
}

export interface ComplaintClaim {
  claim_id: string
  count_label: string // e.g. "Count I"
  title: string
  cause_of_action: string
  required_elements: {
    element_id: string
    text: string
    alleged: boolean
    proven: boolean
    authority: string
  }[]
  alleged_facts: string[]
  missing_facts: string[]
  potential_defenses: { name: string; authority: string; viability: RiskLevel }[]
  relevant_law: { citation: string; canonical_id: string; reason: string }[]
  risk_level: RiskLevel
}

export interface ComplaintAllegation {
  allegation_id: string
  paragraph: number
  text: string
  suggested_response: ResponseType
  reason: string
  evidence_needed: string[]
}

export interface ComplaintDeadline {
  deadline_id: string
  description: string
  due_date: string
  days_remaining: number
  source_citation: string
  severity: "critical" | "warning" | "info"
}

export interface ComplaintAnalysis {
  complaint_id: string
  filename: string
  uploaded_at: string
  court: string
  case_number: string
  user_role: "defendant" | "plaintiff"
  service_date: string
  summary: string
  parties: ComplaintParty[]
  claims: ComplaintClaim[]
  allegations: ComplaintAllegation[]
  deadlines: ComplaintDeadline[]
  defense_candidates: { name: string; authority: string; rationale: string; viability: RiskLevel }[]
  motion_candidates: { name: string; authority: string; basis: string }[]
  counterclaim_candidates: { name: string; authority: string; basis: string }[]
  evidence_checklist: { item: string; obtained: boolean; needed_for: string }[]
  draft_answer_preview: string
}

// ===== Drafting Studio =====

export type DraftType =
  | "complaint"
  | "answer"
  | "motion"
  | "demand_letter"
  | "public_records_request"
  | "legal_memo"
  | "agency_complaint"
  | "declaration"

export type ParagraphRole = "facts" | "law" | "analysis" | "relief" | "heading"

export type FactCheckParagraphStatus = "supported" | "unsupported" | "needs_review"

export interface DraftParagraph {
  paragraph_id: string
  role: ParagraphRole
  heading_level?: 1 | 2 | 3
  text: string
  source_authorities: { citation: string; canonical_id: string; pinpoint?: string }[]
  user_facts: string[]
  factcheck_status: FactCheckParagraphStatus
  factcheck_note?: string
}

export interface DraftDocument {
  draft_id: string
  title: string
  draft_type: DraftType
  matter_id: string | null
  created_at: string
  updated_at: string
  paragraphs: DraftParagraph[]
}

export interface AuthoritySuggestion {
  citation: string
  canonical_id: string
  title: string
  status: LegalStatus
  authority_family?: string
  authority_level?: number
  authority_tier?: string
  source_role?: string
  primary_law?: boolean
  official_commentary?: boolean
  controlling_weight?: number
  edition_year: number
  snippet: string
  cites_count: number
  cited_by_count: number
  signals: ProvisionSignal[]
}

// ===== Admin / Graph Ops =====

export type RunStatus = "running" | "succeeded" | "failed" | "queued" | "partial"

export interface OpsRun {
  run_id: string
  kind: "crawl" | "parse" | "resolver" | "seed" | "embedding"
  started_at: string
  ended_at: string | null
  duration_ms: number | null
  status: RunStatus
  items_processed: number
  items_succeeded: number
  items_failed: number
  notes: string
  cost_usd?: number
}

export interface ServiceHealth {
  service: string
  status: "ok" | "degraded" | "down"
  latency_p50_ms: number
  latency_p99_ms: number
  error_rate: number
  uptime_30d: number
}

export interface CostBucket {
  vendor: string
  category: "embedding" | "llm" | "storage" | "compute"
  spend_30d_usd: number
  spend_7d_usd: number
  forecast_30d_usd: number
}

export interface AdminDashboard {
  metrics: {
    chapters_fetched: number
    sections_parsed: number
    provisions_parsed: number
    chunks_created: number
    citations_extracted: number
    citations_resolved: number
    chunks_embedded: number
    embedding_failures: number
    neo4j_nodes: number
    neo4j_edges: number
    qc_status: QCStatus
  }
  recent_runs: OpsRun[]
  services: ServiceHealth[]
  costs: CostBucket[]
}

// ===== Developer API Portal =====

export type HttpMethod = "GET" | "POST" | "PUT" | "DELETE" | "PATCH"

export interface ApiParam {
  name: string
  in: "path" | "query" | "body" | "header"
  type: string
  required: boolean
  description: string
  example?: string
}

export interface ApiEndpoint {
  endpoint_id: string
  group: string
  method: HttpMethod
  path: string
  summary: string
  description: string
  params: ApiParam[]
  request_example?: string
  response_example: string
  status_codes: { code: number; description: string }[]
}

export interface ApiGroup {
  group_id: string
  title: string
  description: string
  endpoints: ApiEndpoint[]
}
