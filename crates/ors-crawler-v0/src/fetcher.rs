use crate::artifact_store::RawArtifact;
use crate::connectors::SourceItem;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct FetchOutcome {
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchPolicy {
    pub user_agent: String,
    pub delay_ms: u64,
    pub timeout_secs: u64,
    pub max_attempts: u32,
    pub concurrency: usize,
    pub allow_network: bool,
    pub use_cache: bool,
}

impl Default for FetchPolicy {
    fn default() -> Self {
        Self {
            user_agent: "NeighborOS-ORSGraph/0.1 registry crawler".to_string(),
            delay_ms: 500,
            timeout_secs: 45,
            max_attempts: 3,
            concurrency: 2,
            allow_network: true,
            use_cache: true,
        }
    }
}

pub fn client(policy: &FetchPolicy) -> Result<Client> {
    Ok(Client::builder()
        .user_agent(policy.user_agent.clone())
        .timeout(Duration::from_secs(policy.timeout_secs))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?)
}

pub async fn fetch_item(
    client: &Client,
    source_id: &str,
    item: &SourceItem,
    policy: &FetchPolicy,
    fixture_dir: Option<PathBuf>,
) -> Result<(Option<String>, Vec<u8>)> {
    fetch_item_with_metadata(client, source_id, item, policy, fixture_dir)
        .await
        .map(|outcome| (outcome.content_type, outcome.bytes))
}

pub async fn fetch_item_with_metadata(
    client: &Client,
    source_id: &str,
    item: &SourceItem,
    policy: &FetchPolicy,
    fixture_dir: Option<PathBuf>,
) -> Result<FetchOutcome> {
    if let Some(fixture_dir) = fixture_dir {
        if let Some(bytes) = read_fixture(&fixture_dir, source_id, item)? {
            return Ok(FetchOutcome {
                content_type: item.content_type.clone(),
                etag: None,
                last_modified: None,
                bytes,
            });
        }
    }
    if !policy.allow_network {
        return Err(anyhow!(
            "network disabled and no fixture found for {}:{}",
            source_id,
            item.item_id
        ));
    }
    if policy.delay_ms > 0 {
        sleep(Duration::from_millis(policy.delay_ms)).await;
    }
    let url = item
        .url
        .as_deref()
        .ok_or_else(|| anyhow!("{}:{} has no URL", source_id, item.item_id))?;

    let mut last_err = anyhow!("no fetch attempts made");
    for attempt in 1..=policy.max_attempts.max(1) {
        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    last_err = anyhow!("GET {url} failed with status {status}");
                } else {
                    let content_type = response
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .map(ToOwned::to_owned);
                    let etag = response
                        .headers()
                        .get(reqwest::header::ETAG)
                        .and_then(|value| value.to_str().ok())
                        .map(ToOwned::to_owned);
                    let last_modified = response
                        .headers()
                        .get(reqwest::header::LAST_MODIFIED)
                        .and_then(|value| value.to_str().ok())
                        .map(ToOwned::to_owned);
                    let bytes = response.bytes().await?.to_vec();
                    return Ok(FetchOutcome {
                        content_type,
                        etag,
                        last_modified,
                        bytes,
                    });
                }
            }
            Err(error) => last_err = error.into(),
        }
        if attempt < policy.max_attempts {
            sleep(Duration::from_millis(500 * 2u64.pow(attempt - 1))).await;
        }
    }
    Err(last_err.context(format!("all fetch attempts failed for {url}")))
}

fn read_fixture(
    fixture_dir: &std::path::Path,
    source_id: &str,
    item: &SourceItem,
) -> Result<Option<Vec<u8>>> {
    let candidates = [
        fixture_dir
            .join(source_id)
            .join(format!("{}.json", item.item_id)),
        fixture_dir
            .join(source_id)
            .join(format!("{}.html", item.item_id)),
        fixture_dir
            .join(source_id)
            .join(format!("{}.txt", item.item_id)),
        fixture_dir
            .join(source_id)
            .join(format!("{}.pdf", item.item_id)),
        fixture_dir.join(format!("{source_id}.json")),
        fixture_dir.join(format!("{source_id}.html")),
        fixture_dir.join(format!("{source_id}.txt")),
        fixture_dir.join(format!("{source_id}.pdf")),
    ];
    for path in candidates {
        if path.exists() {
            return fs::read(&path)
                .with_context(|| format!("failed to read fixture {}", path.display()))
                .map(Some);
        }
    }
    Ok(None)
}

#[allow(dead_code)]
pub fn artifact_debug(artifact: &RawArtifact) -> String {
    format!(
        "{} {} bytes {}",
        artifact.metadata.item_id, artifact.metadata.byte_len, artifact.metadata.raw_hash
    )
}
