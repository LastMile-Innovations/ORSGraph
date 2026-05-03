use super::*;

impl CaseBuilderService {
    pub(super) async fn document_bytes_as_text(
        &self,
        document: &CaseDocument,
    ) -> ApiResult<String> {
        match self.document_bytes(document).await {
            Ok(bytes) => {
                Ok(
                    parse_document_bytes(&document.filename, document.mime_type.as_deref(), &bytes)
                        .text
                        .unwrap_or_default(),
                )
            }
            Err(ApiError::NotFound(_)) => Ok(String::new()),
            Err(error) => Err(error),
        }
    }

    pub(super) async fn document_bytes(&self, document: &CaseDocument) -> ApiResult<Bytes> {
        if document.storage_status == "deleted" {
            return Err(ApiError::NotFound("Document has been deleted".to_string()));
        }
        if let Some(key) = document.storage_key.as_deref() {
            return self.object_store.get_bytes(key).await;
        }
        if let Some(path) = document.storage_path.as_deref() {
            return fs::read(path).await.map(Bytes::from).map_err(io_error);
        }
        Err(ApiError::NotFound(
            "Document source bytes are not available".to_string(),
        ))
    }

    pub(super) async fn document_presigned_get_url(
        &self,
        document: &CaseDocument,
        expires_in: Duration,
    ) -> ApiResult<Option<String>> {
        if document.storage_status == "deleted" || self.object_store.provider() != "r2" {
            return Ok(None);
        }
        let Some(key) = document.storage_key.as_deref() else {
            return Ok(None);
        };
        let signed = self.object_store.presign_get(key, expires_in).await?;
        if !signed.method.eq_ignore_ascii_case("GET") {
            return Err(ApiError::External(
                "R2 signed media URL was not a GET operation.".to_string(),
            ));
        }
        Ok(Some(signed.url))
    }

    pub(super) fn ensure_upload_size(&self, bytes: u64) -> ApiResult<()> {
        if bytes > self.max_upload_bytes {
            Err(ApiError::BadRequest(format!(
                "Upload is {bytes} bytes; maximum is {} bytes",
                self.max_upload_bytes
            )))
        } else {
            Ok(())
        }
    }
}
