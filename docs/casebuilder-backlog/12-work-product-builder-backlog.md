# 12 - WorkProduct Builder Backlog

WorkProduct Builder is the shared legal document editor for CaseBuilder. It is the canonical engine for creating, editing, versioning, fact-checking, cite-checking, rule-checking, exporting, and printing legal work product.

The WorkProduct AST refactor has landed the core architectural decision:

```text
WorkProduct.document_ast = canonical current legal document
```

`WorkProduct.blocks`, `marks`, `anchors`, and `findings` remain only compatibility/projection surfaces while old callers migrate. New legal document work must read and write the AST or AST patches.

Complaint is the first concrete profile/facade:

```text
WorkProduct(type = "complaint")
```

The same AST, support graph, rule finding model, Case History, AI command surface, and export pipeline should later power answers, counterclaims, motions, declarations, affidavits, demand letters, notices, legal memos, exhibit lists, filing packets, proposed orders, discovery, settlement letters, and appeal briefs.

## Current Code Baseline

- Backend `WorkProduct` now carries `document_ast: WorkProductDocument`.
- `WorkProductDocument` includes schema version, document/work-product/matter IDs, product type, title, metadata, nested blocks, structured links, citation uses, exhibit references, rule findings, and timestamps.
- `WorkProductBlock` is AST-capable with stable `block_id`, serialized `type`, `order_index`, optional `parent_id`, nested `children`, link/citation/exhibit/finding references, paragraph/count/section/sentence fields, support status, and legacy block projection fields.
- Structured AST records exist for `WorkProductLink`, `WorkProductCitationUse`, `WorkProductExhibitReference`, `TextRange`, `AstPatch`, `AstOperation`, AST validation responses, markdown conversion responses, and rendered AST responses.
- Existing create/update/block/support/QC/export/history paths refresh the AST and then rebuild old projections, rather than treating the flat arrays as truth.
- WorkProduct support links have first-class create, update, and delete API routes; relation/status changes and removals rebuild the AST links, compatibility projections, and Case History support-use changes.
- Text-range support/citation/exhibit links can be written through a dedicated WorkProduct route that adds AST `source_text_range` records and records Case History changes.
- Existing block routes remain adapters. `POST /work-products/:id/ast/patch` is the canonical patch path for operations that flat block endpoints cannot express cleanly.
- AST patch operations exist for insert/update/delete/move/split/merge blocks, paragraph renumbering, links, citations, exhibits, rule findings, and template application.
- AST validation checks schema version, duplicate/missing block IDs, parent integrity, broken block references, unresolved citations/exhibits, and first-pass complaint structure warnings.
- Markdown, HTML, and plain-text conversion endpoints exist. Markdown conversion is intentionally simple and warning-aware; rich metadata round-trip remains future work.
- Case History hashes, manifests, snapshots, compare, restore, export artifacts, changed-since-export, and AI audit now include or derive from `document_ast`.
- Compare now returns bounded `layer_diffs` for support, citations, exhibits, rule findings, formatting, and exports alongside text diffs.
- Scoped restore can restore selected AST blocks, metadata, support links, citations, exhibits, rule findings, formatting, and export state without replacing unrelated current edits.
- Neo4j materialization writes AST block, link, citation, exhibit, and rule-finding state into existing WorkProduct/Version/Support labels where possible.
- Matter-owned support references are validated before support-link and AST-patch writes, and object-backed snapshot/export hydration uses matter-scoped `ObjectBlob` lookup.
- Frontend CaseBuilder types and API helpers normalize `document_ast`, flatten AST blocks for legacy screens, and expose AST patch/validate/convert helpers.
- Smoke coverage creates a WorkProduct, verifies `document_ast`, applies an AST patch, validates, and converts to markdown/html/plain text.
- The 2025 UTCR corpus now generates source-backed procedural requirement nodes, a court-paper formatting profile, and WorkProduct rule packs for general documents, complaints, motions, answers, declarations, and filing packets. See [../legal-corpora/2025-utcr-ingestion.md](../legal-corpora/2025-utcr-ingestion.md).

## Important Architecture Decisions

- The current canonical draft is `WorkProduct.document_ast`.
- Do not add `WorkProductDraft` unless branch/current-draft state actually needs a separate record. Case History branches and snapshots already cover the version subject for this slice.
- `blocks`, `marks`, and `anchors` are not separate truth. They are compatibility projections or route adapters during migration.
- Rule findings target AST IDs. Existing `WorkProductFinding` behavior is the current implementation of AST-targeted `RuleCheckFinding`.
- WorkProduct rule findings should cite both the procedural requirement node and the source UTCR provision when the finding is based on UTCR.
- AI drafting must converge on `AstPatch` proposals. Provider-free/template AI paths are allowed, but accepted AI edits should record patches and Case History changes.
- PDF/DOCX remain review-needed placeholder/skeleton exports until the dedicated renderers and visual checks exist.

## Hybrid Graph/R2 Storage Boundary

- Neo4j is authoritative for current legal meaning: WorkProduct identity, current AST shape, block hierarchy, support/citation/exhibit/rule relationships, queryable history, hashes, and ownership edges.
- R2/local `ObjectStore` is authoritative for heavy immutable bytes: uploaded evidence, extraction artifacts, large snapshot JSON, render/projection caches, export outputs, filing packets, thumbnails, OCR/page manifests, and large block-content fallbacks.
- Public editor/detail APIs still return hydrated `WorkProduct.document_ast`. List/search/QC/history views should stay graph-bounded and use summaries unless callers explicitly request the full AST.
- Storage refs such as `manifest_ref`, `full_state_ref`, `SnapshotManifest.storage_ref`, `SnapshotEntityState.state_ref`, and export artifact refs must hold `ObjectBlob` IDs, not raw R2 keys.
- Object keys must be opaque or hash-scoped. Never place filenames, party names, case titles, draft text, or legal facts in R2 keys.

## Architecture Target

Every legal document should use the same pipeline:

```text
Matter
-> WorkProduct
-> WorkProductDocument AST
-> Rich text / Markdown / preview projections
-> Facts / evidence / authority / exhibit links
-> Rule checks and QC findings
-> Version snapshots
-> Export artifacts
```

Future branch support may introduce:

```text
VersionBranch -> current snapshot -> current WorkProduct.document_ast
```

Only add a separate `WorkProductDraft` record if branches need multiple simultaneously editable draft bodies outside the current WorkProduct.

## Completion Roadmap

The AST foundation is not complete until all projections and downstream systems can trust it without special-case fallbacks.

Release gates for a complete AST platform:

1. Schema: typed block invariants, version migration, canonical serialization, stable ID policy, and fixture corpus are locked.
2. Mutation: every edit path writes `AstPatch` or full `document_ast` replacement with validation, concurrency checks, and history.
3. Projections: rich text, markdown, HTML, plain text, preview, PDF, DOCX, and graph materialization are views of the AST.
4. Legal intelligence: facts, evidence, citations, exhibits, authorities, rule findings, QC, AI, and version diffs attach to AST node IDs and text ranges.
5. Frontend: one shared WorkProductEditor reads/writes AST and renders profile-specific panels without duplicating document logic.
6. Safety: matter isolation, validation, unresolved citation/exhibit warnings, unsupported allegation warnings, export warnings, and no-legal-advice copy are enforced consistently.
7. Performance: large matters and long documents use bounded reads, incremental rendering, cacheable projections, and compact snapshots.
8. Tests: unit, fixture, property, contract, smoke, visual, export, migration, security, and performance checks cover the AST and every projection.

## Backlog

## CB-WPB-001 - Canonical WorkProduct Builder backlog integration
- Priority: P0
- Area: Planning/backlog
- Problem: Complaint, answer, motion, declaration, export, QC, and draft work can drift into separate builder stacks.
- Expected behavior: This file is the canonical backlog for shared WorkProduct Builder work. Complaint-specific work remains in `10-complaint-editor-backlog.md` as the first profile/facade.
- Implementation notes: Cross-link V0/V0.1/V0.2/V1 items to `CB-WPB-*` whenever a task touches editor, versioning, rule packs, support links, export, or document-type templates.
- Acceptance checks: README, current status, feature inventory, and phase backlogs point future document-builder work here.
- Dependencies: None.
- Status: Done

## CB-WPB-002 - Current draft and branch model boundary
- Priority: P0
- Area: Data model/API
- Problem: Earlier backlog language assumed `WorkProductDraft` was required before AST work. The implemented refactor intentionally made `WorkProduct.document_ast` the canonical current draft.
- Expected behavior: Keep `WorkProduct.document_ast` as the current draft. Add a first-class `WorkProductDraft` only if branch alternatives or simultaneous drafts need a separate editable state record.
- Implementation notes: Case History `VersionBranch` and `VersionSnapshot` remain the branch/version source. Do not resurrect generic complaint drafts as legal document truth.
- Acceptance checks: New legal document writes update `document_ast`; branch features document why a separate draft table/node is needed before adding one.
- Dependencies: Case History V0, current `WorkProduct` DTOs.
- Status: Deferred

## CB-WPB-003 - Canonical WorkProductDocument AST contract
- Priority: P0
- Area: AST/editor
- Problem: Legal work product needs a structured document model, not a rich-text blob or flat block array.
- Expected behavior: `WorkProductDocument` stores schema version, metadata, typed/nested blocks, support links, citations, exhibit references, rule findings, stable IDs, and timestamps.
- Implementation notes: The V0 implementation uses one AST-capable `WorkProductBlock` struct with serialized `type`/`order_index` and block-kind fields. A stricter enum/registry can be extracted after more profiles land.
- Acceptance checks: WorkProduct get/patch returns `document_ast`; AST patches mutate canonical state; old block projections rebuild from the AST; tests cover validation, patching, markdown conversion, hash layers, and route contracts.
- Dependencies: `CB-X-001`, `CB-X-013`.
- Status: Done
- Completed: Backend/frontend DTOs, service normalization, projection rebuild, patch application, validation, conversion helpers, hash/snapshot integration, graph materialization, and smoke coverage are implemented.

## CB-WPB-004 - Template and profile registry
- Priority: P0
- Area: Templates
- Problem: WorkProduct types need shared templates and profile metadata instead of hard-coded editor branches.
- Expected behavior: Typed templates/profiles for complaint, answer, motion, declaration, affidavit, demand letter, legal memo, notice, exhibit list, filing packet, and proposed order.
- Implementation notes: Profiles specify required/optional block roles, default rule pack, formatting profile, export profiles, editor modes, and specialized panels.
- Acceptance checks: Creating each supported type from a template produces the expected outline and review-needed state without custom route logic per type.
- Dependencies: `CB-WPB-003`.
- Status: Partial
- Progress: Existing profile/template helpers create WorkProduct skeletons and complaint/motion behavior uses shared WorkProduct state. A typed registry and complete profile templates remain.

## CB-WPB-005 - Shared WorkProduct frontend routes
- Priority: P0
- Area: Frontend/routing
- Problem: The frontend has complaint routes and generic draft routes, but not the canonical WorkProduct Builder route family.
- Expected behavior: Add routes for `/casebuilder/matters/:matterId/work-products`, `/new`, `/:workProductId`, `/:workProductId/editor`, `/qc`, `/preview`, `/export`, and `/history`.
- Implementation notes: Friendly aliases such as `/complaint`, `/answer`, `/motion`, `/declaration`, and `/memo` should resolve to a typed WorkProduct and then use the same editor.
- Acceptance checks: Route smoke covers list, new, editor, QC, preview, export, history, and friendly aliases without losing matter/work-product context.
- Dependencies: `CB-WPB-004`, current route helpers.
- Status: Done
- Completed: Added canonical WorkProduct routes for list, new/template flow, detail, editor, QC, preview, export, and history, with `/answer`, `/motion`, `/declaration`, and `/memo` aliases resolving into existing WorkProducts or the typed new-product flow. Route smoke now covers the shared route family and aliases.

## CB-WPB-006 - Shared WorkProduct dashboard and template picker
- Priority: P1
- Area: Frontend/product
- Problem: Users need a matter-level documents/drafts/filings entry point rather than separate builder entry points.
- Expected behavior: Show all work products for a matter with type, status, support/QC state, updated date, template, current branch, export status, and next action. New WorkProduct flow chooses type and template.
- Implementation notes: Use user-facing wording: Documents, Drafts, Filings, Work Product. Keep demo/live/offline labels visible.
- Acceptance checks: Empty, demo, and live matters can create or open a work product; complaint appears as a complaint-profile WorkProduct.
- Dependencies: `CB-WPB-005`.
- Status: Done
- Completed: Added a matter-level WorkProduct dashboard with search, status/QC/export summaries, product cards, template cards for complaint, answer, motion, declaration, legal memo, demand letter, notice, exhibit list, and proposed order, plus live `POST /work-products` creation into the shared editor route.

## CB-WPB-007 - Shared three-pane WorkProductEditor shell
- Priority: P0
- Area: Frontend/editor
- Problem: The complaint workbench shell exists, but the reusable WorkProductEditor does not.
- Expected behavior: Build a generic three-pane editor: left outline/templates/sections/versions/tasks, center editor/preview, and right facts/evidence/authority/citations/rules/formatting/AI/QC inspector.
- Implementation notes: Specialized panels such as caption builder, count builder, prayer for relief, signature block, and certificate of service appear only when the active profile requires them.
- Acceptance checks: Answer, motion, declaration, and legal memo profiles can render in the same shell without copying complaint-specific components.
- Dependencies: `CB-WPB-005`, `CB-WPB-006`.
- Status: Partial
- Progress: Added the first reusable WorkProduct workbench with left AST outline, center overview/editor/QC/preview/export/history panels, and right inspector for support counts, support-link management, QC, and provider-free commands. Rich-text schema, profile-specific inspector panels, and text-range citation/exhibit editing remain in later shared-editor tasks.

## CB-WPB-008 - Tiptap/ProseMirror schema and custom nodes
- Priority: P1
- Area: Rich text editor
- Problem: Dependencies exist, but the shared legal editor schema is not wired.
- Expected behavior: Add rich text custom nodes/marks for sections, pleading captions, numbered paragraphs, count headings, citation marks, evidence/fact/authority/exhibit chips, QC warning marks, page breaks, signatures, and certificates.
- Implementation notes: The AST remains canonical. ProseMirror JSON must round-trip with WorkProduct AST blocks and structured AST references.
- Acceptance checks: Load/edit/save/reload tests prove text, chips, marks, block roles, paragraph locks, and QC anchors survive round trip across at least complaint and motion profiles.
- Dependencies: `CB-WPB-003`, `CB-WPB-007`.
- Status: Todo

## CB-WPB-009 - Markdown mode and AST conversion
- Priority: P1
- Area: Markdown/editor
- Problem: Markdown is now available as backend conversion, but it is not yet a full editor mode or metadata-preserving round trip.
- Expected behavior: Add markdown editor, split preview, AST-to-markdown, markdown-to-AST, AST-to-HTML, and AST-to-plain-text workflows with warning-aware metadata preservation.
- Implementation notes: Preserve unsupported legal blocks as typed metadata, hidden comments, or sidecar JSON rather than silently flattening them.
- Acceptance checks: Legal memo, demand letter, motion, and complaint snippets convert predictably, with warnings for unsupported constructs and stable block IDs where metadata is supplied.
- Dependencies: `CB-WPB-003`, `CB-WPB-007`.
- Status: Partial
- Progress: Backend routes and frontend helpers exist for AST patch/validate, AST-to-markdown, markdown-to-AST, AST-to-HTML, and AST-to-plain-text. Smoke covers conversion. The shared WorkProduct workbench now includes a Markdown round-trip panel that loads the current AST as Markdown, converts edited Markdown back to `document_ast`, and saves it through the canonical WorkProduct patch path. Rich metadata preservation and split-preview editing remain.

## CB-WPB-010 - Structured support links, citations, and exhibit references
- Priority: P0
- Area: Support graph
- Problem: WorkProduct support can no longer live only in mark/anchor arrays or block-local fact/evidence arrays.
- Expected behavior: Any AST block or text range can link to facts, evidence, exhibits, case files, parties, timeline events, legal authority, ORS provisions, definitions, deadlines, penalties, remedies, and source notes.
- Implementation notes: Use `WorkProductLink`, `WorkProductCitationUse`, and `WorkProductExhibitReference` as canonical AST records. Legacy arrays remain projection only.
- Acceptance checks: Links/citations/exhibits attach to stable block IDs and text ranges, survive save/reload/history snapshots, materialize to graph edges, and cannot cross matter boundaries.
- Dependencies: `CB-WPB-003`, `CB-X-002`.
- Status: Partial
- Progress: AST records, patch operations, validation of broken references, graph materialization, frontend types, and API normalization exist. The shared WorkProduct inspector now tracks the selected AST block, can link matter facts, evidence, documents/exhibits, and authority/citation text through first-class support-link API routes, previews block-local linked sources, updates relation labels, and removes block-local links. Selected text in the editor can now create range-level support, citation, and exhibit AST records that remain visible in the inspector. Rich inline ProseMirror marks and deeper cross-matter UI coverage remain.

## CB-WPB-011 - Universal QC and AST-targeted rule findings
- Priority: P0
- Area: QC/rules
- Problem: Complaint and motion checks should not become separate rule engines.
- Expected behavior: Rule findings target exact AST node IDs for document, section, count, block, paragraph, sentence, citation, exhibit, and formatting targets.
- Implementation notes: Existing `WorkProductFinding` is the current implementation of AST-targeted `RuleCheckFinding`; migrate names only when it reduces confusion.
- Acceptance checks: One findings lifecycle drives editor overlay, QC page, tasks, history, and export readiness for all WorkProduct types.
- Dependencies: `CB-V01-015`, `CB-V01-016`, `CB-WPB-010`.
- Status: Partial
- Progress: WorkProduct QC, finding status patching, AST rule-finding storage, AST validation warnings, and Case History events exist. Universal rule-pack engine, matter-level QC run, reopen/comment lifecycle, and sentence-level findings remain.

## CB-WPB-012 - Shared AST-backed export service and profiles
- Priority: P1
- Area: Export
- Problem: Exports must consume the AST rather than editor HTML or flat block arrays.
- Expected behavior: WorkProducts export to PDF, DOCX, HTML, Markdown, plain text, JSON AST, and later filing packet ZIP using shared export profiles.
- Implementation notes: HTML/Markdown/plain text/JSON AST are the current reliable projections. PDF/DOCX stay review-needed skeletons until dedicated renderers and visual verification exist.
- Acceptance checks: Export artifacts are matter-scoped, snapshot-locked, downloadable, warning-aware, hash-backed, and show changed-since-export state.
- Dependencies: `CB-WPB-003`, `CB-WPB-011`, `CB-V02-006`, `CB-V02-007`, `CB-V02-011`.
- Status: Partial
- Progress: Existing preview/export/history paths consume AST state, conversion endpoints exist, export artifacts link to immutable snapshots and hashes, and warnings surface unresolved state. Production DOCX/PDF renderers and filing packet ZIP remain.

## CB-WPB-013 - WorkProduct AI command surface as AST patches
- Priority: P1
- Area: AI
- Problem: AI drafting commands should propose reviewable AST patches, not raw text blobs.
- Expected behavior: Shared commands support draft section, rewrite, optimize, summarize support, find missing evidence, find missing authority, fact-check, citation-check, and export-check. Outputs include `AstPatch`, facts used, evidence used, authorities used, warnings, and assumptions.
- Implementation notes: Provider-free mode remains explicit. Live provider output must never silently mark unsupported assertions as final-supported.
- Acceptance checks: AI draft rejected creates audit record but no AST change; accepted AI patch records a `VersionChange` and preserves support metadata.
- Dependencies: `CB-X-004`, `CB-X-005`, `CB-WPB-010`, `CB-CH-801`.
- Status: Partial
- Progress: `AstPatch` exists, AI audit/version plumbing exists, and provider-free AI states are explicit. AI command outputs still need to become first-class AST patch proposals.

## CB-WPB-014 - Complaint profile facade over canonical AST
- Priority: P0
- Area: Complaint profile
- Problem: Complaint is implemented as a structured facade but should not remain a separate document architecture.
- Expected behavior: Complaint friendly routes remain, but persistence, preview, export, history, support links, QC, and future editor actions use the canonical WorkProduct AST wherever possible.
- Implementation notes: Keep dedicated complaint DTOs only as view-model projections until the shared profile fully covers caption, counts, numbered paragraphs, relief, signature, certificate, filing packet, and Oregon rule checks.
- Acceptance checks: Existing complaint smoke still passes, and a complaint can also be opened through canonical WorkProduct state without losing AST/support/history.
- Dependencies: `CB-WPB-003`, `CB-WPB-005` through `CB-WPB-012`, current complaint implementation.
- Status: Partial
- Progress: Complaint routes synchronize into complaint-profile WorkProducts and Case History. Shared frontend WorkProduct routes/editor still need to replace complaint-specific UI pieces.

## CB-WPB-015 - Answer profile and response-grid integration
- Priority: P1
- Area: Answer profile
- Problem: Answer profile needs both a structured response grid and a reusable WorkProduct editor.
- Expected behavior: Parsed allegations feed an answer response grid, and accepted responses generate/update `WorkProduct(type="answer")` AST blocks.
- Implementation notes: The grid is profile-specific state; answer text, support links, QC, export, and history belong to WorkProduct.
- Acceptance checks: Editing the grid survives refresh and regenerates a consistent answer WorkProduct without duplicating editor/version/export logic.
- Dependencies: `CB-V01-001`, `CB-V01-002`, `CB-V01-017`, `CB-WPB-014`.
- Status: Todo

## CB-WPB-016 - Motion profile
- Priority: P1
- Area: Motion profile
- Problem: Motion profile should not start as another custom drafting surface.
- Expected behavior: Generic motions and specialized motion templates create `WorkProduct(type="motion")` with relief requested, legal standard, facts, argument, conclusion, signature, certificate, and optional proposed order blocks.
- Implementation notes: Use Oregon Circuit Civil Motion rule pack where source-backed; keep specialized motions as templates/rule-pack variants.
- Acceptance checks: User can create a motion from selected facts/evidence/authority, run QC, preview, export, and view history through the shared editor.
- Dependencies: `CB-V02-001`, `CB-V02-002`, `CB-WPB-011`, `CB-WPB-012`.
- Status: Partial
- Progress: Generic `WorkProduct(type="motion")` creation returns canonical AST and is covered by the smoke AST path. Motion-specific frontend profile, templates, rule pack, and editor panels remain.

## CB-WPB-017 - Declaration, affidavit, memo, letter, notice, and exhibit-list profiles
- Priority: P1
- Area: Additional profiles
- Problem: Supporting legal work products need structured editing without bespoke builders.
- Expected behavior: Add profiles for declaration, affidavit, legal memo, demand letter, settlement letter, notice, exhibit list, filing packet, and proposed order.
- Implementation notes: Keep profile-specific controls narrow: declarant identity, personal knowledge, penalty-of-perjury signature, letter recipient/deadline, notice service method, exhibit ordering.
- Acceptance checks: Each profile can create, save, QC, preview, export, and version through the shared editor.
- Dependencies: `CB-WPB-004`, `CB-WPB-007`, `CB-WPB-012`.
- Status: Todo

## CB-WPB-018 - WorkProduct graph model hardening
- Priority: P0
- Area: Graph/data integrity
- Problem: The graph now materializes AST state but still needs stronger typed nodes, constraints, and ownership tests for all legal references.
- Expected behavior: Graph nodes include WorkProduct, WorkProductSection, WorkProductBlock, WorkProductParagraph, WorkProductSentence, CitationUse, EvidenceUse, FactUse, AuthorityUse, ExhibitReference, RuleCheckFinding, FormattingProfile, ExportArtifact, VersionSnapshot, VersionChange, DraftBranch, and Milestone.
- Implementation notes: Use existing labels where possible. Do not add `WorkProductDraft` edges until the branch model needs them.
- Acceptance checks: Graph contract tests cover node labels, uniqueness constraints, ownership edges, support references, citation resolution edges, exhibit references, rule finding flags, and cross-matter rejection.
- Dependencies: `CB-X-002`, `CB-X-014`, Case History graph contracts.
- Status: Partial
- Progress: AST blocks, links, citations, exhibits, findings, snapshots, changes, support-use nodes, and export artifacts materialize through existing graph paths. Stronger constraints and matter-isolation coverage remain.

## CB-WPB-019 - Shared WorkProduct smoke and contract tests
- Priority: P0
- Area: Quality
- Problem: The shared builder is a critical product spine and needs regression coverage before more profiles land.
- Expected behavior: Provider-free tests cover create WorkProduct, choose template, patch AST, validate, link fact/evidence/authority, run QC, preview, export markdown/html/plain text, create snapshot, compare, and restore.
- Implementation notes: Route contract tests should cover AST patch/validate/conversion endpoints and frontend helper names.
- Acceptance checks: Tests fail on DTO drift, route removal, lost support links, broken markdown conversion, preview/export mismatch, or missing history events.
- Dependencies: `CB-WPB-003` through `CB-WPB-014`.
- Status: Partial
- Progress: Rust tests cover AST validation, patching, markdown conversion, hash stability, and rule-finding projection sync. Graph contract tests cover AST DTO/API names. Smoke covers create, patch, validate, and conversion. Full live support/QC/export/history smoke remains.

## CB-WPB-020 - Migration cleanup and legacy Draft boundary
- Priority: P0
- Area: Migration
- Problem: Generic `Draft` and complaint-specific facades remain useful compatibility surfaces, but they must not become competing sources of truth.
- Expected behavior: Define when to keep, migrate, or retire generic Draft records and complaint DTO projections. New legal document types use WorkProduct AST by default.
- Implementation notes: Keep non-legal scratch drafts only if product requires them; otherwise migrate drafting studio concepts into WorkProduct templates/profiles and Case History.
- Acceptance checks: No new answer/motion/declaration/letter work is implemented on generic Draft-only endpoints, and docs clearly identify WorkProduct AST as canonical.
- Dependencies: `CB-WPB-014`, `CB-WPB-015`, `CB-WPB-016`, `CB-WPB-017`.
- Status: Partial
- Progress: Generic draft routes are legacy adapters and complaint uses a WorkProduct facade. Old projections and flat histories still need cleanup before launch.

## CB-WPB-021 - Extract AST helper modules
- Priority: P1
- Area: Backend/refactor
- Problem: The first AST slice intentionally refactored `CaseBuilderService` in place before extracting modules.
- Expected behavior: Extract focused helpers for AST validation, patch application, markdown conversion, HTML/plain-text rendering, diff/hash helpers, citation resolver reuse, support linking, and AI patch shaping.
- Implementation notes: Preserve behavior and tests first. Extract only after route contracts and smoke stay green.
- Acceptance checks: Module extraction changes no API payloads, no history hashes, and no smoke outcomes.
- Dependencies: `CB-WPB-003`, `CB-WPB-019`.
- Status: Todo

## CB-WPB-022 - Strict schema validation and migration harness
- Priority: P1
- Area: AST/data integrity
- Problem: V0 validation catches core errors, but production needs stricter schema and migration behavior.
- Expected behavior: Validate allowed block kinds, required fields per block type, legal section requirements per WorkProduct type, parent/child cycles, order indexes within siblings, link target existence, citation/exhibit status, export profile existence, and schema-version migrations.
- Implementation notes: Keep warnings distinct from blocking errors. Never silently discard unknown legal metadata.
- Acceptance checks: Fixture ASTs for complaint, answer, motion, declaration, memo, notice, and exhibit list pass; malformed fixtures produce stable issue codes.
- Dependencies: `CB-WPB-003`, `CB-WPB-004`, `CB-WPB-011`.
- Status: Todo

## CB-WPB-023 - AST legal diff layers
- Priority: P1
- Area: Versioning/diff
- Problem: V0 compare diffs flattened AST blocks, but legal review needs support, citation, exhibit, rule, formatting, and export layers.
- Expected behavior: Diffs classify text changes, block moves, paragraph renumbering, support link changes, citation changes, exhibit changes, rule finding changes, formatting changes, and export artifact changes.
- Implementation notes: Reuse Case History hash layers and avoid raw JSON diffs in UI.
- Acceptance checks: Compare identifies citation added, evidence removed, exhibit unresolved, warning resolved, count moved, and paragraph renumbered.
- Dependencies: `CB-CH-402`, `CB-CH-403`, `CB-CH-404`.
- Status: Partial
- Progress: Compare now emits bounded hash/summary layer diffs for support links, citation uses, exhibit references, rule findings, formatting profile, and export artifacts. Text diffs remain separate; block move/renumber semantics and UI conflict cards remain.

## CB-WPB-024 - Typed block registry and schema docs
- Priority: P0
- Area: AST/schema
- Problem: The current `WorkProductBlock` is flexible enough for V0 but does not yet make every block kind explicit.
- Expected behavior: Define a typed block registry for caption, heading, section, count, paragraph, numbered paragraph, sentence, quote, list, table, signature, certificate, exhibit reference, page break, markdown, and custom extension blocks.
- Implementation notes: Keep wire compatibility where possible, but generate schema docs from Rust/TypeScript fixtures. A stricter Rust enum can follow once profile coverage proves the field set.
- Acceptance checks: Each block kind has required fields, allowed children, allowed target references, render behavior, validation rules, and frontend editor component mapping.
- Dependencies: `CB-WPB-003`, `CB-WPB-022`.
- Status: Todo

## CB-WPB-025 - Schema versioning and AST migration runner
- Priority: P0
- Area: AST/migration
- Problem: `schema_version` exists, but there is no migration runner for future AST shape changes.
- Expected behavior: Add deterministic migrations from older AST versions to current schema, including legacy projection backfill, idempotent reruns, warnings, and rollback-safe snapshots.
- Implementation notes: Store migration version, migration warnings, original schema version, and migration timestamp in metadata or a migration audit record. Never drop unknown legal metadata without preserving it.
- Acceptance checks: Fixtures for legacy flat WorkProduct, complaint facade, missing IDs, old `block_type`/`ordinal`, old marks/anchors, and future unknown fields migrate to valid AST or return stable blocking errors.
- Dependencies: `CB-WPB-020`, `CB-WPB-022`.
- Status: Todo

## CB-WPB-026 - Canonical serialization and semantic hashing
- Priority: P0
- Area: Integrity/versioning
- Problem: Hashes must be stable across field ordering, timestamps, projection rebuilds, and harmless metadata churn.
- Expected behavior: Canonicalize AST JSON for semantic hashes, support hashes, QC hashes, formatting hashes, and export hashes.
- Implementation notes: Exclude volatile timestamps from semantic hashes; include schema version and meaningful metadata. Document which fields affect each hash layer.
- Acceptance checks: Fixture tests prove equivalent ASTs hash identically, while text, support, citation, exhibit, rule, formatting, and export changes affect only expected hash layers.
- Dependencies: `CB-CH-103`, `CB-WPB-023`.
- Status: Partial
- Progress: Current hash helpers use `document_ast` for core layers. More fixtures and field-level hash policy remain.

## CB-WPB-027 - Patch transaction model and optimistic concurrency
- Priority: P0
- Area: API/mutations
- Problem: AST patches can conflict if multiple editor surfaces or AI proposals edit the same blocks.
- Expected behavior: Add document revision/etag or base snapshot ID to AST patch requests, reject stale patches with conflict details, and make patch operations idempotent where possible.
- Implementation notes: Include `base_snapshot_id`, `base_document_hash`, or `revision` in `AstPatch`. Return conflict targets and suggested rebase context.
- Acceptance checks: Tests cover stale text update, concurrent link add, duplicate patch replay, AI proposal based on old snapshot, and safe retry after network failure.
- Dependencies: `CB-WPB-003`, `CB-CH-108`.
- Status: Todo

## CB-WPB-028 - Complete patch operation invariants
- Priority: P0
- Area: AST/patch
- Problem: Patch operations exist, but production requires stronger invariants for every tree mutation.
- Expected behavior: Insert, update, delete, move, split, merge, and renumber operations preserve unique IDs, parent/child integrity, valid order indexes, valid references, and history anchors.
- Implementation notes: Add tombstone semantics for delete where history/comments/support need old IDs. Split/merge must remap text ranges and support references.
- Acceptance checks: Property tests and fixtures cover nested moves, deleting parents, splitting paragraphs with citations, merging blocks with support links, renumbering paragraphs, and invalid cycles.
- Dependencies: `CB-WPB-022`, `CB-WPB-027`.
- Status: Todo

## CB-WPB-029 - Paragraph numbering and cross-reference engine
- Priority: P0
- Area: Legal structure
- Problem: Pleadings and answers need paragraph numbers and cross-references that survive insert, delete, move, and restore.
- Expected behavior: Renumber numbered paragraphs, detect skipped/duplicate numbers, preserve stable block IDs, update references such as "paragraphs 1 through 20", and keep old-to-new maps for history.
- Implementation notes: Treat paragraph numbers as render state, not identity. Store cross-reference targets separately from text where possible.
- Acceptance checks: Complaint and answer fixtures preserve incorporation clauses, skipped-number warnings, duplicate warnings, old-to-new renumber maps, and restore safety.
- Dependencies: `CB-WPB-028`, `CB-CH-202`, `CB-CH-401`.
- Status: Todo

## CB-WPB-030 - Sentence segmentation and text-range anchoring
- Priority: P0
- Area: Support/fact-checking
- Problem: The AST can represent sentences, but sentence extraction and range anchoring are not reliable enough for every factual assertion.
- Expected behavior: Derive and maintain `SentenceBlock` or `WorkProductSentence` projections with stable IDs, sentence indexes, text ranges, support status, classification, and finding targets.
- Implementation notes: Preserve ranges through edits using offsets plus quotes. Mark ambiguous range repair as needs-review rather than silently moving support.
- Acceptance checks: Sentence fixtures cover abbreviations, citations, quotes, numbered paragraphs, edited text, split/merge, and citation/exhibit spans.
- Dependencies: `CB-V0-026`, `CB-WPB-010`, `CB-WPB-028`.
- Status: Todo

## CB-WPB-031 - Citation lifecycle and ORSGraph resolver integration
- Priority: P0
- Area: Citations/authority
- Problem: `CitationUse` exists, but citations need a full lifecycle from detection to resolution, currentness, pinpoint review, and export.
- Expected behavior: Detect citations in AST text, create/update `CitationUse`, resolve to ORSGraph identities/provisions/rules/cases where possible, flag unresolved/ambiguous/stale/currentness warnings, and support user correction.
- Implementation notes: Reuse existing regex citation detection and ORSGraph search/resolution. Never export broken required citations silently.
- Acceptance checks: ORS, ORCP, UTCR, case-law placeholder, pinpoint, duplicate, stale, ambiguous, and malformed citation fixtures produce stable statuses and AST targets.
- Dependencies: `CB-X-006`, `CB-WPB-010`, `CB-WPB-011`.
- Status: Todo

## CB-WPB-032 - Exhibit lifecycle, renumbering, and packet sync
- Priority: P0
- Area: Exhibits/export
- Problem: `ExhibitReference` exists, but exhibit labels, attachments, packet order, and renumbering need one canonical lifecycle.
- Expected behavior: Detect exhibit references, attach to exhibit/document records, warn on missing/ambiguous exhibits, support renumber-needed status, and sync packet order without silently changing existing references.
- Implementation notes: Preserve stable exhibit IDs separately from human labels. Provide explicit renumber patches and old-to-new maps.
- Acceptance checks: Reordering packet exhibits warns or patches references deterministically; missing attachments block final export; exhibit list generation matches AST references.
- Dependencies: `CB-V02-004`, `CB-V02-012`, `CB-WPB-010`, `CB-WPB-012`.
- Status: Todo

## CB-WPB-033 - Fact/evidence/source support inspector
- Priority: P0
- Area: Support graph/frontend
- Problem: AST links exist, but users need ergonomic support review and repair flows.
- Expected behavior: The inspector shows linked facts, evidence, source spans, documents, pages, quotes, confidence, contradictions, and missing-support actions for the selected AST node or text range.
- Implementation notes: Support chips must deep-link to source preview and allow add/remove/update link operations through AST patches.
- Acceptance checks: Selecting a paragraph, sentence, citation, or exhibit shows the right support; adding/removing support updates AST, graph, QC status, history, and changed-since-export.
- Dependencies: `CB-WPB-007`, `CB-WPB-010`, `CB-V0-020`.
- Status: Todo

## CB-WPB-034 - Authority and rule-link inspector
- Priority: P1
- Area: Authority/frontend
- Problem: Authority attachments currently exist in several target shapes, but WorkProduct editing needs one AST-aware authority inspector.
- Expected behavior: Selected AST nodes show cited authorities, recommended authority, provision links, currentness state, definition-scope warnings, and one-click attach/detach actions.
- Implementation notes: Distinguish legal authority supporting a claim element from citation text present in the document.
- Acceptance checks: Claim/count/paragraph/sentence authority links survive save/reload, appear in Case History support diffs, and cannot cross matters.
- Dependencies: `CB-V02-008`, `CB-WPB-010`, `CB-WPB-031`.
- Status: Todo

## CB-WPB-035 - Rule engine over AST profiles
- Priority: P0
- Area: QC/rules
- Problem: Rule checks must inspect AST/profile state instead of rendered HTML or complaint-only DTOs.
- Expected behavior: Rule packs evaluate WorkProduct type, metadata, blocks, links, citations, exhibits, formatting profile, export state, and Case History state.
- Implementation notes: Use deterministic rule packs first. Profile-specific rules layer over universal rules.
- Acceptance checks: Complaint, answer, motion, declaration, memo, notice, and exhibit-list fixtures produce expected findings with exact AST target IDs.
- Dependencies: `CB-WPB-004`, `CB-WPB-011`, `CB-WPB-022`.
- Status: Todo

## CB-WPB-036 - Matter-level QC aggregation from AST findings
- Priority: P1
- Area: QC/risk
- Problem: WorkProduct findings exist, but matter-level QC needs to aggregate across all work products and graph objects.
- Expected behavior: Matter QC run creates reproducible `QcRun` records summarizing unsupported allegations, unresolved citations, missing exhibits, broken links, weak evidence, contradictions, deadline risks, and export blockers.
- Implementation notes: Findings should point to AST targets when a WorkProduct is involved and to graph node IDs otherwise.
- Acceptance checks: QC dashboard counts update after AST patch, support link, citation resolution, exhibit attach, export, restore, and finding status change.
- Dependencies: `CB-V01-015`, `CB-V01-016`, `CB-WPB-035`.
- Status: Todo

## CB-WPB-037 - Rich text AST adapter
- Priority: P0
- Area: Rich text/frontend
- Problem: The shared editor cannot become canonical until ProseMirror/Tiptap state is a projection of the AST.
- Expected behavior: Implement AST-to-ProseMirror and ProseMirror-to-AST patch conversion for supported block nodes, inline marks, chips, selections, comments, and warning marks.
- Implementation notes: Editor state may cache ProseMirror JSON but cannot become a separate truth. Unsupported AST nodes render as locked/structured widgets.
- Acceptance checks: Round-trip fixtures preserve block IDs, nested structure, text, links, citations, exhibits, rule warning targets, selections, and unsupported custom blocks.
- Dependencies: `CB-WPB-008`, `CB-WPB-027`, `CB-WPB-030`.
- Status: Todo

## CB-WPB-038 - Markdown metadata preservation
- Priority: P1
- Area: Markdown/editor
- Problem: Current markdown conversion loses too much legal metadata for production round-trip editing.
- Expected behavior: Preserve block IDs, type, role, support IDs, citation IDs, exhibit IDs, rule finding IDs, metadata, and unsupported blocks through frontmatter, hidden comments, or sidecar JSON.
- Implementation notes: Keep markdown human-editable. Warn when metadata cannot be represented safely.
- Acceptance checks: AST to markdown to AST preserves stable IDs and references for representative complaint, motion, declaration, memo, and letter fixtures.
- Dependencies: `CB-WPB-009`, `CB-WPB-022`.
- Status: Todo

## CB-WPB-039 - HTML preview renderer parity
- Priority: P1
- Area: Preview/rendering
- Problem: Preview must reflect AST semantics and export warnings without requiring PDF/DOCX generation.
- Expected behavior: Render AST to accessible HTML with profile styles, line numbers where appropriate, citation/exhibit chips in review mode, warnings, and deterministic plain view mode.
- Implementation notes: Preview should share normalized render tree with export where possible.
- Acceptance checks: Snapshot tests and visual checks cover complaint, answer, motion, declaration, memo, notice, and exhibit list previews on desktop/mobile widths.
- Dependencies: `CB-WPB-012`, `CB-WPB-037`.
- Status: Todo

## CB-WPB-040 - DOCX renderer from AST render tree
- Priority: P1
- Area: Export/DOCX
- Problem: Current DOCX artifacts are skeletons.
- Expected behavior: Generate editable DOCX from AST render tree with headings, numbered paragraphs, citations, exhibit references, caption, signature, certificate, tables, and review warnings.
- Implementation notes: Use a structured DOCX library and render/inspect output before marking generation successful.
- Acceptance checks: DOCX fixtures open, render to images/PDF for visual QA, preserve numbering/citations/exhibits, and include artifact hash/snapshot metadata.
- Dependencies: `CB-WPB-012`, `CB-WPB-039`, `CB-V02-006`.
- Status: Todo

## CB-WPB-041 - PDF/court-paper renderer from AST render tree
- Priority: P1
- Area: Export/PDF
- Problem: Current PDF artifacts are skeletons and cannot be treated as court-ready.
- Expected behavior: Generate review-needed PDF from AST render tree with court-paper layout profiles, page numbers, line numbers, margins, caption behavior, page breaks, signatures, certificates, and overflow warnings.
- Implementation notes: Visual verification is required before enabling final filing-copy language.
- Acceptance checks: Rendered PDF fixtures pass page-image checks for overlap, line numbering, margins, caption, signature placement, and warning pages.
- Dependencies: `CB-WPB-012`, `CB-WPB-039`, `CB-V02-007`, `CB-V02-013`.
- Status: Todo

## CB-WPB-042 - Export readiness and blocker policy
- Priority: P0
- Area: Export/safety
- Problem: Exporters need consistent rules for unresolved citations, missing exhibits, unsupported allegations, invalid formatting, and review-needed state.
- Expected behavior: Each export returns readiness status, blockers, warnings, artifact hash, snapshot ID, render profile hash, and clear review-needed labels.
- Implementation notes: Allow internal/review exports with warnings, but block or require explicit override for final filing profiles.
- Acceptance checks: Export fixtures prove unresolved required citations/exhibits are not silently exported; changed-since-export resets only after a new export snapshot.
- Dependencies: `CB-WPB-011`, `CB-WPB-031`, `CB-WPB-032`, `CB-WPB-040`, `CB-WPB-041`.
- Status: Todo

## CB-WPB-043 - AI patch proposal review workflow
- Priority: P0
- Area: AI/editor
- Problem: `AstPatch` exists, but users need accept/reject/edit-before-accept workflows for AI output.
- Expected behavior: AI commands return proposed patches with facts/evidence/authority used, warnings, assumptions, unsupported text, and before/after preview. Users can accept, reject, or edit before accepting.
- Implementation notes: Accepted AI patches create Case History records and AI audit entries. Rejected patches create audit records but no AST mutation.
- Acceptance checks: Tests cover provider-free template mode, stale-base AI patches, unsupported facts, deleted support links, and accepted/rejected audit trails.
- Dependencies: `CB-WPB-013`, `CB-WPB-027`, `CB-CH-801`.
- Status: Todo

## CB-WPB-044 - Scoped restore for AST, support, citations, exhibits, and QC
- Priority: P0
- Area: Restore/history
- Problem: Current restore is whole/block-first and does not cover support/citation/exhibit/QC scopes well enough.
- Expected behavior: Restore text only, block subtree, support links, citations, exhibit references, formatting profile, rule findings, or selected metadata from a snapshot.
- Implementation notes: Dry-run must show support loss, citation regressions, exhibit conflicts, and rule finding changes before applying.
- Acceptance checks: Restore fixtures prove history is preserved, old IDs remain traceable, changed-since-export updates, and scoped restore does not overwrite unrelated newer edits.
- Dependencies: `CB-CH-503`, `CB-WPB-023`, `CB-WPB-027`.
- Status: Partial
- Progress: Restore now scopes AST blocks, document metadata, support links, citations, exhibits, rule findings, formatting, and export state. Targeted block restore preserves unrelated current AST edits, and export-state restore merges snapshot artifacts without deleting newer artifacts. Dry-run conflict cards and richer support-loss previews remain.

## CB-WPB-045 - Branch alternatives and AST merge conflict model
- Priority: P1
- Area: Branching/history
- Problem: Branching legal strategies needs AST-aware compare and merge, not full-document copy-paste.
- Expected behavior: Branches can diverge in text, support, citations, exhibits, rule findings, and formatting, then show merge cards with conflict categories and safe apply choices.
- Implementation notes: Use stable AST IDs and patch rebasing. Conflict cards should classify legal risks, not only text conflicts.
- Acceptance checks: Merge fixtures cover same-block text edits, support added on one branch, citation resolved on another, exhibit renumber conflicts, and QC status conflicts.
- Dependencies: `CB-CH-601`, `CB-CH-602`, `CB-WPB-023`, `CB-WPB-044`.
- Status: Todo

## CB-WPB-046 - Large-document AST performance budget
- Priority: P0
- Area: Performance
- Problem: Large complaints, motion packets, and exhibit lists can make full AST payloads and projection rebuilds slow.
- Expected behavior: Define and meet performance budgets for load, patch, validate, preview, snapshot, diff, and export on large AST fixtures.
- Implementation notes: Add pagination/windowing for block lists, incremental validation, cached render trees, lazy inspectors, graph excerpts for oversized block text, R2-backed immutable payload refs, and bounded response fields where needed.
- Acceptance checks: Large fixture tests cover 1,000+ blocks, 5,000+ links, 500+ citations, nested sections, object-backed snapshots/exports, and multiple snapshots within agreed local thresholds.
- Dependencies: `CB-X-010`, `CB-X-018`, `CB-WPB-047`, `CB-WPB-061`, `CB-WPB-063`.
- Status: Partial
- Progress: Backend now has configurable inline thresholds, graph block text excerpts/hashes for oversized materialization, and summary list responses unless `include=document_ast` is requested. Large fixture budgets and UI windowing remain.

## CB-WPB-047 - Snapshot size, compression, and projection cache policy
- Priority: P1
- Area: Performance/storage
- Problem: Full AST snapshots, render trees, and projection caches can grow quickly.
- Expected behavior: Define when to inline snapshots, when to store `ObjectBlob` refs, how to compress large states, and how to invalidate cached projections.
- Implementation notes: Use content hashes and snapshot manifests. Keep restore fast without duplicating every projection as truth; refs point to `ObjectBlob` IDs and are hydrated server-side.
- Acceptance checks: Large fixture history remains paginated; snapshot detail payloads are bounded; R2/local-backed restore works; projection cache rebuilds are deterministic.
- Dependencies: `CB-CH-107`, `CB-CH-1102`, `CB-WPB-026`, `CB-WPB-061`, `CB-WPB-062`, `CB-WPB-064`.
- Status: Partial
- Progress: Snapshot full-state/entity-state offload now uses configurable thresholds and `ObjectBlob` refs, with restore/compare hydration from the object store. Compression and projection-cache policy remain.

## CB-WPB-048 - Frontend AST state manager and autosave queue
- Priority: P0
- Area: Frontend/editor
- Problem: Shared editor needs safe local state, autosave, pending patch queue, conflict handling, undo/redo, and offline/error behavior.
- Expected behavior: Frontend maintains editor projection state, emits AST patches, queues autosaves, handles validation errors/conflicts, shows saved/pending/error states, and supports local undo/redo over patches.
- Implementation notes: Keep a clear boundary between projection state and canonical AST from the server.
- Acceptance checks: UI tests cover rapid edits, failed save, stale patch conflict, retry, undo/redo, reload while pending, and validation error display.
- Dependencies: `CB-WPB-005`, `CB-WPB-007`, `CB-WPB-027`, `CB-WPB-037`.
- Status: Todo

## CB-WPB-049 - AST Inspector and developer diagnostics
- Priority: P2
- Area: Developer tooling/frontend
- Problem: A structured document tree is hard to debug without a visible inspector.
- Expected behavior: Add an internal/debug AST inspector showing block tree, selected node JSON, links, citations, exhibits, rule findings, validation issues, render tree, and patch history.
- Implementation notes: Keep disabled or hidden from normal users unless explicitly enabled.
- Acceptance checks: Developers can diagnose broken refs, wrong order indexes, missing metadata, and projection mismatch without querying Neo4j manually.
- Dependencies: `CB-WPB-007`, `CB-WPB-022`.
- Status: Todo

## CB-WPB-050 - Accessibility and responsive behavior for AST editor
- Priority: P1
- Area: Frontend/accessibility
- Problem: Dense legal AST editing can become unusable for keyboard, screen-reader, tablet, and mobile users.
- Expected behavior: Outline, editor, inspector, chips, warnings, modals, compare, and export controls are keyboard reachable, labeled, responsive, and free of layout overlap.
- Implementation notes: Use stable dimensions for paragraph numbers, chips, toolbar controls, tabs, and warning markers.
- Acceptance checks: Keyboard smoke, responsive screenshots, and axe-style checks cover editor, inspector, preview, compare, and export.
- Dependencies: `CB-X-007`, `CB-X-008`, `CB-WPB-007`.
- Status: Todo

## CB-WPB-051 - Matter isolation and AST authorization tests
- Priority: P0
- Area: Security/privacy
- Problem: AST links can reference sensitive facts, evidence, documents, sources, exhibits, snapshots, and exports.
- Expected behavior: Every AST route, patch operation, conversion, preview, export, history, support link, citation resolution, and restore checks matter ownership for all referenced IDs.
- Implementation notes: Reject cross-matter target IDs even when the source WorkProduct belongs to the caller's matter.
- Acceptance checks: Cross-matter fixtures attempt link, citation, exhibit, patch, export, restore, compare, and source preview misuse and receive not-found/forbidden responses.
- Dependencies: `CB-X-002`, `CB-X-016`, `CB-WPB-010`, `CB-WPB-018`.
- Status: Partial
- Progress: Support-link writes, complaint support links, AST patch references, restored products, artifact download lookup, snapshot hydration, compare, restore, and `ObjectBlob` reads now flow through matter-scoped service lookups. Contract/unit coverage guards those paths; full live cross-matter route fixtures remain.

## CB-WPB-052 - Sensitive logging and telemetry policy
- Priority: P0
- Area: Privacy/observability
- Problem: AST patches and diffs can contain confidential legal text.
- Expected behavior: Logs and metrics include IDs, counts, sizes, hash prefixes, operation names, validation codes, and durations, not raw text, prompts, citations with private context, or source quotes.
- Implementation notes: Add redaction helpers for patch/diff/export logs.
- Acceptance checks: Review tests or snapshots catch accidental text/prompt logging in AST patch, validation, AI, export, compare, and restore paths.
- Dependencies: `CB-X-009`, `CB-X-019`, `CB-CH-1103`.
- Status: Partial
- Progress: AST patch conflict/validation errors no longer echo patch IDs, prompts, or legal text; provider-free AI history summaries avoid raw prompts; R2 storage errors avoid raw backend details; WorkProduct export download filenames are hash-derived. A full logging/telemetry redaction audit remains.

## CB-WPB-053 - AST fixture and golden corpus
- Priority: P0
- Area: Quality/fixtures
- Problem: AST behavior needs stable fixtures across document types and edge cases.
- Expected behavior: Add fixture corpus for complaint, answer, motion, declaration, affidavit, memo, demand letter, notice, exhibit list, filing packet, malformed ASTs, legacy flat WorkProducts, and large documents.
- Implementation notes: Store expected validation issues, markdown/html/plain projections, hash layers, and diff summaries.
- Acceptance checks: CI fixtures fail on unintended schema, validation, conversion, hash, or diff changes.
- Dependencies: `CB-WPB-022`, `CB-WPB-026`, `CB-WPB-038`, `CB-WPB-039`.
- Status: Todo

## CB-WPB-054 - Property and fuzz tests for AST operations
- Priority: P1
- Area: Quality/testing
- Problem: Tree patch operations can create rare corruption bugs that fixed examples miss.
- Expected behavior: Add property/fuzz tests for patch sequences, tree integrity, no cycles, unique IDs, reference preservation, split/merge ranges, and round-trip conversions.
- Implementation notes: Generate random but valid ASTs plus invalid mutation attempts. Keep seeds deterministic in CI.
- Acceptance checks: Random patch sequences either produce a valid AST or a stable validation error, never panic or corrupt references silently.
- Dependencies: `CB-WPB-028`, `CB-WPB-030`, `CB-WPB-038`.
- Status: Todo

## CB-WPB-055 - Projection parity test matrix
- Priority: P0
- Area: Quality/rendering
- Problem: Every projection can drift from the AST in different ways.
- Expected behavior: Test AST to rich text, markdown, HTML, plain text, preview, DOCX, PDF, graph, Case History snapshot, and export artifact parity.
- Implementation notes: Start with implemented projections and mark future renderers expected-fail or pending only where the code is not real yet.
- Acceptance checks: A fixture change updates all expected projections intentionally; no projection drops required legal metadata without warning.
- Dependencies: `CB-WPB-037`, `CB-WPB-038`, `CB-WPB-039`, `CB-WPB-040`, `CB-WPB-041`.
- Status: Todo

## CB-WPB-056 - End-to-end AST smoke matrix
- Priority: P0
- Area: Quality/smoke
- Problem: The AST platform is only trustworthy if the whole chain works together.
- Expected behavior: Smoke creates a matter, creates multiple WorkProduct types, patches AST, links fact/evidence/authority, resolves citation, attaches exhibit, runs QC, previews, exports, snapshots, compares, restores, and verifies changed-since-export.
- Implementation notes: Keep provider-free and deterministic. Add opt-in live Neo4j/API mode and a fast mocked/local mode if needed.
- Acceptance checks: Smoke fails on missing history events, broken references, lost links, invalid export warnings, bad restore, projection drift, or cross-matter leaks.
- Dependencies: `CB-WPB-019`, `CB-WPB-031`, `CB-WPB-032`, `CB-WPB-036`, `CB-WPB-044`.
- Status: Todo

## CB-WPB-057 - Legacy projection removal plan
- Priority: P0
- Area: Migration/cleanup
- Problem: Compatibility `blocks`, `marks`, `anchors`, flat findings, complaint DTOs, and generic Draft routes can keep drifting unless explicitly retired or demoted.
- Expected behavior: Define which old fields remain as read-only projections, which routes become adapters, and which stored fields are removed before launch.
- Implementation notes: Provide migration steps, compatibility warnings, and final deletion criteria.
- Acceptance checks: No production write path treats old projections as authoritative; docs and tests fail when new document work bypasses `document_ast`.
- Dependencies: `CB-WPB-020`, `CB-CH-105`.
- Status: Todo

## CB-WPB-058 - Backend module extraction and service boundaries
- Priority: P1
- Area: Backend/refactor
- Problem: AST logic living inside `CaseBuilderService` will slow development and make test scope too broad.
- Expected behavior: Extract modules/services for AST model helpers, validation, patching, conversion, render tree, diff/hash, rule engine, citation resolver, support linker, export orchestration, and AI patch proposal handling.
- Implementation notes: Behavior must stay stable through extraction; keep public route contracts unchanged.
- Acceptance checks: Existing tests pass before/after extraction, module-level tests cover each boundary, and no route payload changes unintentionally.
- Dependencies: `CB-WPB-021`, `CB-WPB-053`.
- Status: Todo

## CB-WPB-059 - Public API and developer documentation
- Priority: P1
- Area: Documentation/API
- Problem: The AST contract is central enough that it needs durable docs, examples, and compatibility notes.
- Expected behavior: Publish internal docs for AST schema, block kinds, patch operations, validation issue codes, markdown metadata, export readiness, Case History integration, and migration policy.
- Implementation notes: Generate examples from fixtures where possible.
- Acceptance checks: Docs include copy-paste examples for create, patch, validate, convert, link support, add citation, add exhibit, run QC, export, snapshot, compare, and restore.
- Dependencies: `CB-WPB-024`, `CB-WPB-053`.
- Status: Todo

## CB-WPB-060 - AST production release gate
- Priority: P0
- Area: Release/quality
- Problem: The AST should not be considered production-complete because the foundation exists.
- Expected behavior: Define the release checklist for schema, validation, patching, editor, projections, links, citations, exhibits, QC, AI, versioning, export, security, performance, and docs.
- Implementation notes: This is the gate that turns "AST backend foundation wired" into "AST platform ready for all WorkProduct types."
- Acceptance checks: All P0 `CB-WPB-*` AST completion tickets are Done, all required tests run in CI, no legacy write path bypasses AST, production PDF/DOCX readiness is accurately labeled, no large immutable AST/export artifact is stored only as Neo4j payload, and safety/privacy review passes.
- Dependencies: `CB-WPB-024` through `CB-WPB-065`.
- Status: Todo

## CB-WPB-061 - Graph/R2 AST storage policy
- Priority: P0
- Area: Storage/performance
- Problem: The AST needs clear inline-vs-object storage rules so Neo4j remains queryable instead of becoming a blob store.
- Expected behavior: Configure inline thresholds for snapshot entity state, full snapshot state, and block text materialization; keep small JSON inline and store larger immutable payloads through `ObjectStore`.
- Implementation notes: Default thresholds are 64 KiB entity state, 256 KiB full snapshot state, and 64 KiB block text graph materialization. Refs store `ObjectBlob` IDs, not raw storage keys.
- Acceptance checks: Unit tests cover threshold boundaries, hash stability, opaque object keys, and no raw filenames/case text in generated keys.
- Dependencies: `CB-V0F-013`, `CB-V0F-014`, `CB-WPB-026`, `CB-WPB-047`.
- Status: Partial
- Progress: Configurable thresholds, snapshot offload, object-backed export storage, graph block excerpts/hashes, bounded VersionChange state summaries, hash-scoped document/work-product object keys, and key opacity tests are implemented. Compression and retention policy remain.

## CB-WPB-062 - Snapshot/state object writer and hydrator
- Priority: P0
- Area: History/storage
- Problem: Large Case History snapshots cannot stay only as inline graph payloads.
- Expected behavior: Write oversized `VersionSnapshot.full_state`, `SnapshotEntityState.state`, and snapshot manifests to R2/local object storage, link them through `ObjectBlob`, and hydrate them for compare/restore/detail reads.
- Implementation notes: Compare and restore must load R2-backed state server-side after matter ownership is verified.
- Acceptance checks: Restore works from inline and object-backed snapshots; matter A cannot hydrate matter B snapshot refs; graph can compare entity hashes before loading full state.
- Dependencies: `CB-CH-107`, `CB-CH-1102`, `CB-WPB-061`.
- Status: Partial
- Progress: Snapshot full-state/entity-state refs and compare/restore/detail hydration are implemented through the existing matter-scoped `ObjectStore`; public snapshot lists omit inline full state. Contract coverage guards matter-owned blob hydration. Full live cross-matter fixtures and compression remain.

## CB-WPB-063 - Current AST graph hydration and summary-list optimization
- Priority: P0
- Area: API/performance
- Problem: WorkProduct list/search/QC pages should not ship full AST bodies by default.
- Expected behavior: `GET /work-products/:id` returns hydrated `document_ast`; `GET /work-products` returns bounded summaries unless `include=document_ast` is requested.
- Implementation notes: Legal intelligence queries should use graph nodes/edges and hashes, not R2 reads.
- Acceptance checks: List contract proves AST arrays are omitted by default; detail contract proves AST is hydrated; QC/support/citation graph queries do not scan object blobs.
- Dependencies: `CB-X-010`, `CB-X-014`, `CB-WPB-003`, `CB-WPB-061`.
- Status: Partial
- Progress: Route-level and frontend API `include=document_ast` support, default summary payloads, bounded snapshot-list payloads, graph excerpts/hashes for oversized block text, export-history hash comparison without snapshot hydration, and large-list summary coverage are implemented. Dedicated list DTOs and full large-fixture response budgets remain.

## CB-WPB-064 - Projection and export cache storage in R2
- Priority: P1
- Area: Export/rendering
- Problem: Rendered HTML/Markdown/PDF/DOCX/plain-text exports and future render caches are immutable artifacts, not graph truth.
- Expected behavior: Store generated export content under hash-scoped object keys, link artifacts to `ObjectBlob`, expose safe API/presigned download URLs, and invalidate caches by AST/render-profile hashes.
- Implementation notes: Keep previews short in graph payloads. Raw R2 keys stay server-side.
- Acceptance checks: Export artifacts have `object_blob_id`, `size_bytes`, MIME, hash, snapshot ID, and changed-since-export state; downloads never expose raw storage keys to normal clients.
- Dependencies: `CB-WPB-012`, `CB-WPB-039`, `CB-WPB-040`, `CB-WPB-041`, `CB-WPB-061`.
- Status: Partial
- Progress: WorkProduct exports now write content through `ObjectStore`, link `ObjectBlob`, keep bounded previews, return safe local/API download metadata, and can return presigned R2 downloads when configured. Download responses expose blob IDs/artifact metadata rather than raw storage keys. Projection render-cache invalidation remains.

## CB-WPB-065 - AST storage lifecycle and cleanup
- Priority: P0
- Area: Privacy/retention
- Problem: Snapshot/export/projection objects need the same retention, legal hold, tombstone, and cleanup guarantees as evidence uploads.
- Expected behavior: Matter deletion/archive/retention policies cover AST snapshot blobs, projection caches, export artifacts, and oversized block-content objects.
- Implementation notes: Use object refs and graph ownership edges for reconciliation; logs report IDs/counts/hashes, not text.
- Acceptance checks: Deleted/tombstoned matter artifacts cannot be downloaded or hydrated; orphan detection finds unreferenced AST/export objects.
- Dependencies: `CB-X-017`, `CB-X-021`, `CB-WPB-061`, `CB-WPB-064`.
- Status: Todo
