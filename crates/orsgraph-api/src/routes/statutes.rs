use crate::error::ApiResult;
use crate::routes::authority::authority_headers_for_state;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;

#[derive(serde::Deserialize)]
pub struct StatuteListParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub q: Option<String>,
    pub chapter: Option<String>,
    pub status: Option<String>,
}

pub async fn list_statutes(
    Query(params): Query<StatuteListParams>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let statutes = state
        .neo4j_service
        .list_statutes(
            params.limit,
            params.offset,
            params.q.as_deref(),
            params.chapter.as_deref(),
            params.status.as_deref(),
        )
        .await?;
    Ok((
        authority_headers_for_state(&state, "origin"),
        Json(statutes),
    ))
}

pub async fn get_statute_page(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let statute = state.neo4j_service.get_statute_page(&citation).await?;
    Ok((authority_headers_for_state(&state, "origin"), Json(statute)))
}

pub async fn get_statute(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let statute = state.neo4j_service.get_statute(&citation).await?;
    Ok((authority_headers_for_state(&state, "origin"), Json(statute)))
}

pub async fn get_provisions(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let provisions = state.neo4j_service.get_provisions(&citation).await?;
    Ok((
        authority_headers_for_state(&state, "origin"),
        Json(provisions),
    ))
}

pub async fn get_citations(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let citations = state.neo4j_service.get_citations(&citation).await?;
    Ok((
        authority_headers_for_state(&state, "origin"),
        Json(citations),
    ))
}

pub async fn get_semantics(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let semantics = state.neo4j_service.get_semantics(&citation).await?;
    Ok((
        authority_headers_for_state(&state, "origin"),
        Json(semantics),
    ))
}

pub async fn get_history(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let history = state.neo4j_service.get_history(&citation).await?;
    Ok((authority_headers_for_state(&state, "origin"), Json(history)))
}

pub async fn get_chunks(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let chunks = state.neo4j_service.get_chunks(&citation).await?;
    Ok((authority_headers_for_state(&state, "origin"), Json(chunks)))
}

pub async fn get_provision(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let provision = state.neo4j_service.get_provision_detail(&id).await?;
    Ok((
        authority_headers_for_state(&state, "origin"),
        Json(provision),
    ))
}
