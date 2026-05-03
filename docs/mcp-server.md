# ORSGraph MCP Server

`orsgraph-mcp` is a read-only Model Context Protocol server for ORSGraph. It
uses the official Rust SDK (`rmcp`) and supports both stdio and Streamable HTTP.

For the full build, smoke, Docker, Railway, client acceptance, and
troubleshooting path, start with the
[MCP End-To-End Runbook](mcp-end-to-end.md).

## Run

Start the ORSGraph API first, then launch the MCP server over stdio:

```bash
export ORSGRAPH_API_BASE_URL=http://127.0.0.1:8080/api/v1
cargo run -p orsgraph-mcp
```

The server writes logs to stderr only. Stdout is reserved for MCP JSON-RPC.

For Streamable HTTP:

```bash
export ORSGRAPH_API_BASE_URL=http://127.0.0.1:8080/api/v1
cargo run -p orsgraph-mcp -- --http --bind 127.0.0.1:8090
```

The MCP endpoint is `http://127.0.0.1:8090/mcp`. `/healthz` returns server
configuration without touching the ORSGraph API.

Streamable HTTP rate-limits `/mcp` by default to 120 requests per 60 seconds.
Use `ORSGRAPH_MCP_RATE_LIMIT_REQUESTS`, `ORSGRAPH_MCP_RATE_LIMIT_WINDOW_SECS`,
or `ORSGRAPH_MCP_RATE_LIMIT_ENABLED=false` to adjust it for a local client or a
fronted public deployment.

For containerized Streamable HTTP:

```bash
docker build -f Dockerfile.mcp -t orsgraph-mcp:local .
docker run --rm -p 8090:8090 \
  -e ORSGRAPH_API_BASE_URL=http://host.docker.internal:8080/api/v1 \
  -e ORSGRAPH_MCP_BEARER_TOKEN=local-dev-secret \
  orsgraph-mcp:local
```

The MCP container entrypoint always starts `orsgraph-mcp --http`, binds to
`0.0.0.0:$PORT` on Railway, and exposes `/healthz` for platform health checks.

## Tools

- `orsgraph_server_info` returns the configured API target and read-only policy.
- `orsgraph_health` calls `GET /api/v1/health`.
- `orsgraph_stats` calls `GET /api/v1/stats`.
- `orsgraph_search` calls `GET /api/v1/search` with typed, validated filters.
- `orsgraph_open` calls `GET /api/v1/search/open`.
- `orsgraph_get_statute` calls `GET /api/v1/statutes/{citation}`.
- `orsgraph_sources` calls `GET /api/v1/sources` with typed, validated filters.
- `orsgraph_source` calls `GET /api/v1/sources/{source_id}`.
- `orsgraph_rules_registry` calls `GET /api/v1/rules/registry`.
- `orsgraph_rule_applicability` calls `GET /api/v1/rules/applicable`.
- `orsgraph_graph_neighborhood` calls `GET /api/v1/graph/neighborhood`.
- `orsgraph_casebuilder_matters` calls `GET /api/v1/matters` only when
  `ORSGRAPH_API_KEY` is configured on the MCP server.
- `orsgraph_casebuilder_matter` calls `GET /api/v1/matters/{matter_id}` only
  when `ORSGRAPH_API_KEY` is configured on the MCP server.

All tools are read-only, clamp large limits, trim string inputs, and return
structured JSON MCP results. There is no shell execution surface and
caller Authorization headers are not forwarded to the ORSGraph API. CaseBuilder
matter lookup uses a fixed server-side `ORSGRAPH_API_KEY` service credential
when, and only when, the operator explicitly configures it.

## Client Config

Use stdio when the client should launch `orsgraph-mcp` itself. Use Streamable
HTTP when `orsgraph-mcp --http` is already running.

Start the HTTP endpoint before using any HTTP template:

```bash
export ORSGRAPH_API_BASE_URL=http://127.0.0.1:8080/api/v1
cargo run -p orsgraph-mcp -- --http --bind 127.0.0.1:8090
```

Use `ORSGRAPH_MCP_REQUEST_TIMEOUT_MS` to change the default 15 second API
request timeout.

For protected read-only API routes, including CaseBuilder matter lookup, set a
server-side ORSGraph API key:

```bash
export ORSGRAPH_API_KEY='replace-with-orsgraph-service-key'
```

Do not put end-user bearer tokens in MCP prompts or client tool arguments.

### Claude Desktop

Config file:

- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

Stdio:

```json
{
  "mcpServers": {
    "orsgraph": {
      "command": "cargo",
      "args": ["run", "-p", "orsgraph-mcp", "--", "--stdio"],
      "env": {
        "ORSGRAPH_API_BASE_URL": "http://127.0.0.1:8080/api/v1"
      }
    }
  }
}
```

For CaseBuilder matter lookup, add `"ORSGRAPH_API_KEY"` to the same `env`
object.

Streamable HTTP through a stdio bridge:

```json
{
  "mcpServers": {
    "orsgraph-http": {
      "command": "npx",
      "args": [
        "-y",
        "mcp-remote",
        "http://127.0.0.1:8090/mcp",
        "--allow-http"
      ]
    }
  }
}
```

Bearer-protected HTTP:

```json
{
  "mcpServers": {
    "orsgraph-http": {
      "command": "npx",
      "args": [
        "-y",
        "mcp-remote",
        "https://mcp.example.com/mcp",
        "--header",
        "Authorization: Bearer replace-with-token"
      ]
    }
  }
}
```

### Codex

Config file: `~/.codex/config.toml`

Stdio:

```toml
[mcp_servers.orsgraph]
command = "cargo"
args = ["run", "-p", "orsgraph-mcp", "--", "--stdio"]
cwd = "/Users/grey/ORSGraph"
startup_timeout_sec = 20
tool_timeout_sec = 60

[mcp_servers.orsgraph.env]
ORSGRAPH_API_BASE_URL = "http://127.0.0.1:8080/api/v1"
# Optional for CaseBuilder matter lookup:
# ORSGRAPH_API_KEY = "replace-with-orsgraph-service-key"
```

Streamable HTTP:

```toml
[mcp_servers.orsgraph_http]
url = "http://127.0.0.1:8090/mcp"
startup_timeout_sec = 20
tool_timeout_sec = 60
```

Bearer-protected HTTP:

```toml
[mcp_servers.orsgraph_http]
url = "https://mcp.example.com/mcp"
bearer_token_env_var = "ORSGRAPH_MCP_BEARER_TOKEN"
startup_timeout_sec = 20
tool_timeout_sec = 60
```

### Cursor

Project config: `.cursor/mcp.json`

Global config: `~/.cursor/mcp.json`

Stdio:

```json
{
  "mcpServers": {
    "orsgraph": {
      "type": "stdio",
      "command": "cargo",
      "args": ["run", "-p", "orsgraph-mcp", "--", "--stdio"],
      "env": {
        "ORSGRAPH_API_BASE_URL": "http://127.0.0.1:8080/api/v1"
      }
    }
  }
}
```

For CaseBuilder matter lookup, add `"ORSGRAPH_API_KEY"` to the same `env`
object.

Streamable HTTP:

```json
{
  "mcpServers": {
    "orsgraph-http": {
      "type": "streamable-http",
      "url": "http://127.0.0.1:8090/mcp"
    }
  }
}
```

Bearer-protected HTTP:

```json
{
  "mcpServers": {
    "orsgraph-http": {
      "type": "streamable-http",
      "url": "https://mcp.example.com/mcp",
      "headers": {
        "Authorization": "Bearer ${env:ORSGRAPH_MCP_BEARER_TOKEN}"
      }
    }
  }
}
```

### MCP Inspector

Stdio:

```bash
npx @modelcontextprotocol/inspector \
  -e ORSGRAPH_API_BASE_URL=http://127.0.0.1:8080/api/v1 \
  -- cargo run -p orsgraph-mcp -- --stdio
```

For CaseBuilder matter lookup, add
`-e ORSGRAPH_API_KEY=replace-with-orsgraph-service-key`.

Streamable HTTP with a config file:

```bash
cat > /tmp/orsgraph-mcp-inspector.json <<'JSON'
{
  "mcpServers": {
    "orsgraph-http": {
      "type": "streamable-http",
      "url": "http://127.0.0.1:8090/mcp"
    }
  }
}
JSON

npx @modelcontextprotocol/inspector \
  --config /tmp/orsgraph-mcp-inspector.json \
  --server orsgraph-http
```

Bearer-protected HTTP:

```bash
cat > /tmp/orsgraph-mcp-inspector.json <<'JSON'
{
  "mcpServers": {
    "orsgraph-http": {
      "type": "streamable-http",
      "url": "https://mcp.example.com/mcp",
      "headers": {
        "Authorization": "Bearer replace-with-token"
      }
    }
  }
}
JSON

npx @modelcontextprotocol/inspector \
  --config /tmp/orsgraph-mcp-inspector.json \
  --server orsgraph-http
```

The HTTP transport is stateful by default and supports `Mcp-Session-Id`, SSE
reconnects, `Last-Event-ID`, `DELETE` session shutdown, Host validation, Origin
validation, and `MCP-Protocol-Version` validation through `rmcp`.

## HTTP Security

Defaults are intentionally local:

- Binds to `127.0.0.1:8090`.
- Allows loopback `Host` values only.
- Allows `Origin` values for `localhost` and `127.0.0.1` on the bind port.
- Refuses non-loopback binds unless static bearer auth or JWT/JWKS auth is set.
- Rate-limits `/mcp` by default and returns `429 Too Many Requests` with
  `Retry-After` when clients exceed the configured window.
- Uses `ORSGRAPH_API_KEY` only as a server-side ORSGraph API credential; it is
  never read from MCP tool input and caller Authorization headers are not
  forwarded.

Static bearer is acceptable for local or private-network use:

```bash
export ORSGRAPH_MCP_TRANSPORT=http
export ORSGRAPH_MCP_BIND=0.0.0.0:8090
export ORSGRAPH_MCP_BEARER_TOKEN='replace-with-a-real-secret'
export ORSGRAPH_MCP_ALLOWED_HOSTS='mcp.example.com'
export ORSGRAPH_MCP_ALLOWED_ORIGINS='https://mcp.example.com'
cargo run -p orsgraph-mcp
```

Clients must send `Authorization: Bearer <token>` to `/mcp`. Leave `/healthz`
unauthenticated for local and platform health checks.

For a public endpoint, prefer JWT/JWKS validation and OAuth protected-resource
metadata:

```bash
export ORSGRAPH_MCP_TRANSPORT=http
export ORSGRAPH_MCP_BIND=0.0.0.0:8090
export ORSGRAPH_MCP_ALLOWED_HOSTS='mcp.example.com'
export ORSGRAPH_MCP_ALLOWED_ORIGINS='https://mcp.example.com'
export ORSGRAPH_MCP_OAUTH_RESOURCE='https://mcp.example.com/mcp'
export ORSGRAPH_MCP_JWT_ISSUER='https://auth.example.com'
export ORSGRAPH_MCP_JWT_AUDIENCE='https://mcp.example.com/mcp'
export ORSGRAPH_MCP_REQUIRED_SCOPES='orsgraph:mcp'
cargo run -p orsgraph-mcp
```

JWT mode:

- Validates `Authorization: Bearer <jwt>` on every `/mcp` request.
- Verifies RS256 signatures against JWKS from `ORSGRAPH_MCP_JWKS_URI` or
  issuer metadata discovery.
- Requires issuer and audience matches, so tokens for other resources are
  rejected.
- Enforces optional scopes from `ORSGRAPH_MCP_REQUIRED_SCOPES`.
- Serves OAuth protected-resource metadata at
  `/.well-known/oauth-protected-resource` and
  `/.well-known/oauth-protected-resource/mcp`.
- Adds `WWW-Authenticate` challenges with `resource_metadata` and scope hints.

Optional JWT/OAuth variables:

- `ORSGRAPH_MCP_JWKS_URI`: explicit JWKS URL when issuer discovery is not enough.
- `ORSGRAPH_MCP_AUTHORIZATION_SERVERS`: comma-separated authorization server
  metadata issuers; defaults to `ORSGRAPH_MCP_JWT_ISSUER`.
- `ORSGRAPH_MCP_OAUTH_SCOPES`: comma-separated scopes to advertise in metadata;
  defaults to `ORSGRAPH_MCP_REQUIRED_SCOPES`.

Optional rate-limit variables:

- `ORSGRAPH_MCP_RATE_LIMIT_REQUESTS`: max `/mcp` requests per window; default
  `120`; set `0` to disable when `ORSGRAPH_MCP_RATE_LIMIT_ENABLED` is unset.
- `ORSGRAPH_MCP_RATE_LIMIT_WINDOW_SECS`: rate-limit window in seconds; default
  `60`.
- `ORSGRAPH_MCP_RATE_LIMIT_ENABLED`: set `false` to disable the limiter.

## Railway Deployment

Run Streamable HTTP MCP as its own Railway service, separate from
`orsgraph-api`. Use the repository root as the service source and set the
service build to Dockerfile mode with `Dockerfile.mcp`; do not reuse the
API service healthcheck.

The checked-in profile template is
[`docs/deploy/railway-orsgraph-mcp.json`](deploy/railway-orsgraph-mcp.json).
It captures the intended service shape:

- `build.builder=DOCKERFILE`
- `build.dockerfilePath=Dockerfile.mcp`
- `deploy.healthcheckPath=/healthz`
- MCP-only watch paths for `crates/orsgraph-mcp/**`, `Dockerfile.mcp`, and the
  MCP entrypoint

Required Railway variables for the `orsgraph-mcp` service:

- `ORSGRAPH_API_BASE_URL`: the reachable `orsgraph-api` base URL ending in
  `/api/v1`.
- `ORSGRAPH_MCP_BEARER_TOKEN` for local/private deployments, or
  `ORSGRAPH_MCP_JWT_ISSUER` plus `ORSGRAPH_MCP_JWT_AUDIENCE` for public
  deployments.
- `ORSGRAPH_MCP_ALLOWED_HOSTS` and `ORSGRAPH_MCP_ALLOWED_ORIGINS` when using a
  custom public domain. The Docker entrypoint auto-populates Railway public and
  private domains when Railway provides them.

Optional service variables:

- `ORSGRAPH_API_KEY`: fixed server-side ORSGraph API key for protected
  read-only tools such as CaseBuilder matter lookup.
- `ORSGRAPH_MCP_RATE_LIMIT_REQUESTS` and
  `ORSGRAPH_MCP_RATE_LIMIT_WINDOW_SECS`: tune the `/mcp` limiter.
- JWT/OAuth metadata variables listed above when exposing the service publicly.

Suggested Railway service setup:

```bash
railway add --service orsgraph-mcp
railway environment edit --service-config orsgraph-mcp build.builder DOCKERFILE
railway environment edit --service-config orsgraph-mcp build.dockerfilePath Dockerfile.mcp
railway environment edit --service-config orsgraph-mcp deploy.healthcheckPath /healthz
railway variable set ORSGRAPH_API_BASE_URL=https://orsgraph-api.example.com/api/v1 --service orsgraph-mcp
railway variable set ORSGRAPH_MCP_BEARER_TOKEN=replace-with-a-real-secret --service orsgraph-mcp
railway up --service orsgraph-mcp --detach -m "deploy orsgraph mcp http service"
```

## Verification

```bash
cargo test -p orsgraph-mcp
```

End-to-end Streamable HTTP smoke with a stub API and local MCP server:

```bash
node scripts/smoke-mcp-http.mjs
```

Build the release MCP binary and run the same end-to-end HTTP smoke against
that exact artifact:

```bash
scripts/test-mcp-e2e.sh
```

Require the MCP Docker deployment image build as part of the same gate:

```bash
scripts/test-mcp-e2e.sh --docker
```

Smoke a local MCP server against a real ORSGraph API:

```bash
node scripts/smoke-mcp-http.mjs \
  --api-base-url http://127.0.0.1:8080/api/v1
```

Smoke an already-running or remote MCP endpoint:

```bash
node scripts/smoke-mcp-http.mjs \
  --mcp-url https://mcp.example.com/mcp \
  --bearer-token "$ORSGRAPH_MCP_BEARER_TOKEN"
```

The build E2E harness accepts the same real API or deployed MCP targets:

```bash
scripts/test-mcp-e2e.sh --api-base-url http://127.0.0.1:8080/api/v1
scripts/test-mcp-e2e.sh --mcp-url https://mcp.example.com/mcp --bearer-token "$ORSGRAPH_MCP_BEARER_TOKEN"
```

The smoke script initializes MCP over HTTP, sends
`notifications/initialized`, runs `tools/list`, calls `orsgraph_health`, and
shuts down any local stub/API processes it started.

For interactive MCP inspection:

```bash
npx @modelcontextprotocol/inspector cargo run -p orsgraph-mcp
```

For HTTP inspection, start the server with `--http`, then connect the inspector
to `http://127.0.0.1:8090/mcp`.
