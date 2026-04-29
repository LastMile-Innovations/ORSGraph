# ORS Crawler V0

A Rust-first V0 crawler/parser for the current Oregon Revised Statutes.

## What it does

1. Discovers ORS chapters from `https://oregon.public.law/statutes`.
2. Fetches canonical Oregon Legislature chapter HTML from `https://www.oregonlegislature.gov/bills_laws/ors/orsNNN.html`.
3. Stores immutable raw HTML artifacts.
4. Normalizes text.
5. Parses sections, provision markers, citation mentions, and retrieval chunks.
6. Emits JSONL records ready for Neo4j loading and embeddings.

## Why Public.Law is used

Public.Law is used for discovery and outline hints. Oregon Legislature is treated as the canonical online source for ORS chapter HTML.

## Run

### Crawler

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
- `GET /api/v1/search?q=&type=&limit=` - Search statutes, provisions, definitions
- `GET /api/v1/statutes/:citation` - Get statute details
- `GET /api/v1/statutes/:citation/provisions` - Get provision tree
- `GET /api/v1/statutes/:citation/citations` - Get citations
- `GET /api/v1/statutes/:citation/semantics` - Get semantic annotations
- `GET /api/v1/statutes/:citation/history` - Get amendment history
- `GET /api/v1/graph/neighborhood?id=&depth=&limit=` - Graph visualization
- `GET /api/v1/qc/summary` - Quality control summary
- `POST /api/v1/ask` - Ask endpoint (stub, returns 501)

## Important legal/source note

The Oregon Legislature online ORS database is not the official printed text. Store this warning on all ORS SourceDocument records and show it in legal answers.

## Next steps

1. Add Neo4j batch loader.
2. Add embedding worker for `retrieval_chunks.jsonl`.
3. Add section-diff cross-validation against Public.Law section pages.
4. Add amendment/change-event parsing from Oregon Laws / statutes affected by measures.
