use super::*;
use std::collections::HashMap;

const CASEBUILDER_EMBEDDING_PROFILE: &str = "casebuilder_markdown_v1";
const CASEBUILDER_EMBEDDING_VECTOR_INDEX: &str = "casebuilder_markdown_embedding_1024";
const CASEBUILDER_EMBEDDING_OUTPUT_DTYPE: &str = "float";
const CASEBUILDER_FULL_FILE_DIRECT_TOKEN_LIMIT: u64 = 28_000;

#[derive(Clone)]
struct CaseBuilderEmbeddingTarget {
    record: CaseBuilderEmbeddingRecord,
    input: String,
}

impl CaseBuilderService {
    pub(super) async fn queue_document_embeddings_after_index(
        &self,
        matter_id: &str,
        document_id: &str,
        index_run_id: Option<String>,
        document_version_id: Option<String>,
    ) -> ApiResult<Option<CaseBuilderEmbeddingRun>> {
        let document = self.get_document(matter_id, document_id).await?;
        if !document_is_markdown_indexable(&document) {
            return Ok(None);
        }
        let mut run = self.new_embedding_run(
            matter_id,
            &document,
            index_run_id,
            document_version_id,
            "queued",
            "queued",
        );
        if !self.embeddings_enabled {
            run.status = "skipped".to_string();
            run.stage = "disabled".to_string();
            run.skipped_count = 1;
            run.completed_at = Some(now_string());
            run.warnings.push(
                "CaseBuilder Markdown embeddings are disabled; set ORS_CASEBUILDER_EMBEDDINGS_ENABLED=true to enable them."
                    .to_string(),
            );
            return self
                .merge_casebuilder_embedding_run(matter_id, &run)
                .await
                .map(Some);
        }
        if self.embeddings.is_none() {
            run.status = "skipped".to_string();
            run.stage = "provider_unconfigured".to_string();
            run.skipped_count = 1;
            run.completed_at = Some(now_string());
            run.warnings.push(
                "Voyage embeddings are enabled but VOYAGE_API_KEY is not configured.".to_string(),
            );
            return self
                .merge_casebuilder_embedding_run(matter_id, &run)
                .await
                .map(Some);
        }
        let queued = self
            .merge_casebuilder_embedding_run(matter_id, &run)
            .await?;
        let worker = self.clone();
        let matter_id = matter_id.to_string();
        tokio::spawn(async move {
            if let Err(error) = worker
                .execute_document_embedding_run(&matter_id, queued)
                .await
            {
                tracing::warn!(error = %error, "CaseBuilder Markdown embedding run failed");
            }
        });
        Ok(Some(run))
    }

    pub async fn run_document_embeddings(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<CaseBuilderEmbeddingRun> {
        let document = self.get_document(matter_id, document_id).await?;
        let run = self.new_embedding_run(
            matter_id,
            &document,
            None,
            document.current_version_id.clone(),
            "queued",
            "queued",
        );
        self.merge_casebuilder_embedding_run(matter_id, &run)
            .await?;
        self.execute_document_embedding_run(matter_id, run).await
    }

    pub async fn run_matter_embeddings(
        &self,
        matter_id: &str,
        request: RunCaseBuilderEmbeddingsRequest,
    ) -> ApiResult<RunCaseBuilderEmbeddingsResponse> {
        self.require_matter(matter_id).await?;
        let requested_ids = request
            .document_ids
            .unwrap_or_default()
            .into_iter()
            .collect::<HashSet<_>>();
        let limit = request.limit.unwrap_or(50).max(1) as usize;
        let mut documents = self.list_documents(matter_id).await?;
        documents.sort_by(|left, right| left.uploaded_at.cmp(&right.uploaded_at));
        let mut selected = documents
            .into_iter()
            .filter(|document| {
                requested_ids.is_empty() || requested_ids.contains(&document.document_id)
            })
            .filter(document_is_markdown_indexable)
            .take(limit)
            .collect::<Vec<_>>();

        let requested = selected.len() as u64;
        let mut processed = 0;
        let mut skipped = 0;
        let mut failed = 0;
        let mut warnings = Vec::new();
        let mut runs = Vec::new();
        for document in selected.drain(..) {
            match self
                .run_document_embeddings(matter_id, &document.document_id)
                .await
            {
                Ok(run) => {
                    match run.status.as_str() {
                        "completed" => processed += 1,
                        "skipped" => skipped += 1,
                        "failed" => failed += 1,
                        _ => {}
                    }
                    runs.push(run);
                }
                Err(error) => {
                    failed += 1;
                    warnings.push(format!(
                        "{} embedding run failed: {error}",
                        document.filename
                    ));
                }
            }
        }
        Ok(RunCaseBuilderEmbeddingsResponse {
            matter_id: matter_id.to_string(),
            requested,
            processed,
            skipped,
            failed,
            runs,
            warnings,
        })
    }

    pub async fn search_casebuilder_embeddings(
        &self,
        matter_id: &str,
        request: CaseBuilderEmbeddingSearchRequest,
    ) -> ApiResult<CaseBuilderEmbeddingSearchResponse> {
        self.require_matter(matter_id).await?;
        let Some(embeddings) = self.embeddings.as_ref() else {
            return Ok(CaseBuilderEmbeddingSearchResponse {
                enabled: self.embeddings_enabled,
                mode: "markdown_embeddings".to_string(),
                query: request.query,
                total: 0,
                results: Vec::new(),
                model: Some(self.embedding_model()),
                profile: Some(CASEBUILDER_EMBEDDING_PROFILE.to_string()),
                dimension: Some(self.embedding_dimension() as u64),
                warnings: vec![
                    "Voyage embeddings are not configured; search is unavailable.".to_string(),
                ],
            });
        };
        if !self.embeddings_enabled {
            return Ok(CaseBuilderEmbeddingSearchResponse {
                enabled: false,
                mode: "markdown_embeddings".to_string(),
                query: request.query,
                total: 0,
                results: Vec::new(),
                model: Some(self.embedding_model()),
                profile: Some(CASEBUILDER_EMBEDDING_PROFILE.to_string()),
                dimension: Some(self.embedding_dimension() as u64),
                warnings: vec!["CaseBuilder Markdown embeddings are disabled.".to_string()],
            });
        }
        let query_text = request.query.trim().to_string();
        if query_text.is_empty() {
            return Err(ApiError::BadRequest(
                "Embedding search query cannot be empty".to_string(),
            ));
        }
        let embedding = embeddings.embed_query(&query_text).await?;
        let records = self
            .search_casebuilder_embedding_records(
                matter_id,
                embedding,
                request.limit.unwrap_or(10),
                &request.document_ids.unwrap_or_default(),
                &request.target_kinds.unwrap_or_default(),
                request.include_stale.unwrap_or(false),
            )
            .await?;
        let results = records
            .into_iter()
            .map(|(record, score)| CaseBuilderEmbeddingSearchResult {
                score,
                target_kind: record.target_kind.clone(),
                target_id: record.target_id.clone(),
                document_id: record.document_id.clone(),
                document_version_id: record.document_version_id.clone(),
                text_excerpt: record.text_excerpt.clone(),
                source_span_ids: record.source_span_ids.clone(),
                text_chunk_ids: record.text_chunk_ids.clone(),
                markdown_ast_node_ids: record.markdown_ast_node_ids.clone(),
                markdown_semantic_unit_ids: record.markdown_semantic_unit_ids.clone(),
                stale: record.stale,
                embedding_record: record,
            })
            .collect::<Vec<_>>();
        Ok(CaseBuilderEmbeddingSearchResponse {
            enabled: true,
            mode: "markdown_embeddings".to_string(),
            query: query_text,
            total: results.len() as u64,
            results,
            model: Some(self.embedding_model()),
            profile: Some(CASEBUILDER_EMBEDDING_PROFILE.to_string()),
            dimension: Some(self.embedding_dimension() as u64),
            warnings: Vec::new(),
        })
    }

    fn new_embedding_run(
        &self,
        matter_id: &str,
        document: &CaseDocument,
        index_run_id: Option<String>,
        document_version_id: Option<String>,
        status: &str,
        stage: &str,
    ) -> CaseBuilderEmbeddingRun {
        let started_at = now_string();
        let seed = format!(
            "{}:{}:{}:{}:{}",
            matter_id,
            document.document_id,
            document_version_id.as_deref().unwrap_or_default(),
            index_run_id.as_deref().unwrap_or_default(),
            started_at
        );
        let embedding_run_id = format!("embedding-run:{}", hex_prefix(seed.as_bytes(), 24));
        CaseBuilderEmbeddingRun {
            embedding_run_id: embedding_run_id.clone(),
            id: embedding_run_id,
            matter_id: matter_id.to_string(),
            document_id: Some(document.document_id.clone()),
            document_version_id,
            index_run_id,
            model: self.embedding_model(),
            profile: CASEBUILDER_EMBEDDING_PROFILE.to_string(),
            dimension: self.embedding_dimension() as u64,
            vector_index_name: CASEBUILDER_EMBEDDING_VECTOR_INDEX.to_string(),
            status: status.to_string(),
            stage: stage.to_string(),
            target_count: 0,
            embedded_count: 0,
            skipped_count: 0,
            stale_count: 0,
            produced_embedding_record_ids: Vec::new(),
            warnings: Vec::new(),
            error_code: None,
            error_message: None,
            retryable: false,
            started_at,
            completed_at: None,
        }
    }

    async fn execute_document_embedding_run(
        &self,
        matter_id: &str,
        mut run: CaseBuilderEmbeddingRun,
    ) -> ApiResult<CaseBuilderEmbeddingRun> {
        run.status = "running".to_string();
        run.stage = "building_inputs".to_string();
        self.merge_casebuilder_embedding_run(matter_id, &run)
            .await?;

        let outcome = self
            .execute_document_embedding_run_inner(matter_id, &mut run)
            .await;
        match outcome {
            Ok(()) => {
                if run.status != "skipped" {
                    run.status = "completed".to_string();
                    run.stage = "completed".to_string();
                }
                run.completed_at = Some(now_string());
                self.merge_casebuilder_embedding_run(matter_id, &run)
                    .await?;
                Ok(run)
            }
            Err(error) => {
                run.status = "failed".to_string();
                run.stage = "provider_failed".to_string();
                run.error_code = Some("embedding_provider_failed".to_string());
                run.error_message = Some(error.to_string());
                run.retryable = true;
                run.completed_at = Some(now_string());
                self.merge_casebuilder_embedding_run(matter_id, &run)
                    .await?;
                Ok(run)
            }
        }
    }

    async fn execute_document_embedding_run_inner(
        &self,
        matter_id: &str,
        run: &mut CaseBuilderEmbeddingRun,
    ) -> ApiResult<()> {
        let document_id = run.document_id.clone().ok_or_else(|| {
            ApiError::Internal("Embedding run is missing document_id".to_string())
        })?;
        let document = self.get_document(matter_id, &document_id).await?;
        if !document_is_markdown_indexable(&document) {
            run.status = "skipped".to_string();
            run.stage = "view_only".to_string();
            run.skipped_count = 1;
            run.warnings
                .push("Only Markdown files are embedded in this CaseBuilder pass.".to_string());
            return Ok(());
        }
        if !self.embeddings_enabled {
            run.status = "skipped".to_string();
            run.stage = "disabled".to_string();
            run.skipped_count = 1;
            run.warnings
                .push("CaseBuilder Markdown embeddings are disabled.".to_string());
            return Ok(());
        }
        let Some(embeddings) = self.embeddings.as_ref() else {
            run.status = "skipped".to_string();
            run.stage = "provider_unconfigured".to_string();
            run.skipped_count = 1;
            run.warnings.push(
                "Voyage embeddings are enabled but VOYAGE_API_KEY is not configured.".to_string(),
            );
            return Ok(());
        };

        let source_text = match document.extracted_text.clone() {
            Some(text) if !text.trim().is_empty() => text,
            _ => self.document_bytes_as_text(&document).await?,
        };
        let index_text = markdown_index_text(&source_text);
        let source_text_hash = sha256_hex(index_text.as_bytes());
        let current_version_id = document.current_version_id.clone();
        let stale_count = self
            .mark_stale_embedding_records(matter_id, &document, &source_text_hash)
            .await?;
        run.stale_count = stale_count;

        let targets = self
            .build_embedding_targets(
                matter_id,
                &document,
                current_version_id.clone(),
                run.index_run_id.clone(),
                Some(run.embedding_run_id.clone()),
                &index_text,
                &source_text_hash,
            )
            .await?;
        if targets.is_empty() {
            run.status = "skipped".to_string();
            run.stage = "no_embedding_inputs".to_string();
            run.skipped_count = 1;
            run.warnings
                .push("No Markdown chunks or semantic units were available to embed.".to_string());
            return Ok(());
        }

        run.stage = "provider_embedding".to_string();
        run.target_count = targets.len() as u64;
        self.merge_casebuilder_embedding_run(matter_id, run).await?;

        let inputs = targets
            .iter()
            .map(|target| target.input.clone())
            .collect::<Vec<_>>();
        let vectors = embeddings.embed_documents(&inputs).await?;
        let mut embedded_vectors = Vec::<Vec<f32>>::new();
        let mut embedded_record_ids = Vec::<String>::new();
        let mut produced_records = Vec::<CaseBuilderEmbeddingRecord>::new();
        for (target, vector) in targets.into_iter().zip(vectors.into_iter()) {
            let mut record = target.record;
            record.status = "embedded".to_string();
            record.stale = false;
            record.embedded_at = Some(now_string());
            let stored = self
                .merge_casebuilder_embedding_record(matter_id, &record, Some(vector.clone()))
                .await?;
            embedded_vectors.push(vector);
            embedded_record_ids.push(stored.embedding_record_id.clone());
            produced_records.push(stored);
        }
        if let Some(centroid) = self.centroid_record_for_document(
            matter_id,
            &document,
            current_version_id,
            run.index_run_id.clone(),
            Some(run.embedding_run_id.clone()),
            &index_text,
            &source_text_hash,
            &produced_records,
            &embedded_vectors,
        ) {
            let centroid_vector = centroid_embedding(&embedded_vectors);
            let stored = self
                .merge_casebuilder_embedding_record(matter_id, &centroid, Some(centroid_vector))
                .await?;
            embedded_record_ids.push(stored.embedding_record_id);
        }
        run.embedded_count = embedded_record_ids.len() as u64;
        run.produced_embedding_record_ids = embedded_record_ids;
        Ok(())
    }

    async fn build_embedding_targets(
        &self,
        matter_id: &str,
        document: &CaseDocument,
        document_version_id: Option<String>,
        index_run_id: Option<String>,
        embedding_run_id: Option<String>,
        index_text: &str,
        source_text_hash: &str,
    ) -> ApiResult<Vec<CaseBuilderEmbeddingTarget>> {
        let mut targets = Vec::new();
        let text_chunks = self
            .list_nodes::<TextChunk>(matter_id, text_chunk_spec())
            .await?
            .into_iter()
            .filter(|chunk| chunk.document_id == document.document_id)
            .filter(|chunk| {
                document_version_id.is_none()
                    || chunk.document_version_id.as_deref() == document_version_id.as_deref()
            })
            .collect::<Vec<_>>();
        let ast_nodes = self
            .list_nodes::<MarkdownAstNode>(matter_id, markdown_ast_node_spec())
            .await?
            .into_iter()
            .filter(|node| node.document_id == document.document_id)
            .filter(|node| {
                document_version_id.is_none()
                    || node.document_version_id.as_deref() == document_version_id.as_deref()
            })
            .collect::<Vec<_>>();
        let ast_node_by_id = ast_nodes
            .iter()
            .map(|node| (node.markdown_ast_node_id.clone(), node.clone()))
            .collect::<HashMap<_, _>>();
        let semantic_units = self
            .list_nodes::<MarkdownSemanticUnit>(matter_id, markdown_semantic_unit_spec())
            .await?
            .into_iter()
            .filter(|unit| unit.document_id == document.document_id)
            .filter(|unit| {
                document_version_id.is_none()
                    || unit.document_version_id.as_deref() == document_version_id.as_deref()
            })
            .collect::<Vec<_>>();

        if approximate_token_count(index_text) <= CASEBUILDER_FULL_FILE_DIRECT_TOKEN_LIMIT {
            let outline = ast_nodes
                .iter()
                .filter(|node| node.node_kind == "heading")
                .filter_map(|node| node.heading_text.clone())
                .take(30)
                .collect::<Vec<_>>()
                .join(" / ");
            let input = format!(
                "Matter: {matter_id}\nDocument: {}\nPath: {}\nOutline: {}\n\n{}",
                document.title,
                document
                    .original_relative_path
                    .as_deref()
                    .unwrap_or(document.filename.as_str()),
                outline,
                index_text
            );
            let label = format!("Full Markdown file: {}", document.title);
            let record = self.embedding_record(
                matter_id,
                document,
                document_version_id.clone(),
                index_run_id.clone(),
                embedding_run_id.clone(),
                "markdown_file",
                document_version_id
                    .as_deref()
                    .unwrap_or(document.document_id.as_str()),
                &label,
                &input,
                source_text_hash,
                "direct",
                text_excerpt(index_text, 320),
                Vec::new(),
                text_chunks
                    .iter()
                    .map(|chunk| chunk.text_chunk_id.clone())
                    .collect(),
                ast_nodes
                    .iter()
                    .map(|node| node.markdown_ast_node_id.clone())
                    .collect(),
                semantic_units
                    .iter()
                    .map(|unit| unit.semantic_unit_id.clone())
                    .collect(),
                Vec::new(),
            );
            targets.push(CaseBuilderEmbeddingTarget { record, input });
        }

        for chunk in &text_chunks {
            let chunk_text = byte_slice(index_text, chunk.byte_start, chunk.byte_end)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| chunk.text_excerpt.clone());
            let input = format!(
                "Document: {}\nChunk ordinal: {}\nUnit type: {}\nStructure path: {}\n\n{}",
                document.title,
                chunk.ordinal,
                chunk
                    .unit_type
                    .clone()
                    .unwrap_or_else(|| "chunk".to_string()),
                chunk.structure_path.clone().unwrap_or_default(),
                chunk_text
            );
            let record = self.embedding_record(
                matter_id,
                document,
                document_version_id.clone(),
                index_run_id.clone(),
                embedding_run_id.clone(),
                "text_chunk",
                &chunk.text_chunk_id,
                &format!("Chunk {}", chunk.ordinal),
                &input,
                source_text_hash,
                "direct",
                text_excerpt(&chunk_text, 320),
                chunk.source_span_id.clone().into_iter().collect(),
                vec![chunk.text_chunk_id.clone()],
                chunk.markdown_ast_node_ids.clone(),
                Vec::new(),
                Vec::new(),
            );
            targets.push(CaseBuilderEmbeddingTarget { record, input });
        }

        for unit in &semantic_units {
            let unit_text = semantic_unit_text(index_text, unit, &ast_node_by_id)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| unit.canonical_label.clone());
            let input = format!(
                "Document: {}\nSemantic role: {}\nUnit kind: {}\nSection path: {}\nCanonical label: {}\nCitations: {}\nDates: {}\nMoney: {}\n\n{}",
                document.title,
                unit.semantic_role,
                unit.unit_kind,
                unit.section_path.clone().unwrap_or_default(),
                unit.canonical_label,
                unit.citation_texts.join("; "),
                unit.date_texts.join("; "),
                unit.money_texts.join("; "),
                unit_text
            );
            let source_span_ids = ast_nodes
                .iter()
                .filter(|node| {
                    unit.markdown_ast_node_ids
                        .contains(&node.markdown_ast_node_id)
                })
                .flat_map(|node| node.source_span_ids.clone())
                .collect::<Vec<_>>();
            let text_chunk_ids = ast_nodes
                .iter()
                .filter(|node| {
                    unit.markdown_ast_node_ids
                        .contains(&node.markdown_ast_node_id)
                })
                .flat_map(|node| node.text_chunk_ids.clone())
                .collect::<Vec<_>>();
            let record = self.embedding_record(
                matter_id,
                document,
                document_version_id.clone(),
                index_run_id.clone(),
                embedding_run_id.clone(),
                "markdown_semantic_unit",
                &unit.semantic_unit_id,
                &unit.canonical_label,
                &input,
                source_text_hash,
                "direct",
                text_excerpt(&unit_text, 320),
                unique_strings(source_span_ids),
                unique_strings(text_chunk_ids),
                unit.markdown_ast_node_ids.clone(),
                vec![unit.semantic_unit_id.clone()],
                Vec::new(),
            );
            targets.push(CaseBuilderEmbeddingTarget { record, input });
        }

        Ok(targets)
    }

    #[allow(clippy::too_many_arguments)]
    fn embedding_record(
        &self,
        matter_id: &str,
        document: &CaseDocument,
        document_version_id: Option<String>,
        index_run_id: Option<String>,
        embedding_run_id: Option<String>,
        target_kind: &str,
        target_id: &str,
        target_label: &str,
        input: &str,
        source_text_hash: &str,
        strategy: &str,
        text_excerpt_value: String,
        source_span_ids: Vec<String>,
        text_chunk_ids: Vec<String>,
        markdown_ast_node_ids: Vec<String>,
        markdown_semantic_unit_ids: Vec<String>,
        centroid_source_record_ids: Vec<String>,
    ) -> CaseBuilderEmbeddingRecord {
        let input_hash = sha256_hex(input.as_bytes());
        let seed = format!(
            "{}:{}:{}:{}:{}:{}:{}",
            target_kind,
            target_id,
            document_version_id.as_deref().unwrap_or_default(),
            self.embedding_model(),
            self.embedding_dimension(),
            CASEBUILDER_EMBEDDING_PROFILE,
            input_hash
        );
        let embedding_record_id = format!(
            "embedding-record:{}:{}",
            sanitize_path_segment(target_kind),
            hex_prefix(seed.as_bytes(), 28)
        );
        let created_at = now_string();
        CaseBuilderEmbeddingRecord {
            embedding_record_id: embedding_record_id.clone(),
            id: embedding_record_id,
            matter_id: matter_id.to_string(),
            document_id: document.document_id.clone(),
            document_version_id,
            index_run_id,
            embedding_run_id,
            target_kind: target_kind.to_string(),
            target_id: target_id.to_string(),
            target_label: target_label.to_string(),
            model: self.embedding_model(),
            profile: CASEBUILDER_EMBEDDING_PROFILE.to_string(),
            dimension: self.embedding_dimension() as u64,
            vector_index_name: CASEBUILDER_EMBEDDING_VECTOR_INDEX.to_string(),
            input_hash,
            source_text_hash: source_text_hash.to_string(),
            chunker_version: Some(CHUNKER_VERSION.to_string()),
            graph_schema_version: Some(markdown_graph::MARKDOWN_GRAPH_SCHEMA_VERSION.to_string()),
            embedding_strategy: strategy.to_string(),
            embedding_input_type: "document".to_string(),
            embedding_output_dtype: CASEBUILDER_EMBEDDING_OUTPUT_DTYPE.to_string(),
            status: "queued".to_string(),
            stale: false,
            review_status: "system".to_string(),
            text_excerpt: Some(text_excerpt_value),
            source_span_ids: unique_strings(source_span_ids),
            text_chunk_ids: unique_strings(text_chunk_ids),
            markdown_ast_node_ids: unique_strings(markdown_ast_node_ids),
            markdown_semantic_unit_ids: unique_strings(markdown_semantic_unit_ids),
            centroid_source_record_ids: unique_strings(centroid_source_record_ids),
            created_at,
            embedded_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn centroid_record_for_document(
        &self,
        matter_id: &str,
        document: &CaseDocument,
        document_version_id: Option<String>,
        index_run_id: Option<String>,
        embedding_run_id: Option<String>,
        index_text: &str,
        source_text_hash: &str,
        records: &[CaseBuilderEmbeddingRecord],
        vectors: &[Vec<f32>],
    ) -> Option<CaseBuilderEmbeddingRecord> {
        if approximate_token_count(index_text) <= CASEBUILDER_FULL_FILE_DIRECT_TOKEN_LIMIT
            || records.is_empty()
            || vectors.is_empty()
        {
            return None;
        }
        let input = format!(
            "Document centroid from {} Markdown chunks and semantic units: {}\n{}",
            records.len(),
            document.title,
            text_excerpt(index_text, 2_000)
        );
        let source_record_ids = records
            .iter()
            .map(|record| record.embedding_record_id.clone())
            .collect::<Vec<_>>();
        Some(
            self.embedding_record(
                matter_id,
                document,
                document_version_id.clone(),
                index_run_id,
                embedding_run_id,
                "markdown_file",
                document_version_id
                    .as_deref()
                    .unwrap_or(document.document_id.as_str()),
                &format!("Full Markdown file centroid: {}", document.title),
                &input,
                source_text_hash,
                "centroid_from_chunks",
                text_excerpt(index_text, 320),
                Vec::new(),
                records
                    .iter()
                    .flat_map(|record| record.text_chunk_ids.clone())
                    .collect(),
                records
                    .iter()
                    .flat_map(|record| record.markdown_ast_node_ids.clone())
                    .collect(),
                records
                    .iter()
                    .flat_map(|record| record.markdown_semantic_unit_ids.clone())
                    .collect(),
                source_record_ids,
            ),
        )
    }

    async fn mark_stale_embedding_records(
        &self,
        matter_id: &str,
        document: &CaseDocument,
        source_text_hash: &str,
    ) -> ApiResult<u64> {
        let mut stale_count = 0;
        let existing = self
            .list_nodes::<CaseBuilderEmbeddingRecord>(
                matter_id,
                casebuilder_embedding_record_spec(),
            )
            .await?
            .into_iter()
            .filter(|record| record.document_id == document.document_id)
            .collect::<Vec<_>>();
        for mut record in existing {
            let current = record.document_version_id.as_deref()
                == document.current_version_id.as_deref()
                && record.model == self.embedding_model()
                && record.dimension == self.embedding_dimension() as u64
                && record.profile == CASEBUILDER_EMBEDDING_PROFILE
                && record.chunker_version.as_deref() == Some(CHUNKER_VERSION)
                && record.graph_schema_version.as_deref()
                    == Some(markdown_graph::MARKDOWN_GRAPH_SCHEMA_VERSION)
                && record.source_text_hash == source_text_hash;
            if !current && !record.stale {
                record.stale = true;
                self.merge_casebuilder_embedding_record(matter_id, &record, None)
                    .await?;
                stale_count += 1;
            }
        }
        Ok(stale_count)
    }

    pub(super) fn embedding_coverage_for_records(
        &self,
        records: &[CaseBuilderEmbeddingRecord],
    ) -> CaseBuilderEmbeddingCoverage {
        let embedded = records
            .iter()
            .filter(|record| record.status == "embedded")
            .count() as u64;
        let current = records
            .iter()
            .filter(|record| record.status == "embedded" && !record.stale)
            .count() as u64;
        CaseBuilderEmbeddingCoverage {
            enabled: self.embeddings_enabled && self.embeddings.is_some(),
            model: Some(self.embedding_model()),
            profile: Some(CASEBUILDER_EMBEDDING_PROFILE.to_string()),
            dimension: Some(self.embedding_dimension() as u64),
            vector_index_name: Some(CASEBUILDER_EMBEDDING_VECTOR_INDEX.to_string()),
            target_count: records.len() as u64,
            embedded_count: embedded,
            current_count: current,
            stale_count: records.iter().filter(|record| record.stale).count() as u64,
            skipped_count: records
                .iter()
                .filter(|record| record.status == "skipped")
                .count() as u64,
            failed_count: records
                .iter()
                .filter(|record| record.status == "failed")
                .count() as u64,
            full_file_embedded: records.iter().any(|record| {
                record.target_kind == "markdown_file"
                    && record.status == "embedded"
                    && !record.stale
            }),
            chunk_embedded: records
                .iter()
                .filter(|record| {
                    record.target_kind == "text_chunk"
                        && record.status == "embedded"
                        && !record.stale
                })
                .count() as u64,
            semantic_unit_embedded: records
                .iter()
                .filter(|record| {
                    record.target_kind == "markdown_semantic_unit"
                        && record.status == "embedded"
                        && !record.stale
                })
                .count() as u64,
        }
    }

    fn embedding_model(&self) -> String {
        self.embeddings
            .as_ref()
            .map(|service| service.model().to_string())
            .unwrap_or_else(|| "voyage-4-large".to_string())
    }

    fn embedding_dimension(&self) -> usize {
        self.embeddings
            .as_ref()
            .map(|service| service.dimension())
            .unwrap_or(1024)
    }
}

fn byte_slice(text: &str, byte_start: Option<u64>, byte_end: Option<u64>) -> Option<String> {
    let start = byte_start? as usize;
    let end = byte_end? as usize;
    if start > end || end > text.len() {
        return None;
    }
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return None;
    }
    Some(text[start..end].to_string())
}

fn semantic_unit_text(
    text: &str,
    unit: &MarkdownSemanticUnit,
    ast_node_by_id: &HashMap<String, MarkdownAstNode>,
) -> Option<String> {
    let mut start = usize::MAX;
    let mut end = 0usize;
    for node_id in &unit.markdown_ast_node_ids {
        let Some(node) = ast_node_by_id.get(node_id) else {
            continue;
        };
        let (Some(node_start), Some(node_end)) = (node.byte_start, node.byte_end) else {
            continue;
        };
        start = start.min(node_start as usize);
        end = end.max(node_end as usize);
    }
    if start == usize::MAX || end == 0 {
        return None;
    }
    byte_slice(text, Some(start as u64), Some(end as u64))
}

fn centroid_embedding(vectors: &[Vec<f32>]) -> Vec<f32> {
    let Some(first) = vectors.first() else {
        return Vec::new();
    };
    let mut centroid = vec![0.0; first.len()];
    for vector in vectors {
        for (index, value) in vector.iter().enumerate().take(centroid.len()) {
            centroid[index] += *value;
        }
    }
    let denom = vectors.len().max(1) as f32;
    for value in &mut centroid {
        *value /= denom;
    }
    normalize_vector(centroid)
}

fn normalize_vector(mut vector: Vec<f32>) -> Vec<f32> {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for value in values {
        if !value.trim().is_empty() && seen.insert(value.clone()) {
            unique.push(value);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_slice_keeps_exact_source_range() {
        let text = "# Intro\nAlpha beta\n";
        assert_eq!(
            byte_slice(text, Some(8), Some(18)),
            Some("Alpha beta".to_string())
        );
        assert_eq!(byte_slice(text, Some(18), Some(8)), None);
    }

    #[test]
    fn centroid_embedding_is_normalized() {
        let centroid = centroid_embedding(&[vec![1.0, 0.0], vec![0.0, 1.0]]);
        let norm = centroid
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        assert!((norm - 1.0).abs() < 0.0001);
        assert!((centroid[0] - centroid[1]).abs() < 0.0001);
    }

    #[test]
    fn unique_strings_preserves_order() {
        assert_eq!(
            unique_strings(vec!["a".to_string(), "b".to_string(), "a".to_string()]),
            vec!["a".to_string(), "b".to_string()]
        );
    }
}
