use crate::error::ApiResult;
use crate::routes::authority::authority_headers_for_state;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;

pub async fn stats(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let stats = state.stats_service.get_stats_response().await?;
    Ok((authority_headers_for_state(&state, "origin"), Json(stats)))
}
