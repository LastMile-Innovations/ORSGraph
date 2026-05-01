# 01 - V0 Foundation Backlog

V0 foundation makes CaseBuilder trustworthy: canonical routing, durable data, private local files, clear demo/offline states, and backend contracts.

## CB-V0F-001 - Canonical `/casebuilder` route tree
- Priority: P0
- Area: Routing
- Problem: The original UI lived under `/matters`, while the CaseBuilder product spec uses `/casebuilder`.
- Expected behavior: `/casebuilder` is canonical; `/matters` remains compatibility-only.
- Implementation notes: Keep route helpers as the only source for generated CaseBuilder links.
- Acceptance checks: `/casebuilder` and seeded matter routes return 200; `/matters` routes redirect.
- Dependencies: None.
- Status: Done

## CB-V0F-002 - Frontend route helper migration
- Priority: P0
- Area: Navigation
- Problem: Hard-coded matter links can regress to legacy `/matters` or unsafe IDs.
- Expected behavior: All CaseBuilder components use route helpers for matter, document, fact, claim, and draft links.
- Implementation notes: Keep `frontend/lib/casebuilder/routes.ts` small and authoritative.
- Acceptance checks: `rg '/matters/' frontend/components/casebuilder frontend/app` returns no generated UI links outside the API adapter and route helpers.
- Dependencies: `CB-V0F-001`.
- Status: Done

## CB-V0F-003 - CaseBuilder backend model module
- Priority: P0
- Area: Backend types
- Problem: Matter graph objects need explicit API DTOs before workflows can become live.
- Expected behavior: Backend models cover Matter, CaseDocument, Fact, Party, TimelineEvent, Evidence, Claim, Defense, Element, Draft, DeadlineInstance, Task, FactCheckFinding, and CitationCheckFinding.
- Implementation notes: Use direct serde DTOs and avoid exposing Neo4j internals.
- Acceptance checks: Models compile and route handlers return typed responses.
- Dependencies: None.
- Status: Done

## CB-V0F-004 - CaseBuilder API route registration
- Priority: P0
- Area: Backend routing
- Problem: Frontend workflows need stable endpoint contracts.
- Expected behavior: `/api/v1/matters` endpoints exist for all V0 contracts, with deferred export endpoints returning explicit errors.
- Implementation notes: Keep route registration covered by contract tests.
- Acceptance checks: `casebuilder_routes_cover_v0_contracts` passes.
- Dependencies: `CB-V0F-003`.
- Status: Done

## CB-V0F-005 - Neo4j constraints and matter graph edges
- Priority: P0
- Area: Graph persistence
- Problem: User case data needs isolated graph nodes and stable IDs.
- Expected behavior: CaseBuilder nodes have uniqueness constraints and matter ownership edges.
- Implementation notes: Use `(:Matter)-[:HAS_*]->(...)` relationships and bridge to ORSGraph authority nodes where possible.
- Acceptance checks: Constraint contract tests pass.
- Dependencies: `CB-V0F-003`.
- Status: Done

## CB-V0F-006 - Local private storage default
- Priority: P0
- Area: File storage
- Problem: Uploaded case materials are sensitive and need a local/private V0 storage path.
- Expected behavior: Upload storage defaults to `data/casebuilder/uploads` and can be overridden with `ORS_CASEBUILDER_STORAGE_DIR`.
- Implementation notes: Keep filenames sanitized and store file hashes in document metadata.
- Acceptance checks: Unit tests cover filename sanitization and hashes.
- Dependencies: None.
- Status: Done

## CB-V0F-007 - Add `data/casebuilder/uploads` git-ignore policy
- Priority: P0
- Area: Privacy
- Problem: Local uploads must never be committed accidentally.
- Expected behavior: All CaseBuilder upload storage paths are ignored.
- Implementation notes: Confirm existing `/data/` ignore is enough or add an explicit comment.
- Acceptance checks: `git status` never shows uploaded matter files.
- Dependencies: `CB-V0F-006`.
- Status: Done
- Completed: Confirmed `/data/` ignores the default upload tree and added an explicit CaseBuilder privacy comment to `.gitignore`.

## CB-V0F-008 - Frontend data adapter
- Priority: P0
- Area: Frontend data
- Problem: Matter pages previously imported seeded mock data directly.
- Expected behavior: Matter pages load through `getMatterState` and matter list through `getMatterSummariesState`.
- Implementation notes: Mock data can remain only behind adapter fallback.
- Acceptance checks: App pages no longer import `mock-matters` directly.
- Dependencies: `CB-V0F-004`.
- Status: Done

## CB-V0F-009 - Live/demo/offline/error labels
- Priority: P0
- Area: Trust and safety
- Problem: Users must know whether they are seeing live matter data or seeded demo data.
- Expected behavior: CaseBuilder surfaces show data state when not live.
- Implementation notes: Use shared banner and eventually unify with ORSGraph data-state components.
- Acceptance checks: Backend-offline matter dashboard shows demo or error state visibly.
- Dependencies: `CB-V0F-008`.
- Status: Done

## CB-V0F-010 - New matter API integration
- Priority: P0
- Area: Matter intake
- Problem: New Matter still opens a demo matter instead of creating a real one.
- Expected behavior: Form submits to `POST /api/v1/matters`, handles pending/error/success, and routes to `/casebuilder/matters/:id`.
- Implementation notes: Preserve a clearly labeled "open demo matter" option.
- Acceptance checks: Creating a matter produces a persisted Neo4j `Matter` and opens the new dashboard.
- Dependencies: `CB-V0F-004`, `CB-V0F-008`.
- Status: Partial
- Progress: New Matter now calls `POST /api/v1/matters`, preserves an explicit demo entry point, uploads selected files through the binary upload endpoint when available, and routes to canonical `/casebuilder/matters/:id`.
- Still needed: Smoke against a live Neo4j-backed API and add regression coverage for error/pending/success states.

## CB-V0F-011 - Frontend mutation API client
- Priority: P0
- Area: API integration
- Problem: Read adapter exists, but creates, uploads, approvals, links, and draft changes need typed callers.
- Expected behavior: `frontend/lib/casebuilder/api.ts` exposes typed functions for all V0 mutations with consistent error handling.
- Implementation notes: Keep mutation functions returning `LoadState` or explicit action result metadata.
- Acceptance checks: Matter creation, upload, fact approve, evidence link, draft save, generate, fact-check, and citation-check all call the API.
- Dependencies: `CB-V0F-008`, `CB-V0F-010`.
- Status: Done
- Completed: `frontend/lib/casebuilder/api.ts` now exposes typed action wrappers for matter creation/update, text and binary upload, extraction, parties, facts, fact approval, timeline events, claims, element mapping, defenses, evidence links, drafts, generation, fact-check, citation-check, authority search/recommend, and authority attach/detach.

## CB-V0F-012 - Route smoke test script
- Priority: P1
- Area: Quality
- Problem: Canonical and legacy route behavior should not rely on manual curl checks.
- Expected behavior: A smoke script checks `/casebuilder`, seeded matter pages, and `/matters` redirects.
- Implementation notes: Use existing frontend scripts style; keep it fast and dependency-light.
- Acceptance checks: Script exits nonzero on 404 or redirect regression.
- Dependencies: `CB-V0F-001`.
- Status: Done
- Progress: `frontend/scripts/smoke-routes.mjs` now checks canonical `/casebuilder` routes and legacy `/matters` redirects.
- Verification: `pnpm run smoke:routes` passed 27 checks against `http://localhost:3000`.

## CB-V0F-013 - R2 evidence lake and Neo4j intelligence graph split
- Priority: P0
- Area: Storage/graph architecture
- Problem: CaseBuilder needs a clean boundary between immutable heavy artifacts and queryable legal/case meaning.
- Expected behavior: R2/local `ObjectStore` stores original uploads, extracted text JSON, OCR output, page thumbnails, redacted copies, exhibit bundles, export artifacts, AST snapshot/projection artifacts, and ingestion manifests; Neo4j stores matter/document/version/blob/page/chunk/evidence/fact/entity/claim/work-product/authority/run meaning and relationships.
- Implementation notes: Treat R2 and Neo4j as two halves of one provenance system. Neo4j nodes should hold stable IDs, hashes, metadata, search excerpts, ownership edges, and relationships, not large binary or immutable artifact payloads.
- Acceptance checks: Architecture docs, DTOs, and service boundaries make it clear which data lives in R2 versus Neo4j, and no new large extracted or AST/export artifact is stored only as a graph payload.
- Dependencies: `CB-V0F-005`, `CB-V0F-006`, `CB-X-013`.
- Status: Todo

## CB-V0F-014 - ObjectBlob content-addressed dedupe model
- Priority: P0
- Area: Storage/chain of custody
- Problem: Multiple documents can reference the same bytes, and slug-derived storage records do not provide strong duplicate detection or chain of custody.
- Expected behavior: Add `ObjectBlob` nodes keyed by `sha256`, with size, MIME type, storage provider, bucket/key, etag, storage class, created timestamp, and retention state.
- Implementation notes: `CaseDocument` and `DocumentVersion` should reference `ObjectBlob` instead of duplicating object metadata; identical uploads can share a blob while preserving separate matter/document context.
- Acceptance checks: Uploading duplicate bytes detects the existing blob, avoids unnecessary storage duplication when policy allows, and preserves matter-scoped document records and audit/provenance.
- Dependencies: `CB-V0F-013`, `CB-X-002`, `CB-X-017`.
- Status: Partial
- Progress: Backend and frontend DTOs now include `ObjectBlob`; CaseBuilder creates content-addressed blob identities keyed by normalized SHA-256 for text and binary uploads, stores provider/bucket/key/hash metadata, links blobs from original document versions, and covers duplicate hash identity/no raw filename leakage in tests.
- Still needed: User-facing duplicate group workflow, storage reuse policy for browser/R2 uploads, retention/legal-hold lifecycle, and full matter-isolation coverage for blob reuse across matters.

## CB-V0F-015 - DocumentVersion provenance model
- Priority: P0
- Area: Document graph
- Problem: A document can have original, redacted, OCR, normalized, and exhibit-bundled representations that need precise provenance.
- Expected behavior: Add `DocumentVersion` nodes linked as `(:CaseDocument)-[:HAS_VERSION]->(:DocumentVersion)-[:STORED_AS]->(:ObjectBlob)`.
- Implementation notes: Versions should capture role (`original`, `redacted`, `ocr`, `normalized_text`, `exhibit_bundle`), source version, artifact kind, created_by process, and current/active flags.
- Acceptance checks: Document viewer, extraction, redaction, OCR, and export workflows can identify the exact version and object blob they used.
- Dependencies: `CB-V0F-014`, `CB-V0-020`.
- Status: Partial
- Progress: Backend and frontend DTOs now include `DocumentVersion`; text and binary uploads create an `original` version linked to the `CaseDocument` and `ObjectBlob`, and extraction responses expose the current document version.
- Still needed: Normalized-text, OCR, redacted, exhibit, and export versions; version history UI; manifest-backed reruns; and migration/compat handling for legacy document records without versions.

## CB-V0F-016 - Opaque object key and ID policy
- Priority: P0
- Area: Privacy/storage
- Problem: Slug-derived document IDs or object keys can leak filenames or case context into R2 paths.
- Expected behavior: New document/object IDs and R2 keys use opaque ULIDs/UUIDs or content-addressed keys without raw filenames, party names, or case facts.
- Implementation notes: Preserve the original filename only in encrypted/private metadata and UI fields; never rely on filename-derived slugs for object keys.
- Acceptance checks: New uploads produce opaque storage keys, tests reject keys containing raw filenames, and existing slug IDs remain readable only as legacy records.
- Dependencies: `CB-V0F-006`, `CB-V0F-014`.
- Status: Partial
- Progress: New CaseBuilder document uploads now use opaque generated document IDs, and AST snapshot/export object keys are hash scoped, so generated object keys no longer include raw filenames; tests cover raw filename leakage in new object keys. Existing slug-shaped IDs remain readable as legacy records.
- Still needed: Apply the same opaque-key policy to future thumbnail, redaction, archive-child, OCR, and all extraction-manifest object keys.

## CB-V0F-017 - R2 artifact manifest layout
- Priority: P0
- Area: Ingestion artifacts
- Problem: Derived extraction artifacts need reproducible storage outside Neo4j.
- Expected behavior: Each document/version has R2 artifacts for `original`, `text.normalized.json`, `pages.json`, `ocr.json` when present, `manifest.json`, and later thumbnails/redactions/export outputs; WorkProduct AST snapshots and exports use the same object/blob reference pattern.
- Implementation notes: The manifest should record object keys, sha256 hashes, extractor versions, model/provider metadata when used, produced graph node IDs, and error state.
- Acceptance checks: Re-running extraction can use the manifest to verify inputs, outputs, and produced graph nodes without reading large payloads from Neo4j.
- Dependencies: `CB-V0F-013`, `CB-V0F-014`, `CB-V0F-015`, `CB-V0-019`.
- Status: Todo
