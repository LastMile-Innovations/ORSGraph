use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub service: String,
    pub neo4j: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub nodes: u64,
    pub relationships: u64,
    pub chapters: u64,
    pub sections: u64,
    pub provisions: u64,
    pub chunks: u64,
    pub citations: u64,
    pub cites_edges: u64,
    pub semantic_nodes: u64,
    pub last_seeded_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub q: String,
    #[serde(default = "default_search_type")]
    pub r#type: String,
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

fn default_search_type() -> String {
    "all".to_string()
}

fn default_search_limit() -> usize {
    20
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub kind: String,
    pub id: String,
    pub citation: String,
    pub title: Option<String>,
    pub snippet: String,
    pub score: f64,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct StatuteDetailResponse {
    pub identity: StatuteIdentity,
    pub current_version: StatuteVersion,
    pub chapter: String,
    pub title: Option<String>,
    pub status: String,
    pub source_document: SourceDocument,
    pub provision_count: u64,
    pub citation_counts: CitationCounts,
    pub semantic_counts: SemanticCounts,
    pub source_notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct StatuteIdentity {
    pub canonical_id: String,
    pub citation: String,
    pub title: Option<String>,
    pub chapter: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct StatuteVersion {
    pub version_id: String,
    pub effective_date: String,
    pub end_date: Option<String>,
    pub is_current: bool,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct SourceDocument {
    pub source_id: String,
    pub url: String,
    pub edition_year: i32,
}

#[derive(Debug, Serialize)]
pub struct StatuteIndexResponse {
    pub items: Vec<StatuteIndexItem>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct StatuteIndexItem {
    pub canonical_id: String,
    pub citation: String,
    pub title: Option<String>,
    pub chapter: String,
    pub status: String,
    pub edition_year: i32,
}

#[derive(Debug, Serialize)]
pub struct SidebarResponse {
    pub corpus: SidebarCorpus,
    pub saved_searches: Vec<SidebarSavedSearch>,
    pub saved_statutes: Vec<SidebarStatute>,
    pub recent_statutes: Vec<SidebarStatute>,
    pub active_matter: Option<SidebarMatter>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct SidebarCorpus {
    pub jurisdiction: String,
    pub corpus: String,
    pub edition_year: i32,
    pub total_statutes: u64,
    pub chapters: Vec<SidebarChapter>,
}

#[derive(Debug, Serialize)]
pub struct SidebarChapter {
    pub chapter: String,
    pub label: String,
    pub count: u64,
    pub items: Vec<StatuteIndexItem>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SidebarSavedSearch {
    pub saved_search_id: String,
    pub query: String,
    pub results: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SidebarStatute {
    pub canonical_id: String,
    pub citation: String,
    pub title: Option<String>,
    pub chapter: String,
    pub status: String,
    pub edition_year: i32,
    pub saved_at: Option<String>,
    pub opened_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SidebarMatter {
    pub matter_id: String,
    pub name: String,
    pub status: String,
    pub updated_at: String,
    pub open_task_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct SaveSidebarSearchRequest {
    pub query: String,
    pub results: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SidebarStatuteRequest {
    pub canonical_id: String,
}

#[derive(Debug, Serialize)]
pub struct CitationCounts {
    pub outbound: u64,
    pub inbound: u64,
}

#[derive(Debug, Serialize)]
pub struct SemanticCounts {
    pub obligations: u64,
    pub exceptions: u64,
    pub deadlines: u64,
    pub penalties: u64,
    pub definitions: u64,
}

#[derive(Debug, Serialize)]
pub struct ProvisionsResponse {
    pub citation: String,
    pub provisions: Vec<ProvisionNode>,
}

#[derive(Debug, Serialize)]
pub struct ProvisionNode {
    pub provision_id: String,
    pub display_citation: String,
    pub local_path: Vec<String>,
    pub depth: usize,
    pub text: String,
    pub children: Vec<ProvisionNode>,
}

#[derive(Debug, Serialize)]
pub struct ProvisionDetailResponse {
    pub parent_statute: StatuteIndexItem,
    pub provision: ProvisionDetail,
    pub ancestors: Vec<ProvisionLink>,
    pub children: Vec<ProvisionDetail>,
    pub siblings: Vec<ProvisionLink>,
    pub chunks: Vec<ProvisionChunk>,
    pub outbound_citations: Vec<Citation>,
    pub inbound_citations: Vec<Citation>,
    pub definitions: Vec<DefinitionItem>,
    pub exceptions: Vec<ProvisionException>,
    pub deadlines: Vec<DeadlineItem>,
    pub qc_notes: Vec<QCNoteItem>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProvisionDetail {
    pub provision_id: String,
    pub display_citation: String,
    pub provision_type: String,
    pub parent_id: Option<String>,
    pub text: String,
    pub text_preview: String,
    pub signals: Vec<String>,
    pub cites_count: u64,
    pub cited_by_count: u64,
    pub chunk_count: u64,
    pub qc_status: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ProvisionLink {
    pub provision_id: String,
    pub citation: String,
}

#[derive(Debug, Serialize)]
pub struct ProvisionChunk {
    pub chunk_id: String,
    pub chunk_type: String,
    pub source_kind: String,
    pub source_id: String,
    pub text: String,
    pub embedding_policy: String,
    pub answer_policy: String,
    pub search_weight: f64,
    pub embedded: bool,
    pub parser_confidence: f64,
}

#[derive(Debug, Serialize)]
pub struct ProvisionException {
    pub exception_id: String,
    pub text: String,
    pub applies_to_provision: String,
    pub source_provision: String,
}

#[derive(Debug, Serialize)]
pub struct QCNoteItem {
    pub note_id: String,
    pub level: String,
    pub category: String,
    pub message: String,
    pub related_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CitationsResponse {
    pub citation: String,
    pub outbound: Vec<Citation>,
    pub inbound: Vec<Citation>,
    pub unresolved: Vec<Citation>,
}

#[derive(Debug, Serialize)]
pub struct Citation {
    pub target_canonical_id: Option<String>,
    pub target_citation: String,
    pub context_snippet: String,
    pub source_provision: String,
    pub resolved: bool,
}

#[derive(Debug, Serialize)]
pub struct SemanticsResponse {
    pub citation: String,
    pub obligations: Vec<SemanticItem>,
    pub exceptions: Vec<SemanticItem>,
    pub deadlines: Vec<DeadlineItem>,
    pub penalties: Vec<SemanticItem>,
    pub definitions: Vec<DefinitionItem>,
}

#[derive(Debug, Serialize)]
pub struct SemanticItem {
    pub text: String,
    pub source_provision: String,
}

#[derive(Debug, Serialize)]
pub struct DeadlineItem {
    pub description: String,
    pub duration: String,
    pub trigger: String,
    pub source_provision: String,
}

#[derive(Debug, Serialize)]
pub struct DefinitionItem {
    pub term: String,
    pub text: String,
    pub source_provision: String,
    pub scope: String,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub citation: String,
    pub source_notes: Vec<String>,
    pub amendments: Vec<Amendment>,
    pub session_laws: Vec<SessionLaw>,
    pub status_events: Vec<StatusEvent>,
}

#[derive(Debug, Serialize)]
pub struct Amendment {
    pub amendment_id: String,
    pub description: String,
    pub effective_date: String,
}

#[derive(Debug, Serialize)]
pub struct SessionLaw {
    pub session_law_id: String,
    pub citation: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct StatusEvent {
    pub event_id: String,
    pub event_type: String,
    pub date: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct GraphNeighborhoodRequest {
    pub id: Option<String>,
    pub citation: Option<String>,
    #[serde(default = "default_graph_depth")]
    pub depth: usize,
    #[serde(default = "default_graph_limit")]
    pub limit: usize,
    #[serde(default = "default_graph_mode")]
    pub mode: String,
    #[serde(default, alias = "relationshipTypes")]
    pub relationship_types: Option<String>,
    #[serde(default, alias = "nodeTypes")]
    pub node_types: Option<String>,
    #[serde(default, alias = "minConfidence")]
    pub min_confidence: Option<f64>,
    #[serde(default, alias = "includeChunks")]
    pub include_chunks: Option<bool>,
    #[serde(default, alias = "includeSimilarity")]
    pub include_similarity: Option<bool>,
    #[serde(default, alias = "similarityThreshold")]
    pub similarity_threshold: Option<f64>,
}

fn default_graph_depth() -> usize {
    1
}

fn default_graph_limit() -> usize {
    100
}

fn default_graph_mode() -> String {
    "legal".to_string()
}

#[derive(Debug, Serialize)]
pub struct GraphNeighborhoodResponse {
    pub center: Option<GraphNode>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub layout: Option<GraphLayoutHint>,
    pub stats: GraphStats,
}

#[derive(Debug, Serialize, Clone)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub labels: Vec<String>,
    pub citation: Option<String>,
    pub title: Option<String>,
    pub chapter: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "textSnippet")]
    pub text_snippet: Option<String>,
    pub size: Option<f64>,
    pub score: Option<f64>,
    #[serde(rename = "similarityScore")]
    pub similarity_score: Option<f64>,
    pub confidence: Option<f64>,
    #[serde(rename = "sourceBacked")]
    pub source_backed: Option<bool>,
    #[serde(rename = "qcWarnings")]
    pub qc_warnings: Vec<String>,
    pub metrics: Option<GraphNodeMetrics>,
    pub href: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    pub label: Option<String>,
    pub kind: String,
    pub weight: Option<f64>,
    pub confidence: Option<f64>,
    #[serde(rename = "similarityScore")]
    pub similarity_score: Option<f64>,
    #[serde(rename = "sourceBacked")]
    pub source_backed: Option<bool>,
    pub style: Option<GraphEdgeStyle>,
}

#[derive(Debug, Serialize, Clone)]
pub struct GraphNodeMetrics {
    pub degree: Option<u64>,
    #[serde(rename = "inDegree")]
    pub in_degree: Option<u64>,
    #[serde(rename = "outDegree")]
    pub out_degree: Option<u64>,
    pub pagerank: Option<f64>,
    #[serde(rename = "semanticCount")]
    pub semantic_count: Option<u64>,
    #[serde(rename = "citationCount")]
    pub citation_count: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct GraphEdgeStyle {
    pub dashed: bool,
    pub width: f64,
    pub color: String,
}

#[derive(Debug, Serialize)]
pub struct GraphLayoutHint {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct GraphStats {
    #[serde(rename = "nodeCount")]
    pub node_count: usize,
    #[serde(rename = "edgeCount")]
    pub edge_count: usize,
    pub truncated: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct QCSummaryResponse {
    pub node_counts_by_label: Vec<NodeCount>,
    pub relationship_counts_by_type: Vec<RelationshipCount>,
    pub orphan_counts: OrphanCounts,
    pub duplicate_counts: DuplicateCounts,
    pub embedding_readiness: EmbeddingReadiness,
    pub cites_coverage: CitesCoverage,
    pub last_qc_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NodeCount {
    pub label: String,
    pub count: u64,
}

#[derive(Debug, Serialize)]
pub struct RelationshipCount {
    pub rel_type: String,
    pub count: u64,
}

#[derive(Debug, Serialize)]
pub struct OrphanCounts {
    pub provisions: u64,
    pub chunks: u64,
    pub citations: u64,
}

#[derive(Debug, Serialize)]
pub struct DuplicateCounts {
    pub legal_text_identities: u64,
    pub provisions: u64,
    pub cites_relationships: u64,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingReadiness {
    pub total_chunks: u64,
    pub embedded_chunks: u64,
    pub coverage: f64,
}

#[derive(Debug, Serialize)]
pub struct CitesCoverage {
    pub total_citations: u64,
    pub resolved_citations: u64,
    pub coverage: f64,
}

#[derive(Debug, Deserialize)]
pub struct AskRequest {
    pub question: String,
    pub mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AskAnswerResponse {
    pub question: String,
    pub mode: String,
    pub short_answer: String,
    pub controlling_law: Vec<AskControllingLaw>,
    pub relevant_provisions: Vec<AskRelevantProvision>,
    pub definitions: Vec<AskSourceText>,
    pub exceptions: Vec<AskSourceText>,
    pub deadlines: Vec<AskDeadline>,
    pub citations: Vec<String>,
    pub caveats: Vec<String>,
    pub retrieved_chunks: Vec<AskRetrievedChunk>,
    pub qc_notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AskControllingLaw {
    pub citation: String,
    pub canonical_id: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct AskRelevantProvision {
    pub citation: String,
    pub provision_id: String,
    pub text_preview: String,
}

#[derive(Debug, Serialize)]
pub struct AskSourceText {
    pub term: Option<String>,
    pub text: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct AskDeadline {
    pub description: String,
    pub duration: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct AskRetrievedChunk {
    pub chunk_id: String,
    pub chunk_type: String,
    pub score: f32,
    pub preview: String,
}
