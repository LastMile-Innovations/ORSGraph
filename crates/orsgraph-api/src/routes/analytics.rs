use crate::error::ApiResult;
use crate::models::home::GraphInsightCard;
use crate::state::AppState;
use axum::{extract::State, Json};

pub async fn home_insights(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<GraphInsightCard>>> {
    let data = state.analytics_service.get_home_insights().await?;
    Ok(Json(data))
}
