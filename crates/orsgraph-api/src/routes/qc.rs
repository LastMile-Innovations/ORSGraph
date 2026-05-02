use crate::error::ApiResult;
use crate::models::api::{QCReportRequest, QCReportResponse, QCRunResponse, QCSummaryResponse};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Query, State};

pub async fn get_qc_summary(State(state): State<AppState>) -> ApiResult<Json<QCSummaryResponse>> {
    let summary = state.neo4j_service.get_qc_summary().await?;
    Ok(Json(summary))
}

pub async fn run_qc(State(state): State<AppState>) -> ApiResult<Json<QCRunResponse>> {
    let run = state.neo4j_service.run_qc().await?;
    Ok(Json(run))
}

pub async fn get_latest_report(
    State(state): State<AppState>,
    Query(params): Query<QCReportRequest>,
) -> ApiResult<Json<QCReportResponse>> {
    let report = state
        .neo4j_service
        .get_qc_report(params.format.as_deref())
        .await?;
    Ok(Json(report))
}
