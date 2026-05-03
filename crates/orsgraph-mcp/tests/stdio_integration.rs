use axum::{Json, Router, extract::Query, routing::get};
use orsgraph_mcp::{OrsGraphApiClient, OrsGraphMcpServer};
use rmcp::{ClientHandler, ServiceExt, model::CallToolRequestParams, object};
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};
use tokio::net::TcpListener;

#[derive(Clone, Default)]
struct TestClient;

impl ClientHandler for TestClient {}

#[tokio::test]
async fn mcp_stdio_lists_and_calls_search_tool() -> anyhow::Result<()> {
    let api_base_url = spawn_test_api().await?;
    let api = OrsGraphApiClient::new(api_base_url, Duration::from_secs(3))
        .map_err(|error| anyhow::anyhow!(error))?;
    let server = OrsGraphMcpServer::new(api);

    let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
    let server_handle = tokio::spawn(async move {
        let service = server.serve(server_transport).await?;
        service.waiting().await?;
        anyhow::Ok(())
    });

    let client = TestClient.serve(client_transport).await?;
    let tools = client.list_all_tools().await?;
    assert!(tools.iter().any(|tool| tool.name == "orsgraph_search"));
    assert!(tools.iter().any(|tool| tool.name == "orsgraph_health"));

    let result = client
        .call_tool(
            CallToolRequestParams::new("orsgraph_search")
                .with_arguments(object!({ "q": "ORS 90.100", "limit": 1 })),
        )
        .await?;

    assert_eq!(result.is_error, Some(false));
    let structured = result.structured_content.expect("structured result");
    assert_eq!(structured["endpoint"], "search");
    assert_eq!(structured["body"]["query"], "ORS 90.100");
    assert_eq!(structured["body"]["limit"], 1);

    client.cancel().await?;
    let _ = server_handle.await;
    Ok(())
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
