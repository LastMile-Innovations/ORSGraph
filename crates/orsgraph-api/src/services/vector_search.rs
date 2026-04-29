use crate::error::{ApiError, ApiResult};
use crate::models::search::SearchResult;
use crate::services::embedding::EmbeddingService;
use crate::services::neo4j::Neo4jService;
use std::sync::Arc;

pub struct VectorSearchService {
    neo4j: Arc<Neo4jService>,
    embeddings: Arc<EmbeddingService>,
    index_name: String,
    top_k: usize,
    min_score: f32,
    profile: String,
}

impl VectorSearchService {
    pub fn new(
        neo4j: Arc<Neo4jService>,
        embeddings: Arc<EmbeddingService>,
        index_name: String,
        top_k: usize,
        min_score: f32,
        profile: String,
    ) -> Self {
        Self {
            neo4j,
            embeddings,
            index_name,
            top_k,
            min_score,
            profile,
        }
    }

    pub async fn search_chunks(&self, query: &str, limit: usize) -> ApiResult<Vec<SearchResult>> {
        if !self.neo4j.vector_index_exists(&self.index_name).await? {
            return Err(ApiError::External(format!(
                "Vector index '{}' is unavailable",
                self.index_name
            )));
        }

        let embedding = self.embeddings.embed_query(query).await?;
        self.neo4j
            .search_vector_chunks(
                &self.index_name,
                embedding,
                self.top_k,
                self.min_score,
                limit.max(1).min(self.top_k),
            )
            .await
    }

    pub fn model(&self) -> &str {
        self.embeddings.model()
    }

    pub fn dimension(&self) -> usize {
        self.embeddings.dimension()
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn top_k(&self) -> usize {
        self.top_k
    }
}
