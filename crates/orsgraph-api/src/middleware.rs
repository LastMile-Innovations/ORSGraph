use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

pub async fn optional_api_key_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let api_key = std::env::var("ORS_API_KEY").ok();

    if let Some(expected_key) = api_key {
        let provided_key = req.headers().get("x-api-key").and_then(|v| v.to_str().ok());

        match provided_key {
            Some(key) if key == expected_key => Ok(next.run(req).await),
            _ => Err(StatusCode::UNAUTHORIZED),
        }
    } else {
        // No API key configured, allow all requests
        Ok(next.run(req).await)
    }
}
