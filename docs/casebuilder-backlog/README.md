# CaseBuilder Backlog

CaseBuilder is the legal workbench layer on top of ORSGraph. This backlog tracks what is already implemented, what is only scaffolded, and what remains across V0, V0.1, V0.2, and V1.

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

## Current done summary

- Canonical `/casebuilder` app routes exist.
- Legacy `/matters` URLs redirect to `/casebuilder`.
- Frontend CaseBuilder links use canonical route helpers.
- Frontend pages use a CaseBuilder data adapter with `live`, `demo`, and `error` states.
- Demo fallback is visibly labeled.
- Rust API includes CaseBuilder models, service, routes, local upload storage, Neo4j constraints, first-pass provenance DTOs/graph nodes, deterministic text extraction with source spans, deterministic draft/fact/citation checks, and authority search bridge to ORSGraph search.
- Route-ready shells exist for parties, complaint builder, graph, QC, export, authorities, and tasks.

## Production wire extension

The backlog now explicitly tracks the missing production wire for the R2 evidence/artifact lake plus Neo4j intelligence graph, case-file indexing harness, remaining provenance depth beyond the first `ObjectBlob`/`DocumentVersion`/`IngestionRun`/`SourceSpan` slice, ingestion manifests, issue spotting, authority attachment, sentence-level draft support, mutable deadlines/tasks, QC runs, finding lifecycle, matter graph API, export packages, audit/retention, DTO/route contracts, matter isolation, and large-matter performance.

## Latest verification

Last verified on 2026-05-01.

- `cargo test -p orsgraph-api casebuilder`
- `./node_modules/.bin/tsc --noEmit --incremental false`
- `cargo check -p orsgraph-api`
- `cargo test -p orsgraph-api`
- `./node_modules/.bin/tsc --noEmit --incremental false`
- `pnpm run build`
- `pnpm run lint`
- Smoke checks:
  - `/casebuilder` returns 200.
  - `/casebuilder/matters/matter%3Asmith-abc` returns 200.
  - `/matters` redirects to `/casebuilder`.
  - `/matters/matter%3Asmith-abc` redirects to `/casebuilder/matters/matter%3Asmith-abc`.

## Phase order

1. Finish V0 foundation gaps that affect trust or persistence.
2. Complete V0 user workflows end to end with live API calls.
3. Add V0.1 builders and dashboards.
4. Add V0.2 export, exhibit, motion, and retrieval depth.
5. Add V1 collaboration, attorney review, court-rule, case-law, and filing capabilities.
