use crate::error::ApiResult;
use crate::models::api::*;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;

pub async fn get_statute(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<StatuteDetailResponse>> {
    let statute = state.neo4j_service.get_statute(&citation).await?;
    Ok(Json(statute))
}

pub async fn get_provisions(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<ProvisionsResponse>> {
    let provisions = state.neo4j_service.get_provisions(&citation).await?;
    Ok(Json(provisions))
}

pub async fn get_citations(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<CitationsResponse>> {
    let citations = state.neo4j_service.get_citations(&citation).await?;
    Ok(Json(citations))
}

pub async fn get_semantics(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<SemanticsResponse>> {
    let semantics = state.neo4j_service.get_semantics(&citation).await?;
    Ok(Json(semantics))
}

pub async fn get_history(
    Path(citation): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<HistoryResponse>> {
    let history = state.neo4j_service.get_history(&citation).await?;
    Ok(Json(history))
}
