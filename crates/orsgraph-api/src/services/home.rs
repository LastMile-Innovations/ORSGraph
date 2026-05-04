use crate::error::ApiResult;
use crate::models::home::*;
use crate::services::analytics::AnalyticsService;
use crate::services::health::HealthService;
use crate::services::stats::StatsService;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct HomeService {
    stats_service: Arc<StatsService>,
    health_service: Arc<HealthService>,
    analytics_service: Arc<AnalyticsService>,
    cache: RwLock<Option<(Instant, HomePageData)>>,
}

impl HomeService {
    pub fn new(
        stats_service: Arc<StatsService>,
        health_service: Arc<HealthService>,
        analytics_service: Arc<AnalyticsService>,
    ) -> Self {
        Self {
            stats_service,
            health_service,
            analytics_service,
            cache: RwLock::new(None),
        }
    }

    pub async fn get_home_page_data(&self) -> ApiResult<HomePageData> {
        {
            let cache = self.cache.read().await;
            if let Some((time, data)) = &*cache {
                if time.elapsed() < Duration::from_secs(30) {
                    return Ok(data.clone());
                }
            }
        }

        let corpus = self.get_corpus_status().await?;
        let health = self.get_system_health().await?;
        let insights = self.analytics_service.get_home_insights().await?;
        let featured_statutes = self.get_featured_statutes().await?;

        let actions = vec![
            HomeAction {
                title: "Search ORS".to_string(),
                description: "Search statutes, provisions, definitions, obligations, deadlines, penalties, notices, source notes, and chunks.".to_string(),
                href: "/search".to_string(),
                icon: "Search".to_string(),
                variant: Some("default".to_string()),
                badges: Some(vec!["keyword".to_string(), "citation".to_string(), "graph".to_string()]),
                status: Some("ready".to_string()),
            },
            HomeAction {
                title: "Ask ORSGraph".to_string(),
                description: "Ask graph-grounded legal questions over Oregon law with citations, provisions, definitions, and currentness warnings.".to_string(),
                href: "/ask".to_string(),
                icon: "MessageSquare".to_string(),
                variant: Some("primary".to_string()),
                badges: Some(vec!["QA".to_string(), "rerank-ready".to_string()]),
                status: Some("ready".to_string()),
            },
            HomeAction {
                title: "Statute Intelligence".to_string(),
                description: "Open a statute with provision tree, citations, definitions, duties, deadlines, penalties, source notes, and chunks.".to_string(),
                href: "/statutes".to_string(),
                icon: "BookOpen".to_string(),
                variant: Some("default".to_string()),
                badges: Some(vec!["deep view".to_string(), "source-backed".to_string()]),
                status: Some("ready".to_string()),
            },
            HomeAction {
                title: "Citation Graph".to_string(),
                description: "Visualize CITES, RESOLVES_TO, DEFINES, EXCEPTION_TO, HAS_VERSION, and semantic reasoning paths.".to_string(),
                href: "/graph".to_string(),
                icon: "Network".to_string(),
                variant: Some("default".to_string()),
                badges: Some(vec!["Neo4j".to_string(), "multi-hop".to_string()]),
                status: Some("ready".to_string()),
            },
            HomeAction {
                title: "Graph Ops".to_string(),
                description: "Track crawl, parse, resolve, seed, materialize, embed, Neo4j topology, API health, and graph runs.".to_string(),
                href: "/admin".to_string(),
                icon: "Activity".to_string(),
                variant: Some("default".to_string()),
                badges: Some(vec!["internal".to_string(), "pipeline".to_string()]),
                status: Some("internal".to_string()),
            },
        ];

        let build = BuildInfo {
            app_version: "0.1.0".to_string(),
            api_version: Some("0.1.0".to_string()),
            graph_edition: Some("ORS 2025".to_string()),
            environment: "development".to_string(),
        };

        let data = HomePageData {
            corpus,
            health,
            actions,
            insights,
            featured_statutes,
            build,
        };

        {
            let mut cache = self.cache.write().await;
            *cache = Some((Instant::now(), data.clone()));
        }

        Ok(data)
    }

    pub async fn get_corpus_status(&self) -> ApiResult<CorpusStatus> {
        let counts = self.stats_service.get_corpus_counts().await?;

        let cites_edges = counts.cites_edges;
        let citation_mentions = counts.citation_mentions;
        let retrieval_chunks = counts.retrieval_chunks;
        let resolved = cites_edges;
        let unresolved = citation_mentions - cites_edges;
        let coverage_percent = if citation_mentions > 0 {
            (resolved as f64 / citation_mentions as f64) * 100.0
        } else {
            0.0
        };

        Ok(CorpusStatus {
            edition_year: 2025,
            source: "Oregon Revised Statutes".to_string(),
            last_updated: None,
            counts,
            citations: CitationCoverage {
                total: citation_mentions,
                resolved,
                unresolved,
                cites_edges,
                coverage_percent,
            },
            embeddings: EmbeddingStatus {
                model: Some("voyage-4-large".to_string()),
                profile: Some("legal_chunk_primary_v1".to_string()),
                embedded: 0,
                total_eligible: retrieval_chunks,
                coverage_percent: 0.0,
                status: "not_started".to_string(),
            },
        })
    }

    pub async fn get_system_health(&self) -> ApiResult<SystemHealth> {
        let neo4j_ok = self.health_service.check_neo4j().await.unwrap_or(false);

        Ok(SystemHealth {
            api: "connected".to_string(),
            neo4j: if neo4j_ok {
                "connected".to_string()
            } else {
                "offline".to_string()
            },
            graph_materialization: "complete".to_string(),
            embeddings: "not_started".to_string(),
            rerank: "missing_key".to_string(),
            last_seeded_at: None,
            last_checked_at: None,
        })
    }

    pub async fn get_featured_statutes(&self) -> ApiResult<Vec<FeaturedStatute>> {
        Ok(vec![
            FeaturedStatute {
                citation: "ORS 90.300".to_string(),
                title: "Security deposits; prepaid rent".to_string(),
                chapter: "Chapter 90".to_string(),
                href: "/statutes/or:ors:90.300".to_string(),
                status: "active".to_string(),
                semantic_types: vec![
                    "obligation".to_string(),
                    "deadline".to_string(),
                    "remedy".to_string(),
                ],
                cited_by_count: Some(15),
                source_backed: Some(true),
            },
            FeaturedStatute {
                citation: "ORS 90.100".to_string(),
                title: "Definitions".to_string(),
                chapter: "Chapter 90".to_string(),
                href: "/statutes/or:ors:90.100".to_string(),
                status: "active".to_string(),
                semantic_types: vec!["definition".to_string()],
                cited_by_count: Some(89),
                source_backed: Some(true),
            },
            FeaturedStatute {
                citation: "ORS 320.005".to_string(),
                title: "Definitions for ORS 320.005 to 320.150".to_string(),
                chapter: "Chapter 320".to_string(),
                href: "/statutes/or:ors:320.005".to_string(),
                status: "active".to_string(),
                semantic_types: vec!["definition".to_string()],
                cited_by_count: None,
                source_backed: Some(true),
            },
            FeaturedStatute {
                citation: "ORS 838.005".to_string(),
                title: "Definitions".to_string(),
                chapter: "Chapter 838".to_string(),
                href: "/statutes/or:ors:838.005".to_string(),
                status: "active".to_string(),
                semantic_types: vec!["definition".to_string()],
                cited_by_count: None,
                source_backed: Some(true),
            },
        ])
    }
}
