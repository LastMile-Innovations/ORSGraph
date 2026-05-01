# 02 - Navigation and Pages

These tasks make the visible app map complete and recoverable.

## NAV-001 - Add matter authorities page
- Priority: P0
- Area: CaseBuilder pages
- Problem: Matter sidebar links to `/matters/[id]/authorities`, but no page exists.
- Evidence: Browser route check returned 404.
- Expected behavior: Authorities page shows statutes, rules, cases, and graph-linked authority used by the matter.
- Implementation notes: For v1, build from existing claim/draft authority data in `mock-matters.ts`; later wire to API. Include empty state for matters without authorities.
- Acceptance checks: Sidebar Authorities link opens a non-404 page for seeded matter and empty matters.
- Dependencies: `STAB-004`, `STAB-005`.
- Status: Done
- Verification: `/matters/smith-abc/authorities` returns 200 and renders linked authority rows.

## NAV-002 - Add matter tasks page
- Priority: P0
- Area: CaseBuilder pages
- Problem: Matter sidebar links to `/matters/[id]/tasks`, but no page exists.
- Evidence: Browser route check returned 404.
- Expected behavior: Tasks page lists open/done tasks, priority, due dates, source deadline, and related documents or claims.
- Implementation notes: Use `tasksSmithAbc` for initial rendering and keep actions read-only until persistence exists.
- Acceptance checks: Sidebar Tasks link opens a non-404 page and task counts match sidebar/dashboard.
- Dependencies: `STAB-004`, `STAB-005`.
- Status: Done
- Verification: `/matters/smith-abc/tasks` returns 200 and renders grouped task lanes.

## NAV-003 - Add route-level loading and error states
- Priority: P1
- Area: App router
- Problem: Core routes have no local `loading.tsx` or `error.tsx`.
- Evidence: Only `frontend/app/layout.tsx` exists among app-level route recovery files.
- Expected behavior: Slow or failing server data calls render branded loading/error states.
- Implementation notes: Start with `/search`, `/ask`, `/statutes/[id]`, `/provisions/[id]`, `/sources/[id]`, and `/matters/[id]`.
- Acceptance checks: Simulated fetch errors render route-specific recovery UI instead of generic failures.
- Dependencies: `DATA-001`.
- Status: Done
- Verification: Added loading/error recovery for app-level, Search, Ask, Statute detail, Provision detail, Source detail, and Matter detail routes.

## NAV-004 - Add branded not-found states
- Priority: P1
- Area: App router
- Problem: Missing routes show the generic Next 404.
- Evidence: Matter route failures displayed the default "This page could not be found."
- Expected behavior: Missing statute, provision, source, and matter pages explain what was not found and provide recovery links.
- Implementation notes: Add global `app/not-found.tsx` plus route-specific not-found surfaces where useful.
- Acceptance checks: Unknown statute and unknown matter IDs show branded not-found pages with links back to search/index.
- Dependencies: `STAB-004`.
- Status: Done
- Verification: Added branded not-found recovery for global routes, Matter, Statute, Provision, and Source detail routes.

## NAV-005 - Add CaseBuilder to top-level navigation
- Priority: P1
- Area: Navigation
- Problem: Top nav omits `/matters` even though CaseBuilder is a major app surface.
- Evidence: `TopNav` contains Search, Ask, Statutes, Graph, and QC only.
- Expected behavior: Users can reach Matters/CaseBuilder from the global nav.
- Implementation notes: Add a concise nav item that fits desktop and responsive nav constraints.
- Acceptance checks: `/matters` is reachable from the top nav and active state works.
- Dependencies: None.
- Status: Done
- Verification: Top navigation includes `Matters` and points to `/matters`.

## NAV-006 - Define global shell responsive behavior
- Priority: P2
- Area: Navigation shell
- Problem: The left rail remains visible in constrained layouts and can squeeze main content.
- Evidence: Search and statute screenshots show dense sidebars competing with primary content.
- Expected behavior: Left rail collapses, hides, or moves behind a menu at defined breakpoints.
- Implementation notes: Define breakpoints once in `Shell` and reuse for matter shell if appropriate.
- Acceptance checks: Home, Search, Statute, Graph, and Matter views remain usable at mobile/tablet/desktop widths.
- Dependencies: `UX-001`, `UX-002`, `UX-003`.
- Status: In progress
- Verification: Global left rail now hides below `lg`, right inspector panels hide below `xl`, and Search/Graph dense sidebars no longer squeeze primary content on narrower widths.

## NAV-007 - Standardize active navigation state
- Priority: P2
- Area: Navigation
- Problem: Multiple nav components compute active state independently.
- Evidence: `TopNav`, `LeftRail`, and `MatterSidebar` each implement path checks.
- Expected behavior: Active link behavior is consistent and does not over-select sibling routes.
- Implementation notes: Introduce shared helper only if duplication causes bugs; otherwise document matching rules.
- Acceptance checks: Active state is correct for all core routes and nested matter routes.
- Dependencies: `STAB-005`.
- Status: Todo

## NAV-008 - Add source and provision recovery paths
- Priority: P2
- Area: Deep links
- Problem: Ask, statute, and document views deep-link to provisions and sources that may not exist in mock/API state.
- Evidence: Links are generated to `/provisions/${provision_id}` and `/sources/${source_id}` from multiple surfaces.
- Expected behavior: Missing deep links provide useful recovery instead of generic 404.
- Implementation notes: Use route-level not-found pages with source/provision search suggestions.
- Acceptance checks: Unknown provision/source IDs render branded recovery pages.
- Dependencies: `NAV-004`.
- Status: Done
- Verification: Added route-level not-found states for `/provisions/[id]` and `/sources/[id]`.

## NAV-009 - Add page metadata for key routes
- Priority: P3
- Area: App metadata
- Problem: All routes inherit the same title and description.
- Evidence: Browser title remained `ORSGraph - Legal Operating Environment` across audited routes.
- Expected behavior: Key pages expose route-specific metadata for easier debugging, sharing, and browser history.
- Implementation notes: Add metadata to static pages and generated metadata for statute/matter detail where data is available.
- Acceptance checks: Browser title reflects route context for Search, Ask, Statute detail, Graph, QC, and Matter detail.
- Dependencies: None.
- Status: Todo
