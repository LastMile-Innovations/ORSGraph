# 06 - Performance and Quality

These tasks add durable verification and performance guardrails after stabilization begins.

## QUAL-001 - Add frontend smoke/e2e test runner
- Priority: P1
- Area: Testing
- Problem: No automated browser smoke tests protect core routes.
- Evidence: `frontend/package.json` has `dev`, `build`, `start`, and `lint`, but no e2e or smoke scripts.
- Expected behavior: A local command can start or target the app and verify core routes.
- Implementation notes: Add Playwright or the repo-preferred browser test runner. Keep the first suite small and high-signal.
- Acceptance checks: Test command visits core route smoke list and fails on 404 or uncaught runtime errors.
- Dependencies: `STAB-002`.
- Status: In progress
- Verification: Added `pnpm run smoke:routes`, which checks 23 core routes and fails on non-2xx responses, redirects, and default Next 404 bodies. Browser-level uncaught runtime checks remain queued.

## QUAL-002 - Add visual regression capture
- Priority: P2
- Area: Visual QA
- Problem: Responsive layout regressions are currently manual to catch.
- Evidence: Search, Graph, and Statute issues were found by screenshot inspection.
- Expected behavior: Core routes can be screenshot at desktop, tablet, and mobile widths.
- Implementation notes: Add screenshot generation to e2e runner; start with artifact capture before strict pixel diffing.
- Acceptance checks: Screenshots are generated for `/`, `/search`, `/graph`, `/statutes/or:ors:3.130`, and `/matters`.
- Dependencies: `QUAL-001`.
- Status: Todo

## QUAL-003 - Add route link integrity test
- Priority: P1
- Area: Testing
- Problem: Visible links can point to missing pages.
- Evidence: Matter sidebar linked to missing authorities/tasks pages and matter cards opened 404.
- Expected behavior: A test crawls visible first-level links from core pages and fails on 404.
- Implementation notes: Keep crawl bounded to local app paths and avoid exhaustive graph traversal.
- Acceptance checks: Test catches missing matter sidebar pages and bad matter IDs.
- Dependencies: `QUAL-001`, `STAB-005`.
- Status: In progress
- Verification: Route smoke covers all seeded matter sidebar routes, clean `/matters/smith-abc/...` URLs, and legacy prefixed matter URLs.

## QUAL-004 - Add accessibility smoke checks
- Priority: P2
- Area: Accessibility
- Problem: Keyboard/focus/a11y regressions are not automatically checked.
- Evidence: Dense custom controls exist across major workflows.
- Expected behavior: Automated checks catch critical accessibility violations on core routes.
- Implementation notes: Add axe or equivalent integration to browser tests once route smoke tests are stable.
- Acceptance checks: Core route accessibility smoke passes with no critical violations.
- Dependencies: `QUAL-001`, `UX-006`.
- Status: Todo

## QUAL-005 - Add graph render health checks
- Priority: P2
- Area: Graph quality
- Problem: Graph page needs verification that canvas renders nonblank and controls stay usable.
- Evidence: Graph uses canvas/Sigma and sample fallback data.
- Expected behavior: Tests verify graph canvas is nonblank and inspector opens expected node details.
- Implementation notes: Use screenshot/pixel check plus DOM checks for inspector content.
- Acceptance checks: Graph smoke fails if canvas is blank or selected node inspector is missing.
- Dependencies: `QUAL-001`, `FLOW-003`, `UX-003`.
- Status: Todo

## QUAL-006 - Add performance budgets for core routes
- Priority: P3
- Area: Performance
- Problem: No frontend performance budget exists.
- Evidence: No Lighthouse, Web Vitals, or route timing checks were found.
- Expected behavior: Core routes have basic budgets for load time, client JS size, and graph render time.
- Implementation notes: Start with local measurement and warnings before enforcing CI failures.
- Acceptance checks: Performance report is generated for Home, Search, Graph, Statute detail, and Matter dashboard.
- Dependencies: `QUAL-001`.
- Status: Todo

## QUAL-007 - Audit bundle and dependency usage
- Priority: P3
- Area: Performance
- Problem: The frontend includes many UI dependencies and graph libraries without bundle visibility.
- Evidence: Dependencies include Radix suite, Recharts, Sigma, Graphology, React Flow-like graph components, and Vercel Analytics.
- Expected behavior: Bundle composition is understood and large client surfaces are justified.
- Implementation notes: Add bundle analyzer or Next build profiling. Identify server/client boundary opportunities.
- Acceptance checks: Bundle report exists and top optimization candidates are documented.
- Dependencies: `QUAL-006`.
- Status: Todo

## QUAL-008 - Add API contract tests for mappers
- Priority: P2
- Area: Data quality
- Problem: API mapper functions use `any` and can drift from backend responses.
- Evidence: `mapStatutePage`, `mapProvision`, and `mapProvisionInspector` accept `any`.
- Expected behavior: API DTO fixtures verify mapper output and failure handling.
- Implementation notes: Add focused tests for statute detail, provisions, citations, semantics, history, and provision inspector mapping.
- Acceptance checks: Mapper tests catch missing/renamed fields and preserve UI view model shape.
- Dependencies: `DATA-008`.
- Status: Todo

## QUAL-009 - Add error logging policy
- Priority: P3
- Area: Observability
- Problem: Expected offline state and unexpected runtime errors are not differentiated.
- Evidence: Dev server logs repeated fetch stack traces while app used mock fallback.
- Expected behavior: Expected offline/mock state is quiet and visible in UI; unexpected errors are logged with context.
- Implementation notes: Add a small logging helper or data fetch wrapper. Avoid leaking sensitive request details.
- Acceptance checks: Offline local mode logs concise notices; unexpected fetch/mapping errors include endpoint and route context.
- Dependencies: `DATA-001`, `STAB-008`.
- Status: In progress
- Verification: Shared ORS API fallback paths now log one concise notice per endpoint; CaseBuilder and unexpected runtime error logging policy remain queued.

## QUAL-010 - Add release checklist for frontend changes
- Priority: P3
- Area: Process
- Problem: There is no written frontend release/verification checklist.
- Evidence: Current verification plan is in this backlog only.
- Expected behavior: Engineers know which commands, routes, and screenshots to verify before merging frontend work.
- Implementation notes: Add a checklist to this README or a separate `docs/frontend-release-checklist.md` after quality commands exist.
- Acceptance checks: Checklist references lint, typecheck, build, route smoke, and responsive screenshots.
- Dependencies: `STAB-010`, `QUAL-001`, `QUAL-002`.
- Status: Todo
