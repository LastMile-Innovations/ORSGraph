# 03 - Data and API Integration

These tasks replace ambiguous mock behavior with explicit live, offline, and demo states.

## DATA-001 - Define frontend API fallback policy
- Priority: P1
- Area: API integration
- Problem: API calls fall back inconsistently across pages.
- Evidence: `lib/api.ts` catches and returns mock data for several functions, while search can show an error or empty state.
- Expected behavior: Every API call has a declared policy: live required, mock allowed, offline state, or disabled.
- Implementation notes: Add a central `DataState` concept such as `source: "live" | "mock" | "offline"` and `error?: string`. Do not silently erase failures.
- Acceptance checks: Each core page can render and label live, mock, and offline states.
- Dependencies: None.
- Status: In progress
- Verification: Added shared `DataState`/`DataStateBanner` primitives and wired Home, Search, Ask, Graph, Statute, Provision, Sources, Complaint, Fact Check, and Matter surfaces into explicit live/mock/demo/offline labeling. Remaining work is to bring CaseBuilder API logging fully onto the shared helper and add DTO tests.

## DATA-002 - Make mock/offline state visible across pages
- Priority: P1
- Area: UX/data clarity
- Problem: Users cannot reliably tell when data is mock.
- Evidence: Home shows a mock banner, but top nav still claims `neo4j ok`; Graph shows a warning while rendering sample nodes; Ask renders mock answer.
- Expected behavior: All mock/demo/offline content is labeled consistently.
- Implementation notes: Create shared badges/banners for `Live`, `Mock`, `Offline`, and `Demo`.
- Acceptance checks: Backend offline state is visible on Home, Ask, Graph, Statutes, Search, Sources, QC, and Matters where relevant.
- Dependencies: `DATA-001`.
- Status: In progress
- Verification: Offline/mock/demo banners are visible on the major fallback/demo pages; QC still needs a live health/data-state source rather than static client data.

## DATA-003 - Replace hard-coded nav health
- Priority: P1
- Area: Health data
- Problem: Top nav displays static health information.
- Evidence: `TopNav` hard-codes `neo4j ok`, `qc warn`, and `edition 2025`.
- Expected behavior: Health reflects API status or clearly says unavailable/mock.
- Implementation notes: Fetch health from the server in the shell or pass health into `TopNav`; avoid per-page duplicated fetches if possible.
- Acceptance checks: With the API offline, nav does not show false healthy status.
- Dependencies: `DATA-001`.
- Status: Done
- Verification: Top navigation no longer claims `neo4j ok` or `qc warn`; it shows unknown health until real shell health wiring exists.

## DATA-004 - Define upload/analyze endpoint contracts
- Priority: P1
- Area: Workflow APIs
- Problem: New Matter, Complaint Analyzer, and Fact Check present upload/analyze concepts without real contracts.
- Evidence: New Matter stores selected files locally and links to demo; Complaint has a static select button; Fact Check imports a static report.
- Expected behavior: Each upload/analyze flow has a documented endpoint contract or is explicitly demo-only.
- Implementation notes: Define minimum request/response shapes for matter creation, document upload, complaint analysis, and fact-check analysis.
- Acceptance checks: Backlog includes API contracts and UI can render pending, success, failure, and demo states.
- Dependencies: None.
- Status: Todo

## DATA-005 - Add matter data API adapter
- Priority: P1
- Area: CaseBuilder data
- Problem: CaseBuilder reads directly from `mock-matters.ts`.
- Evidence: Matter pages import `getMatterById`, `getDocumentsByMatter`, `getClaimsByMatter`, and related helpers from mock data.
- Expected behavior: Pages call a data adapter that can switch between API and mock/demo data.
- Implementation notes: Add `lib/casebuilder/api.ts` or similar. Keep mock helpers behind adapter boundaries.
- Acceptance checks: Matter pages no longer import mock data directly except in the adapter.
- Dependencies: `STAB-001`, `DATA-001`.
- Status: Done
- Verification: Matter pages load through `lib/casebuilder/api.ts`, which normalizes live responses and falls back to seeded demo matters through the adapter.

## DATA-006 - Add search fallback semantics
- Priority: P1
- Area: Search data
- Problem: Offline search can look like no results or raw failure.
- Evidence: Initial `/search` catches errors to `undefined`; client search catches and displays an error string.
- Expected behavior: Search shows distinct states for no query, loading, API unavailable, no results, and results.
- Implementation notes: Return structured state from page loader and client fetches.
- Acceptance checks: Offline backend renders API unavailable state, not zero-result state.
- Dependencies: `DATA-001`.
- Status: Done
- Verification: Search server and client calls use `searchWithParamsState`, preserving API-unavailable state separately from no-results state.

## DATA-007 - Add Ask provenance and fallback labels
- Priority: P1
- Area: Ask data
- Problem: Ask answer can be mock fallback without clear provenance.
- Evidence: `askWithFallback` returns `mockAskAnswer` after API failure.
- Expected behavior: Answers show whether they are live, mock, stale, or generated from bundled demo data.
- Implementation notes: Include source state in `AskAnswer` wrapper instead of mutating answer body.
- Acceptance checks: Offline Ask answer carries visible mock/offline label and warning.
- Dependencies: `DATA-001`.
- Status: Done
- Verification: Ask responses use `askWithFallbackState` and render a data-state banner when the answer is fallback/mock/offline.

## DATA-008 - Align frontend models with API models
- Priority: P1
- Area: Types/API contracts
- Problem: Frontend types and API mapping functions contain ad hoc normalization.
- Evidence: `lib/api.ts` maps statuses, provision types, citations, semantics, and history manually; TypeScript failures show schema drift in mocks.
- Expected behavior: API response types and frontend view models are explicit and tested.
- Implementation notes: Separate API DTO types from UI view models. Keep mapping functions typed instead of `any`.
- Acceptance checks: Mapper unit tests or type-level checks catch shape drift.
- Dependencies: `STAB-001`.
- Status: Todo

## DATA-009 - Document backend endpoint gaps
- Priority: P2
- Area: API roadmap
- Problem: Frontend surfaces imply endpoints that may not exist.
- Evidence: README lists `POST /api/v1/ask` as stub 501; UI includes matter, upload, complaint, fact-check, authorities, tasks, graph, and QC workflows.
- Expected behavior: Missing endpoints are listed with minimum contracts and priority.
- Implementation notes: Add a backend dependency section to the relevant flow tasks or a separate API contract doc if it grows.
- Acceptance checks: Each frontend demo/static flow references a concrete API dependency or demo decision.
- Dependencies: `DATA-004`.
- Status: Todo

## DATA-010 - Add seeded demo mode switch
- Priority: P3
- Area: Demo data
- Problem: Mock data is interwoven with normal app behavior.
- Evidence: Mock imports exist across app pages and components.
- Expected behavior: Demo mode is explicit, optional, and easy to disable.
- Implementation notes: Use env config or app data-state provider to choose live-only, live-with-fallback, or demo mode.
- Acceptance checks: Running in live-only mode does not silently show bundled demo data.
- Dependencies: `DATA-001`, `DATA-005`.
- Status: Todo
