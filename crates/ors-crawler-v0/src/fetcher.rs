use crate::artifact_store::RawArtifact;
use crate::connectors::SourceItem;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use url::Url;

const MAX_ODATA_PAGES: usize = 250;

#[derive(Debug, Clone)]
pub struct FetchOutcome {
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct CacheValidators {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

impl CacheValidators {
    pub fn is_empty(&self) -> bool {
        self.etag.as_deref().is_none_or(str::is_empty)
            && self.last_modified.as_deref().is_none_or(str::is_empty)
    }
}

#[derive(Debug, Clone)]
pub enum FetchResult {
    Fetched(FetchOutcome),
    NotModified,
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
    match fetch_item_with_cache_validation(client, source_id, item, policy, fixture_dir, None)
        .await?
    {
        FetchResult::Fetched(outcome) => Ok(outcome),
        FetchResult::NotModified => Err(anyhow!(
            "{}:{} returned 304 without a cached artifact",
            source_id,
            item.item_id
        )),
    }
}

pub async fn fetch_item_with_cache_validation(
    client: &Client,
    source_id: &str,
    item: &SourceItem,
    policy: &FetchPolicy,
    fixture_dir: Option<PathBuf>,
    validators: Option<&CacheValidators>,
) -> Result<FetchResult> {
    if let Some(fixture_dir) = fixture_dir {
        if let Some(bytes) = read_fixture(&fixture_dir, source_id, item)? {
            return Ok(FetchResult::Fetched(FetchOutcome {
                content_type: item.content_type.clone(),
                etag: None,
                last_modified: None,
                bytes,
            }));
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
        let mut request = client.get(url);
        if let Some(validators) = validators.filter(|validators| !validators.is_empty()) {
            if let Some(etag) = validators.etag.as_deref().filter(|value| !value.is_empty()) {
                request = request.header(reqwest::header::IF_NONE_MATCH, etag);
            }
            if let Some(last_modified) = validators
                .last_modified
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                request = request.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
            }
        }
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                if status == reqwest::StatusCode::NOT_MODIFIED {
                    return Ok(FetchResult::NotModified);
                } else if !status.is_success() {
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
                    let mut bytes = response.bytes().await?.to_vec();
                    if source_id == "or_leg_odata" {
                        bytes = follow_odata_pages(client, url, bytes, policy).await?;
                    }
                    return Ok(FetchResult::Fetched(FetchOutcome {
                        content_type,
                        etag,
                        last_modified,
                        bytes,
                    }));
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

async fn follow_odata_pages(
    client: &Client,
    first_url: &str,
    first_page: Vec<u8>,
    policy: &FetchPolicy,
) -> Result<Vec<u8>> {
    let mut combined: Value = match serde_json::from_slice(&first_page) {
        Ok(value) => value,
        Err(_) => return Ok(first_page),
    };
    let Some(row_shape) = ODataRowShape::detect(&combined) else {
        return Ok(first_page);
    };
    let mut next_url =
        match odata_next_link(&combined).map(|link| resolve_next_url(first_url, link)) {
            Some(url) => Some(url),
            None => return Ok(first_page),
        };

    let mut page_count = 1usize;
    while let Some(url) = next_url.take() {
        page_count += 1;
        if page_count > MAX_ODATA_PAGES {
            return Err(anyhow!(
                "OData paging exceeded {MAX_ODATA_PAGES} pages while fetching {first_url}"
            ));
        }

        let page_bytes = fetch_odata_page(client, &url, policy).await?;
        let page: Value = serde_json::from_slice(&page_bytes)
            .with_context(|| format!("failed to parse OData page {url}"))?;
        append_odata_rows(&mut combined, &page, row_shape)?;
        next_url = odata_next_link(&page).map(|link| resolve_next_url(&url, link));
    }
    clear_odata_next_link(&mut combined);
    Ok(serde_json::to_vec(&combined)?)
}

async fn fetch_odata_page(client: &Client, url: &str, policy: &FetchPolicy) -> Result<Vec<u8>> {
    if policy.delay_ms > 0 {
        sleep(Duration::from_millis(policy.delay_ms)).await;
    }

    let mut last_err = anyhow!("no OData page fetch attempts made");
    for attempt in 1..=policy.max_attempts.max(1) {
        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    last_err = anyhow!("GET {url} failed with status {status}");
                } else {
                    return Ok(response.bytes().await?.to_vec());
                }
            }
            Err(error) => last_err = error.into(),
        }
        if attempt < policy.max_attempts {
            sleep(Duration::from_millis(500 * 2u64.pow(attempt - 1))).await;
        }
    }
    Err(last_err.context(format!("all OData page fetch attempts failed for {url}")))
}

#[derive(Debug, Clone, Copy)]
enum ODataRowShape {
    DResults,
    Value,
    Results,
}

impl ODataRowShape {
    fn detect(value: &Value) -> Option<Self> {
        if value
            .pointer("/d/results")
            .and_then(Value::as_array)
            .is_some()
        {
            Some(Self::DResults)
        } else if value.pointer("/value").and_then(Value::as_array).is_some() {
            Some(Self::Value)
        } else if value
            .pointer("/results")
            .and_then(Value::as_array)
            .is_some()
        {
            Some(Self::Results)
        } else {
            None
        }
    }
}

fn append_odata_rows(combined: &mut Value, page: &Value, shape: ODataRowShape) -> Result<()> {
    let page_rows = rows_for_shape(page, shape)
        .ok_or_else(|| anyhow!("OData next page did not match the first page row shape"))?;
    let combined_rows = rows_for_shape_mut(combined, shape)
        .ok_or_else(|| anyhow!("combined OData page lost its row array"))?;
    combined_rows.extend(page_rows.iter().cloned());
    Ok(())
}

fn rows_for_shape(value: &Value, shape: ODataRowShape) -> Option<&Vec<Value>> {
    match shape {
        ODataRowShape::DResults => value.pointer("/d/results")?.as_array(),
        ODataRowShape::Value => value.pointer("/value")?.as_array(),
        ODataRowShape::Results => value.pointer("/results")?.as_array(),
    }
}

fn rows_for_shape_mut(value: &mut Value, shape: ODataRowShape) -> Option<&mut Vec<Value>> {
    match shape {
        ODataRowShape::DResults => value.pointer_mut("/d/results")?.as_array_mut(),
        ODataRowShape::Value => value.pointer_mut("/value")?.as_array_mut(),
        ODataRowShape::Results => value.pointer_mut("/results")?.as_array_mut(),
    }
}

fn odata_next_link(value: &Value) -> Option<&str> {
    value
        .pointer("/d/__next")
        .or_else(|| value.pointer("/odata.nextLink"))
        .or_else(|| value.pointer("/@odata.nextLink"))
        .and_then(Value::as_str)
}

fn clear_odata_next_link(value: &mut Value) {
    if let Some(object) = value.as_object_mut() {
        object.remove("odata.nextLink");
        object.remove("@odata.nextLink");
    }
    if let Some(object) = value.pointer_mut("/d").and_then(Value::as_object_mut) {
        object.remove("__next");
    }
}

fn resolve_next_url(current_url: &str, next_link: &str) -> String {
    Url::parse(next_link)
        .or_else(|_| Url::parse(current_url).and_then(|base| base.join(next_link)))
        .map(|url| url.to_string())
        .unwrap_or_else(|_| next_link.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::SourceItem;
    use std::collections::BTreeMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn conditional_fetch_sends_validators_and_accepts_304() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0u8; 4096];
            let bytes_read = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..bytes_read]).to_ascii_lowercase();
            assert!(request.contains("if-none-match: \"abc\""));
            assert!(request.contains("if-modified-since: fri, 01 may 2026 00:00:00 gmt"));
            socket
                .write_all(b"HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
        });

        let policy = FetchPolicy {
            delay_ms: 0,
            ..Default::default()
        };
        let client = client(&policy).unwrap();
        let item = SourceItem {
            item_id: "item-1".to_string(),
            url: Some(format!("http://{addr}/item-1")),
            title: None,
            content_type: None,
            metadata: BTreeMap::new(),
        };
        let validators = CacheValidators {
            etag: Some("\"abc\"".to_string()),
            last_modified: Some("Fri, 01 May 2026 00:00:00 GMT".to_string()),
        };

        let result = fetch_item_with_cache_validation(
            &client,
            "test_source",
            &item,
            &policy,
            None,
            Some(&validators),
        )
        .await
        .unwrap();
        assert!(matches!(result, FetchResult::NotModified));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn oregon_leg_odata_fetch_follows_next_links() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buffer = vec![0u8; 4096];
                let bytes_read = socket.read(&mut buffer).await.unwrap();
                let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                let body = if request.starts_with("GET /first ") {
                    format!(r#"{{"d":{{"results":[{{"id":1}}],"__next":"http://{addr}/second"}}}}"#)
                } else {
                    r#"{"d":{"results":[{"id":2}]}}"#.to_string()
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });

        let policy = FetchPolicy {
            delay_ms: 0,
            max_attempts: 1,
            ..Default::default()
        };
        let client = client(&policy).unwrap();
        let item = SourceItem {
            item_id: "Measures_2025R1".to_string(),
            url: Some(format!("http://{addr}/first")),
            title: None,
            content_type: Some("application/json".to_string()),
            metadata: BTreeMap::new(),
        };

        let result =
            fetch_item_with_cache_validation(&client, "or_leg_odata", &item, &policy, None, None)
                .await
                .unwrap();
        let FetchResult::Fetched(outcome) = result else {
            panic!("expected fetched OData outcome");
        };
        let value: Value = serde_json::from_slice(&outcome.bytes).unwrap();
        let rows = value.pointer("/d/results").unwrap().as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert!(value.pointer("/d/__next").is_none());
        server.await.unwrap();
    }
}
