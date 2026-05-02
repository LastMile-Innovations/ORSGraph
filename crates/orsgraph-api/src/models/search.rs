use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub r#type: Option<String>,
    pub authority_family: Option<String>,
    pub authority_tier: Option<String>,
    pub jurisdiction: Option<String>,
    pub source_role: Option<String>,
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
    pub primary_law: Option<bool>,
    pub official_commentary: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchRetrievalFilters {
    pub result_type: Option<String>,
    pub authority_family: Option<String>,
    pub authority_tier: Option<String>,
    pub jurisdiction: Option<String>,
    pub source_role: Option<String>,
    pub chapter: Option<String>,
    pub status: Option<String>,
    pub semantic_type: Option<String>,
    pub current_only: bool,
    pub source_backed_only: bool,
    pub has_citations: bool,
    pub has_deadlines: bool,
    pub has_penalties: bool,
    pub needs_review: bool,
    pub primary_law: bool,
    pub official_commentary: bool,
}

impl SearchRetrievalFilters {
    pub fn from_query(query: &SearchQuery) -> Self {
        Self {
            result_type: normalized_filter(query.r#type.as_deref()),
            authority_family: normalized_authority_filter(query.authority_family.as_deref()),
            authority_tier: normalized_filter(query.authority_tier.as_deref()),
            jurisdiction: normalized_filter(query.jurisdiction.as_deref()),
            source_role: normalized_filter(query.source_role.as_deref()),
            chapter: query
                .chapter
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            status: normalized_filter(query.status.as_deref()),
            semantic_type: normalized_filter(query.semantic_type.as_deref()),
            current_only: query.current_only.unwrap_or(false),
            source_backed_only: query.source_backed.unwrap_or(false),
            has_citations: query.has_citations.unwrap_or(false),
            has_deadlines: query.has_deadlines.unwrap_or(false),
            has_penalties: query.has_penalties.unwrap_or(false),
            needs_review: query.needs_review.unwrap_or(false),
            primary_law: query.primary_law.unwrap_or(false),
            official_commentary: query.official_commentary.unwrap_or(false),
        }
    }

    pub fn applied_filter_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        if self.result_type.is_some() {
            names.push("type".to_string());
        }
        if self.authority_family.is_some() {
            names.push("authority_family".to_string());
        }
        if self.authority_tier.is_some() {
            names.push("authority_tier".to_string());
        }
        if self.jurisdiction.is_some() {
            names.push("jurisdiction".to_string());
        }
        if self.source_role.is_some() {
            names.push("source_role".to_string());
        }
        if self.chapter.is_some() {
            names.push("chapter".to_string());
        }
        if self.status.is_some() {
            names.push("status".to_string());
        }
        if self.semantic_type.is_some() {
            names.push("semantic_type".to_string());
        }
        if self.current_only {
            names.push("current_only".to_string());
        }
        if self.source_backed_only {
            names.push("source_backed".to_string());
        }
        if self.has_citations {
            names.push("has_citations".to_string());
        }
        if self.has_deadlines {
            names.push("has_deadlines".to_string());
        }
        if self.has_penalties {
            names.push("has_penalties".to_string());
        }
        if self.needs_review {
            names.push("needs_review".to_string());
        }
        if self.primary_law {
            names.push("primary_law".to_string());
        }
        if self.official_commentary {
            names.push("official_commentary".to_string());
        }
        names
    }

    pub fn vector_chunk_type(&self) -> Option<&'static str> {
        let requested = self
            .result_type
            .as_deref()
            .or(self.semantic_type.as_deref())
            .map(|value| value.to_ascii_lowercase());

        if self.has_deadlines {
            return Some("deadline_block");
        }
        if self.has_penalties {
            return Some("penalty_block");
        }

        match requested.as_deref() {
            Some("definition") | Some("definedterm") => Some("definition_block"),
            Some("exception") => Some("exception_block"),
            Some("deadline") => Some("deadline_block"),
            Some("penalty") => Some("penalty_block"),
            Some("notice") | Some("requirednotice") => Some("contextual_provision"),
            Some("formatting") | Some("formattingrequirement") => Some("formatting_requirement"),
            Some("filing") | Some("filingrequirement") => Some("filing_requirement"),
            Some("service") | Some("servicerequirement") => Some("service_requirement"),
            Some("efiling") | Some("efilingrequirement") => Some("efiling_requirement"),
            Some("certificate") | Some("certificateofservicerequirement") => {
                Some("certificate_requirement")
            }
            Some("exhibit") | Some("exhibitrequirement") => Some("exhibit_requirement"),
            Some("protected_info") | Some("protectedinformationrequirement") => {
                Some("protected_info_requirement")
            }
            _ => None,
        }
    }
}

fn normalized_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("all"))
        .map(ToString::to_string)
}

pub fn normalized_authority_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("all"))
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "ors" | "or:ors" | "statute" | "statutes" => Some("ORS".to_string()),
            "usconst" | "us_const" | "us:constitution" | "constitution" | "u.s. constitution"
            | "us constitution" => Some("USCONST".to_string()),
            "conan" | "constitution_annotated" | "constitution annotated" | "official_commentary" => {
                Some("CONAN".to_string())
            }
            "utcr" | "or:utcr" | "court_rule" | "court_rules" | "rule" | "rules" => {
                Some("UTCR".to_string())
            }
            _ => Some(value.to_ascii_uppercase()),
        })
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
    pub mode: SearchMode,
    pub total: usize,
    pub limit: u32,
    pub offset: u32,
    pub results: Vec<SearchResult>,
    pub facets: Option<SearchFacets>,
    pub warnings: Vec<String>,
    pub analysis: SearchAnalysis,
    pub retrieval: RetrievalInfo,
    pub embeddings: Option<EmbeddingsInfo>,
    pub rerank: Option<RerankInfo>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct SearchAnalysis {
    pub normalized_query: String,
    pub intent: String,
    pub inferred_authority_family: Option<String>,
    pub citations: Vec<QueryCitation>,
    pub ranges: Vec<QueryCitationRange>,
    pub inferred_chapter: Option<String>,
    pub residual_text: Option<String>,
    pub expansion_terms: Vec<QueryExpansionTerm>,
    pub expansion_count: usize,
    pub applied_filters: Vec<String>,
    pub timings: SearchTimingInfo,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct SearchTimingInfo {
    pub total_ms: u64,
    pub retrieval_ms: u64,
    pub graph_ms: u64,
    pub rerank_ms: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct QueryCitation {
    pub raw: String,
    pub authority_family: String,
    pub normalized: String,
    pub base: String,
    pub chapter: String,
    pub section: String,
    pub subsections: Vec<String>,
    pub parent: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct QueryCitationRange {
    pub raw: String,
    pub authority_family: String,
    pub start: String,
    pub end: String,
    pub chapter: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct QueryExpansionTerm {
    pub term: String,
    pub normalized_term: Option<String>,
    pub kind: String,
    pub source_id: Option<String>,
    pub source_citation: Option<String>,
    pub score: f32,
}

#[derive(Debug, Serialize, Default)]
pub struct RetrievalInfo {
    pub exact_candidates: usize,
    pub fulltext_candidates: usize,
    pub vector_candidates: usize,
    pub filtered_candidates: usize,
    pub capped_candidates: usize,
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
    pub authority_family: Option<String>,
    pub authority_type: Option<String>,
    pub authority_level: Option<i32>,
    pub authority_tier: Option<String>,
    pub jurisdiction_id: Option<String>,
    pub source_role: Option<String>,
    pub primary_law: Option<bool>,
    pub official_commentary: Option<bool>,
    pub controlling_weight: Option<f32>,
    pub corpus_id: Option<String>,
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
    pub expansion: Option<f32>,
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
    pub citation: Option<String>,
    pub canonical_id: Option<String>,
    pub match_type: DirectMatchType,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct DirectOpenResponse {
    pub matched: bool,
    pub match_type: DirectMatchType,
    pub normalized_query: String,
    pub citation: String,
    pub canonical_id: String,
    pub href: String,
    pub parent: Option<DirectOpenParent>,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DirectMatchType {
    ExactProvision,
    ExactStatute,
    ParentStatute,
    None,
}

#[derive(Debug, Serialize)]
pub struct DirectOpenParent {
    pub citation: String,
    pub canonical_id: String,
    pub href: String,
}
