use super::*;

impl CaseBuilderService {
    pub(super) async fn materialize_fact_edges(&self, fact: &CaseFact) -> ApiResult<()> {
        for document_id in &fact.source_document_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (f:Fact {fact_id: $fact_id})
                         MATCH (d:CaseDocument {document_id: $document_id})
                         MERGE (f)-[:SUPPORTED_BY]->(d)
                         MERGE (d)-[:SUPPORTS]->(f)",
                    )
                    .param("fact_id", fact.fact_id.clone())
                    .param("document_id", document_id.clone()),
                )
                .await?;
        }
        for evidence_id in &fact.source_evidence_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (f:Fact {fact_id: $fact_id})
                         MATCH (e:Evidence {evidence_id: $evidence_id})
                         MERGE (f)-[:SUPPORTED_BY]->(e)
                         MERGE (e)-[:SUPPORTS]->(f)",
                    )
                    .param("fact_id", fact.fact_id.clone())
                    .param("evidence_id", evidence_id.clone()),
                )
                .await?;
        }
        for span in &fact.source_spans {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (f:Fact {fact_id: $fact_id})
                         MATCH (s:SourceSpan {source_span_id: $source_span_id})
                         MERGE (f)-[:SUPPORTED_BY]->(s)
                         MERGE (s)-[:SUPPORTS]->(f)",
                    )
                    .param("fact_id", fact.fact_id.clone())
                    .param("source_span_id", span.source_span_id.clone()),
                )
                .await?;
        }
        Ok(())
    }

    pub(super) async fn materialize_evidence_edges(
        &self,
        evidence: &CaseEvidence,
    ) -> ApiResult<()> {
        self.neo4j
            .run_rows(
                query(
                    "MATCH (e:Evidence {evidence_id: $evidence_id})
                     MATCH (d:CaseDocument {document_id: $document_id})
                     MERGE (e)-[:DERIVED_FROM]->(d)",
                )
                .param("evidence_id", evidence.evidence_id.clone())
                .param("document_id", evidence.document_id.clone()),
            )
            .await?;
        for fact_id in &evidence.supports_fact_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:Evidence {evidence_id: $evidence_id})
                         MATCH (f:Fact {fact_id: $fact_id})
                         MERGE (e)-[:SUPPORTS]->(f)
                         MERGE (f)-[:SUPPORTED_BY]->(e)",
                    )
                    .param("evidence_id", evidence.evidence_id.clone())
                    .param("fact_id", fact_id.clone()),
                )
                .await?;
        }
        for fact_id in &evidence.contradicts_fact_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:Evidence {evidence_id: $evidence_id})
                         MATCH (f:Fact {fact_id: $fact_id})
                         MERGE (e)-[:CONTRADICTS]->(f)
                         MERGE (f)-[:CONTRADICTED_BY]->(e)",
                    )
                    .param("evidence_id", evidence.evidence_id.clone())
                    .param("fact_id", fact_id.clone()),
                )
                .await?;
        }
        for span in &evidence.source_spans {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (e:Evidence {evidence_id: $evidence_id})
                         MATCH (s:SourceSpan {source_span_id: $source_span_id})
                         MERGE (e)-[:QUOTES]->(s)
                         MERGE (s)-[:SUPPORTS]->(e)",
                    )
                    .param("evidence_id", evidence.evidence_id.clone())
                    .param("source_span_id", span.source_span_id.clone()),
                )
                .await?;
        }
        Ok(())
    }

    pub(super) async fn materialize_claim_edges(&self, claim: &CaseClaim) -> ApiResult<()> {
        for fact_id in &claim.fact_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:Claim {claim_id: $claim_id})
                         MATCH (f:Fact {fact_id: $fact_id})
                         MERGE (f)-[:SATISFIES_ELEMENT]->(c)
                         MERGE (c)-[:SUPPORTED_BY_FACT]->(f)",
                    )
                    .param("claim_id", claim.claim_id.clone())
                    .param("fact_id", fact_id.clone()),
                )
                .await?;
        }
        for evidence_id in &claim.evidence_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:Claim {claim_id: $claim_id})
                         MATCH (e:Evidence {evidence_id: $evidence_id})
                         MERGE (c)-[:SUPPORTED_BY_EVIDENCE]->(e)",
                    )
                    .param("claim_id", claim.claim_id.clone())
                    .param("evidence_id", evidence_id.clone()),
                )
                .await?;
        }
        for authority in &claim.authorities {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:Claim {claim_id: $claim_id})
                         OPTIONAL MATCH (p:Provision {canonical_id: $canonical_id})
                         OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $canonical_id})
                         WITH c, coalesce(p, i) AS authority
                         FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                           MERGE (c)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                         )",
                    )
                    .param("claim_id", claim.claim_id.clone())
                    .param("canonical_id", authority.canonical_id.clone()),
                )
                .await?;
        }
        for element in &claim.elements {
            let element_payload = to_payload(element)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:Claim {claim_id: $claim_id})
                         MERGE (e:Element {element_id: $element_id})
                         SET e.payload = $payload,
                             e.matter_id = $matter_id,
                             e.element_id = $element_id,
                             e.text = $text
                         MERGE (c)-[:HAS_ELEMENT]->(e)",
                    )
                    .param("claim_id", claim.claim_id.clone())
                    .param("matter_id", claim.matter_id.clone())
                    .param("element_id", element.element_id.clone())
                    .param("text", element.text.clone())
                    .param("payload", element_payload),
                )
                .await?;
            for fact_id in &element.fact_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (e:Element {element_id: $element_id})
                             MATCH (f:Fact {fact_id: $fact_id})
                             MERGE (f)-[:SATISFIES_ELEMENT]->(e)
                             MERGE (e)-[:SUPPORTED_BY_FACT]->(f)",
                        )
                        .param("element_id", element.element_id.clone())
                        .param("fact_id", fact_id.clone()),
                    )
                    .await?;
            }
            for evidence_id in &element.evidence_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (e:Element {element_id: $element_id})
                             MATCH (ev:Evidence {evidence_id: $evidence_id})
                             MERGE (e)-[:SUPPORTED_BY_EVIDENCE]->(ev)",
                        )
                        .param("element_id", element.element_id.clone())
                        .param("evidence_id", evidence_id.clone()),
                    )
                    .await?;
            }
            let mut authorities = element.authorities.clone();
            if let Some(value) = &element.authority {
                push_authority(
                    &mut authorities,
                    AuthorityRef {
                        citation: value.clone(),
                        canonical_id: value.clone(),
                        reason: None,
                        pinpoint: None,
                    },
                );
            }
            for authority in authorities {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (e:Element {element_id: $element_id})
                             OPTIONAL MATCH (p:Provision {canonical_id: $canonical_id})
                             OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $canonical_id})
                             WITH e, coalesce(p, i) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (e)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                             )",
                        )
                        .param("element_id", element.element_id.clone())
                        .param("canonical_id", authority.canonical_id),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    pub(super) async fn materialize_draft_edges(&self, draft: &CaseDraft) -> ApiResult<()> {
        for paragraph in &draft.paragraphs {
            let payload = to_payload(paragraph)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (d:Draft {draft_id: $draft_id})
                         MERGE (p:DraftParagraph {paragraph_id: $paragraph_id})
                         SET p.payload = $payload,
                             p.matter_id = $matter_id,
                             p.draft_id = $draft_id,
                             p.paragraph_id = $paragraph_id,
                             p.role = $role
                         MERGE (d)-[:HAS_PARAGRAPH]->(p)",
                    )
                    .param("draft_id", draft.draft_id.clone())
                    .param("matter_id", draft.matter_id.clone())
                    .param("paragraph_id", paragraph.paragraph_id.clone())
                    .param("role", paragraph.role.clone())
                    .param("payload", payload),
                )
                .await?;
            for authority in &paragraph.authorities {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:DraftParagraph {paragraph_id: $paragraph_id})
                             OPTIONAL MATCH (provision:Provision {canonical_id: $canonical_id})
                             OPTIONAL MATCH (identity:LegalTextIdentity {canonical_id: $canonical_id})
                             WITH p, coalesce(provision, identity) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (p)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                             )",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("canonical_id", authority.canonical_id.clone()),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    pub(super) async fn materialize_work_product_edges(
        &self,
        product: &WorkProduct,
    ) -> ApiResult<()> {
        for block in &product.blocks {
            let payload = work_product_block_graph_payload(
                block,
                self.ast_storage_policy.block_inline_bytes,
            )?;
            let text_excerpt =
                block_text_excerpt(&block.text, self.ast_storage_policy.block_inline_bytes);
            let text_hash = sha256_hex(block.text.as_bytes());
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (w:WorkProduct {work_product_id: $work_product_id})
                         MERGE (b:WorkProductBlock {block_id: $block_id})
                         SET b.payload = $payload,
                             b.matter_id = $matter_id,
                             b.work_product_id = $work_product_id,
                             b.block_id = $block_id,
                             b.role = $role,
                             b.title = $title,
                             b.text = $text,
                             b.text_hash = $text_hash,
                             b.text_size_bytes = $text_size_bytes,
                             b.ordinal = $ordinal
                         MERGE (w)-[:HAS_BLOCK]->(b)
                         WITH b
                         OPTIONAL MATCH (parent:WorkProductBlock {block_id: $parent_block_id})
                         FOREACH (_ IN CASE WHEN parent IS NULL THEN [] ELSE [1] END |
                           MERGE (parent)-[:HAS_CHILD_BLOCK]->(b)
                         )",
                    )
                    .param("work_product_id", product.work_product_id.clone())
                    .param("matter_id", product.matter_id.clone())
                    .param("block_id", block.block_id.clone())
                    .param("role", block.role.clone())
                    .param("title", block.title.clone())
                    .param("text", text_excerpt)
                    .param("text_hash", text_hash)
                    .param("text_size_bytes", block.text.len() as i64)
                    .param("ordinal", block.ordinal as i64)
                    .param(
                        "parent_block_id",
                        block.parent_block_id.clone().unwrap_or_default(),
                    )
                    .param("payload", payload),
                )
                .await?;
            for fact_id in &block.fact_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                             MATCH (f:Fact {fact_id: $fact_id, matter_id: $matter_id})
                             MERGE (b)-[:SUPPORTED_BY_FACT]->(f)",
                        )
                        .param("block_id", block.block_id.clone())
                        .param("matter_id", product.matter_id.clone())
                        .param("fact_id", fact_id.clone()),
                    )
                    .await?;
            }
            for evidence_id in &block.evidence_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                             OPTIONAL MATCH (e:Evidence {evidence_id: $evidence_id, matter_id: $matter_id})
                             OPTIONAL MATCH (d:CaseDocument {document_id: $evidence_id, matter_id: $matter_id})
                             OPTIONAL MATCH (s:SourceSpan {source_span_id: $evidence_id, matter_id: $matter_id})
                             WITH b, coalesce(e, d, s) AS support
                             FOREACH (_ IN CASE WHEN support IS NULL THEN [] ELSE [1] END |
                               MERGE (b)-[:SUPPORTED_BY_EVIDENCE]->(support)
                             )",
                        )
                        .param("block_id", block.block_id.clone())
                        .param("matter_id", product.matter_id.clone())
                        .param("evidence_id", evidence_id.clone()),
                    )
                    .await?;
            }
            for authority in &block.authorities {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                             OPTIONAL MATCH (p:Provision {canonical_id: $canonical_id})
                             OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $canonical_id})
                             WITH b, coalesce(p, i) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (b)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                             )",
                        )
                        .param("block_id", block.block_id.clone())
                        .param("matter_id", product.matter_id.clone())
                        .param("canonical_id", authority.canonical_id.clone()),
                    )
                    .await?;
            }
        }

        for link in &product.document_ast.links {
            match link.target_type.as_str() {
                "fact" => {
                    self.neo4j
                        .run_rows(
                            query(
                                "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                                 MATCH (f:Fact {fact_id: $target_id, matter_id: $matter_id})
                                 MERGE (b)-[:SUPPORTED_BY_FACT]->(f)",
                            )
                            .param("block_id", link.source_block_id.clone())
                            .param("matter_id", product.matter_id.clone())
                            .param("target_id", link.target_id.clone()),
                        )
                        .await?;
                }
                "evidence" | "case_document" | "document_page" | "text_span" | "source_span" => {
                    self.neo4j
                        .run_rows(
                            query(
                                "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (e:Evidence {evidence_id: $target_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (d:CaseDocument {document_id: $target_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (s:SourceSpan {source_span_id: $target_id, matter_id: $matter_id})
                                 WITH b, coalesce(e, d, s) AS support
                                 FOREACH (_ IN CASE WHEN support IS NULL THEN [] ELSE [1] END |
                                   MERGE (b)-[:SUPPORTED_BY_EVIDENCE]->(support)
                                 )",
                            )
                            .param("block_id", link.source_block_id.clone())
                            .param("matter_id", product.matter_id.clone())
                            .param("target_id", link.target_id.clone()),
                        )
                        .await?;
                }
                "legal_authority" | "provision" | "legal_text_identity" | "legal_text" => {
                    self.neo4j
                        .run_rows(
                            query(
                                "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (p:Provision {canonical_id: $target_id})
                                 OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $target_id})
                                 WITH b, coalesce(p, i) AS authority
                                 FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                                   MERGE (b)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                                 )",
                            )
                            .param("block_id", link.source_block_id.clone())
                            .param("matter_id", product.matter_id.clone())
                            .param("target_id", link.target_id.clone()),
                        )
                        .await?;
                }
                "exhibit" => {
                    self.neo4j
                        .run_rows(
                            query(
                                "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (e:Evidence {evidence_id: $target_id, matter_id: $matter_id})
                                 OPTIONAL MATCH (d:CaseDocument {document_id: $target_id, matter_id: $matter_id})
                                 WITH b, coalesce(e, d) AS exhibit
                                 FOREACH (_ IN CASE WHEN exhibit IS NULL THEN [] ELSE [1] END |
                                   MERGE (b)-[:REFERENCES_EXHIBIT]->(exhibit)
                                 )",
                            )
                            .param("block_id", link.source_block_id.clone())
                            .param("matter_id", product.matter_id.clone())
                            .param("target_id", link.target_id.clone()),
                        )
                        .await?;
                }
                _ => {}
            }
        }

        for citation in &product.document_ast.citations {
            if let Some(target_id) = &citation.target_id {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                             OPTIONAL MATCH (p:Provision {canonical_id: $target_id})
                             OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $target_id})
                             WITH b, coalesce(p, i) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (b)-[:CITES]->(authority)
                             )",
                        )
                        .param("block_id", citation.source_block_id.clone())
                        .param("matter_id", product.matter_id.clone())
                        .param("target_id", target_id.clone()),
                    )
                    .await?;
            }
        }

        for anchor in &product.anchors {
            let payload = to_payload(anchor)?;
            let support_use = super::support_linker::legal_support_use_from_anchor(product, anchor);
            let support_payload = to_payload(&support_use)?;
            let support_label = super::support_linker::support_use_label(&support_use.source_type);
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (w:WorkProduct {work_product_id: $work_product_id, matter_id: $matter_id})
                         MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                         MERGE (a:WorkProductAnchor {anchor_id: $anchor_id})
                         SET a.payload = $payload,
                             a.matter_id = $matter_id,
                             a.work_product_id = $work_product_id,
                             a.anchor_id = $anchor_id,
                             a.anchor_type = $anchor_type,
                             a.target_type = $target_type,
                             a.target_id = $target_id,
                             a.relation = $relation,
                             a.status = $status
                         MERGE (w)-[:HAS_ANCHOR]->(a)
                         MERGE (b)-[:HAS_ANCHOR]->(a)
                         WITH a
                         OPTIONAL MATCH (f:Fact {fact_id: $target_id, matter_id: $matter_id})
                         OPTIONAL MATCH (e:Evidence {evidence_id: $target_id, matter_id: $matter_id})
                         OPTIONAL MATCH (d:CaseDocument {document_id: $target_id, matter_id: $matter_id})
                         OPTIONAL MATCH (p:Provision {canonical_id: $canonical_id})
                         OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $canonical_id})
                         WITH a, coalesce(f, e, d, p, i) AS target
                         FOREACH (_ IN CASE WHEN target IS NULL THEN [] ELSE [1] END |
                           MERGE (a)-[:RESOLVES_TO]->(target)
                         )",
                    )
                    .param("work_product_id", product.work_product_id.clone())
                    .param("matter_id", product.matter_id.clone())
                    .param("block_id", anchor.block_id.clone())
                    .param("anchor_id", anchor.anchor_id.clone())
                    .param("anchor_type", anchor.anchor_type.clone())
                    .param("target_type", anchor.target_type.clone())
                    .param("target_id", anchor.target_id.clone())
                    .param(
                        "canonical_id",
                        anchor.canonical_id.clone().unwrap_or_default(),
                    )
                    .param("relation", anchor.relation.clone())
                    .param("status", anchor.status.clone())
                    .param("payload", payload),
                )
                .await?;
            let support_statement = format!(
                "MATCH (w:WorkProduct {{work_product_id: $work_product_id, matter_id: $matter_id}})
                 MATCH (b:WorkProductBlock {{block_id: $block_id, matter_id: $matter_id}})
                 MERGE (u:LegalSupportUse:{support_label} {{support_use_id: $support_use_id}})
                 SET u.payload = $payload,
                     u.matter_id = $matter_id,
                     u.subject_id = $work_product_id,
                     u.branch_id = $branch_id,
                     u.target_type = $target_type,
                     u.target_id = $target_id,
                     u.source_type = $source_type,
                     u.source_id = $source_id,
                     u.relation = $relation,
                     u.status = $status
                 MERGE (w)-[:HAS_SUPPORT_USE]->(u)
                 MERGE (b)-[:HAS_SUPPORT_USE]->(u)",
            );
            self.neo4j
                .run_rows(
                    query(&support_statement)
                        .param("work_product_id", product.work_product_id.clone())
                        .param("block_id", anchor.block_id.clone())
                        .param("matter_id", product.matter_id.clone())
                        .param("branch_id", support_use.branch_id.clone())
                        .param("support_use_id", support_use.support_use_id.clone())
                        .param("target_type", support_use.target_type.clone())
                        .param("target_id", support_use.target_id.clone())
                        .param("source_type", support_use.source_type.clone())
                        .param("source_id", support_use.source_id.clone())
                        .param("relation", support_use.relation.clone())
                        .param("status", support_use.status.clone())
                        .param("payload", support_payload),
                )
                .await?;
            self.materialize_support_use_target(&support_use).await?;
        }

        for mark in &product.marks {
            let payload = to_payload(mark)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (b:WorkProductBlock {block_id: $block_id, matter_id: $matter_id})
                         MERGE (m:WorkProductMark {mark_id: $mark_id})
                         SET m.payload = $payload,
                             m.matter_id = $matter_id,
                             m.work_product_id = $work_product_id,
                             m.mark_id = $mark_id,
                             m.mark_type = $mark_type,
                             m.target_type = $target_type,
                             m.target_id = $target_id,
                             m.status = $status
                         MERGE (b)-[:HAS_MARK]->(m)",
                    )
                    .param("block_id", mark.block_id.clone())
                    .param("matter_id", product.matter_id.clone())
                    .param("work_product_id", product.work_product_id.clone())
                    .param("mark_id", mark.mark_id.clone())
                    .param("mark_type", mark.mark_type.clone())
                    .param("target_type", mark.target_type.clone())
                    .param("target_id", mark.target_id.clone())
                    .param("status", mark.status.clone())
                    .param("payload", payload),
                )
                .await?;
        }

        for finding in &product.findings {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (w:WorkProduct {work_product_id: $work_product_id, matter_id: $matter_id})
                         MERGE (f:WorkProductFinding {finding_id: $finding_id})
                         SET f.matter_id = $matter_id,
                             f.work_product_id = $work_product_id,
                             f.finding_id = $finding_id,
                             f.status = $status,
                             f.severity = $severity,
                             f.category = $category
                         MERGE (w)-[:HAS_FINDING]->(f)",
                    )
                    .param("work_product_id", product.work_product_id.clone())
                    .param("matter_id", product.matter_id.clone())
                    .param("finding_id", finding.finding_id.clone())
                    .param("status", finding.status.clone())
                    .param("severity", finding.severity.clone())
                    .param("category", finding.category.clone()),
                )
                .await?;
        }

        Ok(())
    }

    pub(super) async fn materialize_case_history_edges(
        &self,
        work_product_id: &str,
        branch: &VersionBranch,
        snapshot: &VersionSnapshot,
        manifest: &SnapshotManifest,
        entity_states: &[SnapshotEntityState],
        changes: &[VersionChange],
        change_set: &ChangeSet,
    ) -> ApiResult<()> {
        self.neo4j
            .run_rows(
                query(
                    "MATCH (w:WorkProduct {work_product_id: $work_product_id})
                     MATCH (b:VersionBranch {branch_id: $branch_id})
                     MATCH (s:VersionSnapshot {snapshot_id: $snapshot_id})
                     MATCH (cs:ChangeSet {change_set_id: $change_set_id})
                     MERGE (w)-[:HAS_BRANCH]->(b)
                     MERGE (b)-[:HAS_SNAPSHOT]->(s)
                     MERGE (b)-[:CURRENT_SNAPSHOT]->(s)
                     MERGE (s)-[:CREATED_BY_CHANGESET]->(cs)",
                )
                .param("work_product_id", work_product_id)
                .param("branch_id", branch.branch_id.clone())
                .param("snapshot_id", snapshot.snapshot_id.clone())
                .param("change_set_id", change_set.change_set_id.clone()),
            )
            .await?;
        for parent_id in &snapshot.parent_snapshot_ids {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (s:VersionSnapshot {snapshot_id: $snapshot_id})
                         MATCH (p:VersionSnapshot {snapshot_id: $parent_id})
                         MERGE (s)-[:HAS_PARENT]->(p)",
                    )
                    .param("snapshot_id", snapshot.snapshot_id.clone())
                    .param("parent_id", parent_id.clone()),
                )
                .await?;
        }
        self.neo4j
            .run_rows(
                query(
                    "MATCH (s:VersionSnapshot {snapshot_id: $snapshot_id})
                     MATCH (m:SnapshotManifest {manifest_id: $manifest_id})
                     MERGE (s)-[:HAS_MANIFEST]->(m)",
                )
                .param("snapshot_id", snapshot.snapshot_id.clone())
                .param("manifest_id", manifest.manifest_id.clone()),
            )
            .await?;
        for object_blob_id in [
            snapshot.full_state_ref.as_ref(),
            snapshot.manifest_ref.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (s:VersionSnapshot {snapshot_id: $snapshot_id})
                         MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                         MERGE (s)-[:STORED_AS]->(b)",
                    )
                    .param("snapshot_id", snapshot.snapshot_id.clone())
                    .param("object_blob_id", object_blob_id.clone()),
                )
                .await?;
        }
        for state in entity_states {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (m:SnapshotManifest {manifest_id: $manifest_id})
                         MATCH (es:SnapshotEntityState {entity_state_id: $entity_state_id})
                         MERGE (m)-[:HAS_ENTITY_STATE]->(es)",
                    )
                    .param("manifest_id", manifest.manifest_id.clone())
                    .param("entity_state_id", state.entity_state_id.clone()),
                )
                .await?;
            if let Some(object_blob_id) = state.state_ref.as_ref() {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (es:SnapshotEntityState {entity_state_id: $entity_state_id})
                             MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                             MERGE (es)-[:STORED_AS]->(b)",
                        )
                        .param("entity_state_id", state.entity_state_id.clone())
                        .param("object_blob_id", object_blob_id.clone()),
                    )
                    .await?;
            }
        }
        for change in changes {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (cs:ChangeSet {change_set_id: $change_set_id})
                         MATCH (c:VersionChange {change_id: $change_id})
                         MERGE (cs)-[:HAS_CHANGE]->(c)",
                    )
                    .param("change_set_id", change_set.change_set_id.clone())
                    .param("change_id", change.change_id.clone()),
                )
                .await?;
            if let Some(ai_audit_id) = &change.ai_audit_id {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (c:VersionChange {change_id: $change_id})
                             MATCH (a:AIEditAudit {ai_audit_id: $ai_audit_id})
                             MERGE (c)-[:AI_AUDIT]->(a)",
                        )
                        .param("change_id", change.change_id.clone())
                        .param("ai_audit_id", ai_audit_id.clone()),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    pub(super) async fn materialize_version_subject(&self, product: &WorkProduct) -> ApiResult<()> {
        self.neo4j
            .run_rows(
                query(
                    "MATCH (m:Matter {matter_id: $matter_id})
                     MATCH (w:WorkProduct {work_product_id: $work_product_id})
                     MERGE (m)-[:HAS_VERSION_SUBJECT]->(w)",
                )
                .param("matter_id", product.matter_id.clone())
                .param("work_product_id", product.work_product_id.clone()),
            )
            .await?;
        Ok(())
    }

    pub(super) async fn materialize_support_use_target(
        &self,
        support_use: &LegalSupportUse,
    ) -> ApiResult<()> {
        match support_use.source_type.as_str() {
            "fact" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (u:LegalSupportUse {support_use_id: $support_use_id, matter_id: $matter_id})
                             MATCH (f:Fact {fact_id: $source_id, matter_id: $matter_id})
                             MERGE (u)-[:USES_FACT]->(f)",
                        )
                        .param("support_use_id", support_use.support_use_id.clone())
                        .param("matter_id", support_use.matter_id.clone())
                        .param("source_id", support_use.source_id.clone()),
                    )
                    .await?;
            }
            "evidence" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (u:LegalSupportUse {support_use_id: $support_use_id, matter_id: $matter_id})
                             MATCH (e:Evidence {evidence_id: $source_id, matter_id: $matter_id})
                             MERGE (u)-[:USES_EVIDENCE]->(e)",
                        )
                        .param("support_use_id", support_use.support_use_id.clone())
                        .param("matter_id", support_use.matter_id.clone())
                        .param("source_id", support_use.source_id.clone()),
                    )
                    .await?;
            }
            "source_span" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (u:LegalSupportUse {support_use_id: $support_use_id, matter_id: $matter_id})
                             MATCH (s:SourceSpan {source_span_id: $source_id, matter_id: $matter_id})
                             MERGE (u)-[:USES_SPAN]->(s)",
                        )
                        .param("support_use_id", support_use.support_use_id.clone())
                        .param("matter_id", support_use.matter_id.clone())
                        .param("source_id", support_use.source_id.clone()),
                    )
                    .await?;
            }
            "authority" | "provision" | "citation" => {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (u:LegalSupportUse {support_use_id: $support_use_id, matter_id: $matter_id})
                             OPTIONAL MATCH (p:Provision {canonical_id: $source_id})
                             OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $source_id})
                             WITH u, coalesce(p, i) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (u)-[:USES_AUTHORITY]->(authority)
                             )",
                        )
                        .param("support_use_id", support_use.support_use_id.clone())
                        .param("matter_id", support_use.matter_id.clone())
                        .param("source_id", support_use.source_id.clone()),
                    )
                    .await?;
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) async fn materialize_complaint_edges(
        &self,
        complaint: &ComplaintDraft,
    ) -> ApiResult<()> {
        for section in &complaint.sections {
            let payload = to_payload(section)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:ComplaintDraft {complaint_id: $complaint_id})
                         MERGE (s:ComplaintSection {section_id: $section_id})
                         SET s.payload = $payload,
                             s.matter_id = $matter_id,
                             s.complaint_id = $complaint_id,
                             s.section_id = $section_id,
                             s.title = $title
                         MERGE (c)-[:HAS_SECTION]->(s)",
                    )
                    .param("complaint_id", complaint.complaint_id.clone())
                    .param("matter_id", complaint.matter_id.clone())
                    .param("section_id", section.section_id.clone())
                    .param("title", section.title.clone())
                    .param("payload", payload),
                )
                .await?;
        }

        for count in &complaint.counts {
            let payload = to_payload(count)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:ComplaintDraft {complaint_id: $complaint_id})
                         MERGE (n:ComplaintCount {count_id: $count_id})
                         SET n.payload = $payload,
                             n.matter_id = $matter_id,
                             n.complaint_id = $complaint_id,
                             n.count_id = $count_id,
                             n.title = $title,
                             n.legal_theory = $legal_theory
                         MERGE (c)-[:HAS_COUNT]->(n)
                         WITH n
                         OPTIONAL MATCH (claim:Claim {claim_id: $claim_id})
                         FOREACH (_ IN CASE WHEN claim IS NULL THEN [] ELSE [1] END |
                           MERGE (n)-[:IMPLEMENTS_CLAIM]->(claim)
                         )",
                    )
                    .param("complaint_id", complaint.complaint_id.clone())
                    .param("matter_id", complaint.matter_id.clone())
                    .param("count_id", count.count_id.clone())
                    .param("title", count.title.clone())
                    .param("legal_theory", count.legal_theory.clone())
                    .param("claim_id", count.claim_id.clone().unwrap_or_default())
                    .param("payload", payload),
                )
                .await?;
            for fact_id in &count.fact_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (n:ComplaintCount {count_id: $count_id})
                             MATCH (f:Fact {fact_id: $fact_id})
                             MERGE (n)-[:SUPPORTED_BY_FACT]->(f)",
                        )
                        .param("count_id", count.count_id.clone())
                        .param("fact_id", fact_id.clone()),
                    )
                    .await?;
            }
            for evidence_id in &count.evidence_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (n:ComplaintCount {count_id: $count_id})
                             MATCH (e:Evidence {evidence_id: $evidence_id})
                             MERGE (n)-[:SUPPORTED_BY_EVIDENCE]->(e)",
                        )
                        .param("count_id", count.count_id.clone())
                        .param("evidence_id", evidence_id.clone()),
                    )
                    .await?;
            }
            for authority in &count.authorities {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (n:ComplaintCount {count_id: $count_id})
                             OPTIONAL MATCH (p:Provision {canonical_id: $canonical_id})
                             OPTIONAL MATCH (i:LegalTextIdentity {canonical_id: $canonical_id})
                             WITH n, coalesce(p, i) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (n)-[:SUPPORTED_BY_AUTHORITY]->(authority)
                             )",
                        )
                        .param("count_id", count.count_id.clone())
                        .param("canonical_id", authority.canonical_id.clone()),
                    )
                    .await?;
            }
        }

        for paragraph in &complaint.paragraphs {
            let payload = to_payload(paragraph)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:ComplaintDraft {complaint_id: $complaint_id})
                         MERGE (p:PleadingParagraph {paragraph_id: $paragraph_id})
                         SET p.payload = $payload,
                             p.matter_id = $matter_id,
                             p.complaint_id = $complaint_id,
                             p.paragraph_id = $paragraph_id,
                             p.number = $number,
                             p.display_number = $display_number,
                             p.original_label = $original_label,
                             p.source_span_id = $source_span_id,
                             p.role = $role,
                             p.text = $text
                         MERGE (c)-[:HAS_PARAGRAPH]->(p)
                         WITH p
                         OPTIONAL MATCH (s:ComplaintSection {section_id: $section_id})
                         OPTIONAL MATCH (n:ComplaintCount {count_id: $count_id})
                         OPTIONAL MATCH (source_span:SourceSpan {source_span_id: $source_span_id})
                         FOREACH (_ IN CASE WHEN s IS NULL THEN [] ELSE [1] END |
                           MERGE (s)-[:HAS_PARAGRAPH]->(p)
                         )
                         FOREACH (_ IN CASE WHEN n IS NULL THEN [] ELSE [1] END |
                           MERGE (n)-[:HAS_PARAGRAPH]->(p)
                         )
                         FOREACH (_ IN CASE WHEN source_span IS NULL THEN [] ELSE [1] END |
                           MERGE (p)-[:DERIVED_FROM]->(source_span)
                         )",
                    )
                    .param("complaint_id", complaint.complaint_id.clone())
                    .param("matter_id", complaint.matter_id.clone())
                    .param("paragraph_id", paragraph.paragraph_id.clone())
                    .param("number", paragraph.number as i64)
                    .param(
                        "display_number",
                        paragraph
                            .display_number
                            .clone()
                            .unwrap_or_else(|| paragraph.number.to_string()),
                    )
                    .param(
                        "original_label",
                        paragraph.original_label.clone().unwrap_or_default(),
                    )
                    .param(
                        "source_span_id",
                        paragraph.source_span_id.clone().unwrap_or_default(),
                    )
                    .param("role", paragraph.role.clone())
                    .param("text", paragraph.text.clone())
                    .param(
                        "section_id",
                        paragraph.section_id.clone().unwrap_or_default(),
                    )
                    .param("count_id", paragraph.count_id.clone().unwrap_or_default())
                    .param("payload", payload),
                )
                .await?;
            for sentence in &paragraph.sentences {
                let payload = to_payload(sentence)?;
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:PleadingParagraph {paragraph_id: $paragraph_id})
                             MERGE (s:PleadingSentence {sentence_id: $sentence_id})
                             SET s.payload = $payload,
                                 s.matter_id = $matter_id,
                                 s.complaint_id = $complaint_id,
                                 s.paragraph_id = $paragraph_id,
                                 s.sentence_id = $sentence_id,
                                 s.text = $text
                             MERGE (p)-[:HAS_SENTENCE]->(s)",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("matter_id", complaint.matter_id.clone())
                        .param("complaint_id", complaint.complaint_id.clone())
                        .param("sentence_id", sentence.sentence_id.clone())
                        .param("text", sentence.text.clone())
                        .param("payload", payload),
                    )
                    .await?;
            }
            for fact_id in &paragraph.fact_ids {
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:PleadingParagraph {paragraph_id: $paragraph_id})
                             MATCH (f:Fact {fact_id: $fact_id})
                             MERGE (p)-[:SUPPORTED_BY_FACT]->(f)",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("fact_id", fact_id.clone()),
                    )
                    .await?;
            }
            for evidence_use in &paragraph.evidence_uses {
                let payload = to_payload(evidence_use)?;
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:PleadingParagraph {paragraph_id: $paragraph_id})
                             MERGE (u:EvidenceUse {evidence_use_id: $evidence_use_id})
                             SET u.payload = $payload,
                                 u.matter_id = $matter_id,
                                 u.complaint_id = $complaint_id,
                                 u.evidence_use_id = $evidence_use_id,
                                 u.relation = $relation
                             MERGE (p)-[:HAS_EVIDENCE_USE]->(u)
                             WITH u
                             OPTIONAL MATCH (e:Evidence {evidence_id: $evidence_id})
                             OPTIONAL MATCH (d:CaseDocument {document_id: $document_id})
                             OPTIONAL MATCH (span:SourceSpan {source_span_id: $source_span_id})
                             FOREACH (_ IN CASE WHEN e IS NULL THEN [] ELSE [1] END |
                               MERGE (u)-[:USES_EVIDENCE]->(e)
                             )
                             FOREACH (_ IN CASE WHEN d IS NULL THEN [] ELSE [1] END |
                               MERGE (u)-[:USES_DOCUMENT]->(d)
                             )
                             FOREACH (_ IN CASE WHEN span IS NULL THEN [] ELSE [1] END |
                               MERGE (u)-[:USES_SPAN]->(span)
                             )",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("matter_id", complaint.matter_id.clone())
                        .param("complaint_id", complaint.complaint_id.clone())
                        .param("evidence_use_id", evidence_use.evidence_use_id.clone())
                        .param("relation", evidence_use.relation.clone())
                        .param(
                            "evidence_id",
                            evidence_use.evidence_id.clone().unwrap_or_default(),
                        )
                        .param(
                            "document_id",
                            evidence_use.document_id.clone().unwrap_or_default(),
                        )
                        .param(
                            "source_span_id",
                            evidence_use.source_span_id.clone().unwrap_or_default(),
                        )
                        .param("payload", payload),
                    )
                    .await?;
            }
            for citation_use in &paragraph.citation_uses {
                let payload = to_payload(citation_use)?;
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:PleadingParagraph {paragraph_id: $paragraph_id})
                             MERGE (u:CitationUse {citation_use_id: $citation_use_id})
                             SET u.payload = $payload,
                                 u.matter_id = $matter_id,
                                 u.complaint_id = $complaint_id,
                                 u.citation_use_id = $citation_use_id,
                                 u.citation = $citation,
                                 u.status = $status
                             MERGE (p)-[:HAS_CITATION_USE]->(u)
                             WITH u
                             OPTIONAL MATCH (provision:Provision {canonical_id: $canonical_id})
                             OPTIONAL MATCH (identity:LegalTextIdentity {canonical_id: $canonical_id})
                             WITH u, coalesce(provision, identity) AS authority
                             FOREACH (_ IN CASE WHEN authority IS NULL THEN [] ELSE [1] END |
                               MERGE (u)-[:RESOLVES_TO]->(authority)
                             )
                             WITH u, authority
                             FOREACH (_ IN CASE WHEN authority IS NULL AND $canonical_id <> '' AND $status <> 'unresolved' THEN [1] ELSE [] END |
                               MERGE (external:ExternalAuthority {canonical_id: $canonical_id})
	                               SET external.canonical_id = $canonical_id,
	                                   external.citation = $citation,
	                                   external.authority_type = $authority_type,
	                                   external.source = 'source_backed_external_rule',
	                                   external.source_url = $source_url,
	                                   external.edition = $authority_edition,
	                                   external.currentness = $currentness
	                               MERGE (u)-[:RESOLVES_TO]->(external)
	                             )",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("matter_id", complaint.matter_id.clone())
                        .param("complaint_id", complaint.complaint_id.clone())
	                        .param("citation_use_id", citation_use.citation_use_id.clone())
	                        .param("citation", citation_use.citation.clone())
	                        .param("status", citation_use.status.clone())
	                        .param("currentness", citation_use.currentness.clone())
	                        .param("authority_type", authority_type_for_citation(&citation_use.citation))
	                        .param("source_url", authority_source_url_for_citation(&citation_use.citation))
	                        .param(
	                            "authority_edition",
	                            authority_edition_for_citation(&citation_use.citation),
	                        )
	                        .param("canonical_id", citation_use.canonical_id.clone().unwrap_or_default())
	                        .param("payload", payload),
	                )
                    .await?;
            }
            for exhibit_reference in &paragraph.exhibit_references {
                let payload = to_payload(exhibit_reference)?;
                self.neo4j
                    .run_rows(
                        query(
                            "MATCH (p:PleadingParagraph {paragraph_id: $paragraph_id})
                             MERGE (x:ExhibitReference {exhibit_reference_id: $exhibit_reference_id})
                             SET x.payload = $payload,
                                 x.matter_id = $matter_id,
                                 x.complaint_id = $complaint_id,
                                 x.exhibit_reference_id = $exhibit_reference_id,
                                 x.exhibit_label = $exhibit_label,
                                 x.status = $status
                             MERGE (p)-[:HAS_EXHIBIT_REFERENCE]->(x)
                             WITH x
                             OPTIONAL MATCH (d:CaseDocument {document_id: $document_id})
                             OPTIONAL MATCH (e:Evidence {evidence_id: $evidence_id})
                             FOREACH (_ IN CASE WHEN d IS NULL THEN [] ELSE [1] END |
                               MERGE (x)-[:REFERENCES_DOCUMENT]->(d)
                             )
                             FOREACH (_ IN CASE WHEN e IS NULL THEN [] ELSE [1] END |
                               MERGE (x)-[:REFERENCES_EVIDENCE]->(e)
                             )",
                        )
                        .param("paragraph_id", paragraph.paragraph_id.clone())
                        .param("matter_id", complaint.matter_id.clone())
                        .param("complaint_id", complaint.complaint_id.clone())
                        .param("exhibit_reference_id", exhibit_reference.exhibit_reference_id.clone())
                        .param("exhibit_label", exhibit_reference.exhibit_label.clone())
                        .param("status", exhibit_reference.status.clone())
                        .param("document_id", exhibit_reference.document_id.clone().unwrap_or_default())
                        .param("evidence_id", exhibit_reference.evidence_id.clone().unwrap_or_default())
                        .param("payload", payload),
                    )
                    .await?;
            }
        }

        for relief in &complaint.relief {
            let payload = to_payload(relief)?;
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:ComplaintDraft {complaint_id: $complaint_id})
                         MERGE (r:ReliefRequest {relief_id: $relief_id})
                         SET r.payload = $payload,
                             r.matter_id = $matter_id,
                             r.complaint_id = $complaint_id,
                             r.relief_id = $relief_id,
                             r.category = $category,
                             r.text = $text
                         MERGE (c)-[:REQUESTS_RELIEF]->(r)",
                    )
                    .param("complaint_id", complaint.complaint_id.clone())
                    .param("matter_id", complaint.matter_id.clone())
                    .param("relief_id", relief.relief_id.clone())
                    .param("category", relief.category.clone())
                    .param("text", relief.text.clone())
                    .param("payload", payload),
                )
                .await?;
        }

        for finding in &complaint.findings {
            self.neo4j
                .run_rows(
                    query(
                        "MATCH (c:ComplaintDraft {complaint_id: $complaint_id})
                         MERGE (f:RuleCheckFinding {finding_id: $finding_id})
                         SET f.matter_id = $matter_id,
                             f.complaint_id = $complaint_id,
                             f.finding_id = $finding_id,
                             f.status = $status,
                             f.severity = $severity,
                             f.category = $category
                         MERGE (c)-[:HAS_RULE_FINDING]->(f)",
                    )
                    .param("complaint_id", complaint.complaint_id.clone())
                    .param("matter_id", complaint.matter_id.clone())
                    .param("finding_id", finding.finding_id.clone())
                    .param("status", finding.status.clone())
                    .param("severity", finding.severity.clone())
                    .param("category", finding.category.clone()),
                )
                .await?;
        }

        Ok(())
    }

    pub(super) async fn sync_fact_evidence_link(
        &self,
        matter_id: &str,
        evidence_id: &str,
        fact_id: &str,
        relation: &str,
    ) -> ApiResult<()> {
        let mut fact = self
            .get_node::<CaseFact>(matter_id, fact_spec(), fact_id)
            .await?;
        if relation == "contradicts" {
            push_unique(
                &mut fact.contradicted_by_evidence_ids,
                evidence_id.to_string(),
            );
            fact.status = "contradicted".to_string();
            fact.needs_verification = true;
        } else {
            push_unique(&mut fact.source_evidence_ids, evidence_id.to_string());
            if matches!(
                fact.status.as_str(),
                "proposed" | "alleged" | "needs_evidence"
            ) {
                fact.status = "supported".to_string();
            }
            fact.confidence = fact.confidence.max(0.8);
            fact.needs_verification = false;
        }
        let fact = self
            .merge_node(matter_id, fact_spec(), fact_id, &fact)
            .await?;
        self.materialize_fact_edges(&fact).await
    }

    pub(super) async fn sync_claim_element_evidence(
        &self,
        matter_id: &str,
        evidence_id: &str,
        fact_id: &str,
    ) -> ApiResult<()> {
        for mut claim in self.list_claims(matter_id).await? {
            let mut changed = false;
            for element in &mut claim.elements {
                if element.fact_ids.contains(&fact_id.to_string()) {
                    push_unique(&mut element.evidence_ids, evidence_id.to_string());
                    element.satisfied = true;
                    changed = true;
                }
            }
            if claim.fact_ids.contains(&fact_id.to_string()) {
                push_unique(&mut claim.evidence_ids, evidence_id.to_string());
                changed = true;
            }
            if changed {
                let claim = self
                    .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
                    .await?;
                self.materialize_claim_edges(&claim).await?;
            }
        }
        Ok(())
    }

    pub(super) async fn detach_authority_edge(
        &self,
        label: &str,
        id_key: &str,
        id: &str,
        authority: &AuthorityRef,
    ) -> ApiResult<()> {
        let statement = format!(
            "MATCH (n:{label} {{{id_key}: $id}})-[r:SUPPORTED_BY_AUTHORITY]->(authority)
             WHERE authority.canonical_id = $canonical_id
             DELETE r",
            label = label,
            id_key = id_key,
        );
        self.neo4j
            .run_rows(
                query(&statement)
                    .param("id", id.to_string())
                    .param("canonical_id", authority.canonical_id.clone()),
            )
            .await?;
        Ok(())
    }
}
