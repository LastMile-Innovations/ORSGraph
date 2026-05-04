use super::*;
use crate::auth::AuthContext;

impl CaseBuilderService {
    pub async fn list_matters(&self) -> ApiResult<Vec<MatterSummary>> {
        let rows = self
            .neo4j
            .run_rows(query(
                "MATCH (m:Matter)
                 OPTIONAL MATCH (m)-[:HAS_DOCUMENT]->(doc:CaseDocument)
                 WITH m, count(DISTINCT doc) AS document_count
                 OPTIONAL MATCH (m)-[:HAS_FACT]->(fact:Fact)
                 WITH m, document_count, count(DISTINCT fact) AS fact_count
                 OPTIONAL MATCH (m)-[:HAS_EVIDENCE]->(evidence:Evidence)
                 WITH m, document_count, fact_count, count(DISTINCT evidence) AS evidence_count
                 OPTIONAL MATCH (m)-[:HAS_CLAIM]->(claim:Claim)
                 WITH m, document_count, fact_count, evidence_count, count(DISTINCT claim) AS claim_count
                 OPTIONAL MATCH (m)-[:HAS_DRAFT]->(draft:Draft)
                 WITH m, document_count, fact_count, evidence_count, claim_count, count(DISTINCT draft) AS draft_count
                 OPTIONAL MATCH (m)-[:HAS_TASK]->(task:Task)
                 WITH m, document_count, fact_count, evidence_count, claim_count, draft_count,
                      count(DISTINCT CASE WHEN task.status <> 'done' THEN task END) AS open_task_count
                 RETURN m.payload AS payload,
                        document_count, fact_count, evidence_count, claim_count, draft_count, open_task_count
                 ORDER BY m.updated_at DESC",
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                let payload = row
                    .get::<String>("payload")
                    .map_err(|error| ApiError::Internal(error.to_string()))?;
                let mut matter = from_payload::<MatterSummary>(&payload)?;
                matter.document_count = row_u64(&row, "document_count");
                matter.fact_count = row_u64(&row, "fact_count");
                matter.evidence_count = row_u64(&row, "evidence_count");
                matter.claim_count = row_u64(&row, "claim_count");
                matter.draft_count = row_u64(&row, "draft_count");
                matter.open_task_count = row_u64(&row, "open_task_count");
                Ok(matter)
            })
            .collect()
    }

    pub async fn list_matters_for_auth(&self, auth: &AuthContext) -> ApiResult<Vec<MatterSummary>> {
        if auth.is_service() {
            return self.list_matters().await;
        }
        let subject = auth.subject()?;
        let rows = self
            .neo4j
            .run_rows(
                query(
                    "MATCH (m:Matter {owner_subject: $owner_subject})
                     OPTIONAL MATCH (m)-[:HAS_DOCUMENT]->(doc:CaseDocument)
                     WITH m, count(DISTINCT doc) AS document_count
                     OPTIONAL MATCH (m)-[:HAS_FACT]->(fact:Fact)
                     WITH m, document_count, count(DISTINCT fact) AS fact_count
                     OPTIONAL MATCH (m)-[:HAS_EVIDENCE]->(evidence:Evidence)
                     WITH m, document_count, fact_count, count(DISTINCT evidence) AS evidence_count
                     OPTIONAL MATCH (m)-[:HAS_CLAIM]->(claim:Claim)
                     WITH m, document_count, fact_count, evidence_count, count(DISTINCT claim) AS claim_count
                     OPTIONAL MATCH (m)-[:HAS_DRAFT]->(draft:Draft)
                     WITH m, document_count, fact_count, evidence_count, claim_count, count(DISTINCT draft) AS draft_count
                     OPTIONAL MATCH (m)-[:HAS_TASK]->(task:Task)
                     WITH m, document_count, fact_count, evidence_count, claim_count, draft_count,
                          count(DISTINCT CASE WHEN task.status <> 'done' THEN task END) AS open_task_count
                     RETURN m.payload AS payload,
                            document_count, fact_count, evidence_count, claim_count, draft_count, open_task_count
                     ORDER BY m.updated_at DESC",
                )
                .param("owner_subject", subject),
            )
            .await?;

        rows.into_iter()
            .map(|row| {
                let payload = row
                    .get::<String>("payload")
                    .map_err(|error| ApiError::Internal(error.to_string()))?;
                let mut matter = from_payload::<MatterSummary>(&payload)?;
                matter.document_count = row_u64(&row, "document_count");
                matter.fact_count = row_u64(&row, "fact_count");
                matter.evidence_count = row_u64(&row, "evidence_count");
                matter.claim_count = row_u64(&row, "claim_count");
                matter.draft_count = row_u64(&row, "draft_count");
                matter.open_task_count = row_u64(&row, "open_task_count");
                Ok(matter)
            })
            .collect()
    }

    pub async fn create_matter(
        &self,
        request: CreateMatterRequest,
        auth: &AuthContext,
    ) -> ApiResult<MatterBundle> {
        let request = normalize_create_matter_request(request)?;
        let now = now_string();
        let matter_id = generate_id("matter", &request.name);
        let owner_subject = auth.subject()?.to_string();
        let matter = MatterSummary {
            matter_id: matter_id.clone(),
            short_name: Some(short_name(&request.name)),
            name: request.name,
            matter_type: request.matter_type,
            status: "intake".to_string(),
            user_role: request.user_role,
            jurisdiction: request.jurisdiction,
            court: request.court,
            case_number: request.case_number,
            owner_subject: Some(owner_subject.clone()),
            owner_email: auth.email.clone(),
            owner_name: auth.name.clone(),
            created_by_subject: Some(owner_subject),
            created_at: now.clone(),
            updated_at: now,
            document_count: 0,
            fact_count: 0,
            evidence_count: 0,
            claim_count: 0,
            draft_count: 0,
            open_task_count: 0,
            next_deadline: None,
        };

        self.merge_matter(&matter).await?;
        if let Some(settings) = request.settings {
            self.create_initial_matter_settings(&matter_id, settings, auth)
                .await?;
        }
        self.get_matter(&matter_id).await
    }

    pub async fn authorize_matter_access(
        &self,
        matter_id: &str,
        auth: &AuthContext,
        admin_role: &str,
    ) -> ApiResult<()> {
        if auth.is_service() {
            return Ok(());
        }
        let matter = self.get_matter_summary(matter_id).await?;
        if auth.is_admin(admin_role) {
            return Ok(());
        }
        let subject = auth.subject()?;
        match matter.owner_subject.as_deref() {
            Some(owner) if owner == subject => Ok(()),
            Some(_) => Err(ApiError::Forbidden("Matter access denied".to_string())),
            None => Err(ApiError::NotFound(format!("Matter {matter_id} not found"))),
        }
    }

    pub async fn claim_ownerless_matters(
        &self,
        request: ClaimOwnerlessMattersRequest,
    ) -> ApiResult<Vec<MatterSummary>> {
        let owner_subject = request.owner_subject.trim();
        if owner_subject.is_empty() {
            return Err(ApiError::BadRequest(
                "owner_subject must not be empty".to_string(),
            ));
        }
        let limit = request.limit.unwrap_or(50).clamp(1, 500) as i64;
        let rows = self
            .neo4j
            .run_rows(
                query(
                    "MATCH (m:Matter)
                     WHERE m.owner_subject IS NULL
                     RETURN m.payload AS payload
                     ORDER BY m.updated_at DESC
                     LIMIT $limit",
                )
                .param("limit", limit),
            )
            .await?;
        let mut claimed = Vec::new();
        for row in rows {
            let payload = row
                .get::<String>("payload")
                .map_err(|error| ApiError::Internal(error.to_string()))?;
            let mut matter = from_payload::<MatterSummary>(&payload)?;
            matter.owner_subject = Some(owner_subject.to_string());
            matter.owner_email = request.owner_email.clone();
            matter.owner_name = request.owner_name.clone();
            if matter.created_by_subject.is_none() {
                matter.created_by_subject = Some(owner_subject.to_string());
            }
            matter.updated_at = now_string();
            claimed.push(self.merge_matter(&matter).await?);
        }
        Ok(claimed)
    }

    pub async fn get_matter(&self, matter_id: &str) -> ApiResult<MatterBundle> {
        let summary = self.get_matter_summary(matter_id).await?;
        Ok(MatterBundle {
            id: summary.matter_id.clone(),
            title: summary.name.clone(),
            documents: self.list_documents(matter_id).await?,
            parties: self.list_parties(matter_id).await?,
            facts: self.list_facts(matter_id).await?,
            timeline: self.list_timeline(matter_id).await?,
            timeline_suggestions: self.list_timeline_suggestions(matter_id).await?,
            timeline_agent_runs: self
                .list_nodes(matter_id, timeline_agent_run_spec())
                .await?,
            claims: self.list_claims(matter_id).await?,
            evidence: self.list_evidence(matter_id).await?,
            defenses: self.list_defenses(matter_id).await?,
            deadlines: self.list_deadlines(matter_id).await?,
            tasks: self.list_tasks(matter_id).await?,
            drafts: self.list_drafts(matter_id).await?,
            work_products: self.list_work_products(matter_id).await?,
            fact_check_findings: self.list_fact_check_findings(matter_id, None).await?,
            citation_check_findings: self.list_citation_check_findings(matter_id, None).await?,
            summary,
        })
    }

    pub async fn get_matter_graph(&self, matter_id: &str) -> ApiResult<CaseGraphResponse> {
        let matter = self.get_matter(matter_id).await?;
        let artifacts = CaseGraphArtifacts {
            document_versions: self.list_nodes(matter_id, document_version_spec()).await?,
            index_runs: self.list_nodes(matter_id, index_run_spec()).await?,
            source_spans: self.list_nodes(matter_id, source_span_spec()).await?,
            text_chunks: self.list_nodes(matter_id, text_chunk_spec()).await?,
            evidence_spans: self.list_nodes(matter_id, evidence_span_spec()).await?,
            entity_mentions: self.list_nodes(matter_id, entity_mention_spec()).await?,
            entities: self.list_nodes(matter_id, case_entity_spec()).await?,
            search_index_records: self
                .list_nodes(matter_id, search_index_record_spec())
                .await?,
            extraction_manifests: self
                .list_nodes(matter_id, extraction_artifact_manifest_spec())
                .await?,
            markdown_ast_documents: self
                .list_nodes(matter_id, markdown_ast_document_spec())
                .await?,
            markdown_ast_nodes: self.list_nodes(matter_id, markdown_ast_node_spec()).await?,
            markdown_semantic_units: self
                .list_nodes(matter_id, markdown_semantic_unit_spec())
                .await?,
            embedding_runs: self
                .list_nodes(matter_id, casebuilder_embedding_run_spec())
                .await?,
            embedding_records: self
                .list_nodes(matter_id, casebuilder_embedding_record_spec())
                .await?,
        };
        Ok(build_case_graph(&matter, &artifacts))
    }

    pub async fn run_matter_qc(&self, matter_id: &str) -> ApiResult<QcRun> {
        let matter = self.get_matter(matter_id).await?;
        Ok(build_matter_qc_run(&matter))
    }

    pub async fn spot_issues(
        &self,
        matter_id: &str,
        request: IssueSpotRequest,
    ) -> ApiResult<IssueSpotResponse> {
        let matter = self.get_matter(matter_id).await?;
        Ok(build_issue_spot_response(&matter, request))
    }

    pub async fn create_matter_export_package(
        &self,
        matter_id: &str,
        format: &str,
    ) -> ApiResult<AiActionResponse<ExportPackage>> {
        let matter = self.get_matter(matter_id).await?;
        let package = build_matter_export_package(&matter, format);
        Ok(AiActionResponse {
            enabled: true,
            mode: "deterministic_package".to_string(),
            message: format!(
                "{} export package prepared with review-needed warnings.",
                humanize_export_format(format)
            ),
            result: Some(package),
        })
    }

    pub async fn list_matter_audit_events(&self, matter_id: &str) -> ApiResult<Vec<AuditEvent>> {
        let matter = self.get_matter(matter_id).await?;
        Ok(build_matter_audit_events(&matter))
    }

    pub async fn patch_matter(
        &self,
        matter_id: &str,
        request: PatchMatterRequest,
    ) -> ApiResult<MatterBundle> {
        let request = normalize_patch_matter_request(request)?;
        let mut matter = self.get_matter_summary(matter_id).await?;
        if let Some(value) = request.name {
            matter.name = value;
            matter.short_name = Some(short_name(&matter.name));
        }
        if let Some(value) = request.matter_type {
            matter.matter_type = value;
        }
        if let Some(value) = request.status {
            matter.status = value;
        }
        if let Some(value) = request.user_role {
            matter.user_role = value;
        }
        if let Some(value) = request.jurisdiction {
            matter.jurisdiction = value;
        }
        if let Some(value) = request.court {
            matter.court = value;
        }
        if let Some(value) = request.case_number {
            matter.case_number = value;
        }
        matter.updated_at = now_string();
        self.merge_matter(&matter).await?;
        self.get_matter(matter_id).await
    }

    pub async fn delete_matter(&self, matter_id: &str) -> ApiResult<()> {
        self.require_matter(matter_id).await?;

        let (mut storage_keys, object_blob_ids) = self.matter_delete_targets(matter_id).await?;
        match self.list_documents(matter_id).await {
            Ok(documents) => {
                for document in documents {
                    if let Some(key) = document.storage_key {
                        storage_keys.insert(key);
                    }
                }
            }
            Err(error) => {
                tracing::warn!(
                    matter_id,
                    "Failed to enumerate legacy document storage keys before matter delete: {}",
                    error
                );
            }
        }
        match self
            .list_nodes::<DocumentVersion>(matter_id, document_version_spec())
            .await
        {
            Ok(versions) => {
                for version in versions {
                    storage_keys.insert(version.storage_key);
                }
            }
            Err(error) => {
                tracing::warn!(
                    matter_id,
                    "Failed to enumerate document version storage keys before matter delete: {}",
                    error
                );
            }
        }
        match self
            .list_nodes::<IngestionRun>(matter_id, ingestion_run_spec())
            .await
        {
            Ok(runs) => {
                for run in runs {
                    storage_keys.extend(run.produced_object_keys);
                }
            }
            Err(error) => {
                tracing::warn!(
                    matter_id,
                    "Failed to enumerate ingestion-run storage keys before matter delete: {}",
                    error
                );
            }
        }

        self.neo4j
            .run_rows(
                query(
                    "MATCH (n)
                     WHERE n.matter_id = $matter_id AND NOT n:Matter
                     DETACH DELETE n",
                )
                .param("matter_id", matter_id),
            )
            .await?;
        self.neo4j
            .run_rows(
                query("MATCH (m:Matter {matter_id: $matter_id}) DETACH DELETE m")
                    .param("matter_id", matter_id),
            )
            .await?;
        if !object_blob_ids.is_empty() {
            self.neo4j
                .run_rows(
                    query(
                        "UNWIND $object_blob_ids AS object_blob_id
                         MATCH (b:ObjectBlob {object_blob_id: object_blob_id})
                         OPTIONAL MATCH (:Matter)-[:USES_OBJECT_BLOB]->(b)
                         WITH b, count(*) AS remaining_matter_refs
                         WHERE remaining_matter_refs = 0
                         DETACH DELETE b",
                    )
                    .param("object_blob_ids", object_blob_ids),
                )
                .await?;
        }

        for key in storage_keys {
            if let Err(error) = self.object_store.delete(&key).await {
                tracing::warn!(
                    matter_id,
                    storage_key = key,
                    "Failed to delete stored matter object: {}",
                    error
                );
            }
        }
        Ok(())
    }

    async fn matter_delete_targets(
        &self,
        matter_id: &str,
    ) -> ApiResult<(HashSet<String>, Vec<String>)> {
        let rows = self
            .neo4j
            .run_rows(
                query(
                    "MATCH (m:Matter {matter_id: $matter_id})
                     OPTIONAL MATCH (m)-[:USES_OBJECT_BLOB]->(b:ObjectBlob)
                     WITH b
                     WHERE b IS NOT NULL
                     OPTIONAL MATCH (other:Matter)-[:USES_OBJECT_BLOB]->(b)
                     WHERE other.matter_id <> $matter_id
                     WITH b, count(other) AS other_matter_refs
                     RETURN
                       [key IN collect(DISTINCT CASE
                          WHEN other_matter_refs = 0 THEN b.storage_key
                          ELSE NULL
                        END) WHERE key IS NOT NULL AND key <> ''] AS storage_keys,
                       collect(DISTINCT b.object_blob_id) AS object_blob_ids",
                )
                .param("matter_id", matter_id),
            )
            .await?;
        let storage_keys = rows
            .first()
            .and_then(|row| row.get::<Vec<String>>("storage_keys").ok())
            .unwrap_or_default()
            .into_iter()
            .collect();
        let object_blob_ids = rows
            .first()
            .and_then(|row| row.get::<Vec<String>>("object_blob_ids").ok())
            .unwrap_or_default();
        Ok((storage_keys, object_blob_ids))
    }

    pub async fn create_party(
        &self,
        matter_id: &str,
        request: CreatePartyRequest,
    ) -> ApiResult<CaseParty> {
        self.require_matter(matter_id).await?;
        let id = generate_id("party", &request.name);
        let party = CaseParty {
            id: id.clone(),
            party_id: id,
            matter_id: matter_id.to_string(),
            name: request.name,
            role: request.role.unwrap_or_else(|| "witness".to_string()),
            party_type: request
                .party_type
                .unwrap_or_else(|| "individual".to_string()),
            represented_by: request.represented_by,
            contact_email: request.contact_email,
            contact_phone: request.contact_phone,
            notes: request.notes,
        };
        self.merge_node(matter_id, party_spec(), &party.party_id, &party)
            .await
    }

    pub async fn list_parties(&self, matter_id: &str) -> ApiResult<Vec<CaseParty>> {
        self.list_nodes(matter_id, party_spec()).await
    }

    pub async fn create_fact(
        &self,
        matter_id: &str,
        request: CreateFactRequest,
    ) -> ApiResult<CaseFact> {
        self.require_matter(matter_id).await?;
        let mut source_spans = Vec::new();
        for source_span_id in request.source_span_ids.unwrap_or_default() {
            source_spans.push(
                self.get_node::<SourceSpan>(matter_id, source_span_spec(), &source_span_id)
                    .await
                    .map_err(|error| matter_reference_error(error, "source_span"))?,
            );
        }
        let id = generate_id("fact", &request.statement);
        let fact = CaseFact {
            id: id.clone(),
            fact_id: id,
            matter_id: matter_id.to_string(),
            statement: request.statement.clone(),
            text: request.statement,
            status: request.status.unwrap_or_else(|| "alleged".to_string()),
            confidence: request.confidence.unwrap_or(0.7),
            date: request.date,
            party_id: request.party_id,
            source_document_ids: request.source_document_ids.unwrap_or_default(),
            source_evidence_ids: request.source_evidence_ids.unwrap_or_default(),
            contradicted_by_evidence_ids: Vec::new(),
            supports_claim_ids: Vec::new(),
            supports_defense_ids: Vec::new(),
            used_in_draft_ids: Vec::new(),
            needs_verification: true,
            source_spans,
            markdown_ast_node_ids: request.markdown_ast_node_ids.unwrap_or_default(),
            notes: request.notes,
        };
        let fact = self
            .merge_node(matter_id, fact_spec(), &fact.fact_id, &fact)
            .await?;
        self.materialize_fact_edges(&fact).await?;
        Ok(fact)
    }

    pub async fn list_facts(&self, matter_id: &str) -> ApiResult<Vec<CaseFact>> {
        self.list_nodes(matter_id, fact_spec()).await
    }

    pub async fn patch_fact(
        &self,
        matter_id: &str,
        fact_id: &str,
        request: PatchFactRequest,
    ) -> ApiResult<CaseFact> {
        let mut fact = self
            .get_node::<CaseFact>(matter_id, fact_spec(), fact_id)
            .await?;
        if let Some(value) = request.statement {
            fact.statement = value.clone();
            fact.text = value;
        }
        if let Some(value) = request.status {
            fact.status = value;
        }
        if let Some(value) = request.confidence {
            fact.confidence = value;
        }
        if request.date.is_some() {
            fact.date = request.date;
        }
        if request.party_id.is_some() {
            fact.party_id = request.party_id;
        }
        if request.notes.is_some() {
            fact.notes = request.notes;
        }
        self.merge_node(matter_id, fact_spec(), fact_id, &fact)
            .await
    }

    pub async fn approve_fact(&self, matter_id: &str, fact_id: &str) -> ApiResult<CaseFact> {
        let mut fact = self
            .get_node::<CaseFact>(matter_id, fact_spec(), fact_id)
            .await?;
        fact.status = "supported".to_string();
        fact.confidence = fact.confidence.max(0.85);
        fact.needs_verification = false;
        self.merge_node(matter_id, fact_spec(), fact_id, &fact)
            .await
    }

    pub async fn create_timeline_event(
        &self,
        matter_id: &str,
        request: CreateTimelineEventRequest,
    ) -> ApiResult<CaseTimelineEvent> {
        self.require_matter(matter_id).await?;
        let id = generate_id("event", &request.title);
        let event = CaseTimelineEvent {
            id: id.clone(),
            event_id: id,
            matter_id: matter_id.to_string(),
            date: request.date,
            title: request.title,
            description: request.description,
            kind: request.kind.unwrap_or_else(|| "other".to_string()),
            category: "user".to_string(),
            status: "complete".to_string(),
            source_document_id: request.source_document_id,
            party_ids: request.party_ids.unwrap_or_default(),
            linked_fact_ids: request.linked_fact_ids.unwrap_or_default(),
            linked_claim_ids: request.linked_claim_ids.unwrap_or_default(),
            source_span_ids: request.source_span_ids.unwrap_or_default(),
            text_chunk_ids: request.text_chunk_ids.unwrap_or_default(),
            markdown_ast_node_ids: request.markdown_ast_node_ids.unwrap_or_default(),
            suggestion_id: request.suggestion_id,
            agent_run_id: request.agent_run_id,
            date_confidence: 1.0,
            disputed: false,
        };
        let event = self
            .merge_node(matter_id, timeline_spec(), &event.event_id, &event)
            .await?;
        self.materialize_timeline_event_edges(&event).await?;
        Ok(event)
    }

    pub async fn list_timeline(&self, matter_id: &str) -> ApiResult<Vec<CaseTimelineEvent>> {
        self.list_nodes(matter_id, timeline_spec()).await
    }

    pub async fn create_evidence(
        &self,
        matter_id: &str,
        request: CreateEvidenceRequest,
    ) -> ApiResult<CaseEvidence> {
        self.require_matter(matter_id).await?;
        let mut document = self.get_document(matter_id, &request.document_id).await?;
        let provenance = self
            .ensure_document_original_provenance(matter_id, &mut document)
            .await?;
        if provenance.is_some() {
            self.merge_node(matter_id, document_spec(), &document.document_id, &document)
                .await?;
        }
        let id = generate_id("evidence", &request.quote);
        let source_context = source_context_from_provenance(provenance.as_ref());
        let source_spans = vec![manual_evidence_source_span(
            matter_id,
            &request.document_id,
            &id,
            request.source_span.as_deref(),
            &request.quote,
            &source_context,
        )];
        let evidence = CaseEvidence {
            id: id.clone(),
            evidence_id: id,
            matter_id: matter_id.to_string(),
            document_id: request.document_id,
            source_span: request
                .source_span
                .unwrap_or_else(|| "document".to_string()),
            quote: request.quote,
            evidence_type: request
                .evidence_type
                .unwrap_or_else(|| "document_text".to_string()),
            strength: request.strength.unwrap_or_else(|| "moderate".to_string()),
            confidence: request.confidence.unwrap_or(0.75),
            exhibit_label: request.exhibit_label,
            supports_fact_ids: request.supports_fact_ids.unwrap_or_default(),
            contradicts_fact_ids: request.contradicts_fact_ids.unwrap_or_default(),
            source_spans,
        };
        for span in &evidence.source_spans {
            self.merge_source_span(matter_id, span).await?;
        }
        let evidence = self
            .merge_node(matter_id, evidence_spec(), &evidence.evidence_id, &evidence)
            .await?;
        for fact_id in &evidence.supports_fact_ids {
            self.sync_fact_evidence_link(matter_id, &evidence.evidence_id, fact_id, "supports")
                .await?;
            self.sync_claim_element_evidence(matter_id, &evidence.evidence_id, fact_id)
                .await?;
        }
        for fact_id in &evidence.contradicts_fact_ids {
            self.sync_fact_evidence_link(matter_id, &evidence.evidence_id, fact_id, "contradicts")
                .await?;
        }
        self.materialize_evidence_edges(&evidence).await?;
        Ok(evidence)
    }

    pub async fn list_evidence(&self, matter_id: &str) -> ApiResult<Vec<CaseEvidence>> {
        self.list_nodes(matter_id, evidence_spec()).await
    }

    pub async fn link_evidence_fact(
        &self,
        matter_id: &str,
        evidence_id: &str,
        request: LinkEvidenceFactRequest,
    ) -> ApiResult<CaseEvidence> {
        let mut evidence = self
            .get_node::<CaseEvidence>(matter_id, evidence_spec(), evidence_id)
            .await?;
        let relation = request.relation.unwrap_or_else(|| "supports".to_string());
        match relation.as_str() {
            "contradicts" => {
                push_unique(&mut evidence.contradicts_fact_ids, request.fact_id.clone())
            }
            _ => push_unique(&mut evidence.supports_fact_ids, request.fact_id.clone()),
        }
        let evidence = self
            .merge_node(matter_id, evidence_spec(), evidence_id, &evidence)
            .await?;
        self.sync_fact_evidence_link(
            matter_id,
            &evidence.evidence_id,
            &request.fact_id,
            &relation,
        )
        .await?;
        self.sync_claim_element_evidence(matter_id, &evidence.evidence_id, &request.fact_id)
            .await?;
        self.materialize_evidence_edges(&evidence).await?;
        Ok(evidence)
    }

    pub async fn create_claim(
        &self,
        matter_id: &str,
        request: CreateClaimRequest,
    ) -> ApiResult<CaseClaim> {
        self.require_matter(matter_id).await?;
        let id = generate_id("claim", &request.title);
        let elements = request
            .elements
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(index, element)| {
                let authority = element.authority;
                let authorities = authority
                    .as_ref()
                    .map(|value| AuthorityRef {
                        citation: value.clone(),
                        canonical_id: value.clone(),
                        reason: None,
                        pinpoint: None,
                    })
                    .into_iter()
                    .collect();
                CaseElement {
                    id: format!("{id}:element:{}", index + 1),
                    element_id: format!("{id}:element:{}", index + 1),
                    matter_id: matter_id.to_string(),
                    text: element.text,
                    authority,
                    authorities,
                    satisfied: false,
                    fact_ids: element.fact_ids.unwrap_or_default(),
                    evidence_ids: element.evidence_ids.unwrap_or_default(),
                    missing_facts: Vec::new(),
                }
            })
            .collect::<Vec<_>>();
        let claim = CaseClaim {
            id: id.clone(),
            claim_id: id,
            matter_id: matter_id.to_string(),
            kind: request.kind.unwrap_or_else(|| "claim".to_string()),
            title: request.title.clone(),
            name: request.title,
            claim_type: request.claim_type.unwrap_or_else(|| "custom".to_string()),
            legal_theory: request.legal_theory.unwrap_or_default(),
            status: request.status.unwrap_or_else(|| "candidate".to_string()),
            risk_level: request.risk_level.unwrap_or_else(|| "medium".to_string()),
            fact_ids: request.fact_ids.unwrap_or_default(),
            evidence_ids: request.evidence_ids.unwrap_or_default(),
            authorities: request.authorities.unwrap_or_default(),
            elements,
        };
        let claim = self
            .merge_node(matter_id, claim_spec(), &claim.claim_id, &claim)
            .await?;
        self.materialize_claim_edges(&claim).await?;
        Ok(claim)
    }

    pub async fn list_claims(&self, matter_id: &str) -> ApiResult<Vec<CaseClaim>> {
        self.list_nodes(matter_id, claim_spec()).await
    }

    pub async fn map_claim_elements(
        &self,
        matter_id: &str,
        claim_id: &str,
    ) -> ApiResult<CaseClaim> {
        let mut claim = self
            .get_node::<CaseClaim>(matter_id, claim_spec(), claim_id)
            .await?;
        for element in &mut claim.elements {
            element.satisfied = !element.fact_ids.is_empty();
            if element.satisfied {
                element.missing_facts.clear();
            } else if element.missing_facts.is_empty() {
                element
                    .missing_facts
                    .push("No reviewed fact has been linked to this element.".to_string());
            }
        }
        let claim = self
            .merge_node(matter_id, claim_spec(), claim_id, &claim)
            .await?;
        self.materialize_claim_edges(&claim).await?;
        Ok(claim)
    }

    pub async fn create_defense(
        &self,
        matter_id: &str,
        request: CreateDefenseRequest,
    ) -> ApiResult<CaseDefense> {
        self.require_matter(matter_id).await?;
        let id = generate_id("defense", &request.name);
        let defense = CaseDefense {
            id: id.clone(),
            defense_id: id,
            matter_id: matter_id.to_string(),
            name: request.name,
            basis: request.basis.unwrap_or_default(),
            status: request.status.unwrap_or_else(|| "candidate".to_string()),
            applies_to_claim_ids: request.applies_to_claim_ids.unwrap_or_default(),
            required_facts: request.required_facts.unwrap_or_default(),
            fact_ids: request.fact_ids.unwrap_or_default(),
            evidence_ids: request.evidence_ids.unwrap_or_default(),
            authorities: request.authorities.unwrap_or_default(),
            viability: request.viability.unwrap_or_else(|| "medium".to_string()),
        };
        self.merge_node(matter_id, defense_spec(), &defense.defense_id, &defense)
            .await
    }

    pub async fn list_defenses(&self, matter_id: &str) -> ApiResult<Vec<CaseDefense>> {
        self.list_nodes(matter_id, defense_spec()).await
    }

    pub async fn list_deadlines(&self, matter_id: &str) -> ApiResult<Vec<CaseDeadline>> {
        self.list_nodes(matter_id, deadline_spec()).await
    }

    pub async fn create_deadline(
        &self,
        matter_id: &str,
        request: CreateDeadlineRequest,
    ) -> ApiResult<CaseDeadline> {
        self.require_matter(matter_id).await?;
        let deadline_id = generate_id("deadline", &request.title);
        let due_date = request.due_date;
        let deadline = CaseDeadline {
            id: deadline_id.clone(),
            deadline_id: deadline_id.clone(),
            matter_id: matter_id.to_string(),
            title: request.title,
            description: request.description.unwrap_or_default(),
            category: request.category.unwrap_or_else(|| "case".to_string()),
            kind: request.kind.unwrap_or_else(|| "manual".to_string()),
            days_remaining: days_until(&due_date),
            due_date,
            severity: request.severity.unwrap_or_else(|| "info".to_string()),
            source: request.source.unwrap_or_else(|| "manual".to_string()),
            source_citation: request.source_citation,
            source_canonical_id: request.source_canonical_id,
            triggered_by_event_id: request.triggered_by_event_id,
            status: request.status.unwrap_or_else(|| "open".to_string()),
            notes: request.notes,
        };
        self.merge_node(matter_id, deadline_spec(), &deadline_id, &deadline)
            .await
    }

    pub async fn patch_deadline(
        &self,
        matter_id: &str,
        deadline_id: &str,
        request: PatchDeadlineRequest,
    ) -> ApiResult<CaseDeadline> {
        let mut deadline: CaseDeadline = self
            .get_node(matter_id, deadline_spec(), deadline_id)
            .await?;
        if let Some(value) = request.title {
            deadline.title = value;
        }
        if let Some(value) = request.due_date {
            deadline.due_date = value;
            deadline.days_remaining = days_until(&deadline.due_date);
        }
        if let Some(value) = request.description {
            deadline.description = value;
        }
        if let Some(value) = request.category {
            deadline.category = value;
        }
        if let Some(value) = request.kind {
            deadline.kind = value;
        }
        if let Some(value) = request.severity {
            deadline.severity = value;
        }
        if let Some(value) = request.source {
            deadline.source = value;
        }
        if let Some(value) = request.source_citation {
            deadline.source_citation = value;
        }
        if let Some(value) = request.source_canonical_id {
            deadline.source_canonical_id = value;
        }
        if let Some(value) = request.triggered_by_event_id {
            deadline.triggered_by_event_id = value;
        }
        if let Some(value) = request.status {
            deadline.status = value;
        }
        if let Some(value) = request.notes {
            deadline.notes = value;
        }
        self.merge_node(matter_id, deadline_spec(), deadline_id, &deadline)
            .await
    }

    pub async fn compute_deadlines(&self, matter_id: &str) -> ApiResult<ComputeDeadlinesResponse> {
        self.require_matter(matter_id).await?;
        Ok(ComputeDeadlinesResponse {
            generated: Vec::new(),
            warnings: vec![
                "No deterministic deadline rule matched this matter. Add known service, filing, or order dates to compute deadlines.".to_string(),
            ],
        })
    }

    pub async fn list_tasks(&self, matter_id: &str) -> ApiResult<Vec<CaseTask>> {
        self.list_nodes(matter_id, task_spec()).await
    }

    pub async fn create_task(
        &self,
        matter_id: &str,
        request: CreateTaskRequest,
    ) -> ApiResult<CaseTask> {
        self.require_matter(matter_id).await?;
        let task_id = generate_id("task", &request.title);
        let task = CaseTask {
            id: task_id.clone(),
            task_id: task_id.clone(),
            matter_id: matter_id.to_string(),
            title: request.title,
            status: request.status.unwrap_or_else(|| "todo".to_string()),
            priority: request.priority.unwrap_or_else(|| "med".to_string()),
            due_date: request.due_date,
            assigned_to: request.assigned_to,
            related_claim_ids: request.related_claim_ids.unwrap_or_default(),
            related_document_ids: request.related_document_ids.unwrap_or_default(),
            related_deadline_id: request.related_deadline_id,
            source: request.source.unwrap_or_else(|| "manual".to_string()),
            description: request.description,
        };
        self.merge_node(matter_id, task_spec(), &task_id, &task)
            .await
    }

    pub async fn patch_task(
        &self,
        matter_id: &str,
        task_id: &str,
        request: PatchTaskRequest,
    ) -> ApiResult<CaseTask> {
        let mut task: CaseTask = self.get_node(matter_id, task_spec(), task_id).await?;
        if let Some(value) = request.title {
            task.title = value;
        }
        if let Some(value) = request.status {
            task.status = value;
        }
        if let Some(value) = request.priority {
            task.priority = value;
        }
        if let Some(value) = request.due_date {
            task.due_date = value;
        }
        if let Some(value) = request.assigned_to {
            task.assigned_to = value;
        }
        if let Some(value) = request.related_claim_ids {
            task.related_claim_ids = value;
        }
        if let Some(value) = request.related_document_ids {
            task.related_document_ids = value;
        }
        if let Some(value) = request.related_deadline_id {
            task.related_deadline_id = value;
        }
        if let Some(value) = request.source {
            task.source = value;
        }
        if let Some(value) = request.description {
            task.description = value;
        }
        self.merge_node(matter_id, task_spec(), task_id, &task)
            .await
    }
}

struct NormalizedCreateMatterRequest {
    name: String,
    matter_type: String,
    user_role: String,
    jurisdiction: String,
    court: String,
    case_number: Option<String>,
    settings: Option<PatchCaseBuilderMatterSettingsRequest>,
}

struct NormalizedPatchMatterRequest {
    name: Option<String>,
    matter_type: Option<String>,
    status: Option<String>,
    user_role: Option<String>,
    jurisdiction: Option<String>,
    court: Option<String>,
    case_number: Option<Option<String>>,
}

fn normalize_create_matter_request(
    request: CreateMatterRequest,
) -> ApiResult<NormalizedCreateMatterRequest> {
    Ok(NormalizedCreateMatterRequest {
        name: required_trimmed(request.name, "name")?,
        matter_type: optional_choice(
            request.matter_type,
            "matter_type",
            "civil",
            ALLOWED_MATTER_TYPES,
        )?,
        user_role: optional_choice(
            request.user_role,
            "user_role",
            "neutral",
            ALLOWED_USER_ROLES,
        )?,
        jurisdiction: optional_text_or_default(request.jurisdiction, "Oregon"),
        court: optional_text_or_default(request.court, "Unassigned"),
        case_number: optional_trimmed(request.case_number),
        settings: request.settings,
    })
}

fn normalize_patch_matter_request(
    request: PatchMatterRequest,
) -> ApiResult<NormalizedPatchMatterRequest> {
    Ok(NormalizedPatchMatterRequest {
        name: match request.name {
            Some(value) => Some(required_trimmed(value, "name")?),
            None => None,
        },
        matter_type: optional_choice_patch(
            request.matter_type,
            "matter_type",
            ALLOWED_MATTER_TYPES,
        )?,
        status: optional_choice_patch(request.status, "status", ALLOWED_MATTER_STATUSES)?,
        user_role: optional_choice_patch(request.user_role, "user_role", ALLOWED_USER_ROLES)?,
        jurisdiction: request
            .jurisdiction
            .map(|value| optional_text_or_default(Some(value), "Oregon")),
        court: request
            .court
            .map(|value| optional_text_or_default(Some(value), "Unassigned")),
        case_number: request
            .case_number
            .map(|value| optional_trimmed(Some(value))),
    })
}

const ALLOWED_MATTER_TYPES: &[&str] = &[
    "civil",
    "family",
    "small_claims",
    "admin",
    "criminal",
    "appeal",
    "landlord_tenant",
    "employment",
    "fact_check",
    "complaint_analysis",
    "other",
];

const ALLOWED_USER_ROLES: &[&str] = &[
    "plaintiff",
    "defendant",
    "petitioner",
    "respondent",
    "neutral",
    "researcher",
];

const ALLOWED_MATTER_STATUSES: &[&str] = &["active", "intake", "stayed", "closed", "appeal"];

fn required_trimmed(value: String, field: &str) -> ApiResult<String> {
    optional_trimmed(Some(value))
        .ok_or_else(|| ApiError::BadRequest(format!("Matter {field} must not be empty")))
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn optional_text_or_default(value: Option<String>, default: &str) -> String {
    optional_trimmed(value).unwrap_or_else(|| default.to_string())
}

fn optional_choice(
    value: Option<String>,
    field: &str,
    default: &str,
    allowed: &[&str],
) -> ApiResult<String> {
    optional_choice_patch(value, field, allowed)
        .map(|value| value.unwrap_or_else(|| default.to_string()))
}

fn optional_choice_patch(
    value: Option<String>,
    field: &str,
    allowed: &[&str],
) -> ApiResult<Option<String>> {
    let Some(value) = optional_trimmed(value) else {
        return Ok(None);
    };
    if allowed.contains(&value.as_str()) {
        Ok(Some(value))
    } else {
        Err(ApiError::BadRequest(format!(
            "Unsupported matter {field} {value}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn casebuilder_create_matter_validation_trims_and_defaults() {
        let normalized = normalize_create_matter_request(CreateMatterRequest {
            name: "  Smith v. ABC  ".to_string(),
            matter_type: Some("landlord_tenant".to_string()),
            user_role: Some(" plaintiff ".to_string()),
            jurisdiction: Some("  ".to_string()),
            court: Some("  Multnomah County Circuit Court  ".to_string()),
            case_number: Some("  24CV12345  ".to_string()),
            settings: None,
        })
        .expect("valid matter input");

        assert_eq!(normalized.name, "Smith v. ABC");
        assert_eq!(normalized.matter_type, "landlord_tenant");
        assert_eq!(normalized.user_role, "plaintiff");
        assert_eq!(normalized.jurisdiction, "Oregon");
        assert_eq!(normalized.court, "Multnomah County Circuit Court");
        assert_eq!(normalized.case_number.as_deref(), Some("24CV12345"));
    }

    #[test]
    fn casebuilder_create_matter_validation_rejects_empty_or_unknown_values() {
        assert!(
            normalize_create_matter_request(CreateMatterRequest {
                name: "  ".to_string(),
                matter_type: None,
                user_role: None,
                jurisdiction: None,
                court: None,
                case_number: None,
                settings: None,
            })
            .is_err()
        );
        assert!(
            normalize_create_matter_request(CreateMatterRequest {
                name: "Smith".to_string(),
                matter_type: Some("space_law".to_string()),
                user_role: None,
                jurisdiction: None,
                court: None,
                case_number: None,
                settings: None,
            })
            .is_err()
        );
        assert!(
            normalize_create_matter_request(CreateMatterRequest {
                name: "Smith".to_string(),
                matter_type: None,
                user_role: Some("spectator".to_string()),
                jurisdiction: None,
                court: None,
                case_number: None,
                settings: None,
            })
            .is_err()
        );
    }

    #[test]
    fn casebuilder_patch_matter_validation_trims_and_rejects_invalid_choices() {
        let normalized = normalize_patch_matter_request(PatchMatterRequest {
            name: Some("  Updated Matter  ".to_string()),
            matter_type: Some("civil".to_string()),
            status: Some("active".to_string()),
            user_role: Some("researcher".to_string()),
            jurisdiction: Some(" ".to_string()),
            court: Some(" Court  ".to_string()),
            case_number: Some(" ".to_string()),
        })
        .expect("valid patch");

        assert_eq!(normalized.name.as_deref(), Some("Updated Matter"));
        assert_eq!(normalized.matter_type.as_deref(), Some("civil"));
        assert_eq!(normalized.status.as_deref(), Some("active"));
        assert_eq!(normalized.user_role.as_deref(), Some("researcher"));
        assert_eq!(normalized.jurisdiction.as_deref(), Some("Oregon"));
        assert_eq!(normalized.court.as_deref(), Some("Court"));
        assert_eq!(normalized.case_number, Some(None));

        assert!(
            normalize_patch_matter_request(PatchMatterRequest {
                name: None,
                matter_type: None,
                status: Some("archived".to_string()),
                user_role: None,
                jurisdiction: None,
                court: None,
                case_number: None,
            })
            .is_err()
        );
    }
}
