# ORSGraph MCP Server

`orsgraph-mcp` is a read-only Model Context Protocol server for ORSGraph. It
uses the official Rust SDK (`rmcp`) and supports both stdio and Streamable HTTP.

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

## Tools

- `orsgraph_server_info` returns the configured API target and read-only policy.
- `orsgraph_health` calls `GET /api/v1/health`.
- `orsgraph_search` calls `GET /api/v1/search` with typed, validated filters.
- `orsgraph_open` calls `GET /api/v1/search/open`.
- `orsgraph_get_statute` calls `GET /api/v1/statutes/{citation}`.
- `orsgraph_graph_neighborhood` calls `GET /api/v1/graph/neighborhood`.

All tools are read-only, clamp large limits, trim string inputs, and return
structured JSON MCP results. There is no shell execution surface and
Authorization headers are not forwarded to the ORSGraph API.

## Client Config

Example local client entry:

```json
{
  "mcpServers": {
    "orsgraph": {
      "command": "cargo",
      "args": ["run", "-p", "orsgraph-mcp"],
      "env": {
        "ORSGRAPH_API_BASE_URL": "http://127.0.0.1:8080/api/v1"
      }
    }
  }
}
```

Use `ORSGRAPH_MCP_REQUEST_TIMEOUT_MS` to change the default 15 second API
request timeout.

For HTTP-capable MCP clients, configure:

```text
http://127.0.0.1:8090/mcp
```

The HTTP transport is stateful by default and supports `Mcp-Session-Id`, SSE
reconnects, `Last-Event-ID`, `DELETE` session shutdown, Host validation, Origin
validation, and `MCP-Protocol-Version` validation through `rmcp`.

## HTTP Security

Defaults are intentionally local:

- Binds to `127.0.0.1:8090`.
- Allows loopback `Host` values only.
- Allows `Origin` values for `localhost` and `127.0.0.1` on the bind port.
- Refuses non-loopback binds unless `ORSGRAPH_MCP_BEARER_TOKEN` is set.

Remote example:

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

## Verification

```bash
cargo test -p orsgraph-mcp
```

For interactive MCP inspection:

```bash
npx @modelcontextprotocol/inspector cargo run -p orsgraph-mcp
```

For HTTP inspection, start the server with `--http`, then connect the inspector
to `http://127.0.0.1:8090/mcp`.
