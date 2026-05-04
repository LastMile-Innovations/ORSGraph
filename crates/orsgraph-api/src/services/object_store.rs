use crate::config::ApiConfig;
use crate::error::{ApiError, ApiResult};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::{
    Client,
    primitives::ByteStream,
    types::{CompletedMultipartUpload, CompletedPart},
};
use aws_types::region::Region;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;

#[derive(Debug, Clone, Default)]
pub struct PutOptions {
    pub content_type: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct StoredObject {
    pub bucket: Option<String>,
    pub key: String,
    pub content_length: u64,
    pub etag: Option<String>,
    pub content_type: Option<String>,
    pub metadata: BTreeMap<String, String>,
    pub local_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PresignedOperation {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletedMultipartPart {
    pub part_number: u32,
    pub etag: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipartPart {
    pub part_number: u32,
    pub etag: String,
    pub size: u64,
}

#[async_trait]
pub trait ObjectStore: Send + Sync {
    fn provider(&self) -> &'static str;
    fn bucket(&self) -> Option<&str>;
    async fn put_bytes(
        &self,
        key: &str,
        bytes: Bytes,
        options: PutOptions,
    ) -> ApiResult<StoredObject>;
    async fn presign_put(
        &self,
        key: &str,
        options: PutOptions,
        expires_in: Duration,
    ) -> ApiResult<PresignedOperation>;
    async fn create_multipart_upload(&self, key: &str, options: PutOptions) -> ApiResult<String>;
    async fn presign_upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: u32,
        expires_in: Duration,
    ) -> ApiResult<PresignedOperation>;
    async fn list_multipart_parts(
        &self,
        key: &str,
        upload_id: &str,
    ) -> ApiResult<Vec<MultipartPart>>;
    async fn complete_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<CompletedMultipartPart>,
    ) -> ApiResult<StoredObject>;
    async fn abort_multipart_upload(&self, key: &str, upload_id: &str) -> ApiResult<()>;
    async fn presign_get(&self, key: &str, expires_in: Duration) -> ApiResult<PresignedOperation>;
    async fn head(&self, key: &str) -> ApiResult<StoredObject>;
    async fn get_bytes(&self, key: &str) -> ApiResult<Bytes>;
    async fn delete(&self, key: &str) -> ApiResult<()>;
}

pub async fn object_store_from_config(config: &ApiConfig) -> ApiResult<Arc<dyn ObjectStore>> {
    match config.storage_backend.as_str() {
        "r2" => Ok(Arc::new(R2ObjectStore::from_config(config).await?)),
        "local" => Ok(Arc::new(LocalObjectStore::new(
            config.casebuilder_storage_dir.clone().into(),
        ))),
        other => Err(ApiError::Config(config::ConfigError::Message(format!(
            "Unsupported ORS_STORAGE_BACKEND {other}; expected local or r2"
        )))),
    }
}

#[derive(Debug, Clone)]
pub struct LocalObjectStore {
    root: PathBuf,
}

impl LocalObjectStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path_for_key(&self, key: &str) -> PathBuf {
        key.split('/')
            .filter(|part| !part.trim().is_empty())
            .fold(self.root.clone(), |path, part| {
                path.join(sanitize_path_part(part))
            })
    }
}

#[async_trait]
impl ObjectStore for LocalObjectStore {
    fn provider(&self) -> &'static str {
        "local"
    }

    fn bucket(&self) -> Option<&str> {
        None
    }

    async fn put_bytes(
        &self,
        key: &str,
        bytes: Bytes,
        options: PutOptions,
    ) -> ApiResult<StoredObject> {
        let path = self.path_for_key(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(io_error)?;
        }
        fs::write(&path, &bytes).await.map_err(io_error)?;
        Ok(StoredObject {
            bucket: None,
            key: key.to_string(),
            content_length: bytes.len() as u64,
            etag: None,
            content_type: options.content_type,
            metadata: options.metadata,
            local_path: Some(path.to_string_lossy().to_string()),
        })
    }

    async fn presign_put(
        &self,
        _key: &str,
        _options: PutOptions,
        _expires_in: Duration,
    ) -> ApiResult<PresignedOperation> {
        Err(ApiError::BadRequest(
            "Signed browser uploads require ORS_STORAGE_BACKEND=r2".to_string(),
        ))
    }

    async fn create_multipart_upload(&self, _key: &str, _options: PutOptions) -> ApiResult<String> {
        Err(ApiError::BadRequest(
            "Multipart browser uploads require ORS_STORAGE_BACKEND=r2".to_string(),
        ))
    }

    async fn presign_upload_part(
        &self,
        _key: &str,
        _upload_id: &str,
        _part_number: u32,
        _expires_in: Duration,
    ) -> ApiResult<PresignedOperation> {
        Err(ApiError::BadRequest(
            "Multipart browser uploads require ORS_STORAGE_BACKEND=r2".to_string(),
        ))
    }

    async fn list_multipart_parts(
        &self,
        _key: &str,
        _upload_id: &str,
    ) -> ApiResult<Vec<MultipartPart>> {
        Err(ApiError::BadRequest(
            "Multipart browser uploads require ORS_STORAGE_BACKEND=r2".to_string(),
        ))
    }

    async fn complete_multipart_upload(
        &self,
        _key: &str,
        _upload_id: &str,
        _parts: Vec<CompletedMultipartPart>,
    ) -> ApiResult<StoredObject> {
        Err(ApiError::BadRequest(
            "Multipart browser uploads require ORS_STORAGE_BACKEND=r2".to_string(),
        ))
    }

    async fn abort_multipart_upload(&self, _key: &str, _upload_id: &str) -> ApiResult<()> {
        Err(ApiError::BadRequest(
            "Multipart browser uploads require ORS_STORAGE_BACKEND=r2".to_string(),
        ))
    }

    async fn presign_get(
        &self,
        _key: &str,
        _expires_in: Duration,
    ) -> ApiResult<PresignedOperation> {
        Err(ApiError::BadRequest(
            "Signed browser downloads require ORS_STORAGE_BACKEND=r2".to_string(),
        ))
    }

    async fn head(&self, key: &str) -> ApiResult<StoredObject> {
        let path = self.path_for_key(key);
        let metadata = fs::metadata(&path).await.map_err(io_error)?;
        Ok(StoredObject {
            bucket: None,
            key: key.to_string(),
            content_length: metadata.len(),
            etag: None,
            content_type: None,
            metadata: BTreeMap::new(),
            local_path: Some(path.to_string_lossy().to_string()),
        })
    }

    async fn get_bytes(&self, key: &str) -> ApiResult<Bytes> {
        let path = self.path_for_key(key);
        let bytes = fs::read(path).await.map_err(io_error)?;
        Ok(Bytes::from(bytes))
    }

    async fn delete(&self, key: &str) -> ApiResult<()> {
        let path = self.path_for_key(key);
        match fs::remove_file(path).await {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(io_error(error)),
        }
    }
}

#[derive(Clone)]
pub struct R2ObjectStore {
    bucket: String,
    client: Client,
}

impl R2ObjectStore {
    pub async fn from_config(config: &ApiConfig) -> ApiResult<Self> {
        let account_id = config
            .r2_account_id
            .as_deref()
            .ok_or_else(|| missing_r2_config("ORS_R2_ACCOUNT_ID"))?;
        let bucket = config
            .r2_bucket
            .clone()
            .ok_or_else(|| missing_r2_config("ORS_R2_BUCKET"))?;
        let access_key_id = config
            .r2_access_key_id
            .as_deref()
            .ok_or_else(|| missing_r2_config("ORS_R2_ACCESS_KEY_ID"))?;
        let secret_access_key = config
            .r2_secret_access_key
            .as_deref()
            .ok_or_else(|| missing_r2_config("ORS_R2_SECRET_ACCESS_KEY"))?;
        let endpoint = config
            .r2_endpoint
            .clone()
            .unwrap_or_else(|| format!("https://{account_id}.r2.cloudflarestorage.com"));
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(endpoint)
            .region(Region::new("auto"))
            .credentials_provider(Credentials::new(
                access_key_id,
                secret_access_key,
                None,
                None,
                "r2",
            ))
            .load()
            .await;
        Ok(Self {
            bucket,
            client: Client::new(&sdk_config),
        })
    }
}

#[async_trait]
impl ObjectStore for R2ObjectStore {
    fn provider(&self) -> &'static str {
        "r2"
    }

    fn bucket(&self) -> Option<&str> {
        Some(&self.bucket)
    }

    async fn put_bytes(
        &self,
        key: &str,
        bytes: Bytes,
        options: PutOptions,
    ) -> ApiResult<StoredObject> {
        let mut request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(bytes.clone()));
        if let Some(content_type) = &options.content_type {
            request = request.content_type(content_type);
        }
        for (name, value) in &options.metadata {
            request = request.metadata(name, value);
        }
        let response = request
            .send()
            .await
            .map_err(|_| ApiError::External("R2 put_object failed".to_string()))?;
        Ok(StoredObject {
            bucket: Some(self.bucket.clone()),
            key: key.to_string(),
            content_length: bytes.len() as u64,
            etag: response.e_tag().map(clean_etag),
            content_type: options.content_type,
            metadata: options.metadata,
            local_path: None,
        })
    }

    async fn presign_put(
        &self,
        key: &str,
        options: PutOptions,
        expires_in: Duration,
    ) -> ApiResult<PresignedOperation> {
        let presigning_config = PresigningConfig::expires_in(expires_in)
            .map_err(|error| ApiError::BadRequest(format!("Invalid upload URL expiry: {error}")))?;
        let mut request = self.client.put_object().bucket(&self.bucket).key(key);
        if let Some(content_type) = &options.content_type {
            request = request.content_type(content_type);
        }
        for (name, value) in &options.metadata {
            request = request.metadata(name, value);
        }
        let presigned = request
            .presigned(presigning_config)
            .await
            .map_err(|_| ApiError::External("R2 presign PUT failed".to_string()))?;
        let mut headers = headers_to_map(presigned.headers());
        if let Some(content_type) = options.content_type {
            headers
                .entry("content-type".to_string())
                .or_insert(content_type);
        }
        for (name, value) in options.metadata {
            headers
                .entry(format!("x-amz-meta-{name}").to_ascii_lowercase())
                .or_insert(value);
        }
        Ok(PresignedOperation {
            method: presigned.method().to_string(),
            url: presigned.uri().to_string(),
            headers,
        })
    }

    async fn create_multipart_upload(&self, key: &str, options: PutOptions) -> ApiResult<String> {
        let mut request = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key);
        if let Some(content_type) = &options.content_type {
            request = request.content_type(content_type);
        }
        for (name, value) in &options.metadata {
            request = request.metadata(name, value);
        }
        let response = request
            .send()
            .await
            .map_err(|_| ApiError::External("R2 create_multipart_upload failed".to_string()))?;
        response.upload_id().map(str::to_string).ok_or_else(|| {
            ApiError::External("R2 multipart upload returned no upload id".to_string())
        })
    }

    async fn presign_upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: u32,
        expires_in: Duration,
    ) -> ApiResult<PresignedOperation> {
        let presigning_config = PresigningConfig::expires_in(expires_in).map_err(|error| {
            ApiError::BadRequest(format!("Invalid upload part URL expiry: {error}"))
        })?;
        let presigned = self
            .client
            .upload_part()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .part_number(part_number as i32)
            .presigned(presigning_config)
            .await
            .map_err(|_| ApiError::External("R2 presign UploadPart failed".to_string()))?;
        Ok(PresignedOperation {
            method: presigned.method().to_string(),
            url: presigned.uri().to_string(),
            headers: headers_to_map(presigned.headers()),
        })
    }

    async fn list_multipart_parts(
        &self,
        key: &str,
        upload_id: &str,
    ) -> ApiResult<Vec<MultipartPart>> {
        let mut marker = None;
        let mut parts = Vec::new();
        loop {
            let mut request = self
                .client
                .list_parts()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(upload_id)
                .max_parts(1000);
            if let Some(value) = marker.take() {
                request = request.part_number_marker(value);
            }
            let response = request
                .send()
                .await
                .map_err(|_| ApiError::External("R2 list_parts failed".to_string()))?;
            for part in response.parts() {
                let part_number = part.part_number().unwrap_or_default();
                if part_number <= 0 {
                    continue;
                }
                parts.push(MultipartPart {
                    part_number: part_number as u32,
                    etag: part.e_tag().map(clean_etag).unwrap_or_default(),
                    size: part.size().unwrap_or_default().max(0) as u64,
                });
            }
            marker = response.next_part_number_marker().map(str::to_string);
            if marker.is_none() {
                break;
            }
        }
        Ok(parts)
    }

    async fn complete_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<CompletedMultipartPart>,
    ) -> ApiResult<StoredObject> {
        let completed_parts = parts
            .into_iter()
            .map(|part| {
                CompletedPart::builder()
                    .part_number(part.part_number as i32)
                    .e_tag(clean_etag(&part.etag))
                    .build()
            })
            .collect();
        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();
        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
            .map_err(|_| ApiError::External("R2 complete_multipart_upload failed".to_string()))?;
        self.head(key).await
    }

    async fn abort_multipart_upload(&self, key: &str, upload_id: &str) -> ApiResult<()> {
        self.client
            .abort_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|_| ApiError::External("R2 abort_multipart_upload failed".to_string()))?;
        Ok(())
    }

    async fn presign_get(&self, key: &str, expires_in: Duration) -> ApiResult<PresignedOperation> {
        let presigning_config = PresigningConfig::expires_in(expires_in).map_err(|error| {
            ApiError::BadRequest(format!("Invalid download URL expiry: {error}"))
        })?;
        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|_| ApiError::External("R2 presign GET failed".to_string()))?;
        Ok(PresignedOperation {
            method: presigned.method().to_string(),
            url: presigned.uri().to_string(),
            headers: headers_to_map(presigned.headers()),
        })
    }

    async fn head(&self, key: &str) -> ApiResult<StoredObject> {
        let response = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| map_r2_read_error("head_object", error))?;
        Ok(StoredObject {
            bucket: Some(self.bucket.clone()),
            key: key.to_string(),
            content_length: response.content_length().unwrap_or_default().max(0) as u64,
            etag: response.e_tag().map(clean_etag),
            content_type: response.content_type().map(str::to_string),
            metadata: response
                .metadata()
                .map(|metadata| {
                    metadata
                        .iter()
                        .map(|(name, value)| (name.to_string(), value.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            local_path: None,
        })
    }

    async fn get_bytes(&self, key: &str) -> ApiResult<Bytes> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| map_r2_read_error("get_object", error))?;
        let data = response
            .body
            .collect()
            .await
            .map_err(|_| ApiError::External("R2 body read failed".to_string()))?;
        Ok(data.into_bytes())
    }

    async fn delete(&self, key: &str) -> ApiResult<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|_| ApiError::External("R2 delete_object failed".to_string()))?;
        Ok(())
    }
}

pub fn build_document_object_key(document_id: &str, filename: &str) -> String {
    let ext = file_extension(filename).unwrap_or_else(|| "bin".to_string());
    format!(
        "casebuilder/documents/{}/original.{}",
        hash_path_segment(document_id.as_bytes(), 24),
        ext
    )
}

pub fn normalize_sha256(value: &str) -> Option<String> {
    let raw = value.trim().strip_prefix("sha256:").unwrap_or(value.trim());
    if raw.len() == 64 && raw.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(format!("sha256:{}", raw.to_ascii_lowercase()))
    } else {
        None
    }
}

pub fn clean_etag(value: &str) -> String {
    value.trim_matches('"').to_string()
}

fn file_extension(filename: &str) -> Option<String> {
    Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ext.chars()
                .filter(|ch| ch.is_ascii_alphanumeric())
                .take(12)
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|ext| !ext.is_empty())
}

fn sanitize_path_part(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "object".to_string()
    } else {
        sanitized
    }
}

fn hash_path_segment(bytes: &[u8], chars: usize) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(chars);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
        if out.len() >= chars {
            break;
        }
    }
    out.truncate(chars);
    out
}

fn headers_to_map<'a>(
    headers: impl Iterator<Item = (&'a str, &'a str)>,
) -> BTreeMap<String, String> {
    headers
        .map(|(name, value)| (name.to_ascii_lowercase(), value.to_string()))
        .collect()
}

fn missing_r2_config(name: &str) -> ApiError {
    ApiError::Config(config::ConfigError::Message(format!(
        "{name} is required when ORS_STORAGE_BACKEND=r2"
    )))
}

fn map_r2_read_error<E: std::fmt::Display>(operation: &str, error: E) -> ApiError {
    let message = error.to_string();
    if message.contains("NotFound") || message.contains("404") || message.contains("NoSuchKey") {
        ApiError::NotFound(format!("R2 object not found during {operation}"))
    } else {
        ApiError::External(format!("R2 {operation} failed"))
    }
}

fn io_error(error: std::io::Error) -> ApiError {
    match error.kind() {
        std::io::ErrorKind::NotFound => ApiError::NotFound("Stored object not found".to_string()),
        _ => ApiError::Internal(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_document_object_key, normalize_sha256};

    #[test]
    fn object_keys_do_not_include_raw_filenames() {
        let key = build_document_object_key("doc:Tenant Notice:123", "../Secret Notice.pdf");
        assert!(key.starts_with("casebuilder/documents/"));
        assert!(!key.contains("Secret Notice"));
        assert!(!key.contains("Tenant"));
        assert!(!key.contains("Notice"));
        assert!(!key.contains(".."));
        assert!(key.ends_with("/original.pdf"));
    }

    #[test]
    fn normalizes_sha256_values() {
        let hash = "ABCDEFabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234";
        assert_eq!(
            normalize_sha256(hash),
            Some(
                "sha256:abcdefabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234"
                    .to_string()
            )
        );
        assert_eq!(normalize_sha256("not-a-hash"), None);
    }
}
