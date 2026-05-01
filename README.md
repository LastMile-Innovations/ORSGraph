# ORSGraph

A Rust-first crawler/parser, legal graph loader, and CaseBuilder rules backend for Oregon legal authority. The original corpus is the Oregon Revised Statutes. The graph now also supports the 2025 Oregon Uniform Trial Court Rules, court-rules registry/currentness overlays, and local Supplementary Local Rule corpora as first-class source-backed authority.

## What it does

1. Discovers ORS chapters from `https://oregon.public.law/statutes`.
2. Fetches canonical Oregon Legislature chapter HTML from `https://www.oregonlegislature.gov/bills_laws/ors/orsNNN.html`.
3. Stores immutable raw HTML artifacts.
4. Normalizes text.
5. Parses sections/rules, provision markers, citation mentions, procedural semantics, registry currentness, and retrieval chunks.
6. Emits JSONL records ready for Neo4j loading, CaseBuilder rule profiles, and embeddings.

## Why Public.Law is used

Public.Law is used for discovery and outline hints. Oregon Legislature is treated as the canonical online source for ORS chapter HTML.

## Run

### Crawler

ORS crawl:

```bash
cargo run --release -- crawl \
  --out data \
  --delay-ms 900 \
  --max-chapters 0
```

`--max-chapters 0` means no limit.

For a smoke test:

```bash
cargo run --release -- crawl --out data --delay-ms 500 --max-chapters 2
```

Outputs:

```text
data/raw/official/ors001.html
data/normalized/chapters/ors001.txt
data/graph/source_documents.jsonl
data/graph/legal_text_identities.jsonl
data/graph/legal_text_versions.jsonl
data/graph/provisions.jsonl
data/graph/citation_mentions.jsonl
data/graph/retrieval_chunks.jsonl
data/stats.json
```

UTCR parse:

```bash
cargo run --release -p ors-crawler-v0 --bin ors-crawler-v0 -- parse-utcr-pdf \
  --input /Users/grey/Downloads/2025_UTCR.pdf \
  --out data/utcr_2025 \
  --edition-year 2025 \
  --effective-date 2025-08-01 \
  --source-url https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf
```

UTCR outputs stay isolated under `data/utcr_2025/graph/` and should not be mixed into `data/graph/` until a combined seed path is explicitly used. See [2025 UTCR Graph Ingestion](docs/legal-corpora/2025-utcr-ingestion.md) for the graph contract, rule-pack outputs, QC, and seed workflow.

Court rules registry parse:

```bash
cargo run --release -p ors-crawler-v0 --bin ors-crawler-v0 -- parse-court-rules-registry \
  --input data/registry/linn_rules_2026_snapshot.txt \
  --out data/linn_rules_registry_2026 \
  --jurisdiction Linn \
  --snapshot-date 2026-05-01 \
  --source-url https://www.courts.oregon.gov/courts/linn/go/pages/rules.aspx
```

Local SLR PDF parse:

```bash
cargo run --release -p ors-crawler-v0 --bin ors-crawler-v0 -- parse-local-rule-pdf \
  --input /Users/grey/Downloads/Linn_SLR_2026.pdf \
  --out data/linn_slr_2026 \
  --jurisdiction-id or:linn \
  --jurisdiction-name "Linn County" \
  --court-id or:linn:circuit_court \
  --court-name "Linn County Circuit Court" \
  --judicial-district "23rd Judicial District" \
  --edition-year 2026 \
  --effective-date 2026-02-01 \
  --source-url https://www.courts.oregon.gov/courts/linn/go/pages/rules.aspx
```

See [Court Rules Registry Layer](docs/legal-corpora/court-rules-registry-layer.md), [Local SLR PDF Ingestion](docs/legal-corpora/local-slr-pdf-ingestion.md), [Top-Down Expansion Roadmap](docs/legal-corpora/top-down-expansion-roadmap.md), and the [Full Data Model](docs/data-model/full-data-model.md).

### API Server

The `orsgraph-api` crate provides a REST API for querying the Neo4j graph.

```bash
# Set environment variables (see .env.example)
export ORS_API_HOST=127.0.0.1
export ORS_API_PORT=8080
export NEO4J_URI=bolt://localhost:7687
export NEO4J_USER=neo4j
export NEO4J_PASSWORD=your_password

# Run the API server
cargo run --release -p orsgraph-api
```

API endpoints:

- `GET /api/v1/health` - Health check
- `GET /api/v1/stats` - Corpus statistics
- `GET /api/v1/search?q=&type=&limit=&authority_family=` - Search statutes, court rules, provisions, definitions, and procedural requirements. Use `authority_family=ORS`, `authority_family=UTCR`, or omit it for all authority.
- `GET /api/v1/statutes/:citation` - Get statute details
- `GET /api/v1/statutes/:citation/provisions` - Get provision tree
- `GET /api/v1/statutes/:citation/citations` - Get citations
- `GET /api/v1/statutes/:citation/semantics` - Get semantic annotations
- `GET /api/v1/statutes/:citation/history` - Get amendment history
- `GET /api/v1/graph/neighborhood?id=&depth=&limit=` - Graph visualization
- `GET /api/v1/qc/summary` - Quality control summary
- `GET /api/v1/rules/registry` - Court rules registry sources and authority documents
- `GET /api/v1/rules/jurisdictions/:jurisdictionId/current` - Active rules/orders for a jurisdiction today
- `GET /api/v1/rules/jurisdictions/:jurisdictionId/history` - Current, prior, expired, and future rule history
- `GET /api/v1/rules/applicable?jurisdiction=&date=&type=` - Filing-date rule profile by jurisdiction/work-product type
- `GET /api/v1/rules/orders/:authorityDocumentId` - Rule/order detail
- `GET /api/v1/rules/slr/:jurisdictionId/:year` - Supplementary Local Rule edition detail
- `POST /api/v1/ask` - Ask endpoint (stub, returns 501)

## Important legal/source note

The Oregon Legislature online ORS database is not the official printed text. Store this warning on all ORS SourceDocument records and show it in legal answers.

The 2025 UTCR parser records the Oregon Judicial Department PDF source URL and effective date. The court rules registry parser records the official court rules page as provenance and preserves current/future/prior publication buckets separately from computed currentness. Local SLR PDF parsers preserve source pages and official source URLs. Embeddings remain gated until parse QC, seed dry-run, live Neo4j seed, and Neo4j QC pass.

## Next steps

1. Add Neo4j batch loader.
2. Add embedding worker for `retrieval_chunks.jsonl`.
3. Add section-diff cross-validation against Public.Law section pages.
4. Add amendment/change-event parsing from Oregon Laws / statutes affected by measures.
