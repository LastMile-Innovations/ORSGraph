use crate::error::ApiResult;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

pub struct EmbeddingService {
    client: reqwest::Client,
    api_key: String,
    model: String,
    dimension: usize,
    query_cache: Cache<String, Vec<f32>>,
}

#[derive(Serialize)]
struct VoyageEmbeddingRequest {
    input: Vec<String>,
    model: String,
    input_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_dimension: Option<usize>,
}

#[derive(Deserialize)]
struct VoyageEmbeddingResponse {
    data: Vec<VoyageEmbeddingData>,
    usage: VoyageUsage,
}

#[derive(Deserialize)]
struct VoyageEmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize)]
struct VoyageUsage {
    total_tokens: usize,
}

impl EmbeddingService {
    pub fn new(
        api_key: String,
        model: String,
        dimension: usize,
        timeout_ms: u64,
        cache_ttl_seconds: u64,
        cache_max_capacity: u64,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_default();
        let query_cache = Cache::builder()
            .max_capacity(cache_max_capacity.max(1))
            .time_to_live(Duration::from_secs(cache_ttl_seconds.max(1)))
            .build();

        Self {
            client,
            api_key,
            model,
            dimension,
            query_cache,
        }
    }

    pub async fn embed_query(&self, q: &str) -> ApiResult<Vec<f32>> {
        let cache_key = self.cache_key(q);
        if let Some(cached) = self.query_cache.get(&cache_key).await {
            tracing::debug!(
                model = %self.model,
                dimension = self.dimension,
                "Voyage query embedding cache hit"
            );
            return Ok(cached);
        }

        let request = VoyageEmbeddingRequest {
            input: vec![q.to_string()],
            model: self.model.clone(),
            input_type: "query".to_string(),
            output_dimension: Some(self.dimension),
        };

        let response = self
            .client
            .post("https://api.voyageai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(crate::error::ApiError::External(format!(
                "Voyage Embedding API error: {}",
                err_text
            )));
        }

        let VoyageEmbeddingResponse { mut data, usage } = response.json().await?;
        data.sort_by_key(|item| item.index);
        tracing::debug!(
            model = %self.model,
            total_tokens = usage.total_tokens,
            embedding_count = data.len(),
            "Voyage embedding response"
        );

        if let Some(first) = data.into_iter().next() {
            self.query_cache
                .insert(cache_key, first.embedding.clone())
                .await;
            Ok(first.embedding)
        } else {
            Err(crate::error::ApiError::Internal(
                "Voyage returned empty embedding data".to_string(),
            ))
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    fn cache_key(&self, q: &str) -> String {
        let normalized = q
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        format!(
            "{}:{}:{}",
            self.model,
            self.dimension,
            sha256_hex(normalized.as_bytes())
        )
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
