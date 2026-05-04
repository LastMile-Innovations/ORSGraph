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
    #[serde(default)]
    pub auth_enabled: bool,
    #[serde(default)]
    pub auth_issuer: Option<String>,
    #[serde(default)]
    pub auth_audience: Option<String>,
    #[serde(default = "default_auth_admin_role")]
    pub auth_admin_role: String,
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
    #[serde(default = "default_casebuilder_api_upload_max_bytes")]
    pub casebuilder_api_upload_max_bytes: u64,
    #[serde(default = "default_casebuilder_direct_upload_max_bytes")]
    pub casebuilder_direct_upload_max_bytes: u64,
    #[serde(default = "default_casebuilder_single_upload_max_bytes")]
    pub casebuilder_single_upload_max_bytes: u64,
    #[serde(default = "default_casebuilder_multipart_part_bytes")]
    pub casebuilder_multipart_part_bytes: u64,
    #[serde(default = "default_casebuilder_multipart_session_ttl_seconds")]
    pub casebuilder_multipart_session_ttl_seconds: u64,
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
    #[serde(default = "default_casebuilder_timeline_agent_provider")]
    pub casebuilder_timeline_agent_provider: String,
    #[serde(default)]
    pub casebuilder_timeline_agent_model: Option<String>,
    #[serde(default)]
    pub openai_api_key: Option<String>,
    #[serde(default = "default_casebuilder_timeline_agent_timeout_ms")]
    pub casebuilder_timeline_agent_timeout_ms: u64,
    #[serde(default = "default_casebuilder_timeline_agent_max_input_chars")]
    pub casebuilder_timeline_agent_max_input_chars: usize,
    #[serde(default = "default_casebuilder_timeline_agent_harness_version")]
    pub casebuilder_timeline_agent_harness_version: String,
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
    #[serde(default)]
    pub casebuilder_embeddings_enabled: bool,

    // Authority Release / Cost Controls
    #[serde(default = "default_corpus_release_manifest_path")]
    pub corpus_release_manifest_path: String,
    #[serde(default = "default_authority_cache_ttl_seconds")]
    pub authority_cache_ttl_seconds: u64,
    #[serde(default = "default_authority_cache_max_capacity")]
    pub authority_cache_max_capacity: u64,
    #[serde(default = "default_query_embedding_cache_ttl_seconds")]
    pub query_embedding_cache_ttl_seconds: u64,
    #[serde(default = "default_query_embedding_cache_max_capacity")]
    pub query_embedding_cache_max_capacity: u64,
    #[serde(default = "default_rerank_policy")]
    pub rerank_policy: String,
    #[serde(default)]
    pub authority_edge_base_url: Option<String>,

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

fn default_auth_admin_role() -> String {
    "orsgraph_admin".to_string()
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

fn default_casebuilder_api_upload_max_bytes() -> u64 {
    default_r2_max_upload_bytes()
}

fn default_casebuilder_direct_upload_max_bytes() -> u64 {
    default_r2_max_upload_bytes()
}

fn default_casebuilder_single_upload_max_bytes() -> u64 {
    100 * 1024 * 1024
}

fn default_casebuilder_multipart_part_bytes() -> u64 {
    64 * 1024 * 1024
}

fn default_casebuilder_multipart_session_ttl_seconds() -> u64 {
    604_800
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

fn default_casebuilder_timeline_agent_provider() -> String {
    "disabled".to_string()
}

fn default_casebuilder_timeline_agent_timeout_ms() -> u64 {
    12_000
}

fn default_casebuilder_timeline_agent_max_input_chars() -> usize {
    30_000
}

fn default_casebuilder_timeline_agent_harness_version() -> String {
    "timeline-harness-v1".to_string()
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

fn default_corpus_release_manifest_path() -> String {
    "data/graph/corpus_release.json".to_string()
}

fn default_authority_cache_ttl_seconds() -> u64 {
    86_400
}

fn default_authority_cache_max_capacity() -> u64 {
    20_000
}

fn default_query_embedding_cache_ttl_seconds() -> u64 {
    604_800
}

fn default_query_embedding_cache_max_capacity() -> u64 {
    50_000
}

fn default_rerank_policy() -> String {
    "low_confidence".to_string()
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
        if let Some(value) = read_bool("ORS_AUTH_ENABLED") {
            self.auth_enabled = value;
        }
        if let Some(value) = read_string("ORS_AUTH_ISSUER") {
            self.auth_issuer = Some(value);
        }
        if let Some(value) = read_string("ORS_AUTH_AUDIENCE") {
            self.auth_audience = Some(value);
        }
        if let Some(value) = read_string("ORS_AUTH_ADMIN_ROLE") {
            self.auth_admin_role = value;
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
        let legacy_upload_max = read_u64("ORS_R2_MAX_UPLOAD_BYTES");
        if let Some(value) = legacy_upload_max {
            self.r2_max_upload_bytes = value;
            self.casebuilder_api_upload_max_bytes = value;
            self.casebuilder_direct_upload_max_bytes = value;
        }
        if let Some(value) = read_u64("ORS_CASEBUILDER_API_UPLOAD_MAX_BYTES") {
            self.casebuilder_api_upload_max_bytes = value;
        }
        if let Some(value) = read_u64("ORS_CASEBUILDER_DIRECT_UPLOAD_MAX_BYTES") {
            self.casebuilder_direct_upload_max_bytes = value;
        }
        if let Some(value) = read_u64("ORS_CASEBUILDER_SINGLE_UPLOAD_MAX_BYTES") {
            self.casebuilder_single_upload_max_bytes = value;
        }
        if let Some(value) = read_u64("ORS_CASEBUILDER_MULTIPART_PART_BYTES") {
            self.casebuilder_multipart_part_bytes = value;
        }
        if let Some(value) = read_u64("ORS_CASEBUILDER_MULTIPART_SESSION_TTL_SECONDS") {
            self.casebuilder_multipart_session_ttl_seconds = value;
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
        if let Some(value) = read_string("ORS_CASEBUILDER_TIMELINE_AGENT_PROVIDER") {
            self.casebuilder_timeline_agent_provider = value;
        }
        if let Some(value) = read_string("ORS_CASEBUILDER_TIMELINE_AGENT_MODEL") {
            self.casebuilder_timeline_agent_model = Some(value);
        }
        if let Some(value) =
            read_string("ORS_OPENAI_API_KEY").or_else(|| read_string("OPENAI_API_KEY"))
        {
            self.openai_api_key = Some(value);
        }
        if let Some(value) = read_u64("ORS_CASEBUILDER_TIMELINE_AGENT_TIMEOUT_MS") {
            self.casebuilder_timeline_agent_timeout_ms = value;
        }
        if let Some(value) = read_usize("ORS_CASEBUILDER_TIMELINE_AGENT_MAX_INPUT_CHARS") {
            self.casebuilder_timeline_agent_max_input_chars = value;
        }
        if let Some(value) = read_string("ORS_CASEBUILDER_TIMELINE_AGENT_HARNESS_VERSION") {
            self.casebuilder_timeline_agent_harness_version = value;
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
        if let Some(value) = read_bool("ORS_CASEBUILDER_EMBEDDINGS_ENABLED") {
            self.casebuilder_embeddings_enabled = value;
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
        if let Some(value) = read_string("ORS_CORPUS_RELEASE_MANIFEST_PATH") {
            self.corpus_release_manifest_path = value;
        }
        if let Some(value) = read_u64("ORS_AUTHORITY_CACHE_TTL_SECONDS") {
            self.authority_cache_ttl_seconds = value;
        }
        if let Some(value) = read_u64("ORS_AUTHORITY_CACHE_MAX_CAPACITY") {
            self.authority_cache_max_capacity = value;
        }
        if let Some(value) = read_u64("ORS_QUERY_EMBEDDING_CACHE_TTL_SECONDS") {
            self.query_embedding_cache_ttl_seconds = value;
        }
        if let Some(value) = read_u64("ORS_QUERY_EMBEDDING_CACHE_MAX_CAPACITY") {
            self.query_embedding_cache_max_capacity = value;
        }
        if let Some(value) = read_string("ORS_RERANK_POLICY") {
            self.rerank_policy = value;
        }
        if let Some(value) = read_string("ORS_AUTHORITY_EDGE_BASE_URL") {
            self.authority_edge_base_url = Some(value);
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
        self.auth_issuer = self
            .auth_issuer
            .as_ref()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty());
        self.auth_audience = self
            .auth_audience
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        self.auth_admin_role = self.auth_admin_role.trim().to_string();
        if self.auth_admin_role.is_empty() {
            self.auth_admin_role = default_auth_admin_role();
        }
        if self.auth_enabled && self.auth_issuer.is_none() {
            return Err(ConfigError::Message(
                "ORS_AUTH_ISSUER is required when ORS_AUTH_ENABLED=true".to_string(),
            ));
        }
        if self.auth_enabled && self.auth_audience.is_none() {
            return Err(ConfigError::Message(
                "ORS_AUTH_AUDIENCE is required when ORS_AUTH_ENABLED=true".to_string(),
            ));
        }
        self.storage_backend = self.storage_backend.trim().to_ascii_lowercase();
        if !matches!(self.storage_backend.as_str(), "local" | "r2") {
            return Err(ConfigError::Message(format!(
                "Unsupported ORS_STORAGE_BACKEND {}; expected local or r2",
                self.storage_backend
            )));
        }
        normalize_optional_string(&mut self.r2_account_id);
        normalize_optional_string(&mut self.r2_bucket);
        normalize_optional_string(&mut self.r2_access_key_id);
        normalize_optional_string(&mut self.r2_secret_access_key);
        self.r2_endpoint = self
            .r2_endpoint
            .as_ref()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty());
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
        if self.casebuilder_api_upload_max_bytes == 0 {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_API_UPLOAD_MAX_BYTES must be greater than 0".to_string(),
            ));
        }
        if self.casebuilder_direct_upload_max_bytes == 0 {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_DIRECT_UPLOAD_MAX_BYTES must be greater than 0".to_string(),
            ));
        }
        if self.casebuilder_single_upload_max_bytes == 0 {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_SINGLE_UPLOAD_MAX_BYTES must be greater than 0".to_string(),
            ));
        }
        if self.casebuilder_multipart_part_bytes < 5 * 1024 * 1024
            || self.casebuilder_multipart_part_bytes > 5 * 1024 * 1024 * 1024
        {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_MULTIPART_PART_BYTES must be between 5242880 and 5368709120"
                    .to_string(),
            ));
        }
        if self
            .casebuilder_direct_upload_max_bytes
            .div_ceil(self.casebuilder_multipart_part_bytes)
            > 10_000
        {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_DIRECT_UPLOAD_MAX_BYTES requires more than 10000 multipart parts"
                    .to_string(),
            ));
        }
        if self.casebuilder_multipart_session_ttl_seconds == 0
            || self.casebuilder_multipart_session_ttl_seconds > 604_800
        {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_MULTIPART_SESSION_TTL_SECONDS must be between 1 and 604800"
                    .to_string(),
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
        self.casebuilder_timeline_agent_provider = self
            .casebuilder_timeline_agent_provider
            .trim()
            .to_ascii_lowercase();
        if !matches!(
            self.casebuilder_timeline_agent_provider.as_str(),
            "disabled" | "openai"
        ) {
            return Err(ConfigError::Message(format!(
                "Unsupported ORS_CASEBUILDER_TIMELINE_AGENT_PROVIDER {}; expected disabled or openai",
                self.casebuilder_timeline_agent_provider
            )));
        }
        self.casebuilder_timeline_agent_model = self
            .casebuilder_timeline_agent_model
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        self.openai_api_key = self
            .openai_api_key
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if self.casebuilder_timeline_agent_timeout_ms == 0 {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_TIMELINE_AGENT_TIMEOUT_MS must be greater than 0".to_string(),
            ));
        }
        if self.casebuilder_timeline_agent_max_input_chars == 0 {
            return Err(ConfigError::Message(
                "ORS_CASEBUILDER_TIMELINE_AGENT_MAX_INPUT_CHARS must be greater than 0".to_string(),
            ));
        }
        self.casebuilder_timeline_agent_harness_version = self
            .casebuilder_timeline_agent_harness_version
            .trim()
            .to_string();
        if self.casebuilder_timeline_agent_harness_version.is_empty() {
            self.casebuilder_timeline_agent_harness_version =
                default_casebuilder_timeline_agent_harness_version();
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
        self.corpus_release_manifest_path = self.corpus_release_manifest_path.trim().to_string();
        if self.corpus_release_manifest_path.is_empty() {
            self.corpus_release_manifest_path = default_corpus_release_manifest_path();
        }
        if self.authority_cache_ttl_seconds == 0 {
            return Err(ConfigError::Message(
                "ORS_AUTHORITY_CACHE_TTL_SECONDS must be greater than 0".to_string(),
            ));
        }
        if self.authority_cache_max_capacity == 0 {
            return Err(ConfigError::Message(
                "ORS_AUTHORITY_CACHE_MAX_CAPACITY must be greater than 0".to_string(),
            ));
        }
        if self.query_embedding_cache_ttl_seconds == 0 {
            return Err(ConfigError::Message(
                "ORS_QUERY_EMBEDDING_CACHE_TTL_SECONDS must be greater than 0".to_string(),
            ));
        }
        if self.query_embedding_cache_max_capacity == 0 {
            return Err(ConfigError::Message(
                "ORS_QUERY_EMBEDDING_CACHE_MAX_CAPACITY must be greater than 0".to_string(),
            ));
        }
        self.rerank_policy = self.rerank_policy.trim().to_ascii_lowercase();
        if !matches!(
            self.rerank_policy.as_str(),
            "explicit" | "low_confidence" | "always"
        ) {
            return Err(ConfigError::Message(
                "ORS_RERANK_POLICY must be explicit, low_confidence, or always".to_string(),
            ));
        }
        self.authority_edge_base_url = self
            .authority_edge_base_url
            .as_ref()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty());
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

fn normalize_optional_string(value: &mut Option<String>) {
    *value = value
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
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
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn test_config() -> ApiConfig {
        ApiConfig {
            api_host: "127.0.0.1".to_string(),
            api_port: 8080,
            neo4j_uri: "bolt://localhost:7687".to_string(),
            neo4j_user: "neo4j".to_string(),
            neo4j_password: "neo4j".to_string(),
            api_key: None,
            auth_enabled: false,
            auth_issuer: None,
            auth_audience: None,
            auth_admin_role: "orsgraph_admin".to_string(),
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
            casebuilder_api_upload_max_bytes: 10,
            casebuilder_direct_upload_max_bytes: 10,
            casebuilder_single_upload_max_bytes: 100 * 1024 * 1024,
            casebuilder_multipart_part_bytes: 64 * 1024 * 1024,
            casebuilder_multipart_session_ttl_seconds: 604_800,
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
            casebuilder_timeline_agent_provider: "disabled".to_string(),
            casebuilder_timeline_agent_model: None,
            openai_api_key: None,
            casebuilder_timeline_agent_timeout_ms: 12_000,
            casebuilder_timeline_agent_max_input_chars: 30_000,
            casebuilder_timeline_agent_harness_version: "timeline-harness-v1".to_string(),
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
            casebuilder_embeddings_enabled: false,
            corpus_release_manifest_path: "data/graph/corpus_release.json".to_string(),
            authority_cache_ttl_seconds: 86_400,
            authority_cache_max_capacity: 20_000,
            query_embedding_cache_ttl_seconds: 604_800,
            query_embedding_cache_max_capacity: 50_000,
            rerank_policy: "low_confidence".to_string(),
            authority_edge_base_url: None,
            admin_enabled: false,
            admin_allow_kill: false,
            admin_jobs_dir: "data/admin/jobs".to_string(),
            admin_workdir: ".".to_string(),
            admin_crawler_bin: "cargo".to_string(),
            admin_data_dir: "data".to_string(),
        }
    }

    #[test]
    fn storage_backend_defaults_to_local() {
        let mut config = test_config();
        config.storage_backend = "LOCAL".to_string();
        config.normalize_and_validate().unwrap();
        assert_eq!(config.storage_backend, "local");
    }

    #[test]
    fn r2_backend_requires_credentials() {
        let mut config = test_config();
        config.storage_backend = "r2".to_string();
        config.r2_account_id = None;
        config.r2_bucket = Some("bucket".to_string());
        config.r2_access_key_id = Some("access".to_string());
        config.r2_secret_access_key = Some("secret".to_string());

        let error = config.normalize_and_validate().unwrap_err().to_string();
        assert!(error.contains("ORS_R2_ACCOUNT_ID"));
    }

    #[test]
    fn r2_config_values_are_trimmed() {
        let mut config = test_config();
        config.storage_backend = " r2 ".to_string();
        config.r2_account_id = Some(" account-id ".to_string());
        config.r2_bucket = Some(" bucket ".to_string());
        config.r2_access_key_id = Some(" access ".to_string());
        config.r2_secret_access_key = Some(" secret ".to_string());
        config.r2_endpoint = Some(" https://account-id.r2.cloudflarestorage.com/ ".to_string());

        config.normalize_and_validate().unwrap();

        assert_eq!(config.storage_backend, "r2");
        assert_eq!(config.r2_account_id.as_deref(), Some("account-id"));
        assert_eq!(config.r2_bucket.as_deref(), Some("bucket"));
        assert_eq!(config.r2_access_key_id.as_deref(), Some("access"));
        assert_eq!(config.r2_secret_access_key.as_deref(), Some("secret"));
        assert_eq!(
            config.r2_endpoint.as_deref(),
            Some("https://account-id.r2.cloudflarestorage.com")
        );
    }

    #[test]
    fn legacy_r2_upload_limit_applies_to_casebuilder_upload_limits() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("ORS_R2_MAX_UPLOAD_BYTES", "12345");
        }
        let mut config = test_config();
        config.apply_explicit_env_overrides();
        unsafe {
            std::env::remove_var("ORS_R2_MAX_UPLOAD_BYTES");
        }

        assert_eq!(config.r2_max_upload_bytes, 12_345);
        assert_eq!(config.casebuilder_api_upload_max_bytes, 12_345);
        assert_eq!(config.casebuilder_direct_upload_max_bytes, 12_345);
    }

    #[test]
    fn casebuilder_specific_upload_limits_override_legacy_limit() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("ORS_R2_MAX_UPLOAD_BYTES", "12345");
            std::env::set_var("ORS_CASEBUILDER_API_UPLOAD_MAX_BYTES", "50");
            std::env::set_var("ORS_CASEBUILDER_DIRECT_UPLOAD_MAX_BYTES", "200");
            std::env::set_var("ORS_CASEBUILDER_SINGLE_UPLOAD_MAX_BYTES", "100");
            std::env::set_var("ORS_CASEBUILDER_MULTIPART_PART_BYTES", "5242880");
        }
        let mut config = test_config();
        config.apply_explicit_env_overrides();
        unsafe {
            std::env::remove_var("ORS_R2_MAX_UPLOAD_BYTES");
            std::env::remove_var("ORS_CASEBUILDER_API_UPLOAD_MAX_BYTES");
            std::env::remove_var("ORS_CASEBUILDER_DIRECT_UPLOAD_MAX_BYTES");
            std::env::remove_var("ORS_CASEBUILDER_SINGLE_UPLOAD_MAX_BYTES");
            std::env::remove_var("ORS_CASEBUILDER_MULTIPART_PART_BYTES");
        }

        assert_eq!(config.r2_max_upload_bytes, 12_345);
        assert_eq!(config.casebuilder_api_upload_max_bytes, 50);
        assert_eq!(config.casebuilder_direct_upload_max_bytes, 200);
        assert_eq!(config.casebuilder_single_upload_max_bytes, 100);
        assert_eq!(config.casebuilder_multipart_part_bytes, 5 * 1024 * 1024);
    }

    #[test]
    fn multipart_part_size_must_respect_s3_limits() {
        let mut config = test_config();
        config.casebuilder_multipart_part_bytes = 1024;

        let error = config.normalize_and_validate().unwrap_err().to_string();

        assert!(error.contains("ORS_CASEBUILDER_MULTIPART_PART_BYTES"));
    }
}

fn read_f32(name: &str) -> Option<f32> {
    read_string(name).and_then(|value| value.parse().ok())
}
