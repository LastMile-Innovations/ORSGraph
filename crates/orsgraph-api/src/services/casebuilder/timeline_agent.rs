use super::*;
use async_trait::async_trait;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

const TIMELINE_AGENT_TYPE: &str = "timeline_builder";
const TIMELINE_EXTRACTOR_VERSION: &str = "casebuilder-timeline-deterministic-v1";
const TIMELINE_PROMPT_TEMPLATE_ID: &str = "timeline-enrichment-v1";
const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";

#[derive(Clone)]
pub struct TimelineAgentProviderConfig {
    pub provider: String,
    pub model: Option<String>,
    pub openai_api_key: Option<String>,
    pub timeout_ms: u64,
    pub max_input_chars: usize,
    pub harness_version: String,
}

#[derive(Clone, Serialize)]
struct TimelineAgentInput {
    matter_id: String,
    scope_type: String,
    scope_ids: Vec<String>,
    subject_type: String,
    subject_id: Option<String>,
    mode: String,
    fact_inputs: Vec<TimelineAgentFactInput>,
    text_inputs: Vec<TimelineAgentTextInput>,
}

#[derive(Clone, Serialize)]
struct TimelineAgentFactInput {
    source_type: String,
    source_document_id: Option<String>,
    work_product_id: Option<String>,
    block_id: Option<String>,
    index_run_id: Option<String>,
    facts: Vec<CaseFact>,
    chunks: Vec<ExtractedTextChunk>,
    limit: usize,
}

#[derive(Clone, Serialize)]
struct TimelineAgentTextInput {
    text: String,
    source_type: String,
    source_document_id: Option<String>,
    source_span_ids: Vec<String>,
    text_chunk_ids: Vec<String>,
    linked_fact_ids: Vec<String>,
    linked_claim_ids: Vec<String>,
    work_product_id: Option<String>,
    block_id: Option<String>,
    index_run_id: Option<String>,
    limit: usize,
}

pub(super) struct TimelineAgentOutcome {
    pub run: TimelineAgentRun,
    pub suggestions: Vec<TimelineSuggestion>,
}

#[derive(Clone, Serialize)]
struct TimelineProviderRequest {
    matter_id: String,
    agent_run_id: String,
    input_hash: String,
    candidates: Vec<TimelineProviderCandidate>,
}

#[derive(Clone, Serialize)]
struct TimelineProviderCandidate {
    candidate_id: String,
    date: String,
    date_text: String,
    title: String,
    kind: String,
    quote: String,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TimelineProviderResponse {
    #[serde(default)]
    enrichments: Vec<TimelineProviderEnrichment>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TimelineProviderEnrichment {
    candidate_id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    cluster_id: Option<String>,
    #[serde(default)]
    duplicate_of_candidate_id: Option<String>,
    #[serde(default)]
    explanation: Option<String>,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug)]
struct TimelineProviderError {
    code: String,
    message: String,
}

#[async_trait]
trait TimelineEnrichmentProvider {
    fn provider_name(&self) -> &'static str;
    fn model(&self) -> Option<String>;
    fn provider_mode(&self) -> &'static str;
    fn disabled_warning(&self) -> Option<String> {
        None
    }
    async fn enrich(
        &self,
        request: TimelineProviderRequest,
    ) -> Result<TimelineProviderResponse, TimelineProviderError>;
}

struct DisabledTimelineProvider {
    warning: String,
}

#[async_trait]
impl TimelineEnrichmentProvider for DisabledTimelineProvider {
    fn provider_name(&self) -> &'static str {
        "disabled"
    }

    fn model(&self) -> Option<String> {
        None
    }

    fn provider_mode(&self) -> &'static str {
        "template"
    }

    fn disabled_warning(&self) -> Option<String> {
        Some(self.warning.clone())
    }

    async fn enrich(
        &self,
        _request: TimelineProviderRequest,
    ) -> Result<TimelineProviderResponse, TimelineProviderError> {
        Ok(TimelineProviderResponse {
            enrichments: Vec::new(),
            warnings: vec![self.warning.clone()],
        })
    }
}

struct OpenAiTimelineProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    timeout_ms: u64,
}

#[async_trait]
impl TimelineEnrichmentProvider for OpenAiTimelineProvider {
    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn model(&self) -> Option<String> {
        Some(self.model.clone())
    }

    fn provider_mode(&self) -> &'static str {
        "live"
    }

    async fn enrich(
        &self,
        request: TimelineProviderRequest,
    ) -> Result<TimelineProviderResponse, TimelineProviderError> {
        let body = openai_timeline_request_body(&self.model, &request);
        let response = self
            .client
            .post(OPENAI_RESPONSES_URL)
            .bearer_auth(&self.api_key)
            .timeout(Duration::from_millis(self.timeout_ms))
            .json(&body)
            .send()
            .await
            .map_err(|error| TimelineProviderError {
                code: "provider_http_error".to_string(),
                message: error.to_string(),
            })?;
        let status = response.status();
        let payload = response
            .json::<serde_json::Value>()
            .await
            .map_err(|error| TimelineProviderError {
                code: "provider_response_parse_error".to_string(),
                message: error.to_string(),
            })?;
        if !status.is_success() {
            return Err(TimelineProviderError {
                code: "provider_http_status".to_string(),
                message: format!("OpenAI timeline enrichment failed with HTTP {status}."),
            });
        }
        parse_openai_timeline_response(&payload)
    }
}

#[cfg(test)]
struct FakeTimelineProvider {
    response: Result<TimelineProviderResponse, TimelineProviderError>,
}

#[cfg(test)]
#[async_trait]
impl TimelineEnrichmentProvider for FakeTimelineProvider {
    fn provider_name(&self) -> &'static str {
        "fake"
    }

    fn model(&self) -> Option<String> {
        Some("fake-timeline-model".to_string())
    }

    fn provider_mode(&self) -> &'static str {
        "live"
    }

    async fn enrich(
        &self,
        _request: TimelineProviderRequest,
    ) -> Result<TimelineProviderResponse, TimelineProviderError> {
        self.response.clone()
    }
}

impl Clone for TimelineProviderError {
    fn clone(&self) -> Self {
        Self {
            code: self.code.clone(),
            message: self.message.clone(),
        }
    }
}

impl CaseBuilderService {
    pub async fn list_timeline_agent_runs(
        &self,
        matter_id: &str,
    ) -> ApiResult<Vec<TimelineAgentRun>> {
        self.list_nodes(matter_id, timeline_agent_run_spec()).await
    }

    pub async fn get_timeline_agent_run(
        &self,
        matter_id: &str,
        agent_run_id: &str,
    ) -> ApiResult<TimelineAgentRun> {
        self.get_node(matter_id, timeline_agent_run_spec(), agent_run_id)
            .await
    }

    pub(super) async fn execute_timeline_agent_for_request(
        &self,
        matter_id: &str,
        request: TimelineSuggestRequest,
    ) -> ApiResult<TimelineAgentOutcome> {
        self.require_matter(matter_id).await?;
        let input = self
            .timeline_agent_input_for_request(matter_id, request)
            .await?;
        self.execute_timeline_agent(input).await
    }

    pub(super) async fn execute_timeline_agent_for_indexed_document(
        &self,
        matter_id: &str,
        document_id: &str,
        facts: &[CaseFact],
        chunks: &[ExtractedTextChunk],
        index_run_id: &str,
        limit: usize,
    ) -> ApiResult<TimelineAgentOutcome> {
        let input = TimelineAgentInput {
            matter_id: matter_id.to_string(),
            scope_type: "document_index".to_string(),
            scope_ids: vec![document_id.to_string()],
            subject_type: "document_index".to_string(),
            subject_id: Some(document_id.to_string()),
            mode: "deterministic".to_string(),
            fact_inputs: vec![TimelineAgentFactInput {
                source_type: "document_index".to_string(),
                source_document_id: Some(document_id.to_string()),
                work_product_id: None,
                block_id: None,
                index_run_id: Some(index_run_id.to_string()),
                facts: facts.to_vec(),
                chunks: chunks.to_vec(),
                limit,
            }],
            text_inputs: Vec::new(),
        };
        self.execute_timeline_agent(input).await
    }

    pub(super) async fn persist_timeline_agent_outcome(
        &self,
        matter_id: &str,
        outcome: &TimelineAgentOutcome,
    ) -> ApiResult<TimelineAgentOutcome> {
        let run = self
            .merge_timeline_agent_run(matter_id, &outcome.run)
            .await?;
        let mut stored = Vec::with_capacity(outcome.suggestions.len());
        for suggestion in &outcome.suggestions {
            stored.push(
                self.merge_timeline_suggestion(matter_id, suggestion)
                    .await?,
            );
        }
        Ok(TimelineAgentOutcome {
            run,
            suggestions: stored,
        })
    }

    async fn timeline_agent_input_for_request(
        &self,
        matter_id: &str,
        request: TimelineSuggestRequest,
    ) -> ApiResult<TimelineAgentInput> {
        let limit = request.limit.unwrap_or(100).clamp(1, 500) as usize;
        let mode = request.mode.unwrap_or_else(|| "deterministic".to_string());
        if let Some(work_product_id) = request.work_product_id.as_deref() {
            let product = self.get_work_product(matter_id, work_product_id).await?;
            let blocks = flatten_work_product_blocks(&product.document_ast.blocks);
            let selected = if let Some(block_id) = request.block_id.as_deref() {
                if !blocks.iter().any(|block| block.block_id == block_id) {
                    return Err(ApiError::NotFound(format!(
                        "AST block {block_id} not found"
                    )));
                }
                vec![block_id.to_string()]
            } else {
                blocks.iter().map(|block| block.block_id.clone()).collect()
            };
            let available_source_span_ids = self
                .list_nodes::<SourceSpan>(matter_id, source_span_spec())
                .await?
                .into_iter()
                .map(|span| span.source_span_id)
                .collect::<HashSet<_>>();
            let mut text_inputs = Vec::new();
            for block in blocks
                .into_iter()
                .filter(|block| selected.contains(&block.block_id))
                .take(limit)
            {
                let source_span_ids = timeline_ast_source_span_ids(&product, &block);
                validate_timeline_ast_source_span_links(
                    &block.block_id,
                    &source_span_ids,
                    &available_source_span_ids,
                )?;
                text_inputs.push(TimelineAgentTextInput {
                    text: block.text,
                    source_type: "work_product_ast".to_string(),
                    source_document_id: None,
                    source_span_ids,
                    text_chunk_ids: Vec::new(),
                    linked_fact_ids: block.fact_ids,
                    linked_claim_ids: Vec::new(),
                    work_product_id: Some(product.work_product_id.clone()),
                    block_id: Some(block.block_id),
                    index_run_id: None,
                    limit,
                });
            }
            return Ok(TimelineAgentInput {
                matter_id: matter_id.to_string(),
                scope_type: "work_product".to_string(),
                scope_ids: selected,
                subject_type: "work_product".to_string(),
                subject_id: Some(work_product_id.to_string()),
                mode,
                fact_inputs: Vec::new(),
                text_inputs,
            });
        }

        if let Some(source_span_ids) = request.source_span_ids {
            let spans = self
                .list_nodes::<SourceSpan>(matter_id, source_span_spec())
                .await?;
            let mut text_inputs = Vec::new();
            for source_span_id in source_span_ids.iter().take(limit) {
                let span = spans
                    .iter()
                    .find(|candidate| candidate.source_span_id == *source_span_id)
                    .ok_or_else(|| {
                        ApiError::NotFound(format!("Source span {source_span_id} was not found"))
                    })?;
                let Some(quote) = span.quote.clone() else {
                    continue;
                };
                text_inputs.push(TimelineAgentTextInput {
                    text: quote,
                    source_type: "source_span".to_string(),
                    source_document_id: Some(span.document_id.clone()),
                    source_span_ids: vec![span.source_span_id.clone()],
                    text_chunk_ids: span.chunk_id.iter().cloned().collect(),
                    linked_fact_ids: Vec::new(),
                    linked_claim_ids: Vec::new(),
                    work_product_id: None,
                    block_id: None,
                    index_run_id: None,
                    limit,
                });
            }
            return Ok(TimelineAgentInput {
                matter_id: matter_id.to_string(),
                scope_type: "source_span".to_string(),
                scope_ids: source_span_ids,
                subject_type: "source_span".to_string(),
                subject_id: text_inputs
                    .first()
                    .and_then(|input| input.source_span_ids.first().cloned()),
                mode,
                fact_inputs: Vec::new(),
                text_inputs,
            });
        }

        if let Some(document_ids) = request.document_ids {
            let facts = self.list_facts(matter_id).await?;
            let mut fact_inputs = Vec::new();
            let mut text_inputs = Vec::new();
            for document_id in document_ids.iter().take(limit) {
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
                fact_inputs.push(TimelineAgentFactInput {
                    source_type: "document".to_string(),
                    source_document_id: Some(document_id.clone()),
                    work_product_id: None,
                    block_id: None,
                    index_run_id: None,
                    facts: document_facts,
                    chunks: Vec::new(),
                    limit,
                });
                if let Some(text) = document.extracted_text.clone() {
                    text_inputs.push(TimelineAgentTextInput {
                        text,
                        source_type: "document".to_string(),
                        source_document_id: Some(document_id.clone()),
                        source_span_ids: document
                            .source_spans
                            .iter()
                            .map(|span| span.source_span_id.clone())
                            .collect(),
                        text_chunk_ids: Vec::new(),
                        linked_fact_ids: Vec::new(),
                        linked_claim_ids: Vec::new(),
                        work_product_id: None,
                        block_id: None,
                        index_run_id: None,
                        limit,
                    });
                }
            }
            return Ok(TimelineAgentInput {
                matter_id: matter_id.to_string(),
                scope_type: "document".to_string(),
                scope_ids: document_ids.clone(),
                subject_type: "document".to_string(),
                subject_id: document_ids.first().cloned(),
                mode,
                fact_inputs,
                text_inputs,
            });
        }

        let facts = self.list_facts(matter_id).await?;
        Ok(TimelineAgentInput {
            matter_id: matter_id.to_string(),
            scope_type: "matter".to_string(),
            scope_ids: vec![matter_id.to_string()],
            subject_type: "matter".to_string(),
            subject_id: None,
            mode,
            fact_inputs: vec![TimelineAgentFactInput {
                source_type: "matter_graph".to_string(),
                source_document_id: None,
                work_product_id: None,
                block_id: None,
                index_run_id: None,
                facts,
                chunks: Vec::new(),
                limit,
            }],
            text_inputs: Vec::new(),
        })
    }

    async fn execute_timeline_agent(
        &self,
        input: TimelineAgentInput,
    ) -> ApiResult<TimelineAgentOutcome> {
        let started = now_string();
        let timer = Instant::now();
        let input_hash = timeline_agent_input_hash(&input)?;
        let agent_run_id = generate_id(
            "timeline-agent-run",
            &format!("{}:{}:{input_hash}", input.matter_id, input.scope_type),
        );
        if !self
            .timeline_suggestions_enabled_for_matter(&input.matter_id)
            .await?
        {
            return Ok(disabled_timeline_agent_outcome(
                input,
                agent_run_id,
                input_hash,
                started,
                self.timeline_agent.harness_version.clone(),
                timer.elapsed().as_millis() as u64,
            ));
        }
        let mut warnings =
            vec!["Deterministic timeline extraction ran before provider enrichment.".to_string()];
        let mut candidates = Vec::new();
        for fact_input in &input.fact_inputs {
            candidates.extend(timeline_suggestions_from_facts(
                &input.matter_id,
                fact_input.source_document_id.as_deref(),
                &fact_input.facts,
                &fact_input.chunks,
                &fact_input.source_type,
                fact_input.work_product_id.as_deref(),
                fact_input.block_id.as_deref(),
                Some(agent_run_id.as_str()),
                fact_input.index_run_id.as_deref(),
                fact_input.limit,
            ));
        }
        for text_input in &input.text_inputs {
            candidates.extend(timeline_suggestions_from_text(
                &input.matter_id,
                &text_input.text,
                &text_input.source_type,
                text_input.source_document_id.as_deref(),
                text_input.source_span_ids.clone(),
                text_input.text_chunk_ids.clone(),
                text_input.linked_fact_ids.clone(),
                text_input.linked_claim_ids.clone(),
                text_input.work_product_id.as_deref(),
                text_input.block_id.as_deref(),
                Some(agent_run_id.as_str()),
                text_input.index_run_id.as_deref(),
                text_input.limit,
            ));
        }

        let deterministic_candidate_count = candidates.len() as u64;
        let (mut suggestions, duplicate_candidate_count) = dedupe_timeline_candidates(candidates);
        let ai_enrichment_enabled = self
            .ai_timeline_enrichment_enabled_for_matter(&input.matter_id)
            .await?;
        let (provider, mut provider_warnings) = if ai_enrichment_enabled {
            self.timeline_enrichment_provider()
        } else {
            (
                Box::new(DisabledTimelineProvider {
                    warning: "Timeline AI enrichment is disabled in CaseBuilder settings; deterministic suggestions only.".to_string(),
                }) as Box<dyn TimelineEnrichmentProvider + Send + Sync>,
                vec![
                    "Timeline AI enrichment is disabled in CaseBuilder settings.".to_string(),
                ],
            )
        };
        warnings.append(&mut provider_warnings);
        if let Some(warning) = provider.disabled_warning() {
            push_unique(&mut warnings, warning);
        }

        let provider_request = provider_request_for_suggestions(
            &input.matter_id,
            &agent_run_id,
            &input_hash,
            &suggestions,
            self.timeline_agent.max_input_chars,
            &mut warnings,
        )?;
        let mut provider_enriched_count = 0_u64;
        let mut provider_rejected_count = 0_u64;
        let mut error_code = None;
        let mut error_message = None;
        match provider.enrich(provider_request).await {
            Ok(response) => {
                for warning in response.warnings {
                    push_unique(&mut warnings, warning);
                }
                let result = apply_provider_enrichments(&mut suggestions, response.enrichments);
                provider_enriched_count = result.enriched;
                provider_rejected_count = result.rejected;
                warnings.extend(result.warnings);
            }
            Err(error) => {
                error_code = Some(error.code.clone());
                error_message = Some(error.message.clone());
                warnings.push(format!(
                    "Timeline provider enrichment failed; deterministic suggestions were preserved: {}",
                    error.message
                ));
            }
        }

        let existing = self
            .list_nodes::<TimelineSuggestion>(&input.matter_id, timeline_suggestion_spec())
            .await
            .unwrap_or_default();
        let preserve_result = preserve_timeline_review_state(suggestions, &existing);
        let completed = now_string();
        let produced_suggestion_ids = preserve_result
            .suggestions
            .iter()
            .map(|suggestion| suggestion.suggestion_id.clone())
            .collect::<Vec<_>>();
        if preserve_result.suggestions.is_empty() {
            warnings.push("No dated source-backed statements were found.".to_string());
        }
        let status = if error_code.is_some() {
            "completed_with_warnings"
        } else {
            "completed"
        };
        let run = TimelineAgentRun {
            agent_run_id: agent_run_id.clone(),
            id: agent_run_id,
            matter_id: input.matter_id.clone(),
            subject_type: input.subject_type,
            subject_id: input.subject_id,
            agent_type: TIMELINE_AGENT_TYPE.to_string(),
            scope_type: input.scope_type,
            scope_ids: input.scope_ids,
            input_hash: Some(input_hash),
            pipeline_version: self.timeline_agent.harness_version.clone(),
            extractor_version: TIMELINE_EXTRACTOR_VERSION.to_string(),
            prompt_template_id: Some(TIMELINE_PROMPT_TEMPLATE_ID.to_string()),
            provider: provider.provider_name().to_string(),
            model: provider.model(),
            mode: input.mode,
            provider_mode: provider.provider_mode().to_string(),
            status: status.to_string(),
            message: timeline_agent_message(
                provider.provider_mode(),
                preserve_result.suggestions.len(),
            ),
            produced_suggestion_ids,
            warnings,
            started_at: Some(started.clone()),
            completed_at: Some(completed),
            duration_ms: Some(timer.elapsed().as_millis() as u64),
            error_code,
            error_message,
            deterministic_candidate_count,
            provider_enriched_count,
            provider_rejected_count,
            duplicate_candidate_count,
            stored_suggestion_count: preserve_result.suggestions.len() as u64,
            preserved_review_count: preserve_result.preserved_review_count,
            created_at: started,
        };
        Ok(TimelineAgentOutcome {
            run,
            suggestions: preserve_result.suggestions,
        })
    }

    fn timeline_enrichment_provider(
        &self,
    ) -> (
        Box<dyn TimelineEnrichmentProvider + Send + Sync>,
        Vec<String>,
    ) {
        let mut warnings = Vec::new();
        match self.timeline_agent.provider.as_str() {
            "openai" => {
                let Some(model) = self.timeline_agent.model.clone() else {
                    return (
                        Box::new(DisabledTimelineProvider {
                            warning: "OpenAI timeline enrichment is configured without ORS_CASEBUILDER_TIMELINE_AGENT_MODEL; provider-free deterministic mode used.".to_string(),
                        }),
                        warnings,
                    );
                };
                let Some(api_key) = self.timeline_agent.openai_api_key.clone() else {
                    return (
                        Box::new(DisabledTimelineProvider {
                            warning: "OpenAI timeline enrichment is configured without ORS_OPENAI_API_KEY or OPENAI_API_KEY; provider-free deterministic mode used.".to_string(),
                        }),
                        warnings,
                    );
                };
                (
                    Box::new(OpenAiTimelineProvider {
                        client: self.http_client.clone(),
                        api_key,
                        model,
                        timeout_ms: self.timeline_agent.timeout_ms,
                    }),
                    warnings,
                )
            }
            _ => {
                warnings.push(
                    "Timeline AI enrichment provider is disabled; deterministic suggestions only."
                        .to_string(),
                );
                (
                    Box::new(DisabledTimelineProvider {
                        warning: "Provider-free timeline agent recorded deterministic suggestions; no unsupported text was inserted.".to_string(),
                    }),
                    warnings,
                )
            }
        }
    }
}

struct ReviewPreserveResult {
    suggestions: Vec<TimelineSuggestion>,
    preserved_review_count: u64,
}

fn disabled_timeline_agent_outcome(
    input: TimelineAgentInput,
    agent_run_id: String,
    input_hash: String,
    started: String,
    harness_version: String,
    duration_ms: u64,
) -> TimelineAgentOutcome {
    let completed = now_string();
    let run = TimelineAgentRun {
        agent_run_id: agent_run_id.clone(),
        id: agent_run_id,
        matter_id: input.matter_id,
        subject_type: input.subject_type,
        subject_id: input.subject_id,
        agent_type: TIMELINE_AGENT_TYPE.to_string(),
        scope_type: input.scope_type,
        scope_ids: input.scope_ids,
        input_hash: Some(input_hash),
        pipeline_version: harness_version,
        extractor_version: TIMELINE_EXTRACTOR_VERSION.to_string(),
        prompt_template_id: Some(TIMELINE_PROMPT_TEMPLATE_ID.to_string()),
        provider: "disabled".to_string(),
        model: None,
        mode: input.mode,
        provider_mode: "settings_disabled".to_string(),
        status: "skipped".to_string(),
        message: "Timeline suggestions are disabled in CaseBuilder settings.".to_string(),
        produced_suggestion_ids: Vec::new(),
        warnings: vec![
            "Timeline suggestions are disabled in CaseBuilder settings; no suggestions were generated."
                .to_string(),
        ],
        started_at: Some(started.clone()),
        completed_at: Some(completed),
        duration_ms: Some(duration_ms),
        error_code: None,
        error_message: None,
        deterministic_candidate_count: 0,
        provider_enriched_count: 0,
        provider_rejected_count: 0,
        duplicate_candidate_count: 0,
        stored_suggestion_count: 0,
        preserved_review_count: 0,
        created_at: started,
    };
    TimelineAgentOutcome {
        run,
        suggestions: Vec::new(),
    }
}

struct ProviderApplyResult {
    enriched: u64,
    rejected: u64,
    warnings: Vec<String>,
}

fn timeline_agent_input_hash(input: &TimelineAgentInput) -> ApiResult<String> {
    let value = json_value(input)?;
    let text =
        serde_json::to_string(&value).map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(sha256_hex(text.as_bytes()))
}

fn dedupe_timeline_candidates(
    candidates: Vec<TimelineSuggestion>,
) -> (Vec<TimelineSuggestion>, u64) {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(candidates.len());
    let mut duplicates = 0_u64;
    for suggestion in candidates {
        let key = suggestion
            .dedupe_key
            .clone()
            .unwrap_or_else(|| fallback_suggestion_dedupe_key(&suggestion));
        if seen.insert(key) {
            out.push(suggestion);
        } else {
            duplicates += 1;
        }
    }
    (out, duplicates)
}

fn timeline_ast_source_span_ids(product: &WorkProduct, block: &WorkProductBlock) -> Vec<String> {
    let mut source_span_ids = Vec::new();
    for link in &product.document_ast.links {
        if link.source_block_id == block.block_id
            && matches!(link.target_type.as_str(), "source_span" | "text_span")
        {
            push_unique(&mut source_span_ids, link.target_id.clone());
        }
    }
    if let Some(source_span_id) = block.source_span_id.clone() {
        push_unique(&mut source_span_ids, source_span_id);
    }
    for source_span_id in block
        .evidence_ids
        .iter()
        .filter(|id| id.starts_with("source-span:"))
    {
        push_unique(&mut source_span_ids, source_span_id.clone());
    }
    source_span_ids
}

fn validate_timeline_ast_source_span_links(
    block_id: &str,
    source_span_ids: &[String],
    available_source_span_ids: &HashSet<String>,
) -> ApiResult<()> {
    for source_span_id in source_span_ids {
        if !available_source_span_ids.contains(source_span_id) {
            return Err(ApiError::BadRequest(format!(
                "AST block {block_id} references missing source span {source_span_id}"
            )));
        }
    }
    Ok(())
}

fn fallback_suggestion_dedupe_key(suggestion: &TimelineSuggestion) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        suggestion.date,
        suggestion.source_document_id.clone().unwrap_or_default(),
        suggestion.block_id.clone().unwrap_or_default(),
        suggestion.source_type,
        normalize_for_match(
            suggestion
                .description
                .as_deref()
                .unwrap_or(&suggestion.title)
        )
    )
}

fn preserve_timeline_review_state(
    suggestions: Vec<TimelineSuggestion>,
    existing: &[TimelineSuggestion],
) -> ReviewPreserveResult {
    let mut preserved_review_count = 0_u64;
    let mut out = Vec::with_capacity(suggestions.len());
    for mut suggestion in suggestions {
        if let Some(current) = existing
            .iter()
            .find(|item| item.suggestion_id == suggestion.suggestion_id)
        {
            suggestion.status = current.status.clone();
            suggestion.approved_event_id = current.approved_event_id.clone();
            suggestion.created_at = current.created_at.clone();
            if matches!(current.status.as_str(), "approved" | "rejected") {
                suggestion.title = current.title.clone();
                suggestion.description = current.description.clone();
                suggestion.kind = current.kind.clone();
                suggestion.warnings = current.warnings.clone();
                suggestion.updated_at = current.updated_at.clone();
                suggestion.cluster_id = current.cluster_id.clone();
                suggestion.duplicate_of_suggestion_id = current.duplicate_of_suggestion_id.clone();
                suggestion.agent_explanation = current.agent_explanation.clone();
                suggestion.agent_confidence = current.agent_confidence;
                preserved_review_count += 1;
            }
        }
        out.push(suggestion);
    }
    ReviewPreserveResult {
        suggestions: out,
        preserved_review_count,
    }
}

fn provider_request_for_suggestions(
    matter_id: &str,
    agent_run_id: &str,
    input_hash: &str,
    suggestions: &[TimelineSuggestion],
    max_input_chars: usize,
    warnings: &mut Vec<String>,
) -> ApiResult<TimelineProviderRequest> {
    let mut candidates = Vec::new();
    for suggestion in suggestions {
        candidates.push(TimelineProviderCandidate {
            candidate_id: suggestion.suggestion_id.clone(),
            date: suggestion.date.clone(),
            date_text: suggestion.date_text.clone(),
            title: suggestion.title.clone(),
            kind: suggestion.kind.clone(),
            quote: suggestion.description.clone().unwrap_or_default(),
            warnings: suggestion.warnings.clone(),
        });
        let test = TimelineProviderRequest {
            matter_id: matter_id.to_string(),
            agent_run_id: agent_run_id.to_string(),
            input_hash: input_hash.to_string(),
            candidates: candidates.clone(),
        };
        let length = serde_json::to_string(&test)
            .map_err(|error| ApiError::Internal(error.to_string()))?
            .chars()
            .count();
        if length > max_input_chars {
            candidates.pop();
            warnings.push(format!(
                "Timeline provider input was capped at {max_input_chars} chars; remaining candidates stayed deterministic."
            ));
            break;
        }
    }
    Ok(TimelineProviderRequest {
        matter_id: matter_id.to_string(),
        agent_run_id: agent_run_id.to_string(),
        input_hash: input_hash.to_string(),
        candidates,
    })
}

fn apply_provider_enrichments(
    suggestions: &mut [TimelineSuggestion],
    enrichments: Vec<TimelineProviderEnrichment>,
) -> ProviderApplyResult {
    let ids = suggestions
        .iter()
        .map(|suggestion| suggestion.suggestion_id.clone())
        .collect::<HashSet<_>>();
    let mut by_id = suggestions
        .iter_mut()
        .map(|suggestion| (suggestion.suggestion_id.clone(), suggestion))
        .collect::<HashMap<_, _>>();
    let mut enriched = 0_u64;
    let mut rejected = 0_u64;
    let mut warnings = Vec::new();
    for enrichment in enrichments {
        if !ids.contains(&enrichment.candidate_id) {
            rejected += 1;
            warnings.push(format!(
                "Provider enrichment rejected unknown candidate {}.",
                enrichment.candidate_id
            ));
            continue;
        }
        if let Some(duplicate_of) = enrichment.duplicate_of_candidate_id.as_ref() {
            if !ids.contains(duplicate_of) {
                rejected += 1;
                warnings.push(format!(
                    "Provider enrichment for {} referenced unknown duplicate candidate {}.",
                    enrichment.candidate_id, duplicate_of
                ));
                continue;
            }
        }
        let Some(suggestion) = by_id.get_mut(&enrichment.candidate_id) else {
            rejected += 1;
            continue;
        };
        if let Some(title) = clean_provider_text(enrichment.title.as_deref(), 160) {
            suggestion.title = title;
        }
        if let Some(kind) = clean_provider_text(enrichment.kind.as_deref(), 40) {
            suggestion.kind = kind;
        }
        suggestion.cluster_id = clean_provider_text(enrichment.cluster_id.as_deref(), 120);
        suggestion.duplicate_of_suggestion_id = enrichment.duplicate_of_candidate_id;
        suggestion.agent_explanation = clean_provider_text(enrichment.explanation.as_deref(), 600);
        suggestion.agent_confidence = enrichment.confidence.map(|value| value.clamp(0.0, 1.0));
        for warning in enrichment.warnings {
            push_unique(&mut suggestion.warnings, warning);
        }
        suggestion.updated_at = now_string();
        enriched += 1;
    }
    ProviderApplyResult {
        enriched,
        rejected,
        warnings,
    }
}

fn clean_provider_text(value: Option<&str>, max_chars: usize) -> Option<String> {
    let value = value?.split_whitespace().collect::<Vec<_>>().join(" ");
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.chars().take(max_chars).collect())
}

fn timeline_agent_message(provider_mode: &str, count: usize) -> String {
    if provider_mode == "live" {
        format!(
            "Timeline agent generated {count} reviewable suggestions with live provider enrichment."
        )
    } else {
        format!(
            "Timeline agent generated {count} deterministic reviewable suggestions in provider-free mode."
        )
    }
}

fn openai_timeline_request_body(
    model: &str,
    request: &TimelineProviderRequest,
) -> serde_json::Value {
    json!({
        "model": model,
        "instructions": "You enrich CaseBuilder timeline suggestions. Return JSON only. Do not invent dates, quotes, source IDs, facts, spans, chunks, or claims. Only improve title, kind, cluster grouping, explanation, confidence, and warnings for the supplied candidate_id values.",
        "input": serde_json::to_string(request).unwrap_or_else(|_| "{}".to_string()),
        "text": {
            "format": {
                "type": "json_schema",
                "name": "timeline_enrichment_response",
                "strict": true,
                "schema": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "warnings": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "enrichments": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "properties": {
                                    "candidate_id": { "type": "string" },
                                    "title": { "type": ["string", "null"] },
                                    "kind": { "type": ["string", "null"] },
                                    "cluster_id": { "type": ["string", "null"] },
                                    "duplicate_of_candidate_id": { "type": ["string", "null"] },
                                    "explanation": { "type": ["string", "null"] },
                                    "confidence": { "type": ["number", "null"] },
                                    "warnings": {
                                        "type": "array",
                                        "items": { "type": "string" }
                                    }
                                },
                                "required": [
                                    "candidate_id",
                                    "title",
                                    "kind",
                                    "cluster_id",
                                    "duplicate_of_candidate_id",
                                    "explanation",
                                    "confidence",
                                    "warnings"
                                ]
                            }
                        }
                    },
                    "required": ["warnings", "enrichments"]
                }
            }
        }
    })
}

fn parse_openai_timeline_response(
    payload: &serde_json::Value,
) -> Result<TimelineProviderResponse, TimelineProviderError> {
    let text = openai_output_text(payload).ok_or_else(|| TimelineProviderError {
        code: "provider_output_missing".to_string(),
        message: "OpenAI response did not include output text.".to_string(),
    })?;
    serde_json::from_str::<TimelineProviderResponse>(&text).map_err(|error| TimelineProviderError {
        code: "provider_output_invalid".to_string(),
        message: error.to_string(),
    })
}

fn openai_output_text(payload: &serde_json::Value) -> Option<String> {
    if let Some(value) = payload.get("output_text").and_then(|value| value.as_str()) {
        return Some(value.to_string());
    }
    for item in payload.get("output")?.as_array()? {
        for content in item.get("content")?.as_array()? {
            if content.get("type").and_then(|value| value.as_str()) == Some("output_text") {
                if let Some(text) = content.get("text").and_then(|value| value.as_str()) {
                    return Some(text.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_suggestion(id: &str, text: &str) -> TimelineSuggestion {
        TimelineSuggestion {
            suggestion_id: id.to_string(),
            id: id.to_string(),
            matter_id: "matter:test".to_string(),
            date: "2026-04-01".to_string(),
            date_text: "April 1, 2026".to_string(),
            date_confidence: 0.95,
            title: "Tenant reported mold".to_string(),
            description: Some(text.to_string()),
            kind: "notice".to_string(),
            source_type: "document".to_string(),
            source_document_id: Some("doc:test".to_string()),
            source_span_ids: vec!["source-span:test".to_string()],
            text_chunk_ids: vec!["chunk:test".to_string()],
            markdown_ast_node_ids: Vec::new(),
            linked_fact_ids: vec!["fact:test".to_string()],
            linked_claim_ids: Vec::new(),
            work_product_id: None,
            block_id: None,
            agent_run_id: Some("timeline-agent-run:test".to_string()),
            index_run_id: None,
            dedupe_key: Some("2026-04-01:doc:test:tenant-reported-mold".to_string()),
            cluster_id: None,
            duplicate_of_suggestion_id: None,
            agent_explanation: None,
            agent_confidence: None,
            status: "suggested".to_string(),
            warnings: Vec::new(),
            approved_event_id: None,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        }
    }

    #[test]
    fn dedupe_uses_stable_candidate_keys() {
        let first = test_suggestion(
            "timeline-suggestion:1",
            "Tenant reported mold on April 1, 2026.",
        );
        let second = test_suggestion(
            "timeline-suggestion:2",
            "Tenant reported mold on April 1, 2026.",
        );
        let (suggestions, duplicates) = dedupe_timeline_candidates(vec![first, second]);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(duplicates, 1);
    }

    #[test]
    fn ast_source_span_validation_rejects_missing_span_links() {
        let available = HashSet::from(["source-span:known".to_string()]);
        let error = validate_timeline_ast_source_span_links(
            "block:notice",
            &["source-span:missing".to_string()],
            &available,
        )
        .unwrap_err();

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[test]
    fn provider_enrichment_cannot_change_provenance() {
        let mut suggestions = vec![test_suggestion(
            "timeline-suggestion:1",
            "Tenant reported mold on April 1, 2026.",
        )];
        let result = apply_provider_enrichments(
            &mut suggestions,
            vec![TimelineProviderEnrichment {
                candidate_id: "timeline-suggestion:1".to_string(),
                title: Some("Mold notice delivered".to_string()),
                kind: Some("notice".to_string()),
                cluster_id: Some("cluster:mold-notice".to_string()),
                duplicate_of_candidate_id: None,
                explanation: Some("The supplied quote describes a dated notice.".to_string()),
                confidence: Some(0.88),
                warnings: vec!["Provider label requires review.".to_string()],
            }],
        );

        assert_eq!(result.enriched, 1);
        assert_eq!(suggestions[0].date, "2026-04-01");
        assert_eq!(
            suggestions[0].source_document_id.as_deref(),
            Some("doc:test")
        );
        assert_eq!(suggestions[0].title, "Mold notice delivered");
        assert_eq!(suggestions[0].agent_confidence, Some(0.88));
    }

    #[test]
    fn provider_enrichment_rejects_unknown_candidate_ids() {
        let mut suggestions = vec![test_suggestion("timeline-suggestion:1", "Known.")];
        let result = apply_provider_enrichments(
            &mut suggestions,
            vec![TimelineProviderEnrichment {
                candidate_id: "timeline-suggestion:missing".to_string(),
                title: Some("Invented".to_string()),
                kind: None,
                cluster_id: None,
                duplicate_of_candidate_id: None,
                explanation: None,
                confidence: None,
                warnings: Vec::new(),
            }],
        );

        assert_eq!(result.enriched, 0);
        assert_eq!(result.rejected, 1);
        assert_eq!(suggestions[0].title, "Tenant reported mold");
    }

    #[tokio::test]
    async fn fake_provider_returns_deterministic_enrichment() {
        let provider = FakeTimelineProvider {
            response: Ok(TimelineProviderResponse {
                enrichments: vec![TimelineProviderEnrichment {
                    candidate_id: "timeline-suggestion:1".to_string(),
                    title: Some("Reviewed notice".to_string()),
                    kind: Some("notice".to_string()),
                    cluster_id: Some("cluster:notice".to_string()),
                    duplicate_of_candidate_id: None,
                    explanation: Some("Fixture enrichment.".to_string()),
                    confidence: Some(0.75),
                    warnings: Vec::new(),
                }],
                warnings: vec!["fixture-warning".to_string()],
            }),
        };
        let response = provider
            .enrich(TimelineProviderRequest {
                matter_id: "matter:test".to_string(),
                agent_run_id: "timeline-agent-run:test".to_string(),
                input_hash: "hash".to_string(),
                candidates: Vec::new(),
            })
            .await
            .unwrap();

        assert_eq!(
            response.enrichments[0].title.as_deref(),
            Some("Reviewed notice")
        );
        assert_eq!(response.warnings, vec!["fixture-warning".to_string()]);
    }

    #[test]
    fn openai_request_uses_structured_outputs_schema() {
        let request = TimelineProviderRequest {
            matter_id: "matter:test".to_string(),
            agent_run_id: "timeline-agent-run:test".to_string(),
            input_hash: "hash".to_string(),
            candidates: vec![TimelineProviderCandidate {
                candidate_id: "timeline-suggestion:1".to_string(),
                date: "2026-04-01".to_string(),
                date_text: "April 1, 2026".to_string(),
                title: "Tenant reported mold".to_string(),
                kind: "notice".to_string(),
                quote: "Tenant reported mold on April 1, 2026.".to_string(),
                warnings: Vec::new(),
            }],
        };
        let body = openai_timeline_request_body("gpt-test", &request);

        assert_eq!(body["model"], "gpt-test");
        assert_eq!(body["text"]["format"]["type"], "json_schema");
        assert_eq!(body["text"]["format"]["strict"], true);
        assert!(
            body["instructions"]
                .as_str()
                .unwrap()
                .contains("Do not invent dates")
        );
    }

    #[test]
    fn openai_response_rejects_unsupported_fields() {
        let payload = json!({
            "output_text": r#"{"warnings":[],"enrichments":[{"candidate_id":"timeline-suggestion:1","title":"Notice","kind":"notice","cluster_id":null,"duplicate_of_candidate_id":null,"explanation":null,"confidence":0.9,"warnings":[],"date":"2026-05-01"}]}"#
        });

        let error = parse_openai_timeline_response(&payload).unwrap_err();
        assert_eq!(error.code, "provider_output_invalid");
    }
}
