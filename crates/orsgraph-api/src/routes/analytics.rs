use crate::error::ApiResult;
use crate::routes::authority::authority_headers_for_state;
use crate::state::AppState;
use axum::{Json, extract::State, response::IntoResponse};

pub async fn home_insights(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let data = state.analytics_service.get_home_insights().await?;
    Ok((authority_headers_for_state(&state, "origin"), Json(data)))
}
