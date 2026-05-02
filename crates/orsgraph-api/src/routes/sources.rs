use crate::error::ApiResult;
use crate::models::api::SourceIndexRequest;
use crate::routes::authority::authority_headers_for_state;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;

pub async fn list_sources(
    State(state): State<AppState>,
    Query(params): Query<SourceIndexRequest>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        authority_headers_for_state(&state, "origin"),
        Json(state.neo4j_service.list_sources(&params).await?),
    ))
}

pub async fn get_source(
    State(state): State<AppState>,
    Path(source_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        authority_headers_for_state(&state, "origin"),
        Json(state.neo4j_service.get_source(&source_id).await?),
    ))
}
