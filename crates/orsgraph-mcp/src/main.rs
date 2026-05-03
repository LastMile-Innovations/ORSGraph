use orsgraph_mcp::{
    OrsGraphApiClient, OrsGraphMcpServer,
    streamable_http::{
        JwtAuthRuntimeConfig, RateLimitRuntimeConfig, StreamableHttpRuntimeConfig,
        default_allowed_origins, parse_socket_addr, serve_streamable_http, split_csv,
    },
};
use rmcp::{ServiceExt, transport::stdio};
use std::{env, time::Duration};
use tracing_subscriber::EnvFilter;

const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:8080/api/v1";
const DEFAULT_TIMEOUT_MS: u64 = 15_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportMode {
    Stdio,
    Http,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,orsgraph_mcp=debug".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let runtime = RuntimeConfig::from_env_and_args()?;
    let timeout_ms = runtime
        .timeout_ms
        .or_else(|| {
            env::var("ORSGRAPH_MCP_REQUEST_TIMEOUT_MS")
                .ok()
                .and_then(|raw| raw.parse::<u64>().ok())
        })
        .unwrap_or(DEFAULT_TIMEOUT_MS);

    let api = OrsGraphApiClient::new(runtime.api_base_url, Duration::from_millis(timeout_ms))
        .map_err(|error| {
            anyhow::anyhow!("invalid ORSGraph MCP API client configuration: {error}")
        })?;
    let api = if let Some(api_key) = runtime.api_key {
        api.with_api_key(api_key)
    } else {
        api
    };

    match runtime.transport {
        TransportMode::Stdio => serve_stdio(api).await,
        TransportMode::Http => serve_streamable_http(api, runtime.http).await,
    }
}

async fn serve_stdio(api: OrsGraphApiClient) -> anyhow::Result<()> {
    let service = OrsGraphMcpServer::new(api)
        .serve(stdio())
        .await
        .inspect_err(|error| tracing::error!(?error, "MCP stdio server failed"))?;

    service.waiting().await?;
    Ok(())
}

#[derive(Debug)]
struct RuntimeConfig {
    transport: TransportMode,
    api_base_url: String,
    api_key: Option<String>,
    timeout_ms: Option<u64>,
    http: StreamableHttpRuntimeConfig,
}

impl RuntimeConfig {
    fn from_env_and_args() -> anyhow::Result<Self> {
        let mut transport = env::var("ORSGRAPH_MCP_TRANSPORT")
            .ok()
            .as_deref()
            .map(parse_transport)
            .transpose()?
            .unwrap_or(TransportMode::Stdio);
        let mut api_base_url =
            env::var("ORSGRAPH_API_BASE_URL").unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string());
        let mut api_key = env::var("ORSGRAPH_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let mut timeout_ms = env::var("ORSGRAPH_MCP_REQUEST_TIMEOUT_MS")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .or(Some(DEFAULT_TIMEOUT_MS));
        let mut http = StreamableHttpRuntimeConfig::local_default();

        if let Ok(bind) = env::var("ORSGRAPH_MCP_BIND") {
            http = http.with_bind(parse_socket_addr(&bind).map_err(anyhow::Error::msg)?);
        }
        if let Ok(path) = env::var("ORSGRAPH_MCP_PATH") {
            http.mcp_path = path;
        }
        if let Some(hosts) = split_csv(env::var("ORSGRAPH_MCP_ALLOWED_HOSTS").ok()) {
            http.allowed_hosts = hosts;
        }
        if let Some(origins) = split_csv(env::var("ORSGRAPH_MCP_ALLOWED_ORIGINS").ok()) {
            http.allowed_origins = origins;
        }
        if let Ok(token) = env::var("ORSGRAPH_MCP_BEARER_TOKEN") {
            http.bearer_token = Some(token);
        }
        if let Some(jwt) = jwt_from_env()? {
            http.jwt_auth = Some(jwt);
        }
        if let Ok(resource) = env::var("ORSGRAPH_MCP_OAUTH_RESOURCE") {
            http.oauth_resource = Some(resource);
        }
        if let Some(servers) = split_csv(env::var("ORSGRAPH_MCP_AUTHORIZATION_SERVERS").ok()) {
            http.oauth_authorization_servers = servers;
        }
        if let Some(scopes) = split_csv(env::var("ORSGRAPH_MCP_OAUTH_SCOPES").ok()) {
            http.oauth_scopes = scopes;
        }
        if let Ok(value) = env::var("ORSGRAPH_MCP_STATEFUL") {
            http.stateful_mode = parse_bool(&value)?;
        }
        if let Ok(value) = env::var("ORSGRAPH_MCP_JSON_RESPONSE") {
            http.json_response = parse_bool(&value)?;
        }
        if let Ok(value) = env::var("ORSGRAPH_MCP_RATE_LIMIT_REQUESTS") {
            let requests = value.parse::<u64>()?;
            set_rate_limit_requests(&mut http, requests);
        }
        if let Ok(value) = env::var("ORSGRAPH_MCP_RATE_LIMIT_WINDOW_SECS") {
            let seconds = value.parse::<u64>()?;
            http.rate_limit
                .get_or_insert_with(RateLimitRuntimeConfig::default)
                .per = Duration::from_secs(seconds);
        }
        if let Ok(value) = env::var("ORSGRAPH_MCP_RATE_LIMIT_ENABLED") {
            if parse_bool(&value)? {
                http.rate_limit
                    .get_or_insert_with(RateLimitRuntimeConfig::default);
            } else {
                http.rate_limit = None;
            }
        }

        let mut args = env::args().skip(1).peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--stdio" => transport = TransportMode::Stdio,
                "--http" => transport = TransportMode::Http,
                "--transport" => {
                    let value = required_arg("--transport", args.next())?;
                    transport = parse_transport(&value)?;
                }
                "--api-base-url" => api_base_url = required_arg("--api-base-url", args.next())?,
                "--api-key" => {
                    api_key = Some(required_arg("--api-key", args.next())?);
                }
                "--timeout-ms" => {
                    let value = required_arg("--timeout-ms", args.next())?;
                    timeout_ms = Some(value.parse::<u64>()?);
                }
                "--bind" => {
                    let value = required_arg("--bind", args.next())?;
                    http = http.with_bind(parse_socket_addr(&value).map_err(anyhow::Error::msg)?);
                }
                "--mcp-path" => http.mcp_path = required_arg("--mcp-path", args.next())?,
                "--allowed-host" => {
                    http.allowed_hosts
                        .push(required_arg("--allowed-host", args.next())?);
                }
                "--allowed-origin" => {
                    http.allowed_origins
                        .push(required_arg("--allowed-origin", args.next())?);
                }
                "--bearer-token" => {
                    http.bearer_token = Some(required_arg("--bearer-token", args.next())?);
                }
                "--jwt-issuer" => {
                    let issuer = required_arg("--jwt-issuer", args.next())?;
                    http.jwt_auth.get_or_insert_with(default_jwt_config).issuer = issuer;
                }
                "--jwt-audience" => {
                    let audience = required_arg("--jwt-audience", args.next())?;
                    http.jwt_auth
                        .get_or_insert_with(default_jwt_config)
                        .audience = audience;
                }
                "--jwks-uri" => {
                    let jwks_uri = required_arg("--jwks-uri", args.next())?;
                    http.jwt_auth
                        .get_or_insert_with(default_jwt_config)
                        .jwks_uri = Some(jwks_uri);
                }
                "--required-scope" => {
                    let scope = required_arg("--required-scope", args.next())?;
                    http.jwt_auth
                        .get_or_insert_with(default_jwt_config)
                        .required_scopes
                        .push(scope);
                }
                "--oauth-resource" => {
                    http.oauth_resource = Some(required_arg("--oauth-resource", args.next())?);
                }
                "--authorization-server" => {
                    http.oauth_authorization_servers
                        .push(required_arg("--authorization-server", args.next())?);
                }
                "--oauth-scope" => {
                    http.oauth_scopes
                        .push(required_arg("--oauth-scope", args.next())?);
                }
                "--rate-limit-requests" => {
                    let value = required_arg("--rate-limit-requests", args.next())?;
                    set_rate_limit_requests(&mut http, value.parse::<u64>()?);
                }
                "--rate-limit-window-secs" => {
                    let value = required_arg("--rate-limit-window-secs", args.next())?;
                    http.rate_limit
                        .get_or_insert_with(RateLimitRuntimeConfig::default)
                        .per = Duration::from_secs(value.parse::<u64>()?);
                }
                "--disable-rate-limit" => http.rate_limit = None,
                "--stateless" => http.stateful_mode = false,
                "--stateful" => http.stateful_mode = true,
                "--json-response" => http.json_response = true,
                unknown => return Err(anyhow::anyhow!("unknown argument: {unknown}")),
            }
        }

        if http.allowed_origins.is_empty() {
            http.allowed_origins = default_allowed_origins(http.bind);
        }

        Ok(Self {
            transport,
            api_base_url,
            api_key,
            timeout_ms,
            http,
        })
    }
}

fn jwt_from_env() -> anyhow::Result<Option<JwtAuthRuntimeConfig>> {
    let issuer = env::var("ORSGRAPH_MCP_JWT_ISSUER")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let audience = env::var("ORSGRAPH_MCP_JWT_AUDIENCE")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let jwks_uri = env::var("ORSGRAPH_MCP_JWKS_URI")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let required_scopes =
        split_csv(env::var("ORSGRAPH_MCP_REQUIRED_SCOPES").ok()).unwrap_or_default();

    match (issuer, audience) {
        (None, None) if jwks_uri.is_none() && required_scopes.is_empty() => Ok(None),
        (Some(issuer), Some(audience)) => Ok(Some(JwtAuthRuntimeConfig {
            issuer,
            audience,
            jwks_uri,
            required_scopes,
        })),
        _ => Err(anyhow::anyhow!(
            "JWT auth requires both ORSGRAPH_MCP_JWT_ISSUER and ORSGRAPH_MCP_JWT_AUDIENCE"
        )),
    }
}

fn default_jwt_config() -> JwtAuthRuntimeConfig {
    JwtAuthRuntimeConfig {
        issuer: String::new(),
        audience: String::new(),
        jwks_uri: None,
        required_scopes: Vec::new(),
    }
}

fn set_rate_limit_requests(http: &mut StreamableHttpRuntimeConfig, requests: u64) {
    if requests == 0 {
        http.rate_limit = None;
    } else {
        http.rate_limit
            .get_or_insert_with(RateLimitRuntimeConfig::default)
            .requests = requests;
    }
}

fn parse_transport(value: &str) -> anyhow::Result<TransportMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "stdio" => Ok(TransportMode::Stdio),
        "http" | "streamable-http" | "streamable_http" => Ok(TransportMode::Http),
        other => Err(anyhow::anyhow!(
            "invalid transport {other:?}; expected stdio or http"
        )),
    }
}

fn parse_bool(value: &str) -> anyhow::Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(anyhow::anyhow!("invalid boolean value {other:?}")),
    }
}

fn required_arg(name: &str, value: Option<String>) -> anyhow::Result<String> {
    value.ok_or_else(|| anyhow::anyhow!("{name} requires a value"))
}

fn print_help() {
    println!(
        "ORSGraph MCP\n\n\
         Usage:\n  \
           cargo run -p orsgraph-mcp -- --stdio\n  \
           cargo run -p orsgraph-mcp -- --http --bind 127.0.0.1:8090\n\n\
         Options:\n  \
           --stdio                         Run MCP over stdio (default)\n  \
           --http                          Run MCP over Streamable HTTP\n  \
           --api-base-url <url>            ORSGraph API base URL\n  \
           --api-key <key>                 ORSGraph API service key for protected read-only endpoints\n  \
           --timeout-ms <ms>               API request timeout\n  \
           --bind <addr:port>              HTTP bind address (default 127.0.0.1:8090)\n  \
           --mcp-path <path>               MCP endpoint path (default /mcp)\n  \
           --allowed-host <host>           Add an allowed Host value\n  \
           --allowed-origin <origin>       Add an allowed Origin value\n  \
           --bearer-token <token>          Require Authorization: Bearer <token> on /mcp\n  \
           --jwt-issuer <url>              Validate MCP HTTP bearer JWTs from this issuer\n  \
           --jwt-audience <aud>            Required JWT audience for this MCP resource\n  \
           --jwks-uri <url>                Optional explicit JWKS URL\n  \
           --required-scope <scope>        Require a JWT scope; repeatable\n  \
           --oauth-resource <url>          Canonical OAuth resource URL for metadata/challenges\n  \
           --authorization-server <url>    OAuth authorization server metadata issuer; repeatable\n  \
           --oauth-scope <scope>           Advertise supported OAuth scope; repeatable\n  \
           --rate-limit-requests <n>       Max /mcp requests per window (default 120; 0 disables)\n  \
           --rate-limit-window-secs <s>    Rate-limit window in seconds (default 60)\n  \
           --disable-rate-limit            Disable /mcp rate limiting\n  \
           --stateless                     Disable HTTP sessions\n  \
           --json-response                 Return JSON directly in stateless mode\n"
    );
}
