use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HomePageData {
    pub corpus: CorpusStatus,
    pub health: SystemHealth,
    pub actions: Vec<HomeAction>,
    pub insights: Vec<GraphInsightCard>,
    pub featured_statutes: Vec<FeaturedStatute>,
    pub build: BuildInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CorpusStatus {
    pub edition_year: i32,
    pub source: String,
    pub last_updated: Option<String>,
    pub last_qc_run: Option<String>,
    pub qc_status: String,
    pub counts: CorpusCounts,
    pub citations: CitationCoverage,
    pub embeddings: EmbeddingStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CorpusCounts {
    pub sections: i64,
    pub versions: i64,
    pub provisions: i64,
    pub retrieval_chunks: i64,
    pub citation_mentions: i64,
    pub cites_edges: i64,
    pub semantic_nodes: i64,
    pub source_notes: i64,
    pub amendments: i64,
    pub session_laws: i64,
    pub neo4j_nodes: i64,
    pub neo4j_relationships: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CitationCoverage {
    pub total: i64,
    pub resolved: i64,
    pub unresolved: i64,
    pub cites_edges: i64,
    pub coverage_percent: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingStatus {
    pub model: Option<String>,
    pub profile: Option<String>,
    pub embedded: i64,
    pub total_eligible: i64,
    pub coverage_percent: f64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemHealth {
    pub api: String,
    pub neo4j: String,
    pub qc: String,
    pub graph_materialization: String,
    pub embeddings: String,
    pub rerank: String,
    pub last_seeded_at: Option<String>,
    pub last_checked_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HomeAction {
    pub title: String,
    pub description: String,
    pub href: String,
    pub icon: String,
    pub variant: Option<String>,
    pub badges: Option<Vec<String>>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GraphInsightCard {
    pub title: String,
    pub value: String,
    pub subtitle: Option<String>,
    pub href: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedStatute {
    pub citation: String,
    pub title: String,
    pub chapter: String,
    pub href: String,
    pub status: String,
    pub semantic_types: Vec<String>,
    pub cited_by_count: Option<i64>,
    pub source_backed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub app_version: String,
    pub api_version: Option<String>,
    pub graph_edition: Option<String>,
    pub environment: String,
}
