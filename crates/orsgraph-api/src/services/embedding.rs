use crate::error::ApiResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct EmbeddingService {
    client: reqwest::Client,
    api_key: String,
    model: String,
    dimension: usize,
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
    pub fn new(api_key: String, model: String, dimension: usize, timeout_ms: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_key,
            model,
            dimension,
        }
    }

    pub async fn embed_query(&self, q: &str) -> ApiResult<Vec<f32>> {
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
}
