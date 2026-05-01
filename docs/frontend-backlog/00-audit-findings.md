# Frontend Audit Findings

This file is the raw inventory from the frontend review. Implementation-ready tasks are split into the epic files.

## Build and tooling

## AUD-001 - TypeScript validation fails
- Priority: P0
- Area: Build tooling
- Problem: `tsc --noEmit` fails with many frontend type errors.
- Evidence: Fresh `./node_modules/.bin/tsc --noEmit` reports failures across `app/matters/[id]/*`, `components/casebuilder/*`, `components/graph/GraphCanvasSigma.tsx`, `components/orsg/qc/qc-console-client.tsx`, `lib/casebuilder/mock-matters.ts`, and `lib/mock-data.ts`.
- Expected behavior: TypeScript validation passes without suppressing errors.
- Implementation notes: Start with casebuilder type/model drift, then graph Sigma types, then ORSGraph mock/API model drift.
- Acceptance checks: `./node_modules/.bin/tsc --noEmit` exits 0.
- Dependencies: None.
- Status: Done
- Verification: `pnpm run typecheck` exits 0.

## AUD-002 - Next build hides type failures
- Priority: P0
- Area: Build tooling
- Problem: `next.config.mjs` sets `typescript.ignoreBuildErrors: true`, allowing production builds while type validation fails.
- Evidence: `pnpm run build` succeeds while reporting "Skipping validation of types".
- Expected behavior: Production build fails on type errors or the bypass is explicitly scoped to an accepted temporary state.
- Implementation notes: Complete `STAB-001` before removing the bypass.
- Acceptance checks: `pnpm run build` validates TypeScript or a documented temporary guard exists with an owner and removal task.
- Dependencies: `STAB-001`.
- Status: Done
- Verification: `ignoreBuildErrors` removed; `pnpm run build` exits 0.

## AUD-003 - Lint script is broken
- Priority: P0
- Area: Build tooling
- Problem: `pnpm run lint` fails because `eslint` is not installed or configured.
- Evidence: `sh: eslint: command not found`.
- Expected behavior: `pnpm run lint` runs consistently in local and CI contexts.
- Implementation notes: Add the Next-compatible ESLint dependency/config or replace the script with the repo-standard checker.
- Acceptance checks: `pnpm run lint` exits 0 or reports actionable lint issues.
- Dependencies: None.
- Status: Done
- Verification: `pnpm run lint` exits 0.

## AUD-004 - Generated frontend artifacts are present in the worktree
- Priority: P1
- Area: Repo hygiene
- Problem: `frontend/tsconfig.tsbuildinfo` and `frontend/.next` are present locally, and `frontend/next-env.d.ts` plus `frontend/tsconfig.tsbuildinfo` appear dirty after checks.
- Evidence: `git status --short` shows frontend generated artifacts modified.
- Expected behavior: Generated files are ignored or intentionally tracked with stable generation rules.
- Implementation notes: Confirm whether these files are intentionally tracked before changing `.gitignore`.
- Acceptance checks: Running build/type checks does not leave unrelated dirty files.
- Dependencies: None.
- Status: Todo

## Routes and navigation

## AUD-005 - Matter detail route renders 404
- Priority: P0
- Area: CaseBuilder routing
- Problem: Matter cards link to `/matters/matter:smith-abc`, but the browser renders a Next 404.
- Evidence: Browser check of `/matters/matter:smith-abc` returned 404 even though `frontend/app/matters/[id]/page.tsx` exists and `matters` contains `matter:smith-abc`.
- Expected behavior: Opening a matter card loads the seeded matter dashboard.
- Implementation notes: Investigate dynamic segment handling for colon-containing IDs and align URLs, decoding, and lookup IDs.
- Acceptance checks: `/matters/matter:smith-abc` or the chosen URL-safe equivalent opens the matter dashboard.
- Dependencies: None.
- Status: Done
- Verification: `/matters/matter:smith-abc` returns 200.

## AUD-006 - Matter sidebar links point to missing pages
- Priority: P0
- Area: Navigation
- Problem: `MatterSidebar` links to `/matters/[id]/authorities` and `/matters/[id]/tasks`, but no matching pages exist.
- Evidence: Browser checks for `/matters/matter:smith-abc/authorities` and `/matters/matter:smith-abc/tasks` returned 404.
- Expected behavior: Visible navigation never points to missing pages.
- Implementation notes: Add pages if the workflow should exist now; otherwise hide or disable links with clear state.
- Acceptance checks: Every visible matter sidebar link resolves to a non-404 page.
- Dependencies: `STAB-004`.
- Status: Done
- Verification: Authorities and Tasks pages exist and return 200 for the seeded matter.

## AUD-007 - No route-level recovery surfaces
- Priority: P1
- Area: App routing
- Problem: The app only has `app/layout.tsx`; no route-level `loading.tsx`, `error.tsx`, or `not-found.tsx` files were found.
- Evidence: `find frontend/app -name 'loading.tsx' -o -name 'error.tsx' -o -name 'not-found.tsx'` found none.
- Expected behavior: Core routes have understandable loading, error, and not-found states.
- Implementation notes: Add route-level recovery for Search, Ask, Graph, Statutes, Sources, QC, and Matters.
- Acceptance checks: Forced errors and missing IDs render branded recovery states.
- Dependencies: None.
- Status: In progress
- Verification: Added loading, error, and not-found recovery for `/matters/[id]`; other core routes remain queued.

## AUD-008 - Matter URL IDs are not normalized
- Priority: P1
- Area: Routing
- Problem: Matter IDs contain `:` and are used directly in URLs across components.
- Evidence: Links such as `href={`/matters/${m.matter_id}`}` generate `/matters/matter:smith-abc`.
- Expected behavior: Route params have one canonical URL representation and a reversible mapping to internal IDs.
- Implementation notes: Choose encoded IDs, slugs, or route-safe aliases; update all link builders and lookup helpers together.
- Acceptance checks: Matter cards, sidebar links, document links, draft links, facts anchors, and dashboard actions resolve consistently.
- Dependencies: `STAB-004`.
- Status: Done
- Verification: Matter lookups now decode route segments and return a normalized full `Matter`.

## Mock data, static flows, and API state

## AUD-009 - Mock fallback is broad and silent
- Priority: P1
- Area: Data integration
- Problem: `lib/api.ts` falls back to mock data for home, health, graph insights, featured statutes, statute index, statute detail, ask, and provisions.
- Evidence: Browser and server logs show failed API calls falling back to mock data.
- Expected behavior: Users can tell when data is live, mock, or offline.
- Implementation notes: Centralize fallback metadata so pages render consistent banners and labels.
- Acceptance checks: Offline backend produces explicit mock/offline state on every affected page.
- Dependencies: None.
- Status: In progress
- Verification: Shared data-state primitives now label fallback/demo state on major ORS and CaseBuilder surfaces; QC and remaining endpoint contracts still need completion.

## AUD-010 - Top navigation health is hard-coded
- Priority: P1
- Area: Data integration
- Problem: `TopNav` displays `neo4j ok`, `qc warn`, and `edition 2025` as static text.
- Evidence: API was offline, but the nav still displayed `neo4j ok`.
- Expected behavior: Health indicators reflect real health data or clearly say unavailable/mock.
- Implementation notes: Feed health from a provider, server component wrapper, or client fetch with fallback.
- Acceptance checks: With the API offline, nav does not claim Neo4j is OK.
- Dependencies: `DATA-001`.
- Status: Done
- Verification: Top nav now displays unknown health instead of hard-coded `neo4j ok` / `qc warn`.

## AUD-011 - Graph page silently uses sample graph
- Priority: P1
- Area: Graph workflow
- Problem: The graph page shows sample nodes when the API fails.
- Evidence: Browser check showed `API unavailable: Failed to fetch` while still rendering bundled graph data.
- Expected behavior: Sample data is clearly labeled, and users can distinguish sample/demo graph from live graph.
- Implementation notes: Keep the graceful fallback, but make the state prominent and prevent actions that imply live data.
- Acceptance checks: Graph fallback state is visually clear and included in smoke tests.
- Dependencies: `DATA-001`.
- Status: Todo

## AUD-012 - Search has no mock fallback and cramped empty state
- Priority: P1
- Area: Search workflow
- Problem: Server search catches initial API errors and renders empty state; client searches show raw API errors.
- Evidence: `/search` rendered no results while the backend was offline; empty state was squeezed beside filters.
- Expected behavior: Search distinguishes not searched, no results, API unavailable, and mock/demo states.
- Implementation notes: Avoid making API unavailable look like zero results.
- Acceptance checks: Offline API renders an explicit unavailable state with retry and suggested demo query behavior.
- Dependencies: `DATA-001`.
- Status: Todo

## AUD-013 - Ask route defaults to a canned query
- Priority: P2
- Area: Ask workflow
- Problem: `/ask` defaults to "What Oregon laws define district attorney duties?" and falls back to mock answer.
- Evidence: `app/ask/page.tsx` sets the default question and calls `askWithFallback`.
- Expected behavior: First-run Ask experience is intentional and clearly labeled as example or live answer.
- Implementation notes: Decide whether `/ask` should start blank, use examples, or show a demo answer with a badge.
- Acceptance checks: User can tell whether the answer is a live API response or demo fallback.
- Dependencies: `DATA-001`.
- Status: Todo

## AUD-014 - New Matter create action opens a broken demo route
- Priority: P1
- Area: CaseBuilder workflow
- Problem: `create matter` is a link to `/matters/matter:smith-abc`, which currently 404s.
- Evidence: `NewMatterClient` hard-codes the create CTA to the seeded demo matter.
- Expected behavior: Create either creates a real matter, opens a valid demo matter, or is clearly disabled/gated.
- Implementation notes: After routing is repaired, decide whether this remains a demo or becomes a real create flow.
- Acceptance checks: CTA does not navigate to 404 and does not imply persistence when none exists.
- Dependencies: `STAB-004`.
- Status: Todo

## AUD-015 - Complaint upload is static
- Priority: P2
- Area: Complaint Analyzer
- Problem: The "select file" button in the upload step has no file input or upload behavior.
- Evidence: `ComplaintClient` renders a button without an action in `UploadStep`.
- Expected behavior: Upload either works or is clearly marked as demo/static.
- Implementation notes: Add upload contract or demo-state copy and disabled action.
- Acceptance checks: User cannot mistake static mock analysis for newly uploaded analysis.
- Dependencies: `DATA-004`.
- Status: Todo

## AUD-016 - Fact Check is static mock report
- Priority: P2
- Area: Fact Check
- Problem: `/fact-check` imports `factCheckReport` from mock data and has no upload/paste analysis entry point.
- Evidence: `app/fact-check/page.tsx` imports `@/lib/mock-fact-check`.
- Expected behavior: User can start a fact-check or see a clearly labeled demo report.
- Implementation notes: Add entry state, API contract, and demo badge.
- Acceptance checks: Static report is clearly labeled or replaced with live workflow.
- Dependencies: `DATA-004`.
- Status: Todo

## UX and responsive layout

## AUD-017 - Search layout is cramped at narrow widths
- Priority: P2
- Area: Responsive UX
- Problem: Search keeps the left rail and filter panel visible while the empty state becomes narrow and vertically awkward.
- Evidence: Browser screenshot showed the empty state squeezed into a narrow right column.
- Expected behavior: Search adapts with collapsible filters/rail and usable content width.
- Implementation notes: Define breakpoints for hiding/collapsing left rail and filters.
- Acceptance checks: Search is usable at mobile, tablet, and desktop widths.
- Dependencies: None.
- Status: In progress
- Verification: Global left rail and Search filters now collapse below `lg`, preventing the narrow empty/result column seen in the audit.

## AUD-018 - Graph layout needs responsive framing
- Priority: P2
- Area: Responsive UX
- Problem: Graph page uses dense panels and a canvas that can become visually constrained.
- Evidence: Browser screenshot showed controls, warning, canvas, and inspector competing for space.
- Expected behavior: Graph canvas remains usable with controls and inspector collapsed or stacked on smaller viewports.
- Implementation notes: Add responsive shell for graph tools and inspector.
- Acceptance checks: Graph is usable at mobile, tablet, and desktop widths with nonblank canvas.
- Dependencies: None.
- Status: In progress
- Verification: Graph inspector hides below `xl` and graph controls remain out of the primary canvas at narrower widths; browser screenshot/pixel checks remain queued.

## AUD-019 - Statute detail content is cramped by right inspector
- Priority: P2
- Area: Responsive UX
- Problem: Statute detail view can leave primary text narrow while the inspector dominates.
- Evidence: Browser screenshot showed the intelligence panel taking substantial width over the statute text.
- Expected behavior: Statute text remains readable and inspector can collapse or move below at smaller widths.
- Implementation notes: Define responsive behavior for left rail, main statute text, and right inspector.
- Acceptance checks: Statute text is readable and controls remain accessible at tested widths.
- Dependencies: None.
- Status: In progress
- Verification: The global right inspector collapses below `xl`, giving statute text the primary width on tablet/narrow desktop layouts.

## AUD-020 - Home uses hard-coded dark colors
- Priority: P3
- Area: Visual system
- Problem: Home page uses `bg-zinc-950`, `text-zinc-100`, and related hard-coded colors instead of design tokens.
- Evidence: `app/page.tsx` and home components use zinc/indigo utility colors.
- Expected behavior: Home follows the app theme tokens and works in light/dark modes.
- Implementation notes: Replace hard-coded palette with token-based styles after stabilization.
- Acceptance checks: Theme toggle does not produce inconsistent home styling.
- Dependencies: None.
- Status: Todo

## AUD-021 - Dense legal UI needs keyboard and focus audit
- Priority: P2
- Area: Accessibility
- Problem: Many custom buttons, filters, graph controls, and clickable cards need keyboard/focus verification.
- Evidence: Numerous clickable div/card patterns and dense panels exist across Search, Fact Check, Graph, and CaseBuilder.
- Expected behavior: Core interactions are keyboard reachable, visibly focused, and named for assistive tech.
- Implementation notes: Audit with browser keyboard pass and automated accessibility checks once lint/test tooling exists.
- Acceptance checks: Keyboard can complete core route smoke workflows.
- Dependencies: `QUAL-001`.
- Status: Todo

## Tests, performance, and quality gaps

## AUD-022 - No frontend smoke/e2e tests
- Priority: P1
- Area: Testing
- Problem: No route smoke or click-through tests are present for the frontend.
- Evidence: No Playwright/Vitest/e2e scripts were found in `frontend/package.json`.
- Expected behavior: Core route smoke list is checked automatically.
- Implementation notes: Add the smallest test suite that catches route 404s and broken nav.
- Acceptance checks: CI/local command fails if a visible core nav link 404s.
- Dependencies: `STAB-002`.
- Status: In progress
- Verification: Added `pnpm run smoke:routes`; full click-through/browser e2e remains queued.

## AUD-023 - No visual regression baseline
- Priority: P2
- Area: Visual QA
- Problem: Responsive and dense layout regressions require manual screenshot checks.
- Evidence: No visual screenshot test or artifact workflow was found.
- Expected behavior: Core layouts can be captured and compared before closing visual tasks.
- Implementation notes: Add screenshot capture for desktop, tablet, and mobile widths.
- Acceptance checks: Visual artifacts are generated for core routes.
- Dependencies: `QUAL-001`.
- Status: Todo

## AUD-024 - Graph rendering performance is unmeasured
- Priority: P3
- Area: Performance
- Problem: Graph rendering uses Sigma/React Flow components and bundled sample data, but performance budgets are not defined.
- Evidence: Graph dependencies include `sigma` and `graphology`; no performance checks exist.
- Expected behavior: Graph load/render behavior is measured and protected.
- Implementation notes: Start with timing marks and simple node/edge count thresholds.
- Acceptance checks: Graph route has a baseline load/render measurement.
- Dependencies: `QUAL-001`.
- Status: Todo
