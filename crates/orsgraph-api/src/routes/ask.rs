use crate::error::ApiResult;
use crate::models::api::*;
use crate::models::search::{SearchMode, SearchQuery};
use crate::state::AppState;
use axum::{extract::State, Json};
use std::collections::HashSet;

pub async fn ask(
    State(state): State<AppState>,
    Json(request): Json<AskRequest>,
) -> ApiResult<Json<AskAnswerResponse>> {
    let search = state
        .search_service
        .search(SearchQuery {
            q: request.question.clone(),
            r#type: Some("all".to_string()),
            authority_family: None,
            chapter: None,
            status: None,
            mode: Some(SearchMode::Auto),
            limit: Some(8),
            offset: Some(0),
            include: None,
            semantic_type: None,
            current_only: Some(true),
            source_backed: Some(true),
            has_citations: None,
            has_deadlines: None,
            has_penalties: None,
            needs_review: None,
        })
        .await?;

    let mut seen_citations = HashSet::new();
    let mut controlling_law = Vec::new();
    let mut relevant_provisions = Vec::new();
    let mut citations = Vec::new();
    let mut retrieved_chunks = Vec::new();
    let mut qc_notes = Vec::new();

    for result in &search.results {
        if let Some(citation) = &result.citation {
            if seen_citations.insert(citation.clone()) {
                citations.push(citation.clone());
                controlling_law.push(AskControllingLaw {
                    citation: citation.clone(),
                    canonical_id: result
                        .graph
                        .as_ref()
                        .and_then(|g| g.canonical_id.clone())
                        .unwrap_or_else(|| result.id.clone()),
                    reason: result
                        .title
                        .clone()
                        .unwrap_or_else(|| "High-ranking source-backed match".to_string()),
                });
            }
        }

        if result.kind.eq_ignore_ascii_case("provision")
            || result
                .source
                .as_ref()
                .and_then(|source| source.provision_id.as_ref())
                .is_some()
        {
            relevant_provisions.push(AskRelevantProvision {
                citation: result.citation.clone().unwrap_or_else(|| result.id.clone()),
                provision_id: result
                    .source
                    .as_ref()
                    .and_then(|source| source.provision_id.clone())
                    .unwrap_or_else(|| result.id.clone()),
                text_preview: result.snippet.clone(),
            });
        }

        retrieved_chunks.push(AskRetrievedChunk {
            chunk_id: result
                .source
                .as_ref()
                .and_then(|source| source.chunk_id.clone())
                .unwrap_or_else(|| result.id.clone()),
            chunk_type: if result.kind.eq_ignore_ascii_case("chunk") {
                "contextual_provision".to_string()
            } else {
                "full_statute".to_string()
            },
            score: result.score,
            preview: result.snippet.clone(),
        });

        qc_notes.extend(result.qc_warnings.iter().cloned());
    }

    let short_answer = if search.results.is_empty() {
        "I could not find source-backed ORS material for that question in the current graph."
            .to_string()
    } else {
        let top = &search.results[0];
        format!(
            "The strongest source-backed match is {}{}: {}",
            top.citation.clone().unwrap_or_else(|| top.id.clone()),
            top.title
                .as_ref()
                .map(|title| format!(" ({})", title))
                .unwrap_or_default(),
            top.snippet
        )
    };

    let mut caveats = search.warnings;
    caveats.push(
        "This answer is generated from parsed graph records; verify legal conclusions against official Oregon sources.".to_string(),
    );

    Ok(Json(AskAnswerResponse {
        question: request.question,
        mode: request.mode.unwrap_or_else(|| "research".to_string()),
        short_answer,
        controlling_law: controlling_law.into_iter().take(5).collect(),
        relevant_provisions: relevant_provisions.into_iter().take(8).collect(),
        definitions: Vec::new(),
        exceptions: Vec::new(),
        deadlines: Vec::new(),
        citations,
        caveats,
        retrieved_chunks,
        qc_notes,
    }))
}
