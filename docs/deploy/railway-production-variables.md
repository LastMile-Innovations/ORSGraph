# Railway Production Variables

This manifest lists required Railway variables by service without secret values. Keep actual secrets sealed in Railway.

## Public Services

### frontend

- `NEXT_PUBLIC_ORS_API_BASE_URL`: canonical browser-visible API base URL, for example `https://orsgraph-api-production.up.railway.app/api/v1`.
- Do not set backend-only flags here, including `ORS_ADMIN_ENABLED`.
- Do not expose secrets through `NEXT_PUBLIC_*`.
- `NEXT_PUBLIC_API_URL` is deprecated; use `NEXT_PUBLIC_ORS_API_BASE_URL`.

### orsgraph-api

- `PORT`: Railway-provided listener port. `ORS_API_PORT` may override it when needed.
- `ORS_API_HOST`: set to `0.0.0.0` on Railway.
- `NEO4J_URI`: internal Neo4j URI, for example `bolt://neo4j.railway.internal:7687`.
- `NEO4J_USER`: Neo4j user.
- `NEO4J_PASSWORD`: sealed Neo4j password.
- `ORS_API_KEY`: optional sealed API key.
- `ORS_ADMIN_ENABLED`: backend-only admin feature flag.
- `ORS_ADMIN_ALLOW_KILL`: backend-only dangerous-operation flag; keep false unless intentionally running admin jobs.
- `VOYAGE_API_KEY`: sealed key, required only when rerank/vector features are enabled.
- `ORS_RERANK_ENABLED`, `ORS_VECTOR_SEARCH_ENABLED`, `ORS_EMBEDDING_MODEL`, `ORS_VECTOR_INDEX`: optional retrieval tuning flags.
- `ORS_STORAGE_BACKEND`: `local` or `r2`.
- `ORS_R2_ACCOUNT_ID`, `ORS_R2_BUCKET`, `ORS_R2_ACCESS_KEY_ID`, `ORS_R2_SECRET_ACCESS_KEY`, `ORS_R2_ENDPOINT`: sealed R2 settings, required only when `ORS_STORAGE_BACKEND=r2`.
- `ORS_ASSEMBLYAI_ENABLED`, `ASSEMBLYAI_API_KEY`, `ORS_ASSEMBLYAI_WEBHOOK_URL`, `ORS_ASSEMBLYAI_WEBHOOK_SECRET`: optional transcription settings.
- Deprecated duplicates to avoid: `ORS_NEO4J_*` and double-underscore `ORS__*`.

## Private Services

### neo4j

- Keep public domains and TCP proxies disabled unless browser/TCP access is intentionally needed.
- `NEO4J_AUTH`: sealed Neo4j auth value.
- Exactly one volume should be attached at `/data`.
- Current intended service access is private Railway DNS only: `neo4j.railway.internal`.

### ors-crawler

- Keep public domains disabled.
- `ORS_CONTAINER_ROLE`: set to `crawler` for worker/job deployments.
- `NEO4J_URI`, `NEO4J_USER`, `NEO4J_PASSWORD`: same canonical Neo4j variables as the API.
- `S3_ENDPOINT`, `S3_REGION`, `S3_BUCKET`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`: sealed cache/object-storage sync settings when S3 sync is enabled.
- `SEED_MODE`: default `append`; valid values are `skip`, `append`, and `replace`.
- `ORS_ALLOW_PRODUCTION_REPLACE`: must be `true` for destructive replace runs after a volume backup.
- `NEO4J_CLEAR_BATCH_SIZE`: default `100`.
- `SEED_NODE_BATCH_SIZE`: default `1000`.
- `SEED_EDGE_BATCH_SIZE`: default `1000`.
- `SEED_RELATIONSHIP_BATCH_SIZE`: default `500`.
- `REBUILD_GRAPH`: default `false`; set true only when rebuilding graph JSONL from cached official HTML.

## MCP

Railway MCP resources/templates were not registered in this Codex session when this manifest was written. Add Railway MCP as a separate local setup task, then use it for read-only `list-services`, `list-variables`, and log checks before future production mutation.

## Docs Basis

- Railway public networking: https://docs.railway.com/public-networking
- Railway healthchecks: https://docs.railway.com/reference/healthchecks
- Railway variables: https://docs.railway.com/variables
- Railway volumes: https://docs.railway.com/volumes/reference
- Railway config as code: https://docs.railway.com/reference/config-as-code
- Railway MCP server: https://docs.railway.com/ai/mcp-server
