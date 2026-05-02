use crate::error::ApiResult;
use crate::models::search::*;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Query, State},
};

pub async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<SearchResponse>> {
    let response = state.search_service.search(query).await?;
    Ok(Json(response))
}

pub async fn open(
    State(state): State<AppState>,
    Query(params): Query<OpenParams>,
) -> ApiResult<Json<DirectOpenResponse>> {
    let response = state
        .search_service
        .direct_open(&params.q, params.authority_family.as_deref())
        .await?;
    Ok(Json(response))
}

pub async fn suggest(
    State(state): State<AppState>,
    Query(params): Query<SuggestParams>,
) -> ApiResult<Json<Vec<SuggestResult>>> {
    let response = state
        .search_service
        .suggest(&params.q, params.limit)
        .await?;
    Ok(Json(response))
}

#[derive(serde::Deserialize)]
pub struct OpenParams {
    pub q: String,
    pub authority_family: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct SuggestParams {
    pub q: String,
    pub limit: Option<u32>,
}
