# 11 - Case History Version Control Backlog

Case History is CaseBuilder's legal version-control system. It gives complaints and later work products a Git-like audit backbone without exposing Git language to users. The user should experience this as automatic drafting history: compare, restore, create alternatives, bring changes in, mark milestones, and prove exactly what was exported.

Internal name: `LegalVersionControl`.

User-facing name: `Case History`.

## Current Code Baseline

The first graph-native Case History slice is implemented. The codebase now has:

- Canonical work-product Case History DTOs in backend and frontend: `ChangeSet`, `VersionChange`, `VersionSnapshot`, `SnapshotManifest`, `SnapshotEntityState`, `VersionBranch`, `LegalImpactSummary`, `VersionChangeSummary`, `LegalSupportUse`, `AIEditAudit`, restore responses, and compare responses.
- Neo4j constraints/indexes for version nodes, branch nodes, manifest/entity-state nodes, support-use nodes, AI audit nodes, and milestones.
- Deterministic hash helpers for document/work-product state, support graph, QC state, formatting profile, manifest state, and export artifacts.
- Shared work-product history routes for history, change-set detail, snapshot list/detail/create, compare, restore, export history, and AI audit.
- Complaint route aliases that delegate into the same canonical work-product history handlers instead of creating a separate complaint history stack.
- Root/main branch creation, root snapshots, change-set recording, snapshot manifests, entity-state records, export snapshots, AI audit records, and changed-since-export checks.
- Complaint facade save paths synchronize into canonical work-product history, so existing complaint editor actions create `ChangeSet`/`VersionSnapshot` records.
- A Complaint workspace `History` panel with timeline, manual snapshot, text compare, restore dry-run/apply, and export changed-since-export indicator.

The deliberate remaining pre-launch cleanup is:

- Keep `ComplaintDraft` as a temporary typed facade only while the shared `WorkProduct` editor matures.
- Remove or fully demote stored `ComplaintHistoryEvent`, `WorkProductHistoryEvent`, and any generic `DraftVersion` truth before launch.
- Expand first-class support-use nodes so support/citation/QC diffs no longer depend on scanning JSON arrays.
- Add matter-isolation route tests and end-to-end history smoke coverage before calling Case History release-ready.

Because CaseBuilder has not launched yet, continue optimizing for the clean model over compatibility. Do not rebuild version control around complaint-specific persistence.

## Current Implementation Status

### Landed in First Build Slice

- `WorkProduct` is the canonical version subject for durable history.
- Complaint APIs/UI remain as user-friendly facades.
- Canonical routes exist under `/api/v1/matters/:matterId/work-products/:workProductId/...`.
- Complaint aliases exist under `/api/v1/matters/:matterId/complaints/:complaintId/...` and delegate to canonical work-product handlers.
- Every major work-product create/edit/block/support/QC/AI/export/restore action records a `ChangeSet` and snapshot.
- Export creates an immutable snapshot and stores `snapshot_id`, `artifact_hash`, `render_profile_hash`, `qc_status_at_export`, and `changed_since_export`.
- Compare supports V0 text/block diff.
- Restore supports whole work product and block/paragraph scopes, including dry-run warnings.
- AI commands record `AIEditAudit` even in provider-free/template mode.
- Frontend Case History route/panel is wired into the Complaint workspace.

### Still Partial

- `LegalVersionControlService` behavior currently lives inside `CaseBuilderService`; it should be extracted when the next slice touches service boundaries.
- `ComplaintHistoryEvent` and `WorkProductHistoryEvent` models still exist as compatibility/facade projections; they are no longer the desired truth.
- First-class support-use graph nodes exist, but support arrays are still used as current-view convenience fields.
- Compare is text/block-first; support, citation, authority, claim/element, QC, and export diffs are represented only in summaries or future backlog.
- Restore warnings are basic; support-loss and unresolved-citation warnings need richer graph inspection.
- Timeline UI exists, but filters, snapshot detail drawer, audit reports, support graph over time, branch manager, and merge cards remain.

### Verified

- `cargo check -p orsgraph-api`
- `cargo test -p orsgraph-api --test graph_contract`
- `cargo test -p orsgraph-api work_product_hashes_are_stable_and_layered --lib`
- `pnpm run check` from `frontend/`

## Pre-Launch Optimization Decisions

These decisions should be made now, before user data exists:

1. Use one canonical versioned subject model.
   `WorkProduct` is the subject. `product_type=complaint` plus `profile_id=oregon_circuit_civil_complaint` replaces special-case complaint persistence over time.

2. Make the timeline event a `ChangeSet`, not a single low-level change.
   One user action can edit paragraph text, remove a citation, add evidence, and resolve a warning. The timeline should show one legal event with multiple `VersionChange` operations below it.

3. Store important legal links as graph nodes, not arrays.
   Replace or deprecate `fact_ids`, `evidence_ids`, and inline authority arrays on draft objects with first-class `FactUse`, `EvidenceUse`, `AuthorityUse`, `CitationUse`, `ExhibitReference`, and `ElementSupport` nodes.

4. Use immutable state records plus fast current views.
   Current `WorkProduct` and block nodes can stay easy to load. Exact historical state lives in `SnapshotManifest` and `SnapshotEntityState` records, with full JSON/blob fallback only for fast restore.

5. Use projections for UI convenience.
   Timeline cards, simple history rows, support charts, export-history lists, and "changed since export" banners are read models derived from version nodes, not separate truth.

6. Prefer shared work-product endpoints.
   Complaint routes can remain friendly UI routes, but API contracts should converge on `/work-products/:workProductId/...` so motions and answers inherit Case History automatically.

## Product Promise

For any complaint or work product, the user can answer:

- What changed?
- Who or what changed it?
- Why did it change?
- What facts, evidence, exhibits, and authorities were affected?
- Did the edit add or resolve QC warnings?
- What did AI do, what sources did it use, and did the user accept it?
- Can I restore the old version without destroying current history?
- What exact version did I export, send, file, or serve?

## UX Vocabulary

Use these terms in UI:

- History
- Snapshot
- Compare
- Restore
- Create alternative
- Bring changes in
- Milestone
- Review packet
- Draft conflict

Avoid by default:

- Commit
- Rebase
- Detached head
- Cherry-pick
- Merge conflict
- Blame

## Architecture Decision

Use event sourcing plus immutable snapshots.

- `VersionChange` records every meaningful edit, support link, citation change, QC action, AI action, export, restore, merge, and milestone.
- `VersionSnapshot` records full legal state periodically and whenever legal trust requires it.
- Snapshots store or reference canonical state for document AST, support graph, QC state, formatting profile, AI audit payload, and export render metadata.
- Hashes prove integrity: `document_hash`, `support_graph_hash`, `qc_state_hash`, `formatting_hash`, and export `artifact_hash`.
- Restore never deletes history. Restore creates a new `VersionChange` and usually a new `VersionSnapshot`.
- Export always creates an immutable export snapshot before artifact generation.

Snapshot triggers:

- Complaint/work product created.
- Meaningful paragraph/section/count edit.
- Section or count generated.
- Claim/count/relief changed.
- Fact, evidence, exhibit, citation, or authority link changed.
- AI edit accepted or materially edited after insertion.
- QC/rule check completed.
- QC finding resolved, ignored, or reopened.
- Export generated.
- Manual milestone created.
- Branch created.
- Restore or merge completed.
- Every 10 to 25 meaningful changes, whichever is configured.

Noise controls:

- Typing bursts group into one `TextEdit` change after idle debounce.
- Batch support linking creates one `SupportLinkBatch` change.
- Auto-save remains frequent, but auto-snapshot only records meaningful legal state.

## Optimized Data Model

The model should be graph-native: stable legal objects are nodes, legal support relationships are nodes or typed edges, timeline events are grouped, and snapshots are immutable manifests of exact legal state.

### VersionSubject

`WorkProduct` is the canonical version subject. A complaint is a work product with a complaint profile.

```ts
type VersionSubject = {
  subject_id: string // same as work_product_id
  matter_id: string
  subject_type: "work_product"
  product_type:
    | "complaint"
    | "motion"
    | "answer"
    | "declaration"
    | "brief"
    | "notice"
    | "exhibit_list"
    | "filing_packet"
  profile_id: string
  title: string
  current_branch_id: string
  current_snapshot_id: string
  review_status: string
  updated_at: string
}
```

### Complaint As WorkProduct Profile

Complaint should not require a separate persistence tree once the shared model is in place. It should be a `WorkProduct` with complaint-specific block roles and view models.

```ts
type ComplaintWorkProductProfile = {
  profile_id: "oregon_circuit_civil_complaint"
  product_type: "complaint"
  block_roles:
    | "caption"
    | "party_block"
    | "jurisdiction_venue"
    | "section_heading"
    | "factual_paragraph"
    | "count"
    | "count_heading"
    | "count_paragraph"
    | "element"
    | "prayer_for_relief"
    | "signature_block"
    | "certificate_of_service"
  required_roles: string[]
  rule_pack_id: string
  export_profile_ids: string[]
}
```

Implementation rule:

- `ComplaintDraft`, `ComplaintSection`, `ComplaintCount`, and `PleadingParagraph` can remain frontend/backend view models while the UI is being migrated.
- The persisted source of truth should become `WorkProduct`, `WorkProductBlock`, `LegalSupportUse`, `WorkProductFinding`, `WorkProductArtifact`, and Case History nodes.
- Stable block IDs replace paragraph numbers as identity. Paragraph numbers are render state and must be versioned through renumber mapping, not used as primary keys.

### ChangeSet

This is the user-facing timeline event. It groups one or more low-level `VersionChange` operations.

```ts
type ChangeSet = {
  change_set_id: string
  matter_id: string
  subject_id: string
  branch_id: string
  snapshot_id: string
  parent_snapshot_id?: string | null
  title: string
  summary: string
  reason?: string | null
  actor_type: "user" | "ai" | "system"
  actor_id?: string | null
  source:
    | "autosave"
    | "manual_snapshot"
    | "editor"
    | "support_link"
    | "citation"
    | "rule_check"
    | "ai"
    | "export"
    | "restore"
    | "branch"
    | "merge"
    | "milestone"
  created_at: string
  change_ids: string[]
  legal_impact: LegalImpactSummary
}
```

### VersionSnapshot

```ts
type VersionSnapshot = {
  snapshot_id: string
  matter_id: string
  subject_type: "work_product"
  subject_id: string // work_product_id
  product_type: string
  profile_id: string
  branch_id: string
  sequence_number: number
  title: string
  message?: string | null
  created_at: string
  created_by: "user" | "ai" | "system"
  actor_id?: string | null
  snapshot_type:
    | "auto"
    | "manual"
    | "ai_edit"
    | "rule_check"
    | "export"
    | "restore"
    | "merge"
    | "milestone"
    | "branch"
  parent_snapshot_ids: string[]
  document_hash: string
  support_graph_hash: string
  qc_state_hash: string
  formatting_hash: string
  manifest_hash: string
  manifest_ref?: string | null
  full_state_ref?: string | null
  full_state_inline?: unknown
  summary: VersionChangeSummary
}
```

### VersionChangeSummary

```ts
type VersionChangeSummary = {
  text_changes: number
  support_changes: number
  citation_changes: number
  authority_changes: number
  qc_changes: number
  export_changes: number
  ai_changes: number
  targets_changed: Array<{ target_type: string; target_id: string; label?: string }>
  risk_level: "none" | "low" | "medium" | "high"
  user_summary: string
}
```

### SnapshotManifest

The manifest is the exact state map. It lets the graph answer "what changed?" without loading a giant blob for every comparison.

```ts
type SnapshotManifest = {
  manifest_id: string
  snapshot_id: string
  matter_id: string
  subject_id: string
  manifest_hash: string
  entry_count: number
  storage_ref?: string | null
  created_at: string
}
```

### SnapshotEntityState

Each entry points to the state of a meaningful legal object at a snapshot.

```ts
type SnapshotEntityState = {
  entity_state_id: string
  manifest_id: string
  snapshot_id: string
  matter_id: string
  subject_id: string
  entity_type:
    | "work_product"
    | "block"
    | "section"
    | "count"
    | "paragraph"
    | "sentence"
    | "fact_use"
    | "evidence_use"
    | "authority_use"
    | "citation_use"
    | "exhibit_reference"
    | "element_support"
    | "relief_request"
    | "rule_finding"
    | "formatting_profile"
    | "export_artifact"
  entity_id: string
  entity_hash: string
  state_ref?: string | null
  state_inline?: unknown
}
```

### VersionChange

```ts
type VersionChange = {
  change_id: string
  change_set_id: string
  snapshot_id: string
  matter_id: string
  subject_type: "work_product"
  subject_id: string
  branch_id: string
  target_type:
    | "work_product"
    | "section"
    | "count"
    | "block"
    | "paragraph"
    | "sentence"
    | "citation"
    | "evidence_link"
    | "fact_link"
    | "authority_link"
    | "exhibit_reference"
    | "rule_finding"
    | "formatting_profile"
    | "export"
    | "ai_edit"
  target_id: string
  operation:
    | "create"
    | "update"
    | "delete"
    | "move"
    | "link"
    | "unlink"
    | "resolve"
    | "ignore"
    | "restore"
    | "merge"
    | "export"
  before?: unknown
  after?: unknown
  before_hash?: string | null
  after_hash?: string | null
  summary: string
  legal_impact: LegalImpactSummary
  ai_audit_id?: string | null
  created_at: string
  created_by: "user" | "ai" | "system"
  actor_id?: string | null
}
```

### LegalSupportUse

Use one common base shape for support links. Specific node labels can still be `FactUse`, `EvidenceUse`, `AuthorityUse`, `CitationUse`, and `ExhibitReference`.

```ts
type LegalSupportUse = {
  support_use_id: string
  matter_id: string
  subject_id: string
  branch_id: string
  target_type:
    | "work_product"
    | "block"
    | "section"
    | "count"
    | "paragraph"
    | "sentence"
    | "element"
    | "relief_request"
  target_id: string
  source_type:
    | "fact"
    | "evidence"
    | "document"
    | "source_span"
    | "exhibit"
    | "authority"
    | "provision"
    | "citation"
  source_id: string
  relation:
    | "supports"
    | "partially_supports"
    | "contradicts"
    | "context_only"
    | "impeaches"
    | "authenticates"
    | "cites"
    | "requires"
  status: "active" | "needs_review" | "resolved" | "retired" | "tombstoned"
  quote?: string | null
  pinpoint?: string | null
  confidence?: number | null
  created_snapshot_id: string
  retired_snapshot_id?: string | null
}
```

### LegalImpactSummary

```ts
type LegalImpactSummary = {
  affected_counts: string[]
  affected_elements: string[]
  affected_facts: string[]
  affected_evidence: string[]
  affected_authorities: string[]
  affected_exhibits: string[]
  support_status_before?: "supported" | "partial" | "unsupported" | "contradicted"
  support_status_after?: "supported" | "partial" | "unsupported" | "contradicted"
  qc_warnings_added: string[]
  qc_warnings_resolved: string[]
  blocking_issues_added: string[]
  blocking_issues_resolved: string[]
}
```

### VersionBranch

```ts
type VersionBranch = {
  branch_id: string
  matter_id: string
  subject_type: "work_product"
  subject_id: string
  name: string
  description?: string | null
  created_from_snapshot_id: string
  current_snapshot_id: string
  branch_type: "main" | "alternative" | "strategy" | "review" | "filing" | "archive"
  created_at: string
  updated_at: string
  archived_at?: string | null
}
```

### AIEditAudit

```ts
type AIEditAudit = {
  ai_audit_id: string
  matter_id: string
  subject_type: "work_product"
  subject_id: string
  target_type: string
  target_id: string
  command: string
  prompt_template_id?: string | null
  model?: string | null
  provider_mode: "live" | "template" | "deterministic" | "disabled"
  input_fact_ids: string[]
  input_evidence_ids: string[]
  input_authority_ids: string[]
  input_snapshot_id: string
  output_text?: string | null
  inserted_text?: string | null
  user_action: "accepted" | "rejected" | "edited" | "template_recorded"
  warnings: string[]
  created_at: string
}
```

### ExportArtifact Extensions

Extend the current `ExportArtifact` shape with:

```ts
type VersionedExportFields = {
  snapshot_id: string
  artifact_hash: string
  render_profile_hash: string
  qc_status_at_export: "clear" | "warning" | "serious" | "blocking" | "not_run"
  changed_since_export: boolean
  immutable: true
}
```

## Graph Nodes

Add these nodes:

- `ChangeSet`
- `VersionSnapshot`
- `SnapshotManifest`
- `SnapshotEntityState`
- `VersionChange`
- `VersionBranch`
- `Milestone`
- `AIEditAudit`
- `FactUse`
- `AuthorityUse`
- `ElementSupport`
- `ChangeReview`
- `MergeRequest`
- `ConflictResolution`

Reuse and extend:

- `WorkProduct`
- `WorkProductBlock`
- `EvidenceUse`
- `CitationUse`
- `ExhibitReference`
- `ExportArtifact`

Remove or demote before launch:

- `ComplaintHistoryEvent` as stored truth.
- `WorkProductHistoryEvent` as stored truth.
- `DraftVersion` as stored truth.
- Inline `fact_ids`, `evidence_ids`, and authority arrays when a first-class support-use node can represent the relationship.

## Graph Edges

- `(:Matter)-[:HAS_VERSION_SUBJECT]->(:WorkProduct)`
- `(:WorkProduct)-[:HAS_BRANCH]->(:VersionBranch)`
- `(:VersionBranch)-[:CURRENT_SNAPSHOT]->(:VersionSnapshot)`
- `(:VersionBranch)-[:HAS_SNAPSHOT]->(:VersionSnapshot)`
- `(:VersionSnapshot)-[:HAS_PARENT]->(:VersionSnapshot)`
- `(:VersionSnapshot)-[:HAS_MANIFEST]->(:SnapshotManifest)`
- `(:SnapshotManifest)-[:HAS_ENTITY_STATE]->(:SnapshotEntityState)`
- `(:SnapshotEntityState)-[:STATE_OF]->(:WorkProduct|WorkProductBlock|FactUse|EvidenceUse|AuthorityUse|CitationUse|ExhibitReference|ElementSupport|RuleCheckFinding|ExportArtifact)`
- `(:VersionSnapshot)-[:CREATED_BY_CHANGESET]->(:ChangeSet)`
- `(:ChangeSet)-[:HAS_CHANGE]->(:VersionChange)`
- `(:VersionChange)-[:CHANGED]->(:WorkProductBlock|FactUse|EvidenceUse|AuthorityUse|CitationUse|ExhibitReference|ElementSupport|RuleCheckFinding|ExportArtifact)`
- `(:VersionChange)-[:AFFECTED_FACT]->(:Fact)`
- `(:VersionChange)-[:AFFECTED_EVIDENCE]->(:Evidence)`
- `(:VersionChange)-[:AFFECTED_AUTHORITY]->(:Provision)`
- `(:VersionChange)-[:AFFECTED_EXHIBIT]->(:CaseDocument|Evidence)`
- `(:VersionChange)-[:AI_AUDIT]->(:AIEditAudit)`
- `(:VersionSnapshot)-[:PRODUCED_EXPORT]->(:ExportArtifact)`
- `(:Milestone)-[:TAGS]->(:VersionSnapshot)`
- `(:FactUse)-[:USES_FACT]->(:Fact)`
- `(:EvidenceUse)-[:USES_EVIDENCE]->(:Evidence)`
- `(:EvidenceUse)-[:USES_SPAN]->(:SourceSpan)`
- `(:AuthorityUse)-[:USES_AUTHORITY]->(:Provision)`
- `(:CitationUse)-[:CITES_AUTHORITY]->(:Provision)`
- `(:ElementSupport)-[:SUPPORTS_ELEMENT]->(:Element)`
- `(:MergeRequest)-[:FROM_BRANCH]->(:VersionBranch)`
- `(:MergeRequest)-[:TO_BRANCH]->(:VersionBranch)`
- `(:ConflictResolution)-[:RESOLVES]->(:MergeRequest)`

## API Shape

Use shared work-product endpoints as canonical now. Complaint routes can remain friendly route aliases, but they should call the same service and return the same underlying version objects. This is the main pre-launch breaking optimization.

### Canonical Work Product Case History

- `GET /api/v1/matters/:matterId/work-products/:workProductId/history`
- `GET /api/v1/matters/:matterId/work-products/:workProductId/change-sets/:changeSetId`
- `GET /api/v1/matters/:matterId/work-products/:workProductId/snapshots`
- `GET /api/v1/matters/:matterId/work-products/:workProductId/snapshots/:snapshotId`
- `POST /api/v1/matters/:matterId/work-products/:workProductId/snapshots`
- `POST /api/v1/matters/:matterId/work-products/:workProductId/milestones`
- `GET /api/v1/matters/:matterId/work-products/:workProductId/compare?from=&to=&layers=text,support,evidence,authority,qc,formatting,export`
- `POST /api/v1/matters/:matterId/work-products/:workProductId/restore`
- `GET /api/v1/matters/:matterId/work-products/:workProductId/branches`
- `POST /api/v1/matters/:matterId/work-products/:workProductId/branches`
- `PATCH /api/v1/matters/:matterId/work-products/:workProductId/branches/:branchId`
- `POST /api/v1/matters/:matterId/work-products/:workProductId/branches/:branchId/merge`
- `GET /api/v1/matters/:matterId/work-products/:workProductId/audit`
- `GET /api/v1/matters/:matterId/work-products/:workProductId/ai-audit`
- `GET /api/v1/matters/:matterId/work-products/:workProductId/export-history`

### Complaint Route Aliases

- `GET /api/v1/matters/:matterId/complaints/:complaintId/history`
- `GET /api/v1/matters/:matterId/complaints/:complaintId/snapshots`
- `GET /api/v1/matters/:matterId/complaints/:complaintId/snapshots/:snapshotId`
- `POST /api/v1/matters/:matterId/complaints/:complaintId/snapshots`
- `POST /api/v1/matters/:matterId/complaints/:complaintId/milestones`
- `GET /api/v1/matters/:matterId/complaints/:complaintId/compare?from=&to=&layers=text,support,evidence,authority,qc,formatting,export`
- `POST /api/v1/matters/:matterId/complaints/:complaintId/restore`
- `GET /api/v1/matters/:matterId/complaints/:complaintId/branches`
- `POST /api/v1/matters/:matterId/complaints/:complaintId/branches`
- `PATCH /api/v1/matters/:matterId/complaints/:complaintId/branches/:branchId`
- `POST /api/v1/matters/:matterId/complaints/:complaintId/branches/:branchId/merge`
- `GET /api/v1/matters/:matterId/complaints/:complaintId/audit`
- `GET /api/v1/matters/:matterId/complaints/:complaintId/ai-audit`
- `GET /api/v1/matters/:matterId/complaints/:complaintId/export-history`

These should be aliases only. They should not have separate persistence, DTOs, diff logic, or restore code.

### Restore Request

```json
{
  "snapshot_id": "snap_123",
  "scope": "paragraph",
  "target_ids": ["para_18"],
  "mode": "restore_as_new_event",
  "branch_id": "branch_main"
}
```

Scopes:

- `work_product`
- `complaint`
- `block`
- `section`
- `count`
- `paragraph`
- `sentence`
- `citation_links`
- `evidence_links`
- `authority_links`
- `exhibit_links`
- `formatting_profile`
- `qc_decisions`

### API Surface To Remove Before Launch

- Stored generic `/drafts/:draftId/history` version routes unless generic drafts become `WorkProduct`.
- Any endpoint that mutates `ComplaintHistoryEvent` directly.
- Any route that stores support solely as `fact_ids`/`evidence_ids` arrays without creating support-use graph nodes.

## Frontend Components

Add or extend:

- `VersionTimeline`
- `SnapshotCard`
- `ChangeEventCard`
- `CompareVersionsModal`
- `LegalDiffViewer`
- `BranchManager`
- `MilestoneBar`
- `RollbackDialog`
- `MergeChangesPanel`
- `AIEditAuditPanel`
- `SupportHistoryGraph`
- `ExportHistoryPanel`
- `AuditReportPanel`

Where these fit in current UI:

- Add `History` as a first-class Complaint workspace route after `Export`.
- Keep the existing inspector `history` tab for the selected paragraph/count/citation, powered by filtered Case History.
- Add snapshot/compare/restore actions to export cards, QC findings, AI command results, and paragraph action menus.

## Diff Layers

Text layer:

- Added, removed, modified words.
- Paragraph created, deleted, moved, split, merged.
- Section/count title changed.

Support layer:

- Facts added or removed.
- Evidence links added, removed, or changed.
- Exhibit links added, removed, missing, or relabeled.
- Support status changed.

Authority layer:

- Citations added or removed.
- Citation resolved, unresolved, stale, ambiguous, or scope warning changed.
- Authority links changed.

Claim/element layer:

- Count added or removed.
- Claim theory changed.
- Element support changed.
- Relief changed.

QC layer:

- Warnings added or resolved.
- Blocking issues added or resolved.
- Finding ignored or reopened.
- Unsupported allegation count changed.

Export layer:

- Page count changed.
- Caption changed.
- Signature/certificate moved or changed.
- Exhibit references changed.
- Render profile/hash changed.

## Integrity Rules

- No destructive delete without tombstone event.
- No restore without new `VersionChange`.
- No AI insertion without `AIEditAudit`.
- No export without immutable `VersionSnapshot`.
- No final filing packet without version hash.
- No evidence, exhibit, citation, or authority link change without history.
- No paragraph renumbering without old-number to new-number mapping.
- No cross-matter restore, compare, branch, or merge.
- No UI claim of filing-ready unless attached to a specific milestone, export hash, and review status.
- No confidential full text in logs; full text stays in snapshots or storage objects, not operational logs.

## Add, Change, Remove

Add now:

- `ChangeSet` as the timeline/event card model.
- `SnapshotManifest` and `SnapshotEntityState` for graph-aware historical state.
- `FactUse`, `AuthorityUse`, and `ElementSupport` nodes to complete the support graph started by `EvidenceUse`, `CitationUse`, and `ExhibitReference`.
- Canonical work-product history endpoints.
- Branch current-snapshot pointers.
- Projection/rebuild logic for timeline cards and export-history rows.
- Dry-run restore API that returns support loss and QC warning previews.

Change now:

- Make `WorkProduct` the canonical versioned document and Complaint a profile/facade.
- Change snapshot parent edges to child-to-parent `HAS_PARENT` for ancestor queries.
- Change timeline data from flat history arrays to `ChangeSet -> VersionChange`.
- Change support hashes to derive from support-use nodes instead of raw ID arrays.
- Change export artifacts to always point to an immutable snapshot and artifact hash.
- Change AI history from simple command messages to `AIEditAudit` linked to input and output snapshots.

Remove before launch:

- Stored `ComplaintHistoryEvent`, `WorkProductHistoryEvent`, and `DraftVersion` as sources of truth.
- Any new complaint-only version-control service.
- Any restore/compare/export-history code path that does not use `LegalVersionControl`.
- Raw support arrays as authoritative state when a support-use node exists.
- Direct hard delete of legal draft targets without tombstone changes.
- Separate complaint and work-product history APIs that can drift.

## Agile Backlog

### Epic CB-CH-000 - Program Setup and Scope Lock

## CB-CH-000 - Case History owns legal version control
- Priority: P0
- Area: Planning/product
- Problem: Version history is currently a light event stream inside Complaint Editor and not a complete legal version-control system.
- Expected behavior: This file is the source of truth for Case History work, with shared `WorkProduct` version control as the platform and Complaint as the first product profile.
- Implementation notes: Cross-link from `CB-CE-017`, `CB-CE-028`, `CB-V1-004`, and `CB-V1-005` when those docs are next edited. Do not implement a complaint-only version-control stack.
- Acceptance checks: Product scope, vocabulary, data model, API, graph model, UI components, and phased backlog are documented.
- Dependencies: Current Complaint Editor and WorkProduct scaffolding.
- Status: Done

## CB-CH-001 - Pre-launch breaking model cleanup
- Priority: P0
- Area: Architecture/data
- Problem: The current code has `ComplaintDraft`, generic `Draft`, `WorkProduct`, `ComplaintHistoryEvent`, `WorkProductHistoryEvent`, and `DraftVersion`, which can become competing sources of truth.
- Expected behavior: Choose `WorkProduct` as the canonical versioned legal document. Complaint-specific objects become a profile/view/facade, not independent history persistence.
- Implementation notes: This is explicitly allowed because there are no launched users. Prefer the clean model over migration compatibility.
- Acceptance checks: New version-control tickets and APIs reference `subject_id=work_product_id`; no new persistence depends on `ComplaintHistoryEvent` or `DraftVersion`.
- Dependencies: `CB-CE-028`, current Complaint Editor implementation.
- Status: Partial
- Progress: Durable Case History now routes through `work_product_id`; complaint routes are aliases/facades. Stored flat history DTOs still exist and must be removed or fully demoted before launch.

### Epic CB-CH-100 - Core Data Model and Persistence

## CB-CH-101 - Version DTO registry
- Priority: P0
- Area: API/data
- Problem: Backend/frontend type registries lack durable version-control primitives.
- Expected behavior: Add backend Rust and frontend TypeScript DTOs for `VersionSubject`, `ChangeSet`, `VersionSnapshot`, `SnapshotManifest`, `SnapshotEntityState`, `VersionChange`, `VersionChangeSummary`, `LegalImpactSummary`, `VersionBranch`, `Milestone`, `AIEditAudit`, `MergeRequest`, and `ConflictResolution`.
- Implementation notes: `subject_id` should be the canonical `work_product_id`. Complaint IDs can appear in route aliases or UI view models, not in core version-control DTOs.
- Acceptance checks: DTO contract tests serialize/normalize all new types and enforce `matter_id`, `subject_type=work_product`, and `subject_id`.
- Dependencies: `CB-CH-001`, `CB-X-001`, `CB-X-013`, `CB-CE-028`.
- Status: Partial
- Progress: Backend/frontend DTOs now cover core V0 version records, snapshots, manifests, branches, legal impact, AI audit, compare, and restore. `MergeRequest` and `ConflictResolution` remain for the merge-card slice.

## CB-CH-102 - Neo4j constraints, indexes, and edge materialization
- Priority: P0
- Area: Graph/persistence
- Problem: Case History needs queryable graph nodes and matter-scoped indexes.
- Expected behavior: Add constraints and indexes for `ChangeSet`, `VersionSnapshot`, `SnapshotManifest`, `SnapshotEntityState`, `VersionChange`, support-use nodes, branch nodes, milestone nodes, and AI audit nodes.
- Implementation notes: Follow existing `ensure_indexes` and `save_complaint` graph patterns.
- Acceptance checks: Graph contract tests prove unique IDs, matter indexes, branch/snapshot/change-set relationships, snapshot manifest entries, and affected fact/evidence/authority edges.
- Dependencies: `CB-CH-101`, `CB-X-002`.
- Status: Partial
- Progress: Constraints/indexes and first graph edges exist for version nodes, manifests, entity states, branches, changes, support-use nodes, AI audit, export snapshots, and milestones. Add deeper affected-target edges and branch/merge graph contracts next.

## CB-CH-103 - Snapshot hashing service
- Priority: P0
- Area: Integrity
- Problem: Exports and restores need proof of exact document/support/QC/formatting state.
- Expected behavior: Compute deterministic hashes for work-product AST, support graph, QC state, formatting profile, and export render output.
- Implementation notes: Use canonical JSON ordering before hashing. Exclude volatile timestamps from semantic hashes.
- Acceptance checks: Identical legal state hashes identically; changed paragraph/support/QC/formatting state changes only the expected hash.
- Dependencies: `CB-CH-101`.
- Status: Done
- Progress: Deterministic hash helpers exist for document/work-product state, support graph, QC state, formatting profile, manifests, and export artifacts. Unit coverage proves identical state hashes identically and text/support/QC/formatting edits move the expected layer.

## CB-CH-104 - CaseHistoryService scaffold
- Priority: P0
- Area: Backend/service
- Problem: Version events are currently pushed ad hoc into complaint history arrays.
- Expected behavior: Add `CaseHistoryService` or `LegalVersionControlService` with methods for record change set, record operation, create snapshot, write manifest, get timeline, compare, restore, create branch, merge, tag milestone, and record AI audit.
- Implementation notes: Start with complaint UI calls, but service signatures use `matter_id`, `work_product_id`, and `branch_id`.
- Acceptance checks: Service-level tests record changes and snapshots without requiring frontend behavior.
- Dependencies: `CB-CH-101`, `CB-CH-102`, `CB-CH-103`.
- Status: Partial
- Progress: Required V0 service methods exist inside `CaseBuilderService`: root history, change-set recording, snapshot creation, manifest writing, compare, restore, AI audit, and latest export state. Extract to a dedicated `LegalVersionControlService` once branch/merge work begins.

## CB-CH-105 - Remove stored flat history models
- Priority: P1
- Area: Backend/data cleanup
- Problem: Existing `ComplaintHistoryEvent`, `WorkProductHistoryEvent`, and `DraftVersion` invite drift from the real version-control graph.
- Expected behavior: Remove them as stored truth before launch. If the UI needs simple cards, generate them as projections from `ChangeSet` and `VersionSnapshot`.
- Implementation notes: This is a pre-launch breaking cleanup. No user data migration is required.
- Acceptance checks: No service mutates stored flat history arrays; simple history endpoint responses are generated from Case History records.
- Dependencies: `CB-CH-104`.
- Status: Partial
- Progress: Complaint history routes now delegate to canonical work-product history. Compatibility flat models/arrays still exist and should be removed or converted to rebuildable projections before launch.

## CB-CH-106 - First-class support-use graph nodes
- Priority: P0
- Area: Graph/legal support
- Problem: Inline `fact_ids`, `evidence_ids`, and authority arrays make support diffing, branch-specific support, and graph traversal harder.
- Expected behavior: Create or promote `FactUse`, `EvidenceUse`, `AuthorityUse`, `CitationUse`, `ExhibitReference`, and `ElementSupport` as first-class nodes with branch/snapshot lifecycle fields.
- Implementation notes: Existing `EvidenceUse`, `CitationUse`, and `ExhibitReference` are good starts. Add `FactUse`, `AuthorityUse`, and `ElementSupport`; then deprecate raw arrays where these nodes can answer the question.
- Acceptance checks: A query can answer "show every unsupported allegation ever introduced" using graph nodes, not by scanning draft JSON.
- Dependencies: `CB-CH-102`, `CB-CE-008`, `CB-CE-009`.
- Status: Partial
- Progress: `LegalSupportUse`, `FactUse`, `AuthorityUse`, and `ElementSupport` graph constraints exist, and support-link actions create support-use nodes. Current-view arrays remain for UI convenience and need deeper deprecation.

## CB-CH-107 - Snapshot manifest and entity-state storage
- Priority: P0
- Area: Persistence/integrity
- Problem: Full JSON snapshots alone are fast to restore but weak for graph queries and large-matter diffs.
- Expected behavior: Each full snapshot writes a `SnapshotManifest` and `SnapshotEntityState` rows for meaningful legal objects, plus optional full-state blob/ref for restore speed.
- Implementation notes: Store inline state only when small. Use storage refs for large state objects.
- Acceptance checks: Compare can identify changed entity hashes before loading full state payloads.
- Dependencies: `CB-CH-103`, `CB-CH-106`.
- Status: Partial
- Progress: Snapshot creation now writes manifests and entity-state records with inline full state for V0 restore. Large snapshot storage refs and manifest-only diff optimization remain.

## CB-CH-108 - ChangeSet grouping model
- Priority: P0
- Area: Backend/history
- Problem: Timeline cards should represent legal actions, not noisy low-level operations.
- Expected behavior: Every user/system/AI action creates one `ChangeSet` with one or more `VersionChange` operations.
- Implementation notes: A paragraph edit with support loss should be one timeline event with separate text and support changes.
- Acceptance checks: Timeline list uses `ChangeSet`; detail view expands to target-level `VersionChange` records.
- Dependencies: `CB-CH-104`.
- Status: Done
- Progress: Timeline events are `ChangeSet` records with child `VersionChange` operations, source, actor type, snapshot pointer, and legal impact summary.

## CB-CH-109 - Derived read models and projections
- Priority: P1
- Area: API/frontend
- Problem: The UI needs fast simple lists without storing duplicate truth.
- Expected behavior: Generate timeline cards, export-history rows, AI history cards, support coverage series, and changed-since-export banners from Case History records.
- Implementation notes: Materialized projections are allowed later for performance if they can be rebuilt from source records.
- Acceptance checks: Projection rebuild produces the same timeline summaries from the same version graph.
- Dependencies: `CB-CH-108`.
- Status: Partial
- Progress: History, export-history, AI-audit, and changed-since-export responses derive from canonical version records. Support coverage series and audit-report projections remain.

### Epic CB-CH-200 - Automatic Snapshots and Event Capture

## CB-CH-201 - Snapshot on work-product creation
- Priority: P0
- Area: Backend/history
- Problem: New legal work products need a root snapshot for later restore, branch, and compare.
- Expected behavior: Creating a `WorkProduct` with `product_type=complaint` creates main branch and root snapshot after the profile AST is initialized.
- Implementation notes: Root branch name is profile-aware, such as `Main Complaint`.
- Acceptance checks: Creating a complaint-profile work product returns a current branch and root snapshot; history shows "Complaint created".
- Dependencies: `CB-CH-104`.
- Status: Done
- Progress: Creating a work product creates main branch state and a root Case History snapshot. Complaint-created facade paths synchronize into complaint-profile work products.

## CB-CH-202 - Capture block and AST edit changes
- Priority: P0
- Area: Backend/history
- Problem: Paragraph, caption, section, count, relief, signature, and certificate edits need before/after state.
- Expected behavior: Patch/create routes record `ChangeSet` and `VersionChange` entries with target type, operation, before, after, hashes, and legal impact.
- Implementation notes: Prioritize complaint-profile work-product operations: caption block, section block, count block, paragraph block, relief block, signature block, certificate block, and renumber.
- Acceptance checks: Tests prove paragraph text before/after is preserved and renumbering stores old-to-new mapping.
- Dependencies: `CB-CH-104`, `CB-CH-103`.
- Status: Partial
- Progress: Work-product and complaint facade create/patch flows record before/after state for major block and metadata edits. Relief/signature/certificate-specific diffs and paragraph renumber old-to-new mapping remain.

## CB-CH-203 - Capture support, citation, authority, and exhibit changes
- Priority: P0
- Area: Backend/history
- Problem: Legal support changes are as important as text changes.
- Expected behavior: Support mutation routes create support-use graph nodes and record fact, evidence, citation, authority, and exhibit link changes with affected support status.
- Implementation notes: Add unlink/update operations before UI exposes removal.
- Acceptance checks: Removing the only evidence link creates a warning in legal impact.
- Dependencies: `CB-CH-106`, `CB-CH-202`, `CB-CE-008`, `CB-CE-009`.
- Status: Partial
- Progress: Support-link routes record support-use changes and affected fact/evidence/authority summaries. Unlink/update operations and support-loss risk warnings remain.

## CB-CH-204 - Capture QC state changes
- Priority: P0
- Area: Backend/history
- Problem: QC runs and finding status changes must be auditable.
- Expected behavior: QC run creates snapshot and change summary; finding resolve/ignore/reopen records before/after status and warning counts.
- Implementation notes: Store finding IDs in `qc_warnings_added`, `qc_warnings_resolved`, `blocking_issues_added`, and `blocking_issues_resolved`.
- Acceptance checks: Tests prove a resolved finding appears as resolved in compare and timeline.
- Dependencies: `CB-CH-104`, `CB-CE-011`.
- Status: Partial
- Progress: Work-product QC runs and finding status changes record version events and warning IDs. Complaint-specific facade coverage exists through work-product sync; richer QC diff and reopen lifecycle coverage remain.

## CB-CH-205 - Autosnapshot grouping policy
- Priority: P1
- Area: Backend/product logic
- Problem: Snapshot per keystroke would make history noisy and expensive.
- Expected behavior: Introduce logical edit groups and snapshot cadence: every meaningful edit event, full snapshot on configured triggers, and every N events.
- Implementation notes: Frontend save debounce can stay simple; backend groups operations into `ChangeSet` and decides whether to create full snapshot or event-only change.
- Acceptance checks: Rapid paragraph patches can group under one edit session; AI accept/export/QC always create full snapshot.
- Dependencies: `CB-CH-108`, `CB-CH-202`.
- Status: Todo

### Epic CB-CH-300 - API Contracts

## CB-CH-301 - Snapshot endpoints
- Priority: P0
- Area: API
- Problem: Frontend needs list/detail/create snapshot routes.
- Expected behavior: Add canonical work-product snapshot list, detail, and manual create endpoints, with complaint route aliases if useful.
- Implementation notes: Use `/work-products/:workProductId/snapshots` as canonical. Complaint routes should delegate.
- Acceptance checks: Route contract tests fail if snapshot routes disappear.
- Dependencies: `CB-CH-104`, `CB-X-014`.
- Status: Done
- Progress: Canonical snapshot list/detail/create routes exist for work products, with complaint aliases delegating to the same handlers.

## CB-CH-302 - Compare endpoint
- Priority: P0
- Area: API/diff
- Problem: Users need a legal-specific compare, not a raw JSON diff.
- Expected behavior: Add compare endpoint with layer filters and structured response for text, support, authority, claims, QC, formatting, and export.
- Implementation notes: Start with text diff and summary counts in V0; add richer layers incrementally.
- Acceptance checks: Comparing root snapshot to current returns paragraph additions/changes and legal impact summary.
- Dependencies: `CB-CH-107`, `CB-CH-401`, `CB-CH-301`.
- Status: Partial
- Progress: Compare endpoint exists with layer parameters and V0 text/block diff. Support, authority, claim/element, QC, formatting, and export layers remain.

## CB-CH-303 - Restore endpoint
- Priority: P0
- Area: API/restore
- Problem: Rollback must be scoped and non-destructive.
- Expected behavior: Add restore endpoint supporting whole work-product and paragraph/block scopes first.
- Implementation notes: Restore creates `VersionChange` and new snapshot. It returns warning preview before destructive-looking support loss when `dry_run=true`.
- Acceptance checks: Restoring a paragraph keeps later history and records "Restored Paragraph X from Snapshot Y".
- Dependencies: `CB-CH-501`, `CB-CH-301`.
- Status: Partial
- Progress: Restore endpoint supports whole work-product/complaint and block/paragraph scopes with dry-run warnings and non-destructive restore events. Support/QC/formatting scoped restore remains.

## CB-CH-304 - Branch and milestone endpoints
- Priority: P1
- Area: API
- Problem: Legal strategy alternatives and filing states need first-class routes.
- Expected behavior: Add list/create/update branches and create/list milestones.
- Implementation notes: Branch names default to `Alternative Draft`, `TRO Version`, or user-provided name.
- Acceptance checks: Branch from snapshot creates independent current snapshot pointer without duplicating unrelated matter data.
- Dependencies: `CB-CH-601`, `CB-CH-701`.
- Status: Todo

## CB-CH-305 - Audit and export-history endpoints
- Priority: P1
- Area: API/audit
- Problem: AI and exports need filtered history views.
- Expected behavior: Add AI audit, export history, and full audit endpoints.
- Implementation notes: Return IDs, source refs, warning counts, and hashes; avoid raw prompt or text when caller lacks permission in future auth mode.
- Acceptance checks: Export history shows changed-since-export and artifact hash.
- Dependencies: `CB-CH-801`, `CB-CH-901`.
- Status: Partial
- Progress: AI audit and export-history endpoints exist, including changed-since-export and artifact hashes. Full audit/report endpoint remains.

### Epic CB-CH-400 - Legal Diff Engine

## CB-CH-401 - Text and paragraph diff V0
- Priority: P0
- Area: Diff
- Problem: Users need to see text changes between snapshots.
- Expected behavior: Diff paragraphs by stable IDs, detect added/removed/changed/moved paragraphs, and produce word-level text hunks.
- Implementation notes: Avoid implementing full ProseMirror diff until rich editor starts; use current structured AST.
- Acceptance checks: Tests cover paragraph update, create, delete/tombstone, move, and renumber.
- Dependencies: `CB-CH-103`.
- Status: Partial
- Progress: Stable-ID block text diff exists for added/removed/modified V0 comparisons. Word-level hunks, moved paragraph detection, delete tombstones, and renumber diffs remain.

## CB-CH-402 - Support, citation, and authority diff
- Priority: P1
- Area: Diff/legal support
- Problem: Support changes can make a draft legally weaker without obvious text changes.
- Expected behavior: Diff fact IDs, evidence uses, source spans, exhibit refs, citation uses, and authority refs.
- Implementation notes: Show support status transitions and warn when support is lost.
- Acceptance checks: Compare identifies citation added, evidence removed, and support status from supported to unsupported.
- Dependencies: `CB-CH-203`.
- Status: Todo

## CB-CH-403 - Claim, element, relief, and QC diff
- Priority: P1
- Area: Diff/legal structure
- Problem: Counts and rule findings need domain-aware comparison.
- Expected behavior: Diff count additions/removals, claim theory changes, element support, relief changes, finding status, and warning counts.
- Implementation notes: Reuse existing count health and QC categories.
- Acceptance checks: Compare flags new blocking finding and changed relief request.
- Dependencies: `CB-CH-204`, `CB-CE-007`, `CB-CE-011`.
- Status: Todo

## CB-CH-404 - Export/render diff
- Priority: P2
- Area: Diff/export
- Problem: A legally identical text edit can still alter page count, caption, signatures, or exhibit refs.
- Expected behavior: Compare export snapshots for page count, render profile hash, caption block, signature block, certificate, and exhibit references.
- Implementation notes: Start metadata-first; visual export diff can follow once PDF rendering matures.
- Acceptance checks: Compare since last export reports page count and warning changes.
- Dependencies: `CB-CH-901`, `CB-CE-013`, `CB-CE-015`.
- Status: Todo

### Epic CB-CH-500 - Restore and Safety

## CB-CH-501 - Whole work-product restore
- Priority: P0
- Area: Restore
- Problem: Users need a simple way back to a prior legal state.
- Expected behavior: Restore a full work product from snapshot into current branch as a new event and current snapshot.
- Implementation notes: Preserve IDs where possible; tombstone deleted-current nodes instead of hard deleting where graph nodes exist.
- Acceptance checks: Restore returns a work product matching snapshot state and history includes restore source.
- Dependencies: `CB-CH-303`.
- Status: Done
- Progress: Whole work-product/complaint restore applies snapshot state as a new current event and snapshot without erasing history.

## CB-CH-502 - Paragraph restore and copy-old-text mode
- Priority: P0
- Area: Restore
- Problem: Most practical undo needs to restore one paragraph, not the whole pleading.
- Expected behavior: Restore selected paragraph, or copy old paragraph text into current draft without touching support links.
- Implementation notes: Modes: `restore`, `restore_text_only`, `copy_old_text`.
- Acceptance checks: Restore paragraph preserves unrelated paragraphs and records support warnings.
- Dependencies: `CB-CH-501`.
- Status: Partial
- Progress: Paragraph/block restore exists. `restore_text_only` and `copy_old_text` modes remain.

## CB-CH-503 - Support and QC scoped restore
- Priority: P1
- Area: Restore/legal support
- Problem: Users may only want old evidence/citation links or QC decisions.
- Expected behavior: Restore citation links, evidence links, authority links, exhibit links, formatting profile, or QC decisions by scope.
- Implementation notes: Dry-run response should list current support that would be removed.
- Acceptance checks: Restoring citation links does not change paragraph text.
- Dependencies: `CB-CH-402`, `CB-CH-502`.
- Status: Todo

## CB-CH-504 - Restore warning preview
- Priority: P0
- Area: UX/safety
- Problem: Restore can accidentally remove current support or reintroduce warnings.
- Expected behavior: Before applying restore, show affected targets, removed support, unresolved citations, and warnings introduced.
- Implementation notes: API supports `dry_run=true`; UI uses the same response in `RollbackDialog`.
- Acceptance checks: Restoring old unsupported text warns before apply.
- Dependencies: `CB-CH-303`, `CB-CH-503`.
- Status: Partial
- Progress: Restore supports `dry_run=true` and returns first-pass warnings. Rich support-loss, citation, authority, and QC warnings remain.

### Epic CB-CH-600 - Branching and Merge Cards

## CB-CH-601 - Main branch and alternative branch model
- Priority: P1
- Area: Branching
- Problem: Users need legal strategy alternatives without copying files manually.
- Expected behavior: Every versioned work product has a main branch. Users can create alternatives from any snapshot.
- Implementation notes: Branch names use legal language: `TRO Version`, `Short Filing Version`, `Aggressive Version`, `Settlement Draft`.
- Acceptance checks: Branch manager can list, open, rename, archive, and compare branches.
- Dependencies: `CB-CH-304`.
- Status: Todo

## CB-CH-602 - Branch-aware save and snapshot pointers
- Priority: P1
- Area: Backend/branching
- Problem: Edits must apply to the current branch without corrupting main.
- Expected behavior: Work-product state loads from selected branch and each branch has its own current snapshot pointer.
- Implementation notes: In V0.1, branch state may copy full work-product state on branch creation; later storage can dedupe by snapshots.
- Acceptance checks: Editing an alternative does not change main branch current snapshot.
- Dependencies: `CB-CH-601`.
- Status: Todo

## CB-CH-603 - Merge selected changes cards
- Priority: P2
- Area: Merge
- Problem: Legal users need selected imports, not raw Git merges.
- Expected behavior: Cards support bring over count, paragraphs, prayer for relief, citation improvements, evidence links, or QC decisions from another branch.
- Implementation notes: Start with non-conflicting target replacement and text-only merge.
- Acceptance checks: User can bring Count III from another branch into main as a new merge event.
- Dependencies: `CB-CH-602`, `CB-CH-402`, `CB-CH-403`.
- Status: Todo

## CB-CH-604 - Conflict resolver
- Priority: P2
- Area: Merge/conflict
- Problem: Same paragraph/support target may differ between branches.
- Expected behavior: Show current, incoming, AI comparison when available, choose current, choose incoming, combine, or rewrite merged version.
- Implementation notes: Do not require AI provider; AI comparison is optional/template if disabled.
- Acceptance checks: Conflict resolution creates `ConflictResolution` record and merged snapshot.
- Dependencies: `CB-CH-603`, `CB-CH-801`.
- Status: Todo

### Epic CB-CH-700 - Milestones and Review States

## CB-CH-701 - Milestone model and UI
- Priority: P1
- Area: Milestones
- Problem: Legal lifecycle states need durable tags on exact versions.
- Expected behavior: Users can tag snapshots as draft started, facts complete, claims complete, QC reviewed, court ready, exported, filed, or served.
- Implementation notes: Filing-related milestones must remain review-needed unless supported by export hash and user confirmation.
- Acceptance checks: Milestone bar shows exact snapshot and date for each milestone.
- Dependencies: `CB-CH-304`.
- Status: Todo

## CB-CH-702 - Filing-ready guardrails
- Priority: P1
- Area: Legal safety
- Problem: "Court ready" can be misunderstood as legal advice or successful filing.
- Expected behavior: Court-ready milestone means user-reviewed export package, not guaranteed compliance.
- Implementation notes: Tie milestone to QC status, export hash, warnings, and human review label.
- Acceptance checks: UI cannot mark final filing packet without snapshot hash and review status.
- Dependencies: `CB-CH-701`, `CB-CH-901`, `CB-V1-013`.
- Status: Todo

### Epic CB-CH-800 - AI Edit Audit

## CB-CH-801 - AI audit event model
- Priority: P0
- Area: AI audit
- Problem: AI actions are currently recorded as lightweight history messages.
- Expected behavior: Every AI command records command, provider mode, prompt template ID, model, input snapshot, facts/evidence/authority used, output, user action, inserted text, and warnings.
- Implementation notes: Provider-free template mode still records audit events with no inserted unsupported text.
- Acceptance checks: Running existing complaint AI command creates `AIEditAudit` plus version change.
- Dependencies: `CB-CH-101`, `CB-X-004`, `CB-X-005`.
- Status: Partial
- Progress: AI commands now create `AIEditAudit` records and version changes even in template/disabled mode. Source-context completeness and live accept/reject/edit lifecycle remain.

## CB-CH-802 - AI accept/reject/edit flow
- Priority: P1
- Area: AI UX/backend
- Problem: The system must know whether AI output was accepted, rejected, or edited by the user.
- Expected behavior: AI result lifecycle records accepted/rejected/edited and final inserted text.
- Implementation notes: Existing template command can remain disabled; live provider integrations must use this lifecycle before insertion.
- Acceptance checks: AI draft rejected creates audit record but no AST change; accepted creates snapshot.
- Dependencies: `CB-CH-801`, `CB-CE-016`.
- Status: Todo

## CB-CH-803 - Undo AI rewrite
- Priority: P1
- Area: Restore/AI
- Problem: Users need a direct trust-preserving undo for AI edits.
- Expected behavior: AI history card offers compare to pre-AI version, restore pre-AI version, and restore affected paragraph/section only.
- Implementation notes: Use input snapshot from `AIEditAudit`.
- Acceptance checks: Undo AI rewrite restores only affected target and records restore event.
- Dependencies: `CB-CH-502`, `CB-CH-801`.
- Status: Todo

### Epic CB-CH-900 - Export Version Locking

## CB-CH-901 - Immutable export snapshot
- Priority: P0
- Area: Export/integrity
- Problem: Exports need to prove exactly what draft was rendered.
- Expected behavior: Work-product export creates a full export snapshot before artifact generation and links artifact to snapshot.
- Implementation notes: Extend current `ExportArtifact` with snapshot/hash fields.
- Acceptance checks: Export history shows snapshot ID, page count, QC status, artifact hash, and generated timestamp.
- Dependencies: `CB-CH-103`, `CB-CE-014`, `CB-CE-015`.
- Status: Done
- Progress: Work-product export creates an export snapshot and links artifact metadata to `snapshot_id`, artifact hash, render profile hash, QC status, and immutability.

## CB-CH-902 - Changed since last export indicator
- Priority: P0
- Area: Export UX
- Problem: Users need to know whether the draft changed after export.
- Expected behavior: Header/export panel shows "Draft changed since last export" when current snapshot hashes differ from latest export snapshot.
- Implementation notes: Compare document, support, QC, and formatting hashes.
- Acceptance checks: Editing paragraph after export flips indicator to true; exporting again resets it.
- Dependencies: `CB-CH-901`.
- Status: Done
- Progress: Export history compares current document/support/QC/formatting hashes against the latest export snapshot, and the Complaint export panel shows changed-since-export state.

## CB-CH-903 - Export hash report
- Priority: P1
- Area: Audit/export
- Problem: Filing packets need a concise proof report.
- Expected behavior: Generate export history report with artifact hash, snapshot hash, render profile, page count, warnings, and linked milestone.
- Implementation notes: Report can start as JSON/HTML, then PDF later.
- Acceptance checks: Export report includes all generated artifacts for a complaint.
- Dependencies: `CB-CH-901`, `CB-CH-1002`.
- Status: Todo

### Epic CB-CH-1000 - Reports and Trust Views

## CB-CH-1001 - Timeline panel V0
- Priority: P0
- Area: Frontend/history
- Problem: Existing history is not a first-class timeline.
- Expected behavior: Add a Case History route and panel with filters: all, text, AI, facts, evidence, authority, QC, exports, milestones.
- Implementation notes: Use `ChangeSet` projections from the start. Do not build a new UI dependency on stored `ComplaintHistoryEvent`.
- Acceptance checks: Timeline cards show time, actor, change type, summary, affected target, risk indicator, compare, and restore actions.
- Dependencies: `CB-CH-108`, `CB-CH-109`, `CB-CH-301`.
- Status: Partial
- Progress: Complaint workspace now has a Case History panel using `ChangeSet` data, with timeline, manual snapshot, compare, and restore actions. Filters and richer event detail remain.

## CB-CH-1002 - Audit reports
- Priority: P1
- Area: Reports
- Problem: Legal trust requires exportable summaries of history and support.
- Expected behavior: Generate draft history report, AI edit report, evidence support report, citation report, rule compliance report, and export hash report.
- Implementation notes: Reports should be generated from version events and snapshots, not scraped from UI state.
- Acceptance checks: Report endpoints return deterministic, matter-scoped payloads.
- Dependencies: `CB-CH-305`, `CB-CH-801`, `CB-CH-901`.
- Status: Todo

## CB-CH-1003 - Support coverage over time
- Priority: P2
- Area: Analytics
- Problem: Users need to see whether a draft is getting safer.
- Expected behavior: Graph unsupported allegations, unresolved citations, QC warnings, linked facts, linked exhibits, and evidence count over time.
- Implementation notes: Use snapshot summaries for performance.
- Acceptance checks: Support graph updates after support link, QC run, and citation insertion snapshots.
- Dependencies: `CB-CH-402`, `CB-CH-204`.
- Status: Todo

## CB-CH-1004 - Snapshot viewer
- Priority: P1
- Area: Frontend/history
- Problem: Users need detail before compare, restore, branch, milestone, or export.
- Expected behavior: Snapshot viewer shows title, actor, date, parent snapshot, branch, QC status, support coverage, export artifacts, and actions.
- Implementation notes: Fit inside current complaint workbench without overwhelming the three-pane editor.
- Acceptance checks: Snapshot viewer can open from timeline, export history, branch manager, and milestone bar.
- Dependencies: `CB-CH-301`, `CB-CH-1001`.
- Status: Todo

## CB-CH-1005 - Compare modal and legal diff viewer
- Priority: P0
- Area: Frontend/diff
- Problem: Users need a focused compare experience.
- Expected behavior: Modal shows selected snapshot vs current, with toggles for text, facts, evidence, authority, QC, formatting, and export.
- Implementation notes: V0 can ship text layer first with disabled/explained toggles for future layers.
- Acceptance checks: Text diff is readable on desktop and tablet and does not overlap controls.
- Dependencies: `CB-CH-302`, `CB-CH-401`.
- Status: Partial
- Progress: Text compare is wired in the Case History panel. Dedicated modal, layer toggles, and richer legal diff viewer remain.

### Epic CB-CH-1100 - Quality, Security, and Performance

## CB-CH-1101 - Matter isolation tests for history
- Priority: P0
- Area: Privacy
- Problem: Version snapshots can contain sensitive full draft state.
- Expected behavior: Cross-matter access to snapshots, diffs, restores, branches, AI audit, and exports is rejected.
- Implementation notes: Extend existing matter isolation backlog with history-specific routes.
- Acceptance checks: Matter A snapshot cannot be fetched, compared, restored, or exported through Matter B.
- Dependencies: `CB-X-002`, `CB-X-016`, `CB-CH-301`.
- Status: Todo

## CB-CH-1102 - Snapshot storage performance and retention
- Priority: P1
- Area: Performance/storage
- Problem: Full snapshots can become large on long matters.
- Expected behavior: Store inline state for small snapshots and object/blob refs for larger snapshots; keep bounded list endpoints.
- Implementation notes: Use content hashes and optional storage refs compatible with local/R2 artifact lifecycle.
- Acceptance checks: Large fixture history remains paginated and snapshot detail stays under agreed payload limits.
- Dependencies: `CB-X-010`, `CB-X-017`, `CB-CH-103`.
- Status: Todo

## CB-CH-1103 - Logging without sensitive text
- Priority: P0
- Area: Privacy/observability
- Problem: Version systems are tempting places to log full before/after text.
- Expected behavior: Operational logs include IDs, counts, durations, hash prefixes, and statuses, not confidential draft text.
- Implementation notes: Tests or review checklist catch unsafe logging patterns.
- Acceptance checks: Snapshot, compare, restore, AI, and export logs do not include paragraph text or prompts.
- Dependencies: `CB-X-019`, `CB-CH-104`.
- Status: Todo

## CB-CH-1104 - End-to-end history smoke
- Priority: P0
- Area: Quality
- Problem: Case History only matters if it works across edit, support, QC, export, compare, restore, and audit.
- Expected behavior: Smoke flow creates complaint, edits paragraph, links evidence, inserts citation, runs QC, exports, compares to prior snapshot, restores paragraph, and verifies changed-since-export.
- Implementation notes: Keep deterministic and provider-free by default.
- Acceptance checks: Smoke fails on missing version events, broken hashes, incorrect restore, or lost export snapshot.
- Dependencies: `CB-CH-201` through `CB-CH-902`.
- Status: Todo

## Optimized Release Slices

### V0.0 - Landed: Graph-Native History Foundation

Goal: Make the existing Complaint workspace safer immediately while putting all durable history under `WorkProduct`.

Landed:

- `CB-CH-103` Snapshot hashing service
- `CB-CH-108` ChangeSet grouping model
- `CB-CH-201` Snapshot on work-product creation
- `CB-CH-301` Snapshot endpoints
- `CB-CH-501` Whole work-product restore
- `CB-CH-901` Immutable export snapshot
- `CB-CH-902` Changed since last export indicator

Partially landed and carried forward:

- `CB-CH-001`, `CB-CH-101`, `CB-CH-102`, `CB-CH-104`, `CB-CH-105`, `CB-CH-106`, `CB-CH-107`, `CB-CH-109`
- `CB-CH-202`, `CB-CH-203`, `CB-CH-204`
- `CB-CH-302`, `CB-CH-303`, `CB-CH-305`, `CB-CH-401`, `CB-CH-502`, `CB-CH-504`, `CB-CH-801`, `CB-CH-1001`, `CB-CH-1005`

### V0.1 - Next: Release-Hardening History and Legal Diff

Goal: Finish the trust-critical parts of the current branch before building alternatives.

Build in this order:

1. `CB-CH-1101` Matter isolation tests for every history/snapshot/compare/restore/export/AI-audit endpoint.
2. `CB-CH-105` Remove or fully demote stored flat history models and stop mutating flat history arrays.
3. `CB-CH-203` Complete support/citation/authority/exhibit change capture, including unlink/update operations.
4. `CB-CH-402` Add support, citation, and authority diff layers.
5. `CB-CH-403` Add claim, element, relief, and QC diff summaries.
6. `CB-CH-502` Add `restore_text_only` and `copy_old_text` modes.
7. `CB-CH-503` Add support and QC scoped restore.
8. `CB-CH-504` Upgrade dry-run warnings using graph support inspection.
9. `CB-CH-1004` Add snapshot detail viewer.
10. `CB-CH-1005` Turn current compare panel into the full compare modal with layer toggles.
11. `CB-CH-1103` Add sensitive-log guardrails/checklist.
12. `CB-CH-1104` Add end-to-end Case History smoke.

V0.1 ship criteria:

- History routes are matter-isolated by test, not just by convention.
- Flat history is no longer a persistence truth.
- Support/citation/QC diffs are visible enough to catch legal risk.
- Restore preview warns about support loss and unresolved citations.
- Users can restore text only, copy old text, or restore support/QC scopes.
- Smoke covers edit -> support -> QC -> export -> edit -> changed-since-export -> compare -> dry-run restore -> apply restore.

### V0.2 - Alternatives, Milestones, and Audit Reports

Goal: Let users try legal strategies without copy-paste drafts.

Build:

- `CB-CH-205` Autosnapshot grouping policy
- `CB-CH-304` Branch and milestone endpoints
- `CB-CH-601` Main branch and alternative branch model
- `CB-CH-602` Branch-aware save and snapshot pointers
- `CB-CH-701` Milestone model and UI
- `CB-CH-702` Filing-ready guardrails
- `CB-CH-305` Full audit endpoint
- `CB-CH-903` Export hash report
- `CB-CH-1002` Audit reports
- `CB-CH-1003` Support coverage over time

V0.2 ship criteria:

- Users can create an alternative from any snapshot.
- Branch current pointers are independent.
- Milestones tag exact snapshots.
- Export/audit reports can be generated.
- Filing-ready language remains review-needed and hash-backed.

### V0.3 - Merge Cards and Conflict Resolution

Goal: Safely bring selected legal changes across alternatives.

Build:

- `CB-CH-404` Export/render diff
- `CB-CH-603` Merge selected changes cards
- `CB-CH-604` Conflict resolver
- `CB-CH-1102` Snapshot storage performance and retention
- `CB-CH-802` AI accept/reject/edit flow
- `CB-CH-803` Undo AI rewrite

V0.3 ship criteria:

- Users can bring selected counts, paragraphs, citations, evidence links, or relief across alternatives.
- Conflicts show current and incoming legal state with explicit user choice.
- AI edits can be undone directly through Case History.
- Large history remains usable.

### V1 - Collaboration and Review Packets

Goal: Multi-user legal review on top of Case History.

- Connect to `CB-V1-001` multi-user sharing.
- Connect to `CB-V1-002` role-based access controls.
- Connect to `CB-V1-003` attorney review mode.
- Connect to `CB-V1-004` redlines and comments.
- Connect to `CB-V1-005` audit log.
- Add review packets as the legal equivalent of pull requests.

V1 ship criteria:

- Attorney/user comments and approvals attach to snapshots.
- Review packets compare proposed changes against current branch.
- Actor identity is enforced in every version event.
- Filed/served milestones are auditable and access-controlled.

## Next Sprint Plan

Sprint 1 is now done as a first build slice. Sprint 2 should harden the release path, not jump straight to branches.

1. Add matter-isolation tests for all canonical and complaint-alias Case History endpoints.
2. Remove/demote flat history mutations so `ChangeSet` is the only history truth.
3. Add support/citation/authority diff layers and support-loss legal impact warnings.
4. Add restore text-only, copy-old-text, citation-link, evidence-link, authority-link, formatting, and QC scopes.
5. Add snapshot detail viewer and full compare modal.
6. Add end-to-end smoke for the full V0 flow.
7. Extract the version-control methods out of `CaseBuilderService` only if the service boundary starts slowing development; otherwise keep the next slice behavior-first.

## Non-Goals for V0

- Multi-user collaboration.
- Attorney approval workflows.
- Rich ProseMirror document diff.
- Visual PDF redline diff.
- Complex automatic merge.
- E-filing integration.
- Case law citation currentness beyond existing ORSGraph authority support.

## Definition of Done

A Case History ticket is done only when:

- Backend and frontend DTOs agree.
- Route contract tests cover the API shape.
- Matter isolation is enforced.
- Timeline events are `ChangeSet` records with actor, reason/source, grouped operations, and legal impact summary.
- Version changes include target, operation, before/after hashes, before/after state where appropriate, and legal impact summary.
- Support relationships that matter to legal trust are first-class graph nodes.
- Snapshot hashes are deterministic.
- Restore creates new history instead of erasing history.
- Export creates immutable snapshot and artifact hash.
- UI labels are user-friendly and avoid Git terms.
- Provider-free AI mode remains explicit and safe.
- Tests or checklist confirm no sensitive full text is logged.
