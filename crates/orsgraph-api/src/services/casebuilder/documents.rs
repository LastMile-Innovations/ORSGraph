use super::*;

impl CaseBuilderService {
    pub async fn upload_file(
        &self,
        matter_id: &str,
        request: UploadFileRequest,
    ) -> ApiResult<CaseDocument> {
        self.require_matter(matter_id).await?;
        let now = now_string();
        let document_id = generate_opaque_id("doc");
        let title = title_from_filename(&request.filename);
        let relative_path = normalize_upload_relative_path(request.relative_path)?;
        let upload_batch_id = normalize_upload_batch_id(request.upload_batch_id)?;
        let folder = request
            .folder
            .or_else(|| folder_from_relative_path(relative_path.as_deref()))
            .unwrap_or_else(|| "Uploads".to_string());
        let library_path =
            library_path_from_upload(relative_path.as_deref(), &folder, &request.filename)?;
        let bytes = request
            .text
            .as_ref()
            .map(|text| text.len() as u64)
            .or(request.bytes)
            .unwrap_or(0);
        self.ensure_upload_size(bytes)?;
        let object_key = build_document_object_key(&document_id, &request.filename);
        let (stored_object, hash) = if let Some(text) = &request.text {
            let hash = sha256_hex(text.as_bytes());
            let stored = self
                .object_store
                .put_bytes(
                    &object_key,
                    Bytes::copy_from_slice(text.as_bytes()),
                    put_options(request.mime_type.clone(), Some(hash.clone())),
                )
                .await?;
            (Some(stored), Some(hash))
        } else {
            (None, None)
        };
        let storage_status = if stored_object.is_some() {
            "stored"
        } else {
            "metadata_only"
        };

        let mut document = CaseDocument {
            id: document_id.clone(),
            document_id,
            matter_id: matter_id.to_string(),
            filename: request.filename,
            title,
            document_type: request.document_type.unwrap_or_else(|| "other".to_string()),
            mime_type: request.mime_type,
            pages: 1,
            bytes,
            file_hash: hash,
            uploaded_at: now,
            source: "user_upload".to_string(),
            confidentiality: request
                .confidentiality
                .unwrap_or_else(|| "private".to_string()),
            processing_status: if request.text.is_some() {
                "processed".to_string()
            } else {
                "queued".to_string()
            },
            is_exhibit: false,
            exhibit_label: None,
            summary: "Uploaded to CaseBuilder. Run extraction to populate facts and evidence."
                .to_string(),
            date_observed: None,
            parties_mentioned: Vec::new(),
            entities_mentioned: Vec::new(),
            facts_extracted: 0,
            citations_found: 0,
            contradictions_flagged: 0,
            linked_claim_ids: Vec::new(),
            folder,
            storage_path: stored_object
                .as_ref()
                .and_then(|object| object.local_path.clone()),
            storage_provider: self.object_store.provider().to_string(),
            storage_status: storage_status.to_string(),
            storage_bucket: stored_object
                .as_ref()
                .and_then(|object| object.bucket.clone())
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            storage_key: stored_object.as_ref().map(|object| object.key.clone()),
            content_etag: stored_object
                .as_ref()
                .and_then(|object| object.etag.clone()),
            upload_expires_at: None,
            deleted_at: None,
            library_path: Some(library_path),
            archived_at: None,
            archived_reason: None,
            original_relative_path: relative_path,
            upload_batch_id,
            object_blob_id: None,
            current_version_id: None,
            ingestion_run_ids: Vec::new(),
            source_spans: Vec::new(),
            extracted_text: request.text,
        };

        let provenance = stored_object
            .as_ref()
            .map(|object| build_original_provenance(matter_id, &document, object, "stored"));
        if let Some(provenance) = &provenance {
            apply_document_provenance(&mut document, provenance);
        }

        let document = self
            .merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await?;
        if let Some(provenance) = provenance {
            self.persist_document_provenance(matter_id, &provenance)
                .await?;
        }
        Ok(document)
    }

    pub async fn upload_binary_file(
        &self,
        matter_id: &str,
        request: BinaryUploadRequest,
    ) -> ApiResult<CaseDocument> {
        self.require_matter(matter_id).await?;
        self.ensure_upload_size(request.bytes.len() as u64)?;
        validate_mime_type(request.mime_type.as_deref())?;

        let now = now_string();
        let document_id = generate_opaque_id("doc");
        let relative_path = normalize_upload_relative_path(request.relative_path)?;
        let upload_batch_id = normalize_upload_batch_id(request.upload_batch_id)?;
        let folder = request
            .folder
            .or_else(|| folder_from_relative_path(relative_path.as_deref()))
            .unwrap_or_else(|| "Uploads".to_string());
        let library_path =
            library_path_from_upload(relative_path.as_deref(), &folder, &request.filename)?;
        let object_key = build_document_object_key(&document_id, &request.filename);
        let hash = sha256_hex(&request.bytes);
        let stored_object = self
            .object_store
            .put_bytes(
                &object_key,
                request.bytes.clone(),
                put_options(request.mime_type.clone(), Some(hash.clone())),
            )
            .await?;
        let parser = parse_document_bytes(
            &request.filename,
            request.mime_type.as_deref(),
            &request.bytes,
        );
        let parser_id = parser.parser_id.clone();
        let processing_status = parser.status.clone();

        let mut document = CaseDocument {
            id: document_id.clone(),
            document_id,
            matter_id: matter_id.to_string(),
            filename: request.filename.clone(),
            title: title_from_filename(&request.filename),
            document_type: request.document_type.unwrap_or_else(|| "other".to_string()),
            mime_type: request.mime_type,
            pages: 1,
            bytes: stored_object.content_length,
            file_hash: Some(hash),
            uploaded_at: now,
            source: "user_upload".to_string(),
            confidentiality: request
                .confidentiality
                .unwrap_or_else(|| "private".to_string()),
            processing_status,
            is_exhibit: false,
            exhibit_label: None,
            summary: parser.message,
            date_observed: None,
            parties_mentioned: Vec::new(),
            entities_mentioned: Vec::new(),
            facts_extracted: 0,
            citations_found: 0,
            contradictions_flagged: 0,
            linked_claim_ids: Vec::new(),
            folder,
            storage_path: stored_object.local_path.clone(),
            storage_provider: self.object_store.provider().to_string(),
            storage_status: "stored".to_string(),
            storage_bucket: stored_object
                .bucket
                .clone()
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            storage_key: Some(stored_object.key.clone()),
            content_etag: stored_object.etag.clone(),
            upload_expires_at: None,
            deleted_at: None,
            library_path: Some(library_path),
            archived_at: None,
            archived_reason: None,
            original_relative_path: relative_path,
            upload_batch_id,
            object_blob_id: None,
            current_version_id: None,
            ingestion_run_ids: Vec::new(),
            source_spans: Vec::new(),
            extracted_text: parser.text,
        };

        let mut provenance =
            build_original_provenance(matter_id, &document, &stored_object, "stored");
        provenance.ingestion_run.parser_id = Some(parser_id);
        apply_document_provenance(&mut document, &provenance);
        let document = self
            .merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await?;
        self.persist_document_provenance(matter_id, &provenance)
            .await?;
        Ok(document)
    }

    pub async fn create_file_upload(
        &self,
        matter_id: &str,
        request: CreateFileUploadRequest,
    ) -> ApiResult<FileUploadResponse> {
        self.require_matter(matter_id).await?;
        if request.bytes == 0 {
            return Err(ApiError::BadRequest(
                "Upload intent bytes must be greater than 0".to_string(),
            ));
        }
        self.ensure_upload_size(request.bytes)?;
        validate_mime_type(request.mime_type.as_deref())?;

        let normalized_hash = match request.sha256.as_deref() {
            Some(value) => Some(normalize_sha256(value).ok_or_else(|| {
                ApiError::BadRequest("sha256 must be a hex SHA-256 digest".to_string())
            })?),
            None => None,
        };
        let now = now_string();
        let document_id = generate_opaque_id("doc");
        let upload_id = upload_id_for_document(&document_id);
        let relative_path = normalize_upload_relative_path(request.relative_path)?;
        let upload_batch_id = normalize_upload_batch_id(request.upload_batch_id)?;
        let folder = request
            .folder
            .or_else(|| folder_from_relative_path(relative_path.as_deref()))
            .unwrap_or_else(|| "Uploads".to_string());
        let library_path =
            library_path_from_upload(relative_path.as_deref(), &folder, &request.filename)?;
        let object_key = build_document_object_key(&document_id, &request.filename);
        let expires_at = timestamp_after(self.upload_ttl_seconds);
        let presigned = self
            .object_store
            .presign_put(
                &object_key,
                put_options(request.mime_type.clone(), normalized_hash.clone()),
                Duration::from_secs(self.upload_ttl_seconds),
            )
            .await?;

        let document = CaseDocument {
            id: document_id.clone(),
            document_id: document_id.clone(),
            matter_id: matter_id.to_string(),
            filename: request.filename.clone(),
            title: title_from_filename(&request.filename),
            document_type: request.document_type.unwrap_or_else(|| "other".to_string()),
            mime_type: request.mime_type,
            pages: 1,
            bytes: request.bytes,
            file_hash: normalized_hash,
            uploaded_at: now,
            source: "user_upload".to_string(),
            confidentiality: request
                .confidentiality
                .unwrap_or_else(|| "private".to_string()),
            processing_status: "queued".to_string(),
            is_exhibit: false,
            exhibit_label: None,
            summary: "Upload pending. Complete the direct R2 upload to queue extraction."
                .to_string(),
            date_observed: None,
            parties_mentioned: Vec::new(),
            entities_mentioned: Vec::new(),
            facts_extracted: 0,
            citations_found: 0,
            contradictions_flagged: 0,
            linked_claim_ids: Vec::new(),
            folder,
            storage_path: None,
            storage_provider: self.object_store.provider().to_string(),
            storage_status: "pending".to_string(),
            storage_bucket: self.object_store.bucket().map(str::to_string),
            storage_key: Some(object_key),
            content_etag: None,
            upload_expires_at: Some(expires_at.clone()),
            deleted_at: None,
            library_path: Some(library_path),
            archived_at: None,
            archived_reason: None,
            original_relative_path: relative_path,
            upload_batch_id,
            object_blob_id: None,
            current_version_id: None,
            ingestion_run_ids: Vec::new(),
            source_spans: Vec::new(),
            extracted_text: None,
        };
        let document = self
            .merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await?;

        Ok(FileUploadResponse {
            upload_id,
            document_id,
            method: presigned.method,
            url: presigned.url,
            expires_at,
            headers: presigned.headers,
            document,
        })
    }

    pub async fn complete_file_upload(
        &self,
        matter_id: &str,
        upload_id: &str,
        request: CompleteFileUploadRequest,
    ) -> ApiResult<CaseDocument> {
        let mut document = self.get_document(matter_id, &request.document_id).await?;
        if upload_id_for_document(&document.document_id) != upload_id {
            return Err(ApiError::BadRequest(
                "Upload id does not match document".to_string(),
            ));
        }
        if document.storage_status == "deleted" {
            return Err(ApiError::BadRequest(
                "Cannot complete a deleted document upload".to_string(),
            ));
        }
        if let Some(expires_at) = &document.upload_expires_at {
            if parse_timestamp(expires_at).is_some_and(|expires| expires < now_secs()) {
                document.storage_status = "failed".to_string();
                document.summary = "Upload URL expired before completion.".to_string();
                self.merge_node(matter_id, document_spec(), &document.document_id, &document)
                    .await?;
                return Err(ApiError::BadRequest("Upload URL expired".to_string()));
            }
        }
        if let Some(bytes) = request.bytes {
            self.ensure_upload_size(bytes)?;
            if bytes != document.bytes {
                return Err(ApiError::BadRequest(
                    "Completed upload size does not match intent".to_string(),
                ));
            }
        }
        if let Some(sha256) = request.sha256.as_deref() {
            let normalized = normalize_sha256(sha256).ok_or_else(|| {
                ApiError::BadRequest("sha256 must be a hex SHA-256 digest".to_string())
            })?;
            if document
                .file_hash
                .as_deref()
                .is_some_and(|expected| expected != normalized)
            {
                return Err(ApiError::BadRequest(
                    "Completed upload hash does not match intent".to_string(),
                ));
            }
            document.file_hash = Some(normalized);
        }

        let key = document
            .storage_key
            .clone()
            .ok_or_else(|| ApiError::BadRequest("Document has no storage key".to_string()))?;
        let object = self.object_store.head(&key).await?;
        if object.content_length != document.bytes {
            document.storage_status = "failed".to_string();
            document.summary = "Uploaded object size did not match the upload intent.".to_string();
            self.merge_node(matter_id, document_spec(), &document.document_id, &document)
                .await?;
            return Err(ApiError::BadRequest(
                "Uploaded object size did not match intent".to_string(),
            ));
        }
        if let (Some(actual), Some(expected)) = (object.etag.as_deref(), request.etag.as_deref()) {
            if clean_etag(actual) != clean_etag(expected) {
                return Err(ApiError::BadRequest(
                    "Completed upload ETag does not match R2 object".to_string(),
                ));
            }
        }
        if let Some(expected_hash) = document.file_hash.as_deref() {
            if let Some(actual_hash) = object.metadata.get("sha256") {
                if actual_hash != expected_hash {
                    return Err(ApiError::BadRequest(
                        "Completed upload hash metadata does not match intent".to_string(),
                    ));
                }
            }
        }
        if document.file_hash.is_none() {
            document.file_hash = object
                .metadata
                .get("sha256")
                .and_then(|hash| normalize_sha256(hash));
        }

        document.storage_status = "stored".to_string();
        document.storage_bucket = object
            .bucket
            .clone()
            .or_else(|| self.object_store.bucket().map(str::to_string));
        document.content_etag = object.etag.clone();
        document.upload_expires_at = None;
        document.summary =
            "Uploaded to private object storage. Run extraction to populate facts and evidence."
                .to_string();
        let provenance = build_original_provenance(matter_id, &document, &object, "stored");
        apply_document_provenance(&mut document, &provenance);
        let document = self
            .merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await?;
        self.persist_document_provenance(matter_id, &provenance)
            .await?;
        Ok(document)
    }

    pub async fn list_documents(&self, matter_id: &str) -> ApiResult<Vec<CaseDocument>> {
        self.list_nodes(matter_id, document_spec()).await
    }

    pub async fn get_matter_index_summary(&self, matter_id: &str) -> ApiResult<MatterIndexSummary> {
        self.require_matter(matter_id).await?;
        let documents = self.list_documents(matter_id).await?;
        let runs = self
            .list_nodes::<IngestionRun>(matter_id, ingestion_run_spec())
            .await?;
        Ok(build_matter_index_summary(matter_id, &documents, &runs))
    }

    pub async fn run_matter_index(
        &self,
        matter_id: &str,
        request: RunMatterIndexRequest,
    ) -> ApiResult<MatterIndexRunResponse> {
        self.require_matter(matter_id).await?;
        let documents = self.list_documents(matter_id).await?;
        let requested_ids = request
            .document_ids
            .unwrap_or_default()
            .into_iter()
            .filter(|id| !id.trim().is_empty())
            .collect::<Vec<_>>();
        let limit = request.limit.unwrap_or(250).clamp(1, 1_000) as usize;

        let mut targets = Vec::new();
        if requested_ids.is_empty() {
            targets = documents
                .iter()
                .filter(|document| {
                    document.archived_at.is_none()
                        && document_can_attempt_index(document)
                        && document_needs_index(document)
                })
                .take(limit)
                .map(|document| document.document_id.clone())
                .collect();
        } else {
            let known = documents
                .iter()
                .map(|document| document.document_id.as_str())
                .collect::<HashSet<_>>();
            for document_id in requested_ids.into_iter().take(limit) {
                if !known.contains(document_id.as_str()) {
                    return Err(ApiError::NotFound(format!(
                        "Document {document_id} was not found in this matter"
                    )));
                }
                targets.push(document_id);
            }
        }

        let requested = targets.len() as u64;
        let mut processed = 0_u64;
        let mut skipped = 0_u64;
        let mut failed = 0_u64;
        let mut produced_timeline_suggestions = 0_u64;
        let mut results = Vec::with_capacity(targets.len());

        for document_id in targets {
            let document = self.get_document(matter_id, &document_id).await?;
            if document.archived_at.is_some() {
                skipped += 1;
                results.push(MatterIndexRunDocumentResult {
                    document_id,
                    status: "skipped".to_string(),
                    extraction_status: Some(document.processing_status.clone()),
                    message: "Archived documents stay out of active indexing.".to_string(),
                    produced_chunks: 0,
                    produced_facts: document.facts_extracted,
                    produced_timeline_suggestions: 0,
                });
                continue;
            }
            if !document_can_attempt_index(&document) {
                skipped += 1;
                results.push(MatterIndexRunDocumentResult {
                    document_id,
                    status: "skipped".to_string(),
                    extraction_status: Some(document.processing_status.clone()),
                    message: index_skip_message(&document),
                    produced_chunks: 0,
                    produced_facts: document.facts_extracted,
                    produced_timeline_suggestions: 0,
                });
                continue;
            }

            match self.extract_document(matter_id, &document_id).await {
                Ok(extraction) if extraction.status == "processed" => {
                    processed += 1;
                    let suggestion_count = extraction.timeline_suggestions.len() as u64;
                    produced_timeline_suggestions += suggestion_count;
                    results.push(MatterIndexRunDocumentResult {
                        document_id,
                        status: "indexed".to_string(),
                        extraction_status: Some(extraction.status),
                        message: extraction.message,
                        produced_chunks: extraction.chunks.len() as u64,
                        produced_facts: extraction.proposed_facts.len() as u64,
                        produced_timeline_suggestions: suggestion_count,
                    });
                }
                Ok(extraction) => {
                    skipped += 1;
                    let suggestion_count = extraction.timeline_suggestions.len() as u64;
                    results.push(MatterIndexRunDocumentResult {
                        document_id,
                        status: "skipped".to_string(),
                        extraction_status: Some(extraction.status),
                        message: extraction.message,
                        produced_chunks: extraction.chunks.len() as u64,
                        produced_facts: extraction.proposed_facts.len() as u64,
                        produced_timeline_suggestions: suggestion_count,
                    });
                }
                Err(error) => {
                    failed += 1;
                    results.push(MatterIndexRunDocumentResult {
                        document_id,
                        status: "failed".to_string(),
                        extraction_status: None,
                        message: error.to_string(),
                        produced_chunks: 0,
                        produced_facts: 0,
                        produced_timeline_suggestions: 0,
                    });
                }
            }
        }

        let summary = self.get_matter_index_summary(matter_id).await?;
        Ok(MatterIndexRunResponse {
            matter_id: matter_id.to_string(),
            requested,
            processed,
            skipped,
            failed,
            produced_timeline_suggestions,
            results,
            summary,
        })
    }

    pub async fn get_document(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<CaseDocument> {
        self.get_node(matter_id, document_spec(), document_id).await
    }

    pub async fn patch_document(
        &self,
        matter_id: &str,
        document_id: &str,
        request: PatchDocumentRequest,
    ) -> ApiResult<CaseDocument> {
        let mut document = self.get_document(matter_id, document_id).await?;
        if let Some(title) = normalize_optional_label(request.title, "title")? {
            document.title = title;
        }
        if let Some(document_type) =
            normalize_optional_label(request.document_type, "document_type")?
        {
            document.document_type = document_type;
        }
        if let Some(confidentiality) =
            normalize_optional_label(request.confidentiality, "confidentiality")?
        {
            document.confidentiality = confidentiality;
        }
        if let Some(library_path) = request.library_path {
            let normalized =
                normalize_document_library_path(Some(library_path), &document.filename)?;
            document.folder = folder_from_library_path(&normalized);
            document.library_path = Some(normalized);
        }
        if let Some(is_exhibit) = request.is_exhibit {
            document.is_exhibit = is_exhibit;
        }
        if let Some(exhibit_label) = request.exhibit_label {
            document.exhibit_label = normalize_nullable_label(exhibit_label);
        }
        if let Some(date_observed) = request.date_observed {
            document.date_observed = normalize_nullable_label(date_observed);
        }
        self.merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await
    }

    pub async fn archive_document(
        &self,
        matter_id: &str,
        document_id: &str,
        request: ArchiveDocumentRequest,
    ) -> ApiResult<CaseDocument> {
        let mut document = self.get_document(matter_id, document_id).await?;
        document.archived_at = Some(now_string());
        document.archived_reason = normalize_nullable_label(request.reason);
        self.merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await
    }

    pub async fn restore_document(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<CaseDocument> {
        let mut document = self.get_document(matter_id, document_id).await?;
        document.archived_at = None;
        document.archived_reason = None;
        self.merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await
    }

    pub async fn get_document_workspace(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<DocumentWorkspace> {
        let document = self.get_document(matter_id, document_id).await?;
        let current_version = self.current_document_version(matter_id, &document).await?;
        let annotations = self
            .list_document_annotations(matter_id, document_id)
            .await?;
        let source_spans = self
            .list_nodes::<SourceSpan>(matter_id, source_span_spec())
            .await?
            .into_iter()
            .filter(|span| span.document_id == document_id)
            .collect::<Vec<_>>();
        let mut warnings = Vec::new();
        let bytes = match self.document_bytes(&document).await {
            Ok(bytes) => Some(bytes),
            Err(ApiError::NotFound(_)) if document.storage_status != "stored" => None,
            Err(error) => {
                warnings.push(format!("Source bytes are unavailable: {error}"));
                None
            }
        };
        let docx_manifest = if document_is_docx(&document) {
            match bytes.as_ref() {
                Some(bytes) => {
                    match docx_package_manifest(&document, current_version.as_ref(), bytes) {
                        Ok(manifest) => {
                            if !manifest.editable {
                                warnings.push(
                                "DOCX contains unsupported complex OOXML parts; editable text is gated for review."
                                    .to_string(),
                            );
                            }
                            Some(manifest)
                        }
                        Err(error) => {
                            warnings.push(format!("DOCX package manifest failed: {error}"));
                            None
                        }
                    }
                }
                None => None,
            }
        } else {
            None
        };
        let text_content = workspace_text_content(&document, bytes.as_ref());
        let capabilities = document_capabilities(&document, docx_manifest.as_ref());
        let transcriptions = if document_is_media(&document) {
            self.list_transcriptions(matter_id, document_id).await?
        } else {
            Vec::new()
        };
        let content_url = if document.storage_status == "stored" || document.storage_key.is_some() {
            Some(format!(
                "/api/v1/matters/{}/documents/{}/content",
                matter_id, document_id
            ))
        } else {
            None
        };

        Ok(DocumentWorkspace {
            matter_id: matter_id.to_string(),
            document,
            current_version,
            capabilities,
            annotations,
            source_spans,
            transcriptions,
            docx_manifest,
            text_content,
            content_url,
            warnings,
        })
    }

    pub async fn get_document_content_bytes(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<(CaseDocument, Bytes)> {
        let document = self.get_document(matter_id, document_id).await?;
        let bytes = self.document_bytes(&document).await?;
        Ok((document, bytes))
    }

    pub async fn list_document_annotations(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<Vec<DocumentAnnotation>> {
        self.get_document(matter_id, document_id).await?;
        let mut annotations = self
            .list_nodes::<DocumentAnnotation>(matter_id, document_annotation_spec())
            .await?
            .into_iter()
            .filter(|annotation| annotation.document_id == document_id)
            .filter(|annotation| annotation.status != "deleted")
            .collect::<Vec<_>>();
        annotations.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(annotations)
    }

    pub async fn create_document_annotation(
        &self,
        matter_id: &str,
        document_id: &str,
        request: UpsertDocumentAnnotationRequest,
    ) -> ApiResult<DocumentAnnotation> {
        let document = self.get_document(matter_id, document_id).await?;
        validate_document_annotation_range(&request)?;
        self.validate_document_annotation_target(matter_id, &request)
            .await?;
        let annotation_type = normalize_document_annotation_type(&request.annotation_type)?;
        let now = now_string();
        let annotation_id = generate_opaque_id("document-annotation");
        let annotation = DocumentAnnotation {
            annotation_id: annotation_id.clone(),
            id: annotation_id,
            matter_id: matter_id.to_string(),
            document_id: document_id.to_string(),
            document_version_id: document.current_version_id.clone(),
            annotation_type: annotation_type.clone(),
            status: request
                .status
                .filter(|status| !status.trim().is_empty())
                .unwrap_or_else(|| "active".to_string()),
            label: request
                .label
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| default_annotation_label(&annotation_type).to_string()),
            note: request.note.filter(|note| !note.trim().is_empty()),
            color: request.color.filter(|color| !color.trim().is_empty()),
            page_range: request.page_range,
            text_range: request.text_range,
            target_type: request.target_type.filter(|value| !value.trim().is_empty()),
            target_id: request.target_id.filter(|value| !value.trim().is_empty()),
            created_by: "user".to_string(),
            created_at: now.clone(),
            updated_at: now,
        };
        self.merge_document_annotation(matter_id, &annotation).await
    }

    pub async fn save_document_text(
        &self,
        matter_id: &str,
        document_id: &str,
        request: SaveDocumentTextRequest,
    ) -> ApiResult<SaveDocumentTextResponse> {
        let document = self.get_document(matter_id, document_id).await?;
        let mut warnings = Vec::new();
        let (bytes, mime_type, extension) = if document_is_docx(&document) {
            let existing = self.document_bytes(&document).await?;
            let (updated, docx_warnings) =
                docx_with_replaced_document_xml(&existing, &request.text)?;
            warnings.extend(docx_warnings);
            (
                Bytes::from(updated),
                Some(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                        .to_string(),
                ),
                "docx".to_string(),
            )
        } else if document_is_markdown(&document) || document_is_text(&document) {
            (
                Bytes::from(request.text.into_bytes()),
                Some(if document_is_markdown(&document) {
                    "text/markdown".to_string()
                } else {
                    document
                        .mime_type
                        .clone()
                        .unwrap_or_else(|| "text/plain".to_string())
                }),
                document_file_extension(&document).unwrap_or_else(|| "txt".to_string()),
            )
        } else {
            return Err(ApiError::BadRequest(format!(
                "{} cannot be edited as structured text in the OSS document workspace.",
                document.filename
            )));
        };
        let (document, document_version, ingestion_run) = self
            .store_edited_document_bytes(
                matter_id,
                document,
                bytes,
                mime_type,
                &extension,
                "document_workspace_text_save",
            )
            .await?;
        Ok(SaveDocumentTextResponse {
            document,
            document_version,
            ingestion_run,
            warnings,
        })
    }

    pub async fn promote_document_work_product(
        &self,
        matter_id: &str,
        document_id: &str,
        request: PromoteDocumentWorkProductRequest,
    ) -> ApiResult<PromoteDocumentWorkProductResponse> {
        let document = self.get_document(matter_id, document_id).await?;
        if !(document_is_markdown(&document)
            || document_is_text(&document)
            || document_is_docx(&document))
        {
            return Err(ApiError::BadRequest(
                "Only Markdown, text, and supported DOCX text can be promoted into WorkProduct.document_ast in v1."
                    .to_string(),
            ));
        }
        let text = match document.extracted_text.clone() {
            Some(text) if !text.trim().is_empty() => text,
            _ => self.document_bytes_as_text(&document).await?,
        };
        if text.trim().is_empty() {
            return Err(ApiError::BadRequest(
                "No source text is available to promote into a work product.".to_string(),
            ));
        }
        let mut product = self
            .create_work_product(
                matter_id,
                CreateWorkProductRequest {
                    title: request.title.or_else(|| Some(document.title.clone())),
                    product_type: request.product_type.unwrap_or_else(|| "memo".to_string()),
                    template: None,
                    source_draft_id: None,
                    source_complaint_id: None,
                },
            )
            .await?;
        let markdown = if document_is_markdown(&document) {
            text.clone()
        } else {
            text_to_markdown_paragraphs(&text)
        };
        let (document_ast, warnings) =
            super::markdown_adapter::markdown_to_work_product_ast(&product, &markdown);
        product.document_ast = document_ast;
        product.blocks = flatten_work_product_blocks(&product.document_ast.blocks);
        product.history.push(work_product_event(
            matter_id,
            &product.work_product_id,
            "promote_document",
            "document",
            document_id,
            "Document source promoted into canonical WorkProduct AST.",
        ));
        let work_product = self.save_work_product(matter_id, product).await?;
        Ok(PromoteDocumentWorkProductResponse {
            work_product,
            warnings,
        })
    }

    pub async fn extract_document(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<DocumentExtractionResponse> {
        let mut document = self.get_document(matter_id, document_id).await?;
        let provenance = self
            .ensure_document_original_provenance(matter_id, &mut document)
            .await?;
        let text = match document.extracted_text.clone() {
            Some(text) if !text.trim().is_empty() => text,
            _ => self.document_bytes_as_text(&document).await?,
        };

        if text.trim().is_empty() {
            let extraction_status = match document.processing_status.as_str() {
                "ocr_required" | "transcription_deferred" | "unsupported" => {
                    document.processing_status.clone()
                }
                _ => "failed".to_string(),
            };
            let error_code = match extraction_status.as_str() {
                "ocr_required" => "ocr_required",
                "transcription_deferred" => "transcription_deferred",
                "unsupported" => "unsupported_file_type",
                _ => "no_extractable_text",
            };
            document.processing_status = extraction_status.clone();
            document.summary = match extraction_status.as_str() {
                "ocr_required" => {
                    "No extractable text is available yet; OCR is required for this document."
                }
                "transcription_deferred" => {
                    "No extractable text is available yet; transcription is deferred for this media file."
                }
                "unsupported" => {
                    "No extractable text is available for this unsupported deterministic V0 file type."
                }
                _ => "No extractable text is available for this document in V0.",
            }
            .to_string();
            let ingestion_run = provenance.as_ref().map(|provenance| {
                failed_ingestion_run(
                    &provenance.ingestion_run,
                    "extract_text",
                    error_code,
                    &document.summary,
                    matches!(
                        extraction_status.as_str(),
                        "ocr_required" | "transcription_deferred"
                    ),
                )
            });
            if let Some(run) = &ingestion_run {
                self.merge_ingestion_run(matter_id, run).await?;
            }
            let index_run = provenance.as_ref().map(|provenance| {
                failed_index_run(
                    matter_id,
                    &document,
                    provenance,
                    "extract_text",
                    error_code,
                    &document.summary,
                    matches!(
                        extraction_status.as_str(),
                        "ocr_required" | "transcription_deferred"
                    ),
                )
            });
            if let Some(run) = &index_run {
                self.merge_index_run(matter_id, run).await?;
            }
            let document = self
                .merge_node(matter_id, document_spec(), document_id, &document)
                .await?;
            return Ok(DocumentExtractionResponse {
                enabled: true,
                mode: "deterministic".to_string(),
                status: extraction_status,
                message: document.summary.clone(),
                document,
                chunks: Vec::new(),
                proposed_facts: Vec::new(),
                ingestion_run,
                index_run,
                document_version: provenance.map(|provenance| provenance.document_version),
                index_artifacts: Vec::new(),
                artifact_manifest: None,
                pages: Vec::new(),
                text_chunks: Vec::new(),
                evidence_spans: Vec::new(),
                entity_mentions: Vec::new(),
                search_index_records: Vec::new(),
                source_spans: Vec::new(),
                timeline_suggestions: Vec::new(),
            });
        }

        let source_context = source_context_from_provenance(provenance.as_ref());
        let text_sha256 = sha256_hex(text.as_bytes());
        let index_run_id_value = index_run_id(
            document_id,
            source_context.document_version_id.as_deref(),
            &text_sha256,
        );
        let mut chunks = chunk_text(document_id, &text);
        for chunk in &mut chunks {
            chunk.document_version_id = source_context.document_version_id.clone();
            chunk.object_blob_id = source_context.object_blob_id.clone();
            chunk.source_span_id = Some(source_span_id(document_id, "chunk", chunk.page));
        }
        let (pages, text_chunks, evidence_spans, entity_mentions, search_index_records) =
            build_extraction_index_records(
                matter_id,
                document_id,
                &chunks,
                &source_context,
                &index_run_id_value,
            );
        let mut source_spans =
            source_spans_for_chunks(matter_id, document_id, &chunks, &source_context);
        let proposed_facts = propose_facts(matter_id, document_id, &text, &source_context);
        for fact in &proposed_facts {
            source_spans.extend(fact.source_spans.clone());
        }
        let timeline_agent_outcome = self
            .execute_timeline_agent_for_indexed_document(
                matter_id,
                document_id,
                &proposed_facts,
                &chunks,
                &index_run_id_value,
                50,
            )
            .await?;
        let timeline_suggestions = timeline_agent_outcome.suggestions.clone();
        let (index_artifacts, artifact_manifest) = self
            .store_extraction_index_artifacts(
                matter_id,
                &document,
                &text,
                &text_sha256,
                &source_context,
                &index_run_id_value,
                &pages,
                &text_chunks,
                &evidence_spans,
                &entity_mentions,
                &search_index_records,
            )
            .await?;
        document.extracted_text = Some(text.clone());
        document.processing_status = "processed".to_string();
        document.summary = summarize_text(&text);
        document.facts_extracted = proposed_facts.len() as u64;
        document.source_spans = source_spans.clone();
        let document = self
            .merge_node(matter_id, document_spec(), document_id, &document)
            .await?;

        for span in &source_spans {
            self.merge_source_span(matter_id, span).await?;
        }

        for chunk in &chunks {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (d:CaseDocument {document_id: $document_id})
                         MERGE (t:ExtractedText {chunk_id: $chunk_id})
                         SET t.document_id = $document_id,
                             t.matter_id = $matter_id,
                             t.page = $page,
                             t.text = $text,
                             t.document_version_id = $document_version_id,
                             t.object_blob_id = $object_blob_id,
                             t.source_span_id = $source_span_id,
                             t.byte_start = $byte_start,
                             t.byte_end = $byte_end,
                             t.char_start = $char_start,
                             t.char_end = $char_end
                         MERGE (d)-[:HAS_EXTRACTED_TEXT]->(t)
                         WITH d, t
                         OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                         OPTIONAL MATCH (s:SourceSpan {source_span_id: $source_span_id})
                         FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                           MERGE (v)-[:HAS_CHUNK]->(t)
                         )
                         FOREACH (_ IN CASE WHEN s IS NULL THEN [] ELSE [1] END |
                           MERGE (s)-[:QUOTES]->(t)
                         )",
                    )
                    .param("document_id", document_id)
                    .param("matter_id", matter_id)
                    .param("chunk_id", chunk.chunk_id.clone())
                    .param("page", chunk.page as i64)
                    .param("text", chunk.text.clone())
                    .param(
                        "document_version_id",
                        chunk.document_version_id.clone().unwrap_or_default(),
                    )
                    .param("object_blob_id", chunk.object_blob_id.clone().unwrap_or_default())
                    .param("source_span_id", chunk.source_span_id.clone().unwrap_or_default())
                    .param("byte_start", chunk.byte_start.unwrap_or_default() as i64)
                    .param("byte_end", chunk.byte_end.unwrap_or_default() as i64)
                    .param("char_start", chunk.char_start.unwrap_or_default() as i64)
                    .param("char_end", chunk.char_end.unwrap_or_default() as i64),
                )
                .await?;
        }

        let mut stored_facts = Vec::with_capacity(proposed_facts.len());
        for fact in proposed_facts {
            let fact = self
                .merge_node(matter_id, fact_spec(), &fact.fact_id, &fact)
                .await?;
            self.materialize_fact_edges(&fact).await?;
            stored_facts.push(fact);
        }
        let mut produced_ids = produced_node_ids(&chunks, &source_spans, &stored_facts);
        extend_index_node_ids(
            &mut produced_ids,
            &pages,
            &text_chunks,
            &evidence_spans,
            &entity_mentions,
            &search_index_records,
            Some(&artifact_manifest),
            &index_artifacts,
        );
        push_unique(
            &mut produced_ids,
            timeline_agent_outcome.run.agent_run_id.clone(),
        );
        for suggestion in &timeline_suggestions {
            push_unique(&mut produced_ids, suggestion.suggestion_id.clone());
        }
        let mut produced_object_keys = provenance
            .as_ref()
            .map(|provenance| provenance.ingestion_run.produced_object_keys.clone())
            .unwrap_or_default();
        for artifact in &index_artifacts {
            push_unique(&mut produced_object_keys, artifact.storage_key.clone());
        }
        let index_run = provenance.as_ref().map(|provenance| {
            completed_index_run(
                matter_id,
                document_id,
                provenance,
                &index_run_id_value,
                produced_ids.clone(),
                produced_object_keys.clone(),
            )
        });
        if let Some(run) = &index_run {
            self.merge_index_run(matter_id, run).await?;
        }
        self.merge_extraction_artifact_manifest(matter_id, &artifact_manifest)
            .await?;
        for page in &pages {
            self.merge_page(matter_id, page).await?;
        }
        for chunk in &text_chunks {
            self.merge_text_chunk(matter_id, chunk).await?;
        }
        for span in &evidence_spans {
            self.merge_evidence_span(matter_id, span).await?;
        }
        for mention in &entity_mentions {
            self.merge_entity_mention(matter_id, mention).await?;
        }
        for record in &search_index_records {
            self.merge_search_index_record(matter_id, record).await?;
        }
        let timeline_agent_outcome = self
            .persist_timeline_agent_outcome(matter_id, &timeline_agent_outcome)
            .await?;
        let stored_timeline_suggestions = timeline_agent_outcome.suggestions;
        let ingestion_run = provenance.as_ref().map(|provenance| {
            completed_ingestion_run_with_objects(
                &provenance.ingestion_run,
                "review_ready",
                "review_ready",
                produced_ids,
                produced_object_keys,
            )
        });
        if let Some(run) = &ingestion_run {
            self.merge_ingestion_run(matter_id, run).await?;
        }

        Ok(DocumentExtractionResponse {
            enabled: true,
            mode: "deterministic".to_string(),
            status: "processed".to_string(),
            message: "Extracted text chunks and proposed reviewable facts. AI fact extraction is provider-gated in V0.".to_string(),
            document,
            chunks,
            proposed_facts: stored_facts,
            ingestion_run,
            index_run,
            document_version: provenance.map(|provenance| provenance.document_version),
            index_artifacts,
            artifact_manifest: Some(artifact_manifest),
            pages,
            text_chunks,
            evidence_spans,
            entity_mentions,
            search_index_records,
            source_spans,
            timeline_suggestions: stored_timeline_suggestions,
        })
    }

    async fn store_extraction_index_artifacts(
        &self,
        matter_id: &str,
        document: &CaseDocument,
        text: &str,
        text_sha256: &str,
        source_context: &SourceContext,
        index_run_id: &str,
        pages: &[Page],
        text_chunks: &[TextChunk],
        evidence_spans: &[EvidenceSpan],
        entity_mentions: &[EntityMention],
        search_index_records: &[SearchIndexRecord],
    ) -> ApiResult<(Vec<DocumentVersion>, ExtractionArtifactManifest)> {
        let normalized_payload = serde_json::json!({
            "schema_version": "casebuilder.text.normalized.v1",
            "matter_id": matter_id,
            "document_id": document.document_id.clone(),
            "document_version_id": source_context.document_version_id.clone(),
            "object_blob_id": source_context.object_blob_id.clone(),
            "ingestion_run_id": source_context.ingestion_run_id.clone(),
            "index_run_id": index_run_id,
            "text_sha256": text_sha256,
            "text": text,
        });
        let normalized_bytes = serde_json::to_vec(&normalized_payload)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        let normalized_artifact = self
            .store_document_artifact_version_for_creator(
                matter_id,
                document,
                Bytes::from(normalized_bytes),
                Some("application/json".to_string()),
                "normalized_text",
                "text.normalized.json",
                "json",
                false,
                "casebuilder_indexer",
            )
            .await?;

        let pages_payload = serde_json::json!({
            "schema_version": "casebuilder.pages.v1",
            "matter_id": matter_id,
            "document_id": document.document_id.clone(),
            "document_version_id": source_context.document_version_id.clone(),
            "object_blob_id": source_context.object_blob_id.clone(),
            "ingestion_run_id": source_context.ingestion_run_id.clone(),
            "index_run_id": index_run_id,
            "pages": pages,
            "text_chunks": text_chunks,
            "evidence_spans": evidence_spans,
            "entity_mentions": entity_mentions,
            "search_index_records": search_index_records,
        });
        let pages_bytes = serde_json::to_vec(&pages_payload)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        let pages_artifact = self
            .store_document_artifact_version_for_creator(
                matter_id,
                document,
                Bytes::from(pages_bytes),
                Some("application/json".to_string()),
                "index_pages",
                "pages.json",
                "json",
                false,
                "casebuilder_indexer",
            )
            .await?;

        let manifest_id = extraction_manifest_id(
            &document.document_id,
            source_context.document_version_id.as_deref(),
            text_sha256,
        );
        let mut produced_object_keys = vec![
            normalized_artifact.storage_key.clone(),
            pages_artifact.storage_key.clone(),
        ];
        let mut manifest = ExtractionArtifactManifest {
            manifest_id: manifest_id.clone(),
            id: manifest_id,
            matter_id: matter_id.to_string(),
            document_id: document.document_id.clone(),
            document_version_id: source_context.document_version_id.clone(),
            object_blob_id: source_context.object_blob_id.clone(),
            ingestion_run_id: source_context.ingestion_run_id.clone(),
            index_run_id: Some(index_run_id.to_string()),
            normalized_text_version_id: Some(normalized_artifact.document_version_id.clone()),
            pages_version_id: Some(pages_artifact.document_version_id.clone()),
            manifest_version_id: None,
            text_sha256: text_sha256.to_string(),
            pages_sha256: pages_artifact.sha256.clone(),
            manifest_sha256: None,
            page_ids: pages.iter().map(|page| page.page_id.clone()).collect(),
            text_chunk_ids: text_chunks
                .iter()
                .map(|chunk| chunk.text_chunk_id.clone())
                .collect(),
            evidence_span_ids: evidence_spans
                .iter()
                .map(|span| span.evidence_span_id.clone())
                .collect(),
            entity_mention_ids: entity_mentions
                .iter()
                .map(|mention| mention.entity_mention_id.clone())
                .collect(),
            search_index_record_ids: search_index_records
                .iter()
                .map(|record| record.search_index_record_id.clone())
                .collect(),
            produced_object_keys: produced_object_keys.clone(),
            created_at: now_string(),
        };
        let manifest_bytes =
            serde_json::to_vec(&manifest).map_err(|error| ApiError::Internal(error.to_string()))?;
        let manifest_artifact = self
            .store_document_artifact_version_for_creator(
                matter_id,
                document,
                Bytes::from(manifest_bytes),
                Some("application/json".to_string()),
                "index_manifest",
                "manifest.json",
                "json",
                false,
                "casebuilder_indexer",
            )
            .await?;
        push_unique(
            &mut produced_object_keys,
            manifest_artifact.storage_key.clone(),
        );
        manifest.manifest_version_id = Some(manifest_artifact.document_version_id.clone());
        manifest.manifest_sha256 = manifest_artifact.sha256.clone();
        manifest.produced_object_keys = produced_object_keys;

        Ok((
            vec![normalized_artifact, pages_artifact, manifest_artifact],
            manifest,
        ))
    }

    pub async fn create_download_url(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<DownloadUrlResponse> {
        let document = self.get_document(matter_id, document_id).await?;
        if document.storage_status == "deleted" {
            return Err(ApiError::NotFound(format!(
                "Document {document_id} has been deleted"
            )));
        }
        let key = document
            .storage_key
            .as_deref()
            .ok_or_else(|| ApiError::BadRequest("Document has no stored object".to_string()))?;
        let expires_at = timestamp_after(self.download_ttl_seconds);
        let presigned = self
            .object_store
            .presign_get(key, Duration::from_secs(self.download_ttl_seconds))
            .await?;
        Ok(DownloadUrlResponse {
            method: presigned.method,
            url: presigned.url,
            expires_at,
            headers: presigned.headers,
            filename: document.filename,
            mime_type: document.mime_type,
            bytes: document.bytes,
        })
    }

    pub async fn delete_document(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<DeleteDocumentResponse> {
        let mut document = self.get_document(matter_id, document_id).await?;
        if let Some(key) = document.storage_key.clone() {
            self.object_store.delete(&key).await?;
        }
        document.storage_status = "deleted".to_string();
        document.processing_status = "failed".to_string();
        document.summary =
            "Document object deleted; metadata tombstone retained for provenance.".to_string();
        document.deleted_at = Some(now_string());
        document.content_etag = None;
        document.upload_expires_at = None;
        document.extracted_text = None;
        let document = self
            .merge_node(matter_id, document_spec(), document_id, &document)
            .await?;
        Ok(DeleteDocumentResponse {
            deleted: true,
            document,
        })
    }

    pub(super) async fn store_document_artifact_version(
        &self,
        matter_id: &str,
        document: &CaseDocument,
        bytes: Bytes,
        mime_type: Option<String>,
        role: &str,
        artifact_kind: &str,
        extension: &str,
        current: bool,
    ) -> ApiResult<DocumentVersion> {
        self.store_document_artifact_version_for_creator(
            matter_id,
            document,
            bytes,
            mime_type,
            role,
            artifact_kind,
            extension,
            current,
            "casebuilder_transcription",
        )
        .await
    }

    pub(super) async fn store_document_artifact_version_for_creator(
        &self,
        matter_id: &str,
        document: &CaseDocument,
        bytes: Bytes,
        mime_type: Option<String>,
        role: &str,
        artifact_kind: &str,
        extension: &str,
        current: bool,
        created_by: &str,
    ) -> ApiResult<DocumentVersion> {
        self.ensure_upload_size(bytes.len() as u64)?;
        let now = now_string();
        let sha256 = sha256_hex(&bytes);
        let document_version_id =
            artifact_version_id(&document.document_id, artifact_kind, &sha256);
        let storage_key = document_version_object_key(
            matter_id,
            &document.document_id,
            &document_version_id,
            &sha256,
            extension,
        );
        let stored = self
            .object_store
            .put_bytes(
                &storage_key,
                bytes,
                put_options(mime_type.clone(), Some(sha256.clone())),
            )
            .await?;
        let object_blob_id = object_blob_id_for_hash(&sha256);
        let object_blob = ObjectBlob {
            object_blob_id: object_blob_id.clone(),
            id: object_blob_id.clone(),
            sha256: Some(sha256.clone()),
            size_bytes: stored.content_length,
            mime_type: stored.content_type.clone().or(mime_type.clone()),
            storage_provider: self.object_store.provider().to_string(),
            storage_bucket: stored
                .bucket
                .clone()
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            storage_key: stored.key.clone(),
            etag: stored.etag.clone(),
            storage_class: None,
            created_at: now.clone(),
            retention_state: "active".to_string(),
        };
        self.merge_object_blob(matter_id, &object_blob).await?;
        if current {
            for mut version in self
                .list_document_versions(matter_id, &document.document_id)
                .await?
                .into_iter()
                .filter(|version| version.current)
            {
                version.current = false;
                self.merge_document_version(matter_id, &version).await?;
            }
        }
        let version = DocumentVersion {
            document_version_id: document_version_id.clone(),
            id: document_version_id,
            matter_id: matter_id.to_string(),
            document_id: document.document_id.clone(),
            object_blob_id,
            role: role.to_string(),
            artifact_kind: artifact_kind.to_string(),
            source_version_id: document.current_version_id.clone(),
            created_by: created_by.to_string(),
            current,
            created_at: now,
            storage_provider: self.object_store.provider().to_string(),
            storage_bucket: stored
                .bucket
                .clone()
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            storage_key: stored.key,
            sha256: Some(sha256),
            size_bytes: stored.content_length,
            mime_type,
        };
        self.merge_document_version(matter_id, &version).await
    }
}

fn folder_from_relative_path(relative_path: Option<&str>) -> Option<String> {
    relative_path
        .and_then(|path| path.split('/').next())
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
}

fn library_path_from_upload(
    relative_path: Option<&str>,
    folder: &str,
    filename: &str,
) -> ApiResult<String> {
    let candidate = match relative_path {
        Some(path) if path.contains('/') => path.to_string(),
        Some(path) if !folder.trim().is_empty() => format!("{}/{path}", folder.trim()),
        Some(path) => path.to_string(),
        None if !folder.trim().is_empty() => format!("{}/{}", folder.trim(), filename),
        None => filename.to_string(),
    };
    normalize_document_library_path(Some(candidate), filename)
}

fn normalize_document_library_path(value: Option<String>, filename: &str) -> ApiResult<String> {
    let normalized = match normalize_upload_relative_path(value)? {
        Some(path) => Some(path),
        None => normalize_upload_relative_path(Some(filename.to_string()))?,
    };
    normalized.ok_or_else(|| {
        ApiError::BadRequest("library_path must include at least one safe path segment".to_string())
    })
}

fn folder_from_library_path(library_path: &str) -> String {
    library_path
        .split('/')
        .next()
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .unwrap_or("Uploads")
        .to_string()
}

fn normalize_optional_label(value: Option<String>, field: &str) -> ApiResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::BadRequest(format!("{field} cannot be empty")));
    }
    if trimmed.chars().any(|ch| ch == '\0' || ch.is_control()) {
        return Err(ApiError::BadRequest(format!(
            "{field} cannot contain control characters"
        )));
    }
    Ok(Some(trimmed.to_string()))
}

fn normalize_nullable_label(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn build_extraction_index_records(
    matter_id: &str,
    document_id: &str,
    chunks: &[ExtractedTextChunk],
    source_context: &SourceContext,
    index_run_id: &str,
) -> (
    Vec<Page>,
    Vec<TextChunk>,
    Vec<EvidenceSpan>,
    Vec<EntityMention>,
    Vec<SearchIndexRecord>,
) {
    let mut pages = Vec::with_capacity(chunks.len());
    let mut text_chunks = Vec::with_capacity(chunks.len());
    let mut evidence_spans = Vec::with_capacity(chunks.len());
    let mut entity_mentions = Vec::new();
    let mut search_index_records = Vec::with_capacity(chunks.len());
    let now = now_string();

    for (index, chunk) in chunks.iter().enumerate() {
        let page_id = page_id(document_id, chunk.page);
        let text_hash = sha256_hex(chunk.text.as_bytes());
        pages.push(Page {
            page_id: page_id.clone(),
            id: page_id.clone(),
            matter_id: matter_id.to_string(),
            document_id: document_id.to_string(),
            document_version_id: source_context.document_version_id.clone(),
            object_blob_id: source_context.object_blob_id.clone(),
            ingestion_run_id: source_context.ingestion_run_id.clone(),
            index_run_id: Some(index_run_id.to_string()),
            page_number: chunk.page,
            unit_type: "logical_text_page".to_string(),
            title: Some(format!("Page {}", chunk.page)),
            text_hash: Some(text_hash.clone()),
            byte_start: chunk.byte_start,
            byte_end: chunk.byte_end,
            char_start: chunk.char_start,
            char_end: chunk.char_end,
            status: "indexed".to_string(),
        });

        text_chunks.push(TextChunk {
            text_chunk_id: chunk.chunk_id.clone(),
            id: chunk.chunk_id.clone(),
            matter_id: matter_id.to_string(),
            document_id: document_id.to_string(),
            document_version_id: source_context.document_version_id.clone(),
            object_blob_id: source_context.object_blob_id.clone(),
            page_id: Some(page_id),
            source_span_id: chunk.source_span_id.clone(),
            ingestion_run_id: source_context.ingestion_run_id.clone(),
            index_run_id: Some(index_run_id.to_string()),
            ordinal: (index + 1) as u64,
            page: chunk.page,
            text_hash: text_hash.clone(),
            text_excerpt: text_excerpt(&chunk.text, 500),
            token_count: approximate_token_count(&chunk.text),
            byte_start: chunk.byte_start,
            byte_end: chunk.byte_end,
            char_start: chunk.char_start,
            char_end: chunk.char_end,
            status: "indexed".to_string(),
        });

        let evidence_span_id = evidence_span_id_for_chunk(document_id, &chunk.chunk_id);
        evidence_spans.push(EvidenceSpan {
            evidence_span_id: evidence_span_id.clone(),
            id: evidence_span_id,
            matter_id: matter_id.to_string(),
            document_id: document_id.to_string(),
            document_version_id: source_context.document_version_id.clone(),
            object_blob_id: source_context.object_blob_id.clone(),
            text_chunk_id: Some(chunk.chunk_id.clone()),
            source_span_id: chunk.source_span_id.clone(),
            ingestion_run_id: source_context.ingestion_run_id.clone(),
            index_run_id: Some(index_run_id.to_string()),
            quote_hash: text_hash,
            quote_excerpt: text_excerpt(&chunk.text, 500),
            byte_start: chunk.byte_start,
            byte_end: chunk.byte_end,
            char_start: chunk.char_start,
            char_end: chunk.char_end,
            review_status: "unreviewed".to_string(),
        });

        let search_record_id =
            search_index_record_id(document_id, &chunk.chunk_id, CASE_INDEX_VERSION);
        search_index_records.push(SearchIndexRecord {
            search_index_record_id: search_record_id.clone(),
            id: search_record_id,
            matter_id: matter_id.to_string(),
            document_id: document_id.to_string(),
            document_version_id: source_context.document_version_id.clone(),
            text_chunk_id: Some(chunk.chunk_id.clone()),
            index_run_id: Some(index_run_id.to_string()),
            index_name: "casebuilder_document_text".to_string(),
            index_type: "fulltext".to_string(),
            index_version: CASE_INDEX_VERSION.to_string(),
            status: "indexed".to_string(),
            stale: false,
            created_at: now.clone(),
            indexed_at: Some(now.clone()),
        });
        entity_mentions.extend(date_entity_mentions_for_chunk(
            matter_id,
            document_id,
            chunk,
        ));
    }

    (
        pages,
        text_chunks,
        evidence_spans,
        entity_mentions,
        search_index_records,
    )
}

fn completed_index_run(
    matter_id: &str,
    document_id: &str,
    provenance: &DocumentProvenance,
    index_run_id: &str,
    produced_node_ids: Vec<String>,
    produced_object_keys: Vec<String>,
) -> IndexRun {
    IndexRun {
        index_run_id: index_run_id.to_string(),
        id: index_run_id.to_string(),
        matter_id: matter_id.to_string(),
        document_id: document_id.to_string(),
        document_version_id: Some(provenance.document_version.document_version_id.clone()),
        object_blob_id: Some(provenance.object_blob.object_blob_id.clone()),
        ingestion_run_id: Some(provenance.ingestion_run.ingestion_run_id.clone()),
        status: "review_ready".to_string(),
        stage: "graph_persisted".to_string(),
        mode: "deterministic".to_string(),
        started_at: provenance.ingestion_run.started_at.clone(),
        completed_at: Some(now_string()),
        error_code: None,
        error_message: None,
        retryable: false,
        parser_id: provenance.ingestion_run.parser_id.clone(),
        parser_version: provenance.ingestion_run.parser_version.clone(),
        chunker_version: provenance.ingestion_run.chunker_version.clone(),
        citation_resolver_version: provenance.ingestion_run.citation_resolver_version.clone(),
        index_version: provenance.ingestion_run.index_version.clone(),
        produced_node_ids,
        produced_object_keys,
        stale: false,
    }
}

fn failed_index_run(
    matter_id: &str,
    document: &CaseDocument,
    provenance: &DocumentProvenance,
    stage: &str,
    error_code: &str,
    error_message: &str,
    retryable: bool,
) -> IndexRun {
    let seed = provenance
        .ingestion_run
        .input_sha256
        .as_deref()
        .unwrap_or(error_code);
    let index_run_id = index_run_id(
        &document.document_id,
        Some(&provenance.document_version.document_version_id),
        seed,
    );
    IndexRun {
        index_run_id: index_run_id.clone(),
        id: index_run_id,
        matter_id: matter_id.to_string(),
        document_id: document.document_id.clone(),
        document_version_id: Some(provenance.document_version.document_version_id.clone()),
        object_blob_id: Some(provenance.object_blob.object_blob_id.clone()),
        ingestion_run_id: Some(provenance.ingestion_run.ingestion_run_id.clone()),
        status: "failed".to_string(),
        stage: stage.to_string(),
        mode: "deterministic".to_string(),
        started_at: provenance.ingestion_run.started_at.clone(),
        completed_at: Some(now_string()),
        error_code: Some(error_code.to_string()),
        error_message: Some(error_message.to_string()),
        retryable,
        parser_id: provenance.ingestion_run.parser_id.clone(),
        parser_version: provenance.ingestion_run.parser_version.clone(),
        chunker_version: provenance.ingestion_run.chunker_version.clone(),
        citation_resolver_version: provenance.ingestion_run.citation_resolver_version.clone(),
        index_version: provenance.ingestion_run.index_version.clone(),
        produced_node_ids: Vec::new(),
        produced_object_keys: provenance.ingestion_run.produced_object_keys.clone(),
        stale: false,
    }
}

fn extend_index_node_ids(
    ids: &mut Vec<String>,
    pages: &[Page],
    text_chunks: &[TextChunk],
    evidence_spans: &[EvidenceSpan],
    entity_mentions: &[EntityMention],
    search_index_records: &[SearchIndexRecord],
    manifest: Option<&ExtractionArtifactManifest>,
    artifacts: &[DocumentVersion],
) {
    for page in pages {
        push_unique(ids, page.page_id.clone());
    }
    for chunk in text_chunks {
        push_unique(ids, chunk.text_chunk_id.clone());
    }
    for span in evidence_spans {
        push_unique(ids, span.evidence_span_id.clone());
    }
    for mention in entity_mentions {
        push_unique(ids, mention.entity_mention_id.clone());
    }
    for record in search_index_records {
        push_unique(ids, record.search_index_record_id.clone());
    }
    if let Some(manifest) = manifest {
        push_unique(ids, manifest.manifest_id.clone());
    }
    for artifact in artifacts {
        push_unique(ids, artifact.document_version_id.clone());
        push_unique(ids, artifact.object_blob_id.clone());
    }
}

fn document_is_indexed(document: &CaseDocument) -> bool {
    document.facts_extracted > 0
        || document
            .source_spans
            .iter()
            .any(|span| span.extraction_method != "manual_entry")
}

fn document_needs_index(document: &CaseDocument) -> bool {
    !document_is_indexed(document)
        && matches!(
            document.processing_status.as_str(),
            "queued" | "processed" | "processing" | "failed"
        )
}

fn document_can_attempt_index(document: &CaseDocument) -> bool {
    document.storage_status == "stored"
        && !matches!(
            document.processing_status.as_str(),
            "ocr_required" | "transcription_deferred" | "unsupported"
        )
}

fn index_skip_message(document: &CaseDocument) -> String {
    match document.processing_status.as_str() {
        "ocr_required" => "OCR is required before this document can be indexed.".to_string(),
        "transcription_deferred" => {
            "Transcription is required before this media file can be indexed.".to_string()
        }
        "unsupported" => {
            "This file type is not supported by the deterministic indexer.".to_string()
        }
        _ if document.storage_status != "stored" => {
            "The source object is not stored yet, so indexing cannot start.".to_string()
        }
        _ => "Document does not need indexing.".to_string(),
    }
}

fn build_matter_index_summary(
    matter_id: &str,
    documents: &[CaseDocument],
    runs: &[IngestionRun],
) -> MatterIndexSummary {
    let mut processing_counts = BTreeMap::<String, u64>::new();
    let mut storage_counts = BTreeMap::<String, u64>::new();
    let mut folders = BTreeMap::<String, MatterIndexFolderSummary>::new();
    let mut batches = BTreeMap::<String, MatterIndexUploadBatchSummary>::new();
    let mut by_hash = BTreeMap::<String, Vec<&CaseDocument>>::new();
    let mut extractable_pending_document_ids = Vec::new();

    let mut active_documents = 0_u64;
    let mut archived_documents = 0_u64;
    let mut indexed_documents = 0_u64;
    let mut pending_documents = 0_u64;
    let mut failed_documents = 0_u64;
    let mut ocr_required_documents = 0_u64;
    let mut transcription_deferred_documents = 0_u64;
    let mut unsupported_documents = 0_u64;

    for document in documents {
        if document.archived_at.is_some() {
            archived_documents += 1;
            continue;
        }
        active_documents += 1;

        *processing_counts
            .entry(document.processing_status.clone())
            .or_default() += 1;
        *storage_counts
            .entry(document.storage_status.clone())
            .or_default() += 1;

        if document_is_indexed(document) {
            indexed_documents += 1;
        }
        if document_needs_index(document) {
            pending_documents += 1;
        }
        if document.processing_status == "failed" {
            failed_documents += 1;
        }
        if document.processing_status == "ocr_required" {
            ocr_required_documents += 1;
        }
        if document.processing_status == "transcription_deferred" {
            transcription_deferred_documents += 1;
        }
        if document.processing_status == "unsupported" {
            unsupported_documents += 1;
        }
        if document_can_attempt_index(document) && document_needs_index(document) {
            extractable_pending_document_ids.push(document.document_id.clone());
        }

        let folder =
            folders
                .entry(document.folder.clone())
                .or_insert_with(|| MatterIndexFolderSummary {
                    folder: document.folder.clone(),
                    count: 0,
                    indexed: 0,
                    pending: 0,
                    failed: 0,
                });
        folder.count += 1;
        if document_is_indexed(document) {
            folder.indexed += 1;
        }
        if document_needs_index(document) {
            folder.pending += 1;
        }
        if document.processing_status == "failed" {
            folder.failed += 1;
        }

        if let Some(batch_id) = document.upload_batch_id.as_deref() {
            let batch = batches.entry(batch_id.to_string()).or_insert_with(|| {
                MatterIndexUploadBatchSummary {
                    upload_batch_id: batch_id.to_string(),
                    count: 0,
                    indexed: 0,
                    pending: 0,
                    failed: 0,
                }
            });
            batch.count += 1;
            if document_is_indexed(document) {
                batch.indexed += 1;
            }
            if document_needs_index(document) {
                batch.pending += 1;
            }
            if document.processing_status == "failed" {
                batch.failed += 1;
            }
        }

        if let Some(hash) = document.file_hash.as_deref() {
            by_hash.entry(hash.to_string()).or_default().push(document);
        }
    }

    let duplicate_groups = by_hash
        .into_iter()
        .filter_map(|(file_hash, group)| {
            (group.len() > 1).then(|| MatterIndexDuplicateGroup {
                file_hash,
                count: group.len() as u64,
                document_ids: group
                    .iter()
                    .map(|document| document.document_id.clone())
                    .collect(),
                filenames: group
                    .iter()
                    .map(|document| document.filename.clone())
                    .collect(),
            })
        })
        .collect();

    let mut recent_ingestion_runs = runs.to_vec();
    recent_ingestion_runs.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    recent_ingestion_runs.truncate(20);

    MatterIndexSummary {
        matter_id: matter_id.to_string(),
        total_documents: documents.len() as u64,
        active_documents,
        archived_documents,
        indexed_documents,
        pending_documents,
        extractable_pending_documents: extractable_pending_document_ids.len() as u64,
        failed_documents,
        ocr_required_documents,
        transcription_deferred_documents,
        unsupported_documents,
        processing_status_counts: processing_counts
            .into_iter()
            .map(|(status, count)| MatterIndexStatusCount { status, count })
            .collect(),
        storage_status_counts: storage_counts
            .into_iter()
            .map(|(status, count)| MatterIndexStatusCount { status, count })
            .collect(),
        duplicate_groups,
        folders: folders.into_values().collect(),
        upload_batches: batches.into_values().collect(),
        recent_ingestion_runs,
        extractable_pending_document_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_relative_paths_are_relative_and_private() {
        assert_eq!(
            normalize_upload_relative_path(Some("Evidence/Receipts/rent.txt".to_string()))
                .unwrap()
                .as_deref(),
            Some("Evidence/Receipts/rent.txt")
        );
        assert!(normalize_upload_relative_path(Some("../secret.txt".to_string())).is_err());
        assert!(normalize_upload_relative_path(Some("/tmp/secret.txt".to_string())).is_err());
        assert!(normalize_upload_relative_path(Some("C:/secret.txt".to_string())).is_err());
    }

    #[test]
    fn library_paths_preserve_nested_uploads_and_folder_single_files() {
        assert_eq!(
            library_path_from_upload(Some("Evidence/Receipts/rent.txt"), "Evidence", "rent.txt")
                .unwrap(),
            "Evidence/Receipts/rent.txt"
        );
        assert_eq!(
            library_path_from_upload(Some("rent.txt"), "Uploads", "rent.txt").unwrap(),
            "Uploads/rent.txt"
        );
        assert!(
            normalize_document_library_path(Some("../secret.txt".to_string()), "secret.txt")
                .is_err()
        );
    }

    #[test]
    fn matter_index_summary_counts_batches_duplicates_and_extractable_docs() {
        let docs = vec![
            test_document(
                "doc:1",
                "rent.txt",
                "processed",
                2,
                Some("sha256:one"),
                Some("batch:1"),
            ),
            test_document(
                "doc:2",
                "notice.txt",
                "processed",
                0,
                Some("sha256:dup"),
                Some("batch:1"),
            ),
            test_document(
                "doc:3",
                "notice-copy.txt",
                "queued",
                0,
                Some("sha256:dup"),
                Some("batch:1"),
            ),
            test_document(
                "doc:4",
                "scan.png",
                "ocr_required",
                0,
                Some("sha256:image"),
                None,
            ),
        ];

        let summary = build_matter_index_summary("matter:test", &docs, &[]);

        assert_eq!(summary.total_documents, 4);
        assert_eq!(summary.active_documents, 4);
        assert_eq!(summary.archived_documents, 0);
        assert_eq!(summary.indexed_documents, 1);
        assert_eq!(summary.pending_documents, 2);
        assert_eq!(summary.extractable_pending_documents, 2);
        assert_eq!(summary.ocr_required_documents, 1);
        assert_eq!(summary.duplicate_groups.len(), 1);
        assert_eq!(summary.upload_batches[0].upload_batch_id, "batch:1");
        assert_eq!(
            summary.extractable_pending_document_ids,
            vec!["doc:2".to_string(), "doc:3".to_string()]
        );
    }

    #[test]
    fn matter_index_summary_keeps_archived_documents_out_of_active_counts() {
        let mut docs = vec![
            test_document(
                "doc:1",
                "rent.txt",
                "processed",
                2,
                Some("sha256:dup"),
                Some("batch:1"),
            ),
            test_document(
                "doc:2",
                "notice.txt",
                "queued",
                0,
                Some("sha256:dup"),
                Some("batch:1"),
            ),
        ];
        docs[1].archived_at = Some("2026-05-04T00:00:00Z".to_string());
        docs[1].archived_reason = Some("Replaced by better scan".to_string());

        let summary = build_matter_index_summary("matter:test", &docs, &[]);

        assert_eq!(summary.total_documents, 2);
        assert_eq!(summary.active_documents, 1);
        assert_eq!(summary.archived_documents, 1);
        assert_eq!(summary.indexed_documents, 1);
        assert_eq!(summary.pending_documents, 0);
        assert!(summary.duplicate_groups.is_empty());
        assert_eq!(summary.upload_batches[0].count, 1);
    }

    #[test]
    fn extraction_index_records_keep_hashes_offsets_and_search_refs() {
        let context = SourceContext {
            document_version_id: Some("version:doc_1:original".to_string()),
            object_blob_id: Some("blob:sha256:abc".to_string()),
            ingestion_run_id: Some("ingestion:doc_1:primary".to_string()),
        };
        let chunks = vec![ExtractedTextChunk {
            chunk_id: "chunk:doc_1:1".to_string(),
            document_id: "doc:1".to_string(),
            page: 1,
            text: "Tenant paid April rent.".to_string(),
            document_version_id: context.document_version_id.clone(),
            object_blob_id: context.object_blob_id.clone(),
            source_span_id: Some("span:doc_1:chunk:1".to_string()),
            byte_start: Some(4),
            byte_end: Some(27),
            char_start: Some(4),
            char_end: Some(27),
        }];

        let (pages, text_chunks, evidence_spans, entity_mentions, search_records) =
            build_extraction_index_records(
                "matter:test",
                "doc:1",
                &chunks,
                &context,
                "index-run:doc_1:abc",
            );

        assert_eq!(pages[0].page_id, "page:doc_1:1");
        assert_eq!(text_chunks[0].text_chunk_id, "chunk:doc_1:1");
        assert!(text_chunks[0].text_hash.starts_with("sha256:"));
        assert_eq!(text_chunks[0].byte_start, Some(4));
        assert_eq!(
            evidence_spans[0].text_chunk_id.as_deref(),
            Some("chunk:doc_1:1")
        );
        assert_eq!(search_records[0].index_name, "casebuilder_document_text");
        assert_eq!(
            search_records[0].index_run_id.as_deref(),
            Some("index-run:doc_1:abc")
        );
        assert!(entity_mentions.is_empty());
    }

    #[test]
    fn document_index_artifact_keys_are_opaque() {
        let document_id = "doc:Evidence/Client Uploads/Tenant Notice.pdf";
        let version_id = artifact_version_id(document_id, "text.normalized.json", "sha256:abc123");
        let key = document_version_object_key(
            "matter:Blue Ox v Tenant",
            document_id,
            &version_id,
            "sha256:abc123",
            "json",
        );

        assert!(key.starts_with("casebuilder/matters/"));
        assert!(key.ends_with(".json"));
        assert!(!key.contains("Tenant Notice"));
        assert!(!key.contains("Client Uploads"));
        assert!(!key.contains("Blue Ox"));
    }

    fn test_document(
        document_id: &str,
        filename: &str,
        processing_status: &str,
        facts: u64,
        hash: Option<&str>,
        upload_batch_id: Option<&str>,
    ) -> CaseDocument {
        CaseDocument {
            document_id: document_id.to_string(),
            id: document_id.to_string(),
            matter_id: "matter:test".to_string(),
            filename: filename.to_string(),
            title: filename.to_string(),
            document_type: "evidence".to_string(),
            mime_type: Some("text/plain".to_string()),
            pages: 1,
            bytes: 10,
            file_hash: hash.map(str::to_string),
            uploaded_at: "1".to_string(),
            source: "user_upload".to_string(),
            confidentiality: "private".to_string(),
            processing_status: processing_status.to_string(),
            is_exhibit: false,
            exhibit_label: None,
            summary: String::new(),
            date_observed: None,
            parties_mentioned: Vec::new(),
            entities_mentioned: Vec::new(),
            facts_extracted: facts,
            citations_found: 0,
            contradictions_flagged: 0,
            linked_claim_ids: Vec::new(),
            folder: folder_from_relative_path(Some("Evidence/Receipts/rent.txt")).unwrap(),
            storage_path: None,
            storage_provider: "local".to_string(),
            storage_status: "stored".to_string(),
            storage_bucket: None,
            storage_key: Some(format!(
                "casebuilder/matters/m/documents/{document_id}/original.bin"
            )),
            content_etag: None,
            upload_expires_at: None,
            deleted_at: None,
            library_path: Some(format!("Evidence/{filename}")),
            archived_at: None,
            archived_reason: None,
            original_relative_path: Some(format!("Evidence/{filename}")),
            upload_batch_id: upload_batch_id.map(str::to_string),
            object_blob_id: None,
            current_version_id: None,
            ingestion_run_ids: Vec::new(),
            source_spans: Vec::new(),
            extracted_text: None,
        }
    }
}
