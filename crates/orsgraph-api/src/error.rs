use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Neo4j connection error: {0}")]
    Neo4jConnection(#[from] neo4rs::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("External service error: {0}")]
    External(String),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Unauthorized")]
    Unauthorized,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message): (StatusCode, String) = match self {
            ApiError::Neo4jConnection(e) => {
                tracing::error!("Neo4j error: {}", e);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Neo4j connection failed".to_string(),
                )
            }
            ApiError::Config(e) => {
                tracing::error!("Config error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Configuration error".to_string(),
                )
            }
            ApiError::External(msg) => {
                tracing::error!("External service error: {}", msg);
                (StatusCode::SERVICE_UNAVAILABLE, msg)
            }
            ApiError::HttpClient(e) => {
                tracing::error!("HTTP client error: {}", e);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "External service unreachable".to_string(),
                )
            }
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
        };

        let body = json!({
            "error": error_message,
        });

        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
