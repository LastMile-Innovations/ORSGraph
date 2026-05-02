use config::{Config as Cfg, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;

#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    #[serde(default = "default_api_host")]
    pub api_host: String,
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    #[serde(default = "default_neo4j_uri")]
    pub neo4j_uri: String,
    #[serde(default = "default_neo4j_user")]
    pub neo4j_user: String,
    #[serde(default = "default_neo4j_password")]
    pub neo4j_password: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_casebuilder_storage_dir")]
    pub casebuilder_storage_dir: String,
    #[serde(default = "default_storage_backend")]
    pub storage_backend: String,
    #[serde(default)]
    pub r2_account_id: Option<String>,
    #[serde(default)]
    pub r2_bucket: Option<String>,
    #[serde(default)]
    pub r2_access_key_id: Option<String>,
    #[serde(default)]
    pub r2_secret_access_key: Option<String>,
    #[serde(default)]
    pub r2_endpoint: Option<String>,
    #[serde(default = "default_r2_upload_ttl_seconds")]
    pub r2_upload_ttl_seconds: u64,
    #[serde(default = "default_r2_download_ttl_seconds")]
    pub r2_download_ttl_seconds: u64,
    #[serde(default = "default_r2_max_upload_bytes")]
    pub r2_max_upload_bytes: u64,
    #[serde(default = "default_casebuilder_ast_entity_inline_bytes")]
    pub casebuilder_ast_entity_inline_bytes: u64,
    #[serde(default = "default_casebuilder_ast_snapshot_inline_bytes")]
    pub casebuilder_ast_snapshot_inline_bytes: u64,
    #[serde(default = "default_casebuilder_ast_block_inline_bytes")]
    pub casebuilder_ast_block_inline_bytes: u64,
    #[serde(default)]
    pub assemblyai_enabled: bool,
    #[serde(default)]
    pub assemblyai_api_key: Option<String>,
    #[serde(default = "default_assemblyai_base_url")]
    pub assemblyai_base_url: String,
    #[serde(default)]
    pub assemblyai_webhook_url: Option<String>,
    #[serde(default)]
    pub assemblyai_webhook_secret: Option<String>,
    #[serde(default = "default_assemblyai_timeout_ms")]
    pub assemblyai_timeout_ms: u64,
    #[serde(default = "default_assemblyai_max_media_bytes")]
    pub assemblyai_max_media_bytes: u64,
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

    // Internal Admin Operations
    #[serde(default)]
    pub admin_enabled: bool,
    #[serde(default)]
    pub admin_allow_kill: bool,
    #[serde(default = "default_admin_jobs_dir")]
    pub admin_jobs_dir: String,
    #[serde(default = "default_admin_workdir")]
    pub admin_workdir: String,
    #[serde(default = "default_admin_crawler_bin")]
    pub admin_crawler_bin: String,
    #[serde(default = "default_admin_data_dir")]
    pub admin_data_dir: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_casebuilder_storage_dir() -> String {
    "data/casebuilder/uploads".to_string()
}

fn default_storage_backend() -> String {
    "local".to_string()
}

fn default_r2_upload_ttl_seconds() -> u64 {
    900
}

fn default_r2_download_ttl_seconds() -> u64 {
    300
}

fn default_r2_max_upload_bytes() -> u64 {
    50 * 1024 * 1024
}

fn default_casebuilder_ast_entity_inline_bytes() -> u64 {
    64 * 1024
}

fn default_casebuilder_ast_snapshot_inline_bytes() -> u64 {
    256 * 1024
}

fn default_casebuilder_ast_block_inline_bytes() -> u64 {
    64 * 1024
}

fn default_assemblyai_base_url() -> String {
    "https://api.assemblyai.com".to_string()
}

fn default_assemblyai_timeout_ms() -> u64 {
    30_000
}

fn default_assemblyai_max_media_bytes() -> u64 {
    500 * 1024 * 1024
}

fn default_api_host() -> String {
    "127.0.0.1".to_string()
}

fn default_api_port() -> u16 {
    8080
}

fn default_neo4j_uri() -> String {
    "bolt://localhost:7687".to_string()
}

fn default_neo4j_user() -> String {
    "neo4j".to_string()
}

fn default_neo4j_password() -> String {
    "neo4j".to_string()
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

fn default_admin_jobs_dir() -> String {
    "data/admin/jobs".to_string()
}

fn default_admin_workdir() -> String {
    ".".to_string()
}

fn default_admin_crawler_bin() -> String {
    "cargo".to_string()
}

fn default_admin_data_dir() -> String {
    "data".to_string()
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
        config.normalize_and_validate()?;
        Ok(config)
    }

    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.api_host, self.api_port)
            .parse()
            .expect("Invalid socket address")
    }

    fn apply_explicit_env_overrides(&mut self) {
        if let Some(value) = read_string("ORS_API_HOST") {
            self.api_host = value;
        }
        if let Some(value) = read_u16("ORS_API_PORT").or_else(|| read_u16("PORT")) {
            self.api_port = value;
        }
        if let Some(value) = read_string("NEO4J_URI") {
            self.neo4j_uri = value;
        }
        if let Some(value) = read_string("NEO4J_USER") {
            self.neo4j_user = value;
        }
        if let Some(value) = read_string("NEO4J_PASSWORD") {
            self.neo4j_password = value;
        }
        if let Some(value) = read_string("ORS_API_KEY") {
            self.api_key = Some(value);
        }
        if let Some(value) = read_string("ORS_CASEBUILDER_STORAGE_DIR") {
            self.casebuilder_storage_dir = value;
        }
        if let Some(value) = read_string("ORS_STORAGE_BACKEND") {
            self.storage_backend = value;
        }
        if let Some(value) = read_string("ORS_R2_ACCOUNT_ID") {
            self.r2_account_id = Some(value);
        }
        if let Some(value) = read_string("ORS_R2_BUCKET") {
            self.r2_bucket = Some(value);
        }
        if let Some(value) = read_string("ORS_R2_ACCESS_KEY_ID") {
            self.r2_access_key_id = Some(value);
        }
        if let Some(value) = read_string("ORS_R2_SECRET_ACCESS_KEY") {
            self.r2_secret_access_key = Some(value);
        }
        if let Some(value) = read_string("ORS_R2_ENDPOINT") {
            self.r2_endpoint = Some(value);
        }
        if let Some(value) = read_u64("ORS_R2_UPLOAD_TTL_SECONDS") {
            self.r2_upload_ttl_seconds = value;
        }
        if let Some(value) = read_u64("ORS_R2_DOWNLOAD_TTL_SECONDS") {
            self.r2_download_ttl_seconds = value;
        }
        if let Some(value) = read_u64("ORS_R2_MAX_UPLOAD_BYTES") {
            self.r2_max_upload_bytes = value;
        }
        if let Some(value) = read_u64("ORS_CASEBUILDER_AST_ENTITY_INLINE_BYTES") {
            self.casebuilder_ast_entity_inline_bytes = value;
        }
        if let Some(value) = read_u64("ORS_CASEBUILDER_AST_SNAPSHOT_INLINE_BYTES") {
            self.casebuilder_ast_snapshot_inline_bytes = value;
        }
        if let Some(value) = read_u64("ORS_CASEBUILDER_AST_BLOCK_INLINE_BYTES") {
            self.casebuilder_ast_block_inline_bytes = value;
        }
        if let Some(value) = read_bool("ORS_ASSEMBLYAI_ENABLED") {
            self.assemblyai_enabled = value;
        }
        if let Some(value) =
            read_string("ASSEMBLYAI_API_KEY").or_else(|| read_string("ORS_ASSEMBLYAI_API_KEY"))
        {
            self.assemblyai_api_key = Some(value);
        }
        if let Some(value) = read_string("ORS_ASSEMBLYAI_BASE_URL") {
            self.assemblyai_base_url = value;
        }
        if let Some(value) = read_string("ORS_ASSEMBLYAI_WEBHOOK_URL") {
            self.assemblyai_webhook_url = Some(value);
        }
        if let Some(value) = read_string("ORS_ASSEMBLYAI_WEBHOOK_SECRET") {
            self.assemblyai_webhook_secret = Some(value);
        }
        if let Some(value) = read_u64("ORS_ASSEMBLYAI_TIMEOUT_MS") {
            self.assemblyai_timeout_ms = value;
        }
        if let Some(value) = read_u64("ORS_ASSEMBLYAI_MAX_MEDIA_BYTES") {
            self.assemblyai_max_media_bytes = value;
        }

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

        if let Some(value) = read_bool("ORS_ADMIN_ENABLED") {
            self.admin_enabled = value;
        }
        if let Some(value) = read_bool("ORS_ADMIN_ALLOW_KILL") {
            self.admin_allow_kill = value;
        }
        if let Some(value) = read_string("ORS_ADMIN_JOBS_DIR") {
            self.admin_jobs_dir = value;
        }
        if let Some(value) = read_string("ORS_ADMIN_WORKDIR") {
            self.admin_workdir = value;
        }
        if let Some(value) = read_string("ORS_ADMIN_CRAWLER_BIN") {
            self.admin_crawler_bin = value;
        }
        if let Some(value) = read_string("ORS_ADMIN_DATA_DIR") {
            self.admin_data_dir = value;
        }
    }

    fn normalize_and_validate(&mut self) -> Result<(), ConfigError> {
        self.storage_backend = self.storage_backend.trim().to_ascii_lowercase();
        if !matches!(self.storage_backend.as_str(), "local" | "r2") {
            return Err(ConfigError::Message(format!(
                "Unsupported ORS_STORAGE_BACKEND {}; expected local or r2",
                self.storage_backend
            )));
        }
        if self.r2_upload_ttl_seconds == 0 || self.r2_upload_ttl_seconds > 604_800 {
            return Err(ConfigError::Message(
                "ORS_R2_UPLOAD_TTL_SECONDS must be between 1 and 604800".to_string(),
            ));
        }
        if self.r2_download_ttl_seconds == 0 || self.r2_download_ttl_seconds > 604_800 {
            return Err(ConfigError::Message(
                "ORS_R2_DOWNLOAD_TTL_SECONDS must be between 1 and 604800".to_string(),
            ));
        }
        if self.r2_max_upload_bytes == 0 {
            return Err(ConfigError::Message(
                "ORS_R2_MAX_UPLOAD_BYTES must be greater than 0".to_string(),
            ));
        }
        if self.casebuilder_ast_entity_inline_bytes == 0 {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_AST_ENTITY_INLINE_BYTES must be greater than 0".to_string(),
            ));
        }
        if self.casebuilder_ast_snapshot_inline_bytes == 0 {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_AST_SNAPSHOT_INLINE_BYTES must be greater than 0".to_string(),
            ));
        }
        if self.casebuilder_ast_block_inline_bytes == 0 {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_AST_BLOCK_INLINE_BYTES must be greater than 0".to_string(),
            ));
        }
        self.assemblyai_base_url = self
            .assemblyai_base_url
            .trim()
            .trim_end_matches('/')
            .to_string();
        if self.assemblyai_enabled && self.assemblyai_api_key.as_deref().is_none_or(str::is_empty) {
            return Err(ConfigError::Message(
                "ORS_ASSEMBLYAI_API_KEY or ASSEMBLYAI_API_KEY is required when ORS_ASSEMBLYAI_ENABLED=true".to_string(),
            ));
        }
        if self.assemblyai_enabled && self.assemblyai_base_url.is_empty() {
            return Err(ConfigError::Message(
                "ORS_ASSEMBLYAI_BASE_URL must not be empty when AssemblyAI is enabled".to_string(),
            ));
        }
        if self.assemblyai_timeout_ms == 0 {
            return Err(ConfigError::Message(
                "ORS_ASSEMBLYAI_TIMEOUT_MS must be greater than 0".to_string(),
            ));
        }
        if self.assemblyai_max_media_bytes == 0 {
            return Err(ConfigError::Message(
                "ORS_ASSEMBLYAI_MAX_MEDIA_BYTES must be greater than 0".to_string(),
            ));
        }
        if self.storage_backend == "r2" {
            for (name, value) in [
                ("ORS_R2_ACCOUNT_ID", &self.r2_account_id),
                ("ORS_R2_BUCKET", &self.r2_bucket),
                ("ORS_R2_ACCESS_KEY_ID", &self.r2_access_key_id),
                ("ORS_R2_SECRET_ACCESS_KEY", &self.r2_secret_access_key),
            ] {
                if value.as_deref().is_none_or(str::is_empty) {
                    return Err(ConfigError::Message(format!(
                        "{name} is required when ORS_STORAGE_BACKEND=r2"
                    )));
                }
            }
        }
        if self.admin_jobs_dir.trim().is_empty() {
            return Err(ConfigError::Message(
                "ORS_ADMIN_JOBS_DIR must not be empty".to_string(),
            ));
        }
        if self.admin_workdir.trim().is_empty() {
            return Err(ConfigError::Message(
                "ORS_ADMIN_WORKDIR must not be empty".to_string(),
            ));
        }
        if self.admin_crawler_bin.trim().is_empty() {
            return Err(ConfigError::Message(
                "ORS_ADMIN_CRAWLER_BIN must not be empty".to_string(),
            ));
        }
        Ok(())
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

fn read_u64(name: &str) -> Option<u64> {
    read_string(name).and_then(|value| value.parse().ok())
}

fn read_u16(name: &str) -> Option<u16> {
    read_string(name).and_then(|value| value.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::ApiConfig;

    #[test]
    fn storage_backend_defaults_to_local() {
        let mut config = ApiConfig {
            api_host: "127.0.0.1".to_string(),
            api_port: 8080,
            neo4j_uri: "bolt://localhost:7687".to_string(),
            neo4j_user: "neo4j".to_string(),
            neo4j_password: "neo4j".to_string(),
            api_key: None,
            casebuilder_storage_dir: "data/casebuilder/uploads".to_string(),
            storage_backend: "LOCAL".to_string(),
            r2_account_id: None,
            r2_bucket: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_endpoint: None,
            r2_upload_ttl_seconds: 900,
            r2_download_ttl_seconds: 300,
            r2_max_upload_bytes: 10,
            casebuilder_ast_entity_inline_bytes: 64 * 1024,
            casebuilder_ast_snapshot_inline_bytes: 256 * 1024,
            casebuilder_ast_block_inline_bytes: 64 * 1024,
            assemblyai_enabled: false,
            assemblyai_api_key: None,
            assemblyai_base_url: "https://api.assemblyai.com".to_string(),
            assemblyai_webhook_url: None,
            assemblyai_webhook_secret: None,
            assemblyai_timeout_ms: 30_000,
            assemblyai_max_media_bytes: 500 * 1024 * 1024,
            log_level: "info".to_string(),
            voyage_api_key: None,
            rerank_enabled: false,
            rerank_model: "rerank-2.5".to_string(),
            rerank_fallback_model: "rerank-2.5-lite".to_string(),
            rerank_candidates: 150,
            rerank_top_k: 25,
            rerank_max_doc_tokens: 2000,
            rerank_timeout_ms: 8000,
            vector_enabled: false,
            vector_search_enabled: false,
            embedding_model: "voyage-4-large".to_string(),
            vector_index: "retrieval_chunk_embedding_1024".to_string(),
            vector_dimension: 1024,
            vector_top_k: 100,
            vector_min_score: 0.55,
            vector_profile: "legal_chunk_primary_v1".to_string(),
            admin_enabled: false,
            admin_allow_kill: false,
            admin_jobs_dir: "data/admin/jobs".to_string(),
            admin_workdir: ".".to_string(),
            admin_crawler_bin: "cargo".to_string(),
            admin_data_dir: "data".to_string(),
        };

        config.normalize_and_validate().unwrap();
        assert_eq!(config.storage_backend, "local");
    }

    #[test]
    fn r2_backend_requires_credentials() {
        let mut config = ApiConfig {
            api_host: "127.0.0.1".to_string(),
            api_port: 8080,
            neo4j_uri: "bolt://localhost:7687".to_string(),
            neo4j_user: "neo4j".to_string(),
            neo4j_password: "neo4j".to_string(),
            api_key: None,
            casebuilder_storage_dir: "data/casebuilder/uploads".to_string(),
            storage_backend: "r2".to_string(),
            r2_account_id: None,
            r2_bucket: Some("bucket".to_string()),
            r2_access_key_id: Some("access".to_string()),
            r2_secret_access_key: Some("secret".to_string()),
            r2_endpoint: None,
            r2_upload_ttl_seconds: 900,
            r2_download_ttl_seconds: 300,
            r2_max_upload_bytes: 10,
            casebuilder_ast_entity_inline_bytes: 64 * 1024,
            casebuilder_ast_snapshot_inline_bytes: 256 * 1024,
            casebuilder_ast_block_inline_bytes: 64 * 1024,
            assemblyai_enabled: false,
            assemblyai_api_key: None,
            assemblyai_base_url: "https://api.assemblyai.com".to_string(),
            assemblyai_webhook_url: None,
            assemblyai_webhook_secret: None,
            assemblyai_timeout_ms: 30_000,
            assemblyai_max_media_bytes: 500 * 1024 * 1024,
            log_level: "info".to_string(),
            voyage_api_key: None,
            rerank_enabled: false,
            rerank_model: "rerank-2.5".to_string(),
            rerank_fallback_model: "rerank-2.5-lite".to_string(),
            rerank_candidates: 150,
            rerank_top_k: 25,
            rerank_max_doc_tokens: 2000,
            rerank_timeout_ms: 8000,
            vector_enabled: false,
            vector_search_enabled: false,
            embedding_model: "voyage-4-large".to_string(),
            vector_index: "retrieval_chunk_embedding_1024".to_string(),
            vector_dimension: 1024,
            vector_top_k: 100,
            vector_min_score: 0.55,
            vector_profile: "legal_chunk_primary_v1".to_string(),
            admin_enabled: false,
            admin_allow_kill: false,
            admin_jobs_dir: "data/admin/jobs".to_string(),
            admin_workdir: ".".to_string(),
            admin_crawler_bin: "cargo".to_string(),
            admin_data_dir: "data".to_string(),
        };

        let error = config.normalize_and_validate().unwrap_err().to_string();
        assert!(error.contains("ORS_R2_ACCOUNT_ID"));
    }
}

fn read_f32(name: &str) -> Option<f32> {
    read_string(name).and_then(|value| value.parse().ok())
}
