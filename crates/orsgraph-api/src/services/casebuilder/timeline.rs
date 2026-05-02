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
        let outcome = self
            .execute_timeline_agent_for_request(matter_id, request)
            .await?;
        let outcome = self
            .persist_timeline_agent_outcome(matter_id, &outcome)
            .await?;
        Ok(TimelineSuggestResponse {
            enabled: outcome.run.provider_mode == "live",
            mode: outcome.run.mode.clone(),
            message: outcome.run.message.clone(),
            suggestions: outcome.suggestions,
            warnings: outcome.run.warnings.clone(),
            agent_run: Some(outcome.run),
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
}
