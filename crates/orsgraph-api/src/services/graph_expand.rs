use crate::error::ApiResult;
use crate::models::search::SearchResult;
use crate::services::neo4j::Neo4jService;
use std::sync::Arc;

pub struct GraphExpandService {
    neo4j: Arc<Neo4jService>,
}

impl GraphExpandService {
    pub fn new(neo4j: Arc<Neo4jService>) -> Self {
        Self { neo4j }
    }

    pub async fn expand_candidates(&self, candidates: &mut [SearchResult]) -> ApiResult<usize> {
        self.neo4j.expand_search_results(candidates).await
    }
}
