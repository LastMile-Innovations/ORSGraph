# CaseBuilder Backlog

CaseBuilder is the legal workbench layer on top of ORSGraph. This backlog tracks what is already implemented, what is only scaffolded, and what remains across V0, V0.1, V0.2, V1, and dedicated product programs such as the Complaint Editor.

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

- [00-current-status.md](00-current-status.md): What exists today and what is still not real.
- [01-v0-foundation.md](01-v0-foundation.md): Canonical routes, backend contracts, storage, graph persistence, and data state.
- [02-v0-mvp-workflows.md](02-v0-mvp-workflows.md): Matter workspace, upload, extraction, facts, timeline, claims, evidence, drafting, fact-checking, authority search, and complaint builder.
- [03-v0.1-backlog.md](03-v0.1-backlog.md): Answer builder, defenses, deadlines, notices/forms, graph, QC, tasks, and stronger workflow state.
- [04-v0.2-backlog.md](04-v0.2-backlog.md): Motion builder, exhibits, export packets, reranking, and semantic retrieval upgrades.
- [05-v1-backlog.md](05-v1-backlog.md): Multi-user review, attorney mode, court rules, case law, filing workflow, and advanced strategy.
- [06-cross-cutting.md](06-cross-cutting.md): API, data model, privacy, safety, testing, observability, performance, and rollout work.
- [07-feature-inventory.md](07-feature-inventory.md): Status matrix for all major CaseBuilder modules and killer features.
- [08-case-file-indexing-harness-spec.md](08-case-file-indexing-harness-spec.md): Production spec for indexing hundreds to thousands of mixed case files.
- [09-indexing-harness-backlog.md](09-indexing-harness-backlog.md): Implementation backlog for the indexing harness.
- [10-complaint-editor-backlog.md](10-complaint-editor-backlog.md): Dedicated backlog for the full structured Complaint Editor: AST, routes, guided UX, three-pane editor, integration flow, Oregon rule pack, QC, preview, export, AI, history, and filing packet work.
- [11-case-history-version-control.md](11-case-history-version-control.md): Optimized Case History spec and agile backlog for Git-like legal version control across complaint edits, support links, QC, AI audit, exports, restore, branches, milestones, and merge cards.

## Current done summary

- Canonical `/casebuilder` app routes exist.
- Legacy `/matters` URLs redirect to `/casebuilder`.
- Frontend CaseBuilder links use canonical route helpers.
- Frontend pages use a CaseBuilder data adapter with `live`, `demo`, and `error` states.
- Demo fallback is visibly labeled.
- Rust API includes CaseBuilder models, service, routes, local upload storage, binary multipart upload, Neo4j constraints, first-pass provenance DTOs/graph nodes, deterministic text extraction with source spans, deterministic draft/fact/citation checks, authority search bridge to ORSGraph search, authority attach/detach endpoints, structured complaint/work-product editing, and graph-native Case History V0 foundations.
- Route-ready shells exist for parties, graph, QC, export, authorities, and tasks; the Complaint workspace has a structured editor and Case History screen.
- The V0 single-user path is wired: create matter, upload files, extract supported text, review facts, create events/claims/evidence, attach ORS authority, create/generate draft, run checks, view QC findings, edit a complaint-profile work product, view history, compare snapshots, restore safely, and see changed-since-export status.

## Production wire extension

The backlog now explicitly tracks the missing production wire for the R2 evidence/artifact lake plus Neo4j intelligence graph, case-file indexing harness, remaining provenance depth beyond the first `ObjectBlob`/`DocumentVersion`/`IngestionRun`/`SourceSpan` slice, ingestion manifests, issue spotting, sentence-level draft support, mutable deadlines/tasks, QC runs, finding lifecycle, matter graph API, export packages, audit/retention, DTO/route contracts, matter isolation, large-matter performance, and the remaining Case History hardening work: flat-history cleanup, support/QC diff layers, scoped restore, snapshot viewer, branch alternatives, merge cards, and history smoke coverage.

## Latest verification

Last verified on 2026-05-01.

- `cargo check -p orsgraph-api`
- `cargo test -p orsgraph-api --test graph_contract`
- `cargo test -p orsgraph-api work_product_hashes_are_stable_and_layered --lib`
- `pnpm run check` from `frontend/`
- `cargo test -p orsgraph-api casebuilder`
- `cargo test -p orsgraph-api casebuilder_routes_cover_v0_contracts`
- `./node_modules/.bin/tsc --noEmit --incremental false`
- `cargo check -p orsgraph-api`
- `cargo test -p orsgraph-api`
- `./node_modules/.bin/tsc --noEmit --incremental false`
- `pnpm run build`
- `pnpm run lint`
- `node --check scripts/smoke-casebuilder.mjs`
- Smoke checks:
  - `/casebuilder` returns 200.
  - `/casebuilder/matters/matter%3Asmith-abc` returns 200.
  - `/matters` redirects to `/casebuilder`.
  - `/matters/matter%3Asmith-abc` redirects to `/casebuilder/matters/matter%3Asmith-abc`.
- Live `pnpm run smoke:casebuilder` is available but was not run in the latest pass because the local API was not listening on `localhost:8080`.

## Phase order

1. Finish V0 foundation gaps that affect trust or persistence.
2. Complete V0 user workflows end to end with live API calls.
3. Harden the Complaint Editor and shared WorkProduct editor boundary.
4. Finish Case History release hardening: matter isolation, flat-history cleanup, support/QC diff, scoped restore, snapshot viewer, compare modal, sensitive-log guardrails, and smoke coverage.
5. Add branch alternatives, milestones, audit reports, support history, and filing-ready hash reports.
6. Add V0.1 builders and dashboards.
7. Add V0.2 export, exhibit, motion, retrieval depth, merge cards, and filing locks.
8. Add V1 collaboration, attorney review, court-rule, case-law, and filing capabilities.
