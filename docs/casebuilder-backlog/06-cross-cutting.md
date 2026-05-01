# 06 - Cross-Cutting Backlog

These tasks cut across phases and should be worked whenever they unblock trust, reliability, or safety.

## CB-X-001 - API DTO and frontend view-model contract tests
- Priority: P0
- Area: API/data
- Problem: Frontend and backend CaseBuilder shapes can drift.
- Expected behavior: Contract tests or fixture validation catch DTO/view-model drift for existing and production backlog DTOs.
- Implementation notes: Add sample API payloads and mapper tests for matter, document, fact, claim, draft, and findings.
- Acceptance checks: CI fails if required fields disappear or type normalization breaks.
- Dependencies: Current data adapter.
- Status: Todo

## CB-X-002 - Matter isolation tests
- Priority: P0
- Area: Privacy/data integrity
- Problem: Case materials must never leak across matters.
- Expected behavior: Queries always scope by `matter_id` and tests prove cross-matter IDs are not returned.
- Implementation notes: Use a service-level test with two matter fixtures.
- Acceptance checks: Documents/facts/evidence/drafts from matter A cannot be fetched through matter B endpoints.
- Dependencies: Test Neo4j or query-layer mock.
- Status: Todo

## CB-X-003 - File storage lifecycle
- Priority: P0
- Area: Privacy/storage
- Problem: Uploads need creation, retention, deletion, and error policy.
- Expected behavior: Create, read metadata, delete, and cleanup orphaned files safely.
- Implementation notes: Keep paths matter-scoped and sanitized.
- Acceptance checks: Deleting a matter removes or tombstones local files according to policy.
- Dependencies: Matter deletion policy.
- Status: Todo

## CB-X-004 - Provider-gated AI configuration
- Priority: P0
- Area: AI safety
- Problem: AI features must not pretend to be live if no provider is configured.
- Expected behavior: Extraction, issue spotting, drafting, fact-checking, and citation-checking return explicit disabled/template/demo states.
- Implementation notes: Add config fields for provider enablement and model names, but keep deterministic fallback.
- Acceptance checks: With no provider config, UI says disabled/template mode and no hallucinated support appears.
- Dependencies: None.
- Status: Partial

## CB-X-005 - Source-backed AI output schema
- Priority: P1
- Area: AI safety
- Problem: AI outputs need evidence and authority grounding.
- Expected behavior: Each generated fact, issue, element mapping, paragraph, and finding includes source refs or explicit "unsupported" status.
- Implementation notes: Reject or flag unsupported assertions rather than storing them as supported.
- Acceptance checks: No approved fact or supported allegation lacks a source link.
- Dependencies: V0 extraction/drafting.
- Status: Todo

## CB-X-006 - Citation checker currentness
- Priority: P1
- Area: Citation safety
- Problem: Citation checks need currentness and scope warnings, not just missing citation detection.
- Expected behavior: Bad citation, stale/repealed law, wrong definition scope, unresolved citation, and wrong pinpoint findings.
- Implementation notes: Use ORSGraph status/source-note data.
- Acceptance checks: Draft citations show resolved/current/warning states.
- Dependencies: Authority retrieval and ORSGraph currentness data.
- Status: Todo

## CB-X-007 - UX accessibility pass
- Priority: P2
- Area: Frontend UX
- Problem: Dense legal workbench pages need keyboard and screen-reader basics.
- Expected behavior: Forms, tabs, tables, sidebars, dialogs, and graph controls are keyboard-navigable and labeled.
- Implementation notes: Start with upload, fact table, claim builder, draft editor, and QC.
- Acceptance checks: Keyboard smoke pass and axe-style checks for core pages.
- Dependencies: Stable V0 UI.
- Status: Todo

## CB-X-008 - Responsive layout pass
- Priority: P2
- Area: Frontend UX
- Problem: Matter workspace uses dense sidebars and panels that can break on smaller screens.
- Expected behavior: Core pages work at mobile, tablet, laptop, and desktop widths.
- Implementation notes: Avoid nested cards and preserve scan-friendly legal workbench density.
- Acceptance checks: Screenshot pass for `/casebuilder`, matter dashboard, documents, facts, claims, evidence, drafts, QC.
- Dependencies: Stable V0 UI.
- Status: Todo

## CB-X-009 - Observability and action logging
- Priority: P2
- Area: Operations
- Problem: It will be hard to debug ingestion and AI workflows without structured logs.
- Expected behavior: API logs matter actions, ingestion stages, extraction failures, authority retrieval, and disabled AI states without leaking sensitive text.
- Implementation notes: Log IDs and counts, not full document contents.
- Acceptance checks: Failed upload/extraction/fact-check can be diagnosed from logs.
- Dependencies: V0 API workflows.
- Status: Todo

## CB-X-010 - Performance guardrails
- Priority: P2
- Area: Performance
- Problem: Large matters can make graph and draft views slow.
- Expected behavior: Pagination, limits, and lazy loading for documents, facts, evidence, graph nodes, and drafts.
- Implementation notes: Add API `limit/offset` before large real matters; enforce graph node/edge limits and bounded findings responses.
- Acceptance checks: Seeded large fixture remains usable and API responses are bounded.
- Dependencies: Live API adoption.
- Status: Todo

## CB-X-011 - Legal safety review checklist
- Priority: P0
- Area: Safety
- Problem: CaseBuilder must remain transparent and not overstate legal capability.
- Expected behavior: Checklist covers legal information disclaimer, source support, review-needed states, no filing-ready implication, and unsupported allegation warnings.
- Implementation notes: Apply to every builder, checker, export, and AI action.
- Acceptance checks: Safety checklist passes before V0 release.
- Dependencies: V0 workflow completion.
- Status: Todo

## CB-X-012 - Seed/demo data policy
- Priority: P1
- Area: Demo data
- Problem: Demo data is useful but can obscure live/offline state.
- Expected behavior: Demo data is explicit, optional, and easy to disable in live-only mode.
- Implementation notes: Add environment switch for live-only, live-with-demo-fallback, and demo-only.
- Acceptance checks: Live-only mode never falls back silently to seeded matter data.
- Dependencies: Data adapter.
- Status: Todo

## CB-X-013 - Production DTO registry
- Priority: P0
- Area: API/data
- Problem: The production backlog introduces shared objects that need stable names before route work starts.
- Expected behavior: Backend and frontend DTO registries include `ObjectBlob`, `DocumentVersion`, `IngestionRun`, `SourceSpan`, `IssueSuggestion`, `DraftSentence`, `CaseGraphNode`, `CaseGraphEdge`, `QcRun`, `EvidenceGap`, `AuthorityGap`, `Contradiction`, `ExportPackage`, and `AuditEvent`.
- Implementation notes: Add DTOs incrementally with contract fixtures; avoid exposing Neo4j internals or file-system paths.
- Acceptance checks: Contract tests prove each DTO serializes, normalizes in the frontend adapter, and keeps matter ownership fields where relevant.
- Dependencies: `CB-X-001`.
- Status: Partial
- Progress: Backend and frontend DTO registries now include `ObjectBlob`, `DocumentVersion`, `IngestionRun`, and `SourceSpan`; contract tests cover backend/frontend registry presence, serialization, frontend normalization references, matter ownership fields, and filename-safe IDs/object keys for the first provenance slice.
- Still needed: Add the remaining production DTOs: `IssueSuggestion`, `DraftSentence`, `CaseGraphNode`, `CaseGraphEdge`, `QcRun`, `EvidenceGap`, `AuthorityGap`, `Contradiction`, `ExportPackage`, and `AuditEvent`.

## CB-X-014 - Production route contract coverage
- Priority: P0
- Area: API/routing
- Problem: New production wiring routes need explicit contract coverage like the existing V0 route test.
- Expected behavior: Contract tests cover `/issues/spot`, `/graph`, `/qc/run`, finding lifecycle routes, task/deadline CRUD, authority attach routes, and export package status/download routes.
- Implementation notes: Start with route registration tests, then add handler-level fixtures as services become real.
- Acceptance checks: CI fails if any production backlog route is removed or renamed without updating the contract.
- Dependencies: `CB-X-013`, current route contract tests.
- Status: Todo

## CB-X-015 - V0 end-to-end workflow smoke
- Priority: P0
- Area: Quality
- Problem: The product promise depends on the full chain working together, not isolated pages.
- Expected behavior: Automated smoke covers create matter, upload, extract, approve fact, create claim, attach evidence, attach authority, create draft, run checks, open QC, and preview export status.
- Implementation notes: Use a test Neo4j fixture or mocked service boundary; keep it deterministic and safe for local dev.
- Acceptance checks: Smoke fails on broken matter isolation, missing graph links, missing source support, or broken draft/check handoff.
- Dependencies: `CB-V0-018`, `CB-V0-019`, `CB-V0-025`, `CB-V01-015`, `CB-V02-011`.
- Status: Todo

## CB-X-016 - Matter isolation and authorization query audit
- Priority: P0
- Area: Privacy/data integrity
- Problem: New graph, QC, export, and attachment routes increase cross-matter leakage risk.
- Expected behavior: Every CaseBuilder query scopes by `matter_id`, verifies node ownership before writes, and refuses cross-matter IDs.
- Implementation notes: Extend matter isolation tests to documents, facts, evidence, claims, elements, drafts, findings, deadlines, tasks, graph, and exports.
- Acceptance checks: Attempts to attach or fetch matter A records through matter B endpoints return not found or forbidden.
- Dependencies: `CB-X-002`, `CB-X-014`.
- Status: Todo

## CB-X-017 - Storage lifecycle and generated artifact cleanup
- Priority: P0
- Area: Privacy/storage
- Problem: Uploads and generated export artifacts need the same retention/delete guarantees.
- Expected behavior: Matter deletion, archive, and retention workflows cover original uploads, extracted text, generated DOCX/PDF files, export packages, and orphaned objects.
- Implementation notes: Keep local paths and object keys matter-scoped; log IDs and counts, not confidential content.
- Acceptance checks: Storage cleanup tests prove deleted/tombstoned matter files and generated artifacts cannot be downloaded afterward.
- Dependencies: `CB-X-003`, `CB-V02-011`, `CB-V1-010`.
- Status: Todo

## CB-X-018 - Large-matter performance fixture
- Priority: P1
- Area: Performance
- Problem: CaseBuilder must remain usable when a matter contains many documents, facts, evidence rows, draft paragraphs, and findings.
- Expected behavior: Seeded large fixture exercises bounded API responses, lazy UI loading, graph limits, and draft/finding pagination.
- Implementation notes: Include realistic counts for documents, extracted chunks, facts, evidence links, claims/elements, draft sentences, and QC findings.
- Acceptance checks: Core pages render within agreed dev thresholds and never request unbounded graph/finding/document payloads.
- Dependencies: `CB-X-010`, `CB-V01-008`, `CB-V01-015`.
- Status: Todo

## CB-X-019 - Structured logs without sensitive text
- Priority: P1
- Area: Observability/privacy
- Problem: Production debugging needs insight into ingestion, AI, QC, export, and deletion without leaking confidential case content.
- Expected behavior: Logs include matter/document/draft/job IDs, stage names, counts, durations, provider mode, and error codes, but not raw document text or draft contents.
- Implementation notes: Add logging guidance to each service ticket and include regression tests where practical for known unsafe log patterns.
- Acceptance checks: Failed upload, extraction, issue spotting, QC, authority retrieval, and export can be diagnosed from logs without exposing confidential text.
- Dependencies: `CB-X-009`, `CB-X-017`.
- Status: Todo

## CB-X-020 - R2 event notification ingestion queue
- Priority: P1
- Area: Storage/ingestion
- Problem: Upload-complete hooks should eventually enqueue ingestion without relying only on synchronous browser/API flows.
- Expected behavior: Configure R2 object-create notifications to a queue for upload and artifact prefixes, and object-delete notifications for cleanup/reconciliation where useful.
- Implementation notes: Cloudflare R2 event notifications can send bucket change messages to Queues and support event type plus prefix/suffix filters. Keep this optional/configurable for local development.
- Acceptance checks: Creating an object under the configured upload prefix enqueues an ingestion message with bucket/key/etag/size metadata, and duplicate/out-of-order events are idempotent.
- Dependencies: `CB-V0F-017`, `CB-V0-019`, `CB-X-014`.
- Status: Todo

## CB-X-021 - R2 lifecycle and storage-class policy
- Priority: P1
- Area: Storage/retention
- Problem: Evidence, temporary uploads, and generated artifacts need retention and cost policy that does not depend on manual cleanup.
- Expected behavior: Define lifecycle rules for abandoned pending uploads, temporary extraction artifacts, tombstoned exports, and long-lived originals that can transition to Infrequent Access where appropriate.
- Implementation notes: Cloudflare R2 lifecycle rules can delete objects or transition Standard objects to Infrequent Access; Infrequent Access has retrieval-cost tradeoffs, so keep hot preview artifacts in Standard.
- Acceptance checks: Policy docs and config identify prefixes, retention windows, transition timing, and exceptions for legal hold or active matters.
- Dependencies: `CB-X-017`, `CB-V1-012`.
- Status: Todo

## CB-X-022 - R2/Neo4j provenance contract tests
- Priority: P0
- Area: Provenance/testing
- Problem: The evidence lake and graph can drift unless every derived node proves its source object lineage.
- Expected behavior: Contract tests assert that documents, versions, blobs, pages, chunks, evidence spans, facts, and ingestion runs form a complete provenance path back to an R2 object/blob hash.
- Implementation notes: Include tests for missing blob links, stale manifest references, duplicate blob reuse, and derived nodes without source spans.
- Acceptance checks: CI fails when a derived CaseBuilder graph node lacks required object/version/page/chunk/span provenance or references an object outside the matter policy.
- Dependencies: `CB-V0F-014`, `CB-V0F-015`, `CB-V0F-017`, `CB-V0-020`.
- Status: Todo
