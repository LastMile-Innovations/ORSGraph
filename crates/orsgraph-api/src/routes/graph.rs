use crate::error::ApiResult;
use crate::models::api::{GraphNeighborhoodRequest, GraphNeighborhoodResponse};
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
