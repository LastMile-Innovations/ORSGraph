use super::*;

impl CaseBuilderService {
    pub async fn create_draft(
        &self,
        matter_id: &str,
        request: CreateDraftRequest,
    ) -> ApiResult<CaseDraft> {
        let id = generate_id("draft", &request.title);
        let kind = request
            .draft_type
            .clone()
            .unwrap_or_else(|| "legal_memo".to_string());
        if kind == "complaint" {
            return Err(ApiError::BadRequest(
                "Complaint drafting now uses the structured /complaints API, not generic drafts."
                    .to_string(),
            ));
        }
        let product = self
            .create_work_product_with_id(
                matter_id,
                &id,
                CreateWorkProductRequest {
                    title: Some(request.title),
                    product_type: kind,
                    template: None,
                    source_draft_id: Some(id.clone()),
                    source_complaint_id: None,
                },
            )
            .await?;
        let mut draft = work_product_to_draft(&product);
        draft.description = request.description.unwrap_or_default();
        if let Some(status) = request.status {
            draft.status = status;
        }
        Ok(draft)
    }

    pub async fn list_drafts(&self, matter_id: &str) -> ApiResult<Vec<CaseDraft>> {
        Ok(self
            .list_work_products(matter_id)
            .await?
            .into_iter()
            .filter(|product| product.product_type != "complaint")
            .map(|product| work_product_to_draft(&product))
            .collect())
    }

    pub async fn get_draft(&self, matter_id: &str, draft_id: &str) -> ApiResult<CaseDraft> {
        Ok(work_product_to_draft(
            &self.get_work_product(matter_id, draft_id).await?,
        ))
    }

    pub async fn patch_draft(
        &self,
        matter_id: &str,
        draft_id: &str,
        request: PatchDraftRequest,
    ) -> ApiResult<CaseDraft> {
        let mut product = self.get_work_product(matter_id, draft_id).await?;
        let mut draft = work_product_to_draft(&product);
        if let Some(value) = request.title {
            draft.title = value;
        }
        if let Some(value) = request.description {
            draft.description = value;
        }
        if let Some(value) = request.status {
            draft.status = value;
        }
        if let Some(value) = request.sections {
            draft.sections = value;
        }
        if let Some(value) = request.paragraphs {
            draft.paragraphs = value;
        }
        draft.word_count = count_words(&draft.paragraphs, &draft.sections);
        draft.updated_at = now_string();
        product.title = draft.title.clone();
        product.status = draft.status.clone();
        product.blocks = work_product_blocks_from_draft(&draft);
        product.history.push(work_product_event(
            matter_id,
            draft_id,
            "legacy_draft_patch",
            "draft",
            draft_id,
            "Deprecated draft wrapper patched the shared WorkProduct AST.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        Ok(work_product_to_draft(&product))
    }

    pub async fn generate_draft(
        &self,
        matter_id: &str,
        draft_id: &str,
    ) -> ApiResult<AiActionResponse<CaseDraft>> {
        let existing = self.get_work_product(matter_id, draft_id).await?;
        let matter = self.get_matter_summary(matter_id).await?;
        let facts = self.list_facts(matter_id).await?;
        let claims = self.list_claims(matter_id).await?;
        let mut product = default_work_product_from_matter(
            &matter,
            draft_id,
            &existing.title,
            &existing.product_type,
            &facts,
            &claims,
            &now_string(),
        );
        product.created_at = existing.created_at;
        product.source_draft_id = existing.source_draft_id.or(Some(draft_id.to_string()));
        product.history = existing.history;
        product.history.push(work_product_event(
            matter_id,
            draft_id,
            "legacy_draft_generated",
            "draft",
            draft_id,
            "Deprecated draft wrapper regenerated the shared WorkProduct AST.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;

        Ok(AiActionResponse {
            enabled: false,
            mode: "template".to_string(),
            message: "No live drafting provider is configured; generated a deterministic source-linked draft scaffold.".to_string(),
            result: Some(work_product_to_draft(&product)),
        })
    }

    pub async fn fact_check_draft(
        &self,
        matter_id: &str,
        draft_id: &str,
    ) -> ApiResult<AiActionResponse<Vec<FactCheckFinding>>> {
        let draft = self.get_draft(matter_id, draft_id).await?;
        let mut findings = Vec::new();
        for paragraph in &draft.paragraphs {
            if paragraph.factcheck_status == "needs_evidence" {
                let finding_id = generate_id("factcheck", &paragraph.paragraph_id);
                findings.push(FactCheckFinding {
                    id: finding_id.clone(),
                    finding_id,
                    matter_id: matter_id.to_string(),
                    draft_id: draft_id.to_string(),
                    paragraph_id: Some(paragraph.paragraph_id.clone()),
                    finding_type: "unsupported_fact".to_string(),
                    severity: "warning".to_string(),
                    message: "Paragraph has factual text without linked evidence.".to_string(),
                    source_fact_ids: paragraph.fact_ids.clone(),
                    source_evidence_ids: paragraph.evidence_ids.clone(),
                    status: "open".to_string(),
                });
            }
        }
        for finding in &findings {
            self.merge_node(
                matter_id,
                fact_check_finding_spec(),
                &finding.finding_id,
                finding,
            )
            .await?;
        }
        Ok(AiActionResponse {
            enabled: false,
            mode: "deterministic".to_string(),
            message:
                "No live fact-checking provider is configured; ran deterministic support checks."
                    .to_string(),
            result: Some(findings),
        })
    }

    pub async fn citation_check_draft(
        &self,
        matter_id: &str,
        draft_id: &str,
    ) -> ApiResult<AiActionResponse<Vec<CitationCheckFinding>>> {
        let draft = self.get_draft(matter_id, draft_id).await?;
        let mut findings = Vec::new();
        for paragraph in &draft.paragraphs {
            if paragraph.role == "legal_claim" && paragraph.authorities.is_empty() {
                let finding_id = generate_id("citecheck", &paragraph.paragraph_id);
                findings.push(CitationCheckFinding {
                    id: finding_id.clone(),
                    finding_id,
                    matter_id: matter_id.to_string(),
                    draft_id: draft_id.to_string(),
                    citation: String::new(),
                    canonical_id: None,
                    finding_type: "missing_citation".to_string(),
                    severity: "warning".to_string(),
                    message: "Legal claim paragraph has no linked authority.".to_string(),
                    status: "open".to_string(),
                });
            }
        }
        for finding in &findings {
            self.merge_node(
                matter_id,
                citation_check_finding_spec(),
                &finding.finding_id,
                finding,
            )
            .await?;
        }
        Ok(AiActionResponse {
            enabled: false,
            mode: "deterministic".to_string(),
            message:
                "No live citation-checking provider is configured; ran missing-authority checks."
                    .to_string(),
            result: Some(findings),
        })
    }

    pub async fn list_complaints(&self, matter_id: &str) -> ApiResult<Vec<ComplaintDraft>> {
        self.list_nodes(matter_id, complaint_spec()).await
    }

    pub async fn create_complaint(
        &self,
        matter_id: &str,
        request: CreateComplaintRequest,
    ) -> ApiResult<ComplaintDraft> {
        let matter = self.get_matter_summary(matter_id).await?;
        let parties = self.list_parties(matter_id).await.unwrap_or_default();
        let claims = self.list_claims(matter_id).await.unwrap_or_default();
        let facts = self.list_facts(matter_id).await.unwrap_or_default();
        let now = now_string();
        let title = request.title.unwrap_or_else(|| {
            let base = matter.short_name.clone().unwrap_or(matter.name.clone());
            format!("{base} complaint")
        });
        let complaint_id = generate_id("complaint", &title);
        let mut complaint = default_complaint_from_matter(
            &matter,
            &complaint_id,
            &title,
            &parties,
            &claims,
            &facts,
            &now,
        );
        if let Some(source_draft_id) = request.source_draft_id {
            complaint.history.push(complaint_event(
                matter_id,
                &complaint_id,
                "source_draft_linked",
                "draft",
                &source_draft_id,
                "Complaint initialized with a generic draft source.",
            ));
        }
        if let Some(template) = request.template {
            complaint.history.push(complaint_event(
                matter_id,
                &complaint_id,
                "template_selected",
                "complaint",
                &complaint_id,
                &format!("Template selected: {template}."),
            ));
        }
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn import_complaints(
        &self,
        matter_id: &str,
        request: ComplaintImportRequest,
    ) -> ApiResult<ComplaintImportResponse> {
        let mut document_ids = request.document_ids.clone();
        if let Some(document_id) = request.document_id.clone() {
            push_unique(&mut document_ids, document_id);
        }
        if document_ids.is_empty() {
            return Err(ApiError::BadRequest(
                "Complaint import requires at least one document_id".to_string(),
            ));
        }

        let mode = request
            .mode
            .clone()
            .unwrap_or_else(|| "structured_import".to_string());
        let mut imported = Vec::new();
        let mut skipped = Vec::new();
        let mut warnings = Vec::new();

        for document_id in document_ids {
            let title = request.title.clone();
            match self
                .import_complaint_from_document(
                    matter_id,
                    &document_id,
                    ComplaintImportRequest {
                        document_id: Some(document_id.clone()),
                        document_ids: Vec::new(),
                        title,
                        force: request.force,
                        mode: Some(mode.clone()),
                    },
                )
                .await
            {
                Ok(response) => {
                    imported.extend(response.imported);
                    skipped.extend(response.skipped);
                    warnings.extend(response.warnings);
                }
                Err(error) => skipped.push(ComplaintImportResult {
                    document_id,
                    complaint_id: None,
                    status: "failed".to_string(),
                    message: error.to_string(),
                    parser_id: "casebuilder-import-dispatch".to_string(),
                    likely_complaint: false,
                    complaint: None,
                }),
            }
        }

        Ok(ComplaintImportResponse {
            matter_id: matter_id.to_string(),
            mode,
            imported,
            skipped,
            warnings,
        })
    }

    pub async fn import_complaint_from_document(
        &self,
        matter_id: &str,
        document_id: &str,
        request: ComplaintImportRequest,
    ) -> ApiResult<ComplaintImportResponse> {
        let matter = self.get_matter_summary(matter_id).await?;
        let mut document = self.get_document(matter_id, document_id).await?;
        let provenance = self
            .ensure_document_original_provenance(matter_id, &mut document)
            .await?;
        let text = match document.extracted_text.clone() {
            Some(text) if !text.trim().is_empty() => text,
            _ => self.document_bytes_as_text(&document).await?,
        };
        let parser_id = parser_id_for_document(&document);
        let likely_complaint = looks_like_complaint(&document.filename, &text);
        let force = request.force.unwrap_or(false);
        let mode = request
            .mode
            .clone()
            .unwrap_or_else(|| "structured_import".to_string());

        if text.trim().is_empty() {
            return Ok(ComplaintImportResponse {
                matter_id: matter_id.to_string(),
                mode,
                imported: Vec::new(),
                skipped: vec![ComplaintImportResult {
                    document_id: document_id.to_string(),
                    complaint_id: None,
                    status: "no_extractable_text".to_string(),
                    message: "This document has no deterministic text to import yet.".to_string(),
                    parser_id,
                    likely_complaint,
                    complaint: None,
                }],
                warnings: vec!["OCR/transcription or a supported text parser is required before structured complaint import.".to_string()],
            });
        }
        if !likely_complaint && !force {
            return Ok(ComplaintImportResponse {
                matter_id: matter_id.to_string(),
                mode,
                imported: Vec::new(),
                skipped: vec![ComplaintImportResult {
                    document_id: document_id.to_string(),
                    complaint_id: None,
                    status: "not_likely_complaint".to_string(),
                    message: "Document did not meet the complaint-draft detection threshold."
                        .to_string(),
                    parser_id,
                    likely_complaint,
                    complaint: None,
                }],
                warnings: Vec::new(),
            });
        }

        let source_context = source_context_from_provenance(provenance.as_ref());
        let parties = self.list_parties(matter_id).await.unwrap_or_default();
        let claims = self.list_claims(matter_id).await.unwrap_or_default();
        let facts = self.list_facts(matter_id).await.unwrap_or_default();
        let evidence = self.list_evidence(matter_id).await.unwrap_or_default();
        let now = now_string();
        let title = request
            .title
            .clone()
            .unwrap_or_else(|| imported_complaint_title(&document, &text));
        let complaint_id = generate_id("complaint", &format!("{}:{}", document_id, title));
        let mut complaint = build_imported_complaint(
            &matter,
            &document,
            &complaint_id,
            &title,
            &text,
            &parser_id,
            &source_context,
            &parties,
            &claims,
            &facts,
            &evidence,
            &now,
        );
        let manifest_key = format!(
            "casebuilder/documents/{}/artifacts/complaint-import-{}.json",
            sanitize_path_segment(document_id),
            sanitize_path_segment(&complaint_id)
        );
        let manifest = serde_json::json!({
            "matter_id": matter_id,
            "document_id": document_id,
            "complaint_id": complaint_id,
            "parser_id": parser_id,
            "parser_version": PARSER_REGISTRY_VERSION,
            "chunker_version": CHUNKER_VERSION,
            "citation_resolver_version": CITATION_RESOLVER_VERSION,
            "index_version": CASE_INDEX_VERSION,
            "title": title,
            "paragraph_count": complaint.paragraphs.len(),
            "count_count": complaint.counts.len(),
            "citation_count": complaint.paragraphs.iter().map(|p| p.citation_uses.len()).sum::<usize>(),
        });
        let manifest_bytes =
            serde_json::to_vec(&manifest).map_err(|error| ApiError::Internal(error.to_string()))?;
        let manifest_hash = sha256_hex(&manifest_bytes);
        let stored_manifest = self
            .object_store
            .put_bytes(
                &manifest_key,
                Bytes::from(manifest_bytes),
                put_options(
                    Some("application/json".to_string()),
                    Some(manifest_hash.clone()),
                ),
            )
            .await?;

        let mut source_spans = Vec::new();
        for paragraph in &complaint.paragraphs {
            if let Some(provenance) = &paragraph.import_provenance {
                if let Some(source_span_id) = &provenance.source_span_id {
                    source_spans.push(SourceSpan {
                        source_span_id: source_span_id.clone(),
                        id: source_span_id.clone(),
                        matter_id: matter_id.to_string(),
                        document_id: document_id.to_string(),
                        document_version_id: provenance.document_version_id.clone(),
                        object_blob_id: provenance.object_blob_id.clone(),
                        ingestion_run_id: provenance.ingestion_run_id.clone(),
                        page: Some(1),
                        chunk_id: None,
                        byte_start: provenance.byte_start,
                        byte_end: provenance.byte_end,
                        char_start: provenance.char_start,
                        char_end: provenance.char_end,
                        time_start_ms: None,
                        time_end_ms: None,
                        speaker_label: None,
                        quote: Some(paragraph.text.clone()),
                        extraction_method: "complaint_import_paragraph".to_string(),
                        confidence: 0.82,
                        review_status: "unreviewed".to_string(),
                        unavailable_reason: None,
                    });
                }
            }
        }
        for span in &source_spans {
            self.merge_source_span(matter_id, span).await?;
        }
        document.extracted_text = Some(text.clone());
        document.processing_status = "review_ready".to_string();
        document.summary = format!(
            "Imported as structured complaint {} with {} paragraphs and {} citation uses.",
            complaint.complaint_id,
            complaint.paragraphs.len(),
            complaint
                .paragraphs
                .iter()
                .map(|paragraph| paragraph.citation_uses.len())
                .sum::<usize>()
        );
        document.citations_found = complaint
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.citation_uses.len() as u64)
            .sum();
        document.source_spans = source_spans.clone();
        let document = self
            .merge_node(matter_id, document_spec(), document_id, &document)
            .await?;

        let mut run = provenance
            .as_ref()
            .map(|value| {
                completed_ingestion_run(
                    &value.ingestion_run,
                    "review_ready",
                    "complaint_import",
                    complaint_import_node_ids(&complaint, &source_spans),
                )
            })
            .unwrap_or_else(|| IngestionRun {
                ingestion_run_id: primary_ingestion_run_id(document_id),
                id: primary_ingestion_run_id(document_id),
                matter_id: matter_id.to_string(),
                document_id: document_id.to_string(),
                document_version_id: None,
                object_blob_id: None,
                input_sha256: document.file_hash.clone(),
                status: "review_ready".to_string(),
                stage: "complaint_import".to_string(),
                mode: "deterministic".to_string(),
                started_at: now.clone(),
                completed_at: Some(now_string()),
                error_code: None,
                error_message: None,
                retryable: false,
                produced_node_ids: complaint_import_node_ids(&complaint, &source_spans),
                produced_object_keys: Vec::new(),
                parser_id: Some(parser_id.clone()),
                parser_version: Some(PARSER_REGISTRY_VERSION.to_string()),
                chunker_version: Some(CHUNKER_VERSION.to_string()),
                citation_resolver_version: Some(CITATION_RESOLVER_VERSION.to_string()),
                index_version: Some(CASE_INDEX_VERSION.to_string()),
            });
        run.parser_id = Some(parser_id.clone());
        run.parser_version = Some(PARSER_REGISTRY_VERSION.to_string());
        run.chunker_version = Some(CHUNKER_VERSION.to_string());
        run.citation_resolver_version = Some(CITATION_RESOLVER_VERSION.to_string());
        run.index_version = Some(CASE_INDEX_VERSION.to_string());
        push_unique(&mut run.produced_object_keys, stored_manifest.key.clone());
        self.merge_ingestion_run(matter_id, &run).await?;

        complaint.history.push(complaint_event(
            matter_id,
            &complaint.complaint_id,
            "complaint_imported",
            "document",
            document_id,
            &format!(
                "Structured complaint imported from uploaded document; manifest stored at {}.",
                stored_manifest.key
            ),
        ));
        refresh_complaint_state(&mut complaint);
        let complaint = self.save_complaint(matter_id, complaint).await?;

        Ok(ComplaintImportResponse {
            matter_id: matter_id.to_string(),
            mode,
            imported: vec![ComplaintImportResult {
                document_id: document.document_id,
                complaint_id: Some(complaint.complaint_id.clone()),
                status: "imported".to_string(),
                message: "Structured complaint import completed; human review is required."
                    .to_string(),
                parser_id,
                likely_complaint,
                complaint: Some(complaint),
            }],
            skipped: Vec::new(),
            warnings: Vec::new(),
        })
    }

    pub async fn get_complaint(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<ComplaintDraft> {
        self.get_node(matter_id, complaint_spec(), complaint_id)
            .await
    }

    pub async fn patch_complaint(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: PatchComplaintRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        if let Some(value) = request.title {
            complaint.title = value;
        }
        if let Some(value) = request.status {
            complaint.status = value;
        }
        if let Some(value) = request.review_status {
            complaint.review_status = value;
        }
        if let Some(value) = request.setup_stage {
            complaint.setup_stage = value;
        }
        if let Some(value) = request.caption {
            complaint.caption = value;
        }
        if let Some(value) = request.parties {
            complaint.parties = value;
        }
        if let Some(value) = request.sections {
            complaint.sections = value;
        }
        if let Some(value) = request.counts {
            complaint.counts = value;
        }
        if let Some(value) = request.paragraphs {
            complaint.paragraphs = value;
        }
        if let Some(value) = request.relief {
            complaint.relief = value;
        }
        if let Some(value) = request.signature {
            complaint.signature = value;
        }
        if let Some(value) = request.certificate_of_service {
            complaint.certificate_of_service = value;
        }
        if let Some(value) = request.formatting_profile {
            complaint.formatting_profile = value;
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "complaint_updated",
            "complaint",
            complaint_id,
            "Complaint metadata or AST was updated.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn update_complaint_setup(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: PatchComplaintRequest,
    ) -> ApiResult<ComplaintDraft> {
        self.patch_complaint(matter_id, complaint_id, request).await
    }

    pub async fn create_complaint_section(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: CreateComplaintSectionRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let section_id = format!(
            "{complaint_id}:section:{}",
            sanitize_path_segment(&request.title)
        );
        if !complaint
            .sections
            .iter()
            .any(|section| section.section_id == section_id)
        {
            complaint.sections.push(ComplaintSection {
                id: section_id.clone(),
                section_id: section_id.clone(),
                matter_id: matter_id.to_string(),
                complaint_id: complaint_id.to_string(),
                title: request.title,
                section_type: request.section_type.unwrap_or_else(|| "custom".to_string()),
                ordinal: complaint.sections.len() as u64 + 1,
                paragraph_ids: Vec::new(),
                count_ids: Vec::new(),
                review_status: "needs_review".to_string(),
            });
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "section_created",
            "section",
            &section_id,
            "Complaint section created.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn create_complaint_count(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: CreateComplaintCountRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let count_id = format!("{complaint_id}:count:{}", complaint.counts.len() + 1);
        let fact_ids = request
            .claim_id
            .as_ref()
            .and_then(|claim_id| {
                complaint
                    .counts
                    .iter()
                    .find(|count| count.claim_id.as_ref() == Some(claim_id))
                    .map(|count| count.fact_ids.clone())
            })
            .unwrap_or_default();
        complaint.counts.push(ComplaintCount {
            id: count_id.clone(),
            count_id: count_id.clone(),
            matter_id: matter_id.to_string(),
            complaint_id: complaint_id.to_string(),
            ordinal: complaint.counts.len() as u64 + 1,
            title: request.title.clone(),
            claim_id: request.claim_id,
            legal_theory: request.legal_theory.unwrap_or_default(),
            against_party_ids: request.against_party_ids.unwrap_or_default(),
            element_ids: request.element_ids.unwrap_or_default(),
            fact_ids,
            evidence_ids: Vec::new(),
            authorities: Vec::new(),
            relief_ids: request.relief_ids.unwrap_or_default(),
            paragraph_ids: Vec::new(),
            incorporation_range: Some("1 through preceding paragraph".to_string()),
            health: "needs_review".to_string(),
            weaknesses: Vec::new(),
        });
        let paragraph_text = format!("COUNT {} - {}", complaint.counts.len(), request.title);
        let paragraph_id = format!(
            "{complaint_id}:paragraph:{}",
            complaint.paragraphs.len() + 1
        );
        complaint.paragraphs.push(pleading_paragraph(
            matter_id,
            complaint_id,
            &paragraph_id,
            None,
            Some(count_id.clone()),
            "count_heading",
            &paragraph_text,
            complaint.paragraphs.len() as u64 + 1,
            Vec::new(),
            Vec::new(),
        ));
        if let Some(count) = complaint
            .counts
            .iter_mut()
            .find(|count| count.count_id == count_id)
        {
            count.paragraph_ids.push(paragraph_id.clone());
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "count_created",
            "count",
            &count_id,
            "Complaint count created.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn create_complaint_paragraph(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: CreateComplaintParagraphRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let paragraph_id = format!(
            "{complaint_id}:paragraph:{}",
            complaint.paragraphs.len() + 1
        );
        let paragraph = pleading_paragraph(
            matter_id,
            complaint_id,
            &paragraph_id,
            request.section_id.clone(),
            request.count_id.clone(),
            request.role.as_deref().unwrap_or("factual_allegation"),
            &request.text,
            complaint.paragraphs.len() as u64 + 1,
            request.fact_ids.unwrap_or_default(),
            request.evidence_ids.unwrap_or_default(),
        );
        if let Some(section_id) = &request.section_id {
            if let Some(section) = complaint
                .sections
                .iter_mut()
                .find(|section| &section.section_id == section_id)
            {
                push_unique(&mut section.paragraph_ids, paragraph_id.clone());
            }
        }
        if let Some(count_id) = &request.count_id {
            if let Some(count) = complaint
                .counts
                .iter_mut()
                .find(|count| &count.count_id == count_id)
            {
                push_unique(&mut count.paragraph_ids, paragraph_id.clone());
            }
        }
        complaint.paragraphs.push(paragraph);
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "paragraph_created",
            "paragraph",
            &paragraph_id,
            "Pleading paragraph created.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn patch_complaint_paragraph(
        &self,
        matter_id: &str,
        complaint_id: &str,
        paragraph_id: &str,
        request: PatchComplaintParagraphRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let paragraph = complaint
            .paragraphs
            .iter_mut()
            .find(|paragraph| paragraph.paragraph_id == paragraph_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("Pleading paragraph {paragraph_id} not found"))
            })?;
        if paragraph.locked && request.text.is_some() {
            return Err(ApiError::BadRequest(format!(
                "Pleading paragraph {paragraph_id} is locked"
            )));
        }
        if let Some(value) = request.section_id {
            paragraph.section_id = Some(value);
        }
        if let Some(value) = request.count_id {
            paragraph.count_id = Some(value);
        }
        if let Some(value) = request.role {
            paragraph.role = value;
        }
        if let Some(value) = request.text {
            paragraph.text = value;
            paragraph.sentences = pleading_sentences(
                matter_id,
                complaint_id,
                paragraph_id,
                &paragraph.text,
                &paragraph.fact_ids,
            );
        }
        if let Some(value) = request.fact_ids {
            paragraph.fact_ids = value;
            paragraph.sentences = pleading_sentences(
                matter_id,
                complaint_id,
                paragraph_id,
                &paragraph.text,
                &paragraph.fact_ids,
            );
        }
        if let Some(value) = request.evidence_uses {
            paragraph.evidence_uses = value;
        }
        if let Some(value) = request.citation_uses {
            paragraph.citation_uses = value;
        }
        if let Some(value) = request.exhibit_references {
            paragraph.exhibit_references = value;
        }
        if let Some(value) = request.locked {
            paragraph.locked = value;
        }
        if let Some(value) = request.review_status {
            paragraph.review_status = value;
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "paragraph_updated",
            "paragraph",
            paragraph_id,
            "Pleading paragraph updated.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn renumber_complaint_paragraphs(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        renumber_paragraphs(&mut complaint.paragraphs);
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "paragraphs_renumbered",
            "complaint",
            complaint_id,
            "Pleading paragraphs renumbered without changing stable IDs.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn link_complaint_support(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: ComplaintLinkRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        self.validate_complaint_link_references(matter_id, &request)
            .await?;
        match request.target_type.as_str() {
            "paragraph" | "sentence" => {
                let paragraph_id = if request.target_type == "paragraph" {
                    request.target_id.clone()
                } else {
                    complaint
                        .paragraphs
                        .iter()
                        .find(|paragraph| {
                            paragraph
                                .sentences
                                .iter()
                                .any(|sentence| sentence.sentence_id == request.target_id)
                        })
                        .map(|paragraph| paragraph.paragraph_id.clone())
                        .ok_or_else(|| {
                            ApiError::NotFound(format!(
                                "Complaint target {} not found",
                                request.target_id
                            ))
                        })?
                };
                let paragraph = complaint
                    .paragraphs
                    .iter_mut()
                    .find(|paragraph| paragraph.paragraph_id == paragraph_id)
                    .ok_or_else(|| {
                        ApiError::NotFound(format!("Pleading paragraph {paragraph_id} not found"))
                    })?;
                if let Some(fact_id) = request.fact_id.clone() {
                    push_unique(&mut paragraph.fact_ids, fact_id);
                }
                if request.evidence_id.is_some()
                    || request.document_id.is_some()
                    || request.source_span_id.is_some()
                {
                    let id = format!(
                        "{}:evidence-use:{}",
                        paragraph.paragraph_id,
                        paragraph.evidence_uses.len() + 1
                    );
                    paragraph.evidence_uses.push(EvidenceUse {
                        id: id.clone(),
                        evidence_use_id: id,
                        matter_id: matter_id.to_string(),
                        complaint_id: complaint_id.to_string(),
                        target_type: request.target_type.clone(),
                        target_id: request.target_id.clone(),
                        fact_id: request.fact_id.clone(),
                        evidence_id: request.evidence_id.clone(),
                        document_id: request.document_id.clone(),
                        source_span_id: request.source_span_id.clone(),
                        relation: request
                            .relation
                            .clone()
                            .unwrap_or_else(|| "supports".to_string()),
                        quote: request.quote.clone(),
                        status: "needs_review".to_string(),
                    });
                }
                if let Some(citation) = request.citation.clone() {
                    let id = format!(
                        "{}:citation-use:{}",
                        paragraph.paragraph_id,
                        paragraph.citation_uses.len() + 1
                    );
                    paragraph.citation_uses.push(CitationUse {
                        id: id.clone(),
                        citation_use_id: id,
                        matter_id: matter_id.to_string(),
                        complaint_id: complaint_id.to_string(),
                        target_type: request.target_type.clone(),
                        target_id: request.target_id.clone(),
                        citation,
                        canonical_id: request.canonical_id.clone(),
                        pinpoint: request.pinpoint.clone(),
                        quote: request.quote.clone(),
                        status: if request.canonical_id.is_some() {
                            "resolved".to_string()
                        } else {
                            "unresolved".to_string()
                        },
                        currentness: "needs_review".to_string(),
                        scope_warning: None,
                    });
                }
                if let Some(exhibit_label) = request.exhibit_label.clone() {
                    let id = format!(
                        "{}:exhibit-reference:{}",
                        paragraph.paragraph_id,
                        paragraph.exhibit_references.len() + 1
                    );
                    paragraph.exhibit_references.push(ExhibitReference {
                        id: id.clone(),
                        exhibit_reference_id: id,
                        matter_id: matter_id.to_string(),
                        complaint_id: complaint_id.to_string(),
                        target_type: request.target_type.clone(),
                        target_id: request.target_id.clone(),
                        exhibit_label,
                        document_id: request.document_id.clone(),
                        evidence_id: request.evidence_id.clone(),
                        status: if request.document_id.is_some() || request.evidence_id.is_some() {
                            "linked".to_string()
                        } else {
                            "missing".to_string()
                        },
                    });
                }
                paragraph.sentences = pleading_sentences(
                    matter_id,
                    complaint_id,
                    &paragraph.paragraph_id,
                    &paragraph.text,
                    &paragraph.fact_ids,
                );
            }
            "count" => {
                let count = complaint
                    .counts
                    .iter_mut()
                    .find(|count| count.count_id == request.target_id)
                    .ok_or_else(|| {
                        ApiError::NotFound(format!(
                            "Complaint count {} not found",
                            request.target_id
                        ))
                    })?;
                if let Some(fact_id) = request.fact_id {
                    push_unique(&mut count.fact_ids, fact_id);
                }
                if let Some(evidence_id) = request.evidence_id {
                    push_unique(&mut count.evidence_ids, evidence_id);
                }
                if let Some(citation) = request.citation {
                    push_authority(
                        &mut count.authorities,
                        AuthorityRef {
                            citation,
                            canonical_id: request
                                .canonical_id
                                .unwrap_or_else(|| request.target_id.clone()),
                            reason: request.quote,
                            pinpoint: request.pinpoint,
                        },
                    );
                }
            }
            value => {
                return Err(ApiError::BadRequest(format!(
                    "Unsupported complaint link target_type {value}"
                )));
            }
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "support_linked",
            &request.target_type,
            &request.target_id,
            "Support, authority, citation, or exhibit link added.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn run_complaint_qc(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<AiActionResponse<Vec<RuleCheckFinding>>> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let findings = complaint_rule_findings(&complaint);
        complaint.findings = findings.clone();
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "qc_run",
            "complaint",
            complaint_id,
            "Deterministic complaint QC run completed.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await?;
        Ok(AiActionResponse {
            enabled: false,
            mode: "deterministic".to_string(),
            message: "No live rule provider is configured; ran deterministic Oregon complaint checks. Human review is required.".to_string(),
            result: Some(findings),
        })
    }

    pub async fn list_complaint_findings(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<Vec<RuleCheckFinding>> {
        Ok(self.get_complaint(matter_id, complaint_id).await?.findings)
    }

    pub async fn patch_complaint_finding(
        &self,
        matter_id: &str,
        complaint_id: &str,
        finding_id: &str,
        request: PatchRuleFindingRequest,
    ) -> ApiResult<ComplaintDraft> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let finding = complaint
            .findings
            .iter_mut()
            .find(|finding| finding.finding_id == finding_id)
            .ok_or_else(|| ApiError::NotFound(format!("Rule finding {finding_id} not found")))?;
        finding.status = request.status;
        finding.updated_at = now_string();
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "qc_finding_status_changed",
            "finding",
            finding_id,
            "Complaint QC finding status changed.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await
    }

    pub async fn preview_complaint(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<ComplaintPreviewResponse> {
        let complaint = self.get_complaint(matter_id, complaint_id).await?;
        Ok(render_complaint_preview(&complaint))
    }

    pub async fn export_complaint(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: ExportComplaintRequest,
    ) -> ApiResult<ExportArtifact> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let format = request.format.to_ascii_lowercase();
        let supported = [
            "pdf",
            "docx",
            "html",
            "markdown",
            "text",
            "plain_text",
            "json",
        ];
        if !supported.contains(&format.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "Unsupported complaint export format {format}"
            )));
        }
        let profile = request
            .profile
            .unwrap_or_else(|| "clean_filing_copy".to_string());
        let mode = request.mode.unwrap_or_else(|| "review_needed".to_string());
        let rendered = export_complaint_content(
            &complaint,
            &format,
            request.include_exhibits.unwrap_or(true),
            request.include_qc_report.unwrap_or(true),
        )?;
        let artifact_id = format!(
            "{}:artifact:{}:{}",
            complaint_id,
            sanitize_path_segment(&format),
            complaint.export_artifacts.len() + 1
        );
        let warnings = export_warnings(&complaint, &format);
        let artifact = ExportArtifact {
            id: artifact_id.clone(),
            artifact_id: artifact_id.clone(),
            matter_id: matter_id.to_string(),
            complaint_id: complaint_id.to_string(),
            format: format.clone(),
            profile,
            mode,
            status: if matches!(format.as_str(), "pdf" | "docx") {
                "skeleton_review_needed".to_string()
            } else {
                "generated_review_needed".to_string()
            },
            download_url: format!(
                "/api/v1/matters/{}/complaints/{}/artifacts/{}/download",
                matter_id, complaint_id, artifact_id
            ),
            page_count: render_complaint_preview(&complaint).page_count,
            generated_at: now_string(),
            warnings,
            content_preview: rendered,
            object_blob_id: None,
            size_bytes: None,
            mime_type: Some(export_mime_type(&format).to_string()),
            storage_status: Some("legacy_inline".to_string()),
        };
        complaint.export_artifacts.push(artifact.clone());
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "export_generated",
            "export_artifact",
            &artifact_id,
            "Complaint export artifact generated for review.",
        ));
        refresh_complaint_state(&mut complaint);
        self.save_complaint(matter_id, complaint).await?;
        self.merge_node(
            matter_id,
            complaint_artifact_spec(),
            &artifact.artifact_id,
            &artifact,
        )
        .await?;
        Ok(artifact)
    }

    pub async fn get_complaint_artifact(
        &self,
        matter_id: &str,
        complaint_id: &str,
        artifact_id: &str,
    ) -> ApiResult<ExportArtifact> {
        let complaint = self.get_complaint(matter_id, complaint_id).await?;
        complaint
            .export_artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == artifact_id)
            .ok_or_else(|| ApiError::NotFound(format!("Export artifact {artifact_id} not found")))
    }

    pub async fn download_complaint_artifact(
        &self,
        matter_id: &str,
        complaint_id: &str,
        artifact_id: &str,
    ) -> ApiResult<ComplaintDownloadResponse> {
        let artifact = self
            .get_complaint_artifact(matter_id, complaint_id, artifact_id)
            .await?;
        Ok(ComplaintDownloadResponse {
            method: "GET".to_string(),
            url: artifact.download_url.clone(),
            expires_at: timestamp_after(self.download_ttl_seconds),
            headers: BTreeMap::new(),
            filename: format!(
                "{}.{}",
                sanitize_path_segment(&artifact.complaint_id),
                artifact.format
            ),
            mime_type: Some(export_mime_type(&artifact.format).to_string()),
            bytes: artifact.content_preview.as_bytes().len() as u64,
            artifact,
        })
    }

    pub async fn run_complaint_ai_command(
        &self,
        matter_id: &str,
        complaint_id: &str,
        request: ComplaintAiCommandRequest,
    ) -> ApiResult<AiActionResponse<ComplaintDraft>> {
        let mut complaint = self.get_complaint(matter_id, complaint_id).await?;
        let target = request
            .target_id
            .unwrap_or_else(|| complaint_id.to_string());
        let command_label = request.command.replace('_', " ");
        let warning = format!(
            "Provider-free template mode recorded command '{command_label}'. No unsupported facts or legal conclusions were inserted."
        );
        for command in &mut complaint.ai_commands {
            if command.command_id == request.command {
                command.last_message = Some(warning.clone());
                command.status = "template_available".to_string();
            }
        }
        complaint.history.push(complaint_event(
            matter_id,
            complaint_id,
            "ai_command_template",
            "complaint",
            &target,
            &warning,
        ));
        if request.prompt.is_some() {
            complaint.history.push(complaint_event(
                matter_id,
                complaint_id,
                "ai_prompt_recorded",
                "complaint",
                &target,
                "Prompt recorded for human review.",
            ));
        }
        refresh_complaint_state(&mut complaint);
        let complaint = self.save_complaint(matter_id, complaint).await?;
        Ok(AiActionResponse {
            enabled: false,
            mode: "template".to_string(),
            message: warning,
            result: Some(complaint),
        })
    }

    pub async fn filing_packet(
        &self,
        matter_id: &str,
        complaint_id: &str,
    ) -> ApiResult<FilingPacket> {
        Ok(self
            .get_complaint(matter_id, complaint_id)
            .await?
            .filing_packet)
    }

    pub(super) async fn save_complaint(
        &self,
        matter_id: &str,
        mut complaint: ComplaintDraft,
    ) -> ApiResult<ComplaintDraft> {
        let before_product = self
            .get_node::<WorkProduct>(matter_id, work_product_spec(), &complaint.complaint_id)
            .await
            .ok();
        complaint.updated_at = now_string();
        let complaint = self
            .merge_node(
                matter_id,
                complaint_spec(),
                &complaint.complaint_id,
                &complaint,
            )
            .await?;
        self.materialize_complaint_edges(&complaint).await?;
        for finding in &complaint.findings {
            self.merge_node(
                matter_id,
                complaint_finding_spec(),
                &finding.finding_id,
                finding,
            )
            .await?;
        }
        for artifact in &complaint.export_artifacts {
            self.merge_node(
                matter_id,
                complaint_artifact_spec(),
                &artifact.artifact_id,
                artifact,
            )
            .await?;
        }
        let product = work_product_from_complaint(&complaint);
        let version_changes = work_product_facade_change_inputs(before_product.as_ref(), &product);
        self.save_work_product_internal(matter_id, product.clone())
            .await?;
        if !version_changes.is_empty() {
            let snapshot_type = if before_product.is_none() {
                "auto"
            } else if version_changes
                .iter()
                .any(|change| change.target_type == "export")
            {
                "export"
            } else if version_changes
                .iter()
                .any(|change| change.target_type == "rule_finding")
            {
                "rule_check"
            } else {
                "auto"
            };
            let title = if before_product.is_none() {
                "Complaint created"
            } else {
                "Complaint updated"
            };
            let change_set = self
                .record_work_product_change(
                    matter_id,
                    before_product.as_ref(),
                    &product,
                    "user",
                    snapshot_type,
                    title,
                    "Complaint facade synchronized to canonical Case History.",
                    version_changes,
                )
                .await?;
            if snapshot_type == "export" {
                let mut locked_product = product;
                let qc_status_at_export = work_product_qc_status(&locked_product);
                let mut changed = false;
                for artifact in &mut locked_product.artifacts {
                    if artifact.snapshot_id.is_none() {
                        artifact.snapshot_id = Some(change_set.snapshot_id.clone());
                        artifact.qc_status_at_export = Some(qc_status_at_export.clone());
                        artifact.changed_since_export = Some(false);
                        artifact.immutable = Some(true);
                        changed = true;
                    }
                }
                if changed {
                    self.save_work_product_internal(matter_id, locked_product)
                        .await?;
                }
            }
        }
        Ok(complaint)
    }

    pub(super) async fn save_complaint_projection_only(
        &self,
        matter_id: &str,
        mut complaint: ComplaintDraft,
    ) -> ApiResult<ComplaintDraft> {
        complaint.updated_at = now_string();
        let complaint = self
            .merge_node(
                matter_id,
                complaint_spec(),
                &complaint.complaint_id,
                &complaint,
            )
            .await?;
        self.materialize_complaint_edges(&complaint).await?;
        for finding in &complaint.findings {
            self.merge_node(
                matter_id,
                complaint_finding_spec(),
                &finding.finding_id,
                finding,
            )
            .await?;
        }
        for artifact in &complaint.export_artifacts {
            self.merge_node(
                matter_id,
                complaint_artifact_spec(),
                &artifact.artifact_id,
                artifact,
            )
            .await?;
        }
        Ok(complaint)
    }

    pub(super) async fn sync_complaint_projection_from_work_product(
        &self,
        matter_id: &str,
        product: &WorkProduct,
    ) -> ApiResult<()> {
        if product.product_type != "complaint" {
            return Ok(());
        }
        let mut complaint = match self
            .get_node::<ComplaintDraft>(matter_id, complaint_spec(), &product.work_product_id)
            .await
        {
            Ok(complaint) => complaint,
            Err(ApiError::NotFound(_)) => return Ok(()),
            Err(error) => return Err(error),
        };
        complaint.title = product.title.clone();
        complaint.status = product.status.clone();
        complaint.review_status = product.review_status.clone();
        complaint.setup_stage = product.setup_stage.clone();
        complaint.formatting_profile = product.formatting_profile.clone();
        complaint.rule_pack = product.rule_pack.clone();

        for section in &mut complaint.sections {
            if let Some(block) = product
                .blocks
                .iter()
                .find(|block| block.block_id == section.section_id)
            {
                section.title = block.title.clone();
                section.review_status = block.review_status.clone();
            }
        }
        for count in &mut complaint.counts {
            if let Some(block) = product
                .blocks
                .iter()
                .find(|block| block.block_id == count.count_id)
            {
                count.title = block.title.clone();
                count.legal_theory = block.text.clone();
                count.fact_ids = block.fact_ids.clone();
                count.evidence_ids = block.evidence_ids.clone();
                count.authorities = block.authorities.clone();
                count.health = block.review_status.clone();
            }
        }
        for paragraph in &mut complaint.paragraphs {
            if let Some(block) = product
                .blocks
                .iter()
                .find(|block| block.block_id == paragraph.paragraph_id)
            {
                paragraph.text = block.text.clone();
                paragraph.fact_ids = block.fact_ids.clone();
                paragraph.locked = block.locked;
                paragraph.review_status = block.review_status.clone();
                paragraph.evidence_uses = block
                    .evidence_ids
                    .iter()
                    .enumerate()
                    .map(|(index, evidence_id)| {
                        let id = format!("{}:evidence:{}", paragraph.paragraph_id, index + 1);
                        EvidenceUse {
                            evidence_use_id: id.clone(),
                            id,
                            matter_id: matter_id.to_string(),
                            complaint_id: complaint.complaint_id.clone(),
                            target_type: "paragraph".to_string(),
                            target_id: paragraph.paragraph_id.clone(),
                            fact_id: None,
                            evidence_id: Some(evidence_id.clone()),
                            document_id: None,
                            source_span_id: None,
                            relation: "supports".to_string(),
                            quote: None,
                            status: "linked".to_string(),
                        }
                    })
                    .collect();
                paragraph.citation_uses = block
                    .authorities
                    .iter()
                    .enumerate()
                    .map(|(index, authority)| {
                        let id = format!("{}:citation:{}", paragraph.paragraph_id, index + 1);
                        CitationUse {
                            citation_use_id: id.clone(),
                            id,
                            matter_id: matter_id.to_string(),
                            complaint_id: complaint.complaint_id.clone(),
                            target_type: "paragraph".to_string(),
                            target_id: paragraph.paragraph_id.clone(),
                            citation: authority.citation.clone(),
                            canonical_id: Some(authority.canonical_id.clone()),
                            pinpoint: authority.pinpoint.clone(),
                            quote: None,
                            status: "inserted".to_string(),
                            currentness: "unchecked".to_string(),
                            scope_warning: None,
                        }
                    })
                    .collect();
                paragraph.sentences = pleading_sentences(
                    matter_id,
                    &complaint.complaint_id,
                    &paragraph.paragraph_id,
                    &paragraph.text,
                    &paragraph.fact_ids,
                );
            }
        }
        complaint.findings = product
            .findings
            .iter()
            .map(|finding| RuleCheckFinding {
                id: finding.finding_id.clone(),
                finding_id: finding.finding_id.clone(),
                matter_id: finding.matter_id.clone(),
                complaint_id: complaint.complaint_id.clone(),
                rule_id: finding.rule_id.clone(),
                category: finding.category.clone(),
                severity: finding.severity.clone(),
                target_type: finding.target_type.clone(),
                target_id: finding.target_id.clone(),
                message: finding.message.clone(),
                explanation: finding.explanation.clone(),
                suggested_fix: finding.suggested_fix.clone(),
                primary_action: ComplaintAction {
                    action_id: finding.primary_action.action_id.clone(),
                    label: finding.primary_action.label.clone(),
                    action_type: finding.primary_action.action_type.clone(),
                    href: finding.primary_action.href.clone(),
                    target_type: finding.primary_action.target_type.clone(),
                    target_id: finding.primary_action.target_id.clone(),
                },
                status: finding.status.clone(),
                created_at: finding.created_at.clone(),
                updated_at: finding.updated_at.clone(),
            })
            .collect();
        refresh_complaint_state(&mut complaint);
        self.save_complaint_projection_only(matter_id, complaint)
            .await?;
        Ok(())
    }
}
