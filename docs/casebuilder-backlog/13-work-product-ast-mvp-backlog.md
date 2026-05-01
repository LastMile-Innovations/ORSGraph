# 13 - WorkProduct AST MVP Backlog

Last updated: 2026-05-01

This file is the optimized implementation backlog for the WorkProduct AST requirement set. It compresses the full `REQ-AST-*` list into ship-sized requirements while keeping traceability to the original requirement families.

The current target is MVP plus safe Phase 2/3 scaffolding. The AST is canonical for WorkProduct documents; legacy `blocks`, `anchors`, and `findings` are compatibility projections while callers migrate.

## Status Legend

- `Done`: Implemented and verified for the MVP contract.
- `Partial`: Some model, route, service, test, or projection exists, but the requirement is not production-complete.
- `Todo`: Not implemented.
- `Deferred`: Intentionally out of current MVP scope.
- `Blocked`: Needs an external dependency or product decision.

## Current Verification

- `cargo test -p orsgraph-api` passed on 2026-05-01.
- `pnpm run check` from `frontend/` passed on 2026-05-01.
- Frontend check still reports unrelated existing warnings in `frontend/components/orsg/admin/admin-job-detail-client.tsx`.

## Requirement Family Rollup

| Family | Source REQs | Optimized Requirement | Status | Notes |
| --- | --- | --- | --- | --- |
| Core AST foundation | `REQ-AST-001..010` | Store each WorkProduct as canonical `WorkProductDocument.document_ast`, validate before write/export/check, and keep it document-type neutral. | Done | Canonical AST is implemented with direct save, patch save, validate, conversion, QC, preview, and export paths calling validation. |
| Stable identity | `REQ-AST-ID-*` | Stable IDs for blocks, citations, links, exhibits, findings, optional sentences, and tombstoned deletes. | Partial | MVP fields and tombstoning exist. Rich editor preservation and full deleted-node history semantics remain. |
| Document metadata | `REQ-AST-META-*` | WorkProduct metadata includes type, title, jurisdiction/court/case data, templates, profiles, status, authorship, and timestamps. | Done | DTOs were extended non-breakingly. Some references remain optional until profile registries mature. |
| Block model | `REQ-AST-BLOCK-*` | AST supports legal block shapes through stable JSON block records and `type` constants. | Done | MVP block types are supported without a breaking Rust enum, with registry-backed invariants for canonical type, children, titles, numbering, sentence IDs, and count numbers. |
| Caption block | `REQ-AST-CAPTION-*` | Caption data can represent court, county, case number, title, parties, roles, and reusable court-paper layout. | Partial | Data shape and rendering exist. Court-specific layout validation remains Phase 2. |
| Section/count blocks | `REQ-AST-SECTION-*`, `REQ-AST-COUNT-*` | Sections and counts support typed structure, child blocks, targets, elements, requested relief, links, scoring, and diff/restore. | Partial | Core fields, children, links, validation, diff, and restore exist. Completeness scoring is not complete. |
| Paragraph/sentence model | `REQ-AST-PARA-*`, `REQ-AST-SENT-*` | Paragraphs and sentence-compatible blocks track roles, numbering, support, AI/protected status, and sentence support where enabled. | Partial | Paragraph fields and sentence IDs exist. Full sentence segmentation/checking is Phase 2. |
| Numbering/cross references | `REQ-AST-NUM-*` | Renumber numbered paragraphs, detect duplicate/skipped numbers, preserve mappings, and warn on unsafe cross-reference updates. | Partial | Renumber operation and validation are present. Cross-reference parsing/updating needs deeper coverage. |
| Link model | `REQ-AST-LINK-*` | Structured links attach AST nodes/ranges to facts, evidence, authority, exhibits, parties, events, documents, and notes. | Partial | Structured links, patch ops, validation, graph contract, and frontend normalization exist. Inline chips and inspector UX are incomplete. |
| Fact/evidence support | `REQ-AST-FACT-*`, `REQ-AST-EVID-*` | Blocks and ranges link to facts/evidence with relation/confidence and support warnings. | Partial | Link records, support-link routes, and block-level unsupported factual assertion QC exist. Evidence matrix and sentence-level support remain incomplete. |
| Citation uses | `REQ-AST-CITE-*` | Store citation uses with raw/normalized text, target, pinpoint, status, resolver hooks, warnings, and versioning. | Partial | Citation model, patch ops, validation, markdown preservation, deterministic citation-use extraction, and Oregon authority canonicalization exist. Full resolver/currentness lifecycle remains. |
| Exhibit references | `REQ-AST-EXH-*` | Store exhibit references, detect missing/orphan references, and support safe renumbering/export use. | Partial | Exhibit model, patch ops, validation, and markdown preservation exist. Full exhibit lifecycle and renumbering UI remain. |
| Rule findings | `REQ-AST-RULE-*` | Rule findings are AST-targeted, structured, statusful, versioned, and produced from AST inspection. | Partial | MVP finding model and AST-only rule engine cover required blocks, unsupported factual blocks, and citation review findings. Universal rule-pack engine is not complete. |
| Patches | `REQ-AST-PATCH-*` | All mutations are representable as `AstPatch`, atomic, validated, and version-recorded. | Done | Patch service uses clone-apply-validate semantics and stale base hash rejection. |
| Operations | `REQ-AST-OP-*` | Support insert/update/delete/move/split/merge/renumber, link/citation/exhibit/finding ops, templates, and conversions. | Partial | Core mutation ops are implemented. Conversion is intentionally service/API-only, not a mutating patch op. Template expansion remains basic. |
| Validation | `REQ-AST-VAL-*` | Validator checks schema, IDs, parents, cycles, order, required metadata/blocks, references, and structured findings. | Partial | MVP validation exists with blocking/warning distinctions, registry-backed MVP references, and rule finding targets for blocks, links, citations, exhibits, document, and formatting records. Live external registry checks remain. |
| Rich text editor | `REQ-AST-RTE-*` | Rich editor renders AST and saves legal nodes/marks/chips as patches while preserving IDs. | Todo | Deferred to Phase 2 shared editor integration. |
| Markdown | `REQ-AST-MD-*` | Markdown is an editable AST projection with frontmatter/comments preserving metadata where possible. | Partial | Backend conversion, frontend helpers, parsed frontmatter, hidden metadata comments, sidecar rehydration, and round-trip tests exist. Advanced legal metadata preservation remains. |
| Preview/rendering | `REQ-AST-RENDER-*` | Preview, HTML, court paper, PDF, DOCX, plain text, and Markdown consume AST deterministically. | Partial | HTML, plain text, and Markdown are AST-backed. PDF/DOCX are deterministic placeholders pending Phase 2 renderers. |
| Export | `REQ-AST-EXPORT-*` | Export consumes AST, validates first, warns on unresolved legal issues, records artifact metadata, and supports clean/review/internal copies. | Partial | Markdown/HTML/plain text and warning paths are AST-backed, including blocking findings, unresolved citations, and non-attached exhibits. Production PDF/DOCX and filing packet export remain. |
| Rule engine | `REQ-AST-RULEENGINE-*` | Rule engine consumes AST, selected rule packs, creates findings, and applies safe auto-fixes as patches. | Partial | MVP AST rule checks exist. Rule-pack integration and auto-fix lifecycle are incomplete. |
| AI patches | `REQ-AST-AI-*` | AI receives AST context and returns reviewable `AstPatch` proposals with support/citation warnings. | Partial | Provider-free patch scaffold exists. Full review UI, audit settings, and context packaging are Phase 2. |
| Version control | `REQ-AST-VERSION-*` | Patches create version changes; snapshots, diffs, rollback, branches, and export artifacts operate on AST layers. | Partial | Basic snapshot/diff/restore/export integration exists. Branch/merge and full layer diff UX remain. |
| Graph persistence | `REQ-AST-GRAPH-*` | WorkProduct AST nodes and relationships materialize to graph nodes/edges where useful. | Partial | Graph contract and existing materialization cover MVP surfaces. Scale-mode subdocument decisions remain. |
| API | `REQ-AST-API-*` | Existing AST routes remain, explicit matter-scoped GET/PATCH AST routes exist, and errors are structured/transaction-safe. | Done | Backend and frontend helpers expose direct AST get/patch plus patch/validate/convert/export routes. |
| Backend modules | `REQ-AST-MOD-*` | AST logic is split into dedicated backend service modules. | Done | Required modules exist, including placeholder PDF/DOCX renderers for MVP boundaries. |
| QC/safety | `REQ-AST-QC-*` | Detect unsupported facts, unresolved/stale citations, exhibit problems, numbering/cross-ref errors, orphan links, and AI unsupported text. | Partial | Validation/QC now covers block-level unsupported facts, unresolved citation review findings, exhibit status warnings, orphan records, numbering basics, and export-blocking findings. Sentence-level fact support and AI unsupported text enforcement remain. |
| MVP | `REQ-AST-MVP-*` | Ship the core type model, patching, validation, Markdown/HTML/plain conversion, basic version snapshot/diff, and basic export. | Done | MVP service, route, frontend, and contract coverage has landed. Some MVP-plus legal workflows are tracked separately below. |
| Phase 2 | `REQ-AST-P2-*` | Rich editor, inline chips, citation resolver, evidence preview, sentence support, rule engine, court-paper preview, PDF/DOCX, AI workflow. | Todo | Scaffolds exist for several backend modules, but product workflow is future work. |
| Phase 3 | `REQ-AST-P3-*` | Full support/citation/QC diffs, branch/merge, export hash validation, filing packets, rule overlays, collaborative review. | Todo | Outside MVP. |

## Optimized Backlog

| ID | Requirement | Source REQs | Priority | Status | Next Acceptance Check |
| --- | --- | --- | --- | --- | --- |
| WP-AST-001 | Keep `WorkProduct.document_ast` as canonical source of truth for all WorkProduct types. | `REQ-AST-001..004`, `REQ-AST-MVP-001` | P0 | Done | Create/update/patch/export/QC paths read the AST first and rebuild compatibility projections. |
| WP-AST-002 | Preserve public JSON compatibility while extending DTOs for missing metadata, tombstones, sentence IDs, provenance, and validation fields. | `REQ-AST-META-*`, `REQ-AST-ID-*`, `REQ-AST-VAL-019..020` | P0 | Done | Existing callers deserialize old payloads; new optional fields round-trip. |
| WP-AST-003 | Centralize AST helpers in dedicated backend modules. | `REQ-AST-MOD-*` | P0 | Done | `services/mod.rs` exposes AST modules and graph contract covers module presence. |
| WP-AST-004 | Normalize/validate AST before direct save, patch save, export, preview, markdown import, and QC. | `REQ-AST-010`, `REQ-AST-VAL-*`, `REQ-AST-EXPORT-011` | P0 | Done | Validation is called by each persistence/projection entrypoint. |
| WP-AST-005 | Add explicit matter-scoped AST GET/PATCH routes. | `REQ-AST-API-001..004`, `REQ-AST-API-012..014` | P0 | Done | `GET/PATCH /matters/:matter_id/work-products/:work_product_id/ast` exist and are contract-tested. |
| WP-AST-006 | Implement atomic AST patch application with stale base hash rejection. | `REQ-AST-PATCH-*` | P0 | Done | Failed patches leave persisted AST unchanged and return structured patch errors. |
| WP-AST-007 | Implement core block mutation operations. | `REQ-AST-OP-001..007`, `REQ-AST-MVP-016` | P0 | Done | Insert, update, delete/tombstone, move, split, merge, and renumber operations pass unit tests. |
| WP-AST-008 | Implement structured link/citation/exhibit/finding patch operations. | `REQ-AST-OP-008..016`, `REQ-AST-MVP-017..019` | P0 | Done | Add/remove/resolve operations update AST records and validation sees broken references. |
| WP-AST-009 | Treat conversions as service/API projections, not mutating patch operations. | `REQ-AST-OP-018..020`, `REQ-AST-MD-014`, `REQ-AST-RENDER-*` | P0 | Done | Conversion endpoints do not mutate stored AST unless caller explicitly saves returned AST. |
| WP-AST-010 | Support MVP block set through stable JSON `type` constants. | `REQ-AST-BLOCK-*`, `REQ-AST-MVP-003..010` | P0 | Done | Caption, heading, section, count, paragraph, numbered paragraph, signature, certificate, exhibit, page break, markdown, quote, list, table, and sentence-compatible blocks validate/render. |
| WP-AST-011 | Harden block-specific invariants with a typed registry. | `REQ-AST-BLOCK-*`, `REQ-AST-SECTION-*`, `REQ-AST-COUNT-*` | P1 | Partial | Lightweight block registry validates canonical types, leaf/child rules, required titles, paragraph numbers, sentence IDs, and count numbers. Renderer defaults and richer profile usage remain. |
| WP-AST-012 | Preserve stable IDs through Markdown round trips where metadata exists. | `REQ-AST-MD-001..014`, `REQ-AST-ID-009` | P1 | Partial | Parsed frontmatter and hidden sidecar comments preserve block metadata, links, citations, exhibits, findings, and tombstones in core cases; advanced tables and unsupported marks still warn. |
| WP-AST-013 | Add complete Markdown editor UX. | `REQ-AST-MD-*`, `REQ-AST-RTE-001..002` | P1 | Partial | Shared workbench can round-trip Markdown; split preview, warning UI, and conflict handling need completion. |
| WP-AST-014 | Render AST to deterministic HTML and plain text. | `REQ-AST-RENDER-001..008`, `REQ-AST-RENDER-015` | P0 | Done | HTML/plain text outputs are deterministic for the same AST and profile inputs. |
| WP-AST-015 | Replace PDF/DOCX placeholders with production renderers. | `REQ-AST-RENDER-004..005`, `REQ-AST-EXPORT-002..003`, `REQ-AST-P2-008..009` | P2 | Deferred | Visual renderer tests compare generated PDF/DOCX against expected court-paper output. |
| WP-AST-016 | Produce AST-targeted rule findings from AST inspection only. | `REQ-AST-RULE-*`, `REQ-AST-RULEENGINE-*` | P0 | Partial | MVP rule engine inspects AST for required blocks, unsupported factual blocks, and citation review findings; complete rule-pack execution and auto-fix patches remain. |
| WP-AST-017 | Integrate citation resolver with ORS/ORCP/UTCR/SLR graph nodes. | `REQ-AST-CITE-*`, `REQ-AST-P2-003` | P1 | Partial | Deterministic resolver extracts ORS/ORCP/UTCR/session-law citation uses with stable text ranges and canonical IDs. SLR/case law, graph currentness, and ambiguity workflow remain. |
| WP-AST-018 | Implement exhibit reference lifecycle and safe exhibit renumbering. | `REQ-AST-EXH-*`, `REQ-AST-OP-013..014` | P1 | Partial | Missing/ambiguous references validate; full renumber/update UI and filing-packet use remain. |
| WP-AST-019 | Implement unsupported factual assertion detection. | `REQ-AST-FACT-*`, `REQ-AST-EVID-*`, `REQ-AST-QC-001`, `REQ-AST-QC-012..013` | P0 | Partial | Block-level factual paragraphs without fact/evidence/document/source/exhibit support now receive QC findings. Sentence-level support, evidence matrix, and AI-specific unsupported text gates remain. |
| WP-AST-020 | Add sentence-level support and checking. | `REQ-AST-SENT-*`, `REQ-AST-P2-005` | P1 | Todo | Sentence segmentation creates stable sentence IDs, supports evidence/citation checks, and survives edits. |
| WP-AST-021 | Materialize AST links and legal records to graph safely. | `REQ-AST-GRAPH-*`, `REQ-AST-LINK-011..013` | P0 | Partial | Contract tests cover materialization expectations; scale-mode and query coverage need hardening. |
| WP-AST-022 | Keep legacy projections synchronized during migration. | `REQ-AST-002`, `REQ-AST-API-*`, `REQ-AST-MVP-*` | P0 | Done | Old block/finding surfaces update from AST after saves and patches. |
| WP-AST-023 | Implement frontend AST types, helpers, and normalizers. | `REQ-AST-API-*`, `REQ-AST-RTE-*` | P0 | Done | Frontend exposes AST GET/PATCH and normalizes optional fields. |
| WP-AST-024 | Build rich text editor legal nodes/marks/chips. | `REQ-AST-RTE-*`, `REQ-AST-P2-001..002` | P1 | Todo | ProseMirror/Tiptap round-trip tests preserve IDs, nodes, marks, chips, and QC anchors. |
| WP-AST-025 | Complete AI patch proposal/review workflow. | `REQ-AST-AI-*`, `REQ-AST-P2-010` | P1 | Partial | Provider-free AI command path records an empty reviewable `AstPatch` proposal instead of mutating text; accept/reject/edit UI and audit configuration remain. |
| WP-AST-026 | Expand version diff/restore across all AST layers. | `REQ-AST-VERSION-*`, `REQ-AST-P3-001..004` | P1 | Partial | Basic snapshot/diff/restore exists; full support/citation/QC diff and branch/merge are future work. |
| WP-AST-027 | Enforce export readiness with validation warnings. | `REQ-AST-EXPORT-*`, `REQ-AST-QC-010` | P0 | Partial | Export validates AST and warns on blocking findings, unresolved/stale citations, and missing/non-attached exhibits; full filing-readiness gates and hash validation remain. |
| WP-AST-028 | Add required-block validation by WorkProduct type. | `REQ-AST-VAL-011`, `REQ-AST-META-001`, `REQ-AST-BLOCK-*` | P0 | Done | Alias-aware required-block specs now cover supported WorkProduct types and avoid false complaint warnings from seed role names. |
| WP-AST-029 | Add registry-backed profile/template/rule-pack/format validation. | `REQ-AST-VAL-016..018`, `REQ-AST-META-007..009` | P1 | Partial | Validator checks deterministic MVP profile, template, rule-pack, and formatting registries. Live registry resolution remains. |
| WP-AST-030 | Add cross-reference detection and safe paragraph reference updates. | `REQ-AST-NUM-006..010` | P1 | Todo | Incorporation references validate, update safely on renumber, or get flagged when ambiguous. |
| WP-AST-031 | Complete support-link inspector and inline chip UX. | `REQ-AST-LINK-014..015`, `REQ-AST-RTE-005..010`, `REQ-AST-P2-002..004` | P1 | Partial | Block/range link data exists; rich inline chips and source inspector polish remain. |
| WP-AST-032 | Add production fixture corpus and property tests. | `REQ-AST-VAL-*`, `REQ-AST-PATCH-*`, `REQ-AST-MD-*`, `REQ-AST-MVP-*` | P1 | Partial | Focused unit/contract tests exist; broad fixture/property coverage remains. |
| WP-AST-033 | Keep AST list payloads opt-in. | `REQ-AST-API-*`, `REQ-AST-GRAPH-*` | P0 | Done | Full AST remains opt-in with `include=document_ast`; list views stay bounded. |
| WP-AST-034 | Preserve matter/work-product authorization boundary. | `REQ-AST-API-014`, `REQ-AST-LINK-012..013`, `REQ-AST-GRAPH-*` | P0 | Partial | Matter-scoped route and reference checks exist; auth remains future configured enforcement. |
| WP-AST-035 | Document production boundary for DOCX/PDF/filing packet export. | `REQ-AST-EXPORT-*`, `REQ-AST-P2-007..009`, `REQ-AST-P3-006` | P1 | Done | Backlog marks PDF/DOCX as deterministic placeholders until Phase 2 renderer work. |

## Immediate Follow-Up Order

1. Finish `WP-AST-013` so Markdown editor UX shows metadata-loss warnings and conflict states clearly.
2. Finish sentence-level support in `WP-AST-020` so `WP-AST-019` can graduate from block-level QC to sentence-level support checks.
3. Complete SLR/case-law/currentness resolver work in `WP-AST-017` and live registry resolution in `WP-AST-029`.
4. Start `WP-AST-024` only after the block registry and Markdown metadata paths are stable.
5. Defer production PDF/DOCX/filing packet work until court-paper rendering has visual tests.
