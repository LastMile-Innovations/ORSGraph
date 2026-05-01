use crate::error::ApiResult;
use crate::models::api::{SourceDetailResponse, SourceIndexRequest, SourceIndexResponse};
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;

pub async fn list_sources(
    State(state): State<AppState>,
    Query(params): Query<SourceIndexRequest>,
) -> ApiResult<Json<SourceIndexResponse>> {
    Ok(Json(state.neo4j_service.list_sources(&params).await?))
}

pub async fn get_source(
    State(state): State<AppState>,
    Path(source_id): Path<String>,
) -> ApiResult<Json<SourceDetailResponse>> {
    Ok(Json(state.neo4j_service.get_source(&source_id).await?))
}
