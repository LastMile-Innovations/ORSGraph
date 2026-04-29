use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokenizers::Tokenizer;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct VoyageModelConfig {
    pub model: &'static str,
    pub context_tokens: usize,
    pub batch_token_limit: usize,
    pub batch_token_safety_limit: usize,
    pub allowed_dimensions: &'static [usize],
    pub default_dimension: usize,
    pub chars_per_token_estimate: f64,
    pub rpm_limit: usize,
    pub tpm_limit: usize,
}

const FLEXIBLE_4_DIMS: &[usize] = &[256, 512, 1024, 2048];
const FIXED_1024_DIMS: &[usize] = &[1024];
const FIXED_1536_DIMS: &[usize] = &[1536];
const FIXED_512_DIMS: &[usize] = &[512];

pub const VOYAGE_4_LARGE: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-4-large",
    context_tokens: 32_000,
    batch_token_limit: 120_000,
    batch_token_safety_limit: 110_000,
    allowed_dimensions: FLEXIBLE_4_DIMS,
    default_dimension: 1024,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 3_000_000,
};

pub const VOYAGE_4: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-4",
    context_tokens: 32_000,
    batch_token_limit: 320_000,
    batch_token_safety_limit: 300_000,
    allowed_dimensions: FLEXIBLE_4_DIMS,
    default_dimension: 1024,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 8_000_000,
};

pub const VOYAGE_4_LITE: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-4-lite",
    context_tokens: 32_000,
    batch_token_limit: 1_000_000,
    batch_token_safety_limit: 950_000,
    allowed_dimensions: FLEXIBLE_4_DIMS,
    default_dimension: 1024,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 16_000_000,
};

pub const VOYAGE_LAW_2: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-law-2",
    context_tokens: 16_000,
    batch_token_limit: 120_000,
    batch_token_safety_limit: 110_000,
    allowed_dimensions: FIXED_1024_DIMS,
    default_dimension: 1024,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 3_000_000,
};

pub const VOYAGE_FINANCE_2: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-finance-2",
    context_tokens: 32_000,
    batch_token_limit: 120_000,
    batch_token_safety_limit: 110_000,
    allowed_dimensions: FIXED_1024_DIMS,
    default_dimension: 1024,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 3_000_000,
};

pub const VOYAGE_CODE_3: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-code-3",
    context_tokens: 32_000,
    batch_token_limit: 120_000,
    batch_token_safety_limit: 110_000,
    allowed_dimensions: FLEXIBLE_4_DIMS,
    default_dimension: 1024,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 3_000_000,
};

pub const VOYAGE_CODE_2: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-code-2",
    context_tokens: 16_000,
    batch_token_limit: 120_000,
    batch_token_safety_limit: 110_000,
    allowed_dimensions: FIXED_1536_DIMS,
    default_dimension: 1536,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 3_000_000,
};

pub const VOYAGE_3_LARGE: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-3-large",
    context_tokens: 32_000,
    batch_token_limit: 120_000,
    batch_token_safety_limit: 110_000,
    allowed_dimensions: FLEXIBLE_4_DIMS,
    default_dimension: 1024,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 3_000_000,
};

pub const VOYAGE_3_5: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-3.5",
    context_tokens: 32_000,
    batch_token_limit: 320_000,
    batch_token_safety_limit: 300_000,
    allowed_dimensions: FLEXIBLE_4_DIMS,
    default_dimension: 1024,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 8_000_000,
};

pub const VOYAGE_3_5_LITE: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-3.5-lite",
    context_tokens: 32_000,
    batch_token_limit: 1_000_000,
    batch_token_safety_limit: 950_000,
    allowed_dimensions: FLEXIBLE_4_DIMS,
    default_dimension: 1024,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 16_000_000,
};

pub const VOYAGE_3: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-3",
    context_tokens: 32_000,
    batch_token_limit: 320_000,
    batch_token_safety_limit: 300_000,
    allowed_dimensions: FIXED_1024_DIMS,
    default_dimension: 1024,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 8_000_000,
};

pub const VOYAGE_3_LITE: VoyageModelConfig = VoyageModelConfig {
    model: "voyage-3-lite",
    context_tokens: 32_000,
    batch_token_limit: 1_000_000,
    batch_token_safety_limit: 950_000,
    allowed_dimensions: FIXED_512_DIMS,
    default_dimension: 512,
    chars_per_token_estimate: 5.2,
    rpm_limit: 2000,
    tpm_limit: 16_000_000,
};

static TOKENIZER_CACHE: Lazy<Mutex<HashMap<String, Option<Tokenizer>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn get_or_load_tokenizer(model: &str) -> Option<Tokenizer> {
    let mut cache = TOKENIZER_CACHE.lock().unwrap();

    if let Some(tokenizer) = cache.get(model) {
        return tokenizer.clone();
    }

    // Try to load from local config directory first
    let local_path = Path::new("config").join(format!("{}.json", model));
    if local_path.exists() {
        if let Ok(tokenizer) = Tokenizer::from_file(&local_path) {
            cache.insert(model.to_string(), Some(tokenizer.clone()));
            tracing::info!("Loaded tokenizer for {} from local file", model);
            return Some(tokenizer);
        }
    }

    // Try to download from Hugging Face using hf-hub
    let hf_model = format!("voyageai/{}", model);

    if let Ok(api) = hf_hub::api::sync::Api::new() {
        let repo = api.model(hf_model);
        if let Ok(tokenizer_file) = repo.get("tokenizer.json") {
            if let Ok(tokenizer) = Tokenizer::from_file(&tokenizer_file) {
                cache.insert(model.to_string(), Some(tokenizer.clone()));
                tracing::info!("Loaded tokenizer for {} from Hugging Face", model);
                return Some(tokenizer);
            }
        }
    }

    cache.insert(model.to_string(), None);
    tracing::warn!(
        "Failed to load tokenizer for {} from Hugging Face or local file",
        model
    );
    None
}

#[derive(Debug, Clone)]
struct RequestTimestamp {
    timestamp: Instant,
    tokens: usize,
}

#[derive(Debug)]
pub struct RateLimiter {
    rpm_limit: usize,
    tpm_limit: usize,
    requests: Arc<Mutex<VecDeque<RequestTimestamp>>>,
}

impl RateLimiter {
    pub fn new(rpm_limit: usize, tpm_limit: usize) -> Self {
        Self {
            rpm_limit,
            tpm_limit,
            requests: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn acquire(&self, estimated_tokens: usize) -> Result<()> {
        let now = Instant::now();
        let one_minute_ago = now - Duration::from_secs(60);

        // Clean up old requests
        {
            let mut requests = self.requests.lock().unwrap();
            while let Some(front) = requests.front() {
                if front.timestamp < one_minute_ago {
                    requests.pop_front();
                } else {
                    break;
                }
            }
        }

        // Calculate current usage
        let (current_rpm, current_tpm) = {
            let requests = self.requests.lock().unwrap();
            let rpm = requests.len();
            let tpm: usize = requests.iter().map(|r| r.tokens).sum();
            (rpm, tpm)
        };

        // Check if we would exceed limits
        if current_rpm >= self.rpm_limit {
            let wait_time = {
                let requests = self.requests.lock().unwrap();
                if let Some(oldest) = requests.front() {
                    let elapsed = now.duration_since(oldest.timestamp);
                    if elapsed < Duration::from_secs(60) {
                        Duration::from_secs(60) - elapsed
                    } else {
                        Duration::from_millis(0)
                    }
                } else {
                    Duration::from_millis(0)
                }
            };
            if wait_time > Duration::from_millis(0) {
                tracing::info!(
                    "Rate limit reached ({} RPM). Waiting {:?} before next request.",
                    current_rpm,
                    wait_time
                );
                sleep(wait_time).await;
            }
        }

        if current_tpm + estimated_tokens > self.tpm_limit {
            // Calculate wait time based on token usage
            let excess_tokens = (current_tpm + estimated_tokens) - self.tpm_limit;
            let avg_tokens_per_request = if current_rpm > 0 {
                current_tpm / current_rpm
            } else {
                estimated_tokens
            };
            let requests_to_wait =
                (excess_tokens + avg_tokens_per_request - 1) / avg_tokens_per_request;
            let wait_time =
                Duration::from_secs(60) * (requests_to_wait as u32) / (self.rpm_limit as u32);

            tracing::info!(
                "Token limit reached ({} TPM, need {} more). Waiting {:?} before next request.",
                current_tpm,
                excess_tokens,
                wait_time
            );
            sleep(wait_time).await;
        }

        // Record this request
        {
            let mut requests = self.requests.lock().unwrap();
            requests.push_back(RequestTimestamp {
                timestamp: Instant::now(),
                tokens: estimated_tokens,
            });
        }

        Ok(())
    }
}

pub fn model_config(model: &str) -> Option<&'static VoyageModelConfig> {
    match model {
        "voyage-4-large" => Some(&VOYAGE_4_LARGE),
        "voyage-4" => Some(&VOYAGE_4),
        "voyage-4-lite" => Some(&VOYAGE_4_LITE),
        "voyage-law-2" => Some(&VOYAGE_LAW_2),
        "voyage-finance-2" => Some(&VOYAGE_FINANCE_2),
        "voyage-code-3" => Some(&VOYAGE_CODE_3),
        "voyage-code-2" => Some(&VOYAGE_CODE_2),
        "voyage-3-large" => Some(&VOYAGE_3_LARGE),
        "voyage-3.5" => Some(&VOYAGE_3_5),
        "voyage-3.5-lite" => Some(&VOYAGE_3_5_LITE),
        "voyage-3" => Some(&VOYAGE_3),
        "voyage-3-lite" => Some(&VOYAGE_3_LITE),
        _ => None,
    }
}

pub fn estimate_tokens(text: &str, model: &str) -> usize {
    // Try to use actual tokenizer first
    if let Some(tokenizer) = get_or_load_tokenizer(model) {
        if let Ok(encoding) = tokenizer.encode(text, true) {
            return encoding.get_ids().len();
        }
    }

    // Fallback to character-based estimation
    let chars_per_token = model_config(model)
        .map(|config| config.chars_per_token_estimate)
        .unwrap_or(5.2);
    ((text.chars().count() as f64) / chars_per_token).ceil() as usize
}

#[derive(Debug, Serialize)]
pub struct EmbeddingRequest {
    pub input: Vec<String>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dimension: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dtype: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingResponse {
    pub object: String,
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingData {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: usize,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub total_tokens: usize,
}

#[derive(Debug, Deserialize)]
pub struct VoyageErrorResponse {
    pub message: String,
    #[serde(default)]
    pub error_type: Option<String>,
}

#[derive(Debug)]
enum VoyageError {
    InvalidRequest(String),
    Unauthorized(String),
    Forbidden(String),
    RateLimitExceeded(String),
    ServerError(String),
    ServiceUnavailable(String),
    Unknown(String),
}

impl VoyageError {
    fn from_status_code(status: StatusCode, error_text: String) -> Self {
        match status {
            StatusCode::BAD_REQUEST => {
                VoyageError::InvalidRequest(format!(
                    "Invalid request: {}. Check request body, parameters, batch size, or token limits.",
                    error_text
                ))
            }
            StatusCode::UNAUTHORIZED => {
                VoyageError::Unauthorized(format!(
                    "Unauthorized: {}. Check your API key in the Voyage dashboard.",
                    error_text
                ))
            }
            StatusCode::FORBIDDEN => {
                VoyageError::Forbidden(format!(
                    "Forbidden: {}. Your IP address may be blocked. Try a different IP.",
                    error_text
                ))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                VoyageError::RateLimitExceeded(format!(
                    "Rate limit exceeded: {}. Pace your requests.",
                    error_text
                ))
            }
            StatusCode::INTERNAL_SERVER_ERROR => {
                VoyageError::ServerError(format!(
                    "Server error: {}. Retry after a brief wait.",
                    error_text
                ))
            }
            code if code.is_server_error() || code == StatusCode::BAD_GATEWAY
                || code == StatusCode::SERVICE_UNAVAILABLE
                || code == StatusCode::GATEWAY_TIMEOUT => {
                VoyageError::ServiceUnavailable(format!(
                    "Service unavailable ({}): {}. Servers are experiencing high traffic. Retry after a brief wait.",
                    code.as_u16(),
                    error_text
                ))
            }
            _ => VoyageError::Unknown(format!("API error ({}): {}", status.as_u16(), error_text)),
        }
    }

    fn should_retry(&self) -> bool {
        matches!(
            self,
            VoyageError::RateLimitExceeded(_)
                | VoyageError::ServerError(_)
                | VoyageError::ServiceUnavailable(_)
        )
    }

    fn description(&self) -> &str {
        match self {
            VoyageError::InvalidRequest(_) => "Invalid Request",
            VoyageError::Unauthorized(_) => "Unauthorized",
            VoyageError::Forbidden(_) => "Forbidden",
            VoyageError::RateLimitExceeded(_) => "Rate Limit Exceeded",
            VoyageError::ServerError(_) => "Server Error",
            VoyageError::ServiceUnavailable(_) => "Service Unavailable",
            VoyageError::Unknown(_) => "Unknown Error",
        }
    }
}

pub struct VoyageClient {
    client: Client,
    api_key: String,
    rate_limiter: RateLimiter,
}

impl VoyageClient {
    pub fn new(api_key: String, model: &str) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(60)).build()?;
        let config = model_config(model).ok_or_else(|| anyhow!("Unknown model: {}", model))?;
        let rate_limiter = RateLimiter::new(config.rpm_limit, config.tpm_limit);
        Ok(Self {
            client,
            api_key,
            rate_limiter,
        })
    }

    pub async fn embed(
        &self,
        texts: Vec<String>,
        model: &str,
        dimension: Option<i32>,
        input_type: Option<&str>,
        output_dtype: Option<&str>,
    ) -> Result<EmbeddingResponse> {
        // Estimate tokens for rate limiting
        let estimated_tokens: usize = texts.iter().map(|text| estimate_tokens(text, model)).sum();

        // Acquire rate limit before making request
        self.rate_limiter.acquire(estimated_tokens).await?;

        let mut attempts = 0;
        let max_attempts = 5;
        let mut backoff = Duration::from_millis(500);

        // Default to float if not specified
        let output_dtype = output_dtype.unwrap_or("float");

        loop {
            attempts += 1;
            let request = EmbeddingRequest {
                input: texts.clone(),
                model: model.to_string(),
                input_type: input_type.map(|s| s.to_string()),
                truncation: Some(false),
                output_dimension: dimension,
                output_dtype: Some(output_dtype.to_string()),
            };

            let response = self
                .client
                .post("https://api.voyageai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&request)
                .send()
                .await?;

            let status = response.status();
            if status == StatusCode::OK {
                let result: EmbeddingResponse = response.json().await?;
                return Ok(result);
            }

            let error_text = response.text().await?;
            let voyage_error = VoyageError::from_status_code(status, error_text.clone());

            // Only retry for retryable errors
            if voyage_error.should_retry() && attempts < max_attempts {
                // Add jitter to backoff to avoid thundering herd
                let jitter = Duration::from_millis(rand::random::<u64>() % 200);
                let wait_time = backoff + jitter;

                tracing::warn!(
                    "Voyage API error: {} (attempt {}/{}) - {} - retrying in {:?}",
                    voyage_error.description(),
                    attempts,
                    max_attempts,
                    error_text,
                    wait_time
                );
                sleep(wait_time).await;

                // Exponential backoff with cap
                backoff = std::cmp::min(backoff * 2, Duration::from_secs(30));
                continue;
            }

            // For non-retryable errors or max attempts exceeded, return detailed error
            if attempts >= max_attempts {
                tracing::error!(
                    "Voyage API error: Max retry attempts ({}) reached for: {}",
                    max_attempts,
                    voyage_error.description()
                );
            }

            return Err(anyhow!(
                "Voyage API error: {}",
                match voyage_error {
                    VoyageError::InvalidRequest(msg) => msg,
                    VoyageError::Unauthorized(msg) => msg,
                    VoyageError::Forbidden(msg) => msg,
                    VoyageError::RateLimitExceeded(msg) => msg,
                    VoyageError::ServerError(msg) => msg,
                    VoyageError::ServiceUnavailable(msg) => msg,
                    VoyageError::Unknown(msg) => msg,
                }
            ));
        }
    }

    pub fn estimate_tokens(&self, text: &str) -> usize {
        estimate_tokens(text, VOYAGE_4_LARGE.model)
    }

    /// Embed a single query string for retrieval.
    pub async fn embed_query(&self, text: &str, output_dtype: Option<&str>) -> Result<Vec<f32>> {
        let response = self
            .embed(
                vec![text.to_string()],
                VOYAGE_LAW_2.model,
                Some(1024),
                Some("query"),
                output_dtype,
            )
            .await?;

        response
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| anyhow!("No embedding returned from Voyage API"))
    }
}
