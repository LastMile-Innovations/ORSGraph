use crate::config::ApiConfig;
use crate::error::{ApiError, ApiResult};
use crate::models::admin::{
    AdminCrawlerSummary, AdminGraphSummary, AdminHealthSummary, AdminIndexingSummary, AdminJob,
    AdminJobDetail, AdminJobEvent, AdminJobKind, AdminJobParams, AdminJobStatus, AdminLogResponse,
    AdminOverview, AdminPathSummary, AdminPerformanceSummary, AdminSourceArtifact,
    AdminSourceDetail, AdminSourceGraphFile, AdminSourceLocalStatus, AdminSourceRegistryEntry,
    AdminSourceRegistryResponse, AdminSourceRegistryTotals, AdminSourceSummary,
    AdminStartJobRequest,
};
use crate::services::health::HealthService;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Component, Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, SeekFrom};
use tokio::process::{ChildStderr, ChildStdout, Command};
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinSet;

const SOURCE_HYDRATE_CONCURRENCY: usize = 16;
const SOURCE_REGISTRY_RELATIVE_PATH: &str = "docs/data/source-registry.yaml";
const SOURCE_REGISTRY_EMBEDDED: &str = r#"{
  "sources": [
    {
      "source_id": "or_leg_ors_html",
      "name": "Oregon Revised Statutes",
      "owner": "Oregon Legislature",
      "jurisdiction": "or:state",
      "source_type": "static_html",
      "access": "free",
      "official_status": "official",
      "connector_status": "implemented",
      "priority": "P0",
      "source_url": "https://www.oregonlegislature.gov/bills_laws/ors/ors.html",
      "docs_url": "https://www.oregonlegislature.gov/bills_laws/Pages/ORS.aspx",
      "graph_nodes_created": ["Statute", "Chapter", "Title", "SourceDocument"],
      "graph_edges_created": ["IN_CHAPTER", "IN_TITLE", "DERIVED_FROM"]
    }
  ]
}"#;

#[derive(Clone)]
pub struct AdminService {
    inner: Arc<AdminInner>,
}

struct AdminInner {
    config: Arc<ApiConfig>,
    jobs_dir: PathBuf,
    jobs: RwLock<HashMap<String, AdminJob>>,
    running: RwLock<HashMap<String, RunningJob>>,
    counter: AtomicU64,
}

#[derive(Debug, Clone)]
struct RunningJob {
    pid: Option<u32>,
}

#[derive(Debug, Clone)]
struct BuiltCommand {
    program: String,
    args: Vec<String>,
    display: Vec<String>,
    output_paths: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct JobIndex {
    jobs: Vec<String>,
}

impl AdminService {
    pub async fn new(config: Arc<ApiConfig>) -> anyhow::Result<Self> {
        let jobs_dir = PathBuf::from(&config.admin_jobs_dir);

        let service = Self {
            inner: Arc::new(AdminInner {
                config,
                jobs_dir,
                jobs: RwLock::new(HashMap::new()),
                running: RwLock::new(HashMap::new()),
                counter: AtomicU64::new(0),
            }),
        };
        if service.enabled() {
            fs::create_dir_all(&service.inner.jobs_dir).await?;
            service.recover_jobs().await?;
        }
        Ok(service)
    }

    pub fn enabled(&self) -> bool {
        self.inner.config.admin_enabled
    }

    pub fn allow_kill(&self) -> bool {
        self.inner.config.admin_allow_kill
    }

    pub async fn overview(&self, health: &HealthService) -> ApiResult<AdminOverview> {
        self.require_enabled()?;
        let neo4j_ok = health.check_neo4j().await.unwrap_or(false);
        let jobs = self.sorted_jobs().await;
        let crawler = self.crawler_summary(&jobs).await;
        let active_job = jobs.iter().find(|job| !job.status.is_terminal()).cloned();
        let recent_jobs = jobs.iter().take(12).cloned().collect::<Vec<_>>();
        let mut job_counts = BTreeMap::<String, usize>::new();
        for job in &jobs {
            *job_counts
                .entry(job.status.as_str().to_string())
                .or_default() += 1;
        }
        let data_dir = PathBuf::from(&self.inner.config.admin_data_dir);
        let graph_dir = data_dir.join("graph");
        let graph = summarize_graph_fast(&graph_dir).await;
        let corpus_release_manifest_path =
            PathBuf::from(&self.inner.config.corpus_release_manifest_path);
        let corpus_release_id = read_corpus_release_id(&corpus_release_manifest_path)
            .await
            .unwrap_or_else(|| "release:unversioned".to_string());

        Ok(AdminOverview {
            enabled: self.enabled(),
            allow_kill: self.allow_kill(),
            active_job,
            recent_jobs,
            job_counts,
            paths: AdminPathSummary {
                jobs_dir: self.inner.jobs_dir.display().to_string(),
                data_dir: self.inner.config.admin_data_dir.clone(),
                graph_dir: graph_dir.display().to_string(),
            },
            crawler,
            sources: summarize_sources(&data_dir).await,
            graph: graph.clone(),
            indexing: AdminIndexingSummary {
                vector_enabled: self.inner.config.vector_enabled,
                vector_search_enabled: self.inner.config.vector_search_enabled,
                vector_index: self.inner.config.vector_index.clone(),
                vector_dimension: self.inner.config.vector_dimension,
                embedding_model: self.inner.config.embedding_model.clone(),
            },
            performance: AdminPerformanceSummary {
                corpus_release_id,
                corpus_release_manifest_path: corpus_release_manifest_path.display().to_string(),
                authority_cache_ttl_seconds: self.inner.config.authority_cache_ttl_seconds,
                authority_cache_max_capacity: self.inner.config.authority_cache_max_capacity,
                query_embedding_cache_ttl_seconds: self
                    .inner
                    .config
                    .query_embedding_cache_ttl_seconds,
                query_embedding_cache_max_capacity: self
                    .inner
                    .config
                    .query_embedding_cache_max_capacity,
                rerank_policy: self.inner.config.rerank_policy.clone(),
                edge_authority_base_url: self.inner.config.authority_edge_base_url.clone(),
                estimated_graph_storage_gb: bytes_to_gb(graph.bytes),
                estimated_r2_storage_gb: bytes_to_gb(graph.bytes),
                model_spend_policy: "delta_only_by_embedding_input_hash_and_release".to_string(),
            },
            health: AdminHealthSummary {
                api: "connected".to_string(),
                neo4j: if neo4j_ok {
                    "connected".to_string()
                } else {
                    "disconnected".to_string()
                },
                version: "0.1.0".to_string(),
            },
        })
    }

    pub async fn list_jobs(
        &self,
        status: Option<AdminJobStatus>,
        kind: Option<AdminJobKind>,
        limit: usize,
        offset: usize,
    ) -> ApiResult<Vec<AdminJob>> {
        self.require_enabled()?;
        let limit = limit.clamp(1, 100);
        let jobs = self
            .sorted_jobs()
            .await
            .into_iter()
            .filter(|job| status.is_none_or(|wanted| job.status == wanted))
            .filter(|job| kind.is_none_or(|wanted| job.kind == wanted))
            .skip(offset)
            .take(limit)
            .collect();
        Ok(jobs)
    }

    pub async fn list_sources(
        &self,
        priority: Option<String>,
        connector_status: Option<String>,
    ) -> ApiResult<AdminSourceRegistryResponse> {
        self.require_enabled()?;
        let data_dir = PathBuf::from(&self.inner.config.admin_data_dir);
        let mut sources = read_source_registry().await?;
        filter_source_registry_entries(
            &mut sources,
            priority.as_deref(),
            connector_status.as_deref(),
        );
        sources = hydrate_source_local_status(sources, &data_dir).await;
        sources.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.source_id.cmp(&right.source_id))
        });
        let local_source_dirs = sources
            .iter()
            .filter(|source| source.local.source_dir_exists)
            .count();
        let local_artifacts = sources
            .iter()
            .map(|source| source.local.source_artifacts)
            .sum();
        let local_bytes = sources.iter().map(|source| source.local.source_bytes).sum();
        Ok(AdminSourceRegistryResponse {
            totals: AdminSourceRegistryTotals {
                sources: sources.len(),
                p0_sources: sources
                    .iter()
                    .filter(|source| source.priority == "P0")
                    .count(),
                local_source_dirs,
                local_artifacts,
                local_bytes,
            },
            sources,
        })
    }

    pub async fn get_source(&self, source_id: &str) -> ApiResult<AdminSourceDetail> {
        self.require_enabled()?;
        if source_id.contains('/') || source_id.contains('\\') || source_id.contains('\0') {
            return Err(ApiError::BadRequest("invalid source_id".to_string()));
        }
        let data_dir = PathBuf::from(&self.inner.config.admin_data_dir);
        let source = read_source_registry()
            .await?
            .into_iter()
            .find(|source| source.source_id == source_id)
            .ok_or_else(|| ApiError::NotFound("source not found".to_string()))?;
        let source = hydrate_one_source(source, &data_dir).await;
        let source_dir = data_dir.join("sources").join(source_id);
        let stats = read_json_value(&source_dir.join("stats.json")).await;
        let qc_report = read_json_value(&source_dir.join("qc/report.json")).await;
        let graph_files = list_source_graph_files(&source_dir.join("graph")).await;
        let raw_artifacts = list_source_artifacts(&source_dir.join("raw")).await;
        Ok(AdminSourceDetail {
            source,
            stats,
            qc_report,
            graph_files,
            raw_artifacts,
        })
    }

    pub async fn get_job_detail(&self, job_id: &str) -> ApiResult<AdminJobDetail> {
        self.require_enabled()?;
        let job = self.get_job(job_id).await?;
        Ok(AdminJobDetail {
            allow_kill: self.allow_kill(),
            recent_events: self.tail_events(job_id, 80).await,
            stdout_tail: self.tail_log(job_id, "stdout", 120).await?,
            stderr_tail: self.tail_log(job_id, "stderr", 120).await?,
            job,
        })
    }

    pub async fn get_logs(
        &self,
        job_id: &str,
        stream: &str,
        tail: usize,
    ) -> ApiResult<AdminLogResponse> {
        self.require_enabled()?;
        if stream != "stdout" && stream != "stderr" {
            return Err(ApiError::BadRequest(
                "stream must be stdout or stderr".to_string(),
            ));
        }
        self.get_job(job_id).await?;
        Ok(AdminLogResponse {
            job_id: job_id.to_string(),
            stream: stream.to_string(),
            lines: self.tail_log(job_id, stream, tail.clamp(1, 1000)).await?,
        })
    }

    pub async fn start_job(&self, request: AdminStartJobRequest) -> ApiResult<AdminJobDetail> {
        self.require_enabled()?;
        let command = self.build_command(request.kind, &request.params)?;

        if !request.kind.is_read_only() && self.has_active_mutating_job().await {
            return Err(ApiError::Conflict(
                "another mutating admin job is already running".to_string(),
            ));
        }

        let job_id = self.next_job_id();
        let job_dir = self.job_dir(&job_id);
        fs::create_dir_all(&job_dir)
            .await
            .map_err(|e| ApiError::Internal(format!("failed to create job directory: {e}")))?;

        let now = now_ms();
        let job = AdminJob {
            job_id: job_id.clone(),
            kind: request.kind,
            status: AdminJobStatus::Running,
            params: request.params,
            command: command.display.clone(),
            command_display: command.display.join(" "),
            is_read_only: request.kind.is_read_only(),
            created_at_ms: now,
            started_at_ms: Some(now),
            finished_at_ms: None,
            exit_code: None,
            message: None,
            output_paths: command.output_paths.clone(),
            progress: Default::default(),
        };

        {
            let mut jobs = self.inner.jobs.write().await;
            jobs.insert(job_id.clone(), job.clone());
        }
        self.persist_job(&job).await?;
        self.persist_index().await?;
        self.append_event(&job_id, "info", "job started", None)
            .await?;

        if let Err(error) = self.spawn_process(job_id.clone(), command).await {
            let message = error.to_string();
            let _ = self
                .update_job_status(&job_id, AdminJobStatus::Failed, None, Some(message.clone()))
                .await;
            let _ = self
                .append_event(
                    &job_id,
                    "error",
                    &format!("job failed to start: {message}"),
                    None,
                )
                .await;
            return Err(error);
        }
        self.get_job_detail(&job_id).await
    }

    pub async fn cancel_job(&self, job_id: &str) -> ApiResult<AdminJobDetail> {
        self.require_enabled()?;
        let job = self.get_job(job_id).await?;
        if job.status.is_terminal() {
            return Ok(self.get_job_detail(job_id).await?);
        }
        self.update_job_status(
            job_id,
            AdminJobStatus::CancelRequested,
            None,
            Some("cancel requested".to_string()),
        )
        .await?;
        self.signal_job(job_id, "TERM").await?;
        self.append_event(job_id, "warning", "cancel requested", None)
            .await?;
        self.get_job_detail(job_id).await
    }

    pub async fn kill_job(&self, job_id: &str) -> ApiResult<AdminJobDetail> {
        self.require_enabled()?;
        if !self.allow_kill() {
            return Err(ApiError::Forbidden(
                "force kill is disabled; set ORS_ADMIN_ALLOW_KILL=true".to_string(),
            ));
        }
        let job = self.get_job(job_id).await?;
        if job.status.is_terminal() {
            return Ok(self.get_job_detail(job_id).await?);
        }
        self.signal_job(job_id, "KILL").await?;
        self.append_event(job_id, "error", "force kill requested", None)
            .await?;
        self.get_job_detail(job_id).await
    }

    fn require_enabled(&self) -> ApiResult<()> {
        if self.enabled() {
            Ok(())
        } else {
            Err(ApiError::Forbidden(
                "admin operations are disabled; set ORS_ADMIN_ENABLED=true".to_string(),
            ))
        }
    }

    async fn recover_jobs(&self) -> anyhow::Result<()> {
        let mut job_ids = self.read_index().await.unwrap_or_default().jobs;
        if job_ids.is_empty() {
            job_ids = self.scan_job_dirs().await?;
        }
        job_ids.sort();
        job_ids.dedup();

        let mut recovered = HashMap::new();
        for job_id in job_ids {
            let path = self.job_dir(&job_id).join("job.json");
            let Ok(bytes) = fs::read(&path).await else {
                continue;
            };
            let Ok(mut job) = serde_json::from_slice::<AdminJob>(&bytes) else {
                continue;
            };
            if !job.status.is_terminal() {
                job.status = AdminJobStatus::Failed;
                job.finished_at_ms = Some(now_ms());
                job.message = Some("API restarted before job reached a terminal state".to_string());
                let _ = self.persist_job(&job).await;
            }
            recovered.insert(job.job_id.clone(), job);
        }
        *self.inner.jobs.write().await = recovered;
        self.persist_index().await?;
        Ok(())
    }

    fn next_job_id(&self) -> String {
        let count = self.inner.counter.fetch_add(1, Ordering::Relaxed);
        format!("job-{}-{count}", now_ms())
    }

    fn job_dir(&self, job_id: &str) -> PathBuf {
        self.inner.jobs_dir.join(job_id)
    }

    async fn spawn_process(&self, job_id: String, command: BuiltCommand) -> ApiResult<()> {
        let mut child_command = Command::new(&command.program);
        child_command
            .args(&command.args)
            .current_dir(&self.inner.config.admin_workdir)
            .kill_on_drop(true)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        child_command.env("NEO4J_PASSWORD", self.inner.config.neo4j_password.clone());
        if let Some(key) = &self.inner.config.voyage_api_key {
            child_command.env("VOYAGE_API_KEY", key);
        }

        let mut child = child_command
            .spawn()
            .map_err(|e| ApiError::Internal(format!("failed to start job: {e}")))?;
        let pid = child.id();
        {
            self.inner
                .running
                .write()
                .await
                .insert(job_id.clone(), RunningJob { pid });
        }

        if let Some(stdout) = child.stdout.take() {
            let service = self.clone();
            let id = job_id.clone();
            tokio::spawn(async move {
                service.capture_stdout(id, stdout).await;
            });
        }
        if let Some(stderr) = child.stderr.take() {
            let service = self.clone();
            let id = job_id.clone();
            tokio::spawn(async move {
                service.capture_stderr(id, stderr).await;
            });
        }

        let service = self.clone();
        tokio::spawn(async move {
            let result = child.wait().await;
            service.finish_job(job_id, result).await;
        });

        Ok(())
    }

    async fn finish_job(&self, job_id: String, result: std::io::Result<std::process::ExitStatus>) {
        self.inner.running.write().await.remove(&job_id);
        let current = self.get_job(&job_id).await.ok();
        let cancel_requested =
            current.is_some_and(|job| job.status == AdminJobStatus::CancelRequested);
        let (status, exit_code, message) = match result {
            Ok(exit) if exit.success() && !cancel_requested => (
                AdminJobStatus::Succeeded,
                exit.code(),
                Some("job completed".to_string()),
            ),
            Ok(exit) if cancel_requested => (
                AdminJobStatus::Cancelled,
                exit.code(),
                Some("job cancelled".to_string()),
            ),
            Ok(exit) => (
                AdminJobStatus::Failed,
                exit.code(),
                Some(format!("job exited with status {exit}")),
            ),
            Err(error) => (
                AdminJobStatus::Failed,
                None,
                Some(format!("failed while waiting for job: {error}")),
            ),
        };
        let _ = self
            .update_job_status(&job_id, status, exit_code, message.clone())
            .await;
        let _ = self
            .append_event(
                &job_id,
                if status == AdminJobStatus::Succeeded {
                    "info"
                } else {
                    "error"
                },
                message.as_deref().unwrap_or("job finished"),
                None,
            )
            .await;
    }

    async fn capture_stdout(&self, job_id: String, stream: ChildStdout) {
        self.capture_stream(job_id, "stdout", BufReader::new(stream))
            .await;
    }

    async fn capture_stderr(&self, job_id: String, stream: ChildStderr) {
        self.capture_stream(job_id, "stderr", BufReader::new(stream))
            .await;
    }

    async fn capture_stream<R>(&self, job_id: String, stream_name: &str, reader: BufReader<R>)
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = self.append_log_line(&job_id, stream_name, &line).await;
            let _ = self.record_log_progress(&job_id, stream_name, &line).await;
        }
    }

    async fn append_log_line(&self, job_id: &str, stream: &str, line: &str) -> ApiResult<()> {
        let path = self.job_dir(job_id).join(format!("{stream}.log"));
        append_line(&path, line).await?;
        Ok(())
    }

    async fn record_log_progress(&self, job_id: &str, stream: &str, line: &str) -> ApiResult<()> {
        let mut event_message = None;
        if line.contains("═══") || line.contains("Phase ") || line.contains("Crawl Complete")
        {
            event_message = Some(line.trim().to_string());
        }
        let mut changed = None;
        {
            let mut jobs = self.inner.jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                if stream == "stdout" {
                    job.progress.stdout_lines += 1;
                } else {
                    job.progress.stderr_lines += 1;
                }
                if let Some(message) = &event_message {
                    job.progress.phase = Some(message.clone());
                    job.progress.event_count += 1;
                }
                changed = Some(job.clone());
            }
        }
        if let Some(job) = changed {
            self.persist_job(&job).await?;
        }
        if let Some(message) = event_message {
            self.append_event(job_id, "info", &message, Some(stream.to_string()))
                .await?;
        }
        Ok(())
    }

    async fn signal_job(&self, job_id: &str, signal: &str) -> ApiResult<()> {
        let running = self.inner.running.read().await.get(job_id).cloned();
        let pid = running
            .and_then(|job| job.pid)
            .ok_or_else(|| ApiError::NotFound("running process not found".to_string()))?;
        let status = Command::new("kill")
            .arg(format!("-{signal}"))
            .arg(pid.to_string())
            .status()
            .await
            .map_err(|e| ApiError::Internal(format!("failed to signal process {pid}: {e}")))?;
        if status.success() {
            Ok(())
        } else {
            Err(ApiError::Internal(format!(
                "kill -{signal} {pid} exited with {status}"
            )))
        }
    }

    async fn update_job_status(
        &self,
        job_id: &str,
        status: AdminJobStatus,
        exit_code: Option<i32>,
        message: Option<String>,
    ) -> ApiResult<()> {
        let changed = {
            let mut jobs = self.inner.jobs.write().await;
            let job = jobs
                .get_mut(job_id)
                .ok_or_else(|| ApiError::NotFound(format!("job {job_id} not found")))?;
            job.status = status;
            job.exit_code = exit_code;
            job.message = message;
            if status.is_terminal() {
                job.finished_at_ms = Some(now_ms());
            }
            job.clone()
        };
        self.persist_job(&changed).await?;
        Ok(())
    }

    async fn get_job(&self, job_id: &str) -> ApiResult<AdminJob> {
        self.inner
            .jobs
            .read()
            .await
            .get(job_id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("job {job_id} not found")))
    }

    async fn sorted_jobs(&self) -> Vec<AdminJob> {
        let mut jobs = self
            .inner
            .jobs
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        jobs.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
        jobs
    }

    async fn has_active_mutating_job(&self) -> bool {
        self.inner.jobs.read().await.values().any(|job| {
            !job.is_read_only
                && matches!(
                    job.status,
                    AdminJobStatus::Queued
                        | AdminJobStatus::Running
                        | AdminJobStatus::CancelRequested
                )
        })
    }

    async fn crawler_summary(&self, jobs: &[AdminJob]) -> AdminCrawlerSummary {
        let (_program, _args, command_prefix) = self.crawler_program();
        let active_job = jobs.iter().find(|job| !job.status.is_terminal());
        let active_pid = if let Some(job) = active_job {
            self.inner
                .running
                .read()
                .await
                .get(&job.job_id)
                .and_then(|running| running.pid)
        } else {
            None
        };

        let running_jobs = jobs.iter().filter(|job| !job.status.is_terminal()).count();
        let read_only_running_jobs = jobs
            .iter()
            .filter(|job| !job.status.is_terminal() && job.is_read_only)
            .count();
        let mutating_running_jobs = running_jobs.saturating_sub(read_only_running_jobs);
        let last_terminal_status = jobs
            .iter()
            .find(|job| job.status.is_terminal())
            .map(|job| job.status);
        let last_success_at_ms = jobs
            .iter()
            .find(|job| job.status == AdminJobStatus::Succeeded)
            .and_then(job_finished_or_started_at);
        let last_failure_at_ms = jobs
            .iter()
            .find(|job| {
                matches!(
                    job.status,
                    AdminJobStatus::Failed | AdminJobStatus::Cancelled
                )
            })
            .and_then(job_finished_or_started_at);
        let configured_bin = self.inner.config.admin_crawler_bin.trim().to_string();
        let control_mode = if configured_bin == "cargo" {
            "cargo-run".to_string()
        } else {
            "direct-binary".to_string()
        };

        AdminCrawlerSummary {
            configured_bin,
            command_prefix,
            workdir: self.inner.config.admin_workdir.clone(),
            control_mode,
            active_pid,
            active_mutating_job: mutating_running_jobs > 0,
            running_jobs,
            read_only_running_jobs,
            mutating_running_jobs,
            last_success_at_ms,
            last_failure_at_ms,
            last_terminal_status,
        }
    }

    async fn persist_job(&self, job: &AdminJob) -> ApiResult<()> {
        let path = self.job_dir(&job.job_id).join("job.json");
        write_json_pretty(&path, job).await
    }

    async fn persist_index(&self) -> ApiResult<()> {
        let mut ids = self
            .inner
            .jobs
            .read()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        ids.sort();
        let index = JobIndex { jobs: ids };
        write_json_pretty(&self.inner.jobs_dir.join("index.json"), &index).await
    }

    async fn read_index(&self) -> anyhow::Result<JobIndex> {
        let bytes = fs::read(self.inner.jobs_dir.join("index.json")).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    async fn scan_job_dirs(&self) -> anyhow::Result<Vec<String>> {
        let mut ids = Vec::new();
        let mut entries = fs::read_dir(&self.inner.jobs_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.join("job.json").is_file() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
                ids.push(name.to_string());
            }
        }
        Ok(ids)
    }

    async fn append_event(
        &self,
        job_id: &str,
        level: &str,
        message: &str,
        stream: Option<String>,
    ) -> ApiResult<()> {
        let event = AdminJobEvent {
            event_id: format!("evt-{}", now_ms()),
            job_id: job_id.to_string(),
            timestamp_ms: now_ms(),
            level: level.to_string(),
            message: message.to_string(),
            stream,
        };
        let path = self.job_dir(job_id).join("events.jsonl");
        let line = serde_json::to_string(&event)
            .map_err(|e| ApiError::Internal(format!("failed to serialize event: {e}")))?;
        append_line(&path, &line).await?;
        Ok(())
    }

    async fn tail_events(&self, job_id: &str, tail: usize) -> Vec<AdminJobEvent> {
        let path = self.job_dir(job_id).join("events.jsonl");
        tail_lines(&path, tail)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|line| serde_json::from_str::<AdminJobEvent>(&line).ok())
            .collect()
    }

    async fn tail_log(&self, job_id: &str, stream: &str, tail: usize) -> ApiResult<Vec<String>> {
        let path = self.job_dir(job_id).join(format!("{stream}.log"));
        Ok(tail_lines(&path, tail).await.unwrap_or_default())
    }

    fn build_command(
        &self,
        kind: AdminJobKind,
        params: &AdminJobParams,
    ) -> ApiResult<BuiltCommand> {
        self.validate_params(kind, params)?;

        let edition_year = validate_edition_year(params.edition_year.unwrap_or(2025))?;
        let data_dir = self.safe_path_param("data_dir", &self.inner.config.admin_data_dir)?;
        let graph_default = join_path_display(&data_dir, "graph");
        let sources_default = join_path_display(&data_dir, "sources");
        let out_dir = self.safe_path_param(
            "out_dir",
            params.out_dir.as_deref().unwrap_or(data_dir.as_str()),
        )?;
        let graph_dir = self.safe_path_param(
            "graph_dir",
            params
                .graph_dir
                .as_deref()
                .unwrap_or(graph_default.as_str()),
        )?;
        let mut crawler_args = Vec::<String>::new();
        let mut output_paths = BTreeMap::<String, String>::new();

        match kind {
            AdminJobKind::Crawl => {
                let source_out = self.safe_path_param(
                    "out_dir",
                    params
                        .out_dir
                        .as_deref()
                        .unwrap_or(sources_default.as_str()),
                )?;
                crawler_args.push("source-ingest".to_string());
                push_arg(&mut crawler_args, "--source-id", "or_leg_ors_html");
                push_arg(&mut crawler_args, "--out", &source_out);
                push_arg(
                    &mut crawler_args,
                    "--edition-year",
                    edition_year.to_string(),
                );
                if let Some(max) = params.max_chapters {
                    push_arg(&mut crawler_args, "--max-items", max.to_string());
                }
                if let Some(chapters) = params.chapters.as_deref().filter(|v| !v.trim().is_empty())
                {
                    push_arg(&mut crawler_args, "--chapters", chapters);
                }
                self.push_source_ingest_controls(&mut crawler_args, params)?;
                output_paths.insert("sources_dir".to_string(), source_out.to_string());
            }
            AdminJobKind::Parse => {
                let chapters = params.chapters.as_deref().ok_or_else(|| {
                    ApiError::BadRequest("parse jobs require params.chapters".to_string())
                })?;
                let raw_dir = join_path_display(&data_dir, "raw/official");
                crawler_args.push("import-ors-cache".to_string());
                push_arg(&mut crawler_args, "--raw-dir", raw_dir);
                push_arg(&mut crawler_args, "--out", &out_dir);
                push_arg(&mut crawler_args, "--chapters", chapters);
                push_arg(
                    &mut crawler_args,
                    "--edition-year",
                    edition_year.to_string(),
                );
                output_paths.insert("out_dir".to_string(), out_dir.to_string());
                output_paths.insert("graph_dir".to_string(), format!("{out_dir}/graph"));
            }
            AdminJobKind::Qc => {
                let qc_default = join_path_display(&data_dir, "admin/qc");
                let qc_out = self.safe_path_param(
                    "out_dir",
                    params.out_dir.as_deref().unwrap_or(qc_default.as_str()),
                )?;
                crawler_args.push("qc-full".to_string());
                push_arg(&mut crawler_args, "--graph-dir", &graph_dir);
                push_arg(&mut crawler_args, "--out", &qc_out);
                push_arg(
                    &mut crawler_args,
                    "--edition-year",
                    edition_year.to_string(),
                );
                output_paths.insert("report_dir".to_string(), qc_out.to_string());
            }
            AdminJobKind::SeedNeo4j => {
                crawler_args.push("seed-neo4j".to_string());
                push_arg(&mut crawler_args, "--graph-dir", &graph_dir);
                self.push_neo4j_args(&mut crawler_args);
                push_arg(
                    &mut crawler_args,
                    "--edition-year",
                    edition_year.to_string(),
                );
                if params.dry_run.unwrap_or(false) {
                    crawler_args.push("--dry-run".to_string());
                }
                if params.embed.unwrap_or(false) {
                    crawler_args.push("--embed".to_string());
                }
                if params.create_vector_indexes.unwrap_or(false) {
                    crawler_args.push("--create-vector-index".to_string());
                }
                output_paths.insert("graph_dir".to_string(), graph_dir.to_string());
            }
            AdminJobKind::MaterializeNeo4j => {
                crawler_args.push("materialize-neo4j".to_string());
                push_arg(&mut crawler_args, "--graph-dir", &graph_dir);
                self.push_neo4j_args(&mut crawler_args);
                push_arg(
                    &mut crawler_args,
                    "--edition-year",
                    edition_year.to_string(),
                );
                output_paths.insert("graph_dir".to_string(), graph_dir.to_string());
            }
            AdminJobKind::EmbedNeo4j => {
                crawler_args.push("embed-neo4j".to_string());
                self.push_neo4j_args(&mut crawler_args);
                push_arg(
                    &mut crawler_args,
                    "--edition-year",
                    edition_year.to_string(),
                );
                if params.create_vector_indexes.unwrap_or(false) {
                    crawler_args.push("--create-vector-indexes".to_string());
                }
            }
            AdminJobKind::SourceIngest => {
                let source_out = self.safe_path_param(
                    "out_dir",
                    params
                        .out_dir
                        .as_deref()
                        .unwrap_or(sources_default.as_str()),
                )?;
                crawler_args.push("source-ingest".to_string());
                push_arg(&mut crawler_args, "--out", &source_out);
                push_arg(
                    &mut crawler_args,
                    "--edition-year",
                    edition_year.to_string(),
                );
                if let Some(source_id) = params
                    .source_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    push_arg(&mut crawler_args, "--source-id", source_id);
                }
                if let Some(priority) = params
                    .priority
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    push_arg(&mut crawler_args, "--priority", priority);
                }
                if let Some(max_items) = params.max_chapters {
                    push_arg(&mut crawler_args, "--max-items", max_items.to_string());
                }
                if let Some(chapters) = params.chapters.as_deref().filter(|v| !v.trim().is_empty())
                {
                    push_arg(&mut crawler_args, "--chapters", chapters);
                }
                if let Some(session_key) = params
                    .session_key
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    push_arg(&mut crawler_args, "--session-key", session_key);
                }
                self.push_source_ingest_controls(&mut crawler_args, params)?;
                output_paths.insert("sources_dir".to_string(), source_out.to_string());
            }
            AdminJobKind::CombineGraph => {
                let sources_dir = self.safe_path_param(
                    "out_dir",
                    params
                        .out_dir
                        .as_deref()
                        .unwrap_or(sources_default.as_str()),
                )?;
                crawler_args.push("combine-graph".to_string());
                push_arg(&mut crawler_args, "--sources-dir", &sources_dir);
                push_arg(&mut crawler_args, "--out", &graph_dir);
                if let Some(source_id) = params
                    .source_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    push_arg(&mut crawler_args, "--source-id", source_id);
                }
                if let Some(priority) = params
                    .priority
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    push_arg(&mut crawler_args, "--priority", priority);
                }
                output_paths.insert("sources_dir".to_string(), sources_dir.to_string());
                output_paths.insert("graph_dir".to_string(), graph_dir.to_string());
            }
        }

        let (program, mut args, mut display) = self.crawler_program();
        args.extend(crawler_args.clone());
        display.extend(crawler_args);
        Ok(BuiltCommand {
            program,
            args,
            display,
            output_paths,
        })
    }

    fn crawler_program(&self) -> (String, Vec<String>, Vec<String>) {
        let configured = self.inner.config.admin_crawler_bin.trim();
        if configured == "cargo" {
            let args = vec![
                "run".to_string(),
                "-p".to_string(),
                "ors-crawler-v0".to_string(),
                "--bin".to_string(),
                "ors-crawler-v0".to_string(),
                "--".to_string(),
            ];
            let mut display = vec!["cargo".to_string()];
            display.extend(args.clone());
            ("cargo".to_string(), args, display)
        } else {
            (
                configured.to_string(),
                Vec::new(),
                vec![configured.to_string()],
            )
        }
    }

    fn push_neo4j_args(&self, args: &mut Vec<String>) {
        push_arg(args, "--neo4j-uri", &self.inner.config.neo4j_uri);
        push_arg(args, "--neo4j-user", &self.inner.config.neo4j_user);
        push_arg(args, "--neo4j-password-env", "NEO4J_PASSWORD");
    }

    fn push_source_ingest_controls(
        &self,
        args: &mut Vec<String>,
        params: &AdminJobParams,
    ) -> ApiResult<()> {
        if let Some(mode) = params
            .mode
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            push_arg(args, "--mode", validate_ingest_mode(mode)?);
        }
        if params.refresh.unwrap_or(false) {
            args.push("--refresh".to_string());
        }
        if let Some(allow_network) = params.allow_network {
            push_arg(args, "--allow-network", allow_network.to_string());
        }
        Ok(())
    }

    fn validate_params(&self, kind: AdminJobKind, params: &AdminJobParams) -> ApiResult<()> {
        if !matches!(
            kind,
            AdminJobKind::SourceIngest | AdminJobKind::CombineGraph
        ) && (params.source_id.is_some() || params.priority.is_some())
        {
            return Err(ApiError::BadRequest(
                "source_id and priority are reserved for connector-backed source jobs".to_string(),
            ));
        }
        if let Some(year) = params.edition_year {
            validate_edition_year(year)?;
        }
        if let Some(max) = params.max_chapters {
            if max > 524 {
                return Err(ApiError::BadRequest(
                    "max_chapters must be between 0 and 524".to_string(),
                ));
            }
        }
        if let Some(chapters) = params.chapters.as_deref() {
            validate_chapters(chapters)?;
        }
        if let Some(session_key) = params.session_key.as_deref() {
            validate_session_key(session_key)?;
        }
        if let Some(mode) = params.mode.as_deref() {
            validate_ingest_mode(mode)?;
        }
        if !matches!(kind, AdminJobKind::Crawl | AdminJobKind::SourceIngest) {
            reject_param(params.mode.is_some(), "mode", kind)?;
            reject_param(params.refresh.is_some(), "refresh", kind)?;
            reject_param(params.allow_network.is_some(), "allow_network", kind)?;
        }
        if !matches!(kind, AdminJobKind::SourceIngest) && params.session_key.is_some() {
            return Err(ApiError::BadRequest(
                "session_key is reserved for connector-backed source_ingest jobs".to_string(),
            ));
        }
        if let Some(path) = params.out_dir.as_deref() {
            self.safe_path_param("out_dir", path)?;
        }
        if let Some(path) = params.graph_dir.as_deref() {
            self.safe_path_param("graph_dir", path)?;
        }

        match kind {
            AdminJobKind::Crawl => {
                reject_param(params.graph_dir.is_some(), "graph_dir", kind)?;
                reject_param(params.fetch_only.is_some(), "fetch_only", kind)?;
                reject_param(
                    params.skip_citation_resolution.is_some(),
                    "skip_citation_resolution",
                    kind,
                )?;
                reject_param(params.dry_run.is_some(), "dry_run", kind)?;
                reject_param(params.embed.is_some(), "embed", kind)?;
                reject_param(
                    params.create_vector_indexes.is_some(),
                    "create_vector_indexes",
                    kind,
                )?;
            }
            AdminJobKind::Parse => {
                reject_param(params.graph_dir.is_some(), "graph_dir", kind)?;
                reject_param(params.max_chapters.is_some(), "max_chapters", kind)?;
                reject_param(params.fetch_only.is_some(), "fetch_only", kind)?;
                reject_param(
                    params.skip_citation_resolution.is_some(),
                    "skip_citation_resolution",
                    kind,
                )?;
                reject_param(params.dry_run.is_some(), "dry_run", kind)?;
                reject_param(params.embed.is_some(), "embed", kind)?;
                reject_param(
                    params.create_vector_indexes.is_some(),
                    "create_vector_indexes",
                    kind,
                )?;
            }
            AdminJobKind::Qc => {
                reject_param(params.max_chapters.is_some(), "max_chapters", kind)?;
                reject_param(params.chapters.is_some(), "chapters", kind)?;
                reject_param(params.fetch_only.is_some(), "fetch_only", kind)?;
                reject_param(
                    params.skip_citation_resolution.is_some(),
                    "skip_citation_resolution",
                    kind,
                )?;
                reject_param(params.dry_run.is_some(), "dry_run", kind)?;
                reject_param(params.embed.is_some(), "embed", kind)?;
                reject_param(
                    params.create_vector_indexes.is_some(),
                    "create_vector_indexes",
                    kind,
                )?;
            }
            AdminJobKind::SeedNeo4j => {
                reject_param(params.out_dir.is_some(), "out_dir", kind)?;
                reject_param(params.max_chapters.is_some(), "max_chapters", kind)?;
                reject_param(params.chapters.is_some(), "chapters", kind)?;
                reject_param(params.fetch_only.is_some(), "fetch_only", kind)?;
                reject_param(
                    params.skip_citation_resolution.is_some(),
                    "skip_citation_resolution",
                    kind,
                )?;
            }
            AdminJobKind::MaterializeNeo4j => {
                reject_param(params.out_dir.is_some(), "out_dir", kind)?;
                reject_param(params.max_chapters.is_some(), "max_chapters", kind)?;
                reject_param(params.chapters.is_some(), "chapters", kind)?;
                reject_param(params.fetch_only.is_some(), "fetch_only", kind)?;
                reject_param(
                    params.skip_citation_resolution.is_some(),
                    "skip_citation_resolution",
                    kind,
                )?;
                reject_param(params.dry_run.is_some(), "dry_run", kind)?;
                reject_param(params.embed.is_some(), "embed", kind)?;
                reject_param(
                    params.create_vector_indexes.is_some(),
                    "create_vector_indexes",
                    kind,
                )?;
            }
            AdminJobKind::EmbedNeo4j => {
                reject_param(params.out_dir.is_some(), "out_dir", kind)?;
                reject_param(params.graph_dir.is_some(), "graph_dir", kind)?;
                reject_param(params.max_chapters.is_some(), "max_chapters", kind)?;
                reject_param(params.chapters.is_some(), "chapters", kind)?;
                reject_param(params.fetch_only.is_some(), "fetch_only", kind)?;
                reject_param(
                    params.skip_citation_resolution.is_some(),
                    "skip_citation_resolution",
                    kind,
                )?;
                reject_param(params.dry_run.is_some(), "dry_run", kind)?;
                reject_param(params.embed.is_some(), "embed", kind)?;
            }
            AdminJobKind::SourceIngest => {
                reject_param(params.graph_dir.is_some(), "graph_dir", kind)?;
                reject_param(params.fetch_only.is_some(), "fetch_only", kind)?;
                reject_param(
                    params.skip_citation_resolution.is_some(),
                    "skip_citation_resolution",
                    kind,
                )?;
                reject_param(params.dry_run.is_some(), "dry_run", kind)?;
                reject_param(params.embed.is_some(), "embed", kind)?;
                reject_param(
                    params.create_vector_indexes.is_some(),
                    "create_vector_indexes",
                    kind,
                )?;
                if params.source_id.is_none() && params.priority.is_none() {
                    return Err(ApiError::BadRequest(
                        "source_ingest jobs require source_id or priority".to_string(),
                    ));
                }
            }
            AdminJobKind::CombineGraph => {
                reject_param(params.max_chapters.is_some(), "max_chapters", kind)?;
                reject_param(params.chapters.is_some(), "chapters", kind)?;
                reject_param(params.fetch_only.is_some(), "fetch_only", kind)?;
                reject_param(
                    params.skip_citation_resolution.is_some(),
                    "skip_citation_resolution",
                    kind,
                )?;
                reject_param(params.dry_run.is_some(), "dry_run", kind)?;
                reject_param(params.embed.is_some(), "embed", kind)?;
                reject_param(
                    params.create_vector_indexes.is_some(),
                    "create_vector_indexes",
                    kind,
                )?;
            }
        }
        Ok(())
    }

    fn safe_path_param(&self, field: &str, value: &str) -> ApiResult<String> {
        let value = value.trim();
        if value.is_empty() {
            return Err(ApiError::BadRequest(format!("{field} must not be empty")));
        }
        if value.contains('\0') || value.starts_with('~') {
            return Err(ApiError::BadRequest(format!(
                "{field} contains an unsafe path"
            )));
        }
        let path = Path::new(value);
        if path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
        {
            return Err(ApiError::BadRequest(format!("{field} cannot contain ..")));
        }
        let resolved = self.resolve_admin_path(path);
        let data_root = self.resolve_admin_path(Path::new(&self.inner.config.admin_data_dir));
        if !resolved.starts_with(&data_root) {
            return Err(ApiError::BadRequest(format!(
                "{field} must stay under {}",
                self.inner.config.admin_data_dir
            )));
        }
        Ok(value.to_string())
    }

    fn resolve_admin_path(&self, path: &Path) -> PathBuf {
        let workdir = Path::new(&self.inner.config.admin_workdir);
        let base = if workdir.is_absolute() {
            workdir.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(workdir)
        };
        normalize_path(&base.join(path))
    }
}

fn push_arg(args: &mut Vec<String>, name: &str, value: impl ToString) {
    args.push(name.to_string());
    args.push(value.to_string());
}

fn reject_param(present: bool, field: &str, kind: AdminJobKind) -> ApiResult<()> {
    if present {
        Err(ApiError::BadRequest(format!(
            "{field} is not accepted for {} jobs",
            kind.as_str()
        )))
    } else {
        Ok(())
    }
}

fn validate_edition_year(year: i32) -> ApiResult<i32> {
    if (1850..=2100).contains(&year) {
        Ok(year)
    } else {
        Err(ApiError::BadRequest(
            "edition_year must be between 1850 and 2100".to_string(),
        ))
    }
}

fn validate_chapters(chapters: &str) -> ApiResult<()> {
    let chapters = chapters.trim();
    if chapters.is_empty() || chapters.len() > 512 {
        return Err(ApiError::BadRequest(
            "chapters must be a non-empty comma/range list under 512 characters".to_string(),
        ));
    }
    if chapters
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, ',' | '-' | ' ' | '\t'))
    {
        Ok(())
    } else {
        Err(ApiError::BadRequest(
            "chapters may contain only digits, commas, ranges, and spaces".to_string(),
        ))
    }
}

fn validate_ingest_mode(mode: &str) -> ApiResult<&str> {
    let mode = mode.trim();
    match mode {
        "discover" | "fetch" | "parse" | "qc" | "all" => Ok(mode),
        _ => Err(ApiError::BadRequest(
            "mode must be one of discover, fetch, parse, qc, or all".to_string(),
        )),
    }
}

fn validate_session_key(session_key: &str) -> ApiResult<()> {
    let session_key = session_key.trim();
    if session_key.is_empty() || session_key.len() > 32 {
        return Err(ApiError::BadRequest(
            "session_key must be a non-empty value under 32 characters".to_string(),
        ));
    }
    if session_key
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        Ok(())
    } else {
        Err(ApiError::BadRequest(
            "session_key may contain only letters, numbers, dashes, and underscores".to_string(),
        ))
    }
}

fn join_path_display(base: &str, child: &str) -> String {
    let base = base.trim_end_matches('/');
    if base.is_empty() {
        child.to_string()
    } else {
        format!("{base}/{child}")
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
        }
    }
    normalized
}

fn job_finished_or_started_at(job: &AdminJob) -> Option<u128> {
    job.finished_at_ms
        .or(job.started_at_ms)
        .or(Some(job.created_at_ms))
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

async fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> ApiResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| {
            ApiError::Internal(format!("failed to create {}: {e}", parent.display()))
        })?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| ApiError::Internal(format!("failed to serialize json: {e}")))?;
    fs::write(path, bytes)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to write {}: {e}", path.display())))
}

async fn append_line(path: &Path, line: &str) -> ApiResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| {
            ApiError::Internal(format!("failed to create {}: {e}", parent.display()))
        })?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to open {}: {e}", path.display())))?;
    file.write_all(line.as_bytes())
        .await
        .map_err(|e| ApiError::Internal(format!("failed to write {}: {e}", path.display())))?;
    file.write_all(b"\n")
        .await
        .map_err(|e| ApiError::Internal(format!("failed to write {}: {e}", path.display())))?;
    Ok(())
}

async fn tail_lines(path: &Path, tail: usize) -> std::io::Result<Vec<String>> {
    let mut file = match fs::File::open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let len = file.metadata().await?.len();
    let wanted_bytes = ((tail.max(1) as u64) * 240).clamp(16 * 1024, 512 * 1024);
    let read_len = len.min(wanted_bytes);
    file.seek(SeekFrom::Start(len.saturating_sub(read_len)))
        .await?;
    let mut bytes = vec![0; read_len as usize];
    file.read_exact(&mut bytes).await?;
    let text = String::from_utf8_lossy(&bytes);
    let lines = text
        .lines()
        .map(str::to_string)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let start = lines.len().saturating_sub(tail);
    Ok(lines[start..].to_vec())
}

async fn summarize_sources(data_dir: &Path) -> AdminSourceSummary {
    let registry_sources = read_source_registry()
        .await
        .map(|sources| sources.len())
        .unwrap_or(0);
    let (source_dirs, source_artifacts, source_bytes) =
        summarize_dir(&data_dir.join("sources")).await;
    AdminSourceSummary {
        registry_sources,
        source_dirs,
        source_artifacts,
        source_bytes,
    }
}

#[derive(Debug, Deserialize)]
struct SourceRegistryFile {
    sources: Vec<SourceRegistryRawEntry>,
}

#[derive(Debug, Deserialize)]
struct SourceRegistryRawEntry {
    source_id: String,
    name: String,
    owner: String,
    jurisdiction: String,
    source_type: String,
    access: String,
    official_status: String,
    connector_status: String,
    priority: String,
    source_url: String,
    docs_url: String,
    #[serde(default)]
    graph_nodes_created: Vec<String>,
    #[serde(default)]
    graph_edges_created: Vec<String>,
}

async fn read_source_registry() -> ApiResult<Vec<AdminSourceRegistryEntry>> {
    let (text, location) = read_source_registry_text().await?;
    let registry: SourceRegistryFile = serde_json::from_str(&text)
        .map_err(|e| ApiError::Internal(format!("failed to parse {location}: {e}")))?;
    Ok(registry
        .sources
        .into_iter()
        .map(|source| AdminSourceRegistryEntry {
            source_id: source.source_id,
            name: source.name,
            owner: source.owner,
            jurisdiction: source.jurisdiction,
            source_type: source.source_type,
            access: source.access,
            official_status: source.official_status,
            connector_status: source.connector_status,
            priority: source.priority,
            source_url: source.source_url,
            docs_url: source.docs_url,
            graph_nodes_created: source.graph_nodes_created,
            graph_edges_created: source.graph_edges_created,
            local: AdminSourceLocalStatus::default(),
        })
        .collect())
}

async fn read_source_registry_text() -> ApiResult<(String, String)> {
    read_source_registry_text_from(source_registry_path_candidates()).await
}

async fn read_source_registry_text_from(candidates: Vec<PathBuf>) -> ApiResult<(String, String)> {
    for candidate in candidates {
        match fs::read_to_string(&candidate).await {
            Ok(text) => return Ok((text, candidate.display().to_string())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(ApiError::Internal(format!(
                    "failed to read {}: {error}",
                    candidate.display()
                )));
            }
        }
    }

    Ok((
        SOURCE_REGISTRY_EMBEDDED.to_string(),
        format!("embedded {SOURCE_REGISTRY_RELATIVE_PATH}"),
    ))
}

fn filter_source_registry_entries(
    sources: &mut Vec<AdminSourceRegistryEntry>,
    priority: Option<&str>,
    connector_status: Option<&str>,
) {
    if let Some(priority) = priority.filter(|value| !value.trim().is_empty()) {
        sources.retain(|source| source.priority.eq_ignore_ascii_case(priority.trim()));
    }
    if let Some(status) = connector_status.filter(|value| !value.trim().is_empty()) {
        sources.retain(|source| source.connector_status.eq_ignore_ascii_case(status.trim()));
    }
}

fn source_registry_path_candidates() -> Vec<PathBuf> {
    source_registry_path_candidates_from(
        std::env::var_os("ORS_SOURCE_REGISTRY_PATH").map(PathBuf::from),
        std::env::current_exe().ok(),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
}

fn source_registry_path_candidates_from(
    configured_path: Option<PathBuf>,
    current_exe: Option<PathBuf>,
    manifest_dir: &Path,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(path) = configured_path {
        push_unique_path(&mut candidates, path);
    }

    push_unique_path(
        &mut candidates,
        PathBuf::from(SOURCE_REGISTRY_RELATIVE_PATH),
    );

    if let Some(exe_dir) = current_exe.as_deref().and_then(Path::parent) {
        push_unique_path(&mut candidates, exe_dir.join(SOURCE_REGISTRY_RELATIVE_PATH));
    }

    push_unique_path(
        &mut candidates,
        manifest_dir.join(SOURCE_REGISTRY_RELATIVE_PATH),
    );
    push_unique_path(
        &mut candidates,
        manifest_dir
            .join("../..")
            .join(SOURCE_REGISTRY_RELATIVE_PATH),
    );

    candidates
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

async fn hydrate_source_local_status(
    sources: Vec<AdminSourceRegistryEntry>,
    data_dir: &Path,
) -> Vec<AdminSourceRegistryEntry> {
    let mut tasks = JoinSet::new();
    let data_dir = data_dir.to_path_buf();
    let gate = Arc::new(Semaphore::new(SOURCE_HYDRATE_CONCURRENCY));
    let source_count = sources.len();

    for (index, source) in sources.into_iter().enumerate() {
        let data_dir = data_dir.clone();
        let gate = Arc::clone(&gate);
        tasks.spawn(async move {
            let _permit = gate.acquire_owned().await.ok();
            (index, hydrate_one_source(source, &data_dir).await)
        });
    }

    let mut hydrated = vec![None; source_count];
    while let Some(result) = tasks.join_next().await {
        if let Ok((index, source)) = result {
            hydrated[index] = Some(source);
        }
    }
    hydrated.into_iter().flatten().collect()
}

async fn hydrate_one_source(
    mut source: AdminSourceRegistryEntry,
    data_dir: &Path,
) -> AdminSourceRegistryEntry {
    let source_dir = data_dir.join("sources").join(&source.source_id);
    let graph = summarize_graph(&source_dir.join("graph")).await;
    let (_dirs, artifacts, bytes) = summarize_dir(&source_dir).await;
    let source_dir_exists = fs::try_exists(&source_dir).await.unwrap_or(false);
    let qc_status = read_json_value(&source_dir.join("qc/report.json"))
        .await
        .and_then(|value| {
            value
                .get("status")
                .and_then(|status| status.as_str())
                .map(ToOwned::to_owned)
        });
    let last_finished_at = read_json_value(&source_dir.join("stats.json"))
        .await
        .and_then(|value| {
            value
                .get("finished_at")
                .and_then(|finished_at| finished_at.as_str())
                .map(ToOwned::to_owned)
        });
    source.local = AdminSourceLocalStatus {
        source_dir_exists,
        source_artifacts: artifacts,
        source_bytes: bytes,
        graph_files: graph.jsonl_files,
        graph_rows: graph.rows,
        qc_status,
        last_finished_at,
    };
    source
}

async fn list_source_graph_files(path: &Path) -> Vec<AdminSourceGraphFile> {
    let mut files = Vec::new();
    let Ok(mut entries) = fs::read_dir(path).await else {
        return files;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let entry_path = entry.path();
        if entry_path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let file = entry_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown.jsonl")
            .to_string();
        let bytes = entry
            .metadata()
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let rows = count_non_empty_lines(&entry_path).await.unwrap_or(0);
        files.push(AdminSourceGraphFile { file, rows, bytes });
    }
    files.sort_by(|left, right| left.file.cmp(&right.file));
    files
}

async fn list_source_artifacts(path: &Path) -> Vec<AdminSourceArtifact> {
    let mut artifacts = Vec::new();
    let Ok(mut entries) = fs::read_dir(path).await else {
        return artifacts;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let entry_path = entry.path();
        if !entry_path.is_file()
            || entry_path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.ends_with(".metadata.json"))
        {
            continue;
        }
        let file = entry_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("artifact")
            .to_string();
        let bytes = entry
            .metadata()
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let sidecar = entry_path.with_file_name(format!(
            "{}.metadata.json",
            entry_path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("artifact")
        ));
        let metadata = read_json_value(&sidecar).await;
        artifacts.push(AdminSourceArtifact {
            file,
            bytes,
            content_type: metadata
                .as_ref()
                .and_then(|value| value.get("content_type"))
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned),
            raw_hash: metadata
                .as_ref()
                .and_then(|value| value.get("raw_hash"))
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned),
            status: metadata
                .as_ref()
                .and_then(|value| value.get("status"))
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned),
            skipped: metadata
                .as_ref()
                .and_then(|value| value.get("skipped"))
                .and_then(|value| value.as_bool()),
        });
    }
    artifacts.sort_by(|left, right| left.file.cmp(&right.file));
    artifacts
}

async fn read_json_value(path: &Path) -> Option<serde_json::Value> {
    let bytes = fs::read(path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

async fn read_corpus_release_id(path: &Path) -> Option<String> {
    read_json_value(path)
        .await
        .and_then(|value| {
            value
                .get("release_id")
                .and_then(|id| id.as_str())
                .map(str::to_string)
        })
        .filter(|value| !value.trim().is_empty())
}

fn bytes_to_gb(bytes: u64) -> f64 {
    ((bytes as f64 / 1_073_741_824.0) * 1000.0).round() / 1000.0
}

async fn summarize_graph(path: &Path) -> AdminGraphSummary {
    summarize_graph_inner(path, true).await
}

async fn summarize_graph_fast(path: &Path) -> AdminGraphSummary {
    summarize_graph_inner(path, false).await
}

async fn summarize_graph_inner(path: &Path, count_rows: bool) -> AdminGraphSummary {
    let mut summary = AdminGraphSummary::default();
    summary.rows_are_exact = count_rows;
    let Ok(mut entries) = fs::read_dir(path).await else {
        return summary;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let entry_path = entry.path();
        if entry_path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        summary.jsonl_files += 1;
        if let Ok(metadata) = entry.metadata().await {
            summary.bytes += metadata.len();
        }
        if count_rows {
            summary.rows += count_non_empty_lines(&entry_path).await.unwrap_or(0);
        }
    }
    summary
}

async fn summarize_dir(path: &Path) -> (usize, usize, u64) {
    let mut dirs = 0usize;
    let mut files = 0usize;
    let mut bytes = 0u64;
    let mut stack = vec![path.to_path_buf()];
    let mut visited = 0usize;
    while let Some(dir) = stack.pop() {
        visited += 1;
        if visited > 10_000 {
            break;
        }
        let Ok(mut entries) = fs::read_dir(&dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let Ok(file_type) = entry.file_type().await else {
                continue;
            };
            if file_type.is_dir() {
                dirs += 1;
                stack.push(entry.path());
            } else if file_type.is_file() {
                files += 1;
                if let Ok(metadata) = entry.metadata().await {
                    bytes += metadata.len();
                }
            }
        }
    }
    (dirs, files, bytes)
}

async fn count_non_empty_lines(path: &Path) -> std::io::Result<usize> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(path)?;
        let mut buffer = vec![0u8; 1024 * 1024];
        let mut count = 0usize;
        let mut line_has_content = false;

        loop {
            let read = std::io::Read::read(&mut file, &mut buffer)?;
            if read == 0 {
                break;
            }
            for byte in &buffer[..read] {
                if *byte == b'\n' {
                    if line_has_content {
                        count += 1;
                    }
                    line_has_content = false;
                } else if !byte.is_ascii_whitespace() {
                    line_has_content = true;
                }
            }
        }

        if line_has_content {
            count += 1;
        }

        Ok(count)
    })
    .await
    .map_err(|error| std::io::Error::other(format!("line counter task failed: {error}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    fn test_config() -> Arc<ApiConfig> {
        test_config_with_jobs_dir("data/admin/test-jobs")
    }

    fn test_config_with_jobs_dir(jobs_dir: &str) -> Arc<ApiConfig> {
        Arc::new(ApiConfig {
            api_host: "127.0.0.1".to_string(),
            api_port: 8080,
            neo4j_uri: "bolt://localhost:7687".to_string(),
            neo4j_user: "neo4j".to_string(),
            neo4j_password: "secret".to_string(),
            api_key: None,
            auth_enabled: false,
            auth_issuer: None,
            auth_audience: None,
            auth_admin_role: "orsgraph_admin".to_string(),
            casebuilder_storage_dir: "data/casebuilder/uploads".to_string(),
            storage_backend: "local".to_string(),
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
            corpus_release_manifest_path: "data/graph/corpus_release.json".to_string(),
            authority_cache_ttl_seconds: 86_400,
            authority_cache_max_capacity: 20_000,
            query_embedding_cache_ttl_seconds: 604_800,
            query_embedding_cache_max_capacity: 50_000,
            rerank_policy: "low_confidence".to_string(),
            authority_edge_base_url: None,
            admin_enabled: true,
            admin_allow_kill: false,
            admin_jobs_dir: jobs_dir.to_string(),
            admin_workdir: ".".to_string(),
            admin_crawler_bin: "cargo".to_string(),
            admin_data_dir: "data".to_string(),
        })
    }

    fn test_service(config: Arc<ApiConfig>) -> AdminService {
        let jobs_dir = PathBuf::from(&config.admin_jobs_dir);
        AdminService {
            inner: Arc::new(AdminInner {
                config,
                jobs_dir,
                jobs: RwLock::new(HashMap::new()),
                running: RwLock::new(HashMap::new()),
                counter: AtomicU64::new(0),
            }),
        }
    }

    fn test_config_with_bin(jobs_dir: &Path, bin: &Path) -> Arc<ApiConfig> {
        let mut config = (*test_config_with_jobs_dir(&jobs_dir.display().to_string())).clone();
        config.admin_crawler_bin = bin.display().to_string();
        Arc::new(config)
    }

    async fn wait_for_terminal(service: &AdminService, job_id: &str) -> AdminJob {
        for _ in 0..100 {
            let job = service.get_job(job_id).await.unwrap();
            if job.status.is_terminal() {
                return job;
            }
            sleep(Duration::from_millis(25)).await;
        }
        service.get_job(job_id).await.unwrap()
    }

    #[test]
    fn source_registry_candidates_cover_runtime_container_path() {
        let configured = PathBuf::from("/custom/source-registry.yaml");
        let candidates = source_registry_path_candidates_from(
            Some(configured.clone()),
            Some(PathBuf::from("/app/orsgraph-api")),
            Path::new("/build/crates/orsgraph-api"),
        );

        assert_eq!(candidates.first(), Some(&configured));
        assert!(candidates.contains(&PathBuf::from(SOURCE_REGISTRY_RELATIVE_PATH)));
        assert!(candidates.contains(&PathBuf::from("/app/docs/data/source-registry.yaml")));
        assert!(candidates.contains(&PathBuf::from(
            "/build/crates/orsgraph-api/docs/data/source-registry.yaml"
        )));
        assert!(candidates.contains(&PathBuf::from(
            "/build/crates/orsgraph-api/../../docs/data/source-registry.yaml"
        )));
    }

    #[tokio::test]
    async fn source_registry_reader_falls_back_to_embedded_registry() {
        let missing =
            std::env::temp_dir().join(format!("orsgraph-missing-source-registry-{}", now_ms()));
        let (text, location) = read_source_registry_text_from(vec![missing]).await.unwrap();
        let registry: SourceRegistryFile = serde_json::from_str(&text).unwrap();

        assert_eq!(
            location,
            format!("embedded {SOURCE_REGISTRY_RELATIVE_PATH}")
        );
        assert!(
            registry
                .sources
                .iter()
                .any(|source| source.source_id == "or_leg_ors_html")
        );
    }

    #[test]
    fn builds_registry_backed_crawl_command() {
        let service = test_service(test_config());
        let command = service
            .build_command(
                AdminJobKind::Crawl,
                &AdminJobParams {
                    max_chapters: Some(2),
                    mode: Some("fetch".to_string()),
                    refresh: Some(true),
                    allow_network: Some(false),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(command.program, "cargo");
        assert!(command.args.contains(&"source-ingest".to_string()));
        assert!(command.args.contains(&"or_leg_ors_html".to_string()));
        assert!(command.args.contains(&"--max-items".to_string()));
        assert!(command.args.contains(&"--mode".to_string()));
        assert!(command.args.contains(&"fetch".to_string()));
        assert!(command.args.contains(&"--refresh".to_string()));
        assert!(command.args.contains(&"--allow-network".to_string()));
        assert!(command.args.contains(&"false".to_string()));
        assert!(!command.display.iter().any(|arg| arg == "secret"));
    }

    #[test]
    fn builds_source_ingest_command() {
        let service = test_service(test_config());
        let command = service
            .build_command(
                AdminJobKind::SourceIngest,
                &AdminJobParams {
                    priority: Some("P0".to_string()),
                    mode: Some("qc".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(command.args.contains(&"source-ingest".to_string()));
        assert!(command.args.contains(&"--priority".to_string()));
        assert!(command.args.contains(&"P0".to_string()));
        assert!(command.args.contains(&"--mode".to_string()));
        assert!(command.args.contains(&"qc".to_string()));
        assert!(command.output_paths.contains_key("sources_dir"));
    }

    #[test]
    fn source_ingest_requires_selection() {
        let service = test_service(test_config());
        let err = service
            .build_command(AdminJobKind::SourceIngest, &AdminJobParams::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("source_id or priority"));
    }

    #[tokio::test]
    async fn lists_registry_sources_for_admin_api() {
        let service = test_service(test_config());
        let response = service
            .list_sources(Some("P0".to_string()), None)
            .await
            .unwrap();

        assert_eq!(response.totals.p0_sources, response.sources.len());
        assert!(
            response
                .sources
                .iter()
                .any(|source| source.source_id == "or_leg_ors_html")
        );
        assert!(
            response
                .sources
                .iter()
                .all(|source| source.priority == "P0")
        );
    }

    #[tokio::test]
    async fn gets_registry_source_detail_for_admin_api() {
        let service = test_service(test_config());
        let detail = service.get_source("or_leg_ors_html").await.unwrap();

        assert_eq!(detail.source.source_id, "or_leg_ors_html");
        assert_eq!(detail.source.priority, "P0");
        assert!(detail.stats.is_none());
        assert!(detail.qc_report.is_none());
        assert!(detail.graph_files.is_empty());
        assert!(detail.raw_artifacts.is_empty());
    }

    #[tokio::test]
    async fn fast_graph_summary_defers_expensive_row_counts() {
        let root = std::env::temp_dir().join(format!("orsgraph-admin-graph-{}", now_ms()));
        fs::create_dir_all(&root).await.unwrap();
        fs::write(root.join("nodes.jsonl"), "{\"id\":1}\n{\"id\":2}\n")
            .await
            .unwrap();

        let fast = summarize_graph_fast(&root).await;
        assert_eq!(fast.jsonl_files, 1);
        assert_eq!(fast.rows, 0);
        assert!(!fast.rows_are_exact);
        assert!(fast.bytes > 0);

        let exact = summarize_graph(&root).await;
        assert_eq!(exact.jsonl_files, 1);
        assert_eq!(exact.rows, 2);
        assert!(exact.rows_are_exact);

        let _ = fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn crawler_summary_exposes_runtime_control_state() {
        let service = test_service(test_config());
        let running_job = AdminJob {
            job_id: "job-running".to_string(),
            kind: AdminJobKind::SourceIngest,
            status: AdminJobStatus::Running,
            params: AdminJobParams {
                source_id: Some("or_leg_ors_html".to_string()),
                ..Default::default()
            },
            command: vec!["cargo".to_string(), "run".to_string()],
            command_display: "cargo run -p ors-crawler-v0".to_string(),
            is_read_only: false,
            created_at_ms: now_ms(),
            started_at_ms: Some(now_ms()),
            finished_at_ms: None,
            exit_code: None,
            message: None,
            output_paths: BTreeMap::new(),
            progress: Default::default(),
        };
        let succeeded_job = AdminJob {
            job_id: "job-succeeded".to_string(),
            kind: AdminJobKind::Qc,
            status: AdminJobStatus::Succeeded,
            params: AdminJobParams::default(),
            command: vec!["cargo".to_string(), "run".to_string()],
            command_display: "cargo run -p ors-crawler-v0".to_string(),
            is_read_only: true,
            created_at_ms: now_ms().saturating_sub(1000),
            started_at_ms: Some(now_ms().saturating_sub(900)),
            finished_at_ms: Some(now_ms().saturating_sub(800)),
            exit_code: Some(0),
            message: Some("job completed".to_string()),
            output_paths: BTreeMap::new(),
            progress: Default::default(),
        };

        {
            let mut jobs = service.inner.jobs.write().await;
            jobs.insert(running_job.job_id.clone(), running_job.clone());
            jobs.insert(succeeded_job.job_id.clone(), succeeded_job);
        }
        service
            .inner
            .running
            .write()
            .await
            .insert(running_job.job_id, RunningJob { pid: Some(4242) });

        let jobs = service.sorted_jobs().await;
        let summary = service.crawler_summary(&jobs).await;

        assert_eq!(summary.configured_bin, "cargo");
        assert_eq!(summary.control_mode, "cargo-run");
        assert_eq!(summary.running_jobs, 1);
        assert_eq!(summary.mutating_running_jobs, 1);
        assert!(summary.active_mutating_job);
        assert_eq!(summary.active_pid, Some(4242));
        assert_eq!(
            summary.last_terminal_status,
            Some(AdminJobStatus::Succeeded)
        );
        assert!(
            summary
                .command_prefix
                .contains(&"ors-crawler-v0".to_string())
        );
        assert!(summary.last_success_at_ms.is_some());
    }

    #[test]
    fn admin_gate_rejects_when_disabled() {
        let mut config = (*test_config()).clone();
        config.admin_enabled = false;
        let service = test_service(Arc::new(config));

        let err = service.require_enabled().unwrap_err().to_string();
        assert!(err.contains("disabled"));
    }

    #[test]
    fn start_request_rejects_unknown_params() {
        let err = serde_json::from_value::<AdminStartJobRequest>(serde_json::json!({
            "kind": "crawl",
            "params": {
                "max_chapters": 1,
                "shell": "rm -rf /"
            }
        }))
        .unwrap_err()
        .to_string();

        assert!(err.contains("unknown field"));
    }

    #[test]
    fn rejects_invalid_paths_and_irrelevant_params() {
        let service = test_service(test_config());
        let err = service
            .build_command(
                AdminJobKind::Crawl,
                &AdminJobParams {
                    out_dir: Some("../outside".to_string()),
                    ..Default::default()
                },
            )
            .unwrap_err()
            .to_string();
        assert!(err.contains("cannot contain"));

        let err = service
            .build_command(
                AdminJobKind::Crawl,
                &AdminJobParams {
                    graph_dir: Some("data/graph".to_string()),
                    ..Default::default()
                },
            )
            .unwrap_err()
            .to_string();
        assert!(err.contains("not accepted"));
    }

    #[tokio::test]
    async fn starts_and_finishes_short_mock_job() {
        let jobs_dir = std::env::temp_dir().join(format!("orsgraph-admin-echo-{}", now_ms()));
        let config = test_config_with_bin(&jobs_dir, Path::new("/bin/echo"));
        let service = AdminService::new(config).await.unwrap();
        let detail = service
            .start_job(AdminStartJobRequest {
                kind: AdminJobKind::Crawl,
                params: AdminJobParams {
                    max_chapters: Some(1),
                    ..Default::default()
                },
            })
            .await
            .unwrap();

        let job = wait_for_terminal(&service, &detail.job.job_id).await;
        assert_eq!(job.status, AdminJobStatus::Succeeded);
        let mut logs = Vec::new();
        for _ in 0..20 {
            logs = service
                .get_logs(&job.job_id, "stdout", 10)
                .await
                .unwrap()
                .lines;
            if logs.iter().any(|line| line.contains("source-ingest")) {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
        assert!(logs.iter().any(|line| line.contains("source-ingest")));

        let _ = fs::remove_dir_all(jobs_dir).await;
    }

    #[tokio::test]
    async fn cancels_running_mock_job() {
        let root = std::env::temp_dir().join(format!("orsgraph-admin-cancel-{}", now_ms()));
        let jobs_dir = root.join("jobs");
        let script = root.join("mock-crawler.sh");
        fs::create_dir_all(&root).await.unwrap();
        fs::write(
            &script,
            "#!/bin/sh\ntrap 'exit 143' TERM\necho 'Phase mock-start'\nwhile true; do sleep 1; done\n",
        )
        .await
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).unwrap();
        }

        let config = test_config_with_bin(&jobs_dir, &script);
        let service = AdminService::new(config).await.unwrap();
        let detail = service
            .start_job(AdminStartJobRequest {
                kind: AdminJobKind::Crawl,
                params: AdminJobParams::default(),
            })
            .await
            .unwrap();
        sleep(Duration::from_millis(100)).await;
        service.cancel_job(&detail.job.job_id).await.unwrap();

        let job = wait_for_terminal(&service, &detail.job.job_id).await;
        assert_eq!(job.status, AdminJobStatus::Cancelled);

        let _ = fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn recovers_running_jobs_as_failed_from_disk() {
        let jobs_dir = std::env::temp_dir().join(format!("orsgraph-admin-test-{}", now_ms()));
        let job_id = "job-recover-1";
        let job_dir = jobs_dir.join(job_id);
        fs::create_dir_all(&job_dir).await.unwrap();

        let job = AdminJob {
            job_id: job_id.to_string(),
            kind: AdminJobKind::Crawl,
            status: AdminJobStatus::Running,
            params: AdminJobParams::default(),
            command: vec!["cargo".to_string(), "run".to_string()],
            command_display: "cargo run".to_string(),
            is_read_only: false,
            created_at_ms: now_ms(),
            started_at_ms: Some(now_ms()),
            finished_at_ms: None,
            exit_code: None,
            message: None,
            output_paths: BTreeMap::new(),
            progress: Default::default(),
        };
        fs::write(
            job_dir.join("job.json"),
            serde_json::to_vec_pretty(&job).unwrap(),
        )
        .await
        .unwrap();

        let service = AdminService::new(test_config_with_jobs_dir(&jobs_dir.display().to_string()))
            .await
            .unwrap();
        let recovered = service.get_job(job_id).await.unwrap();
        assert_eq!(recovered.status, AdminJobStatus::Failed);
        assert!(
            recovered
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("API restarted")
        );

        let _ = fs::remove_dir_all(jobs_dir).await;
    }

    #[tokio::test]
    async fn disabled_admin_does_not_create_job_storage() {
        let root = std::env::temp_dir().join(format!("orsgraph-admin-disabled-{}", now_ms()));
        let jobs_dir = root.join("jobs");
        let mut config = (*test_config_with_jobs_dir(&jobs_dir.display().to_string())).clone();
        config.admin_enabled = false;

        let service = AdminService::new(Arc::new(config)).await.unwrap();

        assert!(!service.enabled());
        assert!(!fs::try_exists(&jobs_dir).await.unwrap());

        let _ = fs::remove_dir_all(root).await;
    }
}
