use crate::error::ApiResult;
use crate::models::search::*;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
};

pub async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> ApiResult<impl IntoResponse> {
    let response = state.search_service.search(query).await?;
    Ok((
        authority_headers(&response.cache_status, &response.corpus_release_id),
        Json(response),
    ))
}

pub async fn open(
    State(state): State<AppState>,
    Query(params): Query<OpenParams>,
) -> ApiResult<impl IntoResponse> {
    let response = state
        .search_service
        .direct_open(&params.q, params.authority_family.as_deref())
        .await?;
    Ok((
        authority_headers(&response.cache_status, &response.corpus_release_id),
        Json(response),
    ))
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

fn authority_headers(cache_status: &str, release_id: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Ok(value) = HeaderValue::from_str(cache_status) {
        headers.insert("x-ors-cache", value);
    }
    if let Ok(value) = HeaderValue::from_str(release_id) {
        headers.insert("x-ors-corpus-release", value);
    }
    headers
}
