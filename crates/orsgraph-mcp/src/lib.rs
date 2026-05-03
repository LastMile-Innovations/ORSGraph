use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;
use url::Url;

pub mod streamable_http;

const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

#[derive(Debug, Clone)]
pub struct OrsGraphApiClient {
    base_url: Url,
    http: reqwest::Client,
}

impl OrsGraphApiClient {
    pub fn new(base_url: impl AsRef<str>, timeout: Duration) -> Result<Self, String> {
        let base_url = normalize_api_base_url(base_url.as_ref())?;
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| format!("failed to build HTTP client: {error}"))?;

        Ok(Self { base_url, http })
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    async fn get_json<Q: Serialize + ?Sized>(
        &self,
        endpoint: &str,
        query: Option<&Q>,
    ) -> Result<ApiEnvelope, String> {
        let url = self
            .base_url
            .join(endpoint)
            .map_err(|error| format!("invalid API endpoint {endpoint}: {error}"))?;
        let mut request = self.http.get(url.clone());
        if let Some(query) = query {
            request = request.query(query);
        }

        let response = request
            .send()
            .await
            .map_err(|error| format!("ORSGraph API request failed for {endpoint}: {error}"))?;
        let status = response.status();
        let body = response.json::<Value>().await.map_err(|error| {
            format!("ORSGraph API returned non-JSON response for {endpoint}: {error}")
        })?;

        if !status.is_success() {
            return Err(format!(
                "ORSGraph API returned HTTP {} for {endpoint}: {}",
                status.as_u16(),
                body
            ));
        }

        Ok(ApiEnvelope {
            endpoint: endpoint.to_string(),
            status: status.as_u16(),
            ok: true,
            body,
        })
    }
}

#[derive(Clone)]
pub struct OrsGraphMcpServer {
    api: OrsGraphApiClient,
    tool_router: ToolRouter<Self>,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for OrsGraphMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(
            Implementation::new("orsgraph-mcp", env!("CARGO_PKG_VERSION"))
                .with_title("ORSGraph MCP")
                .with_description("Read-only MCP tools for the ORSGraph API."),
        )
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
        .with_instructions(
            "Use these read-only tools to inspect ORSGraph health, search legal authority, open a citation, fetch statute detail, and inspect graph neighborhoods."
                .to_string(),
        )
    }
}

#[tool_router(router = tool_router)]
impl OrsGraphMcpServer {
    pub fn new(api: OrsGraphApiClient) -> Self {
        Self {
            api,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "orsgraph_server_info",
        description = "Return this MCP server's current ORSGraph API target and read-only policy."
    )]
    pub async fn server_info(&self) -> Result<CallToolResult, McpError> {
        structured_success(json!({
            "server": "orsgraph-mcp",
            "protocolVersion": ProtocolVersion::V_2025_11_25.as_str(),
            "transport": "stdio",
            "apiBaseUrl": self.api.base_url().as_str(),
            "readOnly": true,
            "tools": [
                "orsgraph_server_info",
                "orsgraph_health",
                "orsgraph_search",
                "orsgraph_open",
                "orsgraph_get_statute",
                "orsgraph_graph_neighborhood"
            ]
        }))
    }

    #[tool(
        name = "orsgraph_health",
        description = "Read ORSGraph API and Neo4j health status."
    )]
    pub async fn health(&self) -> Result<CallToolResult, McpError> {
        self.api_tool("health", None::<&EmptyQuery>).await
    }

    #[tool(
        name = "orsgraph_search",
        description = "Search ORSGraph legal authority using the existing /api/v1/search endpoint."
    )]
    pub async fn search(
        &self,
        Parameters(params): Parameters<SearchToolParams>,
    ) -> Result<CallToolResult, McpError> {
        match params.validated() {
            Ok(query) => self.api_tool("search", Some(&query)).await,
            Err(error) => structured_tool_error("invalid_search_request", error),
        }
    }

    #[tool(
        name = "orsgraph_open",
        description = "Resolve and open the best matching authority record for a citation or query."
    )]
    pub async fn open(
        &self,
        Parameters(params): Parameters<OpenToolParams>,
    ) -> Result<CallToolResult, McpError> {
        match params.validated() {
            Ok(query) => self.api_tool("search/open", Some(&query)).await,
            Err(error) => structured_tool_error("invalid_open_request", error),
        }
    }

    #[tool(
        name = "orsgraph_get_statute",
        description = "Fetch statute detail by citation from /api/v1/statutes/{citation}."
    )]
    pub async fn get_statute(
        &self,
        Parameters(params): Parameters<GetStatuteToolParams>,
    ) -> Result<CallToolResult, McpError> {
        match params.validated_endpoint() {
            Ok(endpoint) => self.api_tool(&endpoint, None::<&EmptyQuery>).await,
            Err(error) => structured_tool_error("invalid_statute_request", error),
        }
    }

    #[tool(
        name = "orsgraph_graph_neighborhood",
        description = "Fetch a read-only graph neighborhood for a node id."
    )]
    pub async fn graph_neighborhood(
        &self,
        Parameters(params): Parameters<GraphNeighborhoodToolParams>,
    ) -> Result<CallToolResult, McpError> {
        match params.validated() {
            Ok(query) => self.api_tool("graph/neighborhood", Some(&query)).await,
            Err(error) => structured_tool_error("invalid_graph_request", error),
        }
    }

    async fn api_tool<Q: Serialize + ?Sized>(
        &self,
        endpoint: &str,
        query: Option<&Q>,
    ) -> Result<CallToolResult, McpError> {
        match self.api.get_json(endpoint, query).await {
            Ok(envelope) => structured_success(envelope),
            Err(error) => structured_tool_error("orsgraph_api_error", error),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiEnvelope {
    pub endpoint: String,
    pub status: u16,
    pub ok: bool,
    pub body: Value,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct SearchToolParams {
    /// Search text, citation, or legal concept.
    pub q: String,
    /// Optional result type filter, such as provision, definition, deadline, or penalty.
    #[serde(default)]
    pub r#type: Option<String>,
    /// Optional authority family filter, such as ORS, UTCR, ORCONST, USCONST, or CONAN.
    #[serde(default)]
    pub authority_family: Option<String>,
    /// Optional retrieval mode: auto, keyword, citation, semantic, or hybrid.
    #[serde(default)]
    pub mode: Option<String>,
    /// Optional chapter filter.
    #[serde(default)]
    pub chapter: Option<String>,
    /// Maximum results to return. Values are clamped to 1..=50.
    #[serde(default)]
    pub limit: Option<u32>,
    /// Require current authority records only.
    #[serde(default)]
    pub current_only: Option<bool>,
    /// Require source-backed records only.
    #[serde(default)]
    pub source_backed: Option<bool>,
}

#[derive(Debug, Serialize)]
struct SearchQuery {
    q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authority_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chapter: Option<String>,
    limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_backed: Option<bool>,
}

impl SearchToolParams {
    fn validated(self) -> Result<SearchQuery, String> {
        let q = required_trimmed("q", self.q, 500)?;
        let mode = validate_enum(
            "mode",
            self.mode,
            &["auto", "keyword", "citation", "semantic", "hybrid"],
        )?;
        Ok(SearchQuery {
            q,
            r#type: optional_trimmed(self.r#type, 80)?,
            authority_family: optional_trimmed(self.authority_family, 40)?,
            mode,
            chapter: optional_trimmed(self.chapter, 20)?,
            limit: clamp_limit(self.limit, 10, 50),
            current_only: self.current_only,
            source_backed: self.source_backed,
        })
    }
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct OpenToolParams {
    /// Citation or search text to resolve.
    pub q: String,
    /// Optional authority family filter, such as ORS, UTCR, ORCONST, USCONST, or CONAN.
    #[serde(default)]
    pub authority_family: Option<String>,
}

#[derive(Debug, Serialize)]
struct OpenQuery {
    q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    authority_family: Option<String>,
}

impl OpenToolParams {
    fn validated(self) -> Result<OpenQuery, String> {
        Ok(OpenQuery {
            q: required_trimmed("q", self.q, 500)?,
            authority_family: optional_trimmed(self.authority_family, 40)?,
        })
    }
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct GetStatuteToolParams {
    /// Statute citation, for example ORS 90.100.
    pub citation: String,
}

impl GetStatuteToolParams {
    fn validated_endpoint(self) -> Result<String, String> {
        let citation = required_trimmed("citation", self.citation, 120)?;
        Ok(format!(
            "statutes/{}",
            utf8_percent_encode(&citation, PATH_SEGMENT_ENCODE_SET)
        ))
    }
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct GraphNeighborhoodToolParams {
    /// Graph node id to inspect.
    pub id: String,
    /// Traversal depth. Values are clamped to 1..=3.
    #[serde(default)]
    pub depth: Option<u32>,
    /// Maximum nodes/edges to return. Values are clamped to 1..=50.
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
struct GraphNeighborhoodQuery {
    id: String,
    depth: u32,
    limit: u32,
}

impl GraphNeighborhoodToolParams {
    fn validated(self) -> Result<GraphNeighborhoodQuery, String> {
        Ok(GraphNeighborhoodQuery {
            id: required_trimmed("id", self.id, 300)?,
            depth: clamp_limit(self.depth, 1, 3),
            limit: clamp_limit(self.limit, 25, 50),
        })
    }
}

#[derive(Debug, Serialize)]
struct EmptyQuery {}

pub fn normalize_api_base_url(raw: &str) -> Result<Url, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("ORSGRAPH_API_BASE_URL cannot be empty".to_string());
    }

    let mut url = Url::parse(trimmed)
        .map_err(|error| format!("ORSGRAPH_API_BASE_URL must be an absolute URL: {error}"))?;
    match url.scheme() {
        "http" | "https" => {}
        scheme => return Err(format!("unsupported ORSGraph API URL scheme: {scheme}")),
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(
            "ORSGRAPH_API_BASE_URL must not include query strings or fragments".to_string(),
        );
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }

    Ok(url)
}

fn required_trimmed(field: &str, value: String, max_len: usize) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field} is required"));
    }
    if value.len() > max_len {
        return Err(format!("{field} must be {max_len} bytes or less"));
    }
    Ok(value.to_string())
}

fn optional_trimmed(value: Option<String>, max_len: usize) -> Result<Option<String>, String> {
    value
        .map(|value| {
            let value = value.trim();
            if value.is_empty() {
                Ok(None)
            } else if value.len() > max_len {
                Err(format!("optional value must be {max_len} bytes or less"))
            } else {
                Ok(Some(value.to_string()))
            }
        })
        .transpose()
        .map(Option::flatten)
}

fn validate_enum(
    field: &str,
    value: Option<String>,
    allowed: &[&str],
) -> Result<Option<String>, String> {
    let Some(value) = optional_trimmed(value, 40)? else {
        return Ok(None);
    };
    let normalized = value.to_ascii_lowercase();
    if allowed.contains(&normalized.as_str()) {
        Ok(Some(normalized))
    } else {
        Err(format!("{field} must be one of {}", allowed.join(", ")))
    }
}

fn clamp_limit(value: Option<u32>, default: u32, max: u32) -> u32 {
    value.unwrap_or(default).clamp(1, max)
}

fn structured_success(value: impl Serialize) -> Result<CallToolResult, McpError> {
    serde_json::to_value(value)
        .map(CallToolResult::structured)
        .map_err(|error| {
            McpError::internal_error(
                "failed to serialize tool result",
                Some(json!({ "reason": error.to_string() })),
            )
        })
}

fn structured_tool_error(
    code: &str,
    message: impl Into<String>,
) -> Result<CallToolResult, McpError> {
    structured_error_value(json!({
        "ok": false,
        "error": {
            "code": code,
            "message": message.into()
        }
    }))
}

fn structured_error_value(value: Value) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::structured_error(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_base_url_with_trailing_slash() {
        let url = normalize_api_base_url("http://127.0.0.1:8080/api/v1").unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:8080/api/v1/");
    }

    #[test]
    fn rejects_base_url_with_query() {
        let error = normalize_api_base_url("http://127.0.0.1:8080/api/v1?x=1").unwrap_err();
        assert!(error.contains("query strings"));
    }

    #[test]
    fn validates_and_clamps_search_request() {
        let query = SearchToolParams {
            q: "  ORS 90.100 ".to_string(),
            r#type: Some(" provision ".to_string()),
            authority_family: Some(" ors ".to_string()),
            mode: Some("HYBRID".to_string()),
            chapter: None,
            limit: Some(500),
            current_only: Some(true),
            source_backed: None,
        }
        .validated()
        .unwrap();

        assert_eq!(query.q, "ORS 90.100");
        assert_eq!(query.r#type.as_deref(), Some("provision"));
        assert_eq!(query.mode.as_deref(), Some("hybrid"));
        assert_eq!(query.limit, 50);
    }

    #[test]
    fn encodes_statute_citation_path_segment() {
        let endpoint = GetStatuteToolParams {
            citation: "ORS 90.100/unsafe".to_string(),
        }
        .validated_endpoint()
        .unwrap();

        assert_eq!(endpoint, "statutes/ORS%2090.100%2Funsafe");
    }

    #[test]
    fn graph_limits_are_clamped() {
        let query = GraphNeighborhoodToolParams {
            id: "node-1".to_string(),
            depth: Some(10),
            limit: Some(500),
        }
        .validated()
        .unwrap();

        assert_eq!(query.depth, 3);
        assert_eq!(query.limit, 50);
    }
}
