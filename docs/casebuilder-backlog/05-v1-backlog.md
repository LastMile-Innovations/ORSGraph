# 05 - V1 Backlog

V1 turns CaseBuilder from a single-user internal workbench into a reviewable legal collaboration product.

## CB-V1-001 - Multi-user matter sharing
- Priority: P1
- Area: Collaboration
- Problem: V0 is single-user/internal.
- Expected behavior: Users can share a matter with role-based permissions.
- Implementation notes: Requires authentication and authorization model.
- Acceptance checks: Shared users can only access authorized matters and actions.
- Dependencies: Auth/identity decision.
- Status: Deferred

## CB-V1-002 - Role-based access controls
- Priority: P1
- Area: Privacy/security
- Problem: Sensitive files need access controls beyond optional API key.
- Expected behavior: Roles for owner, reviewer, attorney, viewer, and admin.
- Implementation notes: Enforce on backend, not only frontend.
- Acceptance checks: Unauthorized matter/document/draft access returns 403.
- Dependencies: `CB-V1-001`.
- Status: Deferred

## CB-V1-003 - Attorney review mode
- Priority: P1
- Area: Review
- Problem: Users need a way to hand off work for legal review without blurring AI output and legal advice.
- Expected behavior: Attorney reviewers can comment, approve, redline, and mark legal-review status.
- Implementation notes: Add explicit review lifecycle and audit trail.
- Acceptance checks: Draft and QC status show attorney review state distinctly from AI checks.
- Dependencies: `CB-V1-001`, `CB-V1-002`.
- Status: Deferred

## CB-V1-004 - Redline drafts and review comments
- Priority: P2
- Area: Collaboration
- Problem: Draft review needs structured comments and revisions.
- Expected behavior: Inline comments, suggested edits, redlines, resolve/reopen, and version snapshots.
- Implementation notes: Use stable WorkProduct AST block IDs and text ranges for anchors. Legacy DraftSection/DraftParagraph IDs can be supported only as projections during migration.
- Acceptance checks: Comments survive edits and link to WorkProduct snapshots.
- Dependencies: Stable WorkProduct AST editor model.
- Status: Deferred

## CB-V1-005 - Audit log
- Priority: P1
- Area: Compliance/privacy
- Problem: Sensitive legal work needs traceability.
- Expected behavior: Log uploads, deletes, shares, AI actions, exports, approvals, and major edits.
- Implementation notes: Store `AuditEvent` records with actor, timestamp, action, target, and summary without leaking full confidential text unnecessarily.
- Acceptance checks: Matter owner can view audit events and exported/deleted activity.
- Dependencies: Auth model.
- Status: Deferred

## CB-V1-006 - Court-rule integration
- Priority: P1
- Area: Legal authority
- Problem: ORS authority alone is insufficient for filings and deadlines.
- Expected behavior: Integrate ORCP, UTCR/local rules, court forms, formatting, service, and filing requirements.
- Implementation notes: Treat rules as source-backed legal graph material with currentness. The Oregon Circuit Civil Complaint ORCP/UTCR seed pack is tracked in `CB-CE-010`; this V1 item remains the broader court-rule corpus and product integration.
- Acceptance checks: Deadline and filing checklist can cite court-rule authority.
- Dependencies: Court-rule corpus ingestion, `CB-CE-010`.
- Status: Deferred

## CB-V1-007 - Case-law integration
- Priority: P1
- Area: Legal authority
- Problem: Claims, defenses, and motions often require case law.
- Expected behavior: Search, cite, and verify case law with quotations, holdings, and currentness warnings.
- Implementation notes: Requires licensed or public case-law source strategy.
- Acceptance checks: Citation checker can distinguish statutes, rules, and cases.
- Dependencies: Case-law corpus decision.
- Status: Deferred

## CB-V1-008 - Filing workflow
- Priority: P1
- Area: Filing
- Problem: V0/V0.2 can package documents but not file them.
- Expected behavior: Filing checklist, final QC, packet validation, court portal instructions or integration where allowed.
- Implementation notes: Do not automate filing without explicit jurisdiction-specific safety checks.
- Acceptance checks: User can confirm packet readiness and understand remaining manual filing steps.
- Dependencies: Export packet, court-rule integration.
- Status: Deferred

## CB-V1-009 - Advanced strategy scoring
- Priority: P2
- Area: Strategy
- Problem: Basic scores need stronger evidence and authority explanations.
- Expected behavior: Transparent claim/defense/damages/deadline/settlement leverage scoring with source-backed rationale.
- Implementation notes: Keep scores advisory and reviewable; never hide weak support.
- Acceptance checks: Every score traces to specific facts, evidence, authorities, and gaps.
- Dependencies: Mature QC and graph data.
- Status: Deferred

## CB-V1-010 - Data retention and matter deletion UI
- Priority: P1
- Area: Privacy
- Problem: Sensitive matter data needs user-facing retention controls.
- Expected behavior: Export, archive, delete, and retention settings with clear consequences.
- Implementation notes: Deletion must remove local files and graph nodes or tombstone according to policy; show which generated exports and upload objects are affected.
- Acceptance checks: Deleted matter is inaccessible and local uploads are removed or securely tombstoned.
- Dependencies: Audit log and storage policy.
- Status: Deferred

## CB-V1-011 - Production auth and identity model
- Priority: P1
- Area: Security
- Problem: Optional API key protection is not enough for a production legal workbench.
- Expected behavior: Choose and implement identity, sessions, matter ownership, service auth, and backend authorization enforcement.
- Implementation notes: Keep V0 single-user/internal until this lands; do not expose multi-user sharing without backend authorization.
- Acceptance checks: Every matter, document, draft, export, and finding endpoint enforces actor authorization and returns 401/403 consistently.
- Dependencies: Product auth decision.
- Status: Deferred

## CB-V1-012 - Matter archive and legal hold states
- Priority: P2
- Area: Privacy/lifecycle
- Problem: Delete is not the only lifecycle state for sensitive legal matters.
- Expected behavior: Add active, archived, deleted/tombstoned, and legal-hold states with clear UI and API behavior.
- Implementation notes: Legal hold blocks destructive deletion; archive hides matter from active workflows but preserves graph and files.
- Acceptance checks: Archived matters are read-only or limited as configured, deleted matters are inaccessible, and legal-hold matters cannot be purged.
- Dependencies: `CB-V1-005`, `CB-V1-010`, `CB-V1-011`.
- Status: Deferred

## CB-V1-013 - Filing workflow safety gate
- Priority: P1
- Area: Filing
- Problem: Export packets can look filing-ready before court-rule, local-rule, and attorney-review integrations exist.
- Expected behavior: Filing workflow distinguishes generated packet, review-needed packet, and user-final packet, with explicit manual filing checklist.
- Implementation notes: No court e-filing automation until jurisdiction-specific rules and safety checks are available.
- Acceptance checks: User cannot confuse an unchecked generated packet with an electronically filed court submission.
- Dependencies: `CB-V02-005`, `CB-CE-018`, `CB-V1-006`, `CB-V1-008`.
- Status: Deferred
