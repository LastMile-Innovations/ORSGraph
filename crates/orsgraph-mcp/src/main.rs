use orsgraph_mcp::{
    OrsGraphApiClient, OrsGraphMcpServer,
    streamable_http::{
        StreamableHttpRuntimeConfig, default_allowed_origins, parse_socket_addr,
        serve_streamable_http, split_csv,
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
        if let Ok(value) = env::var("ORSGRAPH_MCP_STATEFUL") {
            http.stateful_mode = parse_bool(&value)?;
        }
        if let Ok(value) = env::var("ORSGRAPH_MCP_JSON_RESPONSE") {
            http.json_response = parse_bool(&value)?;
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
            timeout_ms,
            http,
        })
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
           --timeout-ms <ms>               API request timeout\n  \
           --bind <addr:port>              HTTP bind address (default 127.0.0.1:8090)\n  \
           --mcp-path <path>               MCP endpoint path (default /mcp)\n  \
           --allowed-host <host>           Add an allowed Host value\n  \
           --allowed-origin <origin>       Add an allowed Origin value\n  \
           --bearer-token <token>          Require Authorization: Bearer <token> on /mcp\n  \
           --stateless                     Disable HTTP sessions\n  \
           --json-response                 Return JSON directly in stateless mode\n"
    );
}
