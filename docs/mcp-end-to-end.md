# ORSGraph MCP End-To-End Runbook

This is the operator path for taking `orsgraph-mcp` from a local build to a
verified Streamable HTTP service. Use it when changing MCP tools, auth,
transport, Docker/Railway config, or client setup.

## What Gets Verified

The end-to-end gate proves:

- The MCP crate formats, compiles, and passes unit/integration tests.
- The release binary starts over Streamable HTTP.
- The server answers `/healthz`.
- An MCP client can initialize with protocol `2025-11-25`.
- The client can send `notifications/initialized`.
- `tools/list` includes `orsgraph_health`.
- `tools/call` can call `orsgraph_health`.
- Locally started stub API and MCP processes shut down cleanly.

The default E2E path uses a stub ORSGraph API so MCP protocol and transport
regressions can be caught without Neo4j, Railway, or a live API.

## Files

- `crates/orsgraph-mcp/`: MCP server crate.
- `scripts/smoke-mcp-http.mjs`: protocol smoke runner.
- `scripts/test-mcp-e2e.sh`: build plus smoke harness.
- `Dockerfile.mcp`: dedicated MCP HTTP container image.
- `docker-mcp-entrypoint.sh`: container entrypoint for `orsgraph-mcp --http`.
- `docs/deploy/railway-orsgraph-mcp.json`: Railway service profile template.
- `docs/mcp-server.md`: reference docs, tools, client configs, and security.

## Fast Local E2E

Run this before treating MCP changes as complete:

```bash
scripts/test-mcp-e2e.sh
```

This command checks script syntax, checks the Docker entrypoint syntax, runs
`cargo fmt --check`, `cargo check`, `cargo test`, builds
`target/release/orsgraph-mcp`, starts that release binary over HTTP, starts a
stub API, calls MCP, and exits.

Expected final output:

```text
ORSGraph MCP HTTP smoke passed
- Protocol: 2025-11-25
- Tools listed: 13
- orsgraph_health: orsgraph-api-stub ok=true HTTP 200
ORSGraph MCP build + end-to-end test passed
```

If the sandbox blocks local port binding, rerun the same command with local
loopback permissions. The tests and smoke both bind `127.0.0.1` ephemeral
ports.

## Local Smoke Only

Use the lighter smoke script when you do not need a release build:

```bash
node scripts/smoke-mcp-http.mjs
```

By default this starts a stub API and starts MCP through `cargo run`.

To smoke the built binary directly:

```bash
cargo build --release -p orsgraph-mcp
node scripts/smoke-mcp-http.mjs \
  --mcp-bin target/release/orsgraph-mcp
```

## Real API E2E

Use this when an ORSGraph API is already running locally or remotely:

```bash
scripts/test-mcp-e2e.sh \
  --api-base-url http://127.0.0.1:8080/api/v1
```

The harness still starts a local MCP release binary, but `orsgraph_health`
calls the real API.

For a protected ORSGraph API route such as CaseBuilder matter lookup, configure
the MCP server with a fixed server-side service key:

```bash
export ORSGRAPH_API_KEY='replace-with-orsgraph-service-key'
scripts/test-mcp-e2e.sh \
  --api-base-url http://127.0.0.1:8080/api/v1
```

Do not pass end-user bearer tokens through MCP tool arguments. Caller
Authorization headers are never forwarded to the ORSGraph API.

## Existing MCP Endpoint Smoke

Use this for a deployed or already-running MCP service:

```bash
node scripts/smoke-mcp-http.mjs \
  --mcp-url https://mcp.example.com/mcp \
  --bearer-token "$ORSGRAPH_MCP_BEARER_TOKEN"
```

The build harness can target the same endpoint after local compile/test/build:

```bash
scripts/test-mcp-e2e.sh \
  --mcp-url https://mcp.example.com/mcp \
  --bearer-token "$ORSGRAPH_MCP_BEARER_TOKEN"
```

When `--mcp-url` is set, the smoke does not start a local MCP process. It
validates the remote endpoint in place.

## Docker E2E

Build the dedicated MCP image:

```bash
docker build -f Dockerfile.mcp -t orsgraph-mcp:local .
```

Run it against a local API:

```bash
docker run --rm -p 8090:8090 \
  -e ORSGRAPH_API_BASE_URL=http://host.docker.internal:8080/api/v1 \
  -e ORSGRAPH_MCP_BEARER_TOKEN=local-dev-secret \
  orsgraph-mcp:local
```

Smoke the running container:

```bash
node scripts/smoke-mcp-http.mjs \
  --mcp-url http://127.0.0.1:8090/mcp \
  --bearer-token local-dev-secret
```

Require Docker image build as part of the standard E2E harness:

```bash
scripts/test-mcp-e2e.sh --docker
```

This requires a running Docker daemon. Without `--docker`, the harness skips the
image build so ordinary local Rust verification does not depend on Docker
Desktop.

## Railway E2E

Run MCP as a separate Railway service from `orsgraph-api`.

Service shape:

- Source root: repository root.
- Builder: Dockerfile.
- Dockerfile path: `Dockerfile.mcp`.
- Healthcheck path: `/healthz`.
- Public MCP endpoint: `/mcp`.
- Watch paths: use `docs/deploy/railway-orsgraph-mcp.json`.

Suggested setup:

```bash
railway add --service orsgraph-mcp
railway environment edit --service-config orsgraph-mcp build.builder DOCKERFILE
railway environment edit --service-config orsgraph-mcp build.dockerfilePath Dockerfile.mcp
railway environment edit --service-config orsgraph-mcp deploy.healthcheckPath /healthz
railway variable set ORSGRAPH_API_BASE_URL=https://orsgraph-api.example.com/api/v1 --service orsgraph-mcp
```

For local or private use, set static bearer auth:

```bash
railway variable set ORSGRAPH_MCP_BEARER_TOKEN=replace-with-a-real-secret --service orsgraph-mcp
```

For a public endpoint, prefer JWT/JWKS:

```bash
railway variable set ORSGRAPH_MCP_JWT_ISSUER=https://auth.example.com --service orsgraph-mcp
railway variable set ORSGRAPH_MCP_JWT_AUDIENCE=https://mcp.example.com/mcp --service orsgraph-mcp
railway variable set ORSGRAPH_MCP_REQUIRED_SCOPES=orsgraph:mcp --service orsgraph-mcp
railway variable set ORSGRAPH_MCP_OAUTH_RESOURCE=https://mcp.example.com/mcp --service orsgraph-mcp
```

Set host and origin allowlists for custom domains:

```bash
railway variable set ORSGRAPH_MCP_ALLOWED_HOSTS=mcp.example.com --service orsgraph-mcp
railway variable set ORSGRAPH_MCP_ALLOWED_ORIGINS=https://mcp.example.com --service orsgraph-mcp
```

Deploy:

```bash
railway up --service orsgraph-mcp --detach -m "deploy orsgraph mcp http service"
```

Verify health:

```bash
curl -fsS https://mcp.example.com/healthz
```

Smoke MCP:

```bash
node scripts/smoke-mcp-http.mjs \
  --mcp-url https://mcp.example.com/mcp \
  --bearer-token "$ORSGRAPH_MCP_BEARER_TOKEN"
```

## Client Acceptance

After the HTTP smoke passes, connect one real MCP client:

- Claude Desktop: use the stdio config or `mcp-remote` bridge in
  `docs/mcp-server.md`.
- Codex: use `~/.codex/config.toml` entries from `docs/mcp-server.md`.
- Cursor: use `.cursor/mcp.json` or global Cursor config.
- MCP Inspector: use the stdio or Streamable HTTP config examples.

Client acceptance is:

- Client connects without protocol errors.
- `orsgraph_health` appears in the tool list.
- `orsgraph_health` returns API health.
- If `ORSGRAPH_API_KEY` is configured, CaseBuilder matter tools appear.
- If no API key is configured, CaseBuilder matter lookup stays disabled.

## Required Environment

Local-only:

- `ORSGRAPH_API_BASE_URL`: only needed when using a real API instead of the
  stub.
- `ORSGRAPH_MCP_BEARER_TOKEN`: needed when testing bearer-protected HTTP.

Railway/container:

- `ORSGRAPH_API_BASE_URL`: required.
- `ORSGRAPH_MCP_BEARER_TOKEN` or JWT/JWKS variables: required for non-loopback
  binds.
- `ORSGRAPH_MCP_ALLOWED_HOSTS`: required for custom public hostnames.
- `ORSGRAPH_MCP_ALLOWED_ORIGINS`: required for browser-origin clients.
- `ORSGRAPH_API_KEY`: optional server-side service credential.

Useful tuning:

- `ORSGRAPH_MCP_RATE_LIMIT_REQUESTS`: default `120`.
- `ORSGRAPH_MCP_RATE_LIMIT_WINDOW_SECS`: default `60`.
- `ORSGRAPH_MCP_REQUEST_TIMEOUT_MS`: default `15000`.
- `ORSGRAPH_MCP_SMOKE_TIMEOUT_MS`: default `30000` for the Node smoke runner.

## CI Hook

The workspace test script can include MCP E2E:

```bash
RUN_MCP_E2E=true scripts/test-all.sh
```

This keeps the normal workspace test path lighter, while making the release
binary smoke available as an explicit gate.

## Troubleshooting

`Operation not permitted` during tests or smoke:

- The sandbox blocked local loopback binds.
- Rerun with permission to bind `127.0.0.1` ephemeral ports.

`HTTP 401` from `/mcp`:

- Missing or wrong bearer token.
- Pass `--bearer-token "$ORSGRAPH_MCP_BEARER_TOKEN"` to the smoke script.

`HTTP 403` from `/mcp`:

- Origin was rejected or JWT scope is insufficient.
- Set `ORSGRAPH_MCP_ALLOWED_ORIGINS` for the client origin.
- For JWT mode, check `ORSGRAPH_MCP_REQUIRED_SCOPES`.

`tools/list` works but `orsgraph_health` fails:

- MCP is up, but `ORSGRAPH_API_BASE_URL` is wrong or the API is down.
- Check the API directly:

```bash
curl -fsS "$ORSGRAPH_API_BASE_URL/health"
```

Docker build cannot start:

- Docker daemon is not running.
- Start Docker Desktop or skip Docker with the default `scripts/test-mcp-e2e.sh`.

Railway deploy starts but healthcheck fails:

- Confirm the service uses `Dockerfile.mcp`, not the API `Dockerfile`.
- Confirm healthcheck path is `/healthz`.
- Confirm `ORSGRAPH_API_BASE_URL` is set.
- Confirm a non-loopback service has bearer auth or JWT/JWKS configured.

Unexpected client behavior after a passing smoke:

- Re-run with MCP Inspector to inspect schemas and payloads interactively.
- Confirm the client is using `/mcp`, not `/healthz` or `/api/v1`.
- Confirm the client sends `MCP-Protocol-Version` when required by its HTTP
  transport.
