use axum::{http::StatusCode, Json};

pub async fn ask() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "Ask endpoint not yet implemented"
        })),
    )
}
