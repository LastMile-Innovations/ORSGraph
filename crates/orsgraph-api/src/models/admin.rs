use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdminJobKind {
    Crawl,
    Parse,
    Qc,
    SeedNeo4j,
    MaterializeNeo4j,
    EmbedNeo4j,
    SourceIngest,
    CombineGraph,
}

impl AdminJobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Crawl => "crawl",
            Self::Parse => "parse",
            Self::Qc => "qc",
            Self::SeedNeo4j => "seed_neo4j",
            Self::MaterializeNeo4j => "materialize_neo4j",
            Self::EmbedNeo4j => "embed_neo4j",
            Self::SourceIngest => "source_ingest",
            Self::CombineGraph => "combine_graph",
        }
    }

    pub fn is_read_only(self) -> bool {
        matches!(self, Self::Qc)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdminJobStatus {
    Queued,
    Running,
    CancelRequested,
    Succeeded,
    Failed,
    Cancelled,
}

impl AdminJobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::CancelRequested => "cancel_requested",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AdminJobParams {
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub out_dir: Option<String>,
    #[serde(default)]
    pub graph_dir: Option<String>,
    #[serde(default)]
    pub edition_year: Option<i32>,
    #[serde(default)]
    pub max_chapters: Option<usize>,
    #[serde(default)]
    pub chapters: Option<String>,
    #[serde(default)]
    pub session_key: Option<String>,
    #[serde(default)]
    pub fetch_only: Option<bool>,
    #[serde(default)]
    pub skip_citation_resolution: Option<bool>,
    #[serde(default)]
    pub dry_run: Option<bool>,
    #[serde(default)]
    pub embed: Option<bool>,
    #[serde(default)]
    pub create_vector_indexes: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminStartJobRequest {
    pub kind: AdminJobKind,
    #[serde(default)]
    pub params: AdminJobParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdminJobProgress {
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub stdout_lines: usize,
    #[serde(default)]
    pub stderr_lines: usize,
    #[serde(default)]
    pub event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminJob {
    pub job_id: String,
    pub kind: AdminJobKind,
    pub status: AdminJobStatus,
    pub params: AdminJobParams,
    pub command: Vec<String>,
    pub command_display: String,
    pub is_read_only: bool,
    pub created_at_ms: u128,
    #[serde(default)]
    pub started_at_ms: Option<u128>,
    #[serde(default)]
    pub finished_at_ms: Option<u128>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub output_paths: BTreeMap<String, String>,
    #[serde(default)]
    pub progress: AdminJobProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminJobEvent {
    pub event_id: String,
    pub job_id: String,
    pub timestamp_ms: u128,
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub stream: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminJobDetail {
    pub job: AdminJob,
    pub allow_kill: bool,
    pub recent_events: Vec<AdminJobEvent>,
    pub stdout_tail: Vec<String>,
    pub stderr_tail: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminOverview {
    pub enabled: bool,
    pub allow_kill: bool,
    pub active_job: Option<AdminJob>,
    pub recent_jobs: Vec<AdminJob>,
    pub job_counts: BTreeMap<String, usize>,
    pub paths: AdminPathSummary,
    pub crawler: AdminCrawlerSummary,
    pub sources: AdminSourceSummary,
    pub graph: AdminGraphSummary,
    pub indexing: AdminIndexingSummary,
    pub health: AdminHealthSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminPathSummary {
    pub jobs_dir: String,
    pub data_dir: String,
    pub graph_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminCrawlerSummary {
    pub configured_bin: String,
    pub command_prefix: Vec<String>,
    pub workdir: String,
    pub control_mode: String,
    pub active_pid: Option<u32>,
    pub active_mutating_job: bool,
    pub running_jobs: usize,
    pub read_only_running_jobs: usize,
    pub mutating_running_jobs: usize,
    #[serde(default)]
    pub last_success_at_ms: Option<u128>,
    #[serde(default)]
    pub last_failure_at_ms: Option<u128>,
    #[serde(default)]
    pub last_terminal_status: Option<AdminJobStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdminSourceSummary {
    pub registry_sources: usize,
    pub source_dirs: usize,
    pub source_artifacts: usize,
    pub source_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminGraphSummary {
    pub jsonl_files: usize,
    pub rows: usize,
    pub bytes: u64,
    #[serde(default = "default_true")]
    pub rows_are_exact: bool,
}

impl Default for AdminGraphSummary {
    fn default() -> Self {
        Self {
            jsonl_files: 0,
            rows: 0,
            bytes: 0,
            rows_are_exact: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminIndexingSummary {
    pub vector_enabled: bool,
    pub vector_search_enabled: bool,
    pub vector_index: String,
    pub vector_dimension: usize,
    pub embedding_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminHealthSummary {
    pub api: String,
    pub neo4j: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminLogResponse {
    pub job_id: String,
    pub stream: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSourceRegistryResponse {
    pub sources: Vec<AdminSourceRegistryEntry>,
    pub totals: AdminSourceRegistryTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdminSourceRegistryTotals {
    pub sources: usize,
    pub p0_sources: usize,
    pub local_source_dirs: usize,
    pub local_artifacts: usize,
    pub local_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSourceRegistryEntry {
    pub source_id: String,
    pub name: String,
    pub owner: String,
    pub jurisdiction: String,
    pub source_type: String,
    pub access: String,
    pub official_status: String,
    pub connector_status: String,
    pub priority: String,
    pub source_url: String,
    pub docs_url: String,
    pub graph_nodes_created: Vec<String>,
    pub graph_edges_created: Vec<String>,
    pub local: AdminSourceLocalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdminSourceLocalStatus {
    pub source_dir_exists: bool,
    pub source_artifacts: usize,
    pub source_bytes: u64,
    pub graph_files: usize,
    pub graph_rows: usize,
    #[serde(default)]
    pub qc_status: Option<String>,
    #[serde(default)]
    pub last_finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSourceDetail {
    pub source: AdminSourceRegistryEntry,
    #[serde(default)]
    pub stats: Option<serde_json::Value>,
    #[serde(default)]
    pub qc_report: Option<serde_json::Value>,
    pub graph_files: Vec<AdminSourceGraphFile>,
    pub raw_artifacts: Vec<AdminSourceArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSourceGraphFile {
    pub file: String,
    pub rows: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSourceArtifact {
    pub file: String,
    pub bytes: u64,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub raw_hash: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub skipped: Option<bool>,
}
