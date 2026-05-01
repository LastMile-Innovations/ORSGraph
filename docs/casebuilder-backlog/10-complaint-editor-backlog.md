# 10 - Complaint Editor Backlog

The Complaint Editor is the pleading-drafting layer inside CaseBuilder. It should feel like a structured legal workbench: complaint outline, numbered allegations, claim/count builder, evidence support, ORSGraph authority links, rule-aware QC, court-paper preview, and export pipeline.

This backlog is the source of truth for the full Complaint Editor program. Existing V0/V0.1/V0.2/V1 backlog items remain shared foundations and dependencies, but the complaint-specific AST, routes, editor, Oregon rule pack, layout, export, AI drafting, and filing packet work live here.

## Product guardrails

- First buildable slice is structured editor first: AST, routes, caption/outline, paragraphs, numbering, save/load, and basic QC.
- The default user journey should be guided and forgiving: start from matter data, show the next useful action, and let advanced users jump directly into editor, claims, evidence, QC, preview, or export.
- Do not make export-first or AI-first choices until the canonical complaint AST is stable.
- The generic `Draft` model remains usable for existing non-complaint drafts; complaint work now routes through the structured Complaint Editor instead of `draft_type=complaint`.
- Editor preview and export must use the same canonical complaint AST and layout logic.
- The product must always label generated text, checks, previews, and exports as review-needed. It must not promise legal advice, guaranteed compliance, or filing readiness.
- Add Tiptap/ProseMirror dependencies only when the shared work-product editor starts; until then use structured AST DTOs and non-rich editor shells.

## 2026-05-01 implementation update

- Complaint work now uses a dedicated structured Complaint Editor instead of generic `Draft` complaint scaffolds.
- Backend complaint DTOs, routes, Neo4j constraints/indexes, graph edge materialization, deterministic QC, preview, export artifacts, AI template states, history, filing packet state, and no-seed defaults are implemented provider-free.
- Frontend CaseBuilder has a complaint workspace route family and three-pane editor workbench for editor, outline, counts, support, QC, preview, and export.
- Generic drafts remain for non-complaint work products; `draft_type=complaint` is intentionally rejected by the backend so complaint work routes through `/complaints`.
- Discovered follow-up: the rich editor should become a shared work-product editor for motions, answers, declarations, briefs, and future products, tracked as `CB-CE-028`.

## UX, flow, and integration principles

- Use a guided "build the complaint" path for first-time or incomplete matters, with clear progress through caption, parties, jurisdiction/venue, facts, counts, relief, QC, preview, and export.
- Keep the three-pane workbench for focused editing, but make the right inspector context-aware so it follows the selected paragraph, count, citation, fact, evidence chip, or rule finding.
- Every warning should have a next action: link evidence, add authority, split paragraph, complete caption, add relief, open source, resolve finding, or create task.
- Preserve user orientation when moving across CaseBuilder: returning from facts, evidence, claims, documents, authorities, QC, or export should land back on the same complaint target when possible.
- Optimize for progressive disclosure. Show essentials first, keep advanced rule/export/AI controls one click away, and avoid overwhelming new users with every panel at once.
- Design for keyboard and scanning: command palette, quick insert actions, stable paragraph numbers, predictable tabs, dense but readable panels, and no layout jumps when warnings or chips appear.

## First implementation sequence

1. Backlog integration and scope cleanup.
2. Complaint AST, DTO registry, graph constraints, and route contracts.
3. Complaint workspace routes, guided setup, and three-pane shell.
4. Caption/outline/count/paragraph builders with renumbering, save/load, progress, and next actions.
5. Fact, evidence, exhibit, and ORSGraph authority linking with deep links back to the selected complaint target.
6. Oregon Circuit Civil Complaint rule pack and complaint QC with actionable remediation flows.
7. Court-paper layout, preview, export artifacts, DOCX/PDF, AI commands, history, filing packet work, accessibility, and integration smoke coverage.

## CB-CE-000 - Backlog integration and scope cleanup
- Priority: P0
- Area: Planning/backlog
- Problem: Complaint Editor work is currently scattered across generic drafting, complaint builder, QC, export, court-rule, and exhibit backlog items.
- Expected behavior: This file owns the full Complaint Editor program, and the existing phase files cross-link to the relevant `CB-CE-*` tasks without duplicating scope.
- Implementation notes: Keep `CB-V0-013` as the complaint entry point; keep V0.1 QC/finding items and V0.2 export/exhibit items as shared dependencies.
- Acceptance checks: README, current status, V0, V0.1, V0.2, V1, feature inventory, and cross-cutting backlog references point to this file where complaint-specific work belongs.
- Dependencies: None.
- Status: Done

## CB-CE-001 - Complaint AST and DTO registry
- Priority: P0
- Area: Data model/API
- Problem: A complaint cannot be managed as one raw text blob or only as generic draft sections.
- Expected behavior: Backend and frontend DTO registries include `ComplaintDraft`, `ComplaintSection`, `ComplaintCount`, `PleadingParagraph`, `PleadingSentence`, `CitationUse`, `EvidenceUse`, `ExhibitReference`, `ReliefRequest`, `SignatureBlock`, `CertificateOfService`, `FormattingProfile`, `RulePack`, `RuleCheckFinding`, and `ExportArtifact`.
- Implementation notes: Store complaint objects alongside existing `Draft` records; include `matter_id` ownership and stable IDs for every node that can receive support links, citations, comments, checks, or export anchors.
- Acceptance checks: DTO contract tests prove serialization, frontend normalization, matter ownership fields, and stable IDs for all complaint AST nodes.
- Dependencies: `CB-X-001`, `CB-X-013`, current Draft/Fact/Evidence/Claim/Authority models.
- Status: Done

## CB-CE-002 - Complaint API contracts and route registration
- Priority: P0
- Area: Backend/API
- Problem: Complaint Editor needs dedicated routes instead of overloading generic draft endpoints.
- Expected behavior: Add route contracts under `/api/v1/matters/:matterId/complaints/*` for complaint CRUD, sections, paragraphs, counts, links, checks, preview, export, and downloads.
- Implementation notes: Preserve existing `/drafts/*` routes. Initial handlers may return structured stub or deferred responses only where a downstream service is intentionally not ready, but route names and request/response DTOs must be stable.
- Acceptance checks: Route contract tests cover all complaint routes and fail if a route is removed or renamed.
- Dependencies: `CB-CE-001`, `CB-X-014`, matter isolation tests.
- Status: Done

## CB-CE-003 - Complaint workspace routes and navigation
- Priority: P0
- Area: Frontend/routing
- Problem: The current complaint page is a workflow hub, not a route family for a full editor.
- Expected behavior: Add canonical routes for `/casebuilder/matters/:matterId/complaint`, `/editor`, `/outline`, `/claims`, `/evidence`, `/qc`, `/preview`, and `/export`, with query/hash support for returning to selected sections, counts, paragraphs, findings, evidence links, and citations.
- Implementation notes: Keep legacy/canonical route helpers consistent and preserve the existing complaint hub as the entry point into the new workspace. Route transitions should preserve complaint context instead of dropping the user at generic top-level pages.
- Acceptance checks: Route smoke checks cover every complaint route with live, demo, and error data states, plus deep-link round trips from complaint to facts/evidence/authority/QC and back.
- Dependencies: `CB-V0-013`, `CB-CE-002`.
- Status: Done

## CB-CE-004 - Three-pane editor shell
- Priority: P0
- Area: Frontend/editor
- Problem: Complaint work needs a dense legal workbench with outline, editor, and inspector in one place.
- Expected behavior: Build a three-pane shell: left complaint outline and progress, center structured pleading editor or court-paper preview, right context-aware inspector tabs for support, authority, rules, formatting, citations, exhibits, AI, and history.
- Implementation notes: Start without rich-text dependencies; use structured AST fixtures and existing CaseBuilder design patterns. The inspector should default to the most useful tab for the current selection rather than requiring the user to hunt for remediation controls.
- Acceptance checks: Shell renders empty, demo, and live complaint states without hidden mock behavior, layout overlap, panel confusion, or lost selection after route refresh.
- Dependencies: `CB-CE-001`, `CB-CE-003`.
- Status: Done

## CB-CE-005 - Caption, court, party, jurisdiction, and venue builders
- Priority: P1
- Area: Complaint structure
- Problem: Caption and party/court metadata are common pleading failure points.
- Expected behavior: Users can enter court name, county, parties, case number, document title, jury demand flag, jurisdiction, venue, and signature/contact fields from structured forms with defaults pulled from the matter graph.
- Implementation notes: Make the form wizard-like for new complaints but editable inline later. Pull default parties and matter court metadata from the matter graph; validate Oregon caption/title requirements in the rule engine when the Oregon profile is selected.
- Acceptance checks: Caption data saves, reloads, appears in preview/export AST, produces QC findings when required fields are missing, and lets users fix missing caption data directly from the finding.
- Dependencies: `CB-V0-009`, `CB-CE-010`.
- Status: Done

## CB-CE-006 - Structured sections, counts, paragraphs, and renumbering
- Priority: P0
- Area: Complaint AST/editor
- Problem: Pleadings need stable numbered paragraphs and separately stated counts, not free-form text.
- Expected behavior: Users can create, edit, reorder, lock, and renumber complaint sections, counts, and pleading paragraphs from either guided outline actions or direct editor actions.
- Implementation notes: Paragraphs include number, section/count ownership, text, sentence children, facts, evidence, citations, exhibits, rule findings, lock state, and review status.
- Acceptance checks: Unit tests cover consecutive numbering, duplicate numbers, skipped numbers, valid incorporation ranges, stable IDs after renumbering, and no confusing visual jumps when paragraph numbers update.
- Dependencies: `CB-CE-001`, `CB-V0-026`.
- Status: Done

## CB-CE-007 - Claim/count element mapper integration
- Priority: P1
- Area: Claims/counts
- Problem: Complaint counts need to map claims to elements, facts, evidence, authority, remedies, and weaknesses.
- Expected behavior: Each count can reference a claim, create custom claim text, show required elements, map facts/evidence/authority to each element, and expose a count health status.
- Implementation notes: Reuse current claim/element/evidence synchronization before adding richer claim templates.
- Acceptance checks: Count health flags missing element support, missing authority, weak evidence, unsupported remedy, and possible deadline issue.
- Dependencies: `CB-V0-010`, `CB-V0-024`, `CB-V02-009`.
- Status: Done

## CB-CE-008 - Fact/evidence/exhibit support linking
- Priority: P1
- Area: Support graph
- Problem: Every factual allegation should be supportable by case files or visibly flagged.
- Expected behavior: Paragraphs and sentences can link to facts, evidence, document spans, and exhibit references with support relationship types: supports, partially supports, contradicts, context only, impeaches, and authenticates.
- Implementation notes: Reuse `SourceSpan` and evidence records; add exhibit references without silently changing stable exhibit labels.
- Acceptance checks: Unsupported factual paragraphs generate findings, support chips open source context, and exhibit references warn when the exhibit is missing or unattached.
- Dependencies: `CB-V0-011`, `CB-V0-020`, `CB-V02-004`, `CB-V02-012`.
- Status: Done

## CB-CE-009 - ORSGraph authority and citation linking
- Priority: P1
- Area: Authority/citations
- Problem: Legal citations and claim authority need durable ORSGraph links, resolution status, currentness, and scope warnings.
- Expected behavior: Users can search/recommend authority from selected text, insert citation uses, resolve citations, link citations to ORSGraph nodes, and see status: resolved, unresolved, ambiguous, stale, or needs review.
- Implementation notes: Reuse current authority search/attach endpoints first; extend targets from claim/element/draft paragraph to complaint count, paragraph, sentence, and citation use.
- Acceptance checks: Citation checks flag unresolved citations, wrong pinpoints, stale authority, quote mismatches, and definition-scope issues where ORSGraph data supports them.
- Dependencies: `CB-V0-012`, `CB-V0-025`, `CB-V02-008`, `CB-X-006`.
- Status: Done

## CB-CE-010 - Oregon ORCP/UTCR versioned rule pack
- Priority: P0
- Area: Rules/compliance
- Problem: Oregon civil complaints need source-backed ORCP and UTCR checks, and rules change over time.
- Expected behavior: Add a versioned `Oregon Circuit Civil Complaint - ORCP + UTCR` rule pack covering ORCP 16, ORCP 18, and UTCR 2.010 checks.
- Implementation notes: Store source citation, source URL, effective date, severity, target type, message, explanation, suggested fix, and auto-fix availability for each rule.
- Acceptance checks: Rule-pack tests cover caption, all parties in complaint title, separate counts, numbered paragraphs, plain/concise ultimate facts, demand for relief, signature/contact fields, double spacing, numbered lines, first-page two-inch blank top area, and one-inch side margins.
- Dependencies: `CB-CE-001`, Oregon court-rule source capture.
- Status: Done

## CB-CE-011 - Complaint QC dashboard and finding lifecycle
- Priority: P0
- Area: QC/risk
- Problem: Complaint-specific checks need one dashboard and durable status transitions.
- Expected behavior: QC categories include structure, rules, formatting, facts, evidence, authority, citations, claims/elements, relief, deadlines, exhibits, and export, with a clear "what to fix next" ordering.
- Implementation notes: Reuse shared `QcRun` and finding lifecycle work; complaint findings must target draft, section, count, paragraph, sentence, citation, exhibit, or export artifact. Each finding should expose a primary remediation action and preserve the user's editor context after the fix.
- Acceptance checks: Findings can be opened from the editor, resolved, ignored, reopened, linked to remediation tasks, and reflected in dashboard counts; clicking a finding lands on the exact target and shows the relevant inspector tab.
- Dependencies: `CB-V01-009`, `CB-V01-015`, `CB-V01-016`, `CB-CE-010`.
- Status: Done

## CB-CE-012 - Tiptap/ProseMirror editor integration
- Priority: P1
- Area: Frontend/editor
- Problem: The structured complaint editor needs stable custom nodes and marks once the AST is proven.
- Expected behavior: Add rich-editor custom nodes/marks for `pleading_section`, `pleading_count`, `pleading_paragraph`, `citation_mark`, `evidence_mark`, `exhibit_mark`, `fact_chip`, `authority_chip`, and `qc_warning_mark` when the shared work-product editor starts.
- Implementation notes: Do not add dependencies before the shared editor abstraction. The current complaint editor intentionally uses structured AST fields and direct save actions so complaint work no longer depends on generic drafts.
- Acceptance checks: Shared editor tests prove load, edit, save, reload, chips, marks, paragraph locks, and QC warning anchors survive a round trip across complaint and later motion/answer profiles.
- Dependencies: `CB-CE-006`, `CB-CE-008`, `CB-CE-009`.
- Completed: Deferred/superseded for the complaint slice by the structured AST workbench and new shared editor epic.
- Verification: No Tiptap/ProseMirror dependencies were added; `pnpm run build` passes.
- Status: Deferred

## CB-CE-013 - Court-paper layout engine and preview
- Priority: P1
- Area: Formatting/preview
- Problem: The editor and final export must not disagree about layout.
- Expected behavior: Build a shared layout path from complaint AST to court-paper preview with page breaks, line numbers, caption block, first-page blank area, margins, double spacing, signature block, certificate, and exhibits.
- Implementation notes: Start HTML/CSS court-paper renderer; keep a later path open for a dedicated layout engine or Typst-like renderer.
- Acceptance checks: Preview tests catch missing line numbers, margin drift, text overlap, signature split issues, and unsupported page overflow states.
- Dependencies: `CB-CE-001`, `CB-CE-010`.
- Status: Done

## CB-CE-014 - Export artifact model and export endpoint
- Priority: P1
- Area: Export/API
- Problem: Complaint exports need durable artifacts, status, warnings, and download URLs.
- Expected behavior: Add `POST /api/v1/matters/:matterId/complaints/:complaintId/export` accepting format, profile, mode, includeExhibits, and includeQcReport, returning artifact ID, download URL, format, page count, and generated timestamp.
- Implementation notes: Reuse the V0.2 `ExportPackage` direction where possible; generated artifacts must be matter-scoped and covered by storage lifecycle policy.
- Acceptance checks: Contract tests cover request/response shape, unsupported format errors, artifact ownership, and download status.
- Dependencies: `CB-V02-011`, `CB-X-017`, `CB-CE-013`.
- Status: Done

## CB-CE-015 - DOCX/PDF/HTML/Markdown/plain text/JSON export
- Priority: P1
- Area: Export/rendering
- Problem: Users need court-ready, editable, review, and internal support-linked export modes.
- Expected behavior: Export formats include PDF, DOCX, HTML, Markdown, plain text, and JSON complaint AST. Export profiles include clean filing copy, editable DOCX, review PDF with annotations, evidence-linked internal copy, citations report, and rule checklist.
- Implementation notes: PDF/DOCX must preserve structured numbering, citation labels, exhibit references, caption, and rule-sensitive formatting.
- Acceptance checks: Export tests verify AST-to-HTML, AST-to-DOCX, AST-to-PDF skeletons, page count metadata, and readable downloads.
- Dependencies: `CB-V02-006`, `CB-V02-007`, `CB-V02-013`, `CB-CE-014`.
- Status: Done

## CB-CE-016 - AI complaint drafting commands with source-backed output
- Priority: P2
- Area: AI drafting
- Problem: AI drafting must help without silently adding unsupported facts or legal claims.
- Expected behavior: Add commands for drafting factual background, drafting a count, rewriting as ultimate facts, splitting paragraphs, making text concise, finding missing evidence/authority, generating prayer, exhibit list, certificate, fact-checking, and citation-checking.
- Implementation notes: Every AI response must return draft text, facts used, evidence used, authorities used, warnings, assumptions, and human-review items. Unsupported proposals must be marked unsupported draft/needs evidence.
- Acceptance checks: With no provider configured, commands show disabled/template mode; with provider enabled, source-backed output schema rejects or flags unsupported assertions.
- Dependencies: `CB-X-004`, `CB-X-005`, `CB-CE-008`, `CB-CE-009`, `CB-CE-011`.
- Status: Done

## CB-CE-017 - Version history and material edit events
- Priority: P2
- Area: History/audit
- Problem: Material pleading edits must be traceable.
- Expected behavior: Record draft created, section generated, paragraph edited, fact linked, citation inserted, QC resolved, and export generated events.
- Implementation notes: Complaint-specific history is superseded by shared Case History. Keep complaint routes as facades, but all durable history must route through canonical work-product `ChangeSet`, `VersionChange`, and `VersionSnapshot` records.
- Acceptance checks: Text, support, citation, rules, AI, restore, and export changes are visible from Case History records, not from stored complaint-only event truth.
- Dependencies: `CB-CH-105`, `CB-CH-402`, `CB-CH-503`, `CB-CH-1104`.
- Status: Partial
- Progress: Case History V0 now records complaint-profile work-product create/edit/support/QC/AI/export/restore events and exposes timeline/compare/restore in the Complaint workspace. Rich support/QC diff, scoped restore, and smoke coverage remain in `11-case-history-version-control.md`.

## CB-CE-018 - Filing packet and exhibit packet builder
- Priority: P2
- Area: Filing packet
- Problem: Complaint filings often need related exhibits, declarations, certificates, cover sheets, and checklists.
- Expected behavior: Build a packet preview with complaint PDF, exhibits, declarations, certificate of service, QC report, filing checklist, and later summons/civil cover sheet/proposed order support.
- Implementation notes: Do not implement e-filing. Distinguish generated packet, review-needed packet, and user-final packet.
- Acceptance checks: Packet preview shows included files, order, warnings, missing items, stable exhibit labels, and downloadable artifacts when generation succeeds.
- Dependencies: `CB-V02-004`, `CB-V02-005`, `CB-V02-012`, `CB-V1-008`, `CB-V1-013`.
- Status: Done

## CB-CE-019 - Safety, no-legal-advice, and review-needed UX
- Priority: P0
- Area: Safety/trust
- Problem: The editor must not imply that generated text, QC, or export is legal advice or guaranteed court compliance.
- Expected behavior: Complaint decision points clearly label source-backed checks, unsupported allegations, unresolved citations, rule warnings, export status, and human-review needs.
- Implementation notes: Use persistent trust affordances in the top bar, QC dashboard, AI panel, preview, and export screens without burying users in repetitive disclaimers.
- Acceptance checks: Safety review confirms no page says or implies legal advice, guaranteed compliance, filing-ready status, or silent AI support.
- Dependencies: `CB-V0-017`, `CB-X-011`.
- Status: Done

## CB-CE-020 - Guided setup and progressive workflow
- Priority: P0
- Area: UX/flow
- Problem: A full complaint editor can overwhelm users if it opens directly into a blank legal workbench.
- Expected behavior: New complaints start with a guided setup flow that imports matter data, confirms caption/parties, chooses jurisdiction/profile, identifies selected claims, and creates a usable outline before opening the full editor.
- Implementation notes: Let experienced users skip to editor. Persist setup progress so users can leave and return without losing state. The top bar should always show current stage, save state, QC state, preview, and export availability.
- Acceptance checks: Empty matters, partially built matters, and existing complaint drafts each open to the most helpful next screen; users can complete setup without reading documentation.
- Dependencies: `CB-CE-003`, `CB-CE-004`, `CB-CE-005`, `CB-CE-006`, `CB-CE-007`.
- Status: Done

## CB-CE-021 - Smart next actions and remediation flow
- Priority: P0
- Area: UX/QC
- Problem: Warnings are only useful if users know what to do next.
- Expected behavior: The editor, outline, inspector, and QC dashboard show prioritized next actions such as add missing caption data, link evidence, add authority, split paragraph, create count, add relief, attach exhibit, run QC, preview, or export.
- Implementation notes: Derive next actions from complaint AST state, rule findings, support gaps, citation status, export readiness, and matter graph availability. Avoid noisy lists; show the top few actions and let users open the full queue when needed.
- Acceptance checks: Every blocking or serious finding has a primary remediation action, and completing the action updates status without requiring a page refresh.
- Dependencies: `CB-CE-011`, `CB-V01-010`, `CB-V01-015`, `CB-V01-016`.
- Status: Done

## CB-CE-022 - Cross-workbench integration and return paths
- Priority: P0
- Area: Integration/navigation
- Problem: Complaint work depends on facts, evidence, documents, claims, authority, QC, and export, but users should not lose their place when moving between tools.
- Expected behavior: From any paragraph, count, support chip, citation, exhibit reference, or finding, users can jump to the relevant CaseBuilder surface and return to the same complaint target.
- Implementation notes: Use stable route params, hashes, and return targets. Integrate with facts, documents, evidence matrix, claims builder, authority search, QC dashboard, tasks, graph, preview, and export without duplicating each surface inside the editor.
- Acceptance checks: Deep-link tests prove round trips from complaint paragraph to document span, fact, evidence, claim element, authority result, QC finding, task, preview page, and export panel preserve context.
- Dependencies: `CB-CE-003`, `CB-CE-008`, `CB-CE-009`, `CB-CE-011`, `CB-V01-008`.
- Status: Done

## CB-CE-023 - Context-aware inspector and quick insert controls
- Priority: P1
- Area: UX/editor
- Problem: Users need common actions close to the selected text instead of spread across distant pages.
- Expected behavior: Selecting a section, count, paragraph, sentence, citation, evidence chip, exhibit chip, or warning updates the inspector with relevant details and quick actions.
- Implementation notes: Quick actions should include add fact, link evidence, search authority, insert citation, create exhibit reference, split paragraph, mark reviewed, open source, and create task. Use familiar icons and tooltips for compact controls.
- Acceptance checks: Inspector selection tests cover each target type, no action appears for an invalid target, and quick actions update the canonical AST or linked graph state.
- Dependencies: `CB-CE-004`, `CB-CE-006`, `CB-CE-008`, `CB-CE-009`, `CB-CE-012`.
- Status: Done

## CB-CE-024 - Command palette, keyboard flow, and quick add
- Priority: P2
- Area: UX/productivity
- Problem: Power users need to draft and remediate without constant mouse travel.
- Expected behavior: Add a complaint command palette and keyboard flow for add paragraph, add count, link fact, link evidence, search authority, run QC, preview, export, jump to section, and open selected source.
- Implementation notes: Keep commands discoverable from a visible button and optional keyboard shortcut. Commands must respect disabled/provider states and explain why unavailable actions are disabled.
- Acceptance checks: Keyboard smoke tests cover navigation, command execution, focus return, escape behavior, and screen-reader labels for command results.
- Dependencies: `CB-CE-004`, `CB-CE-012`, `CB-X-007`.
- Status: Done

## CB-CE-025 - Responsive, accessibility, and dense-workbench polish
- Priority: P1
- Area: UX/accessibility
- Problem: The three-pane editor can become unusable on smaller screens or for keyboard/screen-reader users.
- Expected behavior: Desktop shows the full three-pane workbench; tablet and mobile collapse outline and inspector into accessible drawers/tabs without hiding critical warnings or save/export state.
- Implementation notes: Maintain stable dimensions for toolbar controls, paragraph numbers, chips, counters, and inspector tabs. Avoid nested cards and layout jumps. Keep text readable inside compact controls.
- Acceptance checks: Screenshot and keyboard passes cover desktop, laptop, tablet, and mobile widths; axe-style checks catch unlabeled controls, unreachable tabs, and focus traps.
- Dependencies: `CB-CE-004`, `CB-X-007`, `CB-X-008`.
- Status: Done

## CB-CE-026 - Integration smoke and end-to-end user flow tests
- Priority: P0
- Area: Quality/integration
- Problem: The product promise depends on the whole complaint flow working across CaseBuilder, not isolated components.
- Expected behavior: Add smoke coverage for create complaint, guided setup, add parties/caption, add facts, create count, map element, link evidence, attach authority, run QC, fix one finding, preview, and request export.
- Implementation notes: Keep the smoke provider-free and deterministic. Use explicit deferred/export-not-ready expectations until real rendering is implemented.
- Acceptance checks: Smoke fails on broken route handoffs, lost return context, missing support links, missing authority links, stale QC state, preview/export mismatch, or cross-matter access.
- Dependencies: `CB-CE-001` through `CB-CE-025`, `CB-X-014`, `CB-X-015`.
- Status: Done

## CB-CE-027 - Contract, unit, route, smoke, visual, and export tests
- Priority: P0
- Area: Quality
- Problem: Complaint Editor spans many systems and needs regression coverage before users rely on it.
- Expected behavior: Add backend DTO/route/matter-isolation tests, rule-engine unit tests, frontend route/type tests, editor save/reload tests, smoke checks, visual preview checks, and export artifact tests.
- Implementation notes: Keep tests deterministic and provider-free by default; provider-backed AI tests must be opt-in.
- Acceptance checks: CI/dev checks fail on broken complaint routes, DTO drift, numbering bugs, rule-pack regressions, broken support links, preview/export drift, or unsafe cross-matter access.
- Dependencies: `CB-X-001`, `CB-X-002`, `CB-X-014`, `CB-CE-001` through `CB-CE-026`.
- Status: Done

## CB-CE-028 - Shared work-product editor for motions, answers, declarations, briefs, and future products
- Priority: P0
- Area: Editor/platform
- Problem: Complaint work should not be a one-off editor while motions, answers, declarations, briefs, notices, and exhibit lists remain generic draft blobs.
- Expected behavior: Promote the structured editor architecture into a shared work-product editor with product-specific AST profiles, route families, rule packs, support links, QC, preview, export, history, and AI template states.
- Implementation notes: Complaint Editor remains the first concrete profile. Do not reintroduce `draft_type=complaint`; migrate future work products away from generic drafts when their AST profile starts. Rich-text/Tiptap work should attach to this shared editor layer instead of only to complaints.
- Acceptance checks: Motions/answers/declarations can use the same editor shell, graph support links, findings lifecycle, preview/export path, and no-seed initialization pattern without duplicating complaint-specific code.
- Dependencies: `CB-CE-001` through `CB-CE-027`.
- Status: Todo
