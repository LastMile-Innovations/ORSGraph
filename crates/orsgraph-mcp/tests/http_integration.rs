use axum::{Json, Router, extract::Query, routing::get};
use orsgraph_mcp::{
    OrsGraphApiClient,
    streamable_http::{StreamableHttpRuntimeConfig, streamable_http_router},
};
use reqwest::StatusCode;
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn streamable_http_requires_bearer_when_configured() -> anyhow::Result<()> {
    let api_base_url = spawn_test_api().await?;
    let server = spawn_mcp_http(api_base_url, Some("secret".to_string()), None).await?;

    let response = reqwest::Client::new()
        .post(format!("{}/mcp", server.base_url))
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .json(&initialize_payload())
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    server.shutdown();
    Ok(())
}

#[tokio::test]
async fn streamable_http_rejects_disallowed_origin() -> anyhow::Result<()> {
    let api_base_url = spawn_test_api().await?;
    let server = spawn_mcp_http(
        api_base_url,
        Some("secret".to_string()),
        Some(vec!["http://allowed.example".to_string()]),
    )
    .await?;

    let response = reqwest::Client::new()
        .post(format!("{}/mcp", server.base_url))
        .header("authorization", "Bearer secret")
        .header("origin", "http://evil.example")
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .json(&initialize_payload())
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    server.shutdown();
    Ok(())
}

#[tokio::test]
async fn streamable_http_initializes_session() -> anyhow::Result<()> {
    let api_base_url = spawn_test_api().await?;
    let server = spawn_mcp_http(api_base_url, Some("secret".to_string()), None).await?;

    let response = reqwest::Client::new()
        .post(format!("{}/mcp", server.base_url))
        .header("authorization", "Bearer secret")
        .header("origin", server.base_url.clone())
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .json(&initialize_payload())
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("mcp-session-id").is_some());
    assert!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("text/event-stream"))
    );

    server.shutdown();
    Ok(())
}

fn initialize_payload() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {
                "name": "orsgraph-mcp-http-test",
                "version": "0.1.0"
            }
        }
    })
}

struct TestHttpServer {
    base_url: String,
    cancellation_token: CancellationToken,
    handle: JoinHandle<()>,
}

impl TestHttpServer {
    fn shutdown(self) {
        self.cancellation_token.cancel();
        self.handle.abort();
    }
}

async fn spawn_mcp_http(
    api_base_url: String,
    bearer_token: Option<String>,
    allowed_origins: Option<Vec<String>>,
) -> anyhow::Result<TestHttpServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let api = OrsGraphApiClient::new(api_base_url, Duration::from_secs(3))
        .map_err(|error| anyhow::anyhow!(error))?;
    let mut config = StreamableHttpRuntimeConfig::local_default().with_bind(addr);
    config.bearer_token = bearer_token;
    if let Some(allowed_origins) = allowed_origins {
        config.allowed_origins = allowed_origins;
    }

    let cancellation_token = CancellationToken::new();
    let router = streamable_http_router(api, config, cancellation_token.child_token())
        .map_err(|error| anyhow::anyhow!(error))?;
    let handle = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("mcp http server");
    });

    Ok(TestHttpServer {
        base_url: format!("http://{addr}"),
        cancellation_token,
        handle,
    })
}

async fn spawn_test_api() -> anyhow::Result<String> {
    async fn health() -> Json<Value> {
        Json(json!({
            "ok": true,
            "service": "orsgraph-api",
            "neo4j": "connected",
            "version": "test"
        }))
    }

    async fn search(Query(params): Query<HashMap<String, String>>) -> Json<Value> {
        Json(json!({
            "query": params.get("q").cloned().unwrap_or_default(),
            "mode": "auto",
            "total": 1,
            "limit": params.get("limit").and_then(|value| value.parse::<u32>().ok()).unwrap_or(10),
            "results": [{
                "id": "or:ors:90.100",
                "citation": "ORS 90.100",
                "title": "Definitions",
                "snippet": "Test fixture"
            }]
        }))
    }

    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/search", get(search));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("test API server");
    });

    Ok(format!("http://{addr}/api/v1"))
}
