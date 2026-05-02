use crate::auth::{
    bearer_token, header_value, is_admin_operation, is_public_path, matter_id_from_path,
    AuthContext,
};
use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{Request, State},
    http::HeaderMap,
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
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let expected_key = state.config.api_key.as_deref();
    let webhook_secret = state.config.assemblyai_webhook_secret.as_deref();
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    let auth = if method == axum::http::Method::OPTIONS || is_public_path(&path) {
        AuthContext::anonymous()
    } else if expected_key.is_some_and(|key| !key.trim().is_empty())
        && is_authorized(req.headers(), expected_key)
    {
        AuthContext::service()
    } else if is_assemblyai_webhook_authorized(&path, req.headers(), webhook_secret) {
        AuthContext::service()
    } else if let Some(verifier) = state.auth_verifier.as_ref() {
        let token = bearer_token(req.headers()).ok_or(ApiError::Unauthorized)?;
        verifier.verify_bearer(token).await?
    } else if expected_key.is_some_and(|key| !key.trim().is_empty()) {
        return Err(ApiError::Unauthorized);
    } else {
        AuthContext::anonymous()
    };

    if method != axum::http::Method::OPTIONS
        && !is_public_path(&path)
        && state.config.auth_enabled
        && !auth.is_authenticated()
    {
        return Err(ApiError::Unauthorized);
    }

    if method != axum::http::Method::OPTIONS
        && is_admin_operation(&method, &path)
        && !auth.is_admin(&state.config.auth_admin_role)
    {
        return Err(ApiError::Forbidden("Admin role required".to_string()));
    }

    if method != axum::http::Method::OPTIONS {
        if let Some(matter_id) = matter_id_from_path(&path) {
            state
                .casebuilder_service
                .authorize_matter_access(&matter_id, &auth, &state.config.auth_admin_role)
                .await?;
        }
    }

    req.extensions_mut().insert(auth);
    Ok(next.run(req).await)
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
