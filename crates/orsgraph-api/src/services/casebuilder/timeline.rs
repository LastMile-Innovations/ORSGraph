use super::*;

impl CaseBuilderService {
    pub async fn list_timeline_suggestions(
        &self,
        matter_id: &str,
    ) -> ApiResult<Vec<TimelineSuggestion>> {
        self.list_nodes(matter_id, timeline_suggestion_spec()).await
    }

    pub async fn suggest_timeline(
        &self,
        matter_id: &str,
        request: TimelineSuggestRequest,
    ) -> ApiResult<TimelineSuggestResponse> {
        self.require_matter(matter_id).await?;
        let limit = request.limit.unwrap_or(100).clamp(1, 500) as usize;
        let mut warnings = vec![
            "Timeline AI agent is in provider-free template mode; deterministic suggestions only."
                .to_string(),
        ];
        let (subject_type, subject_id) = timeline_subject_for_request(&request);
        let mut agent_run = TimelineAgentRun {
            agent_run_id: generate_id(
                "timeline-agent-run",
                &format!(
                    "{}:{}:{}",
                    matter_id,
                    subject_type,
                    subject_id.clone().unwrap_or_default()
                ),
            ),
            id: String::new(),
            matter_id: matter_id.to_string(),
            subject_type,
            subject_id,
            mode: request.mode.unwrap_or_else(|| "template".to_string()),
            provider_mode: "template".to_string(),
            status: "recorded".to_string(),
            message: "Provider-free timeline agent recorded deterministic suggestions; no unsupported text was inserted.".to_string(),
            produced_suggestion_ids: Vec::new(),
            warnings: warnings.clone(),
            created_at: now_string(),
        };
        agent_run.id = agent_run.agent_run_id.clone();

        let mut suggestions = Vec::new();
        if let Some(work_product_id) = request.work_product_id.as_deref() {
            let product = self.get_work_product(matter_id, work_product_id).await?;
            suggestions.extend(timeline_suggestions_for_work_product(
                matter_id,
                &product,
                request.block_id.as_deref(),
                Some(agent_run.agent_run_id.as_str()),
                limit,
            ));
        } else if let Some(source_span_ids) = request.source_span_ids.as_ref() {
            suggestions.extend(
                self.timeline_suggestions_for_source_spans(
                    matter_id,
                    source_span_ids,
                    Some(agent_run.agent_run_id.as_str()),
                    limit,
                )
                .await?,
            );
        } else if let Some(document_ids) = request.document_ids.as_ref() {
            suggestions.extend(
                self.timeline_suggestions_for_documents(
                    matter_id,
                    document_ids,
                    Some(agent_run.agent_run_id.as_str()),
                    limit,
                )
                .await?,
            );
        } else {
            let facts = self.list_facts(matter_id).await?;
            suggestions.extend(timeline_suggestions_from_facts(
                matter_id,
                None,
                &facts,
                &[],
                "matter_graph",
                None,
                None,
                Some(agent_run.agent_run_id.as_str()),
                None,
                limit,
            ));
        }

        suggestions.truncate(limit);
        let stored = self
            .store_timeline_suggestions_preserving_review(matter_id, suggestions)
            .await?;
        agent_run.produced_suggestion_ids = stored
            .iter()
            .map(|item| item.suggestion_id.clone())
            .collect();
        if stored.is_empty() {
            warnings.push("No dated source-backed statements were found.".to_string());
            agent_run.warnings = warnings.clone();
        }
        let agent_run = self.merge_timeline_agent_run(matter_id, &agent_run).await?;
        Ok(TimelineSuggestResponse {
            enabled: false,
            mode: "template".to_string(),
            message: agent_run.message.clone(),
            suggestions: stored,
            agent_run: Some(agent_run),
            warnings,
        })
    }

    pub async fn patch_timeline_suggestion(
        &self,
        matter_id: &str,
        suggestion_id: &str,
        request: PatchTimelineSuggestionRequest,
    ) -> ApiResult<TimelineSuggestion> {
        let mut suggestion = self
            .get_node::<TimelineSuggestion>(matter_id, timeline_suggestion_spec(), suggestion_id)
            .await?;
        if let Some(value) = request.date {
            suggestion.date = value;
        }
        if let Some(value) = request.date_text {
            suggestion.date_text = value;
        }
        if let Some(value) = request.date_confidence {
            suggestion.date_confidence = value.clamp(0.0, 1.0);
        }
        if let Some(value) = request.title {
            suggestion.title = value;
        }
        if request.description.is_some() {
            suggestion.description = request.description;
        }
        if let Some(value) = request.kind {
            suggestion.kind = value;
        }
        if request.source_document_id.is_some() {
            suggestion.source_document_id = request.source_document_id;
        }
        if let Some(value) = request.source_span_ids {
            suggestion.source_span_ids = value;
        }
        if let Some(value) = request.text_chunk_ids {
            suggestion.text_chunk_ids = value;
        }
        if let Some(value) = request.linked_fact_ids {
            suggestion.linked_fact_ids = value;
        }
        if let Some(value) = request.linked_claim_ids {
            suggestion.linked_claim_ids = value;
        }
        if let Some(value) = request.status {
            suggestion.status = value;
        }
        if let Some(value) = request.warnings {
            suggestion.warnings = value;
        }
        suggestion.updated_at = now_string();
        self.merge_timeline_suggestion(matter_id, &suggestion).await
    }

    pub async fn approve_timeline_suggestion(
        &self,
        matter_id: &str,
        suggestion_id: &str,
    ) -> ApiResult<TimelineSuggestionApprovalResponse> {
        let mut suggestion = self
            .get_node::<TimelineSuggestion>(matter_id, timeline_suggestion_spec(), suggestion_id)
            .await?;
        let event_id = suggestion
            .approved_event_id
            .clone()
            .unwrap_or_else(|| timeline_event_id_from_suggestion(&suggestion.suggestion_id));
        let event = CaseTimelineEvent {
            id: event_id.clone(),
            event_id: event_id.clone(),
            matter_id: matter_id.to_string(),
            date: suggestion.date.clone(),
            title: suggestion.title.clone(),
            description: suggestion.description.clone(),
            kind: suggestion.kind.clone(),
            category: "suggestion".to_string(),
            status: "complete".to_string(),
            source_document_id: suggestion.source_document_id.clone(),
            party_ids: Vec::new(),
            linked_fact_ids: suggestion.linked_fact_ids.clone(),
            linked_claim_ids: suggestion.linked_claim_ids.clone(),
            source_span_ids: suggestion.source_span_ids.clone(),
            text_chunk_ids: suggestion.text_chunk_ids.clone(),
            suggestion_id: Some(suggestion.suggestion_id.clone()),
            agent_run_id: suggestion.agent_run_id.clone(),
            date_confidence: suggestion.date_confidence,
            disputed: suggestion
                .warnings
                .iter()
                .any(|warning| warning.contains("review")),
        };
        let event = self
            .merge_node(matter_id, timeline_spec(), &event.event_id, &event)
            .await?;
        self.materialize_timeline_event_edges(&event).await?;
        suggestion.status = "approved".to_string();
        suggestion.approved_event_id = Some(event.event_id.clone());
        suggestion.updated_at = now_string();
        let suggestion = self
            .merge_timeline_suggestion(matter_id, &suggestion)
            .await?;
        Ok(TimelineSuggestionApprovalResponse { suggestion, event })
    }

    async fn timeline_suggestions_for_documents(
        &self,
        matter_id: &str,
        document_ids: &[String],
        agent_run_id: Option<&str>,
        limit: usize,
    ) -> ApiResult<Vec<TimelineSuggestion>> {
        let facts = self.list_facts(matter_id).await?;
        let mut suggestions = Vec::new();
        for document_id in document_ids {
            if suggestions.len() >= limit {
                break;
            }
            let document = self.get_document(matter_id, document_id).await?;
            let document_facts = facts
                .iter()
                .filter(|fact| {
                    fact.source_document_ids
                        .iter()
                        .any(|source_id| source_id == document_id)
                })
                .cloned()
                .collect::<Vec<_>>();
            suggestions.extend(timeline_suggestions_from_facts(
                matter_id,
                Some(document_id),
                &document_facts,
                &[],
                "document",
                None,
                None,
                agent_run_id,
                None,
                limit.saturating_sub(suggestions.len()),
            ));
            if suggestions.len() < limit {
                if let Some(text) = document.extracted_text.as_deref() {
                    suggestions.extend(timeline_suggestions_from_text(
                        matter_id,
                        text,
                        "document",
                        Some(document_id),
                        document
                            .source_spans
                            .iter()
                            .map(|span| span.source_span_id.clone())
                            .collect(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                        agent_run_id,
                        None,
                        limit.saturating_sub(suggestions.len()),
                    ));
                }
            }
        }
        Ok(suggestions)
    }

    async fn timeline_suggestions_for_source_spans(
        &self,
        matter_id: &str,
        source_span_ids: &[String],
        agent_run_id: Option<&str>,
        limit: usize,
    ) -> ApiResult<Vec<TimelineSuggestion>> {
        let spans = self
            .list_nodes::<SourceSpan>(matter_id, source_span_spec())
            .await?;
        let mut suggestions = Vec::new();
        for source_span_id in source_span_ids {
            if suggestions.len() >= limit {
                break;
            }
            let span = spans
                .iter()
                .find(|candidate| candidate.source_span_id == *source_span_id)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("Source span {source_span_id} was not found"))
                })?;
            let Some(quote) = span.quote.as_deref() else {
                continue;
            };
            suggestions.extend(timeline_suggestions_from_text(
                matter_id,
                quote,
                "source_span",
                Some(span.document_id.as_str()),
                vec![span.source_span_id.clone()],
                span.chunk_id.iter().cloned().collect(),
                Vec::new(),
                Vec::new(),
                None,
                None,
                agent_run_id,
                None,
                limit.saturating_sub(suggestions.len()),
            ));
        }
        Ok(suggestions)
    }

    async fn store_timeline_suggestions_preserving_review(
        &self,
        matter_id: &str,
        suggestions: Vec<TimelineSuggestion>,
    ) -> ApiResult<Vec<TimelineSuggestion>> {
        let existing = self
            .list_nodes::<TimelineSuggestion>(matter_id, timeline_suggestion_spec())
            .await
            .unwrap_or_default();
        let mut stored = Vec::with_capacity(suggestions.len());
        for mut suggestion in suggestions {
            if let Some(current) = existing
                .iter()
                .find(|item| item.suggestion_id == suggestion.suggestion_id)
            {
                suggestion.status = current.status.clone();
                suggestion.approved_event_id = current.approved_event_id.clone();
                suggestion.created_at = current.created_at.clone();
                if current.status == "approved" || current.status == "rejected" {
                    suggestion.updated_at = current.updated_at.clone();
                }
            }
            stored.push(
                self.merge_timeline_suggestion(matter_id, &suggestion)
                    .await?,
            );
        }
        Ok(stored)
    }
}

fn timeline_subject_for_request(request: &TimelineSuggestRequest) -> (String, Option<String>) {
    if let Some(work_product_id) = &request.work_product_id {
        ("work_product".to_string(), Some(work_product_id.clone()))
    } else if let Some(document_ids) = &request.document_ids {
        ("document".to_string(), document_ids.first().cloned())
    } else if let Some(source_span_ids) = &request.source_span_ids {
        ("source_span".to_string(), source_span_ids.first().cloned())
    } else {
        ("matter".to_string(), None)
    }
}

fn timeline_suggestions_for_work_product(
    matter_id: &str,
    product: &WorkProduct,
    block_id: Option<&str>,
    agent_run_id: Option<&str>,
    limit: usize,
) -> Vec<TimelineSuggestion> {
    let mut suggestions = Vec::new();
    for block in &product.blocks {
        if suggestions.len() >= limit {
            break;
        }
        if block_id.is_some() && block_id != Some(block.block_id.as_str()) {
            continue;
        }
        let source_span_ids = product
            .document_ast
            .links
            .iter()
            .filter(|link| {
                link.source_block_id == block.block_id
                    && matches!(link.target_type.as_str(), "source_span" | "text_span")
            })
            .map(|link| link.target_id.clone())
            .chain(
                block
                    .evidence_ids
                    .iter()
                    .filter(|id| id.starts_with("source-span:"))
                    .cloned(),
            )
            .collect::<Vec<_>>();
        suggestions.extend(timeline_suggestions_from_text(
            matter_id,
            &block.text,
            "work_product_ast",
            None,
            source_span_ids,
            Vec::new(),
            block.fact_ids.clone(),
            Vec::new(),
            Some(product.work_product_id.as_str()),
            Some(block.block_id.as_str()),
            agent_run_id,
            None,
            limit.saturating_sub(suggestions.len()),
        ));
    }
    suggestions
}
