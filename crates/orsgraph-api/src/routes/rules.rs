use crate::error::ApiResult;
use crate::state::AppState;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApplicableRulesQuery {
    pub jurisdiction: String,
    pub date: String,
    #[serde(rename = "type")]
    pub work_product_type: Option<String>,
    pub court: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/rules/registry", get(get_registry))
        .route(
            "/rules/jurisdictions/:jurisdiction_id/current",
            get(get_current_for_jurisdiction),
        )
        .route(
            "/rules/jurisdictions/:jurisdiction_id/history",
            get(get_history_for_jurisdiction),
        )
        .route("/rules/applicable", get(get_applicable_rules))
        .route("/rules/orders/:authority_document_id", get(get_order))
        .route("/rules/slr/:jurisdiction_id/:year", get(get_slr_edition))
}

async fn get_registry(State(state): State<AppState>) -> ApiResult<Json<impl serde::Serialize>> {
    Ok(Json(state.rule_applicability_resolver.registry().await?))
}

async fn get_current_for_jurisdiction(
    State(state): State<AppState>,
    Path(jurisdiction_id): Path<String>,
) -> ApiResult<Json<impl serde::Serialize>> {
    Ok(Json(
        state
            .rule_applicability_resolver
            .current_for_jurisdiction(&jurisdiction_id)
            .await?,
    ))
}

async fn get_history_for_jurisdiction(
    State(state): State<AppState>,
    Path(jurisdiction_id): Path<String>,
) -> ApiResult<Json<impl serde::Serialize>> {
    Ok(Json(
        state
            .rule_applicability_resolver
            .history_for_jurisdiction(&jurisdiction_id)
            .await?,
    ))
}

async fn get_applicable_rules(
    State(state): State<AppState>,
    Query(params): Query<ApplicableRulesQuery>,
) -> ApiResult<Json<impl serde::Serialize>> {
    Ok(Json(
        state
            .rule_applicability_resolver
            .applicable(
                &params.jurisdiction,
                params.court.as_deref(),
                params.work_product_type.as_deref().unwrap_or("complaint"),
                &params.date,
            )
            .await?,
    ))
}

async fn get_order(
    State(state): State<AppState>,
    Path(authority_document_id): Path<String>,
) -> ApiResult<Json<impl serde::Serialize>> {
    Ok(Json(
        state
            .rule_applicability_resolver
            .order(&authority_document_id)
            .await?,
    ))
}

async fn get_slr_edition(
    State(state): State<AppState>,
    Path((jurisdiction_id, year)): Path<(String, i64)>,
) -> ApiResult<Json<impl serde::Serialize>> {
    Ok(Json(
        state
            .rule_applicability_resolver
            .slr_edition(&jurisdiction_id, year)
            .await?,
    ))
}
