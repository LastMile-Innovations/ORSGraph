use crate::error::ApiResult;
use crate::services::neo4j::Neo4jService;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct HealthService {
    neo4j: Arc<Neo4jService>,
    cache: RwLock<Option<(Instant, bool)>>,
}

impl HealthService {
    pub fn new(neo4j: Arc<Neo4jService>) -> Self {
        Self {
            neo4j,
            cache: RwLock::new(None),
        }
    }

    pub async fn check_neo4j(&self) -> ApiResult<bool> {
        {
            let cache = self.cache.read().await;
            if let Some((time, ok)) = &*cache {
                if time.elapsed() < Duration::from_secs(5) {
                    return Ok(*ok);
                }
            }
        }

        let result = self.neo4j.health_check().await;
        let ok = result.unwrap_or(false);

        {
            let mut cache = self.cache.write().await;
            *cache = Some((Instant::now(), ok));
        }

        Ok(ok)
    }
}
