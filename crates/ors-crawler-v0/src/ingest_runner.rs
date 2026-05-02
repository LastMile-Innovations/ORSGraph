use crate::artifact_store::{ArtifactMetadata, ArtifactStore, RawArtifact};
use crate::connectors::{ConnectorOptions, DataConnector, SourceItem, connector_for};
use crate::corpus_release::{CorpusReleaseEmbedding, write_corpus_release_manifest};
use crate::embedding_profiles::LEGAL_CHUNK_PRIMARY;
use crate::fetcher::{
    CacheValidators, FetchOutcome, FetchPolicy, FetchResult, client,
    fetch_item_with_cache_validation,
};
use crate::graph_batch::GraphBatch;
use crate::source_registry::{
    SourcePriority, SourceRegistry, SourceRegistryEntry, by_id, load_default_registry,
    load_registry, validate_registry, write_canonical_registry_json,
};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IngestMode {
    Discover,
    Fetch,
    Parse,
    Qc,
    All,
}

impl IngestMode {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "discover" => Ok(Self::Discover),
            "fetch" => Ok(Self::Fetch),
            "parse" => Ok(Self::Parse),
            "qc" => Ok(Self::Qc),
            "all" => Ok(Self::All),
            other => Err(anyhow!("unsupported ingest mode: {other}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceIngestOptions {
    pub registry_path: Option<PathBuf>,
    pub out: PathBuf,
    pub source_id: Option<String>,
    pub priority: Option<SourcePriority>,
    pub mode: IngestMode,
    pub fixture_dir: Option<PathBuf>,
    pub fetch_policy: FetchPolicy,
    pub edition_year: i32,
    pub chapters: Option<String>,
    pub session_key: Option<String>,
    pub max_items: usize,
    pub fail_on_qc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRun {
    pub source_id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub mode: IngestMode,
    pub discovered_items: usize,
    pub artifacts: Vec<ArtifactMetadata>,
    pub graph_files: usize,
    pub graph_rows: usize,
    pub qc_status: String,
}

pub fn validate_source_registry(path: Option<PathBuf>, write_yaml: bool) -> Result<()> {
    let registry = if let Some(path) = path {
        load_registry(path)?
    } else {
        load_default_registry()?
    };
    let report = validate_registry(&registry);
    if write_yaml {
        write_canonical_registry_json("docs/data/source-registry.yaml", &registry)?;
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.is_valid() {
        Ok(())
    } else {
        Err(anyhow!("source registry validation failed"))
    }
}

pub async fn run_source_ingest(options: SourceIngestOptions) -> Result<Vec<IngestRun>> {
    let registry = load_for_options(options.registry_path.as_deref())?;
    let report = validate_registry(&registry);
    if !report.is_valid() {
        return Err(anyhow!(
            "source registry validation failed: {}",
            report.errors.join("; ")
        ));
    }
    let selected = select_sources(&registry, options.source_id.as_deref(), options.priority)?;
    if selected.is_empty() {
        return Err(anyhow!("no sources matched source-ingest selection"));
    }

    let store = ArtifactStore::new(&options.out);
    let client = client(&options.fetch_policy)?;
    let mut runs = Vec::new();
    for entry in selected {
        let started_at = Utc::now();
        let connector: Arc<dyn DataConnector> = Arc::from(connector_for(
            entry.clone(),
            ConnectorOptions {
                edition_year: options.edition_year,
                chapters: options.chapters.clone(),
                session_key: options.session_key.clone(),
                max_items: options.max_items,
            },
        ));
        let mut items = connector.discover().await?;
        if options.max_items > 0 && items.len() > options.max_items {
            items.truncate(options.max_items);
        }
        store.ensure_source_dirs(&entry.source_id)?;
        store.write_json(&entry.source_id, "manifest.json", &entry)?;

        if options.mode == IngestMode::Discover {
            store.write_json(
                &entry.source_id,
                "stats.json",
                &serde_json::json!({
                    "source_id": entry.source_id,
                    "discovered_items": items.len(),
                    "mode": "discover"
                }),
            )?;
            runs.push(IngestRun {
                source_id: entry.source_id,
                started_at,
                finished_at: Utc::now(),
                mode: options.mode,
                discovered_items: items.len(),
                artifacts: Vec::new(),
                graph_files: 0,
                graph_rows: 0,
                qc_status: "not_run".to_string(),
            });
            continue;
        }

        let should_parse = matches!(
            options.mode,
            IngestMode::Parse | IngestMode::Qc | IngestMode::All
        );
        let mut results = stream::iter(items.iter().cloned().enumerate())
            .map(|(index, item)| {
                let entry = entry.clone();
                let store = store.clone();
                let client = client.clone();
                let connector = connector.clone();
                let policy = options.fetch_policy.clone();
                let fixture_dir = options.fixture_dir.clone();
                async move {
                    process_item(
                        index,
                        &entry,
                        &store,
                        &client,
                        connector.as_ref(),
                        item,
                        &policy,
                        fixture_dir,
                        should_parse,
                    )
                    .await
                }
            })
            .buffer_unordered(options.fetch_policy.concurrency.max(1))
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        results.sort_by_key(|result| result.index);

        let mut artifacts = Vec::new();
        let mut batch = GraphBatch::default();
        for result in results {
            artifacts.push(result.artifact);
            merge_batch(&mut batch, result.batch);
        }

        if matches!(
            options.mode,
            IngestMode::Parse | IngestMode::Qc | IngestMode::All
        ) {
            batch.write_to_dir(store.graph_dir(&entry.source_id))?;
        }
        let qc = if matches!(options.mode, IngestMode::Qc | IngestMode::All) {
            connector.qc(&artifacts, &batch).await?
        } else {
            crate::source_qc::QcReport {
                source_id: entry.source_id.clone(),
                status: crate::source_qc::QcReportStatus::Warning,
                artifacts: artifacts.len(),
                graph_files: batch.files.len(),
                graph_rows: batch.row_count(),
                warnings: vec!["QC not requested for this ingest mode".to_string()],
                errors: Vec::new(),
            }
        };
        store.write_json(&entry.source_id, "qc/report.json", &qc)?;
        if options.fail_on_qc && qc.is_failure() {
            return Err(anyhow!("{} QC failed: {:?}", entry.source_id, qc.errors));
        }

        let run = IngestRun {
            source_id: entry.source_id.clone(),
            started_at,
            finished_at: Utc::now(),
            mode: options.mode,
            discovered_items: items.len(),
            artifacts,
            graph_files: batch.files.len(),
            graph_rows: batch.row_count(),
            qc_status: format!("{:?}", qc.status).to_ascii_lowercase(),
        };
        store.write_json(&entry.source_id, "stats.json", &run)?;
        runs.push(run);
    }
    Ok(runs)
}

struct ItemIngestResult {
    index: usize,
    artifact: ArtifactMetadata,
    batch: GraphBatch,
}

#[allow(clippy::too_many_arguments)]
async fn process_item(
    index: usize,
    entry: &SourceRegistryEntry,
    store: &ArtifactStore,
    client: &Client,
    connector: &dyn DataConnector,
    item: SourceItem,
    policy: &FetchPolicy,
    fixture_dir: Option<PathBuf>,
    should_parse: bool,
) -> Result<ItemIngestResult> {
    let cached = if policy.use_cache {
        store.read_cached(&entry.source_id, &item)?
    } else {
        None
    };
    let artifact = if item.url.is_none() {
        if let Some(cached) = cached {
            cached
        } else {
            let outcome = FetchOutcome {
                content_type: item.content_type.clone(),
                etag: None,
                last_modified: None,
                bytes: serde_json::to_vec_pretty(entry)
                    .context("failed to serialize registry-only artifact")?,
            };
            store.write_raw(
                &entry.source_id,
                &item.item_id,
                &entry.source_url,
                outcome.content_type,
                outcome.bytes,
                outcome.etag,
                outcome.last_modified,
                "generated",
            )?
        }
    } else if let Some(cached) = cached {
        let validators = validators_for_cached_artifact(&cached);
        if policy.allow_network && validators.as_ref().is_some_and(|value| !value.is_empty()) {
            match fetch_item_with_cache_validation(
                client,
                &entry.source_id,
                &item,
                policy,
                fixture_dir,
                validators.as_ref(),
            )
            .await?
            {
                FetchResult::Fetched(outcome) => store.write_raw(
                    &entry.source_id,
                    &item.item_id,
                    item.url.as_deref().unwrap_or(&entry.source_url),
                    outcome.content_type,
                    outcome.bytes,
                    outcome.etag,
                    outcome.last_modified,
                    "fetched",
                )?,
                FetchResult::NotModified => {
                    let mut cached = cached;
                    cached.metadata.status = "not_modified".to_string();
                    cached.metadata.skipped = true;
                    cached
                }
            }
        } else {
            cached
        }
    } else {
        let outcome = if item.url.is_none() {
            FetchOutcome {
                content_type: item.content_type.clone(),
                etag: None,
                last_modified: None,
                bytes: serde_json::to_vec_pretty(entry)
                    .context("failed to serialize registry-only artifact")?,
            }
        } else {
            match fetch_item_with_cache_validation(
                client,
                &entry.source_id,
                &item,
                policy,
                fixture_dir,
                None,
            )
            .await?
            {
                FetchResult::Fetched(outcome) => outcome,
                FetchResult::NotModified => {
                    return Err(anyhow!(
                        "{}:{} returned 304 without a cached artifact",
                        entry.source_id,
                        item.item_id
                    ));
                }
            }
        };
        store.write_raw(
            &entry.source_id,
            &item.item_id,
            item.url.as_deref().unwrap_or(&entry.source_url),
            outcome.content_type,
            outcome.bytes,
            outcome.etag,
            outcome.last_modified,
            if item.url.is_some() {
                "fetched"
            } else {
                "generated"
            },
        )?
    };

    let batch = if should_parse {
        connector.parse(&artifact).await?
    } else {
        GraphBatch::default()
    };

    Ok(ItemIngestResult {
        index,
        artifact: artifact.metadata,
        batch,
    })
}

fn validators_for_cached_artifact(artifact: &RawArtifact) -> Option<CacheValidators> {
    let validators = CacheValidators {
        etag: artifact.metadata.etag.clone(),
        last_modified: artifact.metadata.last_modified.clone(),
    };
    if validators.is_empty() {
        None
    } else {
        Some(validators)
    }
}

pub fn combine_graph(
    registry_path: Option<PathBuf>,
    sources_dir: PathBuf,
    out: PathBuf,
    source_id: Option<String>,
    priority: Option<SourcePriority>,
) -> Result<usize> {
    let registry = load_for_options(registry_path.as_deref())?;
    let selected = select_sources(&registry, source_id.as_deref(), priority)?;
    let selected_ids = selected
        .into_iter()
        .map(|source| source.source_id)
        .collect::<BTreeSet<_>>();
    fs::create_dir_all(&out)?;

    let mut total_rows = 0usize;
    let mut by_file = std::collections::BTreeMap::<String, BTreeSet<String>>::new();
    for source_id in &selected_ids {
        let graph_dir = sources_dir.join(source_id.as_str()).join("graph");
        if !graph_dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&graph_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| anyhow!("invalid graph file name {}", path.display()))?
                .to_string();
            let lines = fs::read_to_string(&path)?;
            by_file.entry(file_name.clone()).or_default();
            for line in lines.lines().map(str::trim).filter(|line| !line.is_empty()) {
                by_file
                    .entry(file_name.clone())
                    .or_default()
                    .insert(line.to_string());
            }
        }
    }
    for (file_name, rows) in by_file {
        total_rows += rows.len();
        let file = fs::File::create(out.join(file_name))?;
        let mut writer = std::io::BufWriter::new(file);
        for row in rows {
            writer.write_all(row.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
    }
    write_corpus_release_manifest(
        &out,
        &sources_dir,
        &selected_ids,
        CorpusReleaseEmbedding::from_profile(&LEGAL_CHUNK_PRIMARY),
    )?;
    Ok(total_rows)
}

fn load_for_options(path: Option<&Path>) -> Result<SourceRegistry> {
    if let Some(path) = path {
        load_registry(path)
    } else {
        load_default_registry()
    }
}

fn select_sources(
    registry: &SourceRegistry,
    source_id: Option<&str>,
    priority: Option<SourcePriority>,
) -> Result<Vec<SourceRegistryEntry>> {
    if let Some(source_id) = source_id {
        let by_id = by_id(registry);
        return by_id
            .get(source_id)
            .cloned()
            .map(|source| vec![source])
            .ok_or_else(|| anyhow!("unknown source_id {source_id}"));
    }
    if let Some(priority) = priority {
        return Ok(registry
            .sources
            .iter()
            .filter(|source| source.priority == priority)
            .cloned()
            .collect());
    }
    Err(anyhow!("source-ingest requires --source-id or --priority"))
}

fn merge_batch(target: &mut GraphBatch, next: GraphBatch) {
    for (file_name, rows) in next.files {
        target.files.entry(file_name).or_default().extend(rows);
    }
}
