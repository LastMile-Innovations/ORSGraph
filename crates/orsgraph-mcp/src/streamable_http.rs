use crate::{OrsGraphApiClient, OrsGraphMcpServer};
use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use serde::Serialize;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

const DEFAULT_MCP_PATH: &str = "/mcp";
const DEFAULT_HTTP_BIND: &str = "127.0.0.1:8090";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamableHttpRuntimeConfig {
    pub bind: SocketAddr,
    pub mcp_path: String,
    pub allowed_hosts: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub bearer_token: Option<String>,
    pub stateful_mode: bool,
    pub json_response: bool,
    pub sse_keep_alive: Option<Duration>,
    pub sse_retry: Option<Duration>,
}

impl StreamableHttpRuntimeConfig {
    pub fn local_default() -> Self {
        Self {
            bind: DEFAULT_HTTP_BIND
                .parse()
                .expect("valid default bind address"),
            mcp_path: DEFAULT_MCP_PATH.to_string(),
            allowed_hosts: vec![
                "localhost".to_string(),
                "127.0.0.1".to_string(),
                "::1".to_string(),
            ],
            allowed_origins: vec![
                "http://localhost:8090".to_string(),
                "http://127.0.0.1:8090".to_string(),
            ],
            bearer_token: None,
            stateful_mode: true,
            json_response: false,
            sse_keep_alive: Some(Duration::from_secs(15)),
            sse_retry: Some(Duration::from_secs(3)),
        }
    }

    pub fn with_bind(mut self, bind: SocketAddr) -> Self {
        self.bind = bind;
        if self.allowed_origins == ["http://localhost:8090", "http://127.0.0.1:8090"] {
            self.allowed_origins = default_allowed_origins(bind);
        }
        self
    }

    pub fn normalized_mcp_path(&self) -> String {
        normalize_mcp_path(&self.mcp_path)
    }

    pub fn validate(&self) -> Result<(), String> {
        let path = self.normalized_mcp_path();
        if path == "/" {
            return Err("ORSGRAPH_MCP_PATH must not be /".to_string());
        }
        if !self.is_loopback_bind() && self.bearer_token.as_deref().unwrap_or("").is_empty() {
            return Err(
                "Streamable HTTP on a non-loopback bind requires ORSGRAPH_MCP_BEARER_TOKEN"
                    .to_string(),
            );
        }
        Ok(())
    }

    pub fn is_loopback_bind(&self) -> bool {
        match self.bind.ip() {
            IpAddr::V4(ip) => ip.is_loopback(),
            IpAddr::V6(ip) => ip.is_loopback(),
        }
    }
}

#[derive(Debug, Clone)]
struct HttpAppState {
    api_base_url: String,
    bind: SocketAddr,
    mcp_path: String,
    auth_required: bool,
    stateful_mode: bool,
    json_response: bool,
    allowed_hosts: Vec<String>,
    allowed_origins: Vec<String>,
}

#[derive(Debug, Serialize)]
struct HealthzResponse {
    ok: bool,
    server: &'static str,
    transport: &'static str,
    bind: String,
    mcp_path: String,
    api_base_url: String,
    auth_required: bool,
    stateful_mode: bool,
    json_response: bool,
    allowed_hosts: Vec<String>,
    allowed_origins: Vec<String>,
}

#[derive(Clone)]
struct BearerAuth {
    token: Arc<str>,
}

pub fn streamable_http_router(
    api: OrsGraphApiClient,
    config: StreamableHttpRuntimeConfig,
    cancellation_token: CancellationToken,
) -> Result<Router, String> {
    config.validate()?;

    let mcp_path = config.normalized_mcp_path();
    let server_config = StreamableHttpServerConfig::default()
        .with_allowed_hosts(config.allowed_hosts.clone())
        .with_allowed_origins(config.allowed_origins.clone())
        .with_stateful_mode(config.stateful_mode)
        .with_json_response(config.json_response)
        .with_sse_keep_alive(config.sse_keep_alive)
        .with_sse_retry(config.sse_retry)
        .with_cancellation_token(cancellation_token);

    let service_api = api.clone();
    let mcp_service: StreamableHttpService<OrsGraphMcpServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(OrsGraphMcpServer::new(service_api.clone())),
            LocalSessionManager::default().into(),
            server_config,
        );

    let mut mcp_router = Router::new().nest_service(&mcp_path, mcp_service);
    if let Some(token) = config
        .bearer_token
        .as_deref()
        .filter(|token| !token.is_empty())
    {
        mcp_router = mcp_router.layer(middleware::from_fn_with_state(
            BearerAuth {
                token: Arc::from(token.to_string()),
            },
            bearer_auth_middleware,
        ));
    }

    let app_state = HttpAppState {
        api_base_url: api.base_url().as_str().to_string(),
        bind: config.bind,
        mcp_path,
        auth_required: config
            .bearer_token
            .as_deref()
            .is_some_and(|token| !token.is_empty()),
        stateful_mode: config.stateful_mode,
        json_response: config.json_response,
        allowed_hosts: config.allowed_hosts,
        allowed_origins: config.allowed_origins,
    };

    Ok(Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .merge(mcp_router)
        .with_state(app_state))
}

pub async fn serve_streamable_http(
    api: OrsGraphApiClient,
    config: StreamableHttpRuntimeConfig,
) -> anyhow::Result<()> {
    config
        .validate()
        .map_err(|error| anyhow::anyhow!("invalid Streamable HTTP MCP configuration: {error}"))?;
    let bind = config.bind;
    let mcp_path = config.normalized_mcp_path();
    let auth_required = config
        .bearer_token
        .as_deref()
        .is_some_and(|token| !token.is_empty());
    let cancellation_token = CancellationToken::new();
    let app = streamable_http_router(api, config, cancellation_token.child_token())
        .map_err(|error| anyhow::anyhow!(error))?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    let local_addr = listener.local_addr()?;

    tracing::info!(
        bind = %local_addr,
        mcp_path = %mcp_path,
        auth_required,
        "starting ORSGraph MCP Streamable HTTP server"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            cancellation_token.cancel();
            tracing::info!("stopping ORSGraph MCP Streamable HTTP server");
        })
        .await?;
    Ok(())
}

async fn index(State(state): State<HttpAppState>) -> impl IntoResponse {
    Json(HealthzResponse {
        ok: true,
        server: "orsgraph-mcp",
        transport: "streamable-http",
        bind: state.bind.to_string(),
        mcp_path: state.mcp_path,
        api_base_url: state.api_base_url,
        auth_required: state.auth_required,
        stateful_mode: state.stateful_mode,
        json_response: state.json_response,
        allowed_hosts: state.allowed_hosts,
        allowed_origins: state.allowed_origins,
    })
}

async fn healthz(State(state): State<HttpAppState>) -> impl IntoResponse {
    Json(HealthzResponse {
        ok: true,
        server: "orsgraph-mcp",
        transport: "streamable-http",
        bind: state.bind.to_string(),
        mcp_path: state.mcp_path,
        api_base_url: state.api_base_url,
        auth_required: state.auth_required,
        stateful_mode: state.stateful_mode,
        json_response: state.json_response,
        allowed_hosts: state.allowed_hosts,
        allowed_origins: state.allowed_origins,
    })
}

async fn bearer_auth_middleware(
    State(auth): State<BearerAuth>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    let is_authorized = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| constant_time_eq(token.as_bytes(), auth.token.as_bytes()));

    if is_authorized {
        return next.run(request).await;
    }

    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Bearer realm=\"orsgraph-mcp\"")],
        "Unauthorized: missing or invalid bearer token",
    )
        .into_response()
}

pub fn parse_socket_addr(raw: &str) -> Result<SocketAddr, String> {
    raw.parse::<SocketAddr>()
        .map_err(|error| format!("invalid socket address {raw:?}: {error}"))
}

pub fn normalize_mcp_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return DEFAULT_MCP_PATH.to_string();
    }
    let mut path = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };
    while path.len() > 1 && path.ends_with('/') {
        path.pop();
    }
    path
}

pub fn split_csv(raw: Option<String>) -> Option<Vec<String>> {
    raw.map(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect()
    })
}

pub fn default_allowed_origins(bind: SocketAddr) -> Vec<String> {
    let port = bind.port();
    vec![
        format!("http://localhost:{port}"),
        format!("http://127.0.0.1:{port}"),
    ]
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut diff = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        diff |= usize::from(left_byte ^ right_byte);
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_mcp_path() {
        assert_eq!(normalize_mcp_path("mcp/"), "/mcp");
        assert_eq!(normalize_mcp_path("/mcp"), "/mcp");
        assert_eq!(normalize_mcp_path(""), "/mcp");
    }

    #[test]
    fn rejects_remote_bind_without_auth() {
        let config = StreamableHttpRuntimeConfig::local_default()
            .with_bind(parse_socket_addr("0.0.0.0:8090").unwrap());
        let error = config.validate().unwrap_err();
        assert!(error.contains("ORSGRAPH_MCP_BEARER_TOKEN"));
    }

    #[test]
    fn accepts_remote_bind_with_auth() {
        let mut config = StreamableHttpRuntimeConfig::local_default()
            .with_bind(parse_socket_addr("0.0.0.0:8090").unwrap());
        config.bearer_token = Some("secret".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn constant_time_eq_checks_length_and_bytes() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secrex"));
        assert!(!constant_time_eq(b"secret", b"secret-extra"));
    }
}
