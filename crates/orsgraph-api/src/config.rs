use config::{Config as Cfg, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;

#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    pub api_host: String,
    pub api_port: u16,
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_log_level")]
    pub log_level: String,

    // Voyage Rerank Config
    #[serde(default)]
    pub voyage_api_key: Option<String>,
    #[serde(default)]
    pub rerank_enabled: bool,
    #[serde(default = "default_rerank_model")]
    pub rerank_model: String,
    #[serde(default = "default_rerank_fallback_model")]
    pub rerank_fallback_model: String,
    #[serde(default = "default_rerank_candidates")]
    pub rerank_candidates: usize,
    #[serde(default = "default_rerank_top_k")]
    pub rerank_top_k: usize,
    #[serde(default = "default_rerank_max_doc_tokens")]
    pub rerank_max_doc_tokens: usize,
    #[serde(default = "default_rerank_timeout_ms")]
    pub rerank_timeout_ms: u64,

    // Voyage Embedding/Vector Config
    #[serde(default)]
    pub vector_enabled: bool,
    #[serde(default)]
    pub vector_search_enabled: bool,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    #[serde(default = "default_vector_index")]
    pub vector_index: String,
    #[serde(default = "default_vector_dimension")]
    pub vector_dimension: usize,
    #[serde(default = "default_vector_top_k")]
    pub vector_top_k: usize,
    #[serde(default = "default_vector_min_score")]
    pub vector_min_score: f32,
    #[serde(default = "default_vector_profile")]
    pub vector_profile: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_rerank_model() -> String {
    "rerank-2.5".to_string()
}

fn default_rerank_fallback_model() -> String {
    "rerank-2.5-lite".to_string()
}

fn default_rerank_candidates() -> usize {
    150
}

fn default_rerank_top_k() -> usize {
    25
}

fn default_rerank_max_doc_tokens() -> usize {
    2000
}

fn default_rerank_timeout_ms() -> u64 {
    8000
}

fn default_embedding_model() -> String {
    "voyage-4-large".to_string()
}

fn default_vector_index() -> String {
    "retrieval_chunk_embedding_1024".to_string()
}

fn default_vector_dimension() -> usize {
    1024
}

fn default_vector_top_k() -> usize {
    100
}

fn default_vector_min_score() -> f32 {
    0.55
}

fn default_vector_profile() -> String {
    "legal_chunk_primary_v1".to_string()
}

impl ApiConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let cfg = Cfg::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(Environment::with_prefix("ORS").separator("__"))
            .add_source(Environment::with_prefix("ORS").separator("_"))
            .add_source(
                Environment::with_prefix("VOYAGE")
                    .prefix_separator("_")
                    .keep_prefix(true),
            );

        let mut config: Self = cfg.build()?.try_deserialize()?;
        config.apply_explicit_env_overrides();
        Ok(config)
    }

    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.api_host, self.api_port)
            .parse()
            .expect("Invalid socket address")
    }

    fn apply_explicit_env_overrides(&mut self) {
        if let Ok(value) = env::var("VOYAGE_API_KEY") {
            if !value.trim().is_empty() {
                self.voyage_api_key = Some(value);
            }
        }

        if let Some(value) = read_bool("ORS_RERANK_ENABLED") {
            self.rerank_enabled = value;
        }
        if let Some(value) = read_string("ORS_RERANK_MODEL") {
            self.rerank_model = value;
        }
        if let Some(value) = read_usize("ORS_RERANK_CANDIDATES") {
            self.rerank_candidates = value;
        }
        if let Some(value) = read_usize("ORS_RERANK_TOP_K") {
            self.rerank_top_k = value;
        }

        if let Some(value) =
            read_bool("ORS_VECTOR_SEARCH_ENABLED").or_else(|| read_bool("ORS_VECTOR_ENABLED"))
        {
            self.vector_search_enabled = value;
            self.vector_enabled = value;
        }
        if let Some(value) = read_string("ORS_VECTOR_INDEX") {
            self.vector_index = value;
        }
        if let Some(value) = read_usize("ORS_VECTOR_TOP_K") {
            self.vector_top_k = value;
        }
        if let Some(value) = read_f32("ORS_VECTOR_MIN_SCORE") {
            self.vector_min_score = value;
        }
        if let Some(value) = read_string("ORS_VECTOR_PROFILE") {
            self.vector_profile = value;
        }
        if let Some(value) = read_usize("ORS_VECTOR_DIMENSION") {
            self.vector_dimension = value;
        }
        if let Some(value) =
            read_string("ORS_EMBEDDING_MODEL").or_else(|| read_string("ORS_VECTOR_MODEL"))
        {
            self.embedding_model = value;
        }
    }
}

fn read_string(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn read_bool(name: &str) -> Option<bool> {
    read_string(name).and_then(|value| match value.to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    })
}

fn read_usize(name: &str) -> Option<usize> {
    read_string(name).and_then(|value| value.parse().ok())
}

fn read_f32(name: &str) -> Option<f32> {
    read_string(name).and_then(|value| value.parse().ok())
}
