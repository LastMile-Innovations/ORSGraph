# Registry-Driven Crawler

This document describes the crawler that exists in `crates/ors-crawler-v0` as of May 1, 2026. It is registry-driven for source discovery and artifact layout, while older single-corpus parser commands still exist for ORS, UTCR, court-rule registry snapshots, and local SLR PDFs.

## Source Of Truth

The source registry is maintained in two forms:

- `docs/data/source-registry.md`: human-readable source table.
- `docs/data/source-registry.yaml`: machine-readable JSON-shaped registry loaded by the crawler.

The runtime types live in `crates/ors-crawler-v0/src/source_registry.rs`.

The registry currently contains 81 sources and 17 P0 sources. `validate-source-registry` validates all rows against the 20 required fields and enum values. It also warns when acceptable-use review is still marked `needs_review`.

## Runtime Modules

The registry-driven path is split across these modules:

| Module | Role |
| --- | --- |
| `source_registry.rs` | Loads, parses, validates, and indexes registry entries. |
| `connectors/mod.rs` | Defines `DataConnector`, `SourceItem`, `ConnectorOptions`, the ORS connector, and the generic registry-backed connector. |
| `oregon_leg_odata.rs` | Source-specific connector for Oregon Legislature OData. |
| `artifact_store.rs` | Creates `data/sources/<source_id>/` layout, writes raw artifacts, sidecar metadata, manifests, stats, and QC JSON. |
| `fetcher.rs` | Fetches source items with retry/backoff, throttle, timeout, fixture fallback, cache use, content metadata capture, and network-disable support. |
| `graph_batch.rs` | Accumulates in-memory JSONL rows and writes graph files. |
| `ingest_runner.rs` | Orchestrates source selection, discovery, fetch, parse, QC, and graph combination. |
| `source_qc.rs` | Shared source-level QC checks. |

The crawler records ETag and Last-Modified response headers when present. If cache use is enabled and a cached artifact has validators, the runner sends `If-None-Match` and/or `If-Modified-Since`; a `304 Not Modified` response preserves the cached artifact and marks it skipped. Cached artifacts without validators are reused unless `--refresh` is set.

## Connector Contract

Connectors implement this trait:

```rust
#[async_trait]
pub trait DataConnector: Send + Sync {
    fn source_id(&self) -> &'static str;
    fn source_kind(&self) -> SourceKind;
    async fn discover(&self) -> Result<Vec<SourceItem>>;
    async fn parse(&self, artifact: &RawArtifact) -> Result<GraphBatch>;
    async fn qc(&self, artifacts: &[ArtifactMetadata], batch: &GraphBatch) -> Result<QcReport>;
}
```

Fetch is centralized in `ingest_runner.rs` and `fetcher.rs`, not implemented by each connector. This keeps retries, fixture lookup, cache behavior, and artifact persistence consistent across sources.

Current connector selection:

| Source | Connector |
| --- | --- |
| `or_leg_ors_html` | ORS HTML connector wrapping the existing ORS DOM parser. |
| `or_leg_odata` | Oregon Legislature OData connector. |
| All other registry entries | Generic registry-backed connector unless a source-specific connector is added. |

The generic connector preserves raw artifacts and emits source-backed placeholder graph rows from the registry contract. It is useful for manifest/QC coverage but is not a replacement for source-specific parsing.

## CLI Commands

The canonical command shape is registry-first:

```text
validate-source-registry
source-ingest
combine-graph
qc-full
seed-neo4j
materialize-neo4j
qc-neo4j
embed-neo4j
```

Legacy ORS commands are still available only as explicitly named compatibility imports:

| Compatibility need | Command |
| --- | --- |
| Live legacy ORS import | `import-ors-legacy` |
| Cached ORS HTML rebuild | `import-ors-cache` |
| Historical aliases | `crawl`, `parse-cached` |

Removed one-off graph fix/audit binaries should be replaced with supported QC, seed dry-run, materialization, and Neo4j QC commands.

Validate the registry:

```sh
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- validate-source-registry
```

Optionally write the canonical machine registry:

```sh
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- validate-source-registry --write-yaml
```

Run one source:

```sh
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- source-ingest \
  --source-id or_leg_odata \
  --out data/sources \
  --session-key 2025R1 \
  --mode all
```

Run P0 sources:

```sh
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- source-ingest \
  --priority P0 \
  --out data/sources \
  --mode all
```

Combine source graph folders:

```sh
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- combine-graph \
  --sources-dir data/sources \
  --priority P0 \
  --out data/graph
```

Load combined JSONL into Neo4j:

```sh
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- seed-neo4j \
  --graph-dir data/graph \
  --neo4j-uri bolt://localhost:7687 \
  --neo4j-user neo4j \
  --neo4j-password-env NEO4J_PASSWORD
```

Current Neo4j behavior is append/upsert by default. The loader creates constraints, reads graph JSONL batches, and writes nodes and relationships with stable IDs and `MERGE`-based materialization. It does not prune rows that disappeared from the JSONL contract. For a clean replacement, run `clear-neo4j --yes` before `seed-neo4j`, or deliberately enable startup seeding with `ORS_RUN_STARTUP_CRAWLER=true` and `SEED_MODE=replace`. Use `seed-neo4j --dry-run` to validate the JSONL row contract without connecting to Neo4j.

The registry path is the primary ORS crawler path:

```sh
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- source-ingest \
  --source-id or_leg_ors_html \
  --chapters 90,105-107 \
  --out data/sources \
  --mode all
```

The older single-corpus ORS importer is still available for compatibility as `import-ors-legacy` (`crawl` remains an alias), and cached HTML rebuilds use `import-ors-cache` (`parse-cached` remains an alias). New admin/Docker workflows should call the explicit import names or the registry `source-ingest`/`combine-graph` path.

## Source Ingest Options

Important `source-ingest` flags:

| Flag | Meaning |
| --- | --- |
| `--source-id <id>` | Run one registry source. |
| `--priority P0` | Run all sources with a matching priority. |
| `--out <dir>` | Root output directory, usually `data/sources`. |
| `--registry <path>` | Alternate registry path. |
| `--mode discover|fetch|parse|qc|all` | Select the ingest phase. |
| `--fixture-dir <dir>` | Read offline fixtures from `<dir>/<source_id>/<item_id>.*`. |
| `--edition-year <year>` | Edition/session default year. |
| `--session-key <key>` | OData session key such as `2025R1`. |
| `--chapters <list>` | ORS chapter list for `or_leg_ors_html`; also retained as a compatibility fallback for OData session selection. |
| `--max-items <n>` | Truncate discovered items for bounded test runs; `0` means no truncation. |
| `--delay-ms <n>` | Per-fetch delay. |
| `--max-attempts <n>` | Retry attempts for live fetches. |
| `--concurrency <n>` | Bounded fetch/parse concurrency. |
| `--allow-network false` | Fail closed unless a fixture or cache satisfies each item. |
| `--refresh` | Ignore artifact sidecar cache and fetch/read fixtures again. Without this flag, validator-backed cached artifacts are conditionally revalidated. |
| `--fail-on-qc` | Return an error if source QC fails. |

## Artifact Layout

Each source run writes:

```text
data/sources/<source_id>/
  raw/
    <item_id>.<ext>
    <item_id>.metadata.json
  normalized/
  graph/
    *.jsonl
  qc/
    report.json
  manifest.json
  stats.json
```

`manifest.json` is the registry entry used for the run. `stats.json` records run timing, item count, artifact metadata, graph file count, row count, and QC status. Raw sidecars include artifact ID, URL, content type, ETag, Last-Modified, retrieval time, SHA-256 hash, byte length, status, and cache skip status.

Artifact IDs are deterministic for a given `source_id`, `item_id`, and raw content hash. Raw file names are generated from safe item IDs.

## Offline Fixtures

Fixture lookup checks these paths in order:

```text
<fixture_dir>/<source_id>/<item_id>.json
<fixture_dir>/<source_id>/<item_id>.html
<fixture_dir>/<source_id>/<item_id>.txt
<fixture_dir>/<source_id>/<item_id>.pdf
<fixture_dir>/<source_id>.json
<fixture_dir>/<source_id>.html
<fixture_dir>/<source_id>.txt
<fixture_dir>/<source_id>.pdf
```

For `or_leg_odata`, use `metadata.txt` for `$metadata` because fixture lookup does not currently check `.xml` files.

Example offline OData test:

```sh
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- source-ingest \
  --source-id or_leg_odata \
  --fixture-dir /private/tmp/orsgraph-odata-fixture \
  --out /private/tmp/orsgraph-odata-out \
  --session-key 2025R1 \
  --mode all \
  --allow-network false \
  --fail-on-qc
```

Then combine:

```sh
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- combine-graph \
  --source-id or_leg_odata \
  --sources-dir /private/tmp/orsgraph-odata-out \
  --out /private/tmp/orsgraph-odata-out/combined-graph
```

## Oregon Legislature OData Connector

`or_leg_odata` is implemented in `oregon_leg_odata.rs`.

Discovery emits these item IDs for a session such as `2025R1`:

```text
metadata
LegislativeSessions
Measures_2025R1
MeasureDocuments_2025R1
MeasureAnalysisDocuments_2025R1
MeasureHistoryActions_2025R1
MeasureSponsors_2025R1
Committees_2025R1
Legislators_2025R1
CommitteeMeetings_2025R1
MeasureVotes_2025R1
CommitteeVotes_2025R1
```

The fetch layer follows live OData paging links before writing the raw artifact. It supports legacy `d.__next` plus `odata.nextLink` and `@odata.nextLink`, then stores a combined JSON payload for parsing. Stale cached artifacts and offline fixtures can still contain a next link; in that case the parser records a paging diagnostic so the source can be refreshed if row counts look truncated.

The parser accepts legacy OData JSON shapes such as `d.results`, `d`, `value`, `results`, top-level arrays, and object keys matching the entity set. It borrows rows from the parsed JSON payload rather than cloning them, records per-entity-set row stats, and deduplicates graph rows by stable ID before returning each graph batch.

OData graph outputs include:

```text
legislative_sessions.jsonl
legislative_measures.jsonl
legislative_measure_documents.jsonl
legislative_measure_versions.jsonl
legislative_measure_history_actions.jsonl
legislative_measure_sponsors.jsonl
legislative_committees.jsonl
legislative_legislators.jsonl
legislative_committee_meetings.jsonl
legislative_votes.jsonl
vote_events.jsonl
vote_records.jsonl
source_documents.jsonl
session_laws.jsonl
status_events.jsonl
lineage_events.jsonl
legal_actors.jsonl
legislative_edges.jsonl
odata_entity_sets.jsonl
odata_metadata_summary.jsonl
odata_entity_set_stats.jsonl
parser_diagnostics.jsonl
```

Stable IDs follow the session-key pattern:

```text
orleg:session:2025R1
orleg:measure:2025R1:HB:2001
orleg:measure-document:2025R1:HB:2001:A-engrossed
orleg:history-action:2025R1:9001
orleg:legislator:2025R1:SMITH
orleg:committee:2025R1:HHC
orleg:vote:measure:2025R1:77
```

Measures with `ChapterNumber` also emit `SessionLaw` rows using the existing ORS-compatible ID shape:

```text
or:laws:<year>:c:<chapter>
```

## QC Behavior

Shared QC fails when:

- No raw artifacts were preserved.
- No graph rows were emitted.
- An artifact is missing a raw hash or raw path.
- A search-page source has `robots_acceptable_use=needs_review`.

The OData connector adds failures for parser diagnostics with `severity=error`, including malformed measure rows missing `SessionKey`, `MeasurePrefix`, or `MeasureNumber`. It also fails if a `Measures_<session>` artifact emits no measure rows.

Search-page connectors fail closed for broad crawling unless the registry has an explicit acceptable-use policy.

## Admin API And Dashboard

The API exposes source registry inspection:

```text
GET /api/v1/admin/sources
GET /api/v1/admin/sources/:source_id
```

Admin jobs allow:

```text
dashboard crawl shortcut
source_ingest
combine_graph
qc
seed_neo4j
materialize_neo4j
embed_neo4j
```

The dashboard Source Registry panel can run selected-source ingest, P0 ingest, and P0 combine jobs. It also has a per-source operations table with priority/status filters, local artifact and graph metrics, and Monitor/Ingest/Combine controls for each registry source. It includes a Legislature session key field plus ingest mode, refresh-cache, and network controls. The admin service validates `session_key` as a short alphanumeric/dash/underscore value and passes supported ingest controls to the crawler as `--session-key`, `--mode`, `--refresh`, and `--allow-network`.

`GET /api/v1/admin/overview` also includes a crawler runtime summary for the dashboard: configured crawler binary, command prefix, admin workdir, active PID, running/read-only/mutating job counts, active mutating lock state, and last terminal job status. The dashboard shows those fields in the Crawler Runtime panel and can cancel the active crawler job directly from the admin landing page. The overview path uses a fast graph summary for large local corpora: file and byte counts are immediate, while exact row counts are reserved for smaller source-detail views and explicit QC jobs.

The dashboard `crawl` shortcut is retained for UI continuity, but it now builds `source-ingest --source-id or_leg_ors_html`, not the legacy ORS crawler. Legacy-only params such as `fetch_only` and `skip_citation_resolution` should not be sent to registry-backed jobs.

## Current Gaps

The registry-driven crawler is functional, but these gaps remain:

- OData fixture-page chaining is not implemented; offline fixtures with next links still emit paging diagnostics unless the fixture is already combined.
- Most non-ORS and non-OData P0 sources still use the generic registry-backed connector or older specialized parser commands.
- Dedicated Neo4j loaders/materializers for the new legislative JSONL files are still needed; currently `combine-graph` merges them as JSONL outputs.
- Fixture lookup does not include `.xml`, so XML metadata fixtures should use `.txt`.
- Full smoke/e2e/live route tests should be run only when explicitly requested.

## Verification Commands

Useful non-live checks:

```sh
cargo test -p ors-crawler-v0 oregon_leg_odata
cargo test -p ors-crawler-v0 source_registry
cargo check -p ors-crawler-v0
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- validate-source-registry
```

The last tested offline OData fixture run emitted 12 discovered items, 63 graph rows, QC `pass`, and 63 combined rows without network access.
