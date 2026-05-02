use crate::error::ApiResult;
use crate::models::admin::{
    AdminJobKind, AdminJobStatus, AdminLogResponse, AdminOverview, AdminSourceDetail,
    AdminSourceRegistryResponse, AdminStartJobRequest,
};
use crate::state::AppState;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/overview", get(get_overview))
        .route("/admin/sources", get(list_sources))
        .route("/admin/sources/{source_id}", get(get_source))
        .route("/admin/jobs", get(list_jobs).post(start_job))
        .route("/admin/jobs/{id}", get(get_job))
        .route("/admin/jobs/{id}/logs", get(get_job_logs))
        .route("/admin/jobs/{id}/cancel", post(cancel_job))
        .route("/admin/jobs/{id}/kill", post(kill_job))
}

#[derive(Debug, Deserialize)]
pub struct JobsQuery {
    pub status: Option<AdminJobStatus>,
    pub kind: Option<AdminJobKind>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub stream: Option<String>,
    pub tail: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SourcesQuery {
    pub priority: Option<String>,
    pub connector_status: Option<String>,
}

async fn get_overview(State(state): State<AppState>) -> ApiResult<Json<AdminOverview>> {
    let overview = state.admin_service.overview(&state.health_service).await?;
    Ok(Json(overview))
}

async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<JobsQuery>,
) -> ApiResult<Json<Vec<crate::models::admin::AdminJob>>> {
    let jobs = state
        .admin_service
        .list_jobs(
            query.status,
            query.kind,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await?;
    Ok(Json(jobs))
}

async fn list_sources(
    State(state): State<AppState>,
    Query(query): Query<SourcesQuery>,
) -> ApiResult<Json<AdminSourceRegistryResponse>> {
    Ok(Json(
        state
            .admin_service
            .list_sources(query.priority, query.connector_status)
            .await?,
    ))
}

async fn get_source(
    State(state): State<AppState>,
    Path(source_id): Path<String>,
) -> ApiResult<Json<AdminSourceDetail>> {
    Ok(Json(state.admin_service.get_source(&source_id).await?))
}

async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<crate::models::admin::AdminJobDetail>> {
    Ok(Json(state.admin_service.get_job_detail(&id).await?))
}

async fn get_job_logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> ApiResult<Json<AdminLogResponse>> {
    Ok(Json(
        state
            .admin_service
            .get_logs(
                &id,
                query.stream.as_deref().unwrap_or("stdout"),
                query.tail.unwrap_or(200),
            )
            .await?,
    ))
}

async fn start_job(
    State(state): State<AppState>,
    Json(request): Json<AdminStartJobRequest>,
) -> ApiResult<Json<crate::models::admin::AdminJobDetail>> {
    Ok(Json(state.admin_service.start_job(request).await?))
}

async fn cancel_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<crate::models::admin::AdminJobDetail>> {
    Ok(Json(state.admin_service.cancel_job(&id).await?))
}

async fn kill_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<crate::models::admin::AdminJobDetail>> {
    Ok(Json(state.admin_service.kill_job(&id).await?))
}
