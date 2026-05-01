# ORSGraph Crawler And API Overview

This document is the current operating map for the crawler, graph ingest, admin job runner, Docker entrypoint, and API server after the crawler/API cleanup branch.

## System Boundary

ORSGraph has three cooperating surfaces:

| Surface | Primary code | Responsibility |
| --- | --- | --- |
| Crawler CLI | `crates/ors-crawler-v0/src/` | Fetch public sources, preserve raw artifacts, parse graph JSONL, run QC, seed/materialize Neo4j, and run embedding maintenance. |
| API server | `crates/orsgraph-api/src/` | Serve search, graph, rules, sources, stats, CaseBuilder, and admin HTTP APIs from Neo4j plus local artifact metadata. |
| Frontend admin | `frontend/components/orsg/admin/` | Start allowlisted crawler jobs, inspect source registry state, follow job logs, and run graph maintenance workflows. |

The crawler is the only surface that writes legal corpus graph JSONL. The API reads graph state from Neo4j and local data directories, and its admin service can launch crawler commands as background jobs.

## End-To-End Data Flow

The supported source pipeline is:

```text
docs/data/source-registry.yaml
  -> validate-source-registry
  -> source-ingest
  -> data/sources/<source_id>/
  -> combine-graph
  -> data/graph/
  -> qc-full
  -> seed-neo4j
  -> materialize-neo4j
  -> embed-neo4j, optional
  -> orsgraph-api
```

`source-ingest` writes per-source artifacts and graph rows. `combine-graph` merges selected source graph folders into the canonical `data/graph` contract. `seed-neo4j` loads nodes and direct relationships. `materialize-neo4j` creates derived graph relationships that depend on previously loaded nodes. `embed-neo4j` is optional and should stay gated until parse QC, seed dry-run, live seed, and Neo4j QC pass.

## Canonical Commands

Run commands from the repo root unless using the Docker image.

| Workflow | Command |
| --- | --- |
| Validate source registry | `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- validate-source-registry` |
| Registry ORS ingest | `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- source-ingest --source-id or_leg_ors_html --out data/sources --max-items 2` |
| Registry P0 ingest | `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- source-ingest --priority P0 --out data/sources` |
| Combine graph JSONL | `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- combine-graph --sources-dir data/sources --out data/graph` |
| Full JSONL QC | `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- qc-full --graph-dir data/graph --out data/admin/qc` |
| Seed dry-run | `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- seed-neo4j --graph-dir data/graph --neo4j-uri bolt://localhost:7687 --neo4j-user neo4j --neo4j-password-env NEO4J_PASSWORD --dry-run` |
| Live seed | `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- seed-neo4j --graph-dir data/graph --neo4j-uri bolt://localhost:7687 --neo4j-user neo4j --neo4j-password-env NEO4J_PASSWORD` |
| Materialize Neo4j | `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- materialize-neo4j --graph-dir data/graph --neo4j-uri bolt://localhost:7687 --neo4j-user neo4j --neo4j-password-env NEO4J_PASSWORD` |
| Neo4j QC | `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- qc-neo4j --graph-dir data/graph --neo4j-uri bolt://localhost:7687 --neo4j-user neo4j --neo4j-password-env NEO4J_PASSWORD` |

Compatibility commands remain available, but new workflows should not prefer them:

| Old shape | Current explicit command |
| --- | --- |
| `crawl` | `import-ors-legacy` |
| `parse-cached` | `import-ors-cache` |
| one-off graph fix/audit binaries | `qc-full`, `seed-neo4j --dry-run`, `materialize-neo4j`, and `qc-neo4j` |

## Data Directories

| Path | Owner | Contents |
| --- | --- | --- |
| `docs/data/source-registry.md` | Docs | Human registry of public data sources. |
| `docs/data/source-registry.yaml` | Crawler | Machine registry loaded by `source-ingest`. |
| `data/sources/<source_id>/raw/` | Crawler | Preserved raw artifacts and sidecar metadata. |
| `data/sources/<source_id>/graph/` | Crawler | Per-source JSONL graph rows. |
| `data/sources/<source_id>/qc/report.json` | Crawler | Source-level QC report. |
| `data/sources/<source_id>/manifest.json` | Crawler | Registry entry snapshot used for the run. |
| `data/sources/<source_id>/stats.json` | Crawler | Run timing, artifact counts, graph rows, and QC status. |
| `data/graph/` | Crawler | Combined graph JSONL contract used by Neo4j loaders. |
| `data/admin/jobs/` | API admin service | Job metadata, event stream, stdout log, and stderr log. |
| `data/admin/qc/` | Crawler/admin | Full graph QC reports. |
| `data/casebuilder/uploads/` | API CaseBuilder | Local object store when `ORS_STORAGE_BACKEND=local`. |

`data/` is local/generated and ignored. Source registry docs are tracked; crawl artifacts are not.

## Neo4j Data Handling

The crawler does not write Neo4j during fetch or parse. Neo4j changes happen only through these maintenance commands:

- `seed-neo4j`
- `materialize-neo4j`
- `embed-neo4j`
- `clear-neo4j`

`seed-neo4j` is additive by default. It creates constraints, reads JSONL rows, and upserts nodes and relationships by stable IDs. Re-running a seed updates matching records and materializes missing edges; it does not truncate the database.

Important behavior:

- Stable row IDs are the conflict keys.
- Cypher loaders use `MERGE` or equivalent idempotent materialization.
- Missing rows in a new JSONL run are not automatically deleted from Neo4j.
- `seed-neo4j --dry-run` validates JSONL row contracts without mutating Neo4j.
- `clear-neo4j --yes` is the explicit destructive reset command.
- Docker uses append mode by default. Set `SEED_MODE=replace` to clear before seeding.

Use append mode for normal refreshes. Use replace mode only when the graph contract changed enough that stale nodes or relationships would be misleading.

## API Server

The API entrypoint is `crates/orsgraph-api/src/main.rs`. It loads `ApiConfig`, connects to Neo4j through `AppState::new`, installs CORS and optional API-key middleware, and mounts `/api/v1` routes.

Route modules should stay thin. Orchestration belongs in services:

| Route area | Route module | Service owner |
| --- | --- | --- |
| Admin/jobs/sources | `routes/admin.rs` | `services/admin.rs` |
| Search/open/suggest | `routes/search.rs` | `services/search.rs`, `services/vector_search.rs`, `services/rerank.rs` |
| Statutes/graph/stats/QC | `routes/statutes.rs`, `routes/graph.rs`, `routes/stats.rs`, `routes/qc.rs` | `services/neo4j.rs`, `services/stats.rs`, `services/graph_expand.rs` |
| Rules/currentness | `routes/rules.rs` | `services/rules.rs` |
| CaseBuilder | `routes/casebuilder.rs` | `services/casebuilder/*`, `services/object_store.rs` |

Configuration is environment-first. Key operational variables:

| Variable | Purpose |
| --- | --- |
| `ORS_API_HOST`, `ORS_API_PORT`, `PORT` | API bind address. Docker maps `PORT` to `ORS_API_PORT` when `ORS_API_PORT` is unset. |
| `NEO4J_URI`, `NEO4J_USER`, `NEO4J_PASSWORD` | Neo4j connection for API and crawler loaders. |
| `ORS_ADMIN_ENABLED` | Enables admin routes and dashboard workflows. |
| `ORS_ADMIN_ALLOW_KILL` | Allows admin job cancellation. |
| `ORS_ADMIN_DATA_DIR` | Data root used by admin command construction. |
| `ORS_ADMIN_JOBS_DIR` | Persistent admin job log directory. |
| `ORS_ADMIN_CRAWLER_BIN` | Crawler binary path. `cargo` is special-cased for local development. |
| `ORS_STORAGE_BACKEND` | `local` or `r2` for CaseBuilder artifacts. |
| `ORS_ASSEMBLYAI_ENABLED`, `ASSEMBLYAI_API_KEY` | Enable external transcription provider support. |
| `VOYAGE_API_KEY`, `ORS_VECTOR_SEARCH_ENABLED` | Embedding, vector search, and rerank support. |

## Admin Job Runner

The admin service builds allowlisted crawler commands from typed job kinds. It writes job metadata and logs under `data/admin/jobs`, streams stdout/stderr, prevents concurrent mutating jobs, and records known output paths for the UI.

Current job kinds:

| Admin job kind | Crawler command |
| --- | --- |
| `crawl` | `source-ingest --source-id or_leg_ors_html` |
| `parse` | `import-ors-cache` |
| `qc` | `qc-full` |
| `seed_neo4j` | `seed-neo4j` |
| `materialize_neo4j` | `materialize-neo4j` |
| `embed_neo4j` | `embed-neo4j` |
| `source_ingest` | `source-ingest` with `source_id` or `priority` |
| `combine_graph` | `combine-graph` |

`crawl` remains an admin concept for the ORS quick workflow, but it now runs the registry-backed ORS connector. `source_id`, `priority`, and `session_key` are reserved for connector-backed source ingest jobs. `fetch_only` and `skip_citation_resolution` are legacy flags and should not be sent to registry-backed crawl jobs.

## Docker

The Docker image contains both binaries:

- `/app/orsgraph-api`
- `/app/ors-crawler-v0`

The default entrypoint starts the API server after optional S3 sync and optional Neo4j seed. Set `RUN_CRAWLER_ONLY=true` to bypass the API server and run a crawler command directly:

```sh
docker run --rm \
  -e RUN_CRAWLER_ONLY=true \
  orsgraph-crawler-refactor-smoke \
  validate-source-registry
```

Runtime seed behavior:

- If `NEO4J_URI` and `NEO4J_PASSWORD` are unset, the entrypoint skips seed and starts the API.
- If credentials are set, the entrypoint can rebuild graph JSONL from cached official ORS HTML with `import-ors-cache`.
- `SEED_MODE=append` is the default.
- `SEED_MODE=replace` runs `clear-neo4j --yes` before `seed-neo4j`.

The image copies `docs/data` so default registry commands work inside the container.

## CaseBuilder Transcription Boundary

CaseBuilder transcription is API-owned, not crawler-owned. The current AssemblyAI provider flow preserves provider metadata and exposes the expanded transcript contract through CaseBuilder DTOs. Supported response fields include:

- redacted audio version metadata
- word-search terms and matches
- sentence and paragraph structures
- `paragraph_ordinal`
- speaker/channel data, including `channel`

Heavy transcript/audio artifacts belong in object storage or local CaseBuilder storage. Neo4j should carry queryable IDs, hashes, excerpts, review state, source-span metadata, and relationships.

## Verification Checklist

Minimum local checks for crawler/API changes:

```sh
cargo fmt --check
cargo check -p ors-crawler-v0
cargo check -p orsgraph-api
cargo test -p ors-crawler-v0 source_registry
cargo test -p ors-crawler-v0 oregon_leg_odata
cargo test -p orsgraph-api services::admin::tests
cargo test -p orsgraph-api assemblyai
```

CLI smoke checks:

```sh
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- validate-source-registry
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- source-ingest --source-id or_leg_ors_html --out /private/tmp/orsgraph-source-smoke --chapters 1 --mode all --allow-network false --fixture-dir /private/tmp/orsgraph-source-fixtures
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- combine-graph --sources-dir /private/tmp/orsgraph-source-smoke --source-id or_leg_ors_html --out /private/tmp/orsgraph-graph-smoke
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- seed-neo4j --graph-dir /private/tmp/orsgraph-graph-smoke --neo4j-uri bolt://localhost:7687 --neo4j-user neo4j --neo4j-password-env NEO4J_PASSWORD --dry-run
```

Frontend checks from `frontend/`:

```sh
pnpm run lint
pnpm run typecheck
pnpm run build
pnpm run smoke:routes
```

Docker checks:

```sh
docker build -t orsgraph-crawler-refactor-smoke .
docker run --rm -e RUN_CRAWLER_ONLY=true orsgraph-crawler-refactor-smoke validate-source-registry
```

Live Neo4j seed, live API startup, and provider-backed transcription smoke are separate checks because they require local services or external credentials.

## Current Gaps

- The registry runner records ETag and Last-Modified headers, but does not yet send conditional HTTP requests.
- OData paging diagnostics exist, but automatic `__next` or `@odata.nextLink` following is not yet implemented.
- Many non-ORS and non-OData registry entries still use the generic connector or older specialized import commands.
- New legislative JSONL files may need more dedicated Neo4j loaders/materializers as their graph contract stabilizes.
- Live Neo4j/API/provider smoke should be opt-in because it mutates external state or depends on credentials.
