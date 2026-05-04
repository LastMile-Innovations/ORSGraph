# Railway Production Variables

This manifest lists required Railway variables by service without secret values. Keep actual secrets sealed in Railway.

## Public Services

### frontend

- `NEXTAUTH_URL`: public frontend URL, currently `https://frontend-production-090c.up.railway.app`.
- `NEXTAUTH_SECRET`: sealed random secret for encrypted session/JWT state.
- `ZITADEL_ISSUER`: public Zitadel issuer URL, currently `https://zitadel-production-ff6c.up.railway.app`.
- `ZITADEL_PROJECT_ID`: optional Zitadel project id when the API audience is the project id instead of the client id.
- `ZITADEL_CLIENT_ID`: sealed Zitadel OIDC application client id.
- `ZITADEL_CLIENT_SECRET`: sealed Zitadel OIDC application client secret.
- `ORS_API_BASE_URL`: server-only API base URL used by the same-origin `/api/ors/*` proxy, for example `https://orsgraph-api-production.up.railway.app/api/v1`.
- `ORS_AUTHORITY_HOTSET_BASE_URL`: optional full R2/custom-domain release prefix used by `/api/authority/*` before API fallback, for example `https://authority.example.com/authority-hotset/release%3A2026-05-02`.
- Do not set backend-only flags here, including `ORS_ADMIN_ENABLED`, `ORS_AUTH_ENABLED`, or `ORS_API_KEY` unless a server-only Route Handler explicitly needs service bypass behavior.
- Do not expose secrets through `NEXT_PUBLIC_*`.
- `NEXT_PUBLIC_API_URL` and `NEXT_PUBLIC_ORS_API_BASE_URL` are deprecated for app calls; use the same-origin `/api/ors/*` proxy.

### orsgraph-api

The production API service must build from the repository root Docker image
(`source.rootDirectory=/`, `build.builder=DOCKERFILE`,
`build.dockerfilePath=Dockerfile`). That image includes both `/app/orsgraph-api`
and `/app/ors-crawler-v0`; isolated `/crates/orsgraph-api` Railpack builds do
not include Cargo or the crawler binary, so admin source-ingest jobs cannot
spawn.

- `PORT`: Railway-provided listener port. `ORS_API_PORT` may override it when needed.
- `ORS_API_HOST`: set to `0.0.0.0` on Railway.
- `NEO4J_URI`: internal Neo4j URI, for example `bolt://neo4j.railway.internal:7687`.
- `NEO4J_USER`: Neo4j user.
- `NEO4J_PASSWORD`: sealed Neo4j password.
- `ORS_API_KEY`: optional sealed API key.
- `ORS_AUTH_ENABLED`: set to `true` after the Zitadel OIDC app exists.
- `ORS_AUTH_ISSUER`: public Zitadel issuer URL, currently `https://zitadel-production-ff6c.up.railway.app`.
- `ORS_AUTH_AUDIENCE`: Zitadel client id or configured API audience accepted by `orsgraph-api`.
- `ORS_AUTH_ADMIN_ROLE`: admin role name, currently `orsgraph_admin`.
- `ORS_ADMIN_ENABLED`: backend-only admin feature flag.
- `ORS_ADMIN_ALLOW_KILL`: backend-only dangerous-operation flag; keep false unless intentionally running admin jobs.
- `ORS_ADMIN_WORKDIR`: set to `/app` on Railway.
- `ORS_ADMIN_CRAWLER_BIN`: set to `/app/ors-crawler-v0` on Railway.
- `ORS_ADMIN_DATA_DIR`: set to `/app/data` on Railway.
- `ORS_ADMIN_JOBS_DIR`: set to `/app/data/admin/jobs` on Railway.
- `VOYAGE_API_KEY`: sealed key, required only when rerank/vector features are enabled.
- `ORS_RERANK_ENABLED`, `ORS_VECTOR_SEARCH_ENABLED`, `ORS_EMBEDDING_MODEL`, `ORS_VECTOR_INDEX`: optional retrieval tuning flags.
- `ORS_CORPUS_RELEASE_MANIFEST_PATH`: release manifest path; default `data/graph/corpus_release.json`.
- `ORS_AUTHORITY_CACHE_TTL_SECONDS`, `ORS_AUTHORITY_CACHE_MAX_CAPACITY`: API-side authority response cache controls.
- `ORS_QUERY_EMBEDDING_CACHE_TTL_SECONDS`, `ORS_QUERY_EMBEDDING_CACHE_MAX_CAPACITY`: query embedding cache controls.
- `ORS_RERANK_POLICY`: `explicit`, `low_confidence`, or `always`; use `explicit` or `low_confidence` to keep runtime model calls off the default path.
- `ORS_STORAGE_BACKEND`: `local` or `r2`.
- `ORS_R2_ACCOUNT_ID`, `ORS_R2_BUCKET`, `ORS_R2_ACCESS_KEY_ID`, `ORS_R2_SECRET_ACCESS_KEY`, `ORS_R2_ENDPOINT`: sealed R2 settings, required only when `ORS_STORAGE_BACKEND=r2`. For CaseBuilder production direct browser uploads, use Cloudflare R2, not an API byte proxy:
  - `ORS_R2_ACCOUNT_ID=<Cloudflare account id>`
  - `ORS_R2_BUCKET=orsgraph-casebuilder-production`
  - `ORS_R2_ENDPOINT=https://<Cloudflare account id>.r2.cloudflarestorage.com`
  - `ORS_R2_ACCESS_KEY_ID` / `ORS_R2_SECRET_ACCESS_KEY`: Cloudflare R2 S3 API token, `Object Read & Write`, scoped to `orsgraph-casebuilder-production`.
  - Bucket CORS must allow origin `https://frontend-production-090c.up.railway.app`, methods `PUT`, `GET`, `HEAD`, request headers `*`, and expose `ETag`.
  - Production smoke should include `ORS_SMOKE_UPLOAD_ORIGIN=https://frontend-production-090c.up.railway.app` so the signed upload path probes preflight and verifies the `ETag` CORS exposure.
- `ORS_ASSEMBLYAI_ENABLED`, `ASSEMBLYAI_API_KEY`, `ORS_ASSEMBLYAI_WEBHOOK_URL`, `ORS_ASSEMBLYAI_WEBHOOK_SECRET`: optional transcription settings.
- Deprecated duplicates to avoid: `ORS_NEO4J_*` and double-underscore `ORS__*`.

### orsgraph-mcp

Run MCP as a separate public or private service from `orsgraph-api`.
The intended build profile is the repository root Dockerfile
`Dockerfile.mcp`, with `/healthz` as the Railway healthcheck. The template in
`docs/deploy/railway-orsgraph-mcp.json` records the intended builder,
healthcheck, restart policy, and MCP-specific watch paths.

- `ORSGRAPH_API_BASE_URL`: required; points at the reachable `orsgraph-api`
  base URL ending in `/api/v1`.
- `ORSGRAPH_MCP_BIND`: optional; the Docker entrypoint defaults to
  `0.0.0.0:$PORT` on Railway.
- `ORSGRAPH_MCP_BEARER_TOKEN`: acceptable only for local/private deployments.
- `ORSGRAPH_MCP_JWT_ISSUER`, `ORSGRAPH_MCP_JWT_AUDIENCE`,
  `ORSGRAPH_MCP_JWKS_URI`, `ORSGRAPH_MCP_REQUIRED_SCOPES`: preferred public
  endpoint auth.
- `ORSGRAPH_MCP_OAUTH_RESOURCE`, `ORSGRAPH_MCP_AUTHORIZATION_SERVERS`,
  `ORSGRAPH_MCP_OAUTH_SCOPES`: optional OAuth protected-resource metadata
  controls.
- `ORSGRAPH_MCP_ALLOWED_HOSTS`: set to the public/custom domain list when not
  relying on the entrypoint's Railway-domain defaults.
- `ORSGRAPH_MCP_ALLOWED_ORIGINS`: set to trusted browser/client origins when
  not relying on the entrypoint's Railway public-domain default.
- `ORSGRAPH_MCP_RATE_LIMIT_REQUESTS`: default `120`; set per deployment risk.
- `ORSGRAPH_MCP_RATE_LIMIT_WINDOW_SECS`: default `60`.
- `ORSGRAPH_API_KEY`: optional fixed service credential for protected
  read-only ORSGraph API routes such as CaseBuilder matter lookup. Do not
  forward user tokens through MCP.

## Private Services

### neo4j

- Keep public domains and TCP proxies disabled unless browser/TCP access is intentionally needed.
- `NEO4J_AUTH`: sealed Neo4j auth value.
- Exactly one volume should be attached at `/data`.
- Current intended service access is private Railway DNS only: `neo4j.railway.internal`.

### ors-crawler

- Keep public domains disabled.
- `ORS_CONTAINER_ROLE`: set to `crawler` for worker/job deployments.
- With Railpack, starting `ors-crawler-v0` without a subcommand exits without crawl/seed work. The admin dashboard should launch explicit crawler jobs.
- `ORS_RUN_STARTUP_CRAWLER`: default `false`; set `true` only for deliberate one-shot startup seeding outside the dashboard.
- `NEO4J_URI`, `NEO4J_USER`, `NEO4J_PASSWORD`: same canonical Neo4j variables as the API.
- `S3_ENDPOINT`, `S3_REGION`, `S3_BUCKET`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`: sealed cache/object-storage sync settings when S3 sync is enabled.
- `SEED_MODE`: only used when `ORS_RUN_STARTUP_CRAWLER=true`; default `append`; valid values are `skip`, `append`, and `replace`.
- `ORS_ALLOW_PRODUCTION_REPLACE`: must be `true` for destructive replace runs after a volume backup.
- `ORS_DATA_DIR`: default `/app/data`.
- `ORS_GRAPH_DIR`: default `/app/data/graph`.
- `ORS_RAW_DIR`: default `/app/data/raw/official`, used only with `REBUILD_GRAPH=true`.
- `NEO4J_CLEAR_BATCH_SIZE`: default `100`.
- `SEED_NODE_BATCH_SIZE`: default `1000`.
- `SEED_EDGE_BATCH_SIZE`: default `1000`.
- `SEED_RELATIONSHIP_BATCH_SIZE`: default `500`.
- `REBUILD_GRAPH`: only used when `ORS_RUN_STARTUP_CRAWLER=true`; default `false`; set true only when rebuilding graph JSONL from cached official HTML.

### zitadel

- Public Railway domain is attached on port `8080`.
- `ZITADEL_EXTERNALDOMAIN`: current Railway-generated public host without scheme.
- `ZITADEL_EXTERNALPORT`: `443`.
- `ZITADEL_EXTERNALSECURE`: `true`.
- Production has an `ORSGraph` project and an `ORSGraph Frontend` OIDC web application.
- Production redirects:
  - `https://frontend-production-090c.up.railway.app/api/auth/callback/zitadel`
- Keep localhost redirects out of the production OIDC app. Use a separate local/dev app if needed.
- Use authorization code with PKCE, a generated client secret, and JWT access tokens so `orsgraph-api` can verify bearer tokens locally against JWKS.
- Enable role assertion for authentication or include role claims in the requested scopes. The frontend uses the NextAuth Zitadel provider and, when `ZITADEL_PROJECT_ID` is set, requests `urn:zitadel:iam:org:project:id:{projectId}:aud` plus `urn:zitadel:iam:org:projects:roles` so tokens can carry the current project-scoped claim `urn:zitadel:iam:org:project:{projectId}:roles`. It also keeps `urn:iam:org:project:roles` for backwards-compatible claims.
- Set `ZITADEL_PROJECT_ID` on `frontend` and `ORS_AUTH_AUDIENCE` on `orsgraph-api` to the same project id so the API validates the intended audience and both frontend/backend parse the same role claim shape.
- Configure post-logout redirects:
  - `https://frontend-production-090c.up.railway.app`
- Keep localhost post-logout redirects out of the production OIDC app.
- Create the project role `orsgraph_admin`; only that role should unlock `/admin` and backend admin operations.

### 2026-05-02 auth bootstrap

- The initial human admin password was rotated and stored in macOS Keychain under `ORSGraph ZITADEL Production Admin`.
- Login V2 was disabled at the instance feature projection because the production ZITADEL service does not deploy the separate self-hosted Login V2 UI container. Without this, interactive login redirects to `/ui/v2/login` and returns HTTP 404.
- `frontend` has `ZITADEL_CLIENT_ID`, `ZITADEL_CLIENT_SECRET`, and `ZITADEL_PROJECT_ID` set in Railway.
- `orsgraph-api` has `ORS_AUTH_ENABLED=true` and `ORS_AUTH_AUDIENCE` set to the same ZITADEL project id.

### 2026-05-02 crawler startup audit

Read-only Railway checks against the linked `ORSGraph` production project found the `ors-crawler` service has no public domain, its latest deployment is stopped, `ORS_RUN_STARTUP_CRAWLER` is unset, `REBUILD_GRAPH=false`, and `SEED_MODE=append`. With the current crawler startup guard, those variables do not trigger crawl, rebuild, clear, or seed work on build/startup.

### 2026-05-02 deployment watch paths

The repo is a shared monorepo on Railway. Each deployable service should use
`build.watchPatterns` so a docs-only or frontend-only push does not rebuild the
Rust API/crawler services.

- `frontend`: `frontend/**`
- `orsgraph-api`: `Cargo.toml`, `Cargo.lock`, `Dockerfile`, `docker-*.sh`, `crates/orsgraph-api/**`, `crates/ors-crawler-v0/**`, `cypher/**`, `docs/data/**`
- `orsgraph-mcp`: `Cargo.toml`, `Cargo.lock`, `Dockerfile.mcp`, `docker-mcp-entrypoint.sh`, `crates/orsgraph-mcp/**`
- `ors-crawler`: `Cargo.toml`, `Cargo.lock`, `crates/ors-crawler-v0/**`, `cypher/**`, `docs/data/**`

The first push that adds these config-as-code files may still redeploy the
affected services. Later pushes should only deploy services whose watch paths
match the changed files.

## MCP

Railway MCP resources/templates were not registered in this Codex session when this manifest was written. Add Railway MCP as a separate local setup task, then use it for read-only `list-services`, `list-variables`, and log checks before future production mutation.

## Docs Basis

- Railway public networking: https://docs.railway.com/public-networking
- Railway healthchecks: https://docs.railway.com/reference/healthchecks
- Railway variables: https://docs.railway.com/variables
- Railway volumes: https://docs.railway.com/volumes/reference
- Railway config as code: https://docs.railway.com/reference/config-as-code
- Railway MCP server: https://docs.railway.com/ai/mcp-server
- NextAuth Zitadel provider: https://next-auth.js.org/providers/zitadel
- ZITADEL scopes: https://zitadel.com/docs/apis/openidoauth/scopes
- ZITADEL roles and role claims: https://zitadel.com/docs/guides/integrate/retrieve-user-roles
