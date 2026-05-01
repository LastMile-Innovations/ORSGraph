# CaseBuilder Backlog

CaseBuilder is the legal workbench layer on top of ORSGraph. This backlog tracks what is already implemented, what is only scaffolded, and what remains across V0, V0.1, V0.2, V1, and dedicated product programs such as WorkProduct Builder, Case History, and the complaint profile.

## How to use this backlog

Work in phase order unless a dependency blocks progress. A feature is not Done just because a route exists; Done means the acceptance checks pass and the UI truthfully labels live, demo, disabled, or offline behavior.

## Priority legend

- `P0`: Blocks navigation, data integrity, privacy, or trust.
- `P1`: Required for the next usable CaseBuilder release.
- `P2`: Important workflow completeness or quality.
- `P3`: Polish, hardening, optimization, or future expansion.

## Status legend

- `Done`: Implemented and verified.
- `Partial`: Some route, model, contract, or UI exists, but the workflow is not production-complete.
- `Todo`: Not implemented.
- `Deferred`: Intentionally out of scope for the current phase.
- `Blocked`: Needs a dependency or product decision.

## Backlog files

- [../data-model/full-data-model.md](../data-model/full-data-model.md): Full ORSGraph/CaseBuilder data model covering jurisdictions, courts, legal corpora, registry/currentness overlays, WorkProduct ASTs, Case History, JSONL files, APIs, and expansion conventions.
- [../legal-corpora/2025-utcr-ingestion.md](../legal-corpora/2025-utcr-ingestion.md): Source-backed 2025 UTCR corpus ingestion, graph contract, procedural requirement extraction, WorkProduct rule packs, search behavior, QC, and seed workflow.
- [../legal-corpora/court-rules-registry-layer.md](../legal-corpora/court-rules-registry-layer.md): Court rules registry/currentness ingestion for SLR/CJO/PJO indexes.
- [../legal-corpora/local-slr-pdf-ingestion.md](../legal-corpora/local-slr-pdf-ingestion.md): Source-backed local SLR PDF ingestion.
- [../legal-corpora/top-down-expansion-roadmap.md](../legal-corpora/top-down-expansion-roadmap.md): Oregon-first, then all states and federal expansion plan.
- [00-current-status.md](00-current-status.md): What exists today and what is still not real.
- [01-v0-foundation.md](01-v0-foundation.md): Canonical routes, backend contracts, storage, graph persistence, and data state.
- [02-v0-mvp-workflows.md](02-v0-mvp-workflows.md): Matter workspace, upload, extraction, facts, timeline, claims, evidence, legacy drafting, fact-checking, authority search, and the complaint profile entry point.
- [03-v0.1-backlog.md](03-v0.1-backlog.md): Answer profile, defenses, deadlines, notices/forms, graph, QC, tasks, and stronger workflow state.
- [04-v0.2-backlog.md](04-v0.2-backlog.md): Motion/declaration/exhibit-list profiles, export packets, reranking, and semantic retrieval upgrades.
- [05-v1-backlog.md](05-v1-backlog.md): Multi-user review, attorney mode, court rules, case law, filing workflow, and advanced strategy.
- [06-cross-cutting.md](06-cross-cutting.md): API, data model, privacy, safety, testing, observability, performance, and rollout work.
- [07-feature-inventory.md](07-feature-inventory.md): Status matrix for all major CaseBuilder modules and killer features.
- [08-case-file-indexing-harness-spec.md](08-case-file-indexing-harness-spec.md): Production spec for indexing hundreds to thousands of mixed case files.
- [09-indexing-harness-backlog.md](09-indexing-harness-backlog.md): Implementation backlog for the indexing harness.
- [10-complaint-editor-backlog.md](10-complaint-editor-backlog.md): Complaint profile backlog for the first structured WorkProduct profile: complaint AST/facade, guided UX, Oregon rule pack, QC, preview, export, AI, history, and filing packet work.
- [11-case-history-version-control.md](11-case-history-version-control.md): Optimized Case History spec and agile backlog for Git-like legal version control across complaint edits, support links, QC, AI audit, exports, restore, branches, milestones, and merge cards.
- [12-work-product-builder-backlog.md](12-work-product-builder-backlog.md): Canonical shared WorkProduct Builder backlog for the `WorkProduct.document_ast` model, AST patching, editor projections, links/citations/exhibits, rule findings, exports, performance, security, test gates, and reusable legal document editing across complaints, answers, motions, declarations, memos, letters, notices, exhibit lists, filing packets, and future filings.
- [13-work-product-ast-mvp-backlog.md](13-work-product-ast-mvp-backlog.md): Optimized status-marked requirement backlog for the WorkProduct AST MVP, mapping the full `REQ-AST-*` family list into ship-sized implementation items with Done/Partial/Todo/Deferred status.
- [14-media-transcript-creator-editor.md](14-media-transcript-creator-editor.md): CaseBuilder media transcript creator/editor spec and backlog for AssemblyAI provider config, transcription jobs, redacted/reviewed artifacts, media transcript review, captions, webhooks, and reviewed-span case links.

## Current done summary

- Canonical `/casebuilder` app routes exist.
- Legacy `/matters` URLs redirect to `/casebuilder`.
- Frontend CaseBuilder links use canonical route helpers.
- Frontend pages use a CaseBuilder data adapter with `live`, `demo`, and `error` states.
- Demo fallback is visibly labeled.
- Rust API includes CaseBuilder models, service, routes, local/R2 object storage, binary multipart upload, Neo4j constraints, first-pass provenance DTOs/graph nodes, deterministic text extraction with source spans, deterministic draft/fact/citation checks, authority search bridge to ORSGraph search, authority attach/detach endpoints, canonical `WorkProduct.document_ast` persistence, AST patch/validate/conversion routes, structured complaint/work-product editing, canonical WorkProduct routes, graph-native Case History V0 foundations, hybrid graph/R2 AST storage for snapshots/exports, bounded legal layer diffs, scoped AST restore, and matter-scoped AST reference validation.
- ORSGraph now has a first-class 2025 UTCR corpus parser/export path, procedural requirement nodes, WorkProduct rule packs, UTCR-aware search parsing, a Court Rules Registry layer for SLR/CJO/PJO currentness, a local SLR PDF parser, and CaseBuilder rule profile resolution for active UTCR/SLR/order overlays by filing date.
- Route-ready shells exist for parties, graph, QC, export, authorities, and tasks; the Complaint workspace has a structured profile editor and Case History screen.
- The V0 single-user path is wired: create matter, upload files, extract supported text, review facts, create events/claims/evidence, attach ORS authority, create/generate legacy drafts, create and patch AST-backed WorkProducts, link matter-owned support, run checks, view QC findings, edit a complaint-profile work product, view history, compare text/legal layers, restore scoped AST state, and see changed-since-export status.

## Production wire extension

The backlog now explicitly tracks the missing production wire after the canonical AST slice: shared WorkProduct frontend routes, reusable `WorkProductEditor`, rich text/markdown metadata round-tripping, stricter AST validation, schema migrations, typed block registry, deeper patch invariants, sentence support, citation/exhibit lifecycles, AST-aware rule packs, matter-level QC runs, projection parity, production PDF/DOCX renderers, performance budgets, remaining graph/R2 lifecycle policy, large-AST snapshot budgets, full live matter-isolation fixtures, sensitive telemetry audits, fixture/property/visual/smoke test gates, R2 evidence/artifact lake plus Neo4j intelligence graph, case-file indexing harness, remaining provenance depth beyond the first `ObjectBlob`/`DocumentVersion`/`IngestionRun`/`SourceSpan` slice, ingestion manifests, issue spotting, mutable deadlines/tasks, matter graph API, production export packages, audit/retention, and the remaining Case History hardening work: flat-history cleanup, branch alternatives, merge cards, and history smoke coverage.

## Latest verification

Last verified on 2026-05-01.

- `cargo check -p orsgraph-api`
- `cargo check -p ors-crawler-v0`
- `cargo fmt --check -p ors-crawler-v0 -p orsgraph-api`
- `cargo test -p ors-crawler-v0 utcr`
- `cargo test -p orsgraph-api services::search::tests`
- `cargo test -p orsgraph-api complaint_import_parser_preserves_labels_counts_and_citations`
- `cargo test -p orsgraph-api citation_canonical_ids_cover_ors_orcp_and_utcr`
- `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- parse-utcr-pdf --input /Users/grey/Downloads/2025_UTCR.pdf --out data/utcr_2025 --edition-year 2025 --effective-date 2025-08-01 --source-url https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf`
- `cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- seed-neo4j --graph-dir data/utcr_2025/graph --neo4j-uri bolt://localhost:7687 --neo4j-user neo4j --neo4j-password-env NEO4J_PASSWORD --dry-run`
- `cargo test -p orsgraph-api`
- `pnpm run check` from `frontend/`
- `node --check frontend/scripts/smoke-casebuilder.mjs`
- Route smoke remains available for canonical `/casebuilder` pages and `/matters` redirects, but was not re-run in the latest AST verification pass.
- Live `pnpm run smoke:casebuilder` is available but was not run in the latest pass because the local API was not listening on `localhost:8080`.
- Frontend build emits expected fallback warnings for local `/sidebar` and `/statutes` when the API is unavailable, but completes.

## Phase order

1. Finish V0 foundation gaps that affect trust or persistence.
2. Complete V0 user workflows end to end with live API calls.
3. Complete the AST platform gates in `CB-WPB-024` through `CB-WPB-065`: schema, migration, patch safety, projections, links/citations/exhibits, QC, AI patches, diff/restore, graph/R2 storage, export, performance, security, and tests.
4. Build the shared WorkProduct Builder UI on the completed AST foundation: route family, dashboard, template picker, editor shell, rich text/markdown projections, chips, and validation UX.
5. Finish Case History release hardening: matter isolation, flat-history cleanup, support/QC diff, scoped restore, snapshot viewer, compare modal, sensitive-log guardrails, and smoke coverage.
6. Add branch alternatives, milestones, audit reports, support history, and filing-ready hash reports.
7. Add V0.1 profiles and dashboards using the canonical WorkProduct AST.
8. Add V0.2 production export, exhibit-list profile, motion profile polish, retrieval depth, merge cards, and filing locks.
9. Add V1 collaboration, attorney review, court-rule, case-law, and filing capabilities.
