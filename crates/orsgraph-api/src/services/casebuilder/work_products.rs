use super::*;

impl CaseBuilderService {
    pub async fn list_work_products(&self, matter_id: &str) -> ApiResult<Vec<WorkProduct>> {
        self.require_matter(matter_id).await?;
        self.migrate_legacy_drafts_to_work_products(matter_id)
            .await?;
        self.migrate_complaints_to_work_products(matter_id).await?;
        let mut products = self
            .list_nodes::<WorkProduct>(matter_id, work_product_spec())
            .await?;
        for product in &mut products {
            refresh_work_product_state(product);
        }
        Ok(products)
    }

    pub async fn list_work_products_for_api(
        &self,
        matter_id: &str,
        include_document_ast: bool,
    ) -> ApiResult<Vec<WorkProduct>> {
        let mut products = self.list_work_products(matter_id).await?;
        if !include_document_ast {
            for product in &mut products {
                summarize_work_product_for_list(product);
            }
        }
        Ok(products)
    }

    pub async fn create_work_product(
        &self,
        matter_id: &str,
        request: CreateWorkProductRequest,
    ) -> ApiResult<WorkProduct> {
        let product_type = normalize_work_product_type(&request.product_type)?;
        let title = request
            .title
            .clone()
            .unwrap_or_else(|| format!("{} work product", humanize_product_type(&product_type)));
        let work_product_id = generate_id("work-product", &title);
        self.create_work_product_with_id(matter_id, &work_product_id, request)
            .await
    }

    pub(super) async fn create_work_product_with_id(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: CreateWorkProductRequest,
    ) -> ApiResult<WorkProduct> {
        let matter = self.get_matter_summary(matter_id).await?;
        let facts = self.list_facts(matter_id).await.unwrap_or_default();
        let claims = self.list_claims(matter_id).await.unwrap_or_default();
        let product_type = normalize_work_product_type(&request.product_type)?;
        let now = now_string();
        let title = request
            .title
            .unwrap_or_else(|| format!("{} work product", humanize_product_type(&product_type)));
        let mut product = default_work_product_from_matter(
            &matter,
            work_product_id,
            &title,
            &product_type,
            &facts,
            &claims,
            &now,
        );
        product.source_draft_id = request.source_draft_id;
        product.source_complaint_id = request.source_complaint_id;
        if let Some(template) = request.template {
            product.document_ast.metadata.template_id = Some(template.clone());
            product.history.push(work_product_event(
                matter_id,
                work_product_id,
                "template_selected",
                "work_product",
                work_product_id,
                &format!("Template selected: {template}."),
            ));
        }
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            None,
            &product,
            "editor",
            "auto",
            "Work product created",
            "Shared WorkProduct AST created.",
            vec![VersionChangeInput {
                target_type: "work_product".to_string(),
                target_id: work_product_id.to_string(),
                operation: "create".to_string(),
                before: None,
                after: json_value(&product).ok(),
                summary: "Work product created.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn get_work_product(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<WorkProduct> {
        match self
            .get_node(matter_id, work_product_spec(), work_product_id)
            .await
        {
            Ok(mut product) => {
                refresh_work_product_state(&mut product);
                Ok(product)
            }
            Err(ApiError::NotFound(_)) => {
                self.migrate_legacy_drafts_to_work_products(matter_id)
                    .await?;
                self.migrate_complaints_to_work_products(matter_id).await?;
                let mut product = self
                    .get_node(matter_id, work_product_spec(), work_product_id)
                    .await?;
                refresh_work_product_state(&mut product);
                Ok(product)
            }
            Err(error) => Err(error),
        }
    }

    pub async fn patch_work_product(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: PatchWorkProductRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        if let Some(value) = request.title {
            product.title = value;
        }
        if let Some(value) = request.status {
            product.status = value;
        }
        if let Some(value) = request.review_status {
            product.review_status = value;
        }
        if let Some(value) = request.setup_stage {
            product.setup_stage = value;
        }
        if let Some(value) = request.document_ast {
            product.document_ast = value;
            normalize_work_product_ast(&mut product);
            product.blocks = flatten_work_product_blocks(&product.document_ast.blocks);
            product.marks.clear();
            product.anchors.clear();
        }
        if let Some(value) = request.blocks {
            product.blocks = value;
            super::work_product_ast::rebuild_work_product_ast_from_projection(&mut product);
        }
        if let Some(value) = request.marks {
            product.marks = value;
            super::work_product_ast::rebuild_work_product_ast_from_projection(&mut product);
        }
        if let Some(value) = request.anchors {
            product.anchors = value;
            super::work_product_ast::rebuild_work_product_ast_from_projection(&mut product);
        }
        if let Some(value) = request.formatting_profile {
            product.formatting_profile = value;
        }
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "work_product_updated",
            "work_product",
            work_product_id,
            "Work product metadata or AST was updated.",
        ));
        refresh_work_product_state(&mut product);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "editor",
            "auto",
            "Work product updated",
            "Work product metadata or AST was updated.",
            vec![VersionChangeInput {
                target_type: "work_product".to_string(),
                target_id: work_product_id.to_string(),
                operation: "update".to_string(),
                before: json_value(&before_product).ok(),
                after: json_value(&product).ok(),
                summary: "Work product metadata or AST was updated.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn apply_work_product_ast_patch(
        &self,
        matter_id: &str,
        work_product_id: &str,
        patch: AstPatch,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        normalize_work_product_ast(&mut product);
        let current_document_hash = work_product_hashes(&product)?.document_hash;
        let current_snapshot_id = if patch.base_snapshot_id.is_some() {
            self.latest_work_product_snapshot_id(matter_id, work_product_id)
                .await?
        } else {
            None
        };
        validate_ast_patch_concurrency(
            &patch,
            work_product_id,
            &current_document_hash,
            current_snapshot_id.as_deref(),
        )?;
        self.validate_ast_patch_matter_references(matter_id, &product, &patch)
            .await?;
        product.document_ast = super::ast_patch::apply_ast_patch_atomic(&product, &patch)?;
        normalize_work_product_ast(&mut product);
        let validation = super::ast_validation::validate_work_product_document(&product);
        if !validation.errors.is_empty() {
            let codes = validation
                .errors
                .iter()
                .map(|issue| issue.code.clone())
                .collect::<Vec<_>>()
                .join(",");
            return Err(ApiError::BadRequest(format!(
                "AST patch failed validation: issue_codes={codes}"
            )));
        }
        product.blocks = flatten_work_product_blocks(&product.document_ast.blocks);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "ast_patch_applied",
            "document_ast",
            &product.document_ast.document_id,
            "AST patch applied.",
        ));
        refresh_work_product_state(&mut product);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "editor",
            "ast_patch",
            "AST patch applied",
            "AST patch applied.",
            vec![VersionChangeInput {
                target_type: "document_ast".to_string(),
                target_id: product.document_ast.document_id.clone(),
                operation: "patch".to_string(),
                before: json_value(&before_product.document_ast).ok(),
                after: json_value(&product.document_ast).ok(),
                summary: "AST patch applied.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn validate_work_product_ast(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<AstValidationResponse> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        Ok(super::ast_validation::validate_work_product_document(
            &product,
        ))
    }

    pub async fn get_work_product_ast(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<WorkProductDocument> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        super::work_product_ast::normalize_work_product_ast(&mut product);
        Ok(product.document_ast)
    }

    pub async fn patch_work_product_ast(
        &self,
        matter_id: &str,
        work_product_id: &str,
        document_ast: WorkProductDocument,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        product.document_ast = document_ast;
        super::work_product_ast::normalize_work_product_ast(&mut product);
        super::ast_validation::ensure_work_product_ast_valid(&product, "Work product AST patch")?;
        product.blocks =
            super::work_product_ast::flatten_work_product_blocks(&product.document_ast.blocks);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "ast_replaced",
            "document_ast",
            &product.document_ast.document_id,
            "Canonical WorkProduct AST replaced.",
        ));
        refresh_work_product_state(&mut product);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "editor",
            "ast_patch",
            "AST replaced",
            "Canonical WorkProduct AST replaced.",
            vec![VersionChangeInput {
                target_type: "document_ast".to_string(),
                target_id: product.document_ast.document_id.clone(),
                operation: "replace".to_string(),
                before: json_value(&before_product.document_ast).ok(),
                after: json_value(&product.document_ast).ok(),
                summary: "Canonical WorkProduct AST replaced.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn work_product_ast_to_markdown(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<AstMarkdownResponse> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        super::work_product_ast::normalize_work_product_ast(&mut product);
        super::ast_validation::ensure_work_product_ast_valid(
            &product,
            "Work product AST to Markdown",
        )?;
        Ok(AstMarkdownResponse {
            markdown: super::markdown_adapter::work_product_markdown(&product),
            warnings: super::ast_validation::validate_work_product_document(&product)
                .warnings
                .into_iter()
                .map(|issue| issue.message)
                .collect(),
        })
    }

    pub async fn work_product_ast_from_markdown(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: MarkdownToAstRequest,
    ) -> ApiResult<AstDocumentResponse> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        let (document_ast, mut warnings) =
            super::markdown_adapter::markdown_to_work_product_ast(&product, &request.markdown);
        let mut converted = product.clone();
        converted.document_ast = document_ast;
        super::work_product_ast::normalize_work_product_ast(&mut converted);
        let validation = super::ast_validation::validate_work_product_document(&converted);
        if !validation.errors.is_empty() {
            let codes = validation
                .errors
                .iter()
                .map(|issue| issue.code.clone())
                .collect::<Vec<_>>()
                .join(",");
            return Err(ApiError::BadRequest(format!(
                "Work product Markdown import failed AST validation: issue_codes={codes}"
            )));
        }
        warnings.extend(validation.warnings.into_iter().map(|issue| issue.message));
        Ok(AstDocumentResponse {
            document_ast: converted.document_ast,
            warnings,
        })
    }

    pub async fn work_product_ast_to_html(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<AstRenderedResponse> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        super::work_product_ast::normalize_work_product_ast(&mut product);
        super::ast_validation::ensure_work_product_ast_valid(&product, "Work product AST to HTML")?;
        Ok(AstRenderedResponse {
            html: Some(super::html_renderer::render_work_product_preview(&product).html),
            plain_text: None,
            warnings: super::ast_validation::validate_work_product_document(&product)
                .warnings
                .into_iter()
                .map(|issue| issue.message)
                .collect(),
        })
    }

    pub async fn work_product_ast_to_plain_text(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<AstRenderedResponse> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        super::work_product_ast::normalize_work_product_ast(&mut product);
        super::ast_validation::ensure_work_product_ast_valid(
            &product,
            "Work product AST to plain text",
        )?;
        Ok(AstRenderedResponse {
            html: None,
            plain_text: Some(super::html_renderer::work_product_plain_text(&product)),
            warnings: super::ast_validation::validate_work_product_document(&product)
                .warnings
                .into_iter()
                .map(|issue| issue.message)
                .collect(),
        })
    }

    pub async fn create_work_product_block(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: CreateWorkProductBlockRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let role = request.role.unwrap_or_else(|| "custom".to_string());
        let block_id = format!(
            "{work_product_id}:block:{}",
            product.blocks.len().saturating_add(1)
        );
        product.blocks.push(WorkProductBlock {
            id: block_id.clone(),
            block_id: block_id.clone(),
            matter_id: matter_id.to_string(),
            work_product_id: work_product_id.to_string(),
            block_type: request
                .block_type
                .unwrap_or_else(|| "paragraph".to_string()),
            role: role.clone(),
            title: request
                .title
                .unwrap_or_else(|| humanize_product_type(&role)),
            text: request.text,
            ordinal: product.blocks.len() as u64 + 1,
            parent_block_id: request.parent_block_id,
            fact_ids: request.fact_ids.unwrap_or_default(),
            evidence_ids: request.evidence_ids.unwrap_or_default(),
            authorities: request.authorities.unwrap_or_default(),
            mark_ids: Vec::new(),
            locked: false,
            review_status: "needs_review".to_string(),
            prosemirror_json: None,
            ..WorkProductBlock::default()
        });
        super::work_product_ast::rebuild_work_product_ast_from_projection(&mut product);
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "block_created",
            "block",
            &block_id,
            "Work product block created.",
        ));
        refresh_work_product_state(&mut product);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        let product = self.save_work_product(matter_id, product).await?;
        let after_block = product
            .blocks
            .iter()
            .find(|block| block.block_id == block_id)
            .cloned();
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "editor",
            "auto",
            "Block created",
            "Work product block created.",
            vec![VersionChangeInput {
                target_type: "block".to_string(),
                target_id: block_id.clone(),
                operation: "create".to_string(),
                before: None,
                after: after_block
                    .as_ref()
                    .and_then(|block| json_value(block).ok()),
                summary: "Work product block created.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn patch_work_product_block(
        &self,
        matter_id: &str,
        work_product_id: &str,
        block_id: &str,
        request: PatchWorkProductBlockRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let before_block = product
            .blocks
            .iter()
            .find(|block| block.block_id == block_id)
            .cloned();
        let block = product
            .blocks
            .iter_mut()
            .find(|block| block.block_id == block_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("Work product block {block_id} not found"))
            })?;
        if block.locked && request.text.is_some() {
            return Err(ApiError::BadRequest(format!(
                "Work product block {block_id} is locked"
            )));
        }
        if let Some(value) = request.block_type {
            block.block_type = value;
        }
        if let Some(value) = request.role {
            block.role = value;
        }
        if let Some(value) = request.title {
            block.title = value;
        }
        if let Some(value) = request.text {
            block.text = value;
        }
        if let Some(value) = request.parent_block_id {
            block.parent_block_id = value;
        }
        if let Some(value) = request.fact_ids {
            block.fact_ids = value;
        }
        if let Some(value) = request.evidence_ids {
            block.evidence_ids = value;
        }
        if let Some(value) = request.authorities {
            block.authorities = value;
        }
        if let Some(value) = request.locked {
            block.locked = value;
        }
        if let Some(value) = request.review_status {
            block.review_status = value;
        }
        if let Some(value) = request.prosemirror_json {
            block.prosemirror_json = value;
        }
        super::work_product_ast::rebuild_work_product_ast_from_projection(&mut product);
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "block_updated",
            "block",
            block_id,
            "Work product block updated.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        let after_block = product
            .blocks
            .iter()
            .find(|block| block.block_id == block_id)
            .cloned();
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "editor",
            "auto",
            "Block updated",
            "Work product block updated.",
            vec![VersionChangeInput {
                target_type: "block".to_string(),
                target_id: block_id.to_string(),
                operation: "update".to_string(),
                before: before_block
                    .as_ref()
                    .and_then(|block| json_value(block).ok()),
                after: after_block
                    .as_ref()
                    .and_then(|block| json_value(block).ok()),
                summary: "Work product block updated.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn link_work_product_support(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: WorkProductLinkRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        self.validate_work_product_link_target(matter_id, &request.target_type, &request.target_id)
            .await?;
        let block_index = product
            .blocks
            .iter()
            .position(|block| block.block_id == request.block_id)
            .ok_or_else(|| ApiError::NotFound("Work product block not found".to_string()))?;
        let anchor_id = format!(
            "{}:anchor:{}",
            request.block_id,
            product.anchors.len().saturating_add(1)
        );
        let anchor_type = request.anchor_type.unwrap_or_else(|| {
            if request.citation.is_some() || request.canonical_id.is_some() {
                "authority".to_string()
            } else {
                request.target_type.clone()
            }
        });
        let relation = request.relation.unwrap_or_else(|| "supports".to_string());
        {
            let block = &mut product.blocks[block_index];
            match request.target_type.as_str() {
                "fact" => push_unique(&mut block.fact_ids, request.target_id.clone()),
                "evidence" | "document" | "source_span" => {
                    push_unique(&mut block.evidence_ids, request.target_id.clone())
                }
                "authority" | "provision" | "legal_text" => push_authority(
                    &mut block.authorities,
                    AuthorityRef {
                        citation: request
                            .citation
                            .clone()
                            .unwrap_or_else(|| request.target_id.clone()),
                        canonical_id: request
                            .canonical_id
                            .clone()
                            .unwrap_or_else(|| request.target_id.clone()),
                        reason: Some(relation.clone()),
                        pinpoint: request.pinpoint.clone(),
                    },
                ),
                _ => {}
            }
            push_unique(&mut block.mark_ids, anchor_id.clone());
        }
        product.anchors.push(WorkProductAnchor {
            id: anchor_id.clone(),
            anchor_id: anchor_id.clone(),
            matter_id: matter_id.to_string(),
            work_product_id: work_product_id.to_string(),
            block_id: request.block_id.clone(),
            anchor_type: anchor_type.clone(),
            target_type: request.target_type,
            target_id: request.target_id,
            relation,
            citation: request.citation,
            canonical_id: request.canonical_id,
            pinpoint: request.pinpoint,
            quote: request.quote,
            status: "needs_review".to_string(),
        });
        product.marks.push(WorkProductMark {
            id: format!("{anchor_id}:mark"),
            mark_id: format!("{anchor_id}:mark"),
            matter_id: matter_id.to_string(),
            work_product_id: work_product_id.to_string(),
            block_id: request.block_id.clone(),
            mark_type: anchor_type,
            from_offset: 0,
            to_offset: 0,
            label: "linked support".to_string(),
            target_type: "anchor".to_string(),
            target_id: anchor_id.clone(),
            status: "derived_from_ast".to_string(),
        });
        super::work_product_ast::rebuild_work_product_ast_from_projection(&mut product);
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "support_linked",
            "anchor",
            &anchor_id,
            "Support or authority anchor linked to a work product block.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        let after_anchor = product
            .anchors
            .iter()
            .find(|anchor| anchor.anchor_id == anchor_id)
            .cloned();
        let mut impact = LegalImpactSummary::default();
        if let Some(anchor) = &after_anchor {
            match anchor.target_type.as_str() {
                "fact" => impact.affected_facts.push(anchor.target_id.clone()),
                "evidence" | "document" | "source_span" => {
                    impact.affected_evidence.push(anchor.target_id.clone())
                }
                "authority" | "provision" | "legal_text" => impact.affected_authorities.push(
                    anchor
                        .canonical_id
                        .clone()
                        .unwrap_or_else(|| anchor.target_id.clone()),
                ),
                _ => {}
            }
        }
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "support_link",
            "auto",
            "Support linked",
            "Support or authority anchor linked to a work product block.",
            vec![VersionChangeInput {
                target_type: "support_use".to_string(),
                target_id: anchor_id.clone(),
                operation: "link".to_string(),
                before: None,
                after: after_anchor
                    .as_ref()
                    .and_then(|anchor| json_value(anchor).ok()),
                summary: "Support or authority anchor linked.".to_string(),
                legal_impact: impact,
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn patch_work_product_support(
        &self,
        matter_id: &str,
        work_product_id: &str,
        anchor_id: &str,
        request: PatchWorkProductSupportRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let before_anchor = product
            .anchors
            .iter()
            .find(|anchor| support_anchor_id_matches(anchor, anchor_id))
            .cloned()
            .ok_or_else(|| ApiError::NotFound("Work product support link not found".to_string()))?;
        let after_anchor = apply_work_product_support_update(&mut product, anchor_id, request)?;
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "support_updated",
            "anchor",
            &after_anchor.anchor_id,
            "Support link updated.",
        ));
        refresh_work_product_state(&mut product);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "support_link",
            "auto",
            "Support updated",
            "Support link updated.",
            vec![VersionChangeInput {
                target_type: "support_use".to_string(),
                target_id: after_anchor.anchor_id.clone(),
                operation: "update".to_string(),
                before: json_value(&before_anchor).ok(),
                after: json_value(&after_anchor).ok(),
                summary: "Support link updated.".to_string(),
                legal_impact: legal_impact_for_support_anchor(&after_anchor),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn delete_work_product_support(
        &self,
        matter_id: &str,
        work_product_id: &str,
        anchor_id: &str,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let removed_anchor = apply_work_product_support_removal(&mut product, anchor_id)?;
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "support_removed",
            "anchor",
            &removed_anchor.anchor_id,
            "Support link removed.",
        ));
        refresh_work_product_state(&mut product);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "support_link",
            "auto",
            "Support removed",
            "Support link removed.",
            vec![VersionChangeInput {
                target_type: "support_use".to_string(),
                target_id: removed_anchor.anchor_id.clone(),
                operation: "delete".to_string(),
                before: json_value(&removed_anchor).ok(),
                after: None,
                summary: "Support link removed.".to_string(),
                legal_impact: legal_impact_for_support_anchor(&removed_anchor),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn link_work_product_text_range(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: WorkProductTextRangeLinkRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        self.validate_work_product_link_target(matter_id, &request.target_type, &request.target_id)
            .await?;
        if let Some(document_id) = trimmed_optional_string(request.document_id.as_deref()) {
            self.require_document(matter_id, &document_id).await?;
        }
        let operations = apply_work_product_text_range_link(&mut product, request)?;
        normalize_work_product_ast(&mut product);
        let validation = super::ast_validation::validate_work_product_document(&product);
        if !validation.errors.is_empty() {
            let codes = validation
                .errors
                .iter()
                .map(|issue| issue.code.clone())
                .collect::<Vec<_>>()
                .join(",");
            return Err(ApiError::BadRequest(format!(
                "Text range link failed validation: issue_codes={codes}"
            )));
        }
        product.blocks = flatten_work_product_blocks(&product.document_ast.blocks);
        self.validate_work_product_matter_references(matter_id, &product)
            .await?;
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "text_range_linked",
            "document_ast",
            &product.document_ast.document_id,
            "Text range linked to support, citation, or exhibit.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "support_link",
            "auto",
            "Text range linked",
            "Text range linked to support, citation, or exhibit.",
            vec![VersionChangeInput {
                target_type: "text_range".to_string(),
                target_id: operations
                    .first()
                    .map(ast_operation_target_id)
                    .unwrap_or_else(|| work_product_id.to_string()),
                operation: "link".to_string(),
                before: None,
                after: json_value(&operations).ok(),
                summary: "Text range linked to support, citation, or exhibit.".to_string(),
                legal_impact: legal_impact_for_ast_operations(&operations),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn run_work_product_qc(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<AiActionResponse<Vec<WorkProductFinding>>> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        super::ast_validation::ensure_work_product_ast_valid(&product, "Work product QC")?;
        let before_product = product.clone();
        product.findings = super::rule_engine::work_product_findings(&product);
        product.document_ast.rule_findings = product.findings.clone();
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "qc_run",
            "work_product",
            work_product_id,
            "Deterministic work-product QC run completed.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        let mut impact = LegalImpactSummary::default();
        impact.qc_warnings_added = product
            .findings
            .iter()
            .filter(|finding| finding.status == "open")
            .map(|finding| finding.finding_id.clone())
            .collect();
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "rule_check",
            "rule_check",
            "QC run completed",
            "Deterministic work-product QC run completed.",
            vec![VersionChangeInput {
                target_type: "qc".to_string(),
                target_id: work_product_id.to_string(),
                operation: "update".to_string(),
                before: json_value(&before_product.findings).ok(),
                after: json_value(&product.findings).ok(),
                summary: "Deterministic work-product QC run completed.".to_string(),
                legal_impact: impact,
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(AiActionResponse {
            enabled: false,
            mode: "deterministic".to_string(),
            message: "No live provider is configured; ran deterministic work-product checks."
                .to_string(),
            result: Some(product.findings),
        })
    }

    pub async fn list_work_product_findings(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<WorkProductFinding>> {
        Ok(self
            .get_work_product(matter_id, work_product_id)
            .await?
            .findings)
    }

    pub async fn patch_work_product_finding(
        &self,
        matter_id: &str,
        work_product_id: &str,
        finding_id: &str,
        request: PatchWorkProductFindingRequest,
    ) -> ApiResult<WorkProduct> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let before_finding = product
            .findings
            .iter()
            .find(|finding| finding.finding_id == finding_id)
            .cloned();
        let finding = product
            .findings
            .iter_mut()
            .find(|finding| finding.finding_id == finding_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("Work product finding {finding_id} not found"))
            })?;
        finding.status = request.status;
        finding.updated_at = now_string();
        product.document_ast.rule_findings = product.findings.clone();
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "finding_updated",
            "finding",
            finding_id,
            "Work product finding status changed.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        let after_finding = product
            .findings
            .iter()
            .find(|finding| finding.finding_id == finding_id)
            .cloned();
        let mut impact = LegalImpactSummary::default();
        if let Some(before) = &before_finding {
            if before.status == "open" {
                impact.qc_warnings_resolved.push(before.finding_id.clone());
            }
        }
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "rule_check",
            "auto",
            "Finding updated",
            "Work product finding status changed.",
            vec![VersionChangeInput {
                target_type: "rule_finding".to_string(),
                target_id: finding_id.to_string(),
                operation: "resolve".to_string(),
                before: before_finding
                    .as_ref()
                    .and_then(|finding| json_value(finding).ok()),
                after: after_finding
                    .as_ref()
                    .and_then(|finding| json_value(finding).ok()),
                summary: "Work product finding status changed.".to_string(),
                legal_impact: impact,
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(product)
    }

    pub async fn preview_work_product(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<WorkProductPreviewResponse> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        super::ast_validation::ensure_work_product_ast_valid(&product, "Work product preview")?;
        Ok(super::html_renderer::render_work_product_preview(&product))
    }

    pub async fn export_work_product(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: ExportWorkProductRequest,
    ) -> ApiResult<WorkProductArtifact> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let format = normalize_export_format(&request.format)?;
        super::ast_validation::ensure_work_product_ast_valid(&product, "Work product export")?;
        let profile = request.profile.unwrap_or_else(|| "review".to_string());
        let mode = request.mode.unwrap_or_else(|| "review_needed".to_string());
        let generated_at = now_string();
        let artifact_id = format!("{work_product_id}:artifact:{format}:{generated_at}");
        let warnings = super::html_renderer::work_product_export_warnings(
            &product,
            &format,
            request.include_exhibits.unwrap_or(false),
            request.include_qc_report.unwrap_or(false),
        );
        let export_content =
            super::html_renderer::render_work_product_export_content(&product, &format)?;
        let artifact_hash = sha256_hex(export_content.as_bytes());
        let artifact_key = work_product_export_key(
            matter_id,
            work_product_id,
            &artifact_id,
            &artifact_hash,
            &format,
        );
        let artifact_blob = self
            .store_casebuilder_bytes(
                matter_id,
                &artifact_key,
                Bytes::from(export_content.clone()),
                export_mime_type(&format),
            )
            .await?;
        let content_preview = export_content_preview(&export_content);
        let render_profile_hash = sha256_hex(format!("{format}:{profile}:{mode}").as_bytes());
        let next_sequence = self
            .list_work_product_snapshots(matter_id, work_product_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|snapshot| snapshot.sequence_number)
            .max()
            .unwrap_or(0)
            + 1;
        let snapshot_id = format!("{work_product_id}:snapshot:{next_sequence}");
        let qc_status_at_export = work_product_qc_status(&product);
        let artifact = WorkProductArtifact {
            id: artifact_id.clone(),
            artifact_id: artifact_id.clone(),
            matter_id: matter_id.to_string(),
            work_product_id: work_product_id.to_string(),
            format: format.clone(),
            profile,
            mode,
            status: "generated_review_needed".to_string(),
            download_url: format!(
                "/api/v1/matters/{matter_id}/work-products/{work_product_id}/artifacts/{artifact_id}/download"
            ),
            page_count: super::html_renderer::render_work_product_preview(&product).page_count,
            generated_at,
            warnings,
            content_preview,
            snapshot_id: Some(snapshot_id.clone()),
            artifact_hash: Some(artifact_hash),
            render_profile_hash: Some(render_profile_hash),
            qc_status_at_export: Some(qc_status_at_export),
            changed_since_export: Some(false),
            immutable: Some(true),
            object_blob_id: Some(artifact_blob.object_blob_id.clone()),
            size_bytes: Some(artifact_blob.size_bytes),
            mime_type: artifact_blob.mime_type.clone(),
            storage_status: Some("stored".to_string()),
        };
        product.artifacts.push(artifact.clone());
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "artifact_generated",
            "artifact",
            &artifact_id,
            "Work product export artifact generated for human review.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "export",
            "export",
            "Export generated",
            "Work product export artifact generated for human review.",
            vec![VersionChangeInput {
                target_type: "export".to_string(),
                target_id: artifact_id.clone(),
                operation: "export".to_string(),
                before: None,
                after: json_value(&artifact).ok(),
                summary: "Work product export artifact generated.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: None,
            }],
        )
        .await?;
        Ok(artifact)
    }

    pub async fn get_work_product_artifact(
        &self,
        matter_id: &str,
        work_product_id: &str,
        artifact_id: &str,
    ) -> ApiResult<WorkProductArtifact> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        product
            .artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == artifact_id)
            .ok_or_else(|| ApiError::NotFound("Work product artifact not found".to_string()))
    }

    pub async fn download_work_product_artifact(
        &self,
        matter_id: &str,
        work_product_id: &str,
        artifact_id: &str,
    ) -> ApiResult<WorkProductDownloadResponse> {
        let artifact = self
            .get_work_product_artifact(matter_id, work_product_id, artifact_id)
            .await?;
        let mut headers = BTreeMap::new();
        headers.insert("X-Review-Needed".to_string(), "true".to_string());
        let mut method = "GET".to_string();
        let mut url = artifact.download_url.clone();
        let mut expires_at = now_string();
        let mut bytes = artifact
            .size_bytes
            .unwrap_or_else(|| artifact.content_preview.len() as u64);
        if let Some(object_blob_id) = artifact.object_blob_id.as_deref() {
            let blob = self.get_object_blob(matter_id, object_blob_id).await?;
            bytes = blob.size_bytes;
            expires_at = timestamp_after(self.download_ttl_seconds);
            if self.object_store.provider() == "r2" {
                let presigned = self
                    .object_store
                    .presign_get(
                        &blob.storage_key,
                        Duration::from_secs(self.download_ttl_seconds),
                    )
                    .await?;
                method = presigned.method;
                url = presigned.url;
                headers.extend(presigned.headers);
            }
        }
        Ok(WorkProductDownloadResponse {
            method,
            url,
            expires_at,
            headers,
            filename: safe_work_product_download_filename(&artifact),
            mime_type: Some(export_mime_type(&artifact.format).to_string()),
            bytes,
            artifact,
        })
    }

    pub async fn run_work_product_ai_command(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: WorkProductAiCommandRequest,
    ) -> ApiResult<AiActionResponse<WorkProduct>> {
        let mut product = self.get_work_product(matter_id, work_product_id).await?;
        let before_product = product.clone();
        let message = format!(
            "AI command '{}' is in template mode; no provider is configured.",
            request.command
        );
        let target_id = request
            .target_id
            .clone()
            .unwrap_or_else(|| work_product_id.to_string());
        let proposed_patch =
            super::ai_patch::empty_provider_free_ai_patch(&product, &request.command, "system");
        for command in &mut product.ai_commands {
            if command.command_id == request.command {
                command.last_message = Some(message.clone());
            }
        }
        product.history.push(work_product_event(
            matter_id,
            work_product_id,
            "ai_command_requested",
            request.target_id.as_deref().unwrap_or("work_product"),
            &target_id,
            "AI command requested in provider-free template mode.",
        ));
        refresh_work_product_state(&mut product);
        let product = self.save_work_product(matter_id, product).await?;
        let input_snapshot_id = self
            .list_work_product_snapshots(matter_id, work_product_id)
            .await
            .unwrap_or_default()
            .last()
            .map(|snapshot| snapshot.snapshot_id.clone())
            .unwrap_or_default();
        let ai_audit_id = generate_id(
            "ai-audit",
            &format!("{work_product_id}:{}", request.command),
        );
        let ai_audit = AIEditAudit {
            id: ai_audit_id.clone(),
            ai_audit_id: ai_audit_id.clone(),
            matter_id: matter_id.to_string(),
            subject_type: "work_product".to_string(),
            subject_id: work_product_id.to_string(),
            target_type: request
                .target_id
                .as_ref()
                .map(|_| "block")
                .unwrap_or("work_product")
                .to_string(),
            target_id,
            command: request.command.clone(),
            prompt_template_id: None,
            model: None,
            provider_mode: "template".to_string(),
            input_fact_ids: product
                .blocks
                .iter()
                .flat_map(|block| block.fact_ids.clone())
                .collect(),
            input_evidence_ids: product
                .blocks
                .iter()
                .flat_map(|block| block.evidence_ids.clone())
                .collect(),
            input_authority_ids: product
                .blocks
                .iter()
                .flat_map(|block| {
                    block
                        .authorities
                        .iter()
                        .map(|authority| authority.canonical_id.clone())
                        .collect::<Vec<_>>()
                })
                .collect(),
            input_snapshot_id,
            output_text: None,
            inserted_text: None,
            user_action: "template_recorded".to_string(),
            warnings: vec![
                message.clone(),
                "Provider-free AI returned an empty AstPatch; no unsupported text was inserted."
                    .to_string(),
            ],
            created_at: now_string(),
        };
        self.merge_node(
            matter_id,
            ai_edit_audit_spec(),
            &ai_audit.ai_audit_id,
            &ai_audit,
        )
        .await?;
        self.record_work_product_change(
            matter_id,
            Some(&before_product),
            &product,
            "ai",
            "ai_edit",
            "AI command recorded",
            &message,
            vec![VersionChangeInput {
                target_type: "ai_edit".to_string(),
                target_id: ai_audit.target_id.clone(),
                operation: "create".to_string(),
                before: None,
                after: json_value(&serde_json::json!({
                    "ai_audit": ai_audit.clone(),
                    "proposed_patch": proposed_patch.clone(),
                }))
                .ok(),
                summary: "Provider-free AI patch proposal recorded.".to_string(),
                legal_impact: LegalImpactSummary::default(),
                ai_audit_id: Some(ai_audit.ai_audit_id.clone()),
            }],
        )
        .await?;
        Ok(AiActionResponse {
            enabled: false,
            mode: "template".to_string(),
            message,
            result: Some(product),
        })
    }

    pub async fn work_product_history(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<ChangeSet>> {
        self.get_work_product(matter_id, work_product_id).await?;
        self.list_work_product_change_sets(matter_id, work_product_id)
            .await
    }

    pub async fn get_work_product_change_set(
        &self,
        matter_id: &str,
        work_product_id: &str,
        change_set_id: &str,
    ) -> ApiResult<ChangeSet> {
        self.get_work_product(matter_id, work_product_id).await?;
        let change_set = self
            .get_node::<ChangeSet>(matter_id, change_set_spec(), change_set_id)
            .await?;
        if change_set.subject_id == work_product_id {
            Ok(change_set)
        } else {
            Err(ApiError::NotFound(format!(
                "Change set {change_set_id} not found"
            )))
        }
    }

    pub async fn list_work_product_snapshots(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<VersionSnapshot>> {
        self.get_work_product(matter_id, work_product_id).await?;
        let mut snapshots = self
            .list_nodes::<VersionSnapshot>(matter_id, version_snapshot_spec())
            .await?
            .into_iter()
            .filter(|snapshot| snapshot.subject_id == work_product_id)
            .collect::<Vec<_>>();
        snapshots.sort_by_key(|snapshot| snapshot.sequence_number);
        Ok(snapshots)
    }

    pub async fn list_work_product_snapshots_for_api(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<VersionSnapshot>> {
        let mut snapshots = self
            .list_work_product_snapshots(matter_id, work_product_id)
            .await?;
        for snapshot in &mut snapshots {
            summarize_version_snapshot_for_list(snapshot);
        }
        Ok(snapshots)
    }

    pub(super) async fn latest_work_product_snapshot_id(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Option<String>> {
        Ok(latest_snapshot_id(
            &self
                .list_work_product_snapshots(matter_id, work_product_id)
                .await?,
        ))
    }

    pub async fn get_work_product_snapshot(
        &self,
        matter_id: &str,
        work_product_id: &str,
        snapshot_id: &str,
    ) -> ApiResult<VersionSnapshot> {
        self.get_work_product(matter_id, work_product_id).await?;
        let snapshot = self
            .get_node::<VersionSnapshot>(matter_id, version_snapshot_spec(), snapshot_id)
            .await?;
        if snapshot.subject_id == work_product_id {
            let mut snapshot = snapshot;
            self.hydrate_snapshot_full_state(matter_id, &mut snapshot)
                .await?;
            Ok(snapshot)
        } else {
            Err(ApiError::NotFound(format!(
                "Snapshot {snapshot_id} not found"
            )))
        }
    }

    pub async fn create_work_product_snapshot(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: CreateVersionSnapshotRequest,
    ) -> ApiResult<VersionSnapshot> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        let title = request
            .title
            .unwrap_or_else(|| "Manual snapshot".to_string());
        let change_set = self
            .record_work_product_change(
                matter_id,
                None,
                &product,
                "manual_snapshot",
                "manual",
                &title,
                request
                    .message
                    .as_deref()
                    .unwrap_or("Manual Case History snapshot created."),
                vec![VersionChangeInput {
                    target_type: "work_product".to_string(),
                    target_id: work_product_id.to_string(),
                    operation: "snapshot".to_string(),
                    before: None,
                    after: json_value(&product).ok(),
                    summary: "Manual snapshot created.".to_string(),
                    legal_impact: LegalImpactSummary::default(),
                    ai_audit_id: None,
                }],
            )
            .await?;
        self.get_work_product_snapshot(matter_id, work_product_id, &change_set.snapshot_id)
            .await
    }

    pub async fn compare_work_product_snapshots(
        &self,
        matter_id: &str,
        work_product_id: &str,
        from_snapshot_id: &str,
        to_snapshot_id: Option<&str>,
        layers: Vec<String>,
    ) -> ApiResult<CompareVersionsResponse> {
        let current = self.get_work_product(matter_id, work_product_id).await?;
        let from_snapshot = self
            .get_work_product_snapshot(matter_id, work_product_id, from_snapshot_id)
            .await?;
        let from_product = self
            .product_from_snapshot(matter_id, &from_snapshot)
            .await?;
        let (to_id, to_product) = if let Some(snapshot_id) = to_snapshot_id {
            let snapshot = self
                .get_work_product_snapshot(matter_id, work_product_id, snapshot_id)
                .await?;
            (
                snapshot.snapshot_id.clone(),
                self.product_from_snapshot(matter_id, &snapshot).await?,
            )
        } else {
            ("current".to_string(), current)
        };
        let selected_layers = normalize_compare_layers(layers);
        let text_diffs = if selected_layers.iter().any(|layer| layer == "text") {
            diff_work_product_blocks(&from_product, &to_product)
        } else {
            Vec::new()
        };
        let layer_diffs = super::ast_diff::diff_work_product_layers(
            &from_product,
            &to_product,
            &selected_layers,
        )?;
        let summary = VersionChangeSummary {
            text_changes: text_diffs
                .iter()
                .filter(|diff| diff.status != "unchanged")
                .count() as u64,
            support_changes: layer_change_count(&layer_diffs, "support"),
            citation_changes: layer_change_count(&layer_diffs, "citations"),
            authority_changes: layer_diffs
                .iter()
                .filter(|diff| {
                    diff.layer == "support"
                        && diff.status != "unchanged"
                        && matches!(
                            diff.target_type.as_str(),
                            "legal_authority" | "provision" | "legal_text"
                        )
                })
                .count() as u64,
            qc_changes: layer_change_count(&layer_diffs, "rule_findings"),
            export_changes: layer_change_count(&layer_diffs, "exports"),
            ai_changes: 0,
            targets_changed: text_diffs
                .iter()
                .filter(|diff| diff.status != "unchanged")
                .map(|diff| VersionTargetSummary {
                    target_type: diff.target_type.clone(),
                    target_id: diff.target_id.clone(),
                    label: Some(diff.title.clone()),
                })
                .chain(
                    layer_diffs
                        .iter()
                        .filter(|diff| diff.status != "unchanged")
                        .map(|diff| VersionTargetSummary {
                            target_type: diff.target_type.clone(),
                            target_id: diff.target_id.clone(),
                            label: Some(diff.title.clone()),
                        }),
                )
                .collect(),
            risk_level: "low".to_string(),
            user_summary: "Compared work-product AST layers.".to_string(),
        };
        Ok(CompareVersionsResponse {
            matter_id: matter_id.to_string(),
            subject_id: work_product_id.to_string(),
            from_snapshot_id: from_snapshot.snapshot_id,
            to_snapshot_id: to_id,
            layers: selected_layers,
            summary,
            text_diffs,
            layer_diffs,
        })
    }

    pub async fn restore_work_product_version(
        &self,
        matter_id: &str,
        work_product_id: &str,
        request: RestoreVersionRequest,
    ) -> ApiResult<RestoreVersionResponse> {
        let current = self.get_work_product(matter_id, work_product_id).await?;
        let snapshot = self
            .get_work_product_snapshot(matter_id, work_product_id, &request.snapshot_id)
            .await?;
        let snapshot_product = self.product_from_snapshot(matter_id, &snapshot).await?;
        let scope = request.scope.as_str();
        let dry_run = request.dry_run.unwrap_or(false);
        let target_ids = request.target_ids.unwrap_or_default();
        let (mut restored, warnings) =
            restore_work_product_scope(&current, &snapshot_product, scope, &target_ids)?;
        if dry_run {
            return Ok(RestoreVersionResponse {
                restored: false,
                dry_run: true,
                warnings,
                snapshot_id: snapshot.snapshot_id,
                change_set: None,
                result: Some(restored),
            });
        }
        refresh_work_product_state(&mut restored);
        self.validate_work_product_matter_references(matter_id, &restored)
            .await?;
        let restored = self.save_work_product(matter_id, restored).await?;
        self.sync_complaint_projection_from_work_product(matter_id, &restored)
            .await?;
        let change_set = self
            .record_work_product_change(
                matter_id,
                Some(&current),
                &restored,
                "restore",
                "restore",
                "Version restored",
                &format!("Restored {} from snapshot {}.", scope, snapshot.snapshot_id),
                vec![VersionChangeInput {
                    target_type: scope.to_string(),
                    target_id: work_product_id.to_string(),
                    operation: "restore".to_string(),
                    before: json_value(&current).ok(),
                    after: json_value(&restored).ok(),
                    summary: format!("Restored {scope} from Case History snapshot."),
                    legal_impact: LegalImpactSummary::default(),
                    ai_audit_id: None,
                }],
            )
            .await?;
        Ok(RestoreVersionResponse {
            restored: true,
            dry_run: false,
            warnings,
            snapshot_id: snapshot.snapshot_id,
            change_set: Some(change_set),
            result: Some(restored),
        })
    }

    pub async fn work_product_export_history(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<WorkProductArtifact>> {
        let product = self.get_work_product(matter_id, work_product_id).await?;
        let current_hashes = work_product_hashes(&product)?;
        let snapshots_by_id = self
            .list_work_product_snapshots(matter_id, work_product_id)
            .await?
            .into_iter()
            .map(|snapshot| (snapshot.snapshot_id.clone(), snapshot))
            .collect::<BTreeMap<_, _>>();
        let mut artifacts = Vec::new();
        for mut artifact in product.artifacts {
            if let Some(snapshot_id) = artifact.snapshot_id.clone() {
                artifact.changed_since_export = snapshots_by_id
                    .get(&snapshot_id)
                    .map(|snapshot| {
                        snapshot.document_hash != current_hashes.document_hash
                            || snapshot.support_graph_hash != current_hashes.support_graph_hash
                            || snapshot.qc_state_hash != current_hashes.qc_state_hash
                            || snapshot.formatting_hash != current_hashes.formatting_hash
                    })
                    .or(Some(true));
            }
            artifacts.push(artifact);
        }
        Ok(artifacts)
    }

    pub async fn work_product_ai_audit(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<AIEditAudit>> {
        self.get_work_product(matter_id, work_product_id).await?;
        let mut audits = self
            .list_nodes::<AIEditAudit>(matter_id, ai_edit_audit_spec())
            .await?
            .into_iter()
            .filter(|audit| audit.subject_id == work_product_id)
            .collect::<Vec<_>>();
        audits.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(audits)
    }

    pub(super) async fn apply_snapshot_storage_policy(
        &self,
        matter_id: &str,
        product: &WorkProduct,
        snapshot: &mut VersionSnapshot,
        manifest: &mut SnapshotManifest,
        entity_states: &mut [SnapshotEntityState],
    ) -> ApiResult<()> {
        let full_state = json_value(product)?;
        let full_state_bytes = serde_json::to_vec(&full_state)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        if should_inline_payload(
            full_state_bytes.len(),
            self.ast_storage_policy.snapshot_inline_bytes,
        ) {
            snapshot.full_state_inline = Some(full_state);
            snapshot.full_state_ref = None;
        } else {
            let full_state_hash = sha256_hex(&full_state_bytes);
            let key = snapshot_full_state_key(
                matter_id,
                &product.work_product_id,
                &snapshot.snapshot_id,
                &full_state_hash,
            );
            let blob = self
                .store_casebuilder_bytes(
                    matter_id,
                    &key,
                    Bytes::from(full_state_bytes),
                    "application/json; charset=utf-8",
                )
                .await?;
            snapshot.full_state_inline = None;
            snapshot.full_state_ref = Some(blob.object_blob_id);
        }

        let mut offloaded_any_state = false;
        for state in entity_states.iter_mut() {
            let Some(value) = state.state_inline.clone() else {
                continue;
            };
            let bytes = serde_json::to_vec(&value)
                .map_err(|error| ApiError::Internal(error.to_string()))?;
            if should_inline_payload(bytes.len(), self.ast_storage_policy.entity_inline_bytes) {
                continue;
            }
            let key = snapshot_entity_state_key(
                matter_id,
                &product.work_product_id,
                &snapshot.snapshot_id,
                &state.entity_type,
                &state.entity_hash,
            );
            let blob = self
                .store_casebuilder_bytes(
                    matter_id,
                    &key,
                    Bytes::from(bytes),
                    "application/json; charset=utf-8",
                )
                .await?;
            state.state_inline = None;
            state.state_ref = Some(blob.object_blob_id);
            offloaded_any_state = true;
        }

        let manifest_payload = serde_json::json!({
            "manifest": manifest,
            "entity_states": entity_states.iter().map(|state| serde_json::json!({
                "entity_state_id": state.entity_state_id,
                "entity_type": state.entity_type,
                "entity_id": state.entity_id,
                "entity_hash": state.entity_hash,
                "state_ref": state.state_ref,
                "inline": state.state_inline.is_some(),
            })).collect::<Vec<_>>(),
        });
        let manifest_bytes = serde_json::to_vec(&manifest_payload)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        if offloaded_any_state
            || !should_inline_payload(
                manifest_bytes.len(),
                self.ast_storage_policy.entity_inline_bytes,
            )
        {
            let key = snapshot_manifest_key(
                matter_id,
                &product.work_product_id,
                &snapshot.snapshot_id,
                &manifest.manifest_hash,
            );
            let blob = self
                .store_casebuilder_bytes(
                    matter_id,
                    &key,
                    Bytes::from(manifest_bytes),
                    "application/json; charset=utf-8",
                )
                .await?;
            manifest.storage_ref = Some(blob.object_blob_id);
            snapshot.manifest_ref = manifest.storage_ref.clone();
        }
        Ok(())
    }

    pub(super) async fn product_from_snapshot(
        &self,
        matter_id: &str,
        snapshot: &VersionSnapshot,
    ) -> ApiResult<WorkProduct> {
        if let Some(value) = snapshot.full_state_inline.clone() {
            return serde_json::from_value(value)
                .map_err(|error| ApiError::Internal(error.to_string()));
        }
        let Some(blob_id) = snapshot.full_state_ref.as_deref() else {
            return Err(ApiError::Internal("Snapshot has no state".to_string()));
        };
        self.load_json_blob(matter_id, blob_id).await
    }

    pub(super) async fn hydrate_snapshot_full_state(
        &self,
        matter_id: &str,
        snapshot: &mut VersionSnapshot,
    ) -> ApiResult<()> {
        if snapshot.full_state_inline.is_some() {
            return Ok(());
        }
        let Some(blob_id) = snapshot.full_state_ref.as_deref() else {
            return Ok(());
        };
        snapshot.full_state_inline = Some(self.load_json_blob(matter_id, blob_id).await?);
        Ok(())
    }

    pub(super) async fn list_work_product_change_sets(
        &self,
        matter_id: &str,
        work_product_id: &str,
    ) -> ApiResult<Vec<ChangeSet>> {
        let mut change_sets = self
            .list_nodes::<ChangeSet>(matter_id, change_set_spec())
            .await?
            .into_iter()
            .filter(|change_set| change_set.subject_id == work_product_id)
            .collect::<Vec<_>>();
        change_sets.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(change_sets)
    }

    pub(super) async fn record_work_product_change(
        &self,
        matter_id: &str,
        before_product: Option<&WorkProduct>,
        after_product: &WorkProduct,
        source: &str,
        snapshot_type: &str,
        title: &str,
        summary: &str,
        changes: Vec<VersionChangeInput>,
    ) -> ApiResult<ChangeSet> {
        let now = now_string();
        let branch_id = format!("{}:branch:main", after_product.work_product_id);
        let existing_branch = self
            .list_nodes::<VersionBranch>(matter_id, version_branch_spec())
            .await
            .unwrap_or_default()
            .into_iter()
            .find(|branch| {
                branch.subject_id == after_product.work_product_id && branch.branch_type == "main"
            });
        let parent_snapshot_id = existing_branch.as_ref().and_then(|branch| {
            if branch.current_snapshot_id.is_empty() {
                None
            } else {
                Some(branch.current_snapshot_id.clone())
            }
        });
        let sequence_number = self
            .list_work_product_snapshots(matter_id, &after_product.work_product_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|snapshot| snapshot.sequence_number)
            .max()
            .unwrap_or(0)
            + 1;
        let snapshot_id = format!(
            "{}:snapshot:{}",
            after_product.work_product_id, sequence_number
        );
        let change_set_id = format!(
            "{}:changeset:{}",
            after_product.work_product_id, sequence_number
        );
        let mut change_ids = Vec::new();
        let mut recorded_changes = Vec::new();
        for (index, input) in changes.into_iter().enumerate() {
            let change_id = format!("{change_set_id}:change:{}", index + 1);
            let (before_hash, before) = version_change_state_summary(input.before)?;
            let (after_hash, after) = version_change_state_summary(input.after)?;
            change_ids.push(change_id.clone());
            recorded_changes.push(VersionChange {
                id: change_id.clone(),
                change_id,
                change_set_id: change_set_id.clone(),
                snapshot_id: snapshot_id.clone(),
                matter_id: matter_id.to_string(),
                subject_type: "work_product".to_string(),
                subject_id: after_product.work_product_id.clone(),
                branch_id: branch_id.clone(),
                target_type: input.target_type,
                target_id: input.target_id,
                operation: input.operation,
                before_hash,
                after_hash,
                before,
                after,
                summary: input.summary,
                legal_impact: input.legal_impact,
                ai_audit_id: input.ai_audit_id,
                created_at: now.clone(),
                created_by: if source == "ai" { "ai" } else { "system" }.to_string(),
                actor_id: None,
            });
        }
        let legal_impact =
            merge_legal_impacts(recorded_changes.iter().map(|change| &change.legal_impact));
        let version_summary = version_summary_for_changes(summary, &recorded_changes);
        let hashes = work_product_hashes(after_product)?;
        let (mut manifest, mut entity_states) =
            snapshot_manifest_for_product(matter_id, &snapshot_id, after_product, &now)?;
        let mut snapshot = VersionSnapshot {
            id: snapshot_id.clone(),
            snapshot_id: snapshot_id.clone(),
            matter_id: matter_id.to_string(),
            subject_type: "work_product".to_string(),
            subject_id: after_product.work_product_id.clone(),
            product_type: after_product.product_type.clone(),
            profile_id: after_product.profile.profile_id.clone(),
            branch_id: branch_id.clone(),
            sequence_number,
            title: title.to_string(),
            message: Some(summary.to_string()),
            created_at: now.clone(),
            created_by: if source == "ai" { "ai" } else { "system" }.to_string(),
            actor_id: None,
            snapshot_type: snapshot_type.to_string(),
            parent_snapshot_ids: parent_snapshot_id.clone().into_iter().collect(),
            document_hash: hashes.document_hash,
            support_graph_hash: hashes.support_graph_hash,
            qc_state_hash: hashes.qc_state_hash,
            formatting_hash: hashes.formatting_hash,
            manifest_hash: manifest.manifest_hash.clone(),
            manifest_ref: None,
            full_state_ref: None,
            full_state_inline: None,
            summary: version_summary,
        };
        self.apply_snapshot_storage_policy(
            matter_id,
            after_product,
            &mut snapshot,
            &mut manifest,
            &mut entity_states,
        )
        .await?;
        let branch = VersionBranch {
            id: branch_id.clone(),
            branch_id: branch_id.clone(),
            matter_id: matter_id.to_string(),
            subject_type: "work_product".to_string(),
            subject_id: after_product.work_product_id.clone(),
            name: if after_product.product_type == "complaint" {
                "Main Complaint".to_string()
            } else {
                format!(
                    "Main {}",
                    humanize_product_type(&after_product.product_type)
                )
            },
            description: None,
            created_from_snapshot_id: existing_branch
                .as_ref()
                .map(|branch| branch.created_from_snapshot_id.clone())
                .unwrap_or_else(|| snapshot_id.clone()),
            current_snapshot_id: snapshot_id.clone(),
            branch_type: "main".to_string(),
            created_at: existing_branch
                .as_ref()
                .map(|branch| branch.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now.clone(),
            archived_at: None,
        };
        let change_set = ChangeSet {
            id: change_set_id.clone(),
            change_set_id: change_set_id.clone(),
            matter_id: matter_id.to_string(),
            subject_id: after_product.work_product_id.clone(),
            branch_id: branch_id.clone(),
            snapshot_id: snapshot_id.clone(),
            parent_snapshot_id,
            title: title.to_string(),
            summary: summary.to_string(),
            reason: None,
            actor_type: if source == "ai" { "ai" } else { "system" }.to_string(),
            actor_id: None,
            source: source.to_string(),
            created_at: now,
            change_ids,
            legal_impact,
        };

        self.merge_node(matter_id, version_branch_spec(), &branch.branch_id, &branch)
            .await?;
        self.merge_node(
            matter_id,
            version_snapshot_spec(),
            &snapshot.snapshot_id,
            &snapshot,
        )
        .await?;
        self.merge_node(
            matter_id,
            snapshot_manifest_spec(),
            &manifest.manifest_id,
            &manifest,
        )
        .await?;
        for state in &entity_states {
            self.merge_node(
                matter_id,
                snapshot_entity_state_spec(),
                &state.entity_state_id,
                state,
            )
            .await?;
        }
        for change in &recorded_changes {
            self.merge_node(matter_id, version_change_spec(), &change.change_id, change)
                .await?;
        }
        self.merge_node(
            matter_id,
            change_set_spec(),
            &change_set.change_set_id,
            &change_set,
        )
        .await?;
        self.materialize_case_history_edges(
            &after_product.work_product_id,
            &branch,
            &snapshot,
            &manifest,
            &entity_states,
            &recorded_changes,
            &change_set,
        )
        .await?;
        if before_product.is_none() {
            self.materialize_version_subject(after_product).await?;
        }
        Ok(change_set)
    }

    pub(super) async fn save_work_product(
        &self,
        matter_id: &str,
        mut product: WorkProduct,
    ) -> ApiResult<WorkProduct> {
        product.updated_at = now_string();
        refresh_work_product_state(&mut product);
        super::ast_validation::ensure_work_product_ast_valid(&product, "Work product save")?;
        self.save_work_product_internal(matter_id, product).await
    }

    pub(super) async fn save_work_product_internal(
        &self,
        matter_id: &str,
        mut product: WorkProduct,
    ) -> ApiResult<WorkProduct> {
        refresh_work_product_state(&mut product);
        super::ast_validation::ensure_work_product_ast_valid(&product, "Work product save")?;
        let product = self
            .merge_node(
                matter_id,
                work_product_spec(),
                &product.work_product_id,
                &product,
            )
            .await?;
        self.materialize_work_product_edges(&product).await?;
        for finding in &product.findings {
            self.merge_node(
                matter_id,
                work_product_finding_spec(),
                &finding.finding_id,
                finding,
            )
            .await?;
        }
        for artifact in &product.artifacts {
            self.merge_node(
                matter_id,
                work_product_artifact_spec(),
                &artifact.artifact_id,
                artifact,
            )
            .await?;
            if let Some(object_blob_id) = artifact.object_blob_id.as_deref() {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (a:WorkProductArtifact {artifact_id: $artifact_id})
                             MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                             MERGE (a)-[:STORED_AS]->(b)",
                        )
                        .param("artifact_id", artifact.artifact_id.clone())
                        .param("object_blob_id", object_blob_id.to_string()),
                    )
                    .await?;
            }
        }
        Ok(product)
    }

    pub(super) async fn migrate_legacy_drafts_to_work_products(
        &self,
        matter_id: &str,
    ) -> ApiResult<()> {
        let drafts = self
            .list_nodes::<CaseDraft>(matter_id, draft_spec())
            .await
            .unwrap_or_default();
        for draft in drafts {
            if draft.kind == "complaint" || draft.draft_type == "complaint" {
                continue;
            }
            match self
                .get_node::<WorkProduct>(matter_id, work_product_spec(), &draft.draft_id)
                .await
            {
                Ok(_) => continue,
                Err(ApiError::NotFound(_)) => {
                    let product = work_product_from_draft(&draft);
                    self.save_work_product_internal(matter_id, product).await?;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub(super) async fn migrate_complaints_to_work_products(
        &self,
        matter_id: &str,
    ) -> ApiResult<()> {
        let complaints = self
            .list_nodes::<ComplaintDraft>(matter_id, complaint_spec())
            .await
            .unwrap_or_default();
        for complaint in complaints {
            match self
                .get_node::<WorkProduct>(matter_id, work_product_spec(), &complaint.complaint_id)
                .await
            {
                Ok(_) => continue,
                Err(ApiError::NotFound(_)) => {
                    let product = work_product_from_complaint(&complaint);
                    self.save_work_product_internal(matter_id, product).await?;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}
