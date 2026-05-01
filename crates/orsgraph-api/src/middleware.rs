use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap},
    middleware::Next,
    response::Response,
};

pub async fn optional_api_key_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let expected_key = state.config.api_key.as_deref();

    if is_authorized(req.headers(), expected_key) {
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
    use super::is_authorized;
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
}
