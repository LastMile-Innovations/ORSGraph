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
- Retired generic `draft_type=complaint` creation in favor of `/api/v1/matters/:matterId/complaints`.
- Verification: `cargo test -p orsgraph-api`, `pnpm run typecheck`, `pnpm run lint`, and `pnpm run build` passed.

Follow-up implementation update on 2026-05-01 for graph-native Case History V0:

- Added canonical work-product version DTOs, Neo4j constraints/indexes, deterministic hash helpers, snapshot manifests, entity-state records, change sets, version changes, AI audit records, and immutable export snapshot metadata.
- Added canonical work-product Case History endpoints for history, change-set detail, snapshot list/detail/create, compare, restore, export history, and AI audit, plus complaint aliases that delegate to the same handlers.
- Wired complaint/work-product create, edit, support link, QC, AI, export, and restore flows into canonical Case History.
- Added a Complaint workspace Case History screen with timeline, manual snapshots, text compare, restore dry-run/apply, and changed-since-export status.
- Verification: `cargo check -p orsgraph-api`, `cargo test -p orsgraph-api --test graph_contract`, `cargo test -p orsgraph-api work_product_hashes_are_stable_and_layered --lib`, and `pnpm run check` from `frontend/` passed.

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
- Current behavior: Deterministic draft scaffold, unsupported-fact checks, and missing-citation checks exist on backend. Frontend can create drafts, edit/save section bodies, call scaffold generation, run support checks, and render persisted fact/citation findings in the draft editor and QC page.
- Still needed: Sentence-level finding UI, source-linked check remediation, unsaved-change warnings, richer Case History diff/restore layers, and provider-gated live drafting.
- Status: Partial

### PARTIAL-011 - Complaint Editor / Builder entry point
- Priority: P1
- Area: Complaint workflow
- Current behavior: Complaint Editor is now the canonical complaint work product. It creates a structured complaint AST with caption, sections, counts, paragraphs, support/citation/exhibit links, deterministic QC, preview, export artifacts, history, AI template states, filing packet state, and no-seed defaults.
- Still needed: Shared work-product editor abstraction for motions, answers, declarations, briefs, and future rich-text profiles is tracked in `CB-CE-028`.
- Status: Done

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
- Current behavior: Graph and export pages render route-ready shells or derived summaries; QC renders derived metrics plus persisted draft fact/citation findings. Backend export endpoints are explicit V0.2 deferred stubs.
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

### PARTIAL-015 - Case History hardening
- Priority: P0
- Area: Legal version control
- Current behavior: V0 history foundation is wired and visible in the Complaint workspace.
- Still needed: Remove/demote flat history persistence, add matter-isolation tests, add support/citation/authority/QC diff layers, add scoped restore modes, add snapshot viewer and compare modal polish, add sensitive-log guardrails, add end-to-end history smoke, then implement branch alternatives and merge cards.
- Status: Partial

## Not yet real

- No live AI provider execution for fact extraction, issue spotting, drafting, fact checking, or citation checking.
- No PDF/DOCX/XLSX/image parsing or OCR-backed extraction; binary files are stored but non-text extraction remains deferred.
- No R2 artifact manifest layout for original/text/pages/OCR/redaction/export artifacts.
- No R2 event notification ingestion queue or lifecycle/storage-class policy.
- No OCR/transcription.
- No issue spotting endpoint or claim/defense suggestion queue.
- No sentence-level `DraftSentence` support model.
- No branch alternatives, merge cards, rich support/QC diff layers, scoped support restore, audit reports, or full Case History smoke coverage.
- No matter graph API or interactive CaseBuilder graph renderer.
- No QC run endpoint or persisted gap/contradiction lifecycle.
- No production indexing harness for 100 to 1,000+ mixed files.
- No DOCX/PDF export.
- No e-filing.
- No multi-user collaboration.
- No attorney review mode.
- No case-law integration.
- No production court-rule integration beyond the first deterministic Oregon complaint rule pack.
- No production authentication beyond optional API key.
- No audit log, retention settings, or user-facing matter deletion lifecycle UI.
