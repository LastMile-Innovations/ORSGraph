use crate::artifact_store::ArtifactMetadata;
use crate::embedding_profiles::EmbeddingProfile;
use crate::hash::sha256_hex;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusReleaseManifest {
    pub schema_version: String,
    pub release_id: String,
    pub generated_at: DateTime<Utc>,
    pub source_count: usize,
    pub source_artifact_count: usize,
    pub graph_file_count: usize,
    pub graph_rows: usize,
    pub graph_bytes: u64,
    pub graph_hash: String,
    pub embedding: CorpusReleaseEmbedding,
    pub graph_files: Vec<CorpusReleaseGraphFile>,
    pub source_artifacts: Vec<CorpusReleaseSourceArtifact>,
    pub warnings: Vec<String>,
    pub manifest_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusReleaseEmbedding {
    pub model: String,
    pub profile: String,
    pub dimension: usize,
    pub output_dtype: String,
    pub neo4j_index_name: String,
    pub input_hash_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusReleaseGraphFile {
    pub file: String,
    pub sha256: String,
    pub rows: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusReleaseSourceArtifact {
    pub source_id: String,
    pub item_id: String,
    pub raw_hash: String,
    pub byte_len: usize,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub status: String,
}

impl CorpusReleaseEmbedding {
    pub fn from_profile(profile: &EmbeddingProfile) -> Self {
        Self {
            model: profile.model.to_string(),
            profile: profile.name.to_string(),
            dimension: profile.output_dimension.max(0) as usize,
            output_dtype: profile.output_dtype.to_string(),
            neo4j_index_name: profile.neo4j_index_name.to_string(),
            input_hash_policy: "embedding_input_hash:v1".to_string(),
        }
    }
}

pub fn write_corpus_release_manifest(
    graph_dir: &Path,
    sources_dir: &Path,
    selected_source_ids: &BTreeSet<String>,
    embedding: CorpusReleaseEmbedding,
) -> Result<CorpusReleaseManifest> {
    let (graph_files, graph_rows, graph_bytes, graph_hash) = summarize_graph_dir(graph_dir)?;
    let (source_artifacts, warnings) =
        summarize_source_artifacts(sources_dir, selected_source_ids)?;
    let source_artifact_count = source_artifacts.len();
    let seed = release_seed(&graph_files, &source_artifacts, &embedding);
    let release_hash = sha256_hex(seed.as_bytes());
    let release_hash = release_hash.trim_start_matches("sha256:");
    let manifest = CorpusReleaseManifest {
        schema_version: "orsgraph.corpus_release.v1".to_string(),
        release_id: format!("release:{}", &release_hash[..24]),
        generated_at: Utc::now(),
        source_count: selected_source_ids.len(),
        source_artifact_count,
        graph_file_count: graph_files.len(),
        graph_rows,
        graph_bytes,
        graph_hash,
        embedding,
        graph_files,
        source_artifacts,
        warnings,
        manifest_status: "generated".to_string(),
    };

    fs::write(
        graph_dir.join("corpus_release.json"),
        serde_json::to_string_pretty(&manifest)?,
    )
    .with_context(|| {
        format!(
            "failed to write corpus release manifest under {}",
            graph_dir.display()
        )
    })?;

    Ok(manifest)
}

fn summarize_graph_dir(
    graph_dir: &Path,
) -> Result<(Vec<CorpusReleaseGraphFile>, usize, u64, String)> {
    let mut paths = jsonl_paths(graph_dir)?;
    paths.sort();
    let mut graph_files = Vec::new();
    let mut total_rows = 0usize;
    let mut total_bytes = 0u64;
    let mut graph_seed = String::new();

    for path in paths {
        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read graph file {}", path.display()))?;
        let rows = bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| line.iter().any(|byte| !byte.is_ascii_whitespace()))
            .count();
        let sha256 = sha256_hex(&bytes);
        let file = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        total_rows += rows;
        total_bytes += bytes.len() as u64;
        graph_seed.push_str(&file);
        graph_seed.push(':');
        graph_seed.push_str(&sha256);
        graph_seed.push(':');
        graph_seed.push_str(&rows.to_string());
        graph_seed.push('\n');
        graph_files.push(CorpusReleaseGraphFile {
            file,
            sha256,
            rows,
            bytes: bytes.len() as u64,
        });
    }

    Ok((
        graph_files,
        total_rows,
        total_bytes,
        sha256_hex(graph_seed.as_bytes()),
    ))
}

fn summarize_source_artifacts(
    sources_dir: &Path,
    selected_source_ids: &BTreeSet<String>,
) -> Result<(Vec<CorpusReleaseSourceArtifact>, Vec<String>)> {
    let mut artifacts = Vec::new();
    let mut warnings = Vec::new();

    for source_id in selected_source_ids {
        let raw_dir = sources_dir.join(source_id).join("raw");
        if !raw_dir.exists() {
            warnings.push(format!("source {source_id} has no raw artifact directory"));
            continue;
        }
        let mut metadata_paths = metadata_paths(&raw_dir)?;
        metadata_paths.sort();
        for path in metadata_paths {
            match fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<ArtifactMetadata>(&bytes).ok())
            {
                Some(metadata) => artifacts.push(CorpusReleaseSourceArtifact {
                    source_id: metadata.source_id,
                    item_id: metadata.item_id,
                    raw_hash: metadata.raw_hash,
                    byte_len: metadata.byte_len,
                    etag: metadata.etag,
                    last_modified: metadata.last_modified,
                    status: metadata.status,
                }),
                None => warnings.push(format!(
                    "failed to parse source artifact metadata {}",
                    path.display()
                )),
            }
        }
    }

    artifacts.sort_by(|left, right| {
        left.source_id
            .cmp(&right.source_id)
            .then_with(|| left.item_id.cmp(&right.item_id))
            .then_with(|| left.raw_hash.cmp(&right.raw_hash))
    });
    Ok((artifacts, warnings))
}

fn jsonl_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(path)
        .with_context(|| format!("failed to read graph dir {}", path.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn metadata_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(path)
        .with_context(|| format!("failed to read raw artifact dir {}", path.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.ends_with(".metadata.json"))
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn release_seed(
    graph_files: &[CorpusReleaseGraphFile],
    source_artifacts: &[CorpusReleaseSourceArtifact],
    embedding: &CorpusReleaseEmbedding,
) -> String {
    let mut seed = format!(
        "embedding:{}:{}:{}:{}\n",
        embedding.model, embedding.profile, embedding.dimension, embedding.input_hash_policy
    );
    for file in graph_files {
        seed.push_str(&format!(
            "graph:{}:{}:{}\n",
            file.file, file.sha256, file.rows
        ));
    }
    for artifact in source_artifacts {
        seed.push_str(&format!(
            "source:{}:{}:{}:{}:{:?}:{:?}\n",
            artifact.source_id,
            artifact.item_id,
            artifact.raw_hash,
            artifact.byte_len,
            artifact.etag,
            artifact.last_modified
        ));
    }
    seed
}
