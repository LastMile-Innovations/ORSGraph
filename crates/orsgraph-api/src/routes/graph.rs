use crate::error::ApiResult;
use crate::models::api::{GraphFullRequest, GraphNeighborhoodRequest, GraphPathRequest};
use crate::routes::authority::authority_headers_for_state;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Query, State};
use axum::response::IntoResponse;

pub async fn get_neighborhood(
    Query(params): Query<GraphNeighborhoodRequest>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let neighborhood = state.neo4j_service.get_neighborhood(&params).await?;
    Ok((
        authority_headers_for_state(&state, "origin"),
        Json(neighborhood),
    ))
}

pub async fn get_full(
    Query(params): Query<GraphFullRequest>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let graph = state.neo4j_service.get_full_graph(&params).await?;
    Ok((authority_headers_for_state(&state, "origin"), Json(graph)))
}

pub async fn get_path(
    Query(params): Query<GraphPathRequest>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let path = state.neo4j_service.get_graph_path(&params).await?;
    Ok((authority_headers_for_state(&state, "origin"), Json(path)))
}
