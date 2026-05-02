# Neo4j Cost And Performance Runbook

ORSGraph treats Neo4j as the source-backed graph engine, not the first cache for every public read. Public statute/search reads should hit the release-addressed authority hotset and Cloudflare cache first. Neo4j should do the work only when the request needs graph traversal, search expansion, corpus QC, or ingestion validation.

## Current Guardrails

- `/graph/neighborhood` is the primary graph exploration route. It resolves the center node through label-specific indexed lookups before traversing.
- `/graph/path` resolves both endpoints through indexed lookups before running shortest-path traversal. This keeps the planner anchored to one source/target pair.
- `/graph/full` is an overview sample, not a dump endpoint. It defaults to 250 nodes and at most 750 edges, caps at 1,000 nodes and 3,000 edges, and excludes retrieval chunks unless explicitly requested.
- Retrieval chunks and `SIMILAR_TO` edges are opt-in for graph views. They are useful, but they are also the fastest way to inflate memory, traversal fan-out, and browser render cost.
- Corpus stats are cached in-process for five minutes. The legal corpus changes by release, so 30-second live count refreshes are unnecessary pressure on Railway Neo4j.
- No new environment variables are required for these default protections. The limits are code-level safety rails so production cannot drift into expensive graph dumps by configuration accident.

## Route Policy

Use `/graph/neighborhood` for user-facing graph UI:

- `depth`: capped at 2.
- `limit`: capped at 500 nodes.
- `includeChunks`: default `false`.
- `includeSimilarity`: default `false`; enable only for explicit semantic exploration.
- `mode`: use `legal`, `citation`, `semantic`, `history`, or `hybrid` instead of asking for all relationship types.

Use `/graph/path` for explainable paths between two known nodes:

- Pass canonical ids, provision ids, or citations.
- Keep `limit` small; the API caps it at 10 paths.
- Prefer citation/legal modes for public UX. Use hybrid only when the UI is explicitly showing semantic bridges.

Use `/graph/full` only for admin previews and smoke checks:

- It returns a bounded sample and reports `stats.truncated` when it reaches the configured cap.
- Pass a narrow `nodeTypes` or `relationshipTypes` filter for focused admin diagnostics.
- Do not build a production UX that depends on loading the whole database.

## Index Strategy

The hot path is exact lookup first, traversal second. Keep indexes on ids and citations used to anchor graph queries:

- `LegalTextIdentity.canonical_id`, `LegalTextIdentity.citation`
- `LegalTextVersion.version_id`, `LegalTextVersion.canonical_id`
- `Provision.provision_id`, `Provision.canonical_id`, `Provision.display_citation`, `Provision.version_id`
- semantic/history ids such as `chunk_id`, `semantic_id`, `definition_id`, `source_note_id`, `status_event_id`, and `temporal_effect_id`

The crawler schema and API startup indexes should stay reconciled. If storage/page-cache pressure rises, inspect `SHOW INDEXES` and remove redundant single-property indexes that duplicate uniqueness constraints from the corpus loader.

## Railway Neo4j Operating Model

- Keep Neo4j private to Railway services. The public app should talk to the API, not to Bolt.
- Keep the crawler stopped except during controlled corpus import/QC windows.
- Size memory for the working set, not the theoretical whole corpus. Neo4j page cache should hold hot graph data and native indexes; leave OS memory for the runtime and vector index memory.
- Avoid swap. If the instance is memory constrained, reduce graph fan-out and vector/similarity usage before increasing public traffic.
- Run `neo4j-admin server memory-recommendation` from the Neo4j service shell when resizing plans or after a major corpus import.
- Use Railway metrics for CPU, memory, restart count, and disk growth. If CPU spikes line up with graph routes, inspect `/graph/full` usage first.

## Query Review Checklist

Before adding or changing graph queries:

- Anchor with a label and indexed property before traversal.
- Avoid `MATCH (n)` or `MATCH (a), (b)` in public routes unless it is immediately capped and intentionally admin-only.
- Put hard `LIMIT` values in Cypher, not only in Rust after streaming rows.
- Keep variable-length paths bounded.
- Return only fields needed by the UI; avoid returning whole nodes/relationships.
- Use parameters so Neo4j can reuse plans.
- Profile representative queries against a current corpus before raising caps.

## Measurement Targets

- `citation_open`: Cloudflare/R2 hotset whenever possible; Neo4j fallback should be rare.
- `graph_neighborhood`: p95 under 250 ms for depth 1 legal/citation views after warmup.
- `graph_path`: p95 under 500 ms for exact endpoints after warmup.
- `graph_full`: admin-only; bounded sample should return quickly and never stream the whole store.
- Neo4j fallback count, page-cache miss pressure, and Railway memory should be reviewed after every corpus release.

## References

- Neo4j query tuning: https://neo4j.com/docs/cypher-manual/current/execution-plans/
- Neo4j search-performance indexes: https://neo4j.com/docs/cypher-manual/current/indexes/search-performance-indexes/
- Neo4j shortest path planning: https://neo4j.com/docs/cypher-manual/current/patterns/shortest-paths/
- Neo4j memory configuration: https://neo4j.com/docs/operations-manual/current/performance/memory-configuration/
- Neo4j disks, RAM, and page-cache warmup: https://neo4j.com/docs/operations-manual/current/performance/disks-ram-and-other-tips/
