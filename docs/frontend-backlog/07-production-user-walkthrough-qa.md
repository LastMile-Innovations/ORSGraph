# Production User Walkthrough QA

Target: `https://frontend-production-090c.up.railway.app/dashboard`

Purpose: hands-on production walkthrough from a normal user perspective, tracking bugs, UX/UI issues, accessibility gaps, copy problems, and improvement opportunities discovered while navigating the app.

## Test Notes

- Browser: Chrome via Computer Use
- Test style: manual exploratory walkthrough
- Data policy: use harmless legal-search queries and read-only navigation; avoid creating, deleting, archiving, ingesting, indexing, or mutating production records
- Logging style: rolling log. Each finding includes what happened, why it matters, and how it can be improved.

## Severity Key

- `P0` Critical trust/data-integrity issue that can mislead users or corrupt confidence in the product.
- `P1` High-impact bug or flow blocker.
- `P2` Medium-impact UX, UI, consistency, or reliability issue.
- `P3` Polish, copy, or discoverability improvement.

## Implementation Progress

- 2026-05-04: Fixed first frontend batch locally.
  - Ask now clears stale answer/source state when a new question starts, binds responses to the latest request, and suppresses bundled fallback answers when the live Ask request times out or fails.
  - Default ORS API timeout increased from 5 seconds to 12 seconds to reduce avoidable broad-search/Ask failures while deeper partial-result fallback is still pending.
  - Matters list and matter detail now render safe fallback text instead of `Invalid Date` for malformed timestamps.
  - Admin beta invite creation now requires a valid email before the primary action is enabled or submitted.
  - Facts search now keeps the detail pane aligned to the filtered list and shows filter counts scoped to the current search query.
  - Global search placeholder no longer renders generated copy such as `Search search...`.
  - Search typeahead closes after submitting and only reopens after the user edits the query.
  - Verification: `pnpm run typecheck` passed in `frontend/`.
- 2026-05-04: Fixed second frontend batch locally.
  - Added shared matter-readiness checks for reviewed facts, pending timeline suggestions, claim elements, authorities, deadline inputs, drafts/work products, and setup gaps.
  - Ask Matter now labels the AI harness as limited beta, disables empty scopes, and generates suggested prompts from the actual matter inventory.
  - Deadlines now disables `Auto-compute` until case number, court, trigger dates/source events, and source material are present.
  - Evidence Matrix now shows a setup state instead of `0/0` metrics when no claim elements exist, and disables gap/export actions until the grid is real.
  - Claims now shows an empty-state workflow and locks the unimplemented AI claim suggestion action behind a beta/disabled state.
  - Matter QC now distinguishes `not checked` from clean zeros and surfaces setup gaps before users rely on the dashboard.
  - Tasks now surfaces recommended setup work from detected matter gaps and uses a rolling 14-day due-soon window instead of a hardcoded date.
  - Drafting and export surfaces now display prerequisite checks and disable high-stakes template/export actions until required matter objects exist.
  - Authorities no longer shows endpoint-contract copy in production and explains claim/element/draft prerequisites before attachment.
  - Matter graph hides parser/embedding modes behind diagnostics and translates internal warnings/relationship labels into user-facing copy.
  - Search retrieval diagnostics and per-result score breakdowns moved behind advanced-details disclosures.
  - Facts now flags likely heading/table/markdown extraction noise before downstream use.
  - Dashboard now separates approved events from timeline suggestions and flags invalid timeline dates instead of silently showing an empty timeline.
  - Verification: `pnpm run typecheck` passed in `frontend/`.
- 2026-05-04: Fixed third frontend batch locally.
  - Destructive matter delete actions are less visually prominent while retaining the typed-name confirmation.
  - New Matter now marks the matter name as required, disables creation until it is present, and clarifies file versus folder upload copy.
  - Upload tray now offers a full dismiss control after all active uploads finish, reducing lower-right UI overlap after completion.
  - Sidebar matter labels strip markdown-style link syntax before rendering.
  - Global shell search now direct-opens exact ORS citations consistently with the dashboard fast path.
  - Document workspace now has a stable minimum viewport height so the document/review panes do not collapse into a blank visible canvas.
  - Statute tabs no longer show route-transition loading labels after data appears, loaded tab counts prefer loaded records over stale summary counts, and definition cards deduplicate repeated records.
  - Official source links are normalized to absolute `https://` URLs and blank source metadata renders as `Not available`.
  - Statute intelligence panel is wider on desktop and stale pending copy now tells users the extracted records need refresh instead of saying to open a tab already active.
  - Statute graph now explains the isolated-node/no-edge state instead of implying the statute has no relationships.
  - Verification: `pnpm run typecheck` passed in `frontend/`.
- 2026-05-04: Fixed fourth frontend batch locally.
  - Admin QC now labels the headline percentage as clean coverage, explains warning-only states, and avoids implying warning-heavy runs are blocking failures.
  - Admin source ingest/combine/index workflows now require browser confirmation before queuing mutating production jobs, with warning copy separating read-only QC from state-changing operations.
  - Global Ask now displays a limited-beta AI harness disclosure and states that failed/timed-out requests do not fall back to canned legal answers.
  - Search now shows a visible warning when semantic/vector retrieval is unavailable for a run.
  - Verification: `pnpm run typecheck` passed in `frontend/`.

## Findings

### P0 - Ask can show a stale, unrelated answer after the request times out

- Area: global Ask page, `/ask`
- Steps: asked a security-deposit/deadline question: `What are the security deposit deadlines under ORS chapter 90?`
- What happened: the UI showed a red timeout banner (`API request timed out after 5s`) but also displayed an answer and source pack for an unrelated district-attorney duties question, citing unrelated statutes.
- Why it matters: this is a legal research trust failure. A user can reasonably believe the answer belongs to the question they just asked, even though it appears to be stale state from another request.
- Better behavior: clear previous answer/source state when a new ask begins, bind rendered answer data to a request ID, and never render an answer if the current request fails or times out. Show only a retryable timeout state with the original question preserved.

### P1 - Broad search queries time out after 5 seconds and return no useful fallback

- Area: Search page, `/search`
- Steps: searched `security deposit deadline` in Auto mode, then tried Keyword mode.
- What happened: both attempts timed out after 5 seconds. The UI said the live API could not be reached and the view was limited to data the API already returned, but no partial results were visible.
- Why it matters: common natural-language searches are a core first-run path. A user who does not already know a citation gets blocked quickly.
- Better behavior: increase timeout or stream staged results, add a visible retry control, preserve mode/query clearly, and show partial keyword/citation fallback results when semantic/hybrid retrieval is slow.

### P1 - Production corpus has 0% embedding coverage in QC

- Area: Admin QC console, `/admin/qc`
- Steps: opened QC Console, then selected `Embedding readiness`.
- What happened: the panel reported `367,507` retrieval chunks with `0.00% embedding coverage`.
- Why it matters: semantic search, hybrid search, and Ask depend on retrieval quality. A public production environment with zero embedding coverage will make natural-language workflows slow, sparse, or misleading.
- Better behavior: treat zero embedding coverage as a blocking release/deployment health check, surface the same degraded state to user-facing Search/Ask pages, and provide an admin runbook/action to rebuild embeddings safely.

### P1 - QC reports 16,476 unresolved citation mentions

- Area: Admin QC console, `/admin/qc`
- Steps: opened QC Console, then selected `Citation resolution`.
- What happened: the panel reported `16,476` unresolved citation QC rows and `78.76%` citation resolution coverage.
- Why it matters: unresolved citation mentions reduce statute/source navigation, weaken the graph, and can make citation-based research appear incomplete even when the text corpus contains the references.
- Better behavior: expose unresolved citation samples with source locations, categorize by parser gap versus missing target, and track citation-resolution coverage as a release metric with a threshold.

### P1 - ORS 90.320 text has linked citations but the citation graph reports zero

- Area: statute detail, `/statutes/or%3Aors%3A90.320?tab=citations`
- Steps: opened ORS 90.320 from the Statutes directory and inspected Text/Citations.
- What happened: the statute body visibly linkified references such as ORS 479.270 and ORS 90.325, but the header, right intelligence panel, and Citations tab all reported `CITES 0` and no outbound citation edges.
- Why it matters: users can see citations in the text but cannot inspect them through the citation tab or graph. This undermines the research trail and likely contributes to the QC unresolved-citation count.
- Better behavior: reconcile inline citation detection with graph-edge creation, add a data check for "linked inline citations but zero citation edges," and expose unresolved inline citations in the Citations tab until graph edges are available.

### P1 - Statute intelligence counts do not match tab contents

- Area: statute detail, `/statutes/ORS%2090.300`
- Steps: opened ORS 90.300, then inspected Definitions, Deadlines, Exceptions, Citations, and Source tabs.
- What happened: summary/tab counts showed extracted data, but the tab bodies contradicted them. Deadlines showed a count of 4 while the Deadlines tab said no deadlines were detected. Exceptions showed a count of 6 while the tab said no exceptions or penalties were detected. Citation counts also shifted after loading.
- Why it matters: these counts guide user trust and navigation. In legal workflows, "there are 4 deadlines" versus "no deadlines detected" changes what a user does next.
- Better behavior: derive badges and tab bodies from the same loaded dataset, distinguish "not loaded yet" from "loaded empty," and include an extraction timestamp/status for each intelligence category.

### P1 - ORS 90.320 Definitions count says 2 but the tab is empty

- Area: statute detail, `/statutes/or%3Aors%3A90.320?tab=definitions`
- Steps: opened ORS 90.320, then selected Definitions.
- What happened: the header, tab label, and side panel all showed `Definitions 2`, but the tab body said `No definitions detected for this statute.` The side panel also continued to say `Open Definitions to load extracted terms` while the Definitions tab was already active.
- Why it matters: this is a second statute-level reproduction of the intelligence-count mismatch, showing it is not isolated to ORS 90.300.
- Better behavior: use a single loaded definitions collection for all counts/body states, clear stale "open this tab" helper text after selection, and add a regression fixture for ORS 90.320.

### P1 - CaseBuilder document detail renders as a blank visible canvas

- Area: CaseBuilder document detail, `/casebuilder/matters/:matterId/documents/:documentId`
- Steps: opened a processed markdown document from the matter files list.
- What happened: the accessibility tree contained the markdown editor, extraction intelligence, semantic units, notes/audit tabs, and actions, but the visible page remained mostly blank except for the header and document title/action bar.
- Why it matters: the route appears broken to sighted users even though data is mounted in the DOM. It prevents review of uploaded matter documents.
- Better behavior: fix the layout/paint issue for the document workspace, add a visible loading/error state while panels hydrate, and add a smoke check that asserts key document content is visible, not only present in the DOM.

### P2 - Definitions extraction shows duplicated/mislabeled cards

- Area: statute detail, Definitions tab, `/statutes/ORS%2090.300`
- Steps: opened the Definitions tab for ORS 90.300.
- What happened: two definition cards showed the same "security deposit" definition. One card heading was just `4`; the other was `ORS 90.300(1)`.
- Why it matters: duplicated definitions and numeric-only headings make the extraction feel unreliable and make it harder to cite or reuse.
- Better behavior: deduplicate by normalized definition text and target citation, use a human-readable term/citation heading, and show the provision anchor that produced the definition.

### P2 - Statute Source tab keeps a loading label after data appears

- Area: statute detail, Source tab, `/statutes/ORS%2090.300`
- Steps: opened Source tab after viewing statute detail.
- What happened: source trail/URL data rendered, but a `LOADING SOURCE DATA` label remained visible. Some metadata fields such as retrieved/hash fields appeared blank.
- Why it matters: users cannot tell whether the source audit data is complete, stale, missing, or still loading.
- Better behavior: remove loading labels once the request settles, display explicit `Not available` values for missing metadata, and show a clear loaded/error timestamp.

### P2 - Official source link appears inconsistently formatted

- Area: statute detail, `/statutes/ORS%2090.300`
- Steps: inspected official source link and Source tab.
- What happened: one official-source link surfaced as `oregonlegislature.gov/bills_laws/ors/ors090.html` without a scheme, while the Source tab displayed `https://www.oregonlegislature.gov/bills_laws/ors/ors090.html`.
- Why it matters: users need source links to be reliable and clearly external. Scheme-less links can behave inconsistently depending on how the href is built.
- Better behavior: normalize official URLs to absolute `https://` URLs everywhere and use consistent external-link affordances.

### P2 - Intelligence side panel is too narrow for deadline/extraction text

- Area: statute detail right-side intelligence panel, `/statutes/ORS%2090.300`
- Steps: opened statute intelligence tabs and reviewed the right-side panel.
- What happened: colored deadline/extraction text wrapped into very narrow one- or two-word lines, making the panel difficult to scan.
- Why it matters: the panel is supposed to summarize legal signals, but the layout turns important text into visual noise.
- Better behavior: widen the panel at desktop breakpoints, collapse dense signals into expandable rows, or move long extracted text into a full-width details area.

### P2 - Matters list and matter detail show `Invalid Date`

- Area: CaseBuilder matters, `/casebuilder` and `/casebuilder/matters/:matterId`
- Steps: opened Matters, then opened an existing matter.
- What happened: matter cards showed `UPDATED INVALID DATE`; the matter detail header showed created/updated `Invalid Date`.
- Why it matters: dates are core matter-management metadata. Invalid Date signals data corruption and makes it impossible to sort or assess recency.
- Better behavior: validate date parsing at the API boundary, render a safe fallback such as `No update timestamp`, and log records with malformed timestamps for cleanup.

### P2 - Destructive matter/file actions are too prominent in routine browsing

- Area: CaseBuilder matters and documents
- Steps: browsed matter cards, matter detail, and matter files.
- What happened: delete/archive-style actions are visually prominent and sit near common safe actions such as Open, Manage, and Index.
- Why it matters: production users can accidentally initiate destructive or workflow-changing actions while browsing.
- Better behavior: move destructive actions behind a secondary menu, require a confirmation dialog with the object name, and visually separate read-only actions from mutating actions.

### P2 - Search results expose developer/debug retrieval details to normal users

- Area: Search page, `/search?mode=citation...`
- Steps: searched `ORS 90.300` in Citation mode.
- What happened: the result worked, but the visible result metadata included dense retrieval/debug tags such as intent, vectors, exact/text/graph/rerank, and timing.
- Why it matters: the extra debugging language competes with the statute result and may confuse non-technical legal users.
- Better behavior: move retrieval diagnostics behind an "Advanced details" disclosure or debug mode while keeping user-facing relevance/source indicators visible.

### P2 - Document extraction suggestions look truncated and hard to verify

- Area: CaseBuilder document detail, accessibility-visible content for markdown graph
- Steps: opened a processed markdown document and inspected extraction intelligence content in the accessibility tree.
- What happened: proposed facts and timeline suggestions appeared as clipped table fragments like `mp4\` | March 13, 2026 | ...` rather than clean fact sentences with full source context.
- Why it matters: users need to verify extracted facts before approving or ignoring them. Truncated table fragments make approval risky.
- Better behavior: render each suggestion as a normalized fact/timeline sentence, include the source row/line as supporting context, and provide a preview of what approving will create.

### P2 - Skip-to-content link can point to the previous route

- Area: global shell / accessibility navigation
- Steps: navigated from New Matter to Admin and inspected the skip link target.
- What happened: on `/admin`, the skip link still pointed at `/casebuilder/new#app-main`.
- Why it matters: keyboard and screen-reader users can be sent to the wrong page/anchor, and it suggests shell-level navigation state is not refreshing cleanly on route changes.
- Better behavior: derive the skip-link `href` from the current route on every navigation, or use a route-independent `#app-main` anchor that stays valid within the current document.

### P2 - QC summary reads contradictory: 0% pass rate with zero failures

- Area: Admin QC console, `/admin/qc`
- Steps: opened QC Console from Admin and waited for the summary to load.
- What happened: the run completed and showed `0.00%` pass rate, `383,983` warnings, `0` failures, and `16,476` unresolved citations. The first panel also showed a clean/pass state.
- Why it matters: the page makes it hard to tell whether the corpus is healthy, degraded, or broken. A 0% pass rate normally implies failure, but the failures card says 0.
- Better behavior: separate pass/fail status from warning coverage, rename the metric to something more precise if warnings intentionally reduce the pass rate, and add a plain-language health summary such as `No blocking failures, warnings require review`.

### P2 - Beta invite creation is a one-click primary action on an empty form

- Area: Admin beta access, `/admin/auth`
- Steps: opened Beta Access and reviewed the create-invite form without submitting.
- What happened: `Create and copy invite` is a prominent enabled primary button while the email field appears empty/placeholder-like.
- Why it matters: invite creation changes access state. A single enabled primary action increases the chance of creating an invite accidentally or with invalid/default data.
- Better behavior: disable the action until a valid email is entered, show required-field validation, add a confirmation step that names the invite recipient, and separate `Create invite` from `Copy link` once creation succeeds.

### P2 - Matter Ask suggested prompts do not adapt to available matter data

- Area: CaseBuilder Ask Matter, `/casebuilder/matters/:matterId/ask`
- Steps: opened Ask Matter for a matter with 1 document, 24 facts, 0 claims, and 0 parties.
- What happened: suggested prompts referenced strongest claims, weakest defenses, uninhabitable-unit evidence, affirmative defenses, rent ledgers, and payment records even though the matter dashboard showed no claims/defenses and the indexed source list showed only one document/chunk.
- Why it matters: suggested prompts shape the user's legal workflow. Generic prompts that imply unavailable data can make users think the app has found claims/evidence that it has not actually extracted.
- Better behavior: generate suggestions from actual matter inventory, hide claim/defense prompts until those objects exist, and label starter examples as examples if they are not grounded in the current matter.

### P1 - AI harness appears unfinished but Ask surfaces look production-ready

- Area: global Ask, CaseBuilder Ask Matter, AI-facing controls
- Steps: reviewed the Ask experiences without submitting additional matter prompts, and incorporated product context that the AI harness is not fully built.
- What happened: the UI presents AI entry points as normal production actions (`Ask`, `Ask Matter`, suggested legal prompts), while there is no visible beta/incomplete-state labeling, capability boundary, or explanation of which data sources/tools the harness can actually use.
- Why it matters: legal users may assume the AI answer path is complete, grounded, and safe for matter strategy. If the harness is still partial, that mismatch creates trust, accuracy, and disclosure risk.
- Better behavior: gate unfinished AI flows behind a clear beta/limited-capability state, show what sources are included before the user asks, disable unsupported scopes, and add explicit fallback messages when retrieval, grounding, or tool orchestration is unavailable.

### P1 - Fact extraction promotes markdown fragments and headings as legal facts

- Area: CaseBuilder Facts, `/casebuilder/matters/:matterId/facts`
- Steps: opened the fact table for a matter with 1 uploaded document and 24 extracted facts.
- What happened: several extracted rows are not clean facts. Examples include rows beginning with `mp4\`` from an exhibit list, standalone document headings such as `# NOTICE OF SUBMISSION OF PHYSICAL EXHIBITS`, and introductory filing text promoted alongside actual dated events.
- Why it matters: the fact table is a core legal work surface. If headings, template prose, and malformed markdown become reviewable facts, the user must spend time rejecting noise before they can build claims, timelines, or drafts.
- Better behavior: add document-structure filtering before fact creation, parse exhibit-list rows into normalized event/fact fields, drop headings/front matter from fact candidates, and show an extraction-quality summary before asking the user to approve facts.

### P2 - Fact search filters the list but leaves an unrelated fact selected

- Area: CaseBuilder Facts, `/casebuilder/matters/:matterId/facts`
- Steps: searched `Lepman` in the fact table.
- What happened: the left list narrowed to two Lepman-related facts, but the detail/review pane still showed the previously selected March 13 fact about an ongoing dispute. The filter buttons also continued to show total counts rather than filtered counts.
- Why it matters: users may review, approve, reject, or edit the wrong fact because the detail pane no longer matches the filtered list they are looking at.
- Better behavior: when a filter/search changes the visible list, automatically select the first matching result, clear the detail pane with a `Select a fact` state, or visibly mark the detail as outside the current filter. Add filtered result counts.

### P2 - Timeline and dashboard disagree about event state

- Area: CaseBuilder dashboard and Timeline, `/casebuilder/matters/:matterId/timeline`
- Steps: compared the matter dashboard event count with the Timeline page.
- What happened: the dashboard showed `EVENTS 0` and `UPCOMING TIMELINE` empty, but Timeline showed `22 events across 2 months`, `21 suggestions waiting`, and a grouped timeline with one `INVALID DATE` document event plus March 2026 fact events.
- Why it matters: users cannot tell whether the matter has no events, unapproved suggestions, or real timeline entries. For litigation work, the difference between extracted suggestions and committed events needs to be unmistakable.
- Better behavior: use separate counts such as `0 approved events` and `21 suggested events`, show pending timeline suggestions on the dashboard, and keep invalid-date items in a repair queue instead of counting them as normal events.

### P3 - Timeline review exposes internal agent/run metadata

- Area: CaseBuilder Timeline review queue, `/casebuilder/matters/:matterId/timeline`
- Steps: opened Timeline and reviewed the generated suggestion queue.
- What happened: the user-facing queue showed implementation details including `TEMPLATE disabled completed scope document_index`, `provider-free mode`, `timeline-agent-r...`, `index-run:doc...`, chunk IDs, span IDs, and dedupe keys.
- Why it matters: technical metadata is useful for debugging, but it makes the legal review surface noisy and less trustworthy for normal users.
- Better behavior: move agent/run/chunk/span metadata into an expandable diagnostics panel, keep the default card focused on date, event, source, confidence, and review action, and translate provider status into plain-language capability state.

### P2 - Claims empty state does not help users convert extracted facts into legal theory

- Area: CaseBuilder Claims & Defenses, `/casebuilder/matters/:matterId/claims`
- Steps: opened Claims & Defenses for a matter with 24 extracted facts and 21 timeline suggestions but 0 claims/defenses.
- What happened: the main surface showed tabs with zero counts and a large `No claim selected` blank state. The only visible next actions were `Suggest claims` and `New claim`.
- Why it matters: users need help moving from evidence to legal theory. A blank split-pane with no claim selected feels like an error or unfinished page, especially when the matter already has extracted factual material.
- Better behavior: show an empty-state workflow that summarizes available facts/events, explains that no claims exist yet, offers grounded claim templates by jurisdiction/matter type, and labels `Suggest claims` as beta/AI-assisted if it depends on the unfinished harness.

### P1 - Deadline auto-compute is available without showing required legal inputs

- Area: CaseBuilder Deadlines & Tasks, `/casebuilder/matters/:matterId/deadlines`
- Steps: opened Deadlines for a civil-intake matter with no case number and zero existing deadlines.
- What happened: the page showed `OVERDUE 0`, `UPCOMING 0`, `COMPLETE 0`, `TOTAL 0`, with primary actions `Auto-compute` and `Add deadline`. It did not show which court, case-management order, service dates, hearing dates, limitation periods, or trigger facts would be used before auto-computing.
- Why it matters: deadline calculation is a high-stakes legal function. Running computation without visible inputs can create false confidence or silently miss critical deadlines.
- Better behavior: disable `Auto-compute` until required inputs are present, show a preflight checklist of jurisdiction/court/case number/trigger dates/source documents, and require review before computed deadlines become active.

### P2 - Authorities page shows developer endpoint-contract copy in production

- Area: CaseBuilder Authorities, `/casebuilder/matters/:matterId/authorities`
- Steps: opened Authorities for a matter with zero linked claims/defenses/authorities.
- What happened: the right rail displayed `ENDPOINT CONTRACT` with implementation-facing text: `Production data should come from a matter authorities endpoint that returns canonical IDs, currentness, resolved status, linked theories, and pinpoint citations.`
- Why it matters: users should not see internal API contract notes in the product. It makes the page feel unfinished and exposes implementation gaps instead of giving a useful empty state.
- Better behavior: replace developer notes with a user-facing empty state, move endpoint-contract expectations to engineering docs/tests, and show a health banner only in admin/debug mode.

### P2 - Matter graph reads like an internal diagnostic view instead of a legal graph

- Area: CaseBuilder Graph Viewer, `/casebuilder/matters/:matterId/graph`
- Steps: opened the matter graph overview for a matter showing `736 nodes / 12575 edges`.
- What happened: the page displayed a warning that `large-matter paging and graph persistence remain future hardening`, tabs for low-level layers such as `MARKDOWN AST`, `MARKDOWN SEMANTIC`, and `MARKDOWN EMBEDDINGS`, raw relationship labels such as `SUPPORTS FACT`, and graph nodes created from malformed markdown facts/headings.
- Why it matters: a user expects the matter graph to explain legal relationships, not implementation layers. The current view makes it hard to separate useful matter structure from parser/debug output.
- Better behavior: default to a legal-workflow graph with parties, documents, facts, claims, authorities, deadlines, and drafts; hide parser/embedding layers behind admin diagnostics; and summarize relationship meaning in plain language.

### P1 - Matter QC shows all-green zeros despite obvious unreviewed gaps

- Area: CaseBuilder Risk Dashboard, `/casebuilder/matters/:matterId/qc`
- Steps: opened matter QC after reviewing facts, timeline, claims, authorities, and deadlines.
- What happened: QC showed `Evidence gaps 0`, `Authority gaps 0`, `Contradictions 0`, and `Open findings 0`, with green status icons and no last-run timestamp. This matter still had 24 proposed facts, 0 supported facts, 21 timeline suggestions, 0 claims, 0 authorities, and 0 deadlines.
- Why it matters: a risk dashboard that reports clean zeros before meaningful checks run can mislead users into thinking the matter is complete or safe.
- Better behavior: distinguish `not checked`, `not applicable`, and `checked clean`; show last-run time and coverage; count unreviewed facts/timeline suggestions/no-authority/no-claim states as setup gaps or prerequisites instead of clean passes.

### P2 - Evidence Matrix shows zeroed metrics without explaining missing claim elements

- Area: CaseBuilder Evidence Matrix, `/casebuilder/matters/:matterId/evidence`
- Steps: opened Evidence Matrix for a matter with 24 extracted facts but no claims or claim elements.
- What happened: the page showed `SUPPORTED 0/0`, `WEAK 0/0`, `MISSING 0/0`, and `REBUTTED 0/0`, plus active actions `Suggest gaps` and `Export grid`. The main matrix area was blank with `Select an element`.
- Why it matters: `0/0` can read as complete or meaningless. Users need to know that the matrix cannot evaluate support until claims/elements exist.
- Better behavior: replace zeroed metrics with a setup state such as `Create or suggest claims before mapping evidence`, disable export when there is no grid, and route `Suggest gaps` through claim creation or a clearly labeled beta workflow.

### P2 - Drafting templates are enabled before the matter has drafting prerequisites

- Area: CaseBuilder Drafting Studio, `/casebuilder/matters/:matterId/drafts`
- Steps: opened Drafts for a matter with no parties, claims, authorities, supported facts, or deadlines.
- What happened: the page advertised `AI-assisted drafting with citation grounding` and offered AI templates for Motion for Summary Judgment, Discovery Requests, Demand Letter, Deposition Outline, and Trial Brief.
- Why it matters: legal drafting actions imply a level of matter readiness the app has not established. Starting an MSJ or trial brief from an underbuilt matter can produce unusable or risky work product.
- Better behavior: show prerequisite checks for each template, disable high-stakes drafts until required matter objects exist, and offer lower-risk setup actions such as `Review facts`, `Create claims`, or `Add parties` first.

### P2 - Tasks do not reflect obvious matter setup and review work

- Area: CaseBuilder Tasks, `/casebuilder/matters/:matterId/tasks`
- Steps: opened Tasks after reviewing matter facts, timeline, claims, authorities, deadlines, and QC.
- What happened: the work queue showed `Open 0`, `Due soon 0`, `Blocked 0`, `Complete 0`, and all kanban columns were empty. This same matter had proposed facts, pending timeline suggestions, invalid-date data, no claims, no authorities, and no computed deadlines.
- Why it matters: users rely on task queues to know what to do next. When the app detects unresolved matter setup work but does not turn it into tasks or recommendations, users can miss essential review steps.
- Better behavior: auto-surface system tasks such as `Review extracted facts`, `Resolve invalid timeline date`, `Create claims`, `Link authorities`, and `Review timeline suggestions`; separate system-recommended tasks from user-created tasks.

### P2 - Export and AI settings inherit risky defaults without readiness context

- Area: CaseBuilder Exports and Settings, `/casebuilder/matters/:matterId/export`, `/casebuilder/matters/:matterId/settings`
- Steps: opened Exports, then Settings AI and Export tabs.
- What happened: Exports offered `Prepare` actions for DOCX, PDF, and Filing packet even though no work products existed. Settings showed AI timeline suggestions/enrichment as `Inherit (on)` and export defaults as `Include exhibits Inherit (on)` and `Include QC report Inherit (on)`.
- Why it matters: inherited defaults hide the actual source of truth. Users cannot tell whether AI enrichment is fully available, whether QC has meaningful coverage, or what would be included in a generated export.
- Better behavior: show inherited-from/global policy details, display readiness checks before export preparation, disable empty exports, and warn when included QC/AI-derived content has not been reviewed.

### P3 - Global search accessible placeholder can lag behind the current route

- Area: global shell search, Admin routes
- Steps: opened Admin/Beta Access and compared the visible search placeholder with the accessibility metadata.
- What happened: the visible search affordance was route-specific (`Search admin...`), while the accessibility tree still described the field with a matters-oriented placeholder.
- Why it matters: assistive-technology users get stale context about what the search box will search.
- Better behavior: keep visual placeholder, accessible label, and route-specific search scope in the same source of truth; test this during client-side route transitions.

### P3 - Search entry points behave inconsistently for exact citations

- Area: dashboard search, global shell search, Search page
- Steps: searched `ORS 90.300` from the dashboard hero search, then searched the same citation from the global shell search while in Admin.
- What happened: the dashboard search jumped directly to the statute detail, while the global shell search routed to `/search?q=ORS+90.300`. On the Search page, the global placeholder became `Search search...`.
- Why it matters: users cannot predict whether entering an exact citation will open the authority or show a result list. The repeated placeholder copy also makes the shell feel mechanically generated.
- Better behavior: use one exact-citation behavior across search entry points, or explicitly label one as `Jump to citation` and another as `Search all`. Replace route-generated placeholders like `Search search...` with hand-authored copy.

### P3 - Search typeahead remains open after submit and covers the result

- Area: Search page, `/search?q=ORS+90.300`
- Steps: submitted `ORS 90.300` from the global shell search and landed on Search.
- What happened: the page showed `1 - 1 of 1` result, but the focused search input kept a typeahead panel open with many ORS 90.300 provision suggestions, visually covering the actual result area.
- Why it matters: users can confuse suggestions with search results, and the real result is harder to inspect until focus changes.
- Better behavior: close typeahead on submit/navigation, label suggestions distinctly from committed results, and reopen suggestions only when the user edits the query.

### P3 - Sidebar matter context renders raw markdown-style link text

- Area: global left sidebar on Search/Admin routes
- Steps: navigated across Search and Admin after opening a matter.
- What happened: the sidebar footer showed raw text similar to `matter [lknlkn](frontend-production-090c.up.railway.app/casebuilder/matters/...)` instead of a rendered link or clean current-matter label.
- Why it matters: raw markdown in production UI looks broken and makes the current matter context feel accidental rather than intentional.
- Better behavior: render the matter as a proper link with a concise label such as `Matter: lknlkn`, or hide matter context on global routes where it does not apply.

### P3 - Statute graph isolated-node state does not explain missing relationships

- Area: statute detail Graph tab, `/statutes/or%3Aors%3A90.320?tab=graph`
- Steps: opened ORS 90.320 and selected Graph.
- What happened: the graph rendered a single isolated ORS 90.320 node with an edge legend, but no message explaining that no relationships were found or that citation edges may be unresolved.
- Why it matters: users see a graph visualization and may assume the statute has no legal relationships, even though the text includes linked ORS references.
- Better behavior: add an explicit empty/partial graph state such as `No graph edges available for this statute yet`, include counts of unresolved inline citations when present, and link to QC/source diagnostics for admin users.

### P3 - Upload status widget can obscure lower-right UI

- Area: global upload status widget
- Steps: navigated while an upload/indexing job was active and after it completed.
- What happened: the widget sits in the lower-right corner and can overlap page content. It can be collapsed, but it remains visually present.
- Why it matters: persistent system widgets are helpful, but they should not cover controls or content during normal browsing.
- Better behavior: reserve layout space, allow full dismiss after completion, and keep completed status in a less intrusive toast/history area.

### P3 - New Matter flow copy and validation can be clearer

- Area: New Matter, `/casebuilder/new`
- Steps: opened the form without submitting.
- What happened: the flow is generally strong, but the upload area says "Drag & drop or browse" while the visible button says `UPLOAD FOLDER`, and `CREATE MATTER` appears available before entering a required matter name.
- Why it matters: users may not know whether files or only folders can be uploaded, and enabled submit buttons without required fields invite avoidable validation errors.
- Better behavior: align upload copy with available actions, show required-field markers, and disable `CREATE MATTER` until the required matter name is present.

### P3 - Admin page is powerful but mixes routine status with dangerous operations

- Area: Admin dashboard, `/admin`
- Steps: opened Admin and reviewed the visible status, source registry, and run controls without starting any jobs.
- What happened: the page exposes useful runtime status, but ingest/combine/index/job controls are visible in the same dense surface as read-only monitoring.
- Why it matters: admins need speed, but production operations benefit from strong separation between observing and mutating.
- Better behavior: split monitoring and operations into separate tabs or modes, require confirmation for ingest/index/materialize actions, and visually badge mutating controls.

## Things Working Well

- Dashboard loaded with API and Neo4j connected and showed corpus/graph status clearly.
- The dashboard search for `ORS 90.300` routed directly to the statute detail, which is a good fast path for citation-aware users.
- Statute text and provision browsing loaded, and legal citations were linkified.
- The embedded statute graph and graph explorer loaded a focused ORS 90.300 neighborhood after a short wait.
- Citation-mode search for `ORS 90.300` returned the expected statute result.
- The New Matter flow has a useful guided structure: intent, matter details, files, and configuration.
- The fact table and timeline review queue load extracted matter data with source links and confidence/provenance indicators.
- Matter settings expose per-matter controls for details, AI/timeline behavior, and export defaults.

## Flows Covered

- [x] Dashboard and global navigation
- [x] Search
- [x] Ask ORSGraph
- [x] Statute detail and statute browsing
- [x] Citation graph
- [x] Global shell search
- [x] Matters / CaseBuilder entry points
- [x] CaseBuilder document list/detail
- [x] Admin / runtime health
- [x] Admin QC console
- [x] Admin beta access
- [x] Upload status widget
- [x] CaseBuilder matter dashboard
- [x] CaseBuilder Ask Matter
- [x] CaseBuilder Facts
- [x] CaseBuilder Timeline
- [x] CaseBuilder Evidence Matrix
- [x] CaseBuilder Claims & Defenses
- [x] CaseBuilder Deadlines & Tasks
- [x] CaseBuilder Authorities
- [x] CaseBuilder Matter Graph
- [x] CaseBuilder Matter QC
- [x] CaseBuilder Drafting Studio
- [x] CaseBuilder Tasks
- [x] CaseBuilder Exports
- [x] CaseBuilder Settings

## Next Checks

- Re-test Ask after clearing state or in a fresh browser session to confirm the stale-answer bug is not session-specific.
- Re-test document detail at another viewport size to isolate whether the blank canvas is a responsive layout issue.
- Test keyboard-only navigation and focus order for Search, Statute tabs, and CaseBuilder document review.
- Test mobile/narrow viewports for the statute intelligence panel and CaseBuilder forms.
- Inspect Complaint Editor and Work Product editor states without generating or saving new work product.
- Re-test AI/Ask surfaces after the harness is fully wired and add capability-specific acceptance criteria.
