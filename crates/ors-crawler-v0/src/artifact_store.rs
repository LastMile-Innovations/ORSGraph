use crate::connectors::SourceItem;
use crate::hash::{sha256_hex, stable_id};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub artifact_id: String,
    pub source_id: String,
    pub item_id: String,
    pub url: String,
    pub path: String,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub retrieved_at: DateTime<Utc>,
    pub raw_hash: String,
    pub byte_len: usize,
    pub status: String,
    pub skipped: bool,
}

#[derive(Debug, Clone)]
pub struct RawArtifact {
    pub metadata: ArtifactMetadata,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn source_dir(&self, source_id: &str) -> PathBuf {
        self.root.join(source_id)
    }

    pub fn graph_dir(&self, source_id: &str) -> PathBuf {
        self.source_dir(source_id).join("graph")
    }

    pub fn qc_dir(&self, source_id: &str) -> PathBuf {
        self.source_dir(source_id).join("qc")
    }

    pub fn raw_dir(&self, source_id: &str) -> PathBuf {
        self.source_dir(source_id).join("raw")
    }

    pub fn normalized_dir(&self, source_id: &str) -> PathBuf {
        self.source_dir(source_id).join("normalized")
    }

    pub fn ensure_source_dirs(&self, source_id: &str) -> Result<()> {
        fs::create_dir_all(self.raw_dir(source_id))?;
        fs::create_dir_all(self.normalized_dir(source_id))?;
        fs::create_dir_all(self.graph_dir(source_id))?;
        fs::create_dir_all(self.qc_dir(source_id))?;
        Ok(())
    }

    pub fn write_raw(
        &self,
        source_id: &str,
        item_id: &str,
        url: &str,
        content_type: Option<String>,
        bytes: Vec<u8>,
        etag: Option<String>,
        last_modified: Option<String>,
        status: impl Into<String>,
    ) -> Result<RawArtifact> {
        self.ensure_source_dirs(source_id)?;
        let raw_hash = sha256_hex(&bytes);
        let ext = extension_for(content_type.as_deref(), url);
        let artifact_id = format!(
            "artifact:{}",
            stable_id(&format!("{source_id}:{item_id}:{raw_hash}"))
        );
        let file_name = format!("{}.{ext}", safe_file_stem(item_id));
        let path = self.raw_dir(source_id).join(file_name);
        fs::write(&path, &bytes)?;
        let metadata = ArtifactMetadata {
            artifact_id,
            source_id: source_id.to_string(),
            item_id: item_id.to_string(),
            url: url.to_string(),
            path: path.display().to_string(),
            content_type,
            etag,
            last_modified,
            retrieved_at: Utc::now(),
            raw_hash,
            byte_len: bytes.len(),
            status: status.into(),
            skipped: false,
        };
        self.write_artifact_metadata(source_id, item_id, &metadata)?;
        Ok(RawArtifact { metadata, bytes })
    }

    pub fn read_cached(&self, source_id: &str, item: &SourceItem) -> Result<Option<RawArtifact>> {
        let sidecar_path = self.artifact_metadata_path(source_id, &item.item_id);
        if sidecar_path.exists() {
            let metadata: ArtifactMetadata =
                serde_json::from_slice(&fs::read(&sidecar_path).with_context(|| {
                    format!(
                        "failed to read artifact metadata {}",
                        sidecar_path.display()
                    )
                })?)
                .with_context(|| {
                    format!(
                        "failed to parse artifact metadata {}",
                        sidecar_path.display()
                    )
                })?;
            let bytes = fs::read(&metadata.path)
                .with_context(|| format!("failed to read cached artifact {}", metadata.path))?;
            let mut metadata = metadata;
            metadata.status = "cached".to_string();
            metadata.skipped = true;
            return Ok(Some(RawArtifact { metadata, bytes }));
        }

        for ext in ["json", "html", "txt", "pdf", "xml"] {
            let path =
                self.raw_dir(source_id)
                    .join(format!("{}.{}", safe_file_stem(&item.item_id), ext));
            if !path.exists() {
                continue;
            }
            let bytes = fs::read(&path)
                .with_context(|| format!("failed to read cached artifact {}", path.display()))?;
            let raw_hash = sha256_hex(&bytes);
            let metadata = ArtifactMetadata {
                artifact_id: format!(
                    "artifact:{}",
                    stable_id(&format!("{source_id}:{}:{raw_hash}", item.item_id))
                ),
                source_id: source_id.to_string(),
                item_id: item.item_id.clone(),
                url: item.url.clone().unwrap_or_default(),
                path: path.display().to_string(),
                content_type: content_type_for_ext(ext).map(ToOwned::to_owned),
                etag: None,
                last_modified: None,
                retrieved_at: Utc::now(),
                raw_hash,
                byte_len: bytes.len(),
                status: "cached".to_string(),
                skipped: true,
            };
            return Ok(Some(RawArtifact { metadata, bytes }));
        }

        Ok(None)
    }

    pub fn write_json<T: Serialize>(&self, source_id: &str, name: &str, value: &T) -> Result<()> {
        self.ensure_source_dirs(source_id)?;
        let path = self.source_dir(source_id).join(name);
        fs::write(path, serde_json::to_string_pretty(value)?)?;
        Ok(())
    }

    fn write_artifact_metadata(
        &self,
        source_id: &str,
        item_id: &str,
        metadata: &ArtifactMetadata,
    ) -> Result<()> {
        let path = self.artifact_metadata_path(source_id, item_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(metadata)?)?;
        Ok(())
    }

    fn artifact_metadata_path(&self, source_id: &str, item_id: &str) -> PathBuf {
        self.raw_dir(source_id)
            .join(format!("{}.metadata.json", safe_file_stem(item_id)))
    }
}

fn extension_for(content_type: Option<&str>, url: &str) -> &'static str {
    if let Some(content_type) = content_type {
        let content_type = content_type.to_ascii_lowercase();
        if content_type.contains("json") {
            return "json";
        }
        if content_type.contains("pdf") {
            return "pdf";
        }
        if content_type.contains("html") {
            return "html";
        }
        if content_type.contains("xml") {
            return "xml";
        }
    }
    let url = url.to_ascii_lowercase();
    if url.ends_with(".pdf") {
        "pdf"
    } else if url.ends_with(".json") {
        "json"
    } else if url.ends_with(".xml") {
        "xml"
    } else {
        "html"
    }
}

fn content_type_for_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "json" => Some("application/json"),
        "html" => Some("text/html"),
        "txt" => Some("text/plain"),
        "pdf" => Some("application/pdf"),
        "xml" => Some("application/xml"),
        _ => None,
    }
}

fn safe_file_stem(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "artifact".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_file_names_are_stable() {
        assert_eq!(safe_file_stem("ORS 1.001/html"), "ORS_1.001_html");
    }

    #[test]
    fn cached_artifact_round_trips_with_metadata() {
        let root =
            std::env::temp_dir().join(format!("orsgraph-artifact-store-{}", uuid::Uuid::new_v4()));
        let store = ArtifactStore::new(&root);
        let written = store
            .write_raw(
                "test_source",
                "item:1",
                "https://example.test/item",
                Some("application/json".to_string()),
                br#"{"ok":true}"#.to_vec(),
                Some("\"abc\"".to_string()),
                Some("Fri, 01 May 2026 00:00:00 GMT".to_string()),
                "fetched",
            )
            .expect("write raw artifact");
        let cached = store
            .read_cached(
                "test_source",
                &SourceItem {
                    item_id: "item:1".to_string(),
                    url: Some("https://example.test/item".to_string()),
                    title: None,
                    content_type: None,
                    metadata: Default::default(),
                },
            )
            .expect("read cached artifact")
            .expect("cached artifact exists");
        let _ = fs::remove_dir_all(root);

        assert_eq!(cached.bytes, br#"{"ok":true}"#);
        assert_eq!(cached.metadata.raw_hash, written.metadata.raw_hash);
        assert_eq!(cached.metadata.status, "cached");
        assert!(cached.metadata.skipped);
        assert_eq!(cached.metadata.etag.as_deref(), Some("\"abc\""));
    }
}
