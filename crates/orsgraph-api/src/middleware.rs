use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap},
    middleware::Next,
    response::Response,
};

const ASSEMBLYAI_WEBHOOK_SECRET_HEADER: &str = "x-casebuilder-assemblyai-secret";
const ASSEMBLYAI_WEBHOOK_PATHS: &[&str] = &[
    "/api/v1/casebuilder/webhooks/assemblyai",
    "/casebuilder/webhooks/assemblyai",
];

pub async fn optional_api_key_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let expected_key = state.config.api_key.as_deref();
    let webhook_secret = state.config.assemblyai_webhook_secret.as_deref();

    if is_authorized(req.headers(), expected_key)
        || is_assemblyai_webhook_authorized(req.uri().path(), req.headers(), webhook_secret)
    {
        Ok(next.run(req).await)
    } else {
        Err(ApiError::Unauthorized)
    }
}

pub(crate) fn is_authorized(headers: &HeaderMap, expected_key: Option<&str>) -> bool {
    let Some(expected_key) = expected_key.filter(|key| !key.trim().is_empty()) else {
        return true;
    };

    header_value(headers, "x-api-key").is_some_and(|provided| provided == expected_key)
        || bearer_token(headers).is_some_and(|provided| provided == expected_key)
}

pub(crate) fn is_assemblyai_webhook_authorized(
    path: &str,
    headers: &HeaderMap,
    expected_secret: Option<&str>,
) -> bool {
    let Some(expected_secret) = expected_secret.filter(|secret| !secret.trim().is_empty()) else {
        return false;
    };
    ASSEMBLYAI_WEBHOOK_PATHS.contains(&path)
        && header_value(headers, ASSEMBLYAI_WEBHOOK_SECRET_HEADER)
            .is_some_and(|provided| provided == expected_secret)
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    header_value(headers, header::AUTHORIZATION.as_str())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{is_assemblyai_webhook_authorized, is_authorized};
    use axum::http::{header, HeaderMap, HeaderValue};

    #[test]
    fn allows_requests_when_no_key_is_configured() {
        assert!(is_authorized(&HeaderMap::new(), None));
        assert!(is_authorized(&HeaderMap::new(), Some("")));
    }

    #[test]
    fn accepts_x_api_key_or_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("secret"));
        assert!(is_authorized(&headers, Some("secret")));

        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer secret"),
        );
        assert!(is_authorized(&headers, Some("secret")));
    }

    #[test]
    fn rejects_missing_or_wrong_key() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("wrong"));

        assert!(!is_authorized(&HeaderMap::new(), Some("secret")));
        assert!(!is_authorized(&headers, Some("secret")));
    }

    #[test]
    fn accepts_assemblyai_secret_only_on_webhook_path() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-casebuilder-assemblyai-secret",
            HeaderValue::from_static("webhook-secret"),
        );

        assert!(is_assemblyai_webhook_authorized(
            "/api/v1/casebuilder/webhooks/assemblyai",
            &headers,
            Some("webhook-secret"),
        ));
        assert!(!is_assemblyai_webhook_authorized(
            "/api/v1/matters",
            &headers,
            Some("webhook-secret"),
        ));
        assert!(!is_assemblyai_webhook_authorized(
            "/api/v1/casebuilder/webhooks/assemblyai",
            &headers,
            Some("different"),
        ));
    }
}
