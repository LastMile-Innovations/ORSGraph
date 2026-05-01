# 04 - Core Workflows

These tasks build the product surfaces users actually work through.

## FLOW-001 - Complete Search workflow states
- Priority: P1
- Area: Search
- Problem: Search does not clearly distinguish empty, offline, no-result, and result states.
- Evidence: Offline backend produced empty/error behavior while filters and suggested searches remained visible.
- Expected behavior: Search has polished states for start, loading, API unavailable, no results, results, pagination, and filter changes.
- Implementation notes: Use structured data state from `DATA-006`; collapse filters when unavailable or on constrained widths.
- Acceptance checks: Search for a query with API offline shows explicit unavailable state and retry; no-query state remains suggestion-focused.
- Dependencies: `DATA-006`, `UX-001`.
- Status: Todo

## FLOW-002 - Complete Ask workflow provenance
- Priority: P1
- Area: Ask
- Problem: Ask can render a default mocked answer without enough provenance clarity.
- Evidence: `/ask` calls `askWithFallback` and defaults to a canned question.
- Expected behavior: Users understand whether an answer is live, demo, cached, or unavailable.
- Implementation notes: Add answer metadata display, empty start option, example prompts, and disabled/retry states when live Ask is unavailable.
- Acceptance checks: Offline Ask clearly labels demo/mock answer and does not imply live legal analysis.
- Dependencies: `DATA-007`.
- Status: Todo

## FLOW-003 - Complete Graph explorer fallback behavior
- Priority: P1
- Area: Graph
- Problem: Graph renders bundled sample data when API fails, but user actions still look live.
- Evidence: Browser showed `API unavailable: Failed to fetch` and sample graph nodes.
- Expected behavior: Graph exposes live/sample state and scopes available actions accordingly.
- Implementation notes: Add sample-mode banner, retry, and source labels in inspector. Ensure "Open" links still resolve.
- Acceptance checks: API unavailable state is visible in graph toolbar, canvas, and inspector.
- Dependencies: `DATA-002`, `UX-003`.
- Status: Todo

## FLOW-004 - Complete Statute detail workflow
- Priority: P1
- Area: Statutes
- Problem: Statute detail mixes API detail and mock fallback, with complex tabs and inspector.
- Evidence: `getStatutePageData` falls back to mock statute; UI renders source, provisions, citations, semantics, chunks, QC, and versions.
- Expected behavior: Statute page shows reliable live/mock state, readable text, complete tabs, and source/legal warning.
- Implementation notes: Add data-state banner, tab empty states, loading/error/not-found handling, and responsive inspector.
- Acceptance checks: `ORS 3.130`, indexed-but-not-loaded statutes, and unknown statutes each render correct state.
- Dependencies: `DATA-001`, `NAV-004`, `UX-002`.
- Status: Todo

## FLOW-005 - Complete Sources workflow
- Priority: P2
- Area: Sources
- Problem: Sources are entirely mock-backed and detail fields include hard-coded values.
- Evidence: `/sources` imports `sourceIndex` from `mock-sources`; source detail shows fixed field values such as provisions count.
- Expected behavior: Sources page reads source index/detail data through API adapter or clearly labels demo data.
- Implementation notes: Define source index/detail endpoint contract and update detail page fields to real data.
- Acceptance checks: Source list and detail show data source state and no hard-coded detail metrics.
- Dependencies: `DATA-001`, `DATA-009`.
- Status: Todo

## FLOW-006 - Convert New Matter from demo link to real or gated flow
- Priority: P1
- Area: CaseBuilder
- Problem: New Matter collects local form state and file names, then links to seeded demo matter.
- Evidence: `NewMatterClient` uses local state and hard-coded `href="/matters/matter:smith-abc"`.
- Expected behavior: User can create a matter, or the UI clearly states it is opening a demo matter.
- Implementation notes: If backend is not ready, change CTA to "Open demo matter" and preserve entered data only as local preview. If backend is ready, call create endpoint and route to the created matter.
- Acceptance checks: CTA never opens 404 and never implies saved data unless persistence happened.
- Dependencies: `STAB-004`, `DATA-004`.
- Status: Todo

## FLOW-007 - Complete Matter dashboard data shape
- Priority: P1
- Area: CaseBuilder
- Problem: Matter dashboard expects richer legacy data than current types provide.
- Evidence: TypeScript errors reference missing `CaseDefense`, `CaseTask`, `MatterParty`, `deadline_id`, `draft_id`, and other fields.
- Expected behavior: Dashboard renders from a coherent full matter model.
- Implementation notes: Decide canonical model once; avoid keeping parallel snake-case and camel-case properties unless compatibility requires it.
- Acceptance checks: Matter dashboard typechecks and renders seeded matter with correct counts and sections.
- Dependencies: `STAB-001`, `DATA-005`.
- Status: Todo

## FLOW-008 - Complete Matter documents and document detail
- Priority: P1
- Area: CaseBuilder
- Problem: Document library/viewer have type drift and currently depend on mock document extraction.
- Evidence: TypeScript errors mention missing `DocumentType`, `folder`, `parties_mentioned`, `entities_mentioned`, `uploaded_at`, and extraction fields.
- Expected behavior: Documents list, filters, viewer, extracted facts, entities, clauses, and issues render from canonical document data.
- Implementation notes: Normalize document model and add explicit empty states for extraction pending/failed.
- Acceptance checks: Documents page and a document detail page typecheck and render seeded data.
- Dependencies: `STAB-001`, `DATA-005`.
- Status: Todo

## FLOW-009 - Complete Matter facts, timeline, evidence, and claims
- Priority: P1
- Area: CaseBuilder
- Problem: Evidence/facts/claims/timeline components expect mismatched fields.
- Evidence: TypeScript errors include `supportingFactIds`, `snippet`, `description`, `argument`, and `response` mismatches.
- Expected behavior: Evidence layer pages render from one canonical matter graph model.
- Implementation notes: Build a shared selector layer for derived relationships such as facts supporting claims and evidence coverage.
- Acceptance checks: Facts, Timeline, Evidence, and Claims pages typecheck and render seeded matter.
- Dependencies: `STAB-001`, `FLOW-007`.
- Status: Todo

## FLOW-010 - Complete Matter Ask workflow
- Priority: P2
- Area: CaseBuilder Ask
- Problem: Matter Ask uses local mock replies and citation shapes that do not match types.
- Evidence: TypeScript errors in `ask-matter.tsx` reference missing citation fields and chat thread properties.
- Expected behavior: Matter Ask has clear demo/live state, coherent citations, and scoped source references.
- Implementation notes: Align `MatterChatMessage`, `MatterChatCitation`, and `ChatThread` with UI needs or simplify UI to current model.
- Acceptance checks: Matter Ask typechecks and citations link to valid matter/source routes.
- Dependencies: `STAB-001`, `STAB-005`, `DATA-005`.
- Status: Todo

## FLOW-011 - Complete Draft editor workflow
- Priority: P2
- Area: Drafts
- Problem: Draft editor has mock suggestions and type drift around comments, sources, cite-check issues, and suggestion kinds.
- Evidence: TypeScript errors in `draft-editor.tsx` reference `insert` kind, object source labels, missing `title/detail`, and missing comment IDs.
- Expected behavior: Draft editor supports read, save state, AI suggestion demo/live state, cite-check results, and citation links coherently.
- Implementation notes: Add explicit draft action states: local-only, saved, generated, cite-check pending, cite-check failed.
- Acceptance checks: Draft list and draft detail typecheck and do not imply persistence when unavailable.
- Dependencies: `STAB-001`, `DATA-004`, `DATA-005`.
- Status: Todo

## FLOW-012 - Complete Complaint Analyzer upload and response flow
- Priority: P2
- Area: Complaint Analyzer
- Problem: The upload step is static and analysis is preloaded from mock data.
- Evidence: `ComplaintClient` receives `complaintAnalysis` from `mock-complaint`.
- Expected behavior: User can upload/paste a complaint or view a clearly labeled demo analysis.
- Implementation notes: Add upload/paste entry state, API contract, pending analysis state, failure state, and demo badge.
- Acceptance checks: Upload step has a real input or disabled demo explanation; analysis provenance is visible.
- Dependencies: `DATA-004`.
- Status: Todo

## FLOW-013 - Complete Fact Check entry workflow
- Priority: P2
- Area: Fact Check
- Problem: Fact Check starts directly inside a seeded report.
- Evidence: `/fact-check` imports static `factCheckReport`.
- Expected behavior: User can paste/upload a draft or explicitly open a demo report.
- Implementation notes: Add landing state for input, then report state. Keep report UI but label demo/live state.
- Acceptance checks: `/fact-check` starts with a usable entry point or a clear demo selector.
- Dependencies: `DATA-004`, `UX-006`.
- Status: Todo

## FLOW-014 - Complete QC workflow
- Priority: P2
- Area: QC
- Problem: QC console is mock-backed and type drift exists around corpus status fields.
- Evidence: `qc-console-client.tsx` imports `qcCorpus` and `corpusStatus` from mock data and TypeScript errors reference missing fields.
- Expected behavior: QC console displays live summary or clear mock/offline state with actionable panels.
- Implementation notes: Align `CorpusStatus` type, add API adapter, and use real `/qc/summary` where available.
- Acceptance checks: QC page typechecks and labels live/mock/offline state.
- Dependencies: `STAB-001`, `DATA-001`.
- Status: Todo

