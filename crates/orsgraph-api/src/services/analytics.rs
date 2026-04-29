use crate::error::ApiResult;
use crate::models::home::GraphInsightCard;
use crate::services::neo4j::Neo4jService;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct AnalyticsService {
    neo4j: Arc<Neo4jService>,
    cache: RwLock<Option<(Instant, Vec<GraphInsightCard>)>>,
}

impl AnalyticsService {
    pub fn new(neo4j: Arc<Neo4jService>) -> Self {
        Self {
            neo4j,
            cache: RwLock::new(None),
        }
    }

    pub async fn get_home_insights(&self) -> ApiResult<Vec<GraphInsightCard>> {
        {
            let cache = self.cache.read().await;
            if let Some((time, insights)) = &*cache {
                if time.elapsed() < Duration::from_secs(60) {
                    return Ok(insights.clone());
                }
            }
        }

        // Initially mock as requested, can be wired to Neo4j later
        let insights = vec![
            GraphInsightCard {
                title: "Most cited statute".to_string(),
                value: "ORS 90.100".to_string(),
                subtitle: Some("Definition hub".to_string()),
                href: Some("/statutes/ORS 90.100".to_string()),
                state: Some("ok".to_string()),
            },
            GraphInsightCard {
                title: "Chapter 90".to_string(),
                value: "High semantic density".to_string(),
                subtitle: Some("obligations · deadlines · notices · remedies".to_string()),
                href: Some("/statutes/90".to_string()),
                state: Some("ok".to_string()),
            },
        ];

        {
            let mut cache = self.cache.write().await;
            *cache = Some((Instant::now(), insights.clone()));
        }

        Ok(insights)
    }
}
