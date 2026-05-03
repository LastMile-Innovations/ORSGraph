# ORSGraph Docs

## Operations

- [Crawler And API Overview](crawler-api-overview.md): Current operating map for the crawler CLI, graph JSONL flow, admin job runner, Neo4j lifecycle, Docker entrypoint, API server boundary, CaseBuilder transcription boundary, and verification commands.
- [MCP End-To-End Runbook](mcp-end-to-end.md): Build, smoke, Docker, Railway, client acceptance, CI, and troubleshooting path for the ORSGraph MCP server.
- [MCP Server Reference](mcp-server.md): MCP tools, stdio and Streamable HTTP client config templates, security defaults, and deployment reference.
- [Neo4j Cost And Performance Runbook](deploy/neo4j-cost-performance.md): Runtime guardrails for graph reads, Railway Neo4j sizing, cache posture, query review, and admin-only full-graph use.

## Data Model

- [Full Data Model](data-model/full-data-model.md): Top-down legal graph model for jurisdictions, courts, corpora, legal text, registry/currentness overlays, procedural rule packs, CaseBuilder matters, WorkProduct ASTs, history, JSONL files, API surfaces, and expansion rules for Oregon, other states, and federal law.

## Data Sources

- [Free Public Data Source Registry](data/source-registry.md): Master registry of free public Oregon, federal, case-law, legislative, business, evidence, GIS, and expansion sources for ORSGraph, NeighborOS, and CaseBuilder ingestion.
- [Source Registry Schema](data/source-registry.schema.md): YAML-shaped source manifest schema, connector lifecycle values, provenance requirements, and validation rules for registry entries.
- [Registry-Driven Crawler](data/registry-driven-crawler.md): Current crawler runtime, connector contract, CLI commands, artifact layout, OData connector behavior, offline fixture testing, QC, and admin wiring.

## Legal Corpora

- [2025 UTCR Graph Ingestion](legal-corpora/2025-utcr-ingestion.md): How the Oregon Uniform Trial Court Rules PDF becomes a first-class court-rule corpus, procedural requirement graph, WorkProduct rule-pack source, search authority family, and seedable JSONL output.
- [Court Rules Registry Layer](legal-corpora/court-rules-registry-layer.md): How SLR/CJO/PJO registry tables become source-backed currentness, applicability, supersession, and WorkProduct authority overlays.
- [Local SLR PDF Ingestion](legal-corpora/local-slr-pdf-ingestion.md): How local Supplementary Local Rule PDFs become source-backed SLR corpora with provisions, citations, source pages, and retrieval chunks.
- [Oregon Legislature OData Ingestion](legal-corpora/oregon-legislature-odata-ingestion.md): How the Oregon Legislature OData service should enrich ORSGraph with sessions, measures, documents, history actions, sponsors, votes, committees, legislators, and session-law links.
- [Top-Down Expansion Roadmap](legal-corpora/top-down-expansion-roadmap.md): Oregon-first expansion plan for all county SLRs and later every state and federal law/rule stack.

## Product Backlogs

- [CaseBuilder Backlog](casebuilder-backlog/README.md): Current CaseBuilder implementation status and roadmap.
- [Frontend Backlog](frontend-backlog/README.md): Frontend stabilization, navigation, API integration, workflows, UX, and quality work.
