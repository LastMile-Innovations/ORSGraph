# 00 - Current Status

This file separates what is actually implemented from what is scaffolded or still planned.

## Latest verification

Last verified on 2026-05-01 while extending the production backlog.

- `cargo test -p orsgraph-api` passed.
- `pnpm run typecheck` passed.
- Worktree already had broad unrelated modified/untracked files; backlog updates should stay scoped to `docs/casebuilder-backlog/*`.

Follow-up verification on 2026-05-01 after implementing the first provenance spine slice:

- `cargo test -p orsgraph-api casebuilder` passed.
- `./node_modules/.bin/tsc --noEmit --incremental false` passed.

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

## Partial

### PARTIAL-001 - Matter creation UI
- Priority: P0
- Area: Matter intake
- Current behavior: New Matter calls `POST /api/v1/matters`, shows pending/failure state, preserves a labeled demo path, and routes successful creates to canonical `/casebuilder/matters/:id`.
- Still needed: Live smoke against Neo4j-backed API and add regression coverage for create success/failure.
- Status: Partial

### PARTIAL-002 - File ingestion
- Priority: P0
- Area: Uploads
- Current behavior: Backend supports JSON text upload, local storage, object-store-oriented upload/download plumbing, opaque new document IDs/object keys, content-addressed `ObjectBlob` identity, original `DocumentVersion` records, and initial `IngestionRun` records for stored uploads. New Matter can upload pasted narrative and text-like files after live matter creation.
- Still needed: R2 evidence-lake artifact manifests, real binary parsing/extraction strategy, richer upload progress, user-facing duplicate groups, parser status pipeline, and extraction retry/failure UX.
- Status: Partial

### PARTIAL-003 - Extraction
- Priority: P1
- Area: Document understanding
- Current behavior: Backend deterministic extraction chunks supplied text, stores extracted text nodes, summarizes text, creates proposed facts linked to the source document, records source spans with byte/character offsets and quotes, and updates ingestion run state to `review_ready` or `failed`. OCR and binary parsing are deferred. AI extraction is provider-gated and not live.
- Still needed: R2 extraction manifests/artifacts, PDF/DOCX/XLSX extraction, richer CSV/HTML extraction, OCR provider, extraction review queue UI, entity/date/money extraction, contradiction checks, citation candidates, and automatic graph-build completion state beyond the V0 synchronous path.
- Status: Partial

### PARTIAL-004 - Drafting and checks
- Priority: P1
- Area: Drafting studio
- Current behavior: Deterministic draft scaffold, unsupported-fact checks, and missing-citation checks exist on backend. Frontend can create drafts, edit/save section bodies, call scaffold generation, and run support checks.
- Still needed: Paragraph/sentence finding UI, source-linked check results, unsaved-change warnings, version snapshots, and provider-gated live drafting.
- Status: Partial

### PARTIAL-011 - Complaint Builder
- Priority: P1
- Area: Complaint workflow
- Current behavior: Complaint Builder can create or regenerate a `complaint` draft scaffold from the matter graph and shows the V0 complaint checklist.
- Still needed: Structured caption, court, jurisdiction, venue, counts, remedies, prayer, exhibits, verification, filing checklist, and complaint-specific QC.
- Status: Partial

### PARTIAL-005 - Authority retrieval
- Priority: P1
- Area: ORSGraph bridge
- Current behavior: Backend authority search delegates to ORSGraph search. Frontend authorities page can run live matter-scoped authority searches and still collects linked authority refs from the matter graph.
- Still needed: Attach search results to claims/elements/drafts, currentness display, exact provision linking, definitions/deadlines/remedies/penalties panels, recommendation flow, and authority gap findings.
- Status: Partial

### PARTIAL-006 - Fact review workflow
- Priority: P1
- Area: Fact table
- Current behavior: Proposed facts can be approved, edited, disputed, or rejected from the facts board through live API mutations.
- Still needed: Merge duplicate facts, attach evidence during review, batch review controls, source spans/quotes, and mutation regression tests.
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
- Current behavior: Users can create live claims/counterclaims/defense-labeled theories with elements and a starting supporting fact, then run element mapping.
- Still needed: Edit/delete, richer element templates, evidence and authority attachment, scoring, and matrix synchronization.
- Status: Partial

### PARTIAL-010 - Evidence matrix persistence
- Priority: P1
- Area: Evidence
- Current behavior: Users can create evidence quotes/descriptions from documents, link them as supporting or contradicting facts from the matrix detail panel, and backend-created evidence records include structured `SourceSpan` provenance when document provenance is available.
- Still needed: UI document-span capture, fact-review attachment, edit/delete, exhibit numbering, admissibility notes, and richer claim/element edge updates.
- Status: Partial

### PARTIAL-012 - Graph, QC, and export surfaces
- Priority: P1
- Area: Production wiring
- Current behavior: Graph, QC, and export pages render route-ready shells or derived summaries. Backend export endpoints are explicit V0.2 stubs.
- Still needed: Matter-scoped graph API, interactive graph renderer, QC run endpoint, finding lifecycle, persisted gap/contradiction nodes, export package pipeline, DOCX/PDF generation, and packet preview/download status.
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

## Not yet real

- No live AI provider execution for fact extraction, issue spotting, drafting, fact checking, or citation checking.
- No binary multipart upload endpoint; signed binary upload intent exists, but binary parsing/extraction is not production-complete.
- No R2 artifact manifest layout for original/text/pages/OCR/redaction/export artifacts.
- No R2 event notification ingestion queue or lifecycle/storage-class policy.
- No OCR/transcription.
- No issue spotting endpoint or claim/defense suggestion queue.
- No sentence-level `DraftSentence` support model.
- No matter graph API or interactive CaseBuilder graph renderer.
- No QC run endpoint or persisted gap/contradiction lifecycle.
- No production indexing harness for 100 to 1,000+ mixed files.
- No DOCX/PDF export.
- No e-filing.
- No multi-user collaboration.
- No attorney review mode.
- No case-law integration.
- No court-rule integration.
- No production authentication beyond optional API key.
- No audit log, retention settings, or user-facing matter deletion lifecycle UI.
