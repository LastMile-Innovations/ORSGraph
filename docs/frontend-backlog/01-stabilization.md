# 01 - Stabilization

These tasks come first. They make the frontend buildable, type-safe, navigable, and testable.

## STAB-001 - Restore TypeScript validation
- Priority: P0
- Area: TypeScript
- Problem: `./node_modules/.bin/tsc --noEmit` fails with broad type drift.
- Evidence: Errors include `MatterSummary` passed where full `Matter` is required, missing legacy casebuilder aliases, snake-case/camel-case mismatches, graph Sigma typing issues, and mock data not matching `lib/types.ts`.
- Expected behavior: TypeScript validation passes without suppressions.
- Implementation notes: Normalize the CaseBuilder model first. Either make `getMatterById` return full `Matter` for detail pages or update components to accept summaries plus explicitly passed collections. Then restore missing aliases or migrate callers to current type names. Fix graph Sigma return types and QC corpus status field names.
- Acceptance checks: `./node_modules/.bin/tsc --noEmit` exits 0.
- Dependencies: None.
- Status: Done
- Verification: `pnpm run typecheck` exits 0.

## STAB-002 - Repair lint tooling
- Priority: P0
- Area: Lint
- Problem: `pnpm run lint` fails because `eslint` is not available.
- Evidence: Local run returned `sh: eslint: command not found`.
- Expected behavior: `pnpm run lint` runs a real frontend lint pass.
- Implementation notes: Add Next-compatible ESLint dependencies and config, or replace the script with the repo's chosen checker. Keep rules practical for the current codebase so lint can become a useful gate.
- Acceptance checks: `pnpm run lint` exits 0 or reports actionable lint issues after dependencies install.
- Dependencies: None.
- Status: Done
- Verification: `pnpm run lint` exits 0 with `eslint.config.mjs`.

## STAB-003 - Re-enable build type validation
- Priority: P0
- Area: Next build
- Problem: `next.config.mjs` has `typescript.ignoreBuildErrors: true`.
- Evidence: `pnpm run build` succeeds while saying type validation is skipped.
- Expected behavior: Production build catches type errors.
- Implementation notes: Remove `ignoreBuildErrors` after `STAB-001`, or replace with a short-lived documented guard only if release pressure requires it.
- Acceptance checks: `pnpm run build` validates types and exits 0.
- Dependencies: `STAB-001`.
- Status: Done
- Verification: `typescript.ignoreBuildErrors` was removed and `pnpm run build` exits 0.

## STAB-004 - Fix CaseBuilder matter detail routing
- Priority: P0
- Area: Routing
- Problem: `/matters/matter:smith-abc` renders 404.
- Evidence: Browser check returned a Next 404 for the seeded matter URL.
- Expected behavior: Matter card opens the seeded matter dashboard.
- Implementation notes: Determine whether the failure is from colon IDs, params handling, or lookup mismatch. Choose one canonical route strategy: encoded IDs, URL-safe slugs, or alias mapping. Keep internal `matter_id` stable if other data depends on it.
- Acceptance checks: `/matters/matter:smith-abc` or the chosen canonical matter URL renders `MatterDashboard`.
- Dependencies: None.
- Status: Done
- Verification: `/matters/smith-abc` returns 200 and renders the seeded matter dashboard; legacy `/matters/matter:smith-abc` remains accepted.

## STAB-005 - Repair all matter link generation
- Priority: P0
- Area: Routing
- Problem: Matter links are generated in many components using both `matter.id` and `matter.matter_id`.
- Evidence: Link builders appear in `document-viewer`, `drafts-list`, `evidence-matrix`, `facts-board`, `timeline-view`, `draft-editor`, and `matter-dashboard`.
- Expected behavior: All matter links use the same route helper and resolve correctly.
- Implementation notes: Add a small route helper such as `matterHref(matterId)` and `matterChildHref(matterId, childPath)`, then migrate link builders.
- Acceptance checks: Matter dashboard, documents, document detail, facts anchors, claims anchors, deadlines anchors, drafts, draft detail, authorities, and tasks links do not 404.
- Dependencies: `STAB-004`.
- Status: Done
- Verification: Matter dashboard, documents, authorities, tasks, and core sidebar routes return 200 for clean `/matters/smith-abc/...` URLs.

## STAB-006 - Add broken-route smoke coverage
- Priority: P0
- Area: Test coverage
- Problem: Missing pages and bad matter URLs reached the UI without an automated guard.
- Evidence: Browser pass found 404s for matter dashboard, documents, authorities, and tasks.
- Expected behavior: A smoke test fails when core visible links 404.
- Implementation notes: Use the selected frontend browser test runner from `QUAL-001`. Include direct route visits and click-through from visible cards/nav.
- Acceptance checks: Test command visits core route smoke list and fails on 404.
- Dependencies: `STAB-002`, `QUAL-001`.
- Status: Done
- Verification: `pnpm run smoke:routes` visits 23 core routes, including all seeded matter child routes, and fails on non-2xx, redirects, or default Next 404 bodies.

## STAB-007 - Stop generated artifacts from dirtying the worktree
- Priority: P1
- Area: Repo hygiene
- Problem: Frontend checks can modify generated artifacts.
- Evidence: `frontend/next-env.d.ts` and `frontend/tsconfig.tsbuildinfo` appear dirty in `git status`.
- Expected behavior: Running local checks does not create unrelated dirty changes.
- Implementation notes: Decide whether `tsconfig.tsbuildinfo`, `.next`, and generated Next typings should be ignored or regenerated deterministically.
- Acceptance checks: Clean worktree remains clean after `pnpm run build`, `pnpm run lint`, and `tsc --noEmit`.
- Dependencies: `STAB-001`, `STAB-002`.
- Status: Todo

## STAB-008 - Make API failure logs intentional
- Priority: P1
- Area: Error handling
- Problem: Offline API errors are printed repeatedly in server logs while the UI falls back to mock data.
- Evidence: Dev server logged repeated `TypeError: fetch failed` messages for `/home`, `/ask`, `/statutes`, and statute detail.
- Expected behavior: Offline API state is logged once per area or handled through a structured fallback path.
- Implementation notes: Centralize fetch error handling and fallback metadata. Avoid noisy stack traces for expected local offline state.
- Acceptance checks: With backend offline, dev logs remain readable and UI shows explicit offline/mock state.
- Dependencies: `DATA-001`.
- Status: In progress
- Verification: General ORS API fallback helpers now emit one concise fallback log per endpoint and surface data-state banners; CaseBuilder fallback logging still needs to move into the shared helper.

## STAB-009 - Fix React script-tag console warning
- Priority: P1
- Area: Runtime warnings
- Problem: Browser console reported `Encountered a script tag while rendering React component`.
- Evidence: Dev browser logs showed the warning while navigating 404 matter routes.
- Expected behavior: No React runtime warnings occur during route smoke tests.
- Implementation notes: Reproduce after routing fixes. If it remains, inspect rendered error overlay or components that may inject HTML.
- Acceptance checks: Browser dev logs for core route smoke list contain no React script-tag warning.
- Dependencies: `STAB-004`.
- Status: Done
- Verification: Replaced the `next-themes` wrapper with a local theme provider that does not render an inline script tag.

## STAB-010 - Create a frontend quality command
- Priority: P1
- Area: Developer workflow
- Problem: Validation is split across ad hoc commands and one broken lint script.
- Evidence: Manual audit used `tsc`, `pnpm run lint`, `pnpm run build`, and browser checks separately.
- Expected behavior: One documented command or npm script runs the frontend quality gate.
- Implementation notes: Add `check` or `verify` script that runs lint, typecheck, and build. Keep e2e separate if it requires a dev server.
- Acceptance checks: `pnpm run check` or chosen command exits 0 on a healthy frontend.
- Dependencies: `STAB-001`, `STAB-002`, `STAB-003`.
- Status: Done
- Verification: `pnpm run check` runs lint, typecheck, and production build.
