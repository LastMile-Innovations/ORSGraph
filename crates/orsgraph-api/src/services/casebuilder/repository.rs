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
                         m.owner_subject = $owner_subject,
                         m.owner_email = $owner_email,
                         m.owner_name = $owner_name,
                         m.created_by_subject = $created_by_subject,
                         m.updated_at = $updated_at
                     RETURN m.payload AS payload",
                )
                .param("matter_id", matter.matter_id.clone())
                .param("payload", payload)
                .param("name", matter.name.clone())
                .param("status", matter.status.clone())
                .param("matter_type", matter.matter_type.clone())
                .param("owner_subject", matter.owner_subject.clone())
                .param("owner_email", matter.owner_email.clone())
                .param("owner_name", matter.owner_name.clone())
                .param("created_by_subject", matter.created_by_subject.clone())
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

    pub(super) async fn merge_index_run(
        &self,
        matter_id: &str,
        run: &IndexRun,
    ) -> ApiResult<IndexRun> {
        let run = self
            .merge_node(matter_id, index_run_spec(), &run.index_run_id, run)
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (r:IndexRun {index_run_id: $index_run_id})
                     SET r.document_id = $document_id,
                         r.status = $status,
                         r.stage = $stage,
                         r.stale = $stale
                     MERGE (d)-[:HAS_INDEX_RUN]->(r)
                     WITH d, r
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     OPTIONAL MATCH (i:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:INDEXED]->(v)
                     )
                     FOREACH (_ IN CASE WHEN b IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:DERIVED_FROM]->(b)
                     )
                     FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
                       MERGE (i)-[:SPAWNED_INDEX_RUN]->(r)
                     )",
                )
                .param("document_id", run.document_id.clone())
                .param("index_run_id", run.index_run_id.clone())
                .param(
                    "document_version_id",
                    run.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "object_blob_id",
                    run.object_blob_id.clone().unwrap_or_default(),
                )
                .param(
                    "ingestion_run_id",
                    run.ingestion_run_id.clone().unwrap_or_default(),
                )
                .param("status", run.status.clone())
                .param("stage", run.stage.clone())
                .param("stale", run.stale),
            )
            .await?;
        Ok(run)
    }

    pub(super) async fn merge_timeline_agent_run(
        &self,
        matter_id: &str,
        run: &TimelineAgentRun,
    ) -> ApiResult<TimelineAgentRun> {
        let run = self
            .merge_node(matter_id, timeline_agent_run_spec(), &run.agent_run_id, run)
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (a:TimelineAgentRun {agent_run_id: $agent_run_id})
                     SET a.matter_id = $matter_id,
                         a.status = $status,
                         a.provider_mode = $provider_mode,
                         a.provider = $provider,
                         a.scope_type = $scope_type,
                         a.created_at = $created_at,
                         a.started_at = $started_at,
                         a.completed_at = $completed_at",
                )
                .param("agent_run_id", run.agent_run_id.clone())
                .param("matter_id", matter_id)
                .param("status", run.status.clone())
                .param("provider_mode", run.provider_mode.clone())
                .param("provider", run.provider.clone())
                .param("scope_type", run.scope_type.clone())
                .param("created_at", run.created_at.clone())
                .param("started_at", run.started_at.clone().unwrap_or_default())
                .param("completed_at", run.completed_at.clone().unwrap_or_default()),
            )
            .await?;
        Ok(run)
    }

    pub(super) async fn merge_timeline_suggestion(
        &self,
        matter_id: &str,
        suggestion: &TimelineSuggestion,
    ) -> ApiResult<TimelineSuggestion> {
        let suggestion = self
            .merge_node(
                matter_id,
                timeline_suggestion_spec(),
                &suggestion.suggestion_id,
                suggestion,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (s:TimelineSuggestion {suggestion_id: $suggestion_id})
                     SET s.matter_id = $matter_id,
                         s.status = $status,
                         s.date = $date,
                         s.source_document_id = $source_document_id
                     OPTIONAL MATCH (d:CaseDocument {document_id: $source_document_id})
                     OPTIONAL MATCH (w:WorkProduct {work_product_id: $work_product_id})
                     OPTIONAL MATCH (b:WorkProductBlock {block_id: $block_id})
                     OPTIONAL MATCH (i:IndexRun {index_run_id: $index_run_id})
                     OPTIONAL MATCH (a:TimelineAgentRun {agent_run_id: $agent_run_id})
                     FOREACH (_ IN CASE WHEN d IS NULL THEN [] ELSE [1] END |
                       MERGE (d)-[:PROPOSES_TIMELINE]->(s)
                     )
                     FOREACH (_ IN CASE WHEN w IS NULL THEN [] ELSE [1] END |
                       MERGE (w)-[:PROPOSES_TIMELINE]->(s)
                     )
                     FOREACH (_ IN CASE WHEN b IS NULL THEN [] ELSE [1] END |
                       MERGE (b)-[:PROPOSES_TIMELINE]->(s)
                     )
                     FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
                       MERGE (i)-[:PRODUCES_TIMELINE_SUGGESTION]->(s)
                     )
                     FOREACH (_ IN CASE WHEN a IS NULL THEN [] ELSE [1] END |
                       MERGE (a)-[:PRODUCES_TIMELINE_SUGGESTION]->(s)
                     )",
                )
                .param("suggestion_id", suggestion.suggestion_id.clone())
                .param("matter_id", matter_id)
                .param("status", suggestion.status.clone())
                .param("date", suggestion.date.clone())
                .param(
                    "source_document_id",
                    suggestion.source_document_id.clone().unwrap_or_default(),
                )
                .param(
                    "work_product_id",
                    suggestion.work_product_id.clone().unwrap_or_default(),
                )
                .param("block_id", suggestion.block_id.clone().unwrap_or_default())
                .param(
                    "index_run_id",
                    suggestion.index_run_id.clone().unwrap_or_default(),
                )
                .param(
                    "agent_run_id",
                    suggestion.agent_run_id.clone().unwrap_or_default(),
                ),
            )
            .await?;

        for source_span_id in &suggestion.source_span_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (s:TimelineSuggestion {suggestion_id: $suggestion_id})
                         MATCH (span:SourceSpan {source_span_id: $source_span_id})
                         MERGE (span)-[:PROPOSES_TIMELINE]->(s)",
                    )
                    .param("suggestion_id", suggestion.suggestion_id.clone())
                    .param("source_span_id", source_span_id.clone()),
                )
                .await?;
        }
        for text_chunk_id in &suggestion.text_chunk_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (s:TimelineSuggestion {suggestion_id: $suggestion_id})
                         MATCH (chunk:TextChunk {text_chunk_id: $text_chunk_id})
                         MERGE (chunk)-[:PROPOSES_TIMELINE]->(s)",
                    )
                    .param("suggestion_id", suggestion.suggestion_id.clone())
                    .param("text_chunk_id", text_chunk_id.clone()),
                )
                .await?;
        }
        for markdown_ast_node_id in &suggestion.markdown_ast_node_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (s:TimelineSuggestion {suggestion_id: $suggestion_id})
                         MATCH (n:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                         MERGE (n)-[:PROPOSES_TIMELINE]->(s)",
                    )
                    .param("suggestion_id", suggestion.suggestion_id.clone())
                    .param("markdown_ast_node_id", markdown_ast_node_id.clone()),
                )
                .await?;
        }
        for fact_id in &suggestion.linked_fact_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (s:TimelineSuggestion {suggestion_id: $suggestion_id})
                         MATCH (f:Fact {fact_id: $fact_id})
                         MERGE (f)-[:PROPOSES_TIMELINE]->(s)",
                    )
                    .param("suggestion_id", suggestion.suggestion_id.clone())
                    .param("fact_id", fact_id.clone()),
                )
                .await?;
        }
        for claim_id in &suggestion.linked_claim_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (s:TimelineSuggestion {suggestion_id: $suggestion_id})
                         MATCH (c:Claim {claim_id: $claim_id})
                         MERGE (c)-[:RELATES_TO_TIMELINE]->(s)",
                    )
                    .param("suggestion_id", suggestion.suggestion_id.clone())
                    .param("claim_id", claim_id.clone()),
                )
                .await?;
        }
        Ok(suggestion)
    }

    pub(super) async fn materialize_timeline_event_edges(
        &self,
        event: &CaseTimelineEvent,
    ) -> ApiResult<()> {
        if let Some(document_id) = &event.source_document_id {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:TimelineEvent {event_id: $event_id})
                         MATCH (d:CaseDocument {document_id: $document_id})
                         MERGE (d)-[:DOCUMENTS_EVENT]->(e)",
                    )
                    .param("event_id", event.event_id.clone())
                    .param("document_id", document_id.clone()),
                )
                .await?;
        }
        for fact_id in &event.linked_fact_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:TimelineEvent {event_id: $event_id})
                         MATCH (f:Fact {fact_id: $fact_id})
                         MERGE (f)-[:SUPPORTS_EVENT]->(e)",
                    )
                    .param("event_id", event.event_id.clone())
                    .param("fact_id", fact_id.clone()),
                )
                .await?;
        }
        for source_span_id in &event.source_span_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:TimelineEvent {event_id: $event_id})
                         MATCH (s:SourceSpan {source_span_id: $source_span_id})
                         MERGE (s)-[:SUPPORTS_EVENT]->(e)",
                    )
                    .param("event_id", event.event_id.clone())
                    .param("source_span_id", source_span_id.clone()),
                )
                .await?;
        }
        for text_chunk_id in &event.text_chunk_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:TimelineEvent {event_id: $event_id})
                         MATCH (c:TextChunk {text_chunk_id: $text_chunk_id})
                         MERGE (c)-[:SUPPORTS_EVENT]->(e)",
                    )
                    .param("event_id", event.event_id.clone())
                    .param("text_chunk_id", text_chunk_id.clone()),
                )
                .await?;
        }
        for markdown_ast_node_id in &event.markdown_ast_node_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:TimelineEvent {event_id: $event_id})
                         MATCH (n:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                         MERGE (n)-[:SUPPORTS_EVENT]->(e)",
                    )
                    .param("event_id", event.event_id.clone())
                    .param("markdown_ast_node_id", markdown_ast_node_id.clone()),
                )
                .await?;
        }
        if let Some(suggestion_id) = &event.suggestion_id {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:TimelineEvent {event_id: $event_id})
                         MATCH (s:TimelineSuggestion {suggestion_id: $suggestion_id})
                         MERGE (s)-[:APPROVED_AS]->(e)",
                    )
                    .param("event_id", event.event_id.clone())
                    .param("suggestion_id", suggestion_id.clone()),
                )
                .await?;
        }
        Ok(())
    }

    pub(super) async fn merge_page(&self, matter_id: &str, page: &Page) -> ApiResult<Page> {
        let page = self
            .merge_node(matter_id, page_spec(), &page.page_id, page)
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (p:Page {page_id: $page_id})
                     SET p.document_id = $document_id,
                         p.page_number = $page_number,
                         p.status = $status
                     MERGE (d)-[:HAS_PAGE]->(p)
                     WITH d, p
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     OPTIONAL MATCH (i:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     OPTIONAL MATCH (r:IndexRun {index_run_id: $index_run_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (p)-[:PART_OF_VERSION]->(v)
                     )
                     FOREACH (_ IN CASE WHEN b IS NULL THEN [] ELSE [1] END |
                       MERGE (p)-[:DERIVED_FROM]->(b)
                     )
                     FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
                       MERGE (i)-[:PRODUCED]->(p)
                     )
                     FOREACH (_ IN CASE WHEN r IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:PRODUCED]->(p)
                     )",
                )
                .param("document_id", page.document_id.clone())
                .param("page_id", page.page_id.clone())
                .param(
                    "document_version_id",
                    page.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "object_blob_id",
                    page.object_blob_id.clone().unwrap_or_default(),
                )
                .param(
                    "ingestion_run_id",
                    page.ingestion_run_id.clone().unwrap_or_default(),
                )
                .param(
                    "index_run_id",
                    page.index_run_id.clone().unwrap_or_default(),
                )
                .param("page_number", page.page_number as i64)
                .param("status", page.status.clone()),
            )
            .await?;
        Ok(page)
    }

    pub(super) async fn merge_text_chunk(
        &self,
        matter_id: &str,
        chunk: &TextChunk,
    ) -> ApiResult<TextChunk> {
        let chunk = self
            .merge_node(matter_id, text_chunk_spec(), &chunk.text_chunk_id, chunk)
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (c:TextChunk {text_chunk_id: $text_chunk_id})
                     SET c.document_id = $document_id,
                         c.ordinal = $ordinal,
                         c.status = $status,
                         c.text_hash = $text_hash,
                         c.text_excerpt = $text_excerpt,
                         c.unit_type = $unit_type,
                         c.structure_path = $structure_path
                     MERGE (d)-[:HAS_TEXT_CHUNK]->(c)
                     WITH d, c
                     OPTIONAL MATCH (p:Page {page_id: $page_id})
                     OPTIONAL MATCH (s:SourceSpan {source_span_id: $source_span_id})
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     OPTIONAL MATCH (i:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     OPTIONAL MATCH (r:IndexRun {index_run_id: $index_run_id})
                     FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
                       MERGE (p)-[:HAS_CHUNK]->(c)
                     )
                     FOREACH (_ IN CASE WHEN s IS NULL THEN [] ELSE [1] END |
                       MERGE (s)-[:QUOTES]->(c)
                     )
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (c)-[:PART_OF_VERSION]->(v)
                     )
                     FOREACH (_ IN CASE WHEN b IS NULL THEN [] ELSE [1] END |
                       MERGE (c)-[:DERIVED_FROM]->(b)
                     )
                     FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
                       MERGE (i)-[:PRODUCED]->(c)
                     )
                     FOREACH (_ IN CASE WHEN r IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:PRODUCED]->(c)
                     )",
                )
                .param("document_id", chunk.document_id.clone())
                .param("text_chunk_id", chunk.text_chunk_id.clone())
                .param("page_id", chunk.page_id.clone().unwrap_or_default())
                .param(
                    "source_span_id",
                    chunk.source_span_id.clone().unwrap_or_default(),
                )
                .param(
                    "document_version_id",
                    chunk.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "object_blob_id",
                    chunk.object_blob_id.clone().unwrap_or_default(),
                )
                .param(
                    "ingestion_run_id",
                    chunk.ingestion_run_id.clone().unwrap_or_default(),
                )
                .param(
                    "index_run_id",
                    chunk.index_run_id.clone().unwrap_or_default(),
                )
                .param("ordinal", chunk.ordinal as i64)
                .param("status", chunk.status.clone())
                .param("text_hash", chunk.text_hash.clone())
                .param("text_excerpt", chunk.text_excerpt.clone())
                .param("unit_type", chunk.unit_type.clone().unwrap_or_default())
                .param(
                    "structure_path",
                    chunk.structure_path.clone().unwrap_or_default(),
                ),
            )
            .await?;
        for markdown_ast_node_id in &chunk.markdown_ast_node_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:TextChunk {text_chunk_id: $text_chunk_id})
                         MATCH (n:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                         MERGE (n)-[:OVERLAPS_TEXT_CHUNK]->(c)",
                    )
                    .param("text_chunk_id", chunk.text_chunk_id.clone())
                    .param("markdown_ast_node_id", markdown_ast_node_id.clone()),
                )
                .await?;
        }
        Ok(chunk)
    }

    pub(super) async fn merge_evidence_span(
        &self,
        matter_id: &str,
        span: &EvidenceSpan,
    ) -> ApiResult<EvidenceSpan> {
        let span = self
            .merge_node(
                matter_id,
                evidence_span_spec(),
                &span.evidence_span_id,
                span,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (e:EvidenceSpan {evidence_span_id: $evidence_span_id})
                     SET e.document_id = $document_id,
                         e.review_status = $review_status,
                         e.quote_hash = $quote_hash
                     MERGE (d)-[:HAS_EVIDENCE_SPAN]->(e)
                     WITH d, e
                     OPTIONAL MATCH (c:TextChunk {text_chunk_id: $text_chunk_id})
                     OPTIONAL MATCH (s:SourceSpan {source_span_id: $source_span_id})
                     OPTIONAL MATCH (i:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     OPTIONAL MATCH (r:IndexRun {index_run_id: $index_run_id})
                     FOREACH (_ IN CASE WHEN c IS NULL THEN [] ELSE [1] END |
                       MERGE (c)-[:HAS_EVIDENCE_SPAN]->(e)
                     )
                     FOREACH (_ IN CASE WHEN s IS NULL THEN [] ELSE [1] END |
                       MERGE (e)-[:FROM_SOURCE_SPAN]->(s)
                     )
                     FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
                       MERGE (i)-[:PRODUCED]->(e)
                     )
                     FOREACH (_ IN CASE WHEN r IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:PRODUCED]->(e)
                     )",
                )
                .param("document_id", span.document_id.clone())
                .param("evidence_span_id", span.evidence_span_id.clone())
                .param(
                    "text_chunk_id",
                    span.text_chunk_id.clone().unwrap_or_default(),
                )
                .param(
                    "source_span_id",
                    span.source_span_id.clone().unwrap_or_default(),
                )
                .param(
                    "ingestion_run_id",
                    span.ingestion_run_id.clone().unwrap_or_default(),
                )
                .param(
                    "index_run_id",
                    span.index_run_id.clone().unwrap_or_default(),
                )
                .param("review_status", span.review_status.clone())
                .param("quote_hash", span.quote_hash.clone()),
            )
            .await?;
        for markdown_ast_node_id in &span.markdown_ast_node_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:EvidenceSpan {evidence_span_id: $evidence_span_id})
                         MATCH (n:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                         MERGE (n)-[:OVERLAPS_EVIDENCE_SPAN]->(e)",
                    )
                    .param("evidence_span_id", span.evidence_span_id.clone())
                    .param("markdown_ast_node_id", markdown_ast_node_id.clone()),
                )
                .await?;
        }
        Ok(span)
    }

    pub(super) async fn merge_entity_mention(
        &self,
        matter_id: &str,
        mention: &EntityMention,
    ) -> ApiResult<EntityMention> {
        let mention = self
            .merge_node(
                matter_id,
                entity_mention_spec(),
                &mention.entity_mention_id,
                mention,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (e:EntityMention {entity_mention_id: $entity_mention_id})
                     SET e.document_id = $document_id,
                         e.entity_type = $entity_type,
                         e.review_status = $review_status
                     MERGE (d)-[:HAS_ENTITY_MENTION]->(e)
                     WITH d, e
                     OPTIONAL MATCH (c:TextChunk {text_chunk_id: $text_chunk_id})
                     OPTIONAL MATCH (s:SourceSpan {source_span_id: $source_span_id})
                     OPTIONAL MATCH (ce:CaseEntity {entity_id: $entity_id})
                     FOREACH (_ IN CASE WHEN c IS NULL THEN [] ELSE [1] END |
                       MERGE (c)-[:MENTIONS]->(e)
                     )
                     FOREACH (_ IN CASE WHEN s IS NULL THEN [] ELSE [1] END |
                       MERGE (e)-[:FROM_SOURCE_SPAN]->(s)
                     )
                     FOREACH (_ IN CASE WHEN ce IS NULL THEN [] ELSE [1] END |
                       MERGE (e)-[:RESOLVES_TO]->(ce)
                     )",
                )
                .param("document_id", mention.document_id.clone())
                .param("entity_mention_id", mention.entity_mention_id.clone())
                .param(
                    "text_chunk_id",
                    mention.text_chunk_id.clone().unwrap_or_default(),
                )
                .param(
                    "source_span_id",
                    mention.source_span_id.clone().unwrap_or_default(),
                )
                .param("entity_id", mention.entity_id.clone().unwrap_or_default())
                .param("entity_type", mention.entity_type.clone())
                .param("review_status", mention.review_status.clone()),
            )
            .await?;
        for markdown_ast_node_id in &mention.markdown_ast_node_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:EntityMention {entity_mention_id: $entity_mention_id})
                         MATCH (n:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                         MERGE (n)-[:HAS_ENTITY_MENTION]->(e)",
                    )
                    .param("entity_mention_id", mention.entity_mention_id.clone())
                    .param("markdown_ast_node_id", markdown_ast_node_id.clone()),
                )
                .await?;
        }
        Ok(mention)
    }

    pub(super) async fn merge_case_entity(
        &self,
        matter_id: &str,
        entity: &CaseEntity,
    ) -> ApiResult<CaseEntity> {
        let entity = self
            .merge_node(matter_id, case_entity_spec(), &entity.entity_id, entity)
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (e:CaseEntity {entity_id: $entity_id})
                     SET e.matter_id = $matter_id,
                         e.entity_type = $entity_type,
                         e.normalized_key = $normalized_key,
                         e.canonical_name = $canonical_name,
                         e.review_status = $review_status",
                )
                .param("matter_id", matter_id)
                .param("entity_id", entity.entity_id.clone())
                .param("entity_type", entity.entity_type.clone())
                .param("normalized_key", entity.normalized_key.clone())
                .param("canonical_name", entity.canonical_name.clone())
                .param("review_status", entity.review_status.clone()),
            )
            .await?;
        for mention_id in &entity.mention_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (m:EntityMention {entity_mention_id: $entity_mention_id})
                         MATCH (e:CaseEntity {entity_id: $entity_id})
                         MERGE (m)-[:RESOLVES_TO]->(e)",
                    )
                    .param("entity_mention_id", mention_id.clone())
                    .param("entity_id", entity.entity_id.clone()),
                )
                .await?;
        }
        for party_id in &entity.party_match_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:CaseEntity {entity_id: $entity_id})
                         MATCH (p:Party {party_id: $party_id})
                         MERGE (e)-[:MAY_MATCH_PARTY]->(p)",
                    )
                    .param("entity_id", entity.entity_id.clone())
                    .param("party_id", party_id.clone()),
                )
                .await?;
        }
        Ok(entity)
    }

    pub(super) async fn merge_markdown_ast_document(
        &self,
        matter_id: &str,
        document: &MarkdownAstDocument,
    ) -> ApiResult<MarkdownAstDocument> {
        let document = self
            .merge_node(
                matter_id,
                markdown_ast_document_spec(),
                &document.markdown_ast_document_id,
                document,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (a:MarkdownAstDocument {markdown_ast_document_id: $markdown_ast_document_id})
                     SET a.document_id = $document_id,
                         a.status = $status,
                         a.source_sha256 = $source_sha256,
                         a.parser_id = $parser_id,
                         a.parser_version = $parser_version,
                         a.graph_schema_version = $graph_schema_version,
                         a.node_count = $node_count,
                         a.semantic_unit_count = $semantic_unit_count,
                         a.heading_count = $heading_count,
                         a.reference_count = $reference_count
                     MERGE (d)-[:HAS_MARKDOWN_AST_DOCUMENT]->(a)
                     WITH d, a
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     OPTIONAL MATCH (i:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     OPTIONAL MATCH (r:IndexRun {index_run_id: $index_run_id})
                     OPTIONAL MATCH (root:MarkdownAstNode {markdown_ast_node_id: $root_node_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (v)-[:HAS_MARKDOWN_AST_DOCUMENT]->(a)
                     )
                     FOREACH (_ IN CASE WHEN b IS NULL THEN [] ELSE [1] END |
                       MERGE (a)-[:DERIVED_FROM]->(b)
                     )
                     FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
                       MERGE (i)-[:PRODUCED]->(a)
                     )
                     FOREACH (_ IN CASE WHEN r IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:PRODUCED]->(a)
                     )
                     FOREACH (_ IN CASE WHEN root IS NULL THEN [] ELSE [1] END |
                       MERGE (a)-[:HAS_AST_ROOT]->(root)
                     )",
                )
                .param("document_id", document.document_id.clone())
                .param(
                    "markdown_ast_document_id",
                    document.markdown_ast_document_id.clone(),
                )
                .param(
                    "document_version_id",
                    document.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "object_blob_id",
                    document.object_blob_id.clone().unwrap_or_default(),
                )
                .param(
                    "ingestion_run_id",
                    document.ingestion_run_id.clone().unwrap_or_default(),
                )
                .param("index_run_id", document.index_run_id.clone().unwrap_or_default())
                .param("root_node_id", document.root_node_id.clone())
                .param("status", document.status.clone())
                .param("source_sha256", document.source_sha256.clone())
                .param("parser_id", document.parser_id.clone())
                .param("parser_version", document.parser_version.clone())
                .param("graph_schema_version", document.graph_schema_version.clone())
                .param("node_count", document.node_count as i64)
                .param("semantic_unit_count", document.semantic_unit_count as i64)
                .param("heading_count", document.heading_count as i64)
                .param("reference_count", document.reference_count as i64),
            )
            .await?;
        Ok(document)
    }

    pub(super) async fn merge_markdown_ast_node(
        &self,
        matter_id: &str,
        node: &MarkdownAstNode,
    ) -> ApiResult<MarkdownAstNode> {
        let node = self
            .merge_node(
                matter_id,
                markdown_ast_node_spec(),
                &node.markdown_ast_node_id,
                node,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (n:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                     MATCH (a:MarkdownAstDocument {markdown_ast_document_id: $markdown_ast_document_id})
                     SET n.document_id = $document_id,
                         n.node_kind = $node_kind,
                         n.ordinal = $ordinal,
                         n.depth = $depth,
                         n.ast_path = $ast_path,
                         n.sibling_index = $sibling_index,
                         n.child_count = $child_count,
                         n.structure_path = $structure_path,
                         n.section_path = $section_path,
                         n.semantic_role = $semantic_role,
                         n.semantic_fingerprint = $semantic_fingerprint,
                         n.semantic_unit_id = $semantic_unit_id,
                         n.heading_level = $heading_level,
                         n.heading_text = $heading_text,
                         n.review_status = $review_status,
                         n.text_excerpt = $text_excerpt,
                         n.contains_entity_mention = $contains_entity_mention,
                         n.contains_citation = $contains_citation,
                         n.contains_date = $contains_date,
                         n.contains_money = $contains_money
                     MERGE (d)-[:CONTAINS_AST_NODE]->(n)
                     MERGE (a)-[:CONTAINS_AST_NODE]->(n)
                     WITH d, n, a
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (i:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     OPTIONAL MATCH (r:IndexRun {index_run_id: $index_run_id})
                     OPTIONAL MATCH (u:MarkdownSemanticUnit {semantic_unit_id: $semantic_unit_id})
                     OPTIONAL MATCH (parent:MarkdownAstNode {markdown_ast_node_id: $parent_ast_node_id})
                     OPTIONAL MATCH (prev:MarkdownAstNode {markdown_ast_node_id: $previous_ast_node_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (v)-[:CONTAINS_AST_NODE]->(n)
                     )
                     FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
                       MERGE (i)-[:PRODUCED]->(n)
                     )
                     FOREACH (_ IN CASE WHEN r IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:PRODUCED]->(n)
                     )
                     FOREACH (_ IN CASE WHEN u IS NULL THEN [] ELSE [1] END |
                       MERGE (n)-[:REALIZES_SEMANTIC_UNIT]->(u)
                     )
                     FOREACH (_ IN CASE WHEN parent IS NULL THEN [] ELSE [1] END |
                       MERGE (parent)-[:PARENT_OF]->(n)
                     )
                     FOREACH (_ IN CASE WHEN prev IS NULL THEN [] ELSE [1] END |
                       MERGE (prev)-[:NEXT_AST_NODE]->(n)
                     )
                     FOREACH (_ IN CASE WHEN $is_root THEN [1] ELSE [] END |
                       MERGE (a)-[:HAS_AST_ROOT]->(n)
                     )",
                )
                .param("document_id", node.document_id.clone())
                .param("markdown_ast_node_id", node.markdown_ast_node_id.clone())
                .param(
                    "markdown_ast_document_id",
                    node.markdown_ast_document_id.clone(),
                )
                .param(
                    "document_version_id",
                    node.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "ingestion_run_id",
                    node.ingestion_run_id.clone().unwrap_or_default(),
                )
                .param("index_run_id", node.index_run_id.clone().unwrap_or_default())
                .param(
                    "parent_ast_node_id",
                    node.parent_ast_node_id.clone().unwrap_or_default(),
                )
                .param(
                    "previous_ast_node_id",
                    node.previous_ast_node_id.clone().unwrap_or_default(),
                )
                .param(
                    "semantic_unit_id",
                    node.semantic_unit_id.clone().unwrap_or_default(),
                )
                .param("node_kind", node.node_kind.clone())
                .param("is_root", node.parent_ast_node_id.is_none())
                .param("ordinal", node.ordinal as i64)
                .param("depth", node.depth as i64)
                .param("ast_path", node.ast_path.clone())
                .param("sibling_index", node.sibling_index as i64)
                .param("child_count", node.child_count as i64)
                .param("structure_path", node.structure_path.clone().unwrap_or_default())
                .param("section_path", node.section_path.clone().unwrap_or_default())
                .param(
                    "semantic_role",
                    node.semantic_role.clone().unwrap_or_default(),
                )
                .param(
                    "semantic_fingerprint",
                    node.semantic_fingerprint.clone().unwrap_or_default(),
                )
                .param("heading_level", node.heading_level.unwrap_or_default() as i64)
                .param("heading_text", node.heading_text.clone().unwrap_or_default())
                .param("review_status", node.review_status.clone())
                .param("text_excerpt", node.text_excerpt.clone().unwrap_or_default())
                .param("contains_entity_mention", node.contains_entity_mention)
                .param("contains_citation", node.contains_citation)
                .param("contains_date", node.contains_date)
                .param("contains_money", node.contains_money),
            )
            .await?;
        for source_span_id in &node.source_span_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (n:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                         MATCH (s:SourceSpan {source_span_id: $source_span_id})
                         MERGE (n)-[:OVERLAPS_SOURCE_SPAN]->(s)",
                    )
                    .param("markdown_ast_node_id", node.markdown_ast_node_id.clone())
                    .param("source_span_id", source_span_id.clone()),
                )
                .await?;
        }
        for text_chunk_id in &node.text_chunk_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (n:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                         MATCH (c:TextChunk {text_chunk_id: $text_chunk_id})
                         MERGE (n)-[:OVERLAPS_TEXT_CHUNK]->(c)",
                    )
                    .param("markdown_ast_node_id", node.markdown_ast_node_id.clone())
                    .param("text_chunk_id", text_chunk_id.clone()),
                )
                .await?;
        }
        for evidence_span_id in &node.evidence_span_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (n:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                         MATCH (e:EvidenceSpan {evidence_span_id: $evidence_span_id})
                         MERGE (n)-[:OVERLAPS_EVIDENCE_SPAN]->(e)",
                    )
                    .param("markdown_ast_node_id", node.markdown_ast_node_id.clone())
                    .param("evidence_span_id", evidence_span_id.clone()),
                )
                .await?;
        }
        for search_index_record_id in &node.search_index_record_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (n:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                         MATCH (r:SearchIndexRecord {search_index_record_id: $search_index_record_id})
                         MERGE (n)-[:INDEXED_AS]->(r)",
                    )
                    .param("markdown_ast_node_id", node.markdown_ast_node_id.clone())
                    .param("search_index_record_id", search_index_record_id.clone()),
                )
                .await?;
        }
        Ok(node)
    }

    pub(super) async fn merge_markdown_semantic_unit(
        &self,
        matter_id: &str,
        unit: &MarkdownSemanticUnit,
    ) -> ApiResult<MarkdownSemanticUnit> {
        let unit = self
            .merge_node(
                matter_id,
                markdown_semantic_unit_spec(),
                &unit.semantic_unit_id,
                unit,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (u:MarkdownSemanticUnit {semantic_unit_id: $semantic_unit_id})
                     SET u.document_id = $document_id,
                         u.unit_kind = $unit_kind,
                         u.semantic_role = $semantic_role,
                         u.canonical_label = $canonical_label,
                         u.normalized_key = $normalized_key,
                         u.semantic_fingerprint = $semantic_fingerprint,
                         u.review_status = $review_status,
                         u.occurrence_count = $occurrence_count
                     MERGE (d)-[:HAS_MARKDOWN_SEMANTIC_UNIT]->(u)
                     WITH d, u
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (a:MarkdownAstDocument {markdown_ast_document_id: $markdown_ast_document_id})
                     OPTIONAL MATCH (section:MarkdownAstNode {markdown_ast_node_id: $section_ast_node_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (v)-[:HAS_MARKDOWN_SEMANTIC_UNIT]->(u)
                     )
                     FOREACH (_ IN CASE WHEN a IS NULL THEN [] ELSE [1] END |
                       MERGE (a)-[:HAS_SEMANTIC_UNIT]->(u)
                     )
                     FOREACH (_ IN CASE WHEN section IS NULL THEN [] ELSE [1] END |
                       MERGE (section)-[:CONTAINS_SEMANTIC_UNIT]->(u)
                     )",
                )
                .param("document_id", unit.document_id.clone())
                .param("semantic_unit_id", unit.semantic_unit_id.clone())
                .param(
                    "document_version_id",
                    unit.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "markdown_ast_document_id",
                    unit.markdown_ast_document_id.clone(),
                )
                .param(
                    "section_ast_node_id",
                    unit.section_ast_node_id.clone().unwrap_or_default(),
                )
                .param("unit_kind", unit.unit_kind.clone())
                .param("semantic_role", unit.semantic_role.clone())
                .param("canonical_label", unit.canonical_label.clone())
                .param("normalized_key", unit.normalized_key.clone())
                .param("semantic_fingerprint", unit.semantic_fingerprint.clone())
                .param("review_status", unit.review_status.clone())
                .param("occurrence_count", unit.occurrence_count as i64),
            )
            .await?;
        Ok(unit)
    }

    pub(super) async fn merge_search_index_record(
        &self,
        matter_id: &str,
        record: &SearchIndexRecord,
    ) -> ApiResult<SearchIndexRecord> {
        let record = self
            .merge_node(
                matter_id,
                search_index_record_spec(),
                &record.search_index_record_id,
                record,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (r:SearchIndexRecord {search_index_record_id: $search_index_record_id})
                     SET r.document_id = $document_id,
                         r.status = $status,
                         r.stale = $stale,
                         r.index_name = $index_name,
                         r.index_type = $index_type,
                         r.index_version = $index_version
                     MERGE (d)-[:HAS_SEARCH_INDEX_RECORD]->(r)
                     WITH d, r
                     OPTIONAL MATCH (c:TextChunk {text_chunk_id: $text_chunk_id})
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (i:IndexRun {index_run_id: $index_run_id})
                     FOREACH (_ IN CASE WHEN c IS NULL THEN [] ELSE [1] END |
                       MERGE (c)-[:INDEXED_AS]->(r)
                     )
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:PART_OF_VERSION]->(v)
                     )
                     FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
                       MERGE (i)-[:PRODUCED]->(r)
                     )",
                )
                .param("document_id", record.document_id.clone())
                .param(
                    "search_index_record_id",
                    record.search_index_record_id.clone(),
                )
                .param(
                    "text_chunk_id",
                    record.text_chunk_id.clone().unwrap_or_default(),
                )
                .param(
                    "document_version_id",
                    record.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "index_run_id",
                    record.index_run_id.clone().unwrap_or_default(),
                )
                .param("status", record.status.clone())
                .param("stale", record.stale)
                .param("index_name", record.index_name.clone())
                .param("index_type", record.index_type.clone())
                .param("index_version", record.index_version.clone()),
            )
            .await?;
        Ok(record)
    }

    pub(super) async fn merge_casebuilder_embedding_run(
        &self,
        matter_id: &str,
        run: &CaseBuilderEmbeddingRun,
    ) -> ApiResult<CaseBuilderEmbeddingRun> {
        let run = self
            .merge_node(
                matter_id,
                casebuilder_embedding_run_spec(),
                &run.embedding_run_id,
                run,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (r:CaseBuilderEmbeddingRun {embedding_run_id: $embedding_run_id})
                     SET r.matter_id = $matter_id,
                         r.document_id = $document_id,
                         r.document_version_id = $document_version_id,
                         r.index_run_id = $index_run_id,
                         r.model = $model,
                         r.profile = $profile,
                         r.dimension = $dimension,
                         r.vector_index_name = $vector_index_name,
                         r.status = $status,
                         r.stage = $stage,
                         r.target_count = $target_count,
                         r.embedded_count = $embedded_count,
                         r.skipped_count = $skipped_count,
                         r.stale_count = $stale_count,
                         r.retryable = $retryable,
                         r.started_at = $started_at,
                         r.completed_at = $completed_at
                     WITH r
                     OPTIONAL MATCH (d:CaseDocument {document_id: $document_id})
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (i:IndexRun {index_run_id: $index_run_id})
                     FOREACH (_ IN CASE WHEN d IS NULL THEN [] ELSE [1] END |
                       MERGE (d)-[:HAS_EMBEDDING_RUN]->(r)
                     )
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (v)-[:HAS_EMBEDDING_RUN]->(r)
                     )
                     FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
                       MERGE (i)-[:PRODUCED]->(r)
                     )",
                )
                .param("matter_id", matter_id.to_string())
                .param("embedding_run_id", run.embedding_run_id.clone())
                .param("document_id", run.document_id.clone().unwrap_or_default())
                .param(
                    "document_version_id",
                    run.document_version_id.clone().unwrap_or_default(),
                )
                .param("index_run_id", run.index_run_id.clone().unwrap_or_default())
                .param("model", run.model.clone())
                .param("profile", run.profile.clone())
                .param("dimension", run.dimension as i64)
                .param("vector_index_name", run.vector_index_name.clone())
                .param("status", run.status.clone())
                .param("stage", run.stage.clone())
                .param("target_count", run.target_count as i64)
                .param("embedded_count", run.embedded_count as i64)
                .param("skipped_count", run.skipped_count as i64)
                .param("stale_count", run.stale_count as i64)
                .param("retryable", run.retryable)
                .param("started_at", run.started_at.clone())
                .param("completed_at", run.completed_at.clone().unwrap_or_default()),
            )
            .await?;
        Ok(run)
    }

    pub(super) async fn merge_casebuilder_embedding_record(
        &self,
        matter_id: &str,
        record: &CaseBuilderEmbeddingRecord,
        embedding: Option<Vec<f32>>,
    ) -> ApiResult<CaseBuilderEmbeddingRecord> {
        let record = self
            .merge_node(
                matter_id,
                casebuilder_embedding_record_spec(),
                &record.embedding_record_id,
                record,
            )
            .await?;
        let statement = query(
            "MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
             SET r.matter_id = $matter_id,
                 r.document_id = $document_id,
                 r.document_version_id = $document_version_id,
                 r.index_run_id = $index_run_id,
                 r.embedding_run_id = $embedding_run_id,
                 r.target_kind = $target_kind,
                 r.target_id = $target_id,
                 r.target_label = $target_label,
                 r.model = $model,
                 r.profile = $profile,
                 r.dimension = $dimension,
                 r.vector_index_name = $vector_index_name,
                 r.input_hash = $input_hash,
                 r.source_text_hash = $source_text_hash,
                 r.chunker_version = $chunker_version,
                 r.graph_schema_version = $graph_schema_version,
                 r.embedding_strategy = $embedding_strategy,
                 r.status = $status,
                 r.stale = $stale,
                 r.review_status = $review_status,
                 r.text_excerpt = $text_excerpt,
                 r.embedded_at = $embedded_at
             WITH r
             OPTIONAL MATCH (d:CaseDocument {document_id: $document_id})
             OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
             OPTIONAL MATCH (i:IndexRun {index_run_id: $index_run_id})
             OPTIONAL MATCH (run:CaseBuilderEmbeddingRun {embedding_run_id: $embedding_run_id})
             FOREACH (_ IN CASE WHEN d IS NULL THEN [] ELSE [1] END |
               MERGE (d)-[:HAS_EMBEDDING_RECORD]->(r)
             )
             FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
               MERGE (v)-[:HAS_EMBEDDING_RECORD]->(r)
             )
             FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
               MERGE (i)-[:PRODUCED]->(r)
             )
             FOREACH (_ IN CASE WHEN run IS NULL THEN [] ELSE [1] END |
               MERGE (run)-[:PRODUCED]->(r)
             )",
        )
        .param("matter_id", matter_id.to_string())
        .param("embedding_record_id", record.embedding_record_id.clone())
        .param("document_id", record.document_id.clone())
        .param(
            "document_version_id",
            record.document_version_id.clone().unwrap_or_default(),
        )
        .param(
            "index_run_id",
            record.index_run_id.clone().unwrap_or_default(),
        )
        .param(
            "embedding_run_id",
            record.embedding_run_id.clone().unwrap_or_default(),
        )
        .param("target_kind", record.target_kind.clone())
        .param("target_id", record.target_id.clone())
        .param("target_label", record.target_label.clone())
        .param("model", record.model.clone())
        .param("profile", record.profile.clone())
        .param("dimension", record.dimension as i64)
        .param("vector_index_name", record.vector_index_name.clone())
        .param("input_hash", record.input_hash.clone())
        .param("source_text_hash", record.source_text_hash.clone())
        .param(
            "chunker_version",
            record.chunker_version.clone().unwrap_or_default(),
        )
        .param(
            "graph_schema_version",
            record.graph_schema_version.clone().unwrap_or_default(),
        )
        .param("embedding_strategy", record.embedding_strategy.clone())
        .param("status", record.status.clone())
        .param("stale", record.stale)
        .param("review_status", record.review_status.clone())
        .param(
            "text_excerpt",
            record.text_excerpt.clone().unwrap_or_default(),
        )
        .param(
            "embedded_at",
            record.embedded_at.clone().unwrap_or_default(),
        );
        self.neo4j.run_rows(statement).await?;
        if let Some(embedding) = embedding {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
                         SET r.embedding = $embedding",
                    )
                    .param("embedding_record_id", record.embedding_record_id.clone())
                    .param("embedding", embedding),
                )
                .await?;
        }
        self.link_embedding_record_targets(&record).await?;
        Ok(record)
    }

    async fn link_embedding_record_targets(
        &self,
        record: &CaseBuilderEmbeddingRecord,
    ) -> ApiResult<()> {
        match record.target_kind.as_str() {
            "markdown_file" => {
                if let Some(version_id) = record.document_version_id.as_deref() {
                    self.neo4j
                        .run_rows(
                            query(
                                "MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                                 MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
                                 MERGE (v)-[:HAS_EMBEDDING_RECORD]->(r)",
                            )
                            .param("document_version_id", version_id.to_string())
                            .param("embedding_record_id", record.embedding_record_id.clone()),
                        )
                        .await?;
                }
            }
            "markdown_ast_document" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (a:MarkdownAstDocument {markdown_ast_document_id: $target_id})
                             MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
                             MERGE (a)-[:HAS_EMBEDDING_RECORD]->(r)",
                        )
                        .param("target_id", record.target_id.clone())
                        .param("embedding_record_id", record.embedding_record_id.clone()),
                    )
                    .await?;
            }
            "text_chunk" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (c:TextChunk {text_chunk_id: $target_id})
                             MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
                             MERGE (c)-[:HAS_EMBEDDING_RECORD]->(r)",
                        )
                        .param("target_id", record.target_id.clone())
                        .param("embedding_record_id", record.embedding_record_id.clone()),
                    )
                    .await?;
            }
            "markdown_semantic_unit" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (u:MarkdownSemanticUnit {semantic_unit_id: $target_id})
                             MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
                             MERGE (u)-[:HAS_EMBEDDING_RECORD]->(r)",
                        )
                        .param("target_id", record.target_id.clone())
                        .param("embedding_record_id", record.embedding_record_id.clone()),
                    )
                    .await?;
            }
            _ => {}
        }
        for source_span_id in &record.source_span_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
                         MATCH (s:SourceSpan {source_span_id: $source_span_id})
                         MERGE (r)-[:EMBEDS_SOURCE_SPAN]->(s)",
                    )
                    .param("embedding_record_id", record.embedding_record_id.clone())
                    .param("source_span_id", source_span_id.clone()),
                )
                .await?;
        }
        for text_chunk_id in &record.text_chunk_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
                         MATCH (c:TextChunk {text_chunk_id: $text_chunk_id})
                         MERGE (r)-[:EMBEDS_TEXT_CHUNK]->(c)",
                    )
                    .param("embedding_record_id", record.embedding_record_id.clone())
                    .param("text_chunk_id", text_chunk_id.clone()),
                )
                .await?;
        }
        for ast_node_id in &record.markdown_ast_node_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
                         MATCH (a:MarkdownAstNode {markdown_ast_node_id: $markdown_ast_node_id})
                         MERGE (r)-[:EMBEDS_AST_NODE]->(a)",
                    )
                    .param("embedding_record_id", record.embedding_record_id.clone())
                    .param("markdown_ast_node_id", ast_node_id.clone()),
                )
                .await?;
        }
        for unit_id in &record.markdown_semantic_unit_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
                         MATCH (u:MarkdownSemanticUnit {semantic_unit_id: $semantic_unit_id})
                         MERGE (r)-[:EMBEDS_SEMANTIC_UNIT]->(u)",
                    )
                    .param("embedding_record_id", record.embedding_record_id.clone())
                    .param("semantic_unit_id", unit_id.clone()),
                )
                .await?;
        }
        for source_record_id in &record.centroid_source_record_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (r:CaseBuilderEmbeddingRecord {embedding_record_id: $embedding_record_id})
                         MATCH (source:CaseBuilderEmbeddingRecord {embedding_record_id: $source_record_id})
                         MERGE (r)-[:CENTROID_OF]->(source)",
                    )
                    .param("embedding_record_id", record.embedding_record_id.clone())
                    .param("source_record_id", source_record_id.clone()),
                )
                .await?;
        }
        Ok(())
    }

    pub(super) async fn search_casebuilder_embedding_records(
        &self,
        matter_id: &str,
        embedding: Vec<f32>,
        limit: u64,
        document_ids: &[String],
        target_kinds: &[String],
        include_stale: bool,
    ) -> ApiResult<Vec<(CaseBuilderEmbeddingRecord, f32)>> {
        let limit = limit.max(1).min(50);
        let top_k = limit.max(10).min(100);
        let cypher = format!(
            "MATCH (record:CaseBuilderEmbeddingRecord)
               SEARCH record IN (
                 VECTOR INDEX casebuilder_markdown_embedding_1024
                 FOR $embedding
                 WHERE record.matter_id = $matter_id
                   AND record.status = 'embedded'
                   AND ($include_stale = true OR record.stale = false)
                 LIMIT {top_k}
               ) SCORE AS score
             WHERE ($document_ids_empty = true OR record.document_id IN $document_ids)
               AND ($target_kinds_empty = true OR record.target_kind IN $target_kinds)
             RETURN record.payload AS payload, score
             ORDER BY score DESC
             LIMIT {limit}",
            top_k = top_k,
            limit = limit,
        );
        let rows = self
            .neo4j
            .run_rows(
                query(&cypher)
                    .param("embedding", embedding)
                    .param("matter_id", matter_id.to_string())
                    .param("include_stale", include_stale)
                    .param("document_ids_empty", document_ids.is_empty())
                    .param("document_ids", document_ids.to_vec())
                    .param("target_kinds_empty", target_kinds.is_empty())
                    .param("target_kinds", target_kinds.to_vec()),
            )
            .await?;
        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let payload = row
                .get::<String>("payload")
                .map_err(|error| ApiError::Internal(error.to_string()))?;
            let score = row
                .get::<f64>("score")
                .map(|value| value as f32)
                .or_else(|_| row.get::<f32>("score"))
                .unwrap_or(0.0);
            results.push((from_payload(&payload)?, score));
        }
        Ok(results)
    }

    pub(super) async fn merge_extraction_artifact_manifest(
        &self,
        matter_id: &str,
        manifest: &ExtractionArtifactManifest,
    ) -> ApiResult<ExtractionArtifactManifest> {
        let manifest = self
            .merge_node(
                matter_id,
                extraction_artifact_manifest_spec(),
                &manifest.manifest_id,
                manifest,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (m:ExtractionArtifactManifest {manifest_id: $manifest_id})
                     SET m.document_id = $document_id,
                         m.text_sha256 = $text_sha256,
                         m.manifest_sha256 = $manifest_sha256
                     MERGE (d)-[:HAS_EXTRACTION_MANIFEST]->(m)
                     WITH d, m
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (i:IngestionRun {ingestion_run_id: $ingestion_run_id})
                     OPTIONAL MATCH (r:IndexRun {index_run_id: $index_run_id})
                     OPTIONAL MATCH (n:DocumentVersion {document_version_id: $normalized_text_version_id})
                     OPTIONAL MATCH (p:DocumentVersion {document_version_id: $pages_version_id})
                     OPTIONAL MATCH (mv:DocumentVersion {document_version_id: $manifest_version_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (m)-[:PART_OF_VERSION]->(v)
                     )
                     FOREACH (_ IN CASE WHEN i IS NULL THEN [] ELSE [1] END |
                       MERGE (i)-[:PRODUCED]->(m)
                     )
                     FOREACH (_ IN CASE WHEN r IS NULL THEN [] ELSE [1] END |
                       MERGE (r)-[:PRODUCED]->(m)
                     )
                     FOREACH (_ IN CASE WHEN n IS NULL THEN [] ELSE [1] END |
                       MERGE (m)-[:REFERENCES_ARTIFACT]->(n)
                     )
                     FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
                       MERGE (m)-[:REFERENCES_ARTIFACT]->(p)
                     )
                     FOREACH (_ IN CASE WHEN mv IS NULL THEN [] ELSE [1] END |
                       MERGE (m)-[:STORED_AS]->(mv)
                     )",
                )
                .param("document_id", manifest.document_id.clone())
                .param("manifest_id", manifest.manifest_id.clone())
                .param(
                    "document_version_id",
                    manifest.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "ingestion_run_id",
                    manifest.ingestion_run_id.clone().unwrap_or_default(),
                )
                .param(
                    "index_run_id",
                    manifest.index_run_id.clone().unwrap_or_default(),
                )
                .param(
                    "normalized_text_version_id",
                    manifest
                        .normalized_text_version_id
                        .clone()
                        .unwrap_or_default(),
                )
                .param(
                    "pages_version_id",
                    manifest.pages_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "manifest_version_id",
                    manifest.manifest_version_id.clone().unwrap_or_default(),
                )
                .param("text_sha256", manifest.text_sha256.clone())
                .param(
                    "manifest_sha256",
                    manifest.manifest_sha256.clone().unwrap_or_default(),
                ),
            )
            .await?;
        Ok(manifest)
    }
}
