use crate::error::ApiResult;
use crate::models::api::QCSummaryResponse;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;

pub async fn get_qc_summary(State(state): State<AppState>) -> ApiResult<Json<QCSummaryResponse>> {
    let summary = state.neo4j_service.get_qc_summary().await?;
    Ok(Json(summary))
}
