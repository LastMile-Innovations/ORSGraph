use crate::{OrsGraphApiClient, OrsGraphMcpServer};
use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation, decode, decode_header,
    jwk::{AlgorithmParameters, Jwk, JwkSet, KeyAlgorithm},
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    future::Future,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tower::{Layer, Service};
use url::Url;

const DEFAULT_MCP_PATH: &str = "/mcp";
const DEFAULT_HTTP_BIND: &str = "127.0.0.1:8090";
const JWKS_CACHE_TTL: Duration = Duration::from_secs(300);
const DEFAULT_RATE_LIMIT_REQUESTS: u64 = 120;
const DEFAULT_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamableHttpRuntimeConfig {
    pub bind: SocketAddr,
    pub mcp_path: String,
    pub allowed_hosts: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub bearer_token: Option<String>,
    pub jwt_auth: Option<JwtAuthRuntimeConfig>,
    pub oauth_resource: Option<String>,
    pub oauth_authorization_servers: Vec<String>,
    pub oauth_scopes: Vec<String>,
    pub stateful_mode: bool,
    pub json_response: bool,
    pub sse_keep_alive: Option<Duration>,
    pub sse_retry: Option<Duration>,
    pub rate_limit: Option<RateLimitRuntimeConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JwtAuthRuntimeConfig {
    pub issuer: String,
    pub audience: String,
    pub jwks_uri: Option<String>,
    pub required_scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitRuntimeConfig {
    pub requests: u64,
    pub per: Duration,
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
            jwt_auth: None,
            oauth_resource: None,
            oauth_authorization_servers: Vec::new(),
            oauth_scopes: Vec::new(),
            stateful_mode: true,
            json_response: false,
            sse_keep_alive: Some(Duration::from_secs(15)),
            sse_retry: Some(Duration::from_secs(3)),
            rate_limit: Some(RateLimitRuntimeConfig::default()),
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
        let has_static_bearer = self.bearer_token.as_deref().is_some_and(has_text);
        let has_jwt = self.jwt_auth.is_some();
        if has_static_bearer && has_jwt {
            return Err(
                "configure either ORSGRAPH_MCP_BEARER_TOKEN or ORSGRAPH_MCP_JWT_ISSUER, not both"
                    .to_string(),
            );
        }
        if let Some(jwt) = self.jwt_auth.as_ref() {
            jwt.validate()?;
        }
        if let Some(rate_limit) = self.rate_limit.as_ref() {
            rate_limit.validate()?;
        }
        if let Some(resource) = self.oauth_resource.as_deref() {
            validate_absolute_http_url("ORSGRAPH_MCP_OAUTH_RESOURCE", resource)?;
        }
        for server in &self.oauth_authorization_servers {
            validate_absolute_http_url("ORSGRAPH_MCP_AUTHORIZATION_SERVERS", server)?;
        }
        if !self.is_loopback_bind() && !has_static_bearer && !has_jwt {
            return Err(
                "Streamable HTTP on a non-loopback bind requires ORSGRAPH_MCP_BEARER_TOKEN or ORSGRAPH_MCP_JWT_ISSUER/ORSGRAPH_MCP_JWT_AUDIENCE"
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

impl JwtAuthRuntimeConfig {
    pub fn validate(&self) -> Result<(), String> {
        validate_absolute_http_url("ORSGRAPH_MCP_JWT_ISSUER", &self.issuer)?;
        if !has_text(&self.audience) {
            return Err("ORSGRAPH_MCP_JWT_AUDIENCE is required for JWT auth".to_string());
        }
        if let Some(jwks_uri) = self.jwks_uri.as_deref() {
            validate_absolute_http_url("ORSGRAPH_MCP_JWKS_URI", jwks_uri)?;
        }
        Ok(())
    }
}

impl Default for RateLimitRuntimeConfig {
    fn default() -> Self {
        Self {
            requests: DEFAULT_RATE_LIMIT_REQUESTS,
            per: DEFAULT_RATE_LIMIT_WINDOW,
        }
    }
}

impl RateLimitRuntimeConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.requests == 0 {
            return Err("ORSGRAPH_MCP_RATE_LIMIT_REQUESTS must be greater than 0".to_string());
        }
        if self.per.is_zero() {
            return Err("ORSGRAPH_MCP_RATE_LIMIT_WINDOW_SECS must be greater than 0".to_string());
        }
        Ok(())
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
    auth_mode: &'static str,
    oauth_metadata: Option<OAuthProtectedResourceMetadata>,
    oauth_metadata_url: Option<String>,
    rate_limit_enabled: bool,
    rate_limit_requests: Option<u64>,
    rate_limit_window_seconds: Option<u64>,
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
    auth_mode: &'static str,
    oauth_metadata_url: Option<String>,
    rate_limit_enabled: bool,
    rate_limit_requests: Option<u64>,
    rate_limit_window_seconds: Option<u64>,
}

#[derive(Clone)]
struct BearerAuth {
    token: Arc<str>,
}

#[derive(Clone)]
enum HttpAuth {
    StaticBearer(BearerAuth),
    Jwt(Arc<JwtAuthVerifier>),
}

#[derive(Debug, Clone, Serialize)]
struct OAuthProtectedResourceMetadata {
    resource: String,
    authorization_servers: Vec<String>,
    bearer_methods_supported: Vec<&'static str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    scopes_supported: Vec<String>,
    resource_name: &'static str,
}

#[derive(Clone)]
struct McpRateLimitLayer {
    bucket: Arc<Mutex<TokenBucket>>,
}

#[derive(Clone)]
struct McpRateLimitService<S> {
    inner: S,
    bucket: Arc<Mutex<TokenBucket>>,
}

#[derive(Debug)]
struct TokenBucket {
    config: RateLimitRuntimeConfig,
    remaining: u64,
    window_started: Instant,
}

impl McpRateLimitLayer {
    fn new(config: RateLimitRuntimeConfig) -> Self {
        Self {
            bucket: Arc::new(Mutex::new(TokenBucket::new(config))),
        }
    }
}

impl<S> Layer<S> for McpRateLimitLayer {
    type Service = McpRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        McpRateLimitService {
            inner,
            bucket: Arc::clone(&self.bucket),
        }
    }
}

impl<S> Service<Request<Body>> for McpRateLimitService<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Error: Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let decision = match self.bucket.lock() {
            Ok(mut bucket) => bucket.check(),
            Err(poisoned) => poisoned.into_inner().check(),
        };

        if let RateLimitDecision::Limited {
            retry_after_seconds,
        } = decision
        {
            let response = rate_limit_response(retry_after_seconds);
            return Box::pin(async move { Ok(response) });
        }

        let future = self.inner.call(request);
        Box::pin(future)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RateLimitDecision {
    Allowed,
    Limited { retry_after_seconds: u64 },
}

impl TokenBucket {
    fn new(config: RateLimitRuntimeConfig) -> Self {
        Self {
            remaining: config.requests,
            config,
            window_started: Instant::now(),
        }
    }

    fn check(&mut self) -> RateLimitDecision {
        let now = Instant::now();
        if now.saturating_duration_since(self.window_started) >= self.config.per {
            self.remaining = self.config.requests;
            self.window_started = now;
        }

        if self.remaining == 0 {
            return RateLimitDecision::Limited {
                retry_after_seconds: self.retry_after_seconds(),
            };
        }

        self.remaining -= 1;
        RateLimitDecision::Allowed
    }

    fn retry_after_seconds(&self) -> u64 {
        let elapsed = Instant::now().saturating_duration_since(self.window_started);
        self.config.per.saturating_sub(elapsed).as_secs().max(1)
    }
}

fn rate_limit_response(retry_after_seconds: u64) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        "Too many MCP requests; slow down and retry shortly",
    )
        .into_response();
    if let Ok(value) = HeaderValue::from_str(&retry_after_seconds.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
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

    let resource = config
        .oauth_resource
        .clone()
        .unwrap_or_else(|| local_resource_url(config.bind, &mcp_path));
    let oauth_metadata_url = oauth_metadata_url_for_resource(&resource);
    let oauth_metadata = config.jwt_auth.as_ref().map(|jwt| {
        let authorization_servers = if config.oauth_authorization_servers.is_empty() {
            vec![jwt.issuer.trim_end_matches('/').to_string()]
        } else {
            config.oauth_authorization_servers.clone()
        };
        let scopes_supported = if config.oauth_scopes.is_empty() {
            jwt.required_scopes.clone()
        } else {
            config.oauth_scopes.clone()
        };
        OAuthProtectedResourceMetadata {
            resource: resource.clone(),
            authorization_servers,
            bearer_methods_supported: vec!["header"],
            scopes_supported,
            resource_name: "ORSGraph MCP",
        }
    });

    let auth_layer = match (
        config
            .bearer_token
            .as_deref()
            .filter(|token| has_text(token)),
        config.jwt_auth.clone(),
    ) {
        (Some(token), None) => Some(HttpAuth::StaticBearer(BearerAuth {
            token: Arc::from(token.to_string()),
        })),
        (None, Some(jwt)) => Some(HttpAuth::Jwt(Arc::new(JwtAuthVerifier::new(
            jwt,
            oauth_metadata_url.clone(),
        )))),
        (None, None) => None,
        (Some(_), Some(_)) => unreachable!("validated mutually exclusive auth modes"),
    };

    let auth_mode = match auth_layer {
        Some(HttpAuth::StaticBearer(_)) => "static_bearer",
        Some(HttpAuth::Jwt(_)) => "jwt_jwks",
        None => "none",
    };
    let rate_limit = config.rate_limit.clone();

    let mut mcp_router = Router::new().nest_service(&mcp_path, mcp_service);
    if let Some(auth_layer) = auth_layer {
        mcp_router = mcp_router.layer(middleware::from_fn_with_state(
            auth_layer,
            http_auth_middleware,
        ));
    }
    if let Some(rate_limit) = rate_limit.clone() {
        mcp_router = mcp_router.layer(McpRateLimitLayer::new(rate_limit));
    }

    let app_state = HttpAppState {
        api_base_url: api.base_url().as_str().to_string(),
        bind: config.bind,
        mcp_path: mcp_path.clone(),
        auth_required: auth_mode != "none",
        stateful_mode: config.stateful_mode,
        json_response: config.json_response,
        allowed_hosts: config.allowed_hosts,
        allowed_origins: config.allowed_origins,
        auth_mode,
        oauth_metadata,
        oauth_metadata_url,
        rate_limit_enabled: rate_limit.is_some(),
        rate_limit_requests: rate_limit.as_ref().map(|config| config.requests),
        rate_limit_window_seconds: rate_limit.as_ref().map(|config| config.per.as_secs()),
    };

    let mut router = Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .merge(mcp_router);
    if app_state.oauth_metadata.is_some() {
        router = router.route(
            "/.well-known/oauth-protected-resource",
            get(oauth_protected_resource),
        );
        let path_scoped_metadata = format!("/.well-known/oauth-protected-resource{mcp_path}");
        router = router.route(&path_scoped_metadata, get(oauth_protected_resource));
    }

    Ok(router.with_state(app_state))
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
    let auth_mode = if config.jwt_auth.is_some() {
        "jwt_jwks"
    } else if config.bearer_token.as_deref().is_some_and(has_text) {
        "static_bearer"
    } else {
        "none"
    };
    let auth_required = auth_mode != "none";
    let rate_limit_enabled = config.rate_limit.is_some();
    let cancellation_token = CancellationToken::new();
    let app = streamable_http_router(api, config, cancellation_token.child_token())
        .map_err(|error| anyhow::anyhow!(error))?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    let local_addr = listener.local_addr()?;

    tracing::info!(
        bind = %local_addr,
        mcp_path = %mcp_path,
        auth_required,
        auth_mode,
        rate_limit_enabled,
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
        auth_mode: state.auth_mode,
        oauth_metadata_url: state.oauth_metadata_url,
        rate_limit_enabled: state.rate_limit_enabled,
        rate_limit_requests: state.rate_limit_requests,
        rate_limit_window_seconds: state.rate_limit_window_seconds,
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
        auth_mode: state.auth_mode,
        oauth_metadata_url: state.oauth_metadata_url,
        rate_limit_enabled: state.rate_limit_enabled,
        rate_limit_requests: state.rate_limit_requests,
        rate_limit_window_seconds: state.rate_limit_window_seconds,
    })
}

async fn oauth_protected_resource(State(state): State<HttpAppState>) -> impl IntoResponse {
    match state.oauth_metadata {
        Some(metadata) => Json(metadata).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn http_auth_middleware(
    State(auth): State<HttpAuth>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    match auth {
        HttpAuth::StaticBearer(auth) => static_bearer_auth(headers, request, next, auth).await,
        HttpAuth::Jwt(verifier) => jwt_auth(headers, request, next, verifier).await,
    }
}

async fn static_bearer_auth(
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
    auth: BearerAuth,
) -> Response {
    let is_authorized = bearer_token(&headers)
        .is_some_and(|token| constant_time_eq(token.as_bytes(), auth.token.as_bytes()));

    if is_authorized {
        next.run(request).await
    } else {
        auth_error_response(
            StatusCode::UNAUTHORIZED,
            bearer_challenge(None, &[], None, None),
            "Unauthorized: missing or invalid bearer token",
        )
    }
}

async fn jwt_auth(
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
    verifier: Arc<JwtAuthVerifier>,
) -> Response {
    let Some(token) = bearer_token(&headers) else {
        return auth_error_response(
            StatusCode::UNAUTHORIZED,
            verifier.challenge(None, None),
            "Unauthorized: missing bearer token",
        );
    };

    match verifier.verify(token).await {
        Ok(()) => next.run(request).await,
        Err(JwtAuthFailure::InvalidToken(reason)) => {
            tracing::warn!(reason, "MCP JWT validation failed");
            auth_error_response(
                StatusCode::UNAUTHORIZED,
                verifier.challenge(
                    Some("invalid_token"),
                    Some("Invalid or expired access token"),
                ),
                "Unauthorized: invalid bearer token",
            )
        }
        Err(JwtAuthFailure::InsufficientScope { required }) => auth_error_response(
            StatusCode::FORBIDDEN,
            verifier.insufficient_scope_challenge(&required),
            "Forbidden: insufficient token scope",
        ),
    }
}

fn auth_error_response(status: StatusCode, challenge: String, body: &'static str) -> Response {
    let mut response = (status, body).into_response();
    if let Ok(value) = HeaderValue::from_str(&challenge) {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, value);
    }
    response
}

#[derive(Debug)]
struct JwtAuthVerifier {
    issuer: String,
    audience: String,
    jwks_uri: Option<String>,
    required_scopes: Vec<String>,
    metadata_url: Option<String>,
    client: reqwest::Client,
    discovery: RwLock<Option<CachedDiscovery>>,
    jwks: RwLock<Option<CachedJwks>>,
}

#[derive(Debug, Clone)]
struct CachedDiscovery {
    jwks_uri: String,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct CachedJwks {
    keys: Vec<Jwk>,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct AuthorizationServerMetadata {
    jwks_uri: String,
}

#[derive(Debug, Deserialize)]
struct JwtClaims {
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    scp: Option<Vec<String>>,
    #[serde(default)]
    scopes: Option<Vec<String>>,
    #[serde(flatten)]
    extra: serde_json::Map<String, Value>,
}

#[derive(Debug)]
enum JwtAuthFailure {
    InvalidToken(String),
    InsufficientScope { required: Vec<String> },
}

impl JwtAuthVerifier {
    fn new(config: JwtAuthRuntimeConfig, metadata_url: Option<String>) -> Self {
        Self {
            issuer: config.issuer.trim_end_matches('/').to_string(),
            audience: config.audience,
            jwks_uri: config.jwks_uri,
            required_scopes: config.required_scopes,
            metadata_url,
            client: reqwest::Client::new(),
            discovery: RwLock::new(None),
            jwks: RwLock::new(None),
        }
    }

    async fn verify(&self, token: &str) -> Result<(), JwtAuthFailure> {
        let header = decode_header(token)
            .map_err(|error| JwtAuthFailure::InvalidToken(format!("JWT header error: {error}")))?;
        let jwk = self
            .jwk_for_kid(header.kid.as_deref())
            .await?
            .ok_or_else(|| JwtAuthFailure::InvalidToken("No matching JWT key".to_string()))?;
        let key = DecodingKey::from_jwk(&jwk)
            .map_err(|error| JwtAuthFailure::InvalidToken(format!("JWT key error: {error}")))?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);

        let data = decode::<JwtClaims>(token, &key, &validation).map_err(|error| {
            JwtAuthFailure::InvalidToken(format!("JWT validation error: {error}"))
        })?;

        if !self.required_scopes.is_empty() {
            let granted = claim_scopes(&data.claims);
            let missing: Vec<String> = self
                .required_scopes
                .iter()
                .filter(|scope| !granted.contains(scope.as_str()))
                .cloned()
                .collect();
            if !missing.is_empty() {
                return Err(JwtAuthFailure::InsufficientScope {
                    required: self.required_scopes.clone(),
                });
            }
        }

        Ok(())
    }

    fn challenge(&self, error: Option<&str>, description: Option<&str>) -> String {
        bearer_challenge(
            self.metadata_url.as_deref(),
            &self.required_scopes,
            error,
            description,
        )
    }

    fn insufficient_scope_challenge(&self, required: &[String]) -> String {
        bearer_challenge(
            self.metadata_url.as_deref(),
            required,
            Some("insufficient_scope"),
            Some("Additional scope is required for this MCP server"),
        )
    }

    async fn discovery(&self) -> Result<String, JwtAuthFailure> {
        if let Some(jwks_uri) = self.jwks_uri.as_ref().filter(|value| has_text(value)) {
            return Ok(jwks_uri.clone());
        }
        if let Some(cached) = self.discovery.read().await.as_ref() {
            if cached.expires_at > Instant::now() {
                return Ok(cached.jwks_uri.clone());
            }
        }

        let mut errors = Vec::new();
        for url in authorization_metadata_urls(&self.issuer)? {
            match self
                .client
                .get(url.as_str())
                .send()
                .await
                .and_then(|response| response.error_for_status())
            {
                Ok(response) => match response.json::<AuthorizationServerMetadata>().await {
                    Ok(metadata) if has_text(&metadata.jwks_uri) => {
                        *self.discovery.write().await = Some(CachedDiscovery {
                            jwks_uri: metadata.jwks_uri.clone(),
                            expires_at: Instant::now() + JWKS_CACHE_TTL,
                        });
                        return Ok(metadata.jwks_uri);
                    }
                    Ok(_) => errors.push(format!("{url}: missing jwks_uri")),
                    Err(error) => errors.push(format!("{url}: {error}")),
                },
                Err(error) => errors.push(format!("{url}: {error}")),
            }
        }

        Err(JwtAuthFailure::InvalidToken(format!(
            "authorization server metadata discovery failed: {}",
            errors.join("; ")
        )))
    }

    async fn jwk_for_kid(&self, kid: Option<&str>) -> Result<Option<Jwk>, JwtAuthFailure> {
        let keys = self.jwks().await?;
        Ok(keys.into_iter().find(|key| {
            let kid_matches = match kid {
                Some(kid) => key.common.key_id.as_deref() == Some(kid),
                None => true,
            };
            kid_matches
                && matches!(key.algorithm, AlgorithmParameters::RSA(_))
                && key
                    .common
                    .key_algorithm
                    .is_none_or(|algorithm| algorithm == KeyAlgorithm::RS256)
        }))
    }

    async fn jwks(&self) -> Result<Vec<Jwk>, JwtAuthFailure> {
        if let Some(cached) = self.jwks.read().await.as_ref() {
            if cached.expires_at > Instant::now() {
                return Ok(cached.keys.clone());
            }
        }

        let jwks_uri = self.discovery().await?;
        let jwks = self
            .client
            .get(jwks_uri.as_str())
            .send()
            .await
            .map_err(|error| JwtAuthFailure::InvalidToken(format!("JWKS fetch error: {error}")))?
            .error_for_status()
            .map_err(|error| JwtAuthFailure::InvalidToken(format!("JWKS HTTP error: {error}")))?
            .json::<JwkSet>()
            .await
            .map_err(|error| JwtAuthFailure::InvalidToken(format!("JWKS JSON error: {error}")))?;
        *self.jwks.write().await = Some(CachedJwks {
            keys: jwks.keys.clone(),
            expires_at: Instant::now() + JWKS_CACHE_TTL,
        });
        Ok(jwks.keys)
    }
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

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn bearer_challenge(
    metadata_url: Option<&str>,
    scopes: &[String],
    error: Option<&str>,
    description: Option<&str>,
) -> String {
    let mut parts = vec!["Bearer realm=\"orsgraph-mcp\"".to_string()];
    if let Some(error) = error.filter(|value| has_text(value)) {
        parts.push(format!("error=\"{}\"", quote_header_value(error)));
    }
    if !scopes.is_empty() {
        parts.push(format!(
            "scope=\"{}\"",
            quote_header_value(&scopes.join(" "))
        ));
    }
    if let Some(metadata_url) = metadata_url.filter(|value| has_text(value)) {
        parts.push(format!(
            "resource_metadata=\"{}\"",
            quote_header_value(metadata_url)
        ));
    }
    if let Some(description) = description.filter(|value| has_text(value)) {
        parts.push(format!(
            "error_description=\"{}\"",
            quote_header_value(description)
        ));
    }
    parts.join(", ")
}

fn quote_header_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn claim_scopes(claims: &JwtClaims) -> BTreeSet<String> {
    let mut scopes = BTreeSet::new();
    if let Some(scope) = claims.scope.as_deref() {
        scopes.extend(scope.split_whitespace().map(ToString::to_string));
    }
    if let Some(values) = claims.scp.as_ref() {
        scopes.extend(values.iter().filter(|value| has_text(value)).cloned());
    }
    if let Some(values) = claims.scopes.as_ref() {
        scopes.extend(values.iter().filter(|value| has_text(value)).cloned());
    }
    if let Some(Value::Array(values)) = claims.extra.get("permissions") {
        scopes.extend(
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| has_text(value))
                .map(ToString::to_string),
        );
    }
    scopes
}

fn authorization_metadata_urls(issuer: &str) -> Result<Vec<Url>, JwtAuthFailure> {
    let issuer_url = Url::parse(issuer).map_err(|error| {
        JwtAuthFailure::InvalidToken(format!("invalid JWT issuer URL {issuer:?}: {error}"))
    })?;
    let origin = issuer_url.origin().ascii_serialization();
    let issuer_path = issuer_url.path().trim_end_matches('/');
    let mut urls = Vec::new();

    if issuer_path.is_empty() || issuer_path == "/" {
        urls.push(format!("{origin}/.well-known/oauth-authorization-server"));
        urls.push(format!("{origin}/.well-known/openid-configuration"));
    } else {
        urls.push(format!(
            "{origin}/.well-known/oauth-authorization-server{issuer_path}"
        ));
        urls.push(format!(
            "{origin}/.well-known/openid-configuration{issuer_path}"
        ));
        urls.push(format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        ));
    }

    urls.into_iter()
        .map(|url| {
            Url::parse(&url).map_err(|error| {
                JwtAuthFailure::InvalidToken(format!("invalid discovery URL: {error}"))
            })
        })
        .collect()
}

fn validate_absolute_http_url(name: &str, raw: &str) -> Result<(), String> {
    let url = Url::parse(raw.trim()).map_err(|error| format!("{name} must be a URL: {error}"))?;
    match url.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!("{name} must use http or https, got {scheme}")),
    }
}

fn local_resource_url(bind: SocketAddr, mcp_path: &str) -> String {
    let host = match bind.ip() {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => format!("[{ip}]"),
    };
    format!(
        "http://{host}:{}{}",
        bind.port(),
        normalize_mcp_path(mcp_path)
    )
}

fn oauth_metadata_url_for_resource(resource: &str) -> Option<String> {
    let mut url = Url::parse(resource).ok()?;
    url.set_path("/.well-known/oauth-protected-resource");
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

fn has_text(value: &str) -> bool {
    !value.trim().is_empty()
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
    fn rejects_invalid_rate_limit_config() {
        let mut config = StreamableHttpRuntimeConfig::local_default();
        config.rate_limit = Some(RateLimitRuntimeConfig {
            requests: 0,
            per: Duration::from_secs(60),
        });
        let error = config.validate().unwrap_err();
        assert!(error.contains("ORSGRAPH_MCP_RATE_LIMIT_REQUESTS"));

        config.rate_limit = Some(RateLimitRuntimeConfig {
            requests: 1,
            per: Duration::ZERO,
        });
        let error = config.validate().unwrap_err();
        assert!(error.contains("ORSGRAPH_MCP_RATE_LIMIT_WINDOW_SECS"));
    }

    #[test]
    fn constant_time_eq_checks_length_and_bytes() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secrex"));
        assert!(!constant_time_eq(b"secret", b"secret-extra"));
    }
}
