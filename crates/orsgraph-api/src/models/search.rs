use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub r#type: Option<String>,
    pub chapter: Option<String>,
    pub status: Option<String>,
    pub mode: Option<SearchMode>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub include: Option<String>,
    pub semantic_type: Option<String>,
    pub current_only: Option<bool>,
    pub source_backed: Option<bool>,
    pub has_citations: Option<bool>,
    pub has_deadlines: Option<bool>,
    pub has_penalties: Option<bool>,
    pub needs_review: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    Auto,
    Keyword,
    Citation,
    Semantic,
    Hybrid,
}

impl Default for SearchMode {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub normalized_query: String,
    pub intent: String,
    pub mode: SearchMode,
    pub total: usize,
    pub limit: u32,
    pub offset: u32,
    pub results: Vec<SearchResult>,
    pub facets: Option<SearchFacets>,
    pub warnings: Vec<String>,
    pub retrieval: RetrievalInfo,
    pub embeddings: Option<EmbeddingsInfo>,
    pub rerank: Option<RerankInfo>,
}

#[derive(Debug, Serialize, Default)]
pub struct RetrievalInfo {
    pub exact_candidates: usize,
    pub fulltext_candidates: usize,
    pub vector_candidates: usize,
    pub graph_expanded_candidates: usize,
    pub reranked_candidates: usize,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingsInfo {
    pub enabled: bool,
    pub model: Option<String>,
    pub profile: Option<String>,
    pub dimension: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct RerankInfo {
    pub enabled: bool,
    pub model: Option<String>,
    pub candidate_count: Option<usize>,
    pub returned_count: Option<usize>,
    pub total_tokens: Option<usize>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SearchResult {
    pub id: String,
    pub kind: String,
    pub citation: Option<String>,
    pub title: Option<String>,
    pub chapter: Option<String>,
    pub status: Option<String>,
    pub snippet: String,
    pub score: f32,
    pub vector_score: Option<f32>,
    pub fulltext_score: Option<f32>,
    pub graph_score: Option<f32>,
    pub rerank_score: Option<f32>,
    pub pre_rerank_score: Option<f32>,
    pub rank_source: Option<String>,
    pub score_breakdown: Option<ScoreBreakdown>,
    pub semantic_types: Vec<String>,
    pub source_backed: bool,
    pub qc_warnings: Vec<String>,
    pub href: String,
    pub source: Option<SourceInfo>,
    pub graph: Option<GraphInfo>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ScoreBreakdown {
    pub exact: Option<f32>,
    pub keyword: Option<f32>,
    pub vector: Option<f32>,
    pub rerank: Option<f32>,
    pub graph: Option<f32>,
    pub authority: Option<f32>,
    pub penalties: Option<f32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SourceInfo {
    pub source_document_id: Option<String>,
    pub provision_id: Option<String>,
    pub version_id: Option<String>,
    pub chunk_id: Option<String>,
    pub source_note_id: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct GraphInfo {
    pub canonical_id: Option<String>,
    pub version_id: Option<String>,
    pub provision_id: Option<String>,
    pub connected_node_count: Option<u64>,
    pub citation_count: Option<u64>,
    pub cited_by_count: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SearchFacets {
    pub kinds: std::collections::HashMap<String, usize>,
    pub chapters: std::collections::HashMap<String, usize>,
    pub statuses: std::collections::HashMap<String, usize>,
    pub semantic_types: std::collections::HashMap<String, usize>,
    pub source_backed: SourceBackedFacet,
    pub qc_warnings: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Serialize)]
pub struct SourceBackedFacet {
    pub r#true: usize,
    pub r#false: usize,
}

#[derive(Debug, Serialize)]
pub struct SuggestResult {
    pub label: String,
    pub kind: String,
    pub href: String,
}

#[derive(Debug, Serialize)]
pub struct DirectOpenResponse {
    pub matched: bool,
    pub kind: String,
    pub citation: String,
    pub canonical_id: String,
    pub href: String,
}
