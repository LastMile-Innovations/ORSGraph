use crate::config::ApiConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CorpusReleaseService {
    manifest: Arc<CorpusReleaseManifest>,
    manifest_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorpusReleaseManifest {
    #[serde(default)]
    pub schema_version: String,
    #[serde(default)]
    pub release_id: String,
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub source_count: usize,
    #[serde(default)]
    pub graph_file_count: usize,
    #[serde(default)]
    pub graph_rows: usize,
    #[serde(default)]
    pub graph_bytes: u64,
    #[serde(default)]
    pub graph_hash: Option<String>,
    #[serde(default)]
    pub embedding: Option<CorpusReleaseEmbedding>,
    #[serde(default)]
    pub graph_files: Vec<CorpusReleaseGraphFile>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub manifest_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorpusReleaseEmbedding {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub dimension: usize,
    #[serde(default)]
    pub input_hash_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorpusReleaseGraphFile {
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub rows: usize,
    #[serde(default)]
    pub bytes: u64,
}

impl CorpusReleaseService {
    pub fn from_config(config: &ApiConfig) -> Self {
        Self::from_path(PathBuf::from(&config.corpus_release_manifest_path))
    }

    pub fn from_path(manifest_path: PathBuf) -> Self {
        let manifest = load_manifest(&manifest_path);
        Self {
            manifest: Arc::new(manifest),
            manifest_path,
        }
    }

    pub fn release_id(&self) -> &str {
        &self.manifest.release_id
    }

    pub fn manifest(&self) -> CorpusReleaseManifest {
        (*self.manifest).clone()
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn cache_key(&self, namespace: &str, raw: impl AsRef<[u8]>) -> String {
        format!(
            "{}:{}:{}",
            self.release_id(),
            namespace,
            sha256_hex(raw.as_ref())
        )
    }
}

fn load_manifest(path: &Path) -> CorpusReleaseManifest {
    match std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<CorpusReleaseManifest>(&bytes).ok())
    {
        Some(mut manifest) => {
            if manifest.release_id.trim().is_empty() {
                manifest.release_id = fallback_release_id(path);
                manifest.warnings.push(
                    "manifest missing release_id; synthesized fallback release id".to_string(),
                );
            }
            if manifest.manifest_status.trim().is_empty() {
                manifest.manifest_status = "loaded".to_string();
            }
            manifest
        }
        None => CorpusReleaseManifest {
            schema_version: "orsgraph.corpus_release.v1".to_string(),
            release_id: fallback_release_id(path),
            generated_at: None,
            source_count: 0,
            graph_file_count: 0,
            graph_rows: 0,
            graph_bytes: 0,
            graph_hash: None,
            embedding: None,
            graph_files: Vec::new(),
            warnings: vec![format!(
                "corpus release manifest not found or unreadable at {}",
                path.display()
            )],
            manifest_status: "missing".to_string(),
        },
    }
}

fn fallback_release_id(path: &Path) -> String {
    format!(
        "release:unversioned:{}",
        &sha256_hex(path.display().to_string().as_bytes())[..12]
    )
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
