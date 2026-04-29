use crate::error::ApiResult;
use crate::models::api::StatsResponse;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;

pub async fn stats(State(state): State<AppState>) -> ApiResult<Json<StatsResponse>> {
    let stats = state.stats_service.get_stats_response().await?;
    Ok(Json(stats))
}
