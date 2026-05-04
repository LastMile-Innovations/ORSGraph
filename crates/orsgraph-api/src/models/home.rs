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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_home_serialization_omits_qc_status_fields() {
        let data = HomePageData {
            corpus: CorpusStatus {
                edition_year: 2025,
                source: "Oregon Revised Statutes".to_string(),
                last_updated: None,
                counts: CorpusCounts {
                    sections: 1,
                    versions: 1,
                    provisions: 1,
                    retrieval_chunks: 1,
                    citation_mentions: 1,
                    cites_edges: 1,
                    semantic_nodes: 1,
                    source_notes: 0,
                    amendments: 0,
                    session_laws: 0,
                    neo4j_nodes: 1,
                    neo4j_relationships: 1,
                },
                citations: CitationCoverage {
                    total: 1,
                    resolved: 1,
                    unresolved: 0,
                    cites_edges: 1,
                    coverage_percent: 100.0,
                },
                embeddings: EmbeddingStatus {
                    model: None,
                    profile: None,
                    embedded: 0,
                    total_eligible: 1,
                    coverage_percent: 0.0,
                    status: "not_started".to_string(),
                },
            },
            health: SystemHealth {
                api: "connected".to_string(),
                neo4j: "connected".to_string(),
                graph_materialization: "complete".to_string(),
                embeddings: "not_started".to_string(),
                rerank: "disabled".to_string(),
                last_seeded_at: None,
                last_checked_at: None,
            },
            actions: Vec::new(),
            insights: Vec::new(),
            featured_statutes: Vec::new(),
            build: BuildInfo {
                app_version: "test".to_string(),
                api_version: Some("test".to_string()),
                graph_edition: Some("test".to_string()),
                environment: "test".to_string(),
            },
        };

        let value = serde_json::to_value(data).expect("serialize home data");

        assert!(value["corpus"].get("qcStatus").is_none());
        assert!(value["corpus"].get("lastQcRun").is_none());
        assert!(value["health"].get("qc").is_none());
    }
}
