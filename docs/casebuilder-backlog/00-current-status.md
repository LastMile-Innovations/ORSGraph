# 00 - Current Status

This file separates what is actually implemented from what is scaffolded or still planned.

## Latest verification

Last verified on 2026-05-01 after the canonical WorkProduct AST refactor.

- `cargo test -p orsgraph-api` passed.
- `cargo check -p orsgraph-api` passed.
- `pnpm run check` from `frontend/` passed.
- `node --check frontend/scripts/smoke-casebuilder.mjs` passed.
- Frontend build still logs expected local fallback warnings for `/sidebar` and `/statutes` when no local API is available.

Follow-up verification on 2026-05-01 after implementing the first provenance spine slice:

- `cargo test -p orsgraph-api casebuilder` passed.
- `./node_modules/.bin/tsc --noEmit --incremental false` passed.

Follow-up verification on 2026-05-01 after wiring the V0 single-user CaseBuilder path:

- `cargo test -p orsgraph-api casebuilder` passed.
- `cargo test -p orsgraph-api casebuilder_routes_cover_v0_contracts` passed.
- `./node_modules/.bin/tsc --noEmit --incremental false` passed.
- `pnpm run lint` passed.
- `node --check scripts/smoke-casebuilder.mjs` passed.
- Live `pnpm run smoke:casebuilder` was not run because no local API was listening on `localhost:8080`.

Follow-up planning update on 2026-05-01 for the Complaint Editor backlog:

- Added a dedicated Complaint Editor backlog in `10-complaint-editor-backlog.md`.
- No application code changed for this planning pass.
- Verification should remain docs-focused unless implementation work begins.

Follow-up implementation update on 2026-05-01 for the provider-free Complaint Editor:

- Added complaint-specific backend DTOs, Neo4j constraints/indexes, graph edge materialization, API routes, deterministic Oregon complaint QC, preview, export artifacts, AI template states, history, filing packet state, and no-seed complaint defaults.
- Added the frontend complaint workspace route family and structured three-pane Complaint Editor workbench.
- Retired generic complaint draft creation in favor of structured complaint routes that synchronize into complaint-profile WorkProducts.
- Verification: `cargo test -p orsgraph-api`, `pnpm run typecheck`, `pnpm run lint`, and `pnpm run build` passed.

Follow-up implementation update on 2026-05-01 for graph-native Case History V0:

- Added canonical work-product version DTOs, Neo4j constraints/indexes, deterministic hash helpers, snapshot manifests, entity-state records, change sets, version changes, AI audit records, and immutable export snapshot metadata.
- Added canonical work-product Case History endpoints for history, change-set detail, snapshot list/detail/create, compare, restore, export history, and AI audit, plus complaint aliases that delegate to the same handlers.
- Wired complaint/work-product create, edit, support link, QC, AI, export, and restore flows into canonical Case History.
- Added a Complaint workspace Case History screen with timeline, manual snapshots, text compare, restore dry-run/apply, and changed-since-export status.
- Verification: `cargo check -p orsgraph-api`, `cargo test -p orsgraph-api --test graph_contract`, `cargo test -p orsgraph-api work_product_hashes_are_stable_and_layered --lib`, and `pnpm run check` from `frontend/` passed.

Follow-up backlog integration update on 2026-05-01 for WorkProduct Builder:

- Added `12-work-product-builder-backlog.md` as the canonical shared editor backlog.
- WorkProduct DTOs/routes/history are already partially wired; the remaining shared-builder gaps are shared frontend WorkProduct routes/components, reusable `WorkProductEditor`, type/profile registries, rich text custom nodes/marks, strict schema validation, rich legal diff layers, and production DOCX/PDF export.
- Complaint remains implemented as the first WorkProduct profile/facade while future answers, motions, declarations, letters, notices, exhibit lists, and filing packets converge on shared WorkProduct contracts.
- No application code changed for this docs pass.

Follow-up implementation update on 2026-05-01 for the canonical WorkProduct AST refactor:

- `WorkProduct.document_ast` is now the canonical current legal document for WorkProduct persistence.
- Backend and frontend DTOs include `WorkProductDocument`, AST-capable `WorkProductBlock`, structured links, citation uses, exhibit references, text ranges, rule findings, `AstPatch`, `AstOperation`, validation responses, and conversion responses.
- Existing block/mark/anchor/finding arrays remain compatibility projections or adapters; new legal document work should write AST or AST patches.
- Added AST patch, validate, markdown, HTML, and plain-text conversion routes and frontend API helpers.
- Refactored CaseBuilder service reads/writes, preview/export/history/hash/snapshot/restore/diff paths to consume the AST first.
- Added provider-free tests for AST validation, patching, markdown conversion, hash stability, rule-finding projection sync, route contracts, and smoke syntax.

Follow-up backlog expansion update on 2026-05-01 for AST completion:

- Expanded `12-work-product-builder-backlog.md` through `CB-WPB-065` to cover everything needed to complete, test, optimize, and safely ship the AST platform, including hybrid graph/R2 AST storage.
- Added backlog coverage for typed block registry, schema migrations, canonical hashing, patch concurrency, operation invariants, paragraph/cross-reference handling, sentence anchoring, citation/exhibit lifecycles, support inspectors, AST rule engine, matter-level QC, rich text and markdown round-trip, preview/DOCX/PDF renderers, export readiness, AI patch review, scoped restore, branch merge conflicts, large-document performance, snapshot/cache policy, frontend autosave, AST diagnostics, accessibility, matter isolation, sensitive logging, fixture/property/projection/smoke tests, legacy projection cleanup, module extraction, docs, and production release gates.
- No application code changed for this backlog expansion.

Follow-up implementation update on 2026-05-01 for WorkProduct AST hardening:

- Added bounded legal layer diffs for support links, citations, exhibits, rule findings, formatting, and export artifacts.
- Expanded scoped restore across AST blocks, metadata, support links, citations, exhibits, rule findings, formatting, and export state while preserving unrelated current edits.
- Hardened matter-scoped reference validation for support links, complaint links, AST patches, snapshot hydration, restore, compare, and export downloads.
- Tightened privacy edges: AST conflict/errors avoid prompts/patch IDs/legal text, R2 storage errors avoid raw backend details, WorkProduct download filenames are hash-derived, and new document object keys use hashed path segments.
- Added focused unit/contract/smoke coverage for bounded list payloads, layer diffs, scoped restore, safe download responses, matter-isolated object-backed paths, and frontend `layer_diffs` normalization.

Follow-up implementation update on 2026-05-01 for shared WorkProduct frontend routes:

- Added canonical frontend routes for WorkProduct list, new/template creation, detail, editor, QC, preview, export, and history under `/casebuilder/matters/:matterId/work-products`.
- Added `/answer`, `/motion`, `/declaration`, and `/memo` aliases that resolve into an existing typed WorkProduct or the shared new-product flow.
- Added a matter-level WorkProduct dashboard/template picker and a first reusable three-pane WorkProduct workbench over the canonical AST.
- Added a Markdown round-trip panel to the shared workbench that loads AST-backed Markdown, converts edits back into `document_ast`, and saves through the canonical WorkProduct patch path.
- Added selected-block support controls so the shared WorkProduct inspector can link matter facts, evidence, documents/exhibits, and authority/citation text to AST blocks through first-class support-link API routes, preview the linked source, update support relations, and remove block-local links.
- Added backend PATCH/DELETE support-link routes that rebuild AST links/projections, validate matter scope, and record Case History support-use changes for relation updates and removals.
- Verification: `pnpm run typecheck`, `pnpm run lint` (passes with existing admin warnings), `pnpm run build`, `node --check scripts/smoke-routes.mjs`, `node --check scripts/smoke-casebuilder.mjs`, `SMOKE_BASE_URL=http://localhost:3000 node scripts/smoke-routes.mjs` (55 checks), `cargo test -p orsgraph-api casebuilder_routes_cover_v0_contracts`, and `cargo test -p orsgraph-api casebuilder` passed. A later live `pnpm run smoke:casebuilder` attempt was not completed because no API was listening on `localhost:8080`; no further smoke tests should run without explicit approval.

Follow-up implementation update on 2026-05-01 for first-class WorkProduct support-link lifecycle:

- Added dedicated backend support-link update/delete service methods, routes, frontend API helpers, and WorkProduct inspector calls.
- Relation changes and removals now rebuild AST support links, compatibility block projections, marks, validation state, and Case History support-use changes from the backend.
- Verification: `cargo test -p orsgraph-api support_relation_update_rebuilds_ast_links_and_projection`, `cargo test -p orsgraph-api support_removal_rebuilds_ast_links_and_projection`, `cargo test -p orsgraph-api casebuilder_routes_cover_v0_contracts`, `cargo test -p orsgraph-api casebuilder`, `pnpm run typecheck`, `pnpm run lint` (passes with existing admin warnings), and `pnpm run build` passed. No additional smoke test was run.

Follow-up implementation update on 2026-05-01 for WorkProduct selected-text links:

- Added a backend text-range route that writes AST `source_text_range` support links, citation uses, and exhibit references from one selected text payload.
- Added frontend API helpers and WorkProduct editor selection tracking so selected text can be linked to a fact, evidence item, exhibit/document, or authority citation from the inspector.
- Added inspector visibility for range-level links/citations/exhibits on the selected block.
- Verification: `cargo test -p orsgraph-api text_range_link_adds_support_citation_and_exhibit_records`, `cargo test -p orsgraph-api casebuilder_routes_cover_v0_contracts`, and `pnpm run typecheck` passed. No smoke test was run.

Follow-up implementation update on 2026-05-01 for the 2025 UTCR graph corpus:

- Added `/Users/grey/Downloads/2025_UTCR.pdf` as a first-class `or:utcr` court-rule corpus with its own `LegalCorpus`, `CorpusEdition`, `SourceDocument`, `SourcePage`, `CourtRuleChapter`, rule identity/version, provision, citation, procedural requirement, retrieval chunk, and WorkProduct rule-pack outputs.
- Generated `data/utcr_2025/graph/` with 185 source pages, 24 chapters, 239 rules, 1,738 provisions, 491 citation mentions, 2,565 procedural requirements, 4,900 retrieval chunks, and six WorkProduct rule packs.
- Generalized loader Cypher and Rust seed paths so UTCR can share the legal graph spine without ORS-only labels/defaults, while adding UTCR/court-rule labels and procedural requirement relationships.
- Search now recognizes `UTCR 2.010`, UTCR pin citations, UTCR ranges, and `authority_family=UTCR`; CaseBuilder UTCR citations canonicalize to graph rule IDs instead of defaulting to external-only placeholders once seeded.
- Documentation lives in `docs/legal-corpora/2025-utcr-ingestion.md`.
- Verification: `cargo fmt --check -p ors-crawler-v0 -p orsgraph-api`, `cargo check -p ors-crawler-v0`, `cargo check -p orsgraph-api`, `cargo test -p ors-crawler-v0 utcr`, `cargo test -p orsgraph-api services::search::tests`, targeted CaseBuilder UTCR citation tests, parse command, and seed dry-run passed.
- Live Neo4j seed and embeddings were not run because `NEO4J_PASSWORD` was not set and embeddings remain intentionally gated.

Follow-up implementation and docs update on 2026-05-01 for the Court Rules Registry and local SLR graph layer:

- Added a Court Rules Registry parser for SLR/CJO/PJO index tables, preserving `current_future` versus `prior` publication buckets separately from computed currentness.
- Added graph nodes and JSONL outputs for registry sources/snapshots, publication entries, jurisdictions, courts, rule authority documents, SLR editions, effective intervals, rule topics, applicability, supersession, and WorkProduct authority inclusion.
- Added a local SLR PDF parser and reviewed `/Users/grey/Downloads/Linn_SLR_2026.pdf`; the latest parse produced 31 authority units, 124 provisions, 22 citation mentions, 31 retrieval chunks, and zero diagnostics.
- Added Neo4j load/materialization support and a `RuleApplicabilityResolver` route family for registry, jurisdiction current/history, applicable rules, order detail, and SLR edition detail.
- Generalized the model for top-down expansion: jurisdiction ancestry now supports `us -> state -> county/court`, SLR `SUPPLEMENTS` edges are data-driven through `supplements_corpus_id`, and CaseBuilder rule profiles are no longer Linn-only.
- Added documentation in `docs/data-model/full-data-model.md`, `docs/legal-corpora/court-rules-registry-layer.md`, `docs/legal-corpora/local-slr-pdf-ingestion.md`, and `docs/legal-corpora/top-down-expansion-roadmap.md`.
- Verification: `cargo fmt --check`, `cargo check -p ors-crawler-v0`, `cargo check -p orsgraph-api`, `cargo test -p ors-crawler-v0 court_rules_registry`, `cargo test -p ors-crawler-v0 local_rule_pdf_parser`, and the Linn SLR parse command passed.

## Done

### DONE-001 - Canonical CaseBuilder routes
- Priority: P0
- Area: Routing
- Completed behavior: `/casebuilder` and `/casebuilder/matters/:id/*` routes exist. Existing `/matters` URLs redirect to the canonical CaseBuilder URLs.
- Evidence: `frontend/app/casebuilder/*`, `frontend/proxy.ts`, and smoke checks returning 200/307.
- Verification: `pnpm run build`, `curl -I /casebuilder`, `curl -I /matters`.
- Status: Done

### DONE-002 - CaseBuilder backend contract layer
- Priority: P0
- Area: Backend/API
- Completed behavior: Rust API exposes `/api/v1/matters` endpoints for matters, parties, files, documents, extraction, facts, timeline, claims, defenses, evidence, deadlines, tasks, drafts, authority search, and export stubs.
- Evidence: `crates/orsgraph-api/src/routes/casebuilder.rs`.
- Verification: `cargo test -p orsgraph-api` contract tests.
- Status: Done

### DONE-003 - Case graph persistence foundation
- Priority: P0
- Area: Neo4j graph
- Completed behavior: CaseBuilder service creates constraints/indexes and stores CaseBuilder payloads on Neo4j nodes linked from `Matter`.
- Evidence: `crates/orsgraph-api/src/services/casebuilder.rs`.
- Verification: `casebuilder_constraints_cover_core_graph_nodes`.
- Status: Done

### DONE-004 - Local upload storage foundation
- Priority: P0
- Area: File storage
- Completed behavior: Uploaded text payloads can be written under configurable local storage, defaulting to `data/casebuilder/uploads`.
- Evidence: `ORS_CASEBUILDER_STORAGE_DIR`, storage backend configuration, object-store helpers, `CaseBuilderService::write_upload`.
- Verification: unit tests for filename sanitization and hashing.
- Status: Done

### DONE-005 - Explicit data source state
- Priority: P0
- Area: Frontend data trust
- Completed behavior: Frontend CaseBuilder data loads through an adapter that returns `live`, `demo`, or `error`, and the shell can show a visible banner.
- Evidence: `frontend/lib/casebuilder/api.ts`, `frontend/components/casebuilder/data-state-banner.tsx`.
- Verification: TypeScript check and build.
- Status: Done

### DONE-006 - Route-ready V0.1 shells
- Priority: P1
- Area: Frontend pages
- Completed behavior: Parties, complaint, graph, QC, export, authorities, and tasks routes render non-404 pages.
- Evidence: `frontend/app/matters/[id]/*` and `frontend/app/casebuilder/matters/[id]/*`.
- Verification: `pnpm run build`.
- Status: Done

### DONE-007 - Frontend mutation API client
- Priority: P0
- Area: Frontend/API
- Completed behavior: CaseBuilder has typed frontend action wrappers for V0 mutations, with error results instead of hidden demo fallback.
- Evidence: `frontend/lib/casebuilder/api.ts`.
- Verification: `tsc --noEmit --incremental false`.
- Status: Done

### DONE-008 - CaseBuilder upload ignore policy
- Priority: P0
- Area: Privacy
- Completed behavior: The default local upload tree is covered by `/data/` ignore policy and documented explicitly in `.gitignore`.
- Evidence: `.gitignore`.
- Verification: `git diff --check .gitignore`.
- Status: Done

### DONE-009 - Canonical route smoke coverage
- Priority: P1
- Area: Quality
- Completed behavior: Route smoke script checks canonical `/casebuilder` pages and compatibility redirects from `/matters`.
- Evidence: `frontend/scripts/smoke-routes.mjs`.
- Verification: `pnpm run smoke:routes` passed 27 checks against `http://localhost:3000`.
- Status: Done

### DONE-010 - CaseBuilder provenance spine DTOs and graph constraints
- Priority: P0
- Area: Provenance/data model
- Completed behavior: Backend and frontend CaseBuilder registries include `ObjectBlob`, `DocumentVersion`, `IngestionRun`, and `SourceSpan`; Neo4j constraint/index setup covers blob, version, ingestion run, and source span nodes.
- Evidence: `crates/orsgraph-api/src/models/casebuilder.rs`, `crates/orsgraph-api/src/services/casebuilder.rs`, `crates/orsgraph-api/tests/graph_contract.rs`, `frontend/lib/casebuilder/types.ts`, `frontend/lib/casebuilder/api.ts`.
- Verification: `casebuilder_provenance_dtos_exist_in_backend_and_frontend`, `casebuilder_provenance_dtos_serialize_with_matter_safe_ids`, `casebuilder_constraints_cover_core_graph_nodes`.
- Status: Done

### DONE-011 - V0 single-user end-to-end CaseBuilder wiring
- Priority: P0
- Area: CaseBuilder V0 path
- Completed behavior: A single user can create a matter, upload text-like or binary files through the live API, extract text/proposed facts/source spans from V0-supported text files, review a fact, create timeline events, create claims/evidence, attach ORS authority to claims/elements/draft paragraphs, create/generate a deterministic draft, run deterministic checks, view persisted findings in Draft/QC, and see explicit export-deferred status.
- Evidence: `crates/orsgraph-api/src/routes/casebuilder.rs`, `crates/orsgraph-api/src/services/casebuilder.rs`, `frontend/lib/casebuilder/api.ts`, `frontend/components/casebuilder/*`, `frontend/app/matters/[id]/qc/page.tsx`, `frontend/scripts/smoke-casebuilder.mjs`.
- Verification: `cargo test -p orsgraph-api casebuilder`, `cargo test -p orsgraph-api casebuilder_routes_cover_v0_contracts`, `./node_modules/.bin/tsc --noEmit --incremental false`, `pnpm run lint`, and `node --check scripts/smoke-casebuilder.mjs`.
- Status: Done

### DONE-012 - Case History V0 foundation
- Priority: P0
- Area: Legal version control
- Completed behavior: `WorkProduct` is now the canonical versioned subject for durable history. The backend records `ChangeSet`, `VersionChange`, `VersionSnapshot`, `SnapshotManifest`, `SnapshotEntityState`, `VersionBranch`, `LegalSupportUse`, and `AIEditAudit` records for major create/edit/support/QC/AI/export/restore flows. Canonical work-product history/snapshot/compare/restore/export-history/AI-audit routes exist, complaint aliases delegate to them, exports lock to immutable snapshots, and the Complaint workspace has a Case History panel.
- Evidence: `crates/orsgraph-api/src/models/casebuilder.rs`, `crates/orsgraph-api/src/services/casebuilder.rs`, `crates/orsgraph-api/src/routes/casebuilder.rs`, `frontend/lib/casebuilder/types.ts`, `frontend/lib/casebuilder/api.ts`, `frontend/components/casebuilder/complaint-editor-workbench.tsx`, `docs/casebuilder-backlog/11-case-history-version-control.md`.
- Verification: `cargo check -p orsgraph-api`, `cargo test -p orsgraph-api --test graph_contract`, `cargo test -p orsgraph-api work_product_hashes_are_stable_and_layered --lib`, and `pnpm run check` from `frontend/`.
- Status: Done

### DONE-013 - Canonical WorkProduct AST foundation
- Priority: P0
- Area: WorkProduct Builder / AST
- Completed behavior: `WorkProduct.document_ast` is the canonical current document model. It stores metadata, nested AST blocks, links, citation uses, exhibit references, and rule findings. Existing flat blocks, marks, anchors, and findings are rebuilt as adapter/projection surfaces.
- Evidence: `crates/orsgraph-api/src/models/casebuilder.rs`, `crates/orsgraph-api/src/services/casebuilder.rs`, `crates/orsgraph-api/src/routes/casebuilder.rs`, `frontend/lib/casebuilder/types.ts`, `frontend/lib/casebuilder/api.ts`, `frontend/scripts/smoke-casebuilder.mjs`, `docs/casebuilder-backlog/12-work-product-builder-backlog.md`.
- Verification: `cargo test -p orsgraph-api`, `cargo check -p orsgraph-api`, `pnpm run check`, and `node --check frontend/scripts/smoke-casebuilder.mjs`.
- Status: Done

### DONE-014 - AST completion backlog expansion
- Priority: P0
- Area: Planning/quality
- Completed behavior: The WorkProduct Builder backlog now has explicit tickets for finishing, testing, optimizing, securing, and releasing the AST platform and all major systems that consume it.
- Evidence: `docs/casebuilder-backlog/12-work-product-builder-backlog.md`.
- Verification: Docs consistency and whitespace checks.
- Status: Done

### DONE-015 - Shared WorkProduct frontend routes and dashboard
- Priority: P0
- Area: WorkProduct Builder/frontend
- Completed behavior: CaseBuilder has canonical WorkProduct list, new/template, detail, editor, QC, preview, export, and history routes under `/casebuilder/matters/:id/work-products`, plus typed answer/motion/declaration/memo aliases. Users can see all WorkProducts for a matter, create supported product types from templates, open a reusable AST-backed workbench, edit blocks, round-trip Markdown into the canonical AST, link facts/evidence/documents/authorities to selected AST blocks, preview/update/remove those support links through dedicated backend routes, run QC, preview, export, and inspect history surfaces.
- Evidence: `frontend/app/matters/[id]/work-products/*`, `frontend/app/casebuilder/matters/[id]/work-products/*`, `frontend/app/matters/[id]/work-product-alias-page.tsx`, `frontend/components/casebuilder/work-product-dashboard.tsx`, `frontend/components/casebuilder/work-product-workbench.tsx`, `frontend/lib/casebuilder/routes.ts`, `frontend/scripts/smoke-routes.mjs`.
- Verification: `pnpm run typecheck`, `pnpm run lint`, `pnpm run build`, `SMOKE_BASE_URL=http://localhost:3000 node scripts/smoke-routes.mjs`, `cargo test -p orsgraph-api casebuilder_routes_cover_v0_contracts`, and `cargo test -p orsgraph-api casebuilder`.
- Status: Done

## Partial

### PARTIAL-001 - Matter creation UI
- Priority: P0
- Area: Matter intake
- Current behavior: New Matter calls `POST /api/v1/matters`, shows pending/failure state, preserves a labeled demo path, routes successful creates to canonical `/casebuilder/matters/:id`, and uploads selected files through the live binary endpoint.
- Still needed: Live smoke against Neo4j-backed API and add regression coverage for create success/failure.
- Status: Partial

### PARTIAL-002 - File ingestion
- Priority: P0
- Area: Uploads
- Current behavior: Backend supports JSON text upload and multipart binary upload, local/private storage, object-store-oriented upload/download plumbing, opaque new document IDs/object keys, content-addressed `ObjectBlob` identity, original `DocumentVersion` records, and initial `IngestionRun` records for stored uploads. Text, TXT, Markdown, and HTML-like UTF-8 files can be marked extractable; non-text files are stored with explicit queued/unsupported state.
- Still needed: R2 evidence-lake artifact manifests, PDF/DOCX/XLSX/image parsing or OCR, richer upload progress, user-facing duplicate groups, parser status pipeline, and extraction retry/failure UX.
- Status: Partial

### PARTIAL-003 - Extraction
- Priority: P1
- Area: Document understanding
- Current behavior: Backend deterministic extraction chunks V0-supported text, stores extracted text nodes, summarizes text, creates proposed facts linked to the source document, records source spans with byte/character offsets and quotes, and updates ingestion run state to `review_ready`, `failed`, or unsupported as appropriate. OCR, PDF/DOCX/XLSX parsing, and live AI extraction are deferred.
- Still needed: R2 extraction manifests/artifacts, PDF/DOCX/XLSX extraction, richer CSV/HTML extraction, OCR provider, extraction review queue UI, entity/date/money extraction, contradiction checks, citation candidates, and automatic graph-build completion state beyond the V0 synchronous path.
- Status: Partial

### PARTIAL-004 - Drafting and checks
- Priority: P1
- Area: Drafting studio
- Current behavior: Legacy deterministic draft scaffold, unsupported-fact checks, and missing-citation checks exist on backend. Frontend can create drafts, edit/save section bodies, call scaffold generation, run support checks, and render persisted fact/citation findings. New legal document drafting should use WorkProduct AST and AST patches.
- Still needed: Clear legacy Draft boundary, shared WorkProductEditor UI, sentence-level finding UI, source-linked check remediation, unsaved-change warnings, richer Case History diff/restore layers, and provider-gated live AST patch drafting.
- Status: Partial

### PARTIAL-011 - Complaint profile entry point
- Priority: P1
- Area: Complaint workflow
- Current behavior: Complaint Editor is now the first structured WorkProduct profile/facade. It creates complaint-profile state and synchronizes into canonical WorkProduct AST/Case History. Complaint routes remain friendly facades while shared WorkProduct routes/editor mature.
- Still needed: Finish moving complaint UI onto shared WorkProductEditor contracts, keep complaint DTOs as projections, and migrate future motions, answers, declarations, briefs, letters, notices, exhibit lists, and filing packets through `12-work-product-builder-backlog.md`.
- Status: Partial

### PARTIAL-005 - Authority retrieval
- Priority: P1
- Area: ORSGraph bridge
- Current behavior: Backend authority search delegates to ORSGraph search. Frontend authorities page can run live matter-scoped authority searches, attach/detach selected authority to claims, elements, and draft paragraphs, and collect persisted authority refs from the matter graph.
- Still needed: Currentness display, exact provision linking, definitions/deadlines/remedies/penalties panels, recommendation flow, defense/sentence authority targets, and authority gap findings.
- Status: Partial

### PARTIAL-006 - Fact review workflow
- Priority: P1
- Area: Fact table
- Current behavior: Proposed facts can be approved, edited, disputed, or rejected from the facts board through live API mutations, and source spans/quotes are visible in fact review and document detail.
- Still needed: Merge duplicate facts, attach evidence during review, batch review controls, viewer deep-links to exact spans, and mutation regression tests.
- Status: Partial

### PARTIAL-007 - Timeline creation
- Priority: P1
- Area: Timeline
- Current behavior: Timeline page can create live events and link each event to a source document and fact.
- Still needed: Event editing/deletion, party and claim links, calendar view, deadline overlay actions, and automated event extraction.
- Status: Partial

### PARTIAL-008 - Party map
- Priority: P1
- Area: Parties
- Current behavior: Users can manually create parties/entities with role, type, representation, contact, and notes.
- Still needed: Document-driven entity suggestions, edit/delete, evidence and fact links, de-dupe, and richer entity map visualization.
- Status: Partial

### PARTIAL-009 - Claim builder persistence
- Priority: P1
- Area: Claims
- Current behavior: Users can create live claims/counterclaims/defense-labeled theories with elements and a starting supporting fact, run element mapping, attach authority to claims/elements, and receive evidence/fact/element synchronization after evidence links.
- Still needed: Edit/delete, richer element templates, scoring, and matrix synchronization polish.
- Status: Partial

### PARTIAL-010 - Evidence matrix persistence
- Priority: P1
- Area: Evidence
- Current behavior: Users can create evidence quotes/descriptions from documents, link them as supporting or contradicting facts from the matrix detail panel, and backend-created evidence records include structured `SourceSpan` provenance when document provenance is available. Evidence links now update the evidence record, referenced facts, claim evidence IDs, and applicable claim-element evidence IDs.
- Still needed: UI document-span capture, fact-review attachment, edit/delete, exhibit numbering, admissibility notes, and richer claim/element edge editing.
- Status: Partial

### PARTIAL-012 - Graph, QC, and export surfaces
- Priority: P1
- Area: Production wiring
- Current behavior: Graph and export pages render route-ready shells or derived summaries; QC renders derived metrics plus persisted draft/work-product findings. WorkProduct QC, AST validation, AST conversions, preview, export artifacts, immutable export snapshots, and changed-since-export state exist. PDF/DOCX remain review-needed skeletons.
- Still needed: Matter-level QC run, full finding lifecycle/reopen/comments, matter-scoped graph API, interactive graph renderer, persisted gap/contradiction nodes, production export package pipeline, production DOCX/PDF generation, and packet preview/download status.
- Status: Partial

### PARTIAL-013 - AI/provider-gated legal workbench features
- Priority: P1
- Area: AI safety
- Current behavior: Draft generation, extraction, fact-checking, citation-checking, and authority recommendation use deterministic/template behavior or ORSGraph search. Live AI provider execution is not enabled.
- Still needed: Provider configuration, source-backed output schema, issue spotting queue, live drafting/editing, sentence-level support checks, citation currentness/scope checks, and visible disabled/template/live state on every AI action.
- Status: Partial

### PARTIAL-014 - Case-file indexing harness
- Priority: P0
- Area: Indexing/search
- Current behavior: Basic document records, deterministic text extraction, graph persistence, first-pass provenance DTOs/constraints, original document versions, content-addressed blobs, ingestion runs, and source spans exist for small V0 workflows.
- Still needed: Parser registry, inventory/fingerprint index, R2 artifact writer, manifest-to-graph upserter, full-text/vector adapters, OCR workflow, archive/email/spreadsheet indexing, index console, provenance trail, reindex scheduler, large-fixture benchmark, and quarantine policy.
- Status: Partial

### PARTIAL-015 - Case History hardening
- Priority: P0
- Area: Legal version control
- Current behavior: V0 history foundation is wired and visible in the Complaint workspace.
- Still needed: Remove/demote flat history persistence, add matter-isolation tests, add support/citation/authority/QC diff layers, add scoped restore modes, add snapshot viewer and compare modal polish, add sensitive-log guardrails, add end-to-end history smoke, then implement branch alternatives and merge cards.
- Status: Partial

### PARTIAL-016 - AST production readiness
- Priority: P0
- Area: WorkProduct AST
- Current behavior: Canonical AST persistence, patching, validation, conversion, graph materialization, and history/export integration foundations exist.
- Still needed: Complete `CB-WPB-024` through `CB-WPB-065`, including typed schema, migrations, concurrency, editor round-trips, link/citation/exhibit lifecycles, QC/rule packs, PDF/DOCX renderers, graph/R2 storage hardening, diff/restore layers, performance budgets, security tests, fixture corpus, property tests, smoke matrix, module extraction, and release gate.
- Status: Partial

## Not yet real

- No live AI provider execution for fact extraction, issue spotting, drafting, fact checking, or citation checking.
- No PDF/DOCX/XLSX/image parsing or OCR-backed extraction; binary files are stored but non-text extraction remains deferred.
- No full R2 artifact manifest layout for original/text/pages/OCR/redaction artifacts; WorkProduct AST snapshots/exports now have the first object-backed storage slice.
- No R2 event notification ingestion queue or lifecycle/storage-class policy.
- No OCR/transcription.
- No issue spotting endpoint or claim/defense suggestion queue.
- No production sentence-level WorkProduct support graph beyond AST-capable sentence fields.
- No branch alternatives, merge cards, rich support/QC diff layers, scoped support restore, audit reports, or full Case History smoke coverage.
- No matter graph API or interactive CaseBuilder graph renderer.
- No matter-level QC run endpoint or persisted gap/contradiction lifecycle.
- No production indexing harness for 100 to 1,000+ mixed files.
- No production DOCX/PDF export; current WorkProduct PDF/DOCX artifacts are deterministic review-needed skeletons.
- No polished shared frontend WorkProduct route family or reusable rich-text WorkProductEditor.
- No e-filing.
- No multi-user collaboration.
- No attorney review mode.
- No case-law integration.
- No full production court-rule universe yet. The 2025 UTCR corpus, Linn registry snapshot, and Linn 2026 SLR PDF path exist; ORCP, ORAP, OAR, all other Oregon SLRs/orders, forms, federal/state expansion corpora, and case law remain future corpora.
- No production authentication beyond optional API key.
- No audit log, retention settings, or user-facing matter deletion lifecycle UI.
