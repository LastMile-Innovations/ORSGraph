use super::*;

impl CaseBuilderService {
    pub(super) async fn merge_matter(&self, matter: &MatterSummary) -> ApiResult<MatterSummary> {
        let payload = to_payload(matter)?;
        self.neo4j
            .run_rows(
                query(
                    "MERGE (m:Matter {matter_id: $matter_id})
                     SET m.payload = $payload,
                         m.name = $name,
                         m.status = $status,
                         m.matter_type = $matter_type,
                         m.updated_at = $updated_at
                     RETURN m.payload AS payload",
                )
                .param("matter_id", matter.matter_id.clone())
                .param("payload", payload)
                .param("name", matter.name.clone())
                .param("status", matter.status.clone())
                .param("matter_type", matter.matter_type.clone())
                .param("updated_at", matter.updated_at.clone()),
            )
            .await?;
        Ok(matter.clone())
    }

    pub(super) async fn get_matter_summary(&self, matter_id: &str) -> ApiResult<MatterSummary> {
        let rows = self
            .neo4j
            .run_rows(
                query("MATCH (m:Matter {matter_id: $matter_id}) RETURN m.payload AS payload")
                    .param("matter_id", matter_id),
            )
            .await?;
        let payload = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
            .ok_or_else(|| ApiError::NotFound(format!("Matter {matter_id} not found")))?;
        from_payload(&payload)
    }

    pub(super) async fn require_matter(&self, matter_id: &str) -> ApiResult<()> {
        self.get_matter_summary(matter_id).await.map(|_| ())
    }

    pub(super) async fn merge_node<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        matter_id: &str,
        spec: NodeSpec,
        id: &str,
        value: &T,
    ) -> ApiResult<T> {
        let payload = to_payload(value)?;
        let statement = format!(
            "MATCH (m:Matter {{matter_id: $matter_id}})
             MERGE (n:{label} {{{id_key}: $id}})
             SET n.payload = $payload,
                 n.matter_id = $matter_id,
                 n.{id_key} = $id
             MERGE (m)-[:{edge}]->(n)
             RETURN n.payload AS payload",
            label = spec.label,
            id_key = spec.id_key,
            edge = spec.edge,
        );
        let rows = self
            .neo4j
            .run_rows(
                query(&statement)
                    .param("matter_id", matter_id)
                    .param("id", id)
                    .param("payload", payload),
            )
            .await?;
        let payload = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
            .ok_or_else(|| ApiError::Internal("CaseBuilder write returned no row".to_string()))?;
        from_payload(&payload)
    }

    pub(super) async fn get_node<T: serde::de::DeserializeOwned>(
        &self,
        matter_id: &str,
        spec: NodeSpec,
        id: &str,
    ) -> ApiResult<T> {
        let statement = format!(
            "MATCH (:Matter {{matter_id: $matter_id}})-[:{edge}]->(n:{label} {{{id_key}: $id}})
             RETURN n.payload AS payload",
            label = spec.label,
            id_key = spec.id_key,
            edge = spec.edge,
        );
        let rows = self
            .neo4j
            .run_rows(
                query(&statement)
                    .param("matter_id", matter_id)
                    .param("id", id),
            )
            .await?;
        let payload = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
            .ok_or_else(|| ApiError::NotFound(format!("{} {id} not found", spec.label)))?;
        from_payload(&payload)
    }

    pub(super) async fn list_nodes<T: serde::de::DeserializeOwned>(
        &self,
        matter_id: &str,
        spec: NodeSpec,
    ) -> ApiResult<Vec<T>> {
        let statement = format!(
            "MATCH (:Matter {{matter_id: $matter_id}})-[:{edge}]->(n:{label})
             RETURN n.payload AS payload
             ORDER BY coalesce(n.uploaded_at, n.updated_at, n.created_at, n.{id_key})",
            label = spec.label,
            id_key = spec.id_key,
            edge = spec.edge,
        );
        let rows = self
            .neo4j
            .run_rows(query(&statement).param("matter_id", matter_id))
            .await?;
        rows.into_iter()
            .map(|row| {
                let payload = row
                    .get::<String>("payload")
                    .map_err(|error| ApiError::Internal(error.to_string()))?;
                from_payload(&payload)
            })
            .collect()
    }

    pub(super) async fn persist_document_provenance(
        &self,
        matter_id: &str,
        provenance: &DocumentProvenance,
    ) -> ApiResult<()> {
        self.merge_object_blob(matter_id, &provenance.object_blob)
            .await?;
        self.merge_document_version(matter_id, &provenance.document_version)
            .await?;
        self.merge_ingestion_run(matter_id, &provenance.ingestion_run)
            .await?;
        Ok(())
    }

    pub(super) async fn ensure_document_original_provenance(
        &self,
        matter_id: &str,
        document: &mut CaseDocument,
    ) -> ApiResult<Option<DocumentProvenance>> {
        let Some(key) = document.storage_key.clone() else {
            return Ok(None);
        };
        if document.storage_status == "deleted" {
            return Ok(None);
        }
        let object = StoredObject {
            bucket: document
                .storage_bucket
                .clone()
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            key,
            content_length: document.bytes,
            etag: document.content_etag.clone(),
            content_type: document.mime_type.clone(),
            metadata: document
                .file_hash
                .as_ref()
                .map(|hash| BTreeMap::from([("sha256".to_string(), hash.clone())]))
                .unwrap_or_default(),
            local_path: document.storage_path.clone(),
        };
        let provenance = build_original_provenance(matter_id, document, &object, "stored");
        apply_document_provenance(document, &provenance);
        self.persist_document_provenance(matter_id, &provenance)
            .await?;
        Ok(Some(provenance))
    }

    pub(super) async fn current_document_version(
        &self,
        matter_id: &str,
        document: &CaseDocument,
    ) -> ApiResult<Option<DocumentVersion>> {
        if let Some(version_id) = document.current_version_id.as_deref() {
            match self
                .get_node::<DocumentVersion>(matter_id, document_version_spec(), version_id)
                .await
            {
                Ok(version) => return Ok(Some(version)),
                Err(ApiError::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(self
            .list_document_versions(matter_id, &document.document_id)
            .await?
            .into_iter()
            .find(|version| version.current))
    }

    pub(super) async fn list_document_versions(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<Vec<DocumentVersion>> {
        let mut versions = self
            .list_nodes::<DocumentVersion>(matter_id, document_version_spec())
            .await?
            .into_iter()
            .filter(|version| version.document_id == document_id)
            .collect::<Vec<_>>();
        versions.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(versions)
    }

    pub(super) async fn optional_document_version(
        &self,
        matter_id: &str,
        version_id: &Option<String>,
    ) -> ApiResult<Option<DocumentVersion>> {
        let Some(version_id) = version_id.as_deref() else {
            return Ok(None);
        };
        match self
            .get_node::<DocumentVersion>(matter_id, document_version_spec(), version_id)
            .await
        {
            Ok(version) => Ok(Some(version)),
            Err(ApiError::NotFound(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub(super) async fn merge_document_annotation(
        &self,
        matter_id: &str,
        annotation: &DocumentAnnotation,
    ) -> ApiResult<DocumentAnnotation> {
        let annotation = self
            .merge_node(
                matter_id,
                document_annotation_spec(),
                &annotation.annotation_id,
                annotation,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (a:DocumentAnnotation {annotation_id: $annotation_id})
                     SET a.document_id = $document_id,
                         a.status = $status
                     MERGE (d)-[:HAS_ANNOTATION]->(a)",
                )
                .param("document_id", annotation.document_id.clone())
                .param("annotation_id", annotation.annotation_id.clone())
                .param("status", annotation.status.clone()),
            )
            .await?;
        Ok(annotation)
    }

    pub(super) async fn validate_document_annotation_target(
        &self,
        matter_id: &str,
        request: &UpsertDocumentAnnotationRequest,
    ) -> ApiResult<()> {
        match (
            request
                .target_type
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
            request
                .target_id
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
        ) {
            (None, None) => Ok(()),
            (Some(_), None) | (None, Some(_)) => Err(ApiError::BadRequest(
                "Annotation target_type and target_id must be provided together.".to_string(),
            )),
            (Some(target_type), Some(target_id)) => match target_type {
                "fact" => self.require_fact(matter_id, target_id).await,
                "evidence" => self.require_evidence(matter_id, target_id).await,
                "document" | "case_document" => self.require_document(matter_id, target_id).await,
                "source_span" | "text_span" | "document_page" => {
                    self.require_source_span(matter_id, target_id).await
                }
                _ => Err(ApiError::BadRequest(
                    "Unsupported document annotation target_type.".to_string(),
                )),
            },
        }
    }

    pub(super) async fn store_edited_document_bytes(
        &self,
        matter_id: &str,
        mut document: CaseDocument,
        bytes: Bytes,
        mime_type: Option<String>,
        extension: &str,
        stage: &str,
    ) -> ApiResult<(CaseDocument, DocumentVersion, IngestionRun)> {
        self.ensure_upload_size(bytes.len() as u64)?;
        let now = now_string();
        let sha256 = sha256_hex(&bytes);
        let prior_version_id = document.current_version_id.clone();
        let document_version_id = edited_version_id(&document.document_id, &sha256, now_secs());
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
                bytes.clone(),
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
        let parser = parse_document_bytes(&document.filename, mime_type.as_deref(), &bytes);
        let document_version = DocumentVersion {
            document_version_id: document_version_id.clone(),
            id: document_version_id.clone(),
            matter_id: matter_id.to_string(),
            document_id: document.document_id.clone(),
            object_blob_id: object_blob_id.clone(),
            role: "edited_source".to_string(),
            artifact_kind: "document_package".to_string(),
            source_version_id: prior_version_id.clone(),
            created_by: "user".to_string(),
            current: true,
            created_at: now.clone(),
            storage_provider: self.object_store.provider().to_string(),
            storage_bucket: stored
                .bucket
                .clone()
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            storage_key: stored.key.clone(),
            sha256: Some(sha256.clone()),
            size_bytes: stored.content_length,
            mime_type: mime_type.clone(),
        };
        let ingestion_run_id = edited_ingestion_run_id(&document.document_id, &document_version_id);
        let ingestion_run = IngestionRun {
            ingestion_run_id: ingestion_run_id.clone(),
            id: ingestion_run_id,
            matter_id: matter_id.to_string(),
            document_id: document.document_id.clone(),
            document_version_id: Some(document_version_id.clone()),
            object_blob_id: Some(object_blob_id.clone()),
            input_sha256: Some(sha256.clone()),
            status: parser.status.clone(),
            stage: stage.to_string(),
            mode: "document_workspace".to_string(),
            started_at: now.clone(),
            completed_at: Some(now.clone()),
            error_code: None,
            error_message: None,
            retryable: false,
            produced_node_ids: vec![document_version_id.clone(), object_blob_id.clone()],
            produced_object_keys: vec![stored.key.clone()],
            parser_id: Some(parser.parser_id),
            parser_version: Some(PARSER_REGISTRY_VERSION.to_string()),
            chunker_version: Some(CHUNKER_VERSION.to_string()),
            citation_resolver_version: Some(CITATION_RESOLVER_VERSION.to_string()),
            index_version: Some(CASE_INDEX_VERSION.to_string()),
        };

        for mut version in self
            .list_document_versions(matter_id, &document.document_id)
            .await?
            .into_iter()
            .filter(|version| version.current)
        {
            if version.document_version_id != document_version_id {
                version.current = false;
                self.merge_document_version(matter_id, &version).await?;
            }
        }

        document.bytes = stored.content_length;
        document.file_hash = Some(sha256);
        document.mime_type = mime_type;
        document.storage_provider = self.object_store.provider().to_string();
        document.storage_status = "stored".to_string();
        document.storage_bucket = stored
            .bucket
            .clone()
            .or_else(|| self.object_store.bucket().map(str::to_string));
        document.storage_key = Some(stored.key);
        document.storage_path = stored.local_path;
        document.content_etag = stored.etag;
        document.upload_expires_at = None;
        document.object_blob_id = Some(object_blob_id);
        document.current_version_id = Some(document_version_id);
        document.processing_status = parser.status;
        document.summary = parser.message;
        document.extracted_text = parser.text;
        push_unique(
            &mut document.ingestion_run_ids,
            ingestion_run.ingestion_run_id.clone(),
        );

        self.merge_object_blob(matter_id, &object_blob).await?;
        let document = self
            .merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await?;
        let document_version = self
            .merge_document_version(matter_id, &document_version)
            .await?;
        let ingestion_run = self.merge_ingestion_run(matter_id, &ingestion_run).await?;
        Ok((document, document_version, ingestion_run))
    }

    pub(super) async fn merge_object_blob(
        &self,
        matter_id: &str,
        blob: &ObjectBlob,
    ) -> ApiResult<ObjectBlob> {
        let payload = to_payload(blob)?;
        let rows = self
            .neo4j
            .run_rows(
                query(
                    "MATCH (m:Matter {matter_id: $matter_id})
                     MERGE (b:ObjectBlob {object_blob_id: $object_blob_id})
                     ON CREATE SET b.created_at = $created_at
                     SET b.payload = $payload,
                         b.object_blob_id = $object_blob_id,
                         b.sha256 = $sha256,
                         b.storage_provider = $storage_provider,
                         b.storage_bucket = $storage_bucket,
                         b.storage_key = $storage_key,
                         b.size_bytes = $size_bytes,
                         b.retention_state = $retention_state
                     MERGE (m)-[:USES_OBJECT_BLOB]->(b)
                     RETURN b.payload AS payload",
                )
                .param("matter_id", matter_id)
                .param("object_blob_id", blob.object_blob_id.clone())
                .param("payload", payload)
                .param("created_at", blob.created_at.clone())
                .param("sha256", blob.sha256.clone().unwrap_or_default())
                .param("storage_provider", blob.storage_provider.clone())
                .param(
                    "storage_bucket",
                    blob.storage_bucket.clone().unwrap_or_default(),
                )
                .param("storage_key", blob.storage_key.clone())
                .param("size_bytes", blob.size_bytes as i64)
                .param("retention_state", blob.retention_state.clone()),
            )
            .await?;
        let payload = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
            .ok_or_else(|| ApiError::Internal("ObjectBlob write returned no row".to_string()))?;
        from_payload(&payload)
    }

    pub(super) async fn get_object_blob(
        &self,
        matter_id: &str,
        object_blob_id: &str,
    ) -> ApiResult<ObjectBlob> {
        let rows = self
            .neo4j
            .run_rows(
                query(
                    "MATCH (:Matter {matter_id: $matter_id})-[:USES_OBJECT_BLOB]->(b:ObjectBlob {object_blob_id: $object_blob_id})
                     RETURN b.payload AS payload",
                )
                .param("matter_id", matter_id)
                .param("object_blob_id", object_blob_id),
            )
            .await?;
        let payload = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
            .ok_or_else(|| ApiError::NotFound("ObjectBlob not found".to_string()))?;
        from_payload(&payload)
    }

    pub(super) async fn store_casebuilder_bytes(
        &self,
        matter_id: &str,
        key: &str,
        bytes: Bytes,
        content_type: &str,
    ) -> ApiResult<ObjectBlob> {
        let sha256 = sha256_hex(&bytes);
        let stored = self
            .object_store
            .put_bytes(
                key,
                bytes.clone(),
                put_options(Some(content_type.to_string()), Some(sha256.clone())),
            )
            .await?;
        let now = now_string();
        let blob = ObjectBlob {
            object_blob_id: object_blob_id_for_hash(&sha256),
            id: object_blob_id_for_hash(&sha256),
            sha256: Some(sha256),
            size_bytes: stored.content_length,
            mime_type: stored.content_type.clone(),
            storage_provider: self.object_store.provider().to_string(),
            storage_bucket: stored
                .bucket
                .clone()
                .or_else(|| self.object_store.bucket().map(str::to_string)),
            storage_key: stored.key,
            etag: stored.etag,
            storage_class: None,
            created_at: now,
            retention_state: "active".to_string(),
        };
        self.merge_object_blob(matter_id, &blob).await
    }

    pub(super) async fn load_json_blob<T: serde::de::DeserializeOwned>(
        &self,
        matter_id: &str,
        object_blob_id: &str,
    ) -> ApiResult<T> {
        let blob = self.get_object_blob(matter_id, object_blob_id).await?;
        let bytes = self.object_store.get_bytes(&blob.storage_key).await?;
        serde_json::from_slice(&bytes).map_err(|error| ApiError::Internal(error.to_string()))
    }

    pub(super) async fn validate_ast_patch_matter_references(
        &self,
        matter_id: &str,
        product: &WorkProduct,
        patch: &AstPatch,
    ) -> ApiResult<()> {
        for operation in &patch.operations {
            match operation {
                AstOperation::InsertBlock { block, .. } => {
                    self.validate_ast_block_payload_references(matter_id, product, block)
                        .await?;
                }
                AstOperation::AddLink { link } => {
                    self.validate_work_product_link_target(
                        matter_id,
                        &link.target_type,
                        &link.target_id,
                    )
                    .await?;
                }
                AstOperation::AddCitation { citation } => {
                    self.validate_citation_target_reference(
                        matter_id,
                        citation.target_type.as_str(),
                        citation.target_id.as_deref(),
                    )
                    .await?;
                }
                AstOperation::ResolveCitation {
                    target_type,
                    target_id,
                    ..
                } => {
                    if let Some(target_id) = target_id.as_deref() {
                        self.validate_citation_target_reference(
                            matter_id,
                            target_type.as_deref().unwrap_or("legal_authority"),
                            Some(target_id),
                        )
                        .await?;
                    }
                }
                AstOperation::AddExhibitReference { exhibit } => {
                    self.validate_exhibit_reference_targets(matter_id, exhibit)
                        .await?;
                }
                AstOperation::ResolveExhibitReference { exhibit_id, .. } => {
                    if let Some(exhibit_id) = exhibit_id.as_deref() {
                        self.require_evidence_or_document(matter_id, exhibit_id)
                            .await?;
                    }
                }
                AstOperation::AddRuleFinding { finding } => {
                    if finding.matter_id != matter_id {
                        return Err(ApiError::BadRequest(
                            "AST rule finding matter does not match route matter.".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) async fn validate_work_product_matter_references(
        &self,
        matter_id: &str,
        product: &WorkProduct,
    ) -> ApiResult<()> {
        if product.matter_id != matter_id || product.document_ast.matter_id != matter_id {
            return Err(ApiError::BadRequest(
                "Work product matter does not match route matter.".to_string(),
            ));
        }
        let blocks = flatten_work_product_blocks(&product.document_ast.blocks);
        let block_ids = blocks
            .iter()
            .map(|block| block.block_id.clone())
            .collect::<HashSet<_>>();
        for block in &blocks {
            if block.matter_id != matter_id || block.work_product_id != product.work_product_id {
                return Err(ApiError::BadRequest(
                    "AST block ownership does not match work product.".to_string(),
                ));
            }
            for fact_id in &block.fact_ids {
                self.require_fact(matter_id, fact_id).await?;
            }
            for evidence_id in &block.evidence_ids {
                self.require_evidence_document_or_span(matter_id, evidence_id)
                    .await?;
            }
        }
        for link in &product.document_ast.links {
            if !block_ids.contains(&link.source_block_id) {
                return Err(ApiError::NotFound(
                    "AST link source block not found".to_string(),
                ));
            }
            self.validate_work_product_link_target(matter_id, &link.target_type, &link.target_id)
                .await?;
        }
        for citation in &product.document_ast.citations {
            if !block_ids.contains(&citation.source_block_id) {
                return Err(ApiError::NotFound(
                    "AST citation source block not found".to_string(),
                ));
            }
            self.validate_citation_target_reference(
                matter_id,
                &citation.target_type,
                citation.target_id.as_deref(),
            )
            .await?;
        }
        for exhibit in &product.document_ast.exhibits {
            if !block_ids.contains(&exhibit.source_block_id) {
                return Err(ApiError::NotFound(
                    "AST exhibit source block not found".to_string(),
                ));
            }
            self.validate_exhibit_reference_targets(matter_id, exhibit)
                .await?;
        }
        for finding in &product.document_ast.rule_findings {
            if finding.matter_id != matter_id || finding.work_product_id != product.work_product_id
            {
                return Err(ApiError::BadRequest(
                    "AST rule finding ownership does not match work product.".to_string(),
                ));
            }
            if matches!(
                finding.target_type.as_str(),
                "block" | "paragraph" | "section"
            ) && !block_ids.contains(&finding.target_id)
            {
                return Err(ApiError::NotFound(
                    "AST rule finding target block not found".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub(super) async fn validate_ast_block_payload_references(
        &self,
        matter_id: &str,
        product: &WorkProduct,
        block: &WorkProductBlock,
    ) -> ApiResult<()> {
        let mut stack = vec![block];
        while let Some(block) = stack.pop() {
            if (!block.matter_id.is_empty() && block.matter_id != matter_id)
                || (!block.work_product_id.is_empty()
                    && block.work_product_id != product.work_product_id)
            {
                return Err(ApiError::BadRequest(
                    "AST block ownership does not match work product.".to_string(),
                ));
            }
            for fact_id in &block.fact_ids {
                self.require_fact(matter_id, fact_id).await?;
            }
            for evidence_id in &block.evidence_ids {
                self.require_evidence_document_or_span(matter_id, evidence_id)
                    .await?;
            }
            for child in &block.children {
                stack.push(child);
            }
        }
        Ok(())
    }

    pub(super) async fn validate_complaint_link_references(
        &self,
        matter_id: &str,
        request: &ComplaintLinkRequest,
    ) -> ApiResult<()> {
        if let Some(fact_id) = request.fact_id.as_deref() {
            self.require_fact(matter_id, fact_id).await?;
        }
        if let Some(evidence_id) = request.evidence_id.as_deref() {
            self.require_evidence(matter_id, evidence_id).await?;
        }
        if let Some(document_id) = request.document_id.as_deref() {
            self.require_document(matter_id, document_id).await?;
        }
        if let Some(source_span_id) = request.source_span_id.as_deref() {
            self.require_source_span(matter_id, source_span_id).await?;
        }
        Ok(())
    }

    pub(super) async fn validate_work_product_link_target(
        &self,
        matter_id: &str,
        target_type: &str,
        target_id: &str,
    ) -> ApiResult<()> {
        match target_type {
            "fact" => self.require_fact(matter_id, target_id).await,
            "evidence" => self.require_evidence(matter_id, target_id).await,
            "document" | "case_document" => self.require_document(matter_id, target_id).await,
            "source_span" | "text_span" | "document_page" => {
                self.require_source_span(matter_id, target_id).await
            }
            "exhibit" => {
                self.require_evidence_or_document(matter_id, target_id)
                    .await
            }
            "authority"
            | "legal_authority"
            | "provision"
            | "legal_text_identity"
            | "legal_text" => Ok(()),
            _ => Err(ApiError::BadRequest(
                "Unsupported support target_type.".to_string(),
            )),
        }
    }

    pub(super) async fn validate_citation_target_reference(
        &self,
        matter_id: &str,
        target_type: &str,
        target_id: Option<&str>,
    ) -> ApiResult<()> {
        let Some(target_id) = target_id else {
            return Ok(());
        };
        match target_type {
            "fact" => self.require_fact(matter_id, target_id).await,
            "evidence" => self.require_evidence(matter_id, target_id).await,
            "document" | "case_document" => self.require_document(matter_id, target_id).await,
            "source_span" | "text_span" | "document_page" => {
                self.require_source_span(matter_id, target_id).await
            }
            _ => Ok(()),
        }
    }

    pub(super) async fn validate_exhibit_reference_targets(
        &self,
        matter_id: &str,
        exhibit: &WorkProductExhibitReference,
    ) -> ApiResult<()> {
        if let Some(document_id) = exhibit.document_id.as_deref() {
            self.require_document(matter_id, document_id).await?;
        }
        if let Some(exhibit_id) = exhibit.exhibit_id.as_deref() {
            self.require_evidence_or_document(matter_id, exhibit_id)
                .await?;
        }
        Ok(())
    }

    pub(super) async fn require_fact(&self, matter_id: &str, fact_id: &str) -> ApiResult<()> {
        self.get_node::<CaseFact>(matter_id, fact_spec(), fact_id)
            .await
            .map(|_| ())
            .map_err(|error| matter_reference_error(error, "fact"))
    }

    pub(super) async fn require_evidence(
        &self,
        matter_id: &str,
        evidence_id: &str,
    ) -> ApiResult<()> {
        self.get_node::<CaseEvidence>(matter_id, evidence_spec(), evidence_id)
            .await
            .map(|_| ())
            .map_err(|error| matter_reference_error(error, "evidence"))
    }

    pub(super) async fn require_document(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<()> {
        self.get_node::<CaseDocument>(matter_id, document_spec(), document_id)
            .await
            .map(|_| ())
            .map_err(|error| matter_reference_error(error, "document"))
    }

    pub(super) async fn require_source_span(
        &self,
        matter_id: &str,
        source_span_id: &str,
    ) -> ApiResult<()> {
        self.get_node::<SourceSpan>(matter_id, source_span_spec(), source_span_id)
            .await
            .map(|_| ())
            .map_err(|error| matter_reference_error(error, "source_span"))
    }

    pub(super) async fn require_evidence_or_document(
        &self,
        matter_id: &str,
        target_id: &str,
    ) -> ApiResult<()> {
        match self.require_evidence(matter_id, target_id).await {
            Ok(()) => Ok(()),
            Err(ApiError::NotFound(_)) => self.require_document(matter_id, target_id).await,
            Err(error) => Err(error),
        }
    }

    pub(super) async fn require_evidence_document_or_span(
        &self,
        matter_id: &str,
        target_id: &str,
    ) -> ApiResult<()> {
        match self.require_evidence(matter_id, target_id).await {
            Ok(()) => Ok(()),
            Err(ApiError::NotFound(_)) => match self.require_document(matter_id, target_id).await {
                Ok(()) => Ok(()),
                Err(ApiError::NotFound(_)) => self.require_source_span(matter_id, target_id).await,
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        }
    }

    pub(super) async fn merge_document_version(
        &self,
        matter_id: &str,
        version: &DocumentVersion,
    ) -> ApiResult<DocumentVersion> {
        let version = self
            .merge_node(
                matter_id,
                document_version_spec(),
                &version.document_version_id,
                version,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     MERGE (d)-[:HAS_VERSION]->(v)
                     MERGE (v)-[:VERSION_OF]->(d)
                     MERGE (v)-[:STORED_AS]->(b)
                     MERGE (v)-[:DERIVED_FROM]->(b)",
                )
                .param("document_id", version.document_id.clone())
                .param("document_version_id", version.document_version_id.clone())
                .param("object_blob_id", version.object_blob_id.clone()),
            )
            .await?;
        Ok(version)
    }

    pub(super) async fn merge_ingestion_run(
        &self,
        matter_id: &str,
        run: &IngestionRun,
    ) -> ApiResult<IngestionRun> {
        let run = self
            .merge_node(matter_id, ingestion_run_spec(), &run.ingestion_run_id, run)
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (r:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     MERGE (d)-[:HAS_INGESTION_RUN]->(r)
                     WITH d, r
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:INDEXED]->(v)
                     )
                     FOREACH (_ IN CASE WHEN b IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:DERIVED_FROM]->(b)
                     )",
                )
                .param("document_id", run.document_id.clone())
                .param("ingestion_run_id", run.ingestion_run_id.clone())
                .param(
                    "document_version_id",
                    run.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "object_blob_id",
                    run.object_blob_id.clone().unwrap_or_default(),
                ),
            )
            .await?;
        Ok(run)
    }

    pub(super) async fn merge_source_span(
        &self,
        matter_id: &str,
        span: &SourceSpan,
    ) -> ApiResult<SourceSpan> {
        let span = self
            .merge_node(matter_id, source_span_spec(), &span.source_span_id, span)
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (s:SourceSpan {source_span_id: $source_span_id})
                     MERGE (d)-[:HAS_SOURCE_SPAN]->(s)
                     WITH d, s
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     OPTIONAL MATCH (r:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (s)-[:FROM_VERSION]->(v)
                     )
                     FOREACH (_ IN CASE WHEN b IS NULL THEN [] ELSE [1] END |
                       MERGE (s)-[:DERIVED_FROM]->(b)
                     )
                     FOREACH (_ IN CASE WHEN r IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:PRODUCED]->(s)
                     )",
                )
                .param("document_id", span.document_id.clone())
                .param("source_span_id", span.source_span_id.clone())
                .param(
                    "document_version_id",
                    span.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "object_blob_id",
                    span.object_blob_id.clone().unwrap_or_default(),
                )
                .param(
                    "ingestion_run_id",
                    span.ingestion_run_id.clone().unwrap_or_default(),
                ),
            )
            .await?;
        Ok(span)
    }
}
