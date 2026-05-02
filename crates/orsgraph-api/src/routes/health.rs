use crate::error::ApiResult;
use crate::models::api::HealthResponse;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;

pub async fn health(State(state): State<AppState>) -> ApiResult<Json<HealthResponse>> {
    let neo4j_ok = state.health_service.check_neo4j().await.unwrap_or(false);

    Ok(Json(HealthResponse {
        ok: neo4j_ok,
        service: "orsgraph-api".to_string(),
        neo4j: if neo4j_ok {
            "connected".to_string()
        } else {
            "disconnected".to_string()
        },
        version: "0.1.0".to_string(),
    }))
}
