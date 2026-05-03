use axum::{
    Json, Router,
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
};
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
    assert!(tools.iter().any(|tool| tool.name == "orsgraph_stats"));
    assert!(tools.iter().any(|tool| tool.name == "orsgraph_sources"));
    assert!(
        tools
            .iter()
            .any(|tool| tool.name == "orsgraph_rules_registry")
    );
    assert!(
        tools
            .iter()
            .any(|tool| tool.name == "orsgraph_rule_applicability")
    );

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

    let result = client
        .call_tool(CallToolRequestParams::new("orsgraph_stats"))
        .await?;
    assert_eq!(result.is_error, Some(false));
    let structured = result.structured_content.expect("structured stats result");
    assert_eq!(structured["endpoint"], "stats");
    assert_eq!(structured["body"]["statutes"], 42);

    let result = client
        .call_tool(
            CallToolRequestParams::new("orsgraph_sources")
                .with_arguments(object!({ "q": "UTCR", "limit": 500 })),
        )
        .await?;
    assert_eq!(result.is_error, Some(false));
    let structured = result
        .structured_content
        .expect("structured sources result");
    assert_eq!(structured["endpoint"], "sources");
    assert_eq!(structured["body"]["query"]["q"], "UTCR");
    assert_eq!(structured["body"]["query"]["limit"], 100);

    let result = client
        .call_tool(
            CallToolRequestParams::new("orsgraph_rule_applicability").with_arguments(object!({
                "jurisdiction": "Linn",
                "date": "2026-02-15",
                "work_product_type": "complaint"
            })),
        )
        .await?;
    assert_eq!(result.is_error, Some(false));
    let structured = result
        .structured_content
        .expect("structured applicability result");
    assert_eq!(structured["endpoint"], "rules/applicable");
    assert_eq!(structured["body"]["jurisdiction"], "Linn");
    assert_eq!(structured["body"]["work_product_type"], "complaint");

    let result = client
        .call_tool(
            CallToolRequestParams::new("orsgraph_casebuilder_matter")
                .with_arguments(object!({ "matter_id": "matter:test" })),
        )
        .await?;
    assert_eq!(result.is_error, Some(true));
    let structured = result
        .structured_content
        .expect("structured casebuilder auth error");
    assert_eq!(
        structured["error"]["code"],
        "casebuilder_auth_not_configured"
    );

    client.cancel().await?;
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn mcp_stdio_casebuilder_matter_uses_configured_api_key() -> anyhow::Result<()> {
    let api_base_url = spawn_test_api().await?;
    let api = OrsGraphApiClient::new(api_base_url, Duration::from_secs(3))
        .map_err(|error| anyhow::anyhow!(error))?
        .with_api_key("secret");
    let server = OrsGraphMcpServer::new(api);

    let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
    let server_handle = tokio::spawn(async move {
        let service = server.serve(server_transport).await?;
        service.waiting().await?;
        anyhow::Ok(())
    });

    let client = TestClient.serve(client_transport).await?;
    let result = client
        .call_tool(
            CallToolRequestParams::new("orsgraph_casebuilder_matter")
                .with_arguments(object!({ "matter_id": "matter:test" })),
        )
        .await?;

    assert_eq!(result.is_error, Some(false));
    let structured = result
        .structured_content
        .expect("structured casebuilder result");
    assert_eq!(structured["endpoint"], "matters/matter:test");
    assert_eq!(structured["body"]["matter_id"], "matter:test");
    assert_eq!(structured["body"]["authorized"], true);

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

    async fn stats() -> Json<Value> {
        Json(json!({
            "statutes": 42,
            "sources": 3,
            "rules": 7
        }))
    }

    async fn sources(Query(params): Query<HashMap<String, String>>) -> Json<Value> {
        Json(json!({
            "query": {
                "q": params.get("q").cloned(),
                "limit": params.get("limit").and_then(|value| value.parse::<u32>().ok()).unwrap_or(25),
                "offset": params.get("offset").and_then(|value| value.parse::<u32>().ok()).unwrap_or(0)
            },
            "total": 1,
            "items": [{
                "source_id": "utcr_2025",
                "title": "2025 Uniform Trial Court Rules"
            }]
        }))
    }

    async fn source(Path(source_id): Path<String>) -> Json<Value> {
        Json(json!({
            "source": {
                "source_id": source_id,
                "title": "Source fixture"
            },
            "related_sources": []
        }))
    }

    async fn rules_registry() -> Json<Value> {
        Json(json!({
            "sources": [],
            "authorities": [{
                "authority_document_id": "or:linn:slr:2026",
                "title": "Linn County SLR 2026"
            }]
        }))
    }

    async fn rules_applicable(Query(params): Query<HashMap<String, String>>) -> Json<Value> {
        Json(json!({
            "jurisdiction": params.get("jurisdiction").cloned().unwrap_or_default(),
            "filing_date": params.get("date").cloned().unwrap_or_default(),
            "work_product_type": params.get("type").cloned().unwrap_or_default(),
            "utcr": {
                "edition_year": 2025
            },
            "slr_edition": null,
            "currentness_warnings": []
        }))
    }

    async fn matter(Path(matter_id): Path<String>, headers: HeaderMap) -> impl IntoResponse {
        let authorized = headers
            .get("x-api-key")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value == "secret");
        if !authorized {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "missing api key"
                })),
            );
        }
        (
            StatusCode::OK,
            Json(json!({
                "matter_id": matter_id,
                "name": "Test Matter",
                "authorized": true
            })),
        )
    }

    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/search", get(search))
        .route("/api/v1/stats", get(stats))
        .route("/api/v1/sources", get(sources))
        .route("/api/v1/sources/{source_id}", get(source))
        .route("/api/v1/rules/registry", get(rules_registry))
        .route("/api/v1/rules/applicable", get(rules_applicable))
        .route("/api/v1/matters/{matter_id}", get(matter));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("test API server");
    });

    Ok(format!("http://{addr}/api/v1"))
}
