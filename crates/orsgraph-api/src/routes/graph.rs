use crate::error::ApiResult;
use crate::models::api::{
    GraphNeighborhoodRequest, GraphNeighborhoodResponse, GraphPathRequest, GraphPathResponse,
};
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;

pub async fn get_neighborhood(
    Query(params): Query<GraphNeighborhoodRequest>,
    State(state): State<AppState>,
) -> ApiResult<Json<GraphNeighborhoodResponse>> {
    let neighborhood = state.neo4j_service.get_neighborhood(&params).await?;
    Ok(Json(neighborhood))
}

pub async fn get_path(
    Query(params): Query<GraphPathRequest>,
    State(state): State<AppState>,
) -> ApiResult<Json<GraphPathResponse>> {
    let path = state.neo4j_service.get_graph_path(&params).await?;
    Ok(Json(path))
}
