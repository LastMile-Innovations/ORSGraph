# ORSGraph Frontend Backlog

This backlog turns the frontend audit into an ordered implementation queue. It is intentionally stabilization-first: fix trust, routing, build gates, and visible broken flows before expanding product surface.

## How to use this backlog

Work the queue in order unless a dependency blocks progress. When a task is started, update `Status` from `Todo` to `In progress`; when accepted, update it to `Done` and add the verification notes in the task.

## Priority legend

- `P0`: Blocks core navigation, build trust, or the ability to know whether the frontend is correct.
- `P1`: Breaks flagship workflows or hides real API/product state.
- `P2`: Important product completeness, UX, accessibility, or workflow quality.
- `P3`: Polish, optimization, telemetry, and future hardening.

## Status legend

- `Todo`: Not started.
- `In progress`: Actively being implemented.
- `Blocked`: Waiting on a dependency or decision.
- `Done`: Implemented and verified.
- `Deferred`: Intentionally postponed.

## Task schema

Each backlog item uses this structure:

```md
## ID - Title
- Priority:
- Area:
- Problem:
- Evidence:
- Expected behavior:
- Implementation notes:
- Acceptance checks:
- Dependencies:
- Status:
```

## Backlog files

- [00-audit-findings.md](00-audit-findings.md): Raw audit inventory grouped by issue type.
- [01-stabilization.md](01-stabilization.md): Build, type, lint, routing, and broken-link stabilization.
- [02-navigation-and-pages.md](02-navigation-and-pages.md): Missing pages, shell/nav gaps, loading/error/not-found states.
- [03-data-and-api-integration.md](03-data-and-api-integration.md): Mock data removal, API contracts, offline/fallback policy.
- [04-core-workflows.md](04-core-workflows.md): Search, Ask, Graph, Statutes, Sources, QC, Complaint, Fact Check, CaseBuilder.
- [05-ux-responsive-accessibility.md](05-ux-responsive-accessibility.md): Visual, responsive, keyboard, and accessibility backlog.
- [06-performance-and-quality.md](06-performance-and-quality.md): Performance, smoke tests, visual regression, observability.

## Ordered execution queue

1. `STAB-001`: Restore TypeScript validation. Done.
2. `STAB-002`: Repair lint tooling. Done.
3. `STAB-003`: Remove or gate `ignoreBuildErrors`. Done.
4. `STAB-004`: Fix CaseBuilder matter detail routing. Done.
5. `STAB-005`: Repair matter link generation. Done.
6. `NAV-001`: Add `/matters/[id]/authorities`. Done.
7. `NAV-002`: Add `/matters/[id]/tasks`. Done.
8. `STAB-006`: Add broken-route smoke coverage. Done.
9. `DATA-001`: Define API fallback policy. In progress.
10. `DATA-002`: Make offline/mock state explicit across pages. In progress.
11. `FLOW-001`: Complete Search empty, loading, error, and API-unavailable states.
12. `FLOW-002`: Complete Ask API state and answer provenance behavior.
13. `FLOW-006`: Convert New Matter from demo link to real or clearly gated flow.
14. `UX-001`: Fix Search responsive layout.
15. `UX-002`: Fix Statute detail responsive layout.
16. `UX-003`: Fix Graph responsive layout and canvas framing.
17. `NAV-003`: Add route-level loading and error states. Done for current core dynamic routes.
18. `QUAL-001`: Add frontend smoke/e2e tests. In progress with route smoke.
19. `QUAL-002`: Add visual regression capture for core routes.
20. Continue through the epic files in priority order.

## Definition of done

A task is done when the implementation passes its acceptance checks, does not regress the core route smoke list, and leaves the visible UI in a user-comprehensible state when the backend is offline.

Core route smoke list:

- `/`
- `/search`
- `/ask`
- `/graph`
- `/qc`
- `/statutes`
- `/statutes/or:ors:3.130`
- `/matters`
- `/matters/smith-abc`
- All visible matter sidebar links for the seeded matter

## Latest verification

- `pnpm run check` passes from `frontend/`.
- `pnpm run smoke:routes` passes against `http://localhost:3000`.
- Matter URLs are canonicalized without the internal prefix, for example `/matters/smith-abc/tasks`; legacy `/matters/matter:smith-abc` URLs remain accepted.
