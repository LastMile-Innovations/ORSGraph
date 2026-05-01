# 02 - V0 MVP Workflows

V0 MVP should let a single user create a matter, add files, extract text, review facts, build a timeline, map claims and evidence, retrieve ORS authority, create a complaint-profile WorkProduct, and run support checks.

## CB-V0-001 - Matter dashboard
- Priority: P0
- Area: Matter workspace
- Problem: The dashboard must be useful with both seeded demo data and live matter records.
- Expected behavior: Shows parties, documents, facts, events, claims, defenses, deadlines, tasks, drafts, evidence gaps, and next actions.
- Implementation notes: Keep dashboard derived from one canonical `Matter` object.
- Acceptance checks: Empty, demo, and live matters render without type drift or hidden mock state.
- Dependencies: `CB-V0F-008`.
- Status: Partial

## CB-V0-002 - File upload UI
- Priority: P0
- Area: File ingestion
- Problem: Backend upload contract exists, but the frontend document library is still not a real uploader.
- Expected behavior: User can select/drop files, set document type/folder/confidentiality, upload, see progress, and see failure states.
- Implementation notes: Start with text/plain and pasted text, then add multipart binary support.
- Acceptance checks: Uploading text or binary creates a `CaseDocument` and displays it in the library with truthful processing state.
- Dependencies: `CB-V0F-011`.
- Status: Partial
- Progress: New Matter and Document Library can upload selected files through the binary multipart endpoint. Text-like files can auto-extract; PDFs/images/other unsupported binaries are stored privately and shown as queued/unsupported rather than processed.
- Still needed: Drag/drop progress, duplicate-detection UX, richer extraction status, retry/failure actions, and batch upload polish.

## CB-V0-003 - Binary upload endpoint
- Priority: P0
- Area: File ingestion
- Problem: JSON text upload is not enough for PDFs, DOCX, images, and spreadsheets.
- Expected behavior: API accepts multipart uploads and stores binaries locally with metadata, provenance, and hash.
- Implementation notes: Add size limits, MIME sniffing, extension validation, and safe filenames.
- Acceptance checks: PDF/DOCX/CSV/XLSX/image files create queued or processed document records without exposing unsafe paths.
- Dependencies: `CB-V0F-006`.
- Status: Done
- Completed: Added `POST /api/v1/matters/:matter_id/files/binary` with matter-scoped multipart handling, max body limit, MIME/filename validation, local private byte storage, SHA-256 hashing, `ObjectBlob`/`DocumentVersion`/`IngestionRun` provenance, extractable UTF-8 text handling, and explicit unsupported state for non-text binaries.
- Verification: `cargo test -p orsgraph-api casebuilder`, `cargo test -p orsgraph-api casebuilder_routes_cover_v0_contracts`, and `node --check scripts/smoke-casebuilder.mjs`.

## CB-V0-004 - Text extraction service
- Priority: P1
- Area: Document understanding
- Problem: V0 only extracts supplied text and text-like UTF-8 uploads.
- Expected behavior: Extract text from TXT, Markdown, and HTML-like UTF-8 uploads in V0; keep PDF/DOCX/XLSX/OCR parsing deferred.
- Implementation notes: Keep extraction deterministic and provider-free before adding OCR.
- Acceptance checks: Fixture files produce extracted text chunks and document summaries.
- Dependencies: `CB-V0-003`.
- Status: Partial
- Progress: Text-backed documents and text-like UTF-8 binary uploads can be chunked deterministically, stored as `ExtractedText`, summarized, and re-extracted from the document viewer.
- Still needed: PDF, DOCX, XLSX, richer CSV/HTML parsing fixtures, extraction contract tests, and provider-free metadata extraction for dates/parties/money/citations.

## CB-V0-005 - OCR deferred state
- Priority: P1
- Area: Document understanding
- Problem: Images and scanned PDFs cannot be silently treated as processed.
- Expected behavior: Unsupported OCR cases show queued/unsupported state with clear next action.
- Implementation notes: Do not fabricate text from images without OCR provider.
- Acceptance checks: Image upload does not claim text extraction succeeded.
- Dependencies: `CB-V0-003`.
- Status: Done
- Completed: Non-text binary uploads are persisted but marked queued/unsupported rather than extracted. Frontend upload/document views expose unsupported status so OCR and scanned-PDF limitations are explicit.
- Still needed: OCR provider configuration and `queued_for_ocr` processing belong to the V0.1/V0.2 ingestion lanes.

## CB-V0-006 - Proposed fact extraction
- Priority: P1
- Area: Fact extraction
- Problem: Users need documents converted into reviewable facts.
- Expected behavior: Extraction creates proposed facts with source document IDs, spans/quotes, confidence, and status.
- Implementation notes: Start deterministic for sentence extraction, then provider-gate AI extraction.
- Acceptance checks: User can review proposed facts and see source support.
- Dependencies: `CB-V0-004`.
- Status: Partial
- Progress: Deterministic extraction now creates `proposed` fact nodes from document sentences, links them to the source document, returns them in extraction responses, records source spans/quotes, and updates `facts_extracted`.
- Still needed: Duplicate detection, richer confidence scoring, entity/date extraction, contradiction detection, and provider-gated AI extraction.

## CB-V0-007 - Fact review and approval
- Priority: P1
- Area: Fact table
- Problem: Backend has approve endpoint, but the frontend is read-only.
- Expected behavior: User can approve, edit, reject, mark disputed, merge duplicates, and attach evidence.
- Implementation notes: Wire `PATCH /facts/:factId` and `/approve` before advanced merge UI.
- Acceptance checks: Approved fact persists and dashboard/fact board updates.
- Dependencies: `CB-V0F-011`, `CB-V0-006`.
- Status: Partial
- Progress: Facts board can approve proposed facts, edit statements, mark facts disputed, reject facts, show source spans/quotes, and refresh from the live API. Backend approval now raises confidence and clears `needs_verification`.
- Still needed: Merge duplicates, attach evidence from the review panel, batch review, deep-link opening to exact document spans, and regression tests for fact mutations.

## CB-V0-008 - Timeline builder
- Priority: P1
- Area: Timeline
- Problem: Timeline page displays data but does not create events from live facts/documents.
- Expected behavior: User can create events, link facts/documents/parties, and sort/filter chronology.
- Implementation notes: Use `POST /timeline` and date confidence fields.
- Acceptance checks: Created event persists and appears in timeline/dashboard.
- Dependencies: `CB-V0F-011`.
- Status: Partial
- Progress: Timeline page now includes live event creation with date, title, kind, description, optional source document, and optional linked fact; created events render alongside facts/documents/deadlines/milestones.
- Still needed: Edit/delete events, party/claim links in the UI, calendar view, contradiction/missing-date filters, and automated event extraction from approved facts/documents.

## CB-V0-009 - Party extraction and party map
- Priority: P1
- Area: Parties
- Problem: Parties page exists, but party extraction and mutation are not wired.
- Expected behavior: Extracted people/orgs become proposed parties; user can add/edit parties and link evidence/facts.
- Implementation notes: Start with manual party creation and document entity suggestions.
- Acceptance checks: Party appears in matter sidebar counts, party page, facts, and timeline links.
- Dependencies: `CB-V0-004`, `CB-V0F-011`.
- Status: Partial
- Progress: Parties page now has a live manual party/entity creation form with role, type, counsel, contact fields, notes, and refresh from API.
- Still needed: Entity extraction suggestions from documents, edit/delete, fact/evidence links, de-dupe, and graph visualization.

## CB-V0-010 - Claim builder live persistence
- Priority: P1
- Area: Claims
- Problem: Claim builder renders demo data but does not persist live claims/elements from UI.
- Expected behavior: User can create claims, add elements, map facts/evidence, and map elements.
- Implementation notes: Wire `POST /claims` and `POST /claims/:claimId/map-elements`.
- Acceptance checks: Claim persists, element statuses update, and evidence matrix reflects mappings.
- Dependencies: `CB-V0F-011`, `CB-V0-007`.
- Status: Partial
- Progress: Claims page can create live claims/counterclaims/defense-labeled theories with elements and an initial supporting fact, call live element mapping, and receive synced evidence IDs when evidence is linked to satisfying facts. Authority can be attached to claim and element targets.
- Still needed: Edit/delete claims and elements, richer element templates, evidence mapping controls, strength scoring, and evidence matrix synchronization polish.

## CB-V0-011 - Evidence matrix live persistence
- Priority: P1
- Area: Evidence
- Problem: Evidence matrix is read-oriented and demo-backed.
- Expected behavior: Evidence can be created from document spans and linked as supporting or contradicting facts.
- Implementation notes: Wire `POST /evidence` and `/evidence/:evidenceId/link-fact`.
- Acceptance checks: Support/contradiction edges persist and fact/detail pages update.
- Dependencies: `CB-V0F-011`, `CB-V0-002`.
- Status: Partial
- Progress: Evidence matrix can create evidence quotes/descriptions from a selected document, link them as support or contradiction to facts, show linked evidence in the element detail panel, and trigger backend synchronization across evidence, facts, claims, and claim elements.
- Still needed: Create evidence directly from document spans, attach evidence during fact review, edit/delete evidence, exhibit numbering, admissibility notes, and richer claim/element edge editing.

## CB-V0-012 - Authority search panel
- Priority: P1
- Area: ORS authority
- Problem: Backend authority search exists, but the frontend does not expose live matter-scoped search/recommend.
- Expected behavior: User can search ORSGraph from within a matter and attach authorities to claims/elements/draft paragraphs.
- Implementation notes: Use `/authority/search` first, `/authority/recommend` for selected text later.
- Acceptance checks: Search results show citation, canonical ID, currentness/status, snippet, and link to statute/provision.
- Dependencies: `CB-V0F-011`.
- Status: Partial
- Progress: Authorities page now has a live ORSGraph search panel with query input, result snippets, citations/canonical IDs, scores, warnings, error state, target selectors, and attach controls for claims, elements, and draft paragraphs.
- Still needed: Currentness panels, definitions/deadlines/remedies/penalties grouping, recommendation workflow from selected text, and richer attached-authority management.

## CB-V0-013 - Complaint profile entry point
- Priority: P1
- Area: Complaint workflow
- Problem: Complaint route started as a workflow hub and must remain a friendly entry point while the shared WorkProduct Builder becomes canonical.
- Expected behavior: Provides the V0 entry point into complaint work, creates or opens a complaint-profile WorkProduct/facade, and routes users toward the structured complaint profile editor.
- Implementation notes: Keep this as the bridge into `10-complaint-editor-backlog.md` and `12-work-product-builder-backlog.md`; do not expand legacy Draft endpoints into complaint or future document AST work.
- Acceptance checks: User can create or open a complaint-profile WorkProduct from approved facts and selected claims.
- Dependencies: `CB-V0-007`, `CB-V0-010`, `CB-V0-012`, `CB-V0-014`.
- Status: Partial
- Progress: Complaint work now uses the structured complaint profile/facade, synchronizes into canonical WorkProduct AST and Case History, and keeps V0 safety framing visible.
- Still needed: Shared route/editor migration and AST production hardening move to `CB-WPB-004` through `CB-WPB-065`; complaint-specific projection/polish remains in `CB-CE-*`.

## CB-V0-014 - Drafting Studio live save
- Priority: P1
- Area: Drafting
- Problem: Legacy Draft editor displays content but does not fully match the shared WorkProduct direction.
- Expected behavior: User can create, edit, save, and reload legacy drafts through API while new legal work-product editing moves to WorkProduct Builder.
- Implementation notes: Wire `POST /drafts`, `PATCH /drafts/:draftId`, and local unsaved state warnings.
- Acceptance checks: Draft changes persist across refresh.
- Dependencies: `CB-V0F-011`.
- Status: Partial
- Progress: Drafts list can create drafts from templates; draft editor can edit section bodies and save through `PATCH /drafts/:draftId`.
- Still needed: Unsaved-change warnings, paragraph-level editing/persistence polish, status transitions, robust reload tests, and a clear legacy boundary with canonical WorkProduct AST. Do not add `WorkProductDraft` unless branch/current-draft state needs a separate record.

## CB-V0-015 - Draft generation scaffold
- Priority: P1
- Area: Drafting
- Problem: Backend deterministic scaffold exists, but frontend does not call it.
- Expected behavior: User can generate a source-linked draft scaffold with visible provider-disabled/template mode.
- Implementation notes: Display action result metadata so users know live AI was not used.
- Acceptance checks: Generate button creates/updates paragraphs with fact/evidence/authority links.
- Dependencies: `CB-V0-014`.
- Status: Partial
- Progress: Drafts list and draft editor can call backend template scaffold generation and display provider-disabled/template-mode messages.
- Still needed: Better source link review, authority insertion, visible paragraph/fact/evidence linkage, and migration of legal drafting scaffolds into WorkProduct templates/profiles that emit AST patches.

## CB-V0-016 - Sentence and paragraph support checks
- Priority: P1
- Area: Fact checking
- Problem: Backend deterministic checks exist, but UI is not wired to live findings.
- Expected behavior: Draft editor can run fact-check and citation-check, then show findings by paragraph/sentence.
- Implementation notes: Start paragraph-level because current backend model is paragraph-oriented; add sentence nodes later.
- Acceptance checks: Unsupported fact and missing citation findings display and link to draft paragraphs and sources.
- Dependencies: `CB-V0-014`, `CB-V0-015`.
- Status: Partial
- Progress: Draft editor can run backend deterministic fact-check and citation-check actions, surface action messages, and render persisted fact/citation findings alongside the draft. WorkProduct QC and AST validation can attach findings/warnings to AST targets, and QC also renders persisted findings from the matter.
- Still needed: Sentence-level support graph, link each finding to remediation evidence/authority, full resolve/ignore/reopen lifecycle, and reusable AST warning marks in the shared editor.

## CB-V0-017 - Safety copy and no-legal-advice framing
- Priority: P0
- Area: Product safety
- Problem: CaseBuilder must not imply it is a lawyer or that output is filing-ready.
- Expected behavior: AI, drafting, fact-checking, and export surfaces clearly distinguish legal information, strategy, and review needs.
- Implementation notes: Avoid noisy disclaimers on every card; place persistent trust affordances where decisions happen.
- Acceptance checks: Complaint/draft/fact-check/export pages do not imply legal advice or court readiness.
- Dependencies: None.
- Status: Partial
- Progress: New Matter now includes explicit non-lawyer/non-filing-ready framing and truthfully labels text-only upload behavior.
- Still needed: Add persistent safety/trust affordances to drafting, complaint, fact-check, authority, and export decision points.

## CB-V0-018 - V0 workflow smoke test
- Priority: P1
- Area: Quality
- Problem: The end-to-end V0 path needs automated coverage.
- Expected behavior: Smoke test covers create matter, upload text, extract, create/approve fact, create event, create claim, map element, search authority, create draft, run checks.
- Implementation notes: Backend integration tests may need Neo4j test fixture or mocked service boundary.
- Acceptance checks: CI/dev script fails on broken V0 path.
- Dependencies: `CB-V0F-012`, `CB-V0-002` through `CB-V0-016`.
- Status: Partial
- Progress: Added `pnpm run smoke:casebuilder`, which creates a temp live matter, uploads a text file, extracts/reviews facts, creates timeline/claim/evidence records, attaches authority, creates/generates a draft, runs deterministic checks, verifies export-deferred response, creates an AST-backed WorkProduct, applies an AST patch, validates, converts to markdown/html/plain text, and cleans up.
- Still needed: Run against a live local/API environment and wire into CI once the Neo4j/API fixture is dependable.

## CB-V0-019 - Ingestion job and status pipeline
- Priority: P0
- Area: File ingestion
- Problem: Upload, extraction, review, and graph-build state is scattered across document status fields and UI-derived state.
- Expected behavior: Each upload creates an `IngestionRun`/job with stages for stored, text extracted, artifacts written, facts proposed, entities proposed, review needed, graph updated, failed, or unsupported.
- Implementation notes: Keep V0 synchronous where necessary, but persist run records and stage timestamps so the UI can poll and recover after refresh. Each run records input object hash, extractor version, provider/model if any, produced object keys, produced node IDs, and error state.
- Acceptance checks: Uploading a document shows deterministic progress and ends in either review-ready, processed, failed, or unsupported without silent success.
- Dependencies: `CB-V0F-017`, `CB-V0-002`, `CB-V0-003`, `CB-V0-004`.
- Status: Partial
- Progress: Backend and frontend DTOs now include `IngestionRun`; text uploads and binary uploads create initial ingestion run records, deterministic extraction updates runs to `review_ready`, empty/unextractable text updates runs to `failed`, unsupported binary formats remain explicit, and extraction responses return the run metadata.
- Still needed: Pollable multi-stage job endpoint, parser/artifact stage timestamps, async queue integration, retry actions, user-facing progress UI, quarantine states, and manifest-backed reruns.

## CB-V0-020 - Source span and provenance model
- Priority: P0
- Area: Evidence/provenance
- Problem: Facts, evidence, WorkProduct blocks, and future sentence checks need exact source anchors, not only document IDs or free-text quotes.
- Expected behavior: Add a shared `SourceSpan` DTO for document version, object blob, page, text chunk, byte/character offsets, quote, extraction method, confidence, and review status.
- Implementation notes: Use `SourceSpan` from extraction through fact review, evidence creation, WorkProduct support, and findings. Every derived graph node should trace to the exact R2 object/version/page/chunk/span when available.
- Acceptance checks: Every proposed fact and evidence item can open the source document at the supporting span or show an explicit unavailable-source reason.
- Dependencies: `CB-V0F-014`, `CB-V0F-015`, `CB-V0-004`, `CB-V0-006`, `CB-V0-011`.
- Status: Partial
- Progress: Backend and frontend DTOs now include `SourceSpan`; deterministic text extraction emits chunk and proposed-fact spans with document version, blob, ingestion run, page, byte/character offsets, quote, method, confidence, and review status. Fact review and document detail render available spans/quotes, and manual evidence creation stores structured span provenance when document provenance is available.
- Still needed: Viewer deep-linking to spans, evidence capture from selected document text, sentence-level draft support spans, unavailable-source reasons for legacy/user-entered facts, and R2/page/chunk coordinates for binary-derived artifacts.

## CB-V0-021 - Binary extraction fixture coverage
- Priority: P1
- Area: Document understanding
- Problem: Production support for PDFs, DOCX, XLSX, CSV, and HTML needs repeatable fixtures before provider/OCR work.
- Expected behavior: Fixture documents produce R2-backed normalized extracted text artifacts, source spans, summaries, metadata, and clear unsupported states for scanned/image-only content.
- Implementation notes: Keep OCR out of V0 unless configured; do not mark scanned documents as processed when text is unavailable. Neo4j should store graph-ready chunks and provenance metadata, not the full normalized artifact payload.
- Acceptance checks: Fixture tests cover PDF text, DOCX paragraphs, XLSX sheets, CSV rows, HTML text, image-only upload, and failed parse behavior.
- Dependencies: `CB-V0F-017`, `CB-V0-003`, `CB-V0-004`, `CB-V0-020`.
- Status: Todo

## CB-V0-022 - Document extraction review queue
- Priority: P1
- Area: Fact review
- Problem: Proposed facts, parties, dates, amounts, and citation candidates need a single review workflow before they become trusted graph nodes.
- Expected behavior: User can review extraction batches, approve/reject/edit proposed facts, accept party/date/amount suggestions, and see what remains unreviewed.
- Implementation notes: Use the ingestion job and source-span model; keep rejected suggestions for audit/debug but exclude them from active graph views.
- Acceptance checks: A newly extracted document surfaces a review queue, and approving items updates facts, parties, timeline candidates, and dashboard counts.
- Dependencies: `CB-V0-019`, `CB-V0-020`, `CB-V0-006`, `CB-V0-007`.
- Status: Todo

## CB-V0-023 - Issue spotting endpoint and review queue
- Priority: P1
- Area: Issue spotting
- Problem: Claim and defense suggestions are not live, even though they are central to CaseBuilder's promise.
- Expected behavior: Add `/api/v1/matters/:id/issues/spot` returning `IssueSuggestion` records for possible claims, defenses, deadlines, remedies, penalties, and notice requirements.
- Implementation notes: Start deterministic and authority-search based; provider-gate AI suggestions and label mode/result confidence.
- Acceptance checks: User can review a suggested claim/defense, inspect supporting facts/authority, accept it into the claim/defense builder, or reject it.
- Dependencies: `CB-V0-006`, `CB-V0-012`, `CB-X-004`, `CB-X-005`.
- Status: Todo

## CB-V0-024 - Claim/evidence/element synchronization
- Priority: P1
- Area: Claim builder
- Problem: Facts, evidence, elements, and claims can drift after manual edits or evidence links.
- Expected behavior: Mapping changes update claim elements, evidence matrix rows, fact detail, dashboard gaps, and QC inputs consistently.
- Implementation notes: Persist edges for `Fact SATISFIES_ELEMENT`, `Evidence SUPPORTS_ELEMENT`, and claim/evidence support with matter-scoped writes.
- Acceptance checks: Linking evidence to a fact that satisfies an element updates the claim builder and evidence matrix after refresh.
- Dependencies: `CB-V0-010`, `CB-V0-011`, `CB-V0-020`.
- Status: Done
- Completed: Evidence creation/linking now updates `CaseEvidence`, referenced `CaseFact` support/contradiction lists, claim evidence IDs, and claim-element evidence IDs when linked facts satisfy elements.
- Verification: `cargo test -p orsgraph-api casebuilder`.

## CB-V0-025 - Authority attachment endpoints
- Priority: P1
- Area: ORS authority
- Problem: Authority search can find provisions, but users cannot attach results to claims, elements, or draft paragraphs as durable graph support.
- Expected behavior: Add authority attach/detach endpoints for claims, elements, and draft paragraphs; add defense and sentence targets later.
- Implementation notes: Store `AuthorityRef` plus relationship edges to ORSGraph nodes when canonical IDs resolve; preserve unresolved citations as findings.
- Acceptance checks: Attached authority survives refresh, appears in authorities page, claim builder, draft editor, QC, and graph inputs.
- Dependencies: `CB-V0-012`, `CB-X-006`.
- Status: Done
- Completed: Added `POST /api/v1/matters/:matter_id/authority/attach` and `/detach`, frontend typed helpers, authority attach controls, persistence on claims/elements/draft paragraphs, and graph materialization for resolved canonical IDs.
- Verification: `cargo test -p orsgraph-api casebuilder_routes_cover_v0_contracts`, `cargo test -p orsgraph-api casebuilder`, and `./node_modules/.bin/tsc --noEmit --incremental false`.

## CB-V0-026 - Sentence-level WorkProduct support model
- Priority: P1
- Area: Drafting/fact-checking
- Problem: Current checks are paragraph-level, but the product promise requires every WorkProduct sentence to be support-checkable.
- Expected behavior: Add or materialize `SentenceBlock`/`WorkProductSentence` records with text, role, source fact IDs, evidence IDs, authority refs, support status, and finding anchors.
- Implementation notes: Derive sentences from AST paragraph blocks in V0, then make the shared WorkProduct editor sentence-aware once the model is stable. This is a shared dependency for complaint paragraph/sentence checks, AI drafting, and citation anchors.
- Acceptance checks: Fact-check and citation-check findings can target a sentence or paragraph, and the editor can display each finding at the exact text anchor.
- Dependencies: `CB-WPB-003`, `CB-WPB-010`, `CB-WPB-011`, `CB-V0-016`, `CB-V0-020`, `CB-CE-006`, `CB-CE-011`, `CB-CE-016`.
- Status: Todo

## CB-V0-027 - Graph upserts from ingestion manifests
- Priority: P1
- Area: Ingestion/graph build
- Problem: Extraction and graph updates need a reproducible bridge from R2 artifacts into Neo4j nodes and relationships.
- Expected behavior: Ingestion manifests drive idempotent upserts for pages, text chunks, entity mentions, facts, evidence spans, and source relationships.
- Implementation notes: Treat the manifest as the durable extraction contract; graph upserts should be rerunnable without duplicating nodes.
- Acceptance checks: Replaying the same manifest creates or updates the same graph nodes, preserves produced node IDs on the `IngestionRun`, and does not duplicate facts/chunks/spans.
- Dependencies: `CB-V0F-017`, `CB-V0-019`, `CB-V0-020`.
- Status: Todo
