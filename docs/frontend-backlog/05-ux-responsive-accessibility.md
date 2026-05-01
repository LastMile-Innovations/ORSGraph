# 05 - UX, Responsive Layout, and Accessibility

These tasks make the app usable across screen sizes and interaction modes.

## UX-001 - Fix Search responsive layout
- Priority: P2
- Area: Responsive layout
- Problem: Search content can become too narrow because global left rail and filters remain visible.
- Evidence: Browser screenshot showed the empty state squeezed into a narrow right column.
- Expected behavior: Search keeps enough content width at mobile, tablet, and desktop sizes.
- Implementation notes: Collapse filters into a drawer or top filter bar below a breakpoint. Consider hiding global left rail on Search at narrow widths.
- Acceptance checks: At 390px, 768px, and desktop widths, search input, filters, empty/error/results states are readable and not overlapped.
- Dependencies: `NAV-006`.
- Status: Todo

## UX-002 - Fix Statute detail responsive layout
- Priority: P2
- Area: Responsive layout
- Problem: The statute right inspector can dominate the viewport and reduce legal text readability.
- Evidence: Browser screenshot showed the intelligence panel taking substantial width while text was cramped.
- Expected behavior: Statute text remains readable and inspector is collapsible, below-content, or drawer-based on constrained viewports.
- Implementation notes: Define breakpoints for left rail, statute tabs/text, and right inspector. Preserve quick access to definitions, deadlines, citations, chunks, and QC.
- Acceptance checks: `ORS 3.130` page is readable at 390px, 768px, and desktop widths.
- Dependencies: `NAV-006`.
- Status: Todo

## UX-003 - Fix Graph responsive layout and canvas framing
- Priority: P2
- Area: Responsive layout
- Problem: Graph controls, warning, canvas, and inspector compete for space.
- Evidence: Browser screenshot showed dense graph panels and canvas sharing a narrow area.
- Expected behavior: Graph canvas remains nonblank, framed, and usable with inspector/controls accessible.
- Implementation notes: Collapse inspector into drawer below tablet, keep canvas min-height stable, and provide explicit sample/offline badge.
- Acceptance checks: Graph renders nonblank at tested widths and no controls overlap canvas labels.
- Dependencies: `FLOW-003`, `NAV-006`.
- Status: Todo

## UX-004 - Standardize offline/mock banners
- Priority: P2
- Area: Visual system
- Problem: Offline/mock state is represented differently or not at all across pages.
- Evidence: Home has a banner; Graph has an inline warning; Ask and Statutes can silently render fallback data.
- Expected behavior: State badges and banners are consistent and do not overwhelm primary work.
- Implementation notes: Build shared components for page-level and panel-level data state.
- Acceptance checks: Live, mock, demo, and offline states use consistent visual language.
- Dependencies: `DATA-002`.
- Status: Todo

## UX-005 - Add keyboard navigation pass for core routes
- Priority: P2
- Area: Accessibility
- Problem: Dense custom controls need keyboard verification.
- Evidence: Search filters, graph controls, statute tabs, complaint stepper, and fact-check findings all use custom interactive UI.
- Expected behavior: Users can operate core workflows with keyboard and visible focus.
- Implementation notes: Test Tab, Shift+Tab, Enter, Space, Escape, arrow keys where applicable.
- Acceptance checks: Keyboard can complete route smoke workflows without traps or invisible focus.
- Dependencies: `QUAL-001`.
- Status: Todo

## UX-006 - Improve form semantics and labels
- Priority: P2
- Area: Accessibility
- Problem: Some icon-only or custom controls may lack strong accessible names or form semantics.
- Evidence: Graph toolbar, theme toggle, search filters, upload buttons, and clickable cards need review.
- Expected behavior: Inputs, buttons, tabs, filters, and cards have clear accessible names and states.
- Implementation notes: Prefer semantic buttons/links, add `aria-label` where icon-only, and avoid clickable non-interactive elements.
- Acceptance checks: Automated accessibility checks pass for core routes with no critical violations.
- Dependencies: `STAB-002`, `QUAL-001`.
- Status: Todo

## UX-007 - Standardize page density and typography
- Priority: P3
- Area: Visual system
- Problem: Some pages use large hero/marketing patterns while others use dense operational panels.
- Evidence: Home and Matters have hero-like sections; Search/Graph/Statutes are dense work surfaces.
- Expected behavior: Operational tools prioritize scanning, repeated action, and consistent type scale.
- Implementation notes: Keep home brand signal, but align spacing, typography, and button treatment with the product shell.
- Acceptance checks: Core routes feel like one app family and avoid oversized content inside compact panels.
- Dependencies: `UX-001`, `UX-002`, `UX-003`.
- Status: Todo

## UX-008 - Replace hard-coded home palette with design tokens
- Priority: P3
- Area: Theme
- Problem: Home uses hard-coded zinc/indigo colors.
- Evidence: `app/page.tsx` uses `bg-zinc-950 text-zinc-100`; home components use zinc/indigo utility classes.
- Expected behavior: Home respects app design tokens and theme toggle.
- Implementation notes: Convert colors to semantic tokens after layout stabilization.
- Acceptance checks: Home remains polished in dark and light modes.
- Dependencies: None.
- Status: Todo

## UX-009 - Add consistent empty states
- Priority: P2
- Area: UX states
- Problem: Empty states vary and sometimes hide whether the cause is no data, no query, no API, or no matching filter.
- Evidence: Search empty state, matter empty helper functions, sources no match state, and graph fallback all differ.
- Expected behavior: Empty states explain what happened and offer the next useful action.
- Implementation notes: Create shared empty-state patterns for no query, no result, no data, filtered out, unavailable, and demo.
- Acceptance checks: Core pages show distinct empty states for each cause.
- Dependencies: `DATA-001`.
- Status: Todo

## UX-010 - Add mobile navigation pattern
- Priority: P2
- Area: Navigation
- Problem: Top nav and left rail are desktop-oriented.
- Evidence: Header contains multiple top-level links and status text; left rail is fixed width.
- Expected behavior: Mobile view exposes primary routes and corpus/matter navigation without squeezing content.
- Implementation notes: Add menu/drawer or collapsible rail for mobile. Keep route smoke tests for mobile viewport.
- Acceptance checks: Mobile smoke screenshots show reachable nav and readable content.
- Dependencies: `NAV-006`.
- Status: Todo

## UX-011 - Audit click targets and dense controls
- Priority: P3
- Area: Interaction quality
- Problem: Many controls use very small text and compact click targets.
- Evidence: The app uses many `text-[10px]`, `text-[11px]`, compact icon buttons, chips, and filters.
- Expected behavior: Click targets are usable without sacrificing operational density.
- Implementation notes: Apply minimum target sizing where controls are primary actions; keep metadata labels compact.
- Acceptance checks: Primary controls meet practical target size on touch viewports.
- Dependencies: `UX-010`.
- Status: Todo

## UX-012 - Add visual acceptance screenshots to each UX task
- Priority: P3
- Area: Visual QA
- Problem: Layout tasks need objective closure evidence.
- Evidence: The audit relied on manual browser screenshots.
- Expected behavior: UX tasks include before/after screenshots for relevant widths.
- Implementation notes: Store generated screenshots as CI artifacts or documented local output, not necessarily committed.
- Acceptance checks: UX PRs include desktop, tablet, and mobile screenshots for changed routes.
- Dependencies: `QUAL-002`.
- Status: Todo

