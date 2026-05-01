# 07 - Feature Inventory

This inventory maps the full CaseBuilder feature list to current status and next backlog work.

## Major modules

| # | Module | Current status | Next backlog item |
|---|---|---|---|
| 1 | Matter Workspace | Partial | `CB-V0-001`, `CB-V0F-010` |
| 2 | File / Evidence Ingestion | Partial | `CB-V0-002`, `CB-V0-011`, `CB-V0-019`, `CB-V0-021` |
| 3 | Document Viewer | Partial | `CB-V0-004`, `CB-V0-006` |
| 4 | Fact Extraction | Partial | `CB-V0-006`, `CB-V0-020` |
| 5 | Timeline Builder | Partial | `CB-V0-008` |
| 6 | Party / Entity Map | Partial | `CB-V0-009` |
| 7 | Issue Spotter | Todo | `CB-V0-023`, `CB-X-005` |
| 8 | Claim Builder | Partial | `CB-V0-010` |
| 9 | Defense Builder | Partial | `CB-V01-003` |
| 10 | Element Mapper | Partial | `CB-V0-010`, `CB-V02-009` |
| 11 | Evidence Matrix | Partial | `CB-V0-011`, `CB-V0-020` |
| 12 | Legal Authority Finder | Partial | `CB-V0-012`, `CB-V02-008`, `CB-X-006` |
| 13 | Legacy Drafting Studio | Partial | `CB-V0-014`, `CB-V0-015`, `CB-WPB-020` |
| 14 | WorkProduct Builder | Partial / canonical AST backend wired | `CB-WPB-004` through `CB-WPB-065` |
| 15 | Complaint Profile | Partial / facade over WorkProduct AST | `CB-WPB-014`, `CB-CE-017`, `CB-CH-105`, `CB-CH-1104` |
| 16 | Answer Profile | Todo | `CB-V01-001`, `CB-V01-002`, `CB-WPB-015` |
| 17 | Motion Profile | Partial / AST skeleton wired | `CB-V02-001`, `CB-V02-002`, `CB-WPB-016` |
| 18 | Declaration / Affidavit Profiles | Todo | `CB-V02-003`, `CB-WPB-017` |
| 19 | Exhibit List Profile | Todo | `CB-V02-004`, `CB-WPB-017` |
| 20 | Citation Checker | Partial / provider-gated | `CB-V0-016`, `CB-V0-026`, `CB-X-006`, `CB-WPB-011` |
| 21 | Fact Checker | Partial / provider-gated | `CB-V0-016`, `CB-V0-026`, `CB-X-005`, `CB-WPB-011` |
| 22 | Deadline / Calendar Builder | Partial | `CB-V01-004`, `CB-V01-005` |
| 23 | Notice / Form Profile | Todo | `CB-V01-006`, `CB-WPB-017` |
| 24 | Argument Graph | Partial | `CB-V01-007`, `CB-V01-008` |
| 25 | Case Strategy Board | Todo | `CB-V02-010`, `CB-V1-009` |
| 26 | Task / Backlog System | Partial | `CB-V01-010` |
| 27 | QC / Risk Dashboard | Partial | `CB-V01-009`, `CB-V01-011`, `CB-V01-012`, `CB-WPB-011` |
| 28 | Export / Filing Packager | Partial / AST-backed skeletons | `CB-WPB-012`, `CB-V02-005`, `CB-V02-006`, `CB-V02-007`, `CB-V02-011`, `CB-V1-008` |
| 29 | Collaboration / Review | Deferred | `CB-V1-001` through `CB-V1-005` |
| 30 | Case File Indexing Harness | Todo | `CB-IDX-001` through `CB-IDX-016` |
| 31 | Case History / Legal Version Control | Partial / V0 foundation wired | `CB-CH-105`, `CB-CH-1101`, `CB-CH-402`, `CB-CH-503`, `CB-CH-1104` |

## Killer features

| Killer feature | Current status | Notes |
|---|---|---|
| Upload all case files and automatically build a case graph | Partial | Backend storage, binary upload, graph node foundation, and V0 text extraction exist; PDF/DOCX/XLSX/OCR parsing, manifests, and automatic large-matter graph build remain. |
| Turn documents into reviewable facts with source citations | Partial | Deterministic proposed facts now include structured source spans/quotes and are visible in review/detail UI; duplicate handling and richer extraction remain. |
| Build a chronological timeline automatically | Partial | Timeline page supports live event creation; automated extraction from facts/documents remains. |
| Suggest claims/defenses from facts | Todo | Manual claim creation exists; issue spotting endpoint, queue, and defense suggestion remain. |
| Map facts/evidence to legal elements | Partial | Models, map endpoint, live persistence, and evidence/fact/element synchronization exist; richer element templates and editing remain. |
| Find ORS authority for each claim/defense | Partial | Backend authority search and attach/detach for claims/elements/draft paragraphs exist; recommendation/currentness panels and defense/sentence targets remain. |
| Draft complaints/answers/motions from graph-backed facts | Partial | WorkProduct AST DTOs/routes/history, AST patching, conversions, generic motion skeleton, and structured complaint profile exist; shared frontend WorkProductEditor, answer profile, motion UI, AI patch acceptance, and AST release gates remain. |
| Fact-check every sentence in a work product | Partial | Paragraph-level deterministic checks, AST-targeted rule findings, and persisted editor/QC findings exist; WorkProduct sentence graph nodes and source-backed live checks remain. |
| Show unsupported allegations before filing | Partial | Backend can create unsupported-fact findings and the draft/QC/complaint surfaces render persisted findings; richer matter-level gap/contradiction lifecycle remains. |
| Build exhibit list and filing packet | Partial / skeleton | WorkProduct AST stores exhibit references and export artifacts exist in skeleton form; exhibit-list profile, packet generation, shared production DOCX/PDF, and download status remain. |
| Show deadlines and notice requirements | Partial | Deadline page exists; live deadline detection and RequiredNotice/FormText workflow remain. |
| Visualize the whole case as a graph | Partial | Graph route shell exists; matter graph API, graph modes, and renderer remain. |
| Index hundreds to thousands of mixed files | Todo | Dedicated indexing harness spec and backlog exist; parser registry, index adapters, UI console, and large-fixture benchmark remain. |
| Restore or compare any work-product version | Partial | Case History V0 supports AST-backed WorkProduct snapshots, timeline, text/block compare, whole/block restore, AI audit, export hash, and changed-since-export; AST support/citation/exhibit/QC diff, scoped restore, branch alternatives, merge cards, and full smoke coverage remain. |

## AI feature status

| AI feature group | Current status | Production backlog |
|---|---|---|
| File understanding | Partial / deterministic | `CB-V0-019`, `CB-V0-020`, `CB-X-004`, `CB-X-005` |
| Fact extraction | Partial / deterministic | `CB-V0-006`, `CB-V0-021`, `CB-X-005` |
| Issue spotting | Todo / provider-gated | `CB-V0-023`, `CB-X-004`, `CB-X-005` |
| Element mapping | Partial / deterministic | `CB-V0-010`, `CB-V02-009` |
| Authority retrieval | Partial / live ORSGraph search | `CB-V02-008`, `CB-X-006` |
| Drafting/editor | Partial / template | `CB-WPB-002` through `CB-WPB-065`, `CB-V0-014`, `CB-V0-015`, `CB-V0-026`, `CB-CH-105` |
| Fact/citation checking | Partial / deterministic | `CB-V0-016`, `CB-V0-026`, `CB-X-006` |
| Strategy scoring | Todo | `CB-V02-010`, `CB-V1-009` |

## Storage and provenance architecture

| Layer | Current status | Production backlog |
|---|---|---|
| R2 evidence/artifact lake | Partial | `CB-V0F-013`, `CB-V0F-017`, `CB-WPB-061`, `CB-WPB-062`, `CB-WPB-064`, `CB-X-020`, `CB-X-021` |
| Neo4j case intelligence graph | Partial | `CB-V0F-005`, `CB-V0F-013`, `CB-V0-027`, `CB-X-022` |
| Opaque object keys and blob dedupe | Partial | `CB-V0F-014`, `CB-V0F-016` |
| Document version provenance | Partial | `CB-V0F-015`, `CB-V0-020` |
| Ingestion reproducibility | Partial | `CB-V0-019`, `CB-V0F-017`, `CB-V0-027` |
| Legal version-control reproducibility | Partial | `CB-WPB-023`, `CB-WPB-044`, `CB-WPB-045`, `CB-WPB-056`, `CB-WPB-061`, `CB-WPB-062`, `CB-CH-105`, `CB-CH-1101`, `CB-CH-1104` |
| Retention/storage class policy | Todo | `CB-WPB-065`, `CB-X-017`, `CB-X-021`, `CB-V1-012` |

## Indexing harness status

| Harness layer | Current status | Production backlog |
|---|---|---|
| DTOs and constraints | Partial | `CB-IDX-001` |
| Parser registry and classifier | Todo | `CB-IDX-002` |
| Inventory/fingerprint index | Todo | `CB-IDX-003` |
| R2 normalized artifact writer | Todo | `CB-IDX-004` |
| Manifest-to-graph upserter | Todo | `CB-IDX-005` |
| Full-text and vector adapters | Todo | `CB-IDX-006`, `CB-IDX-007` |
| OCR/archive/email/spreadsheet workflows | Todo | `CB-IDX-008` through `CB-IDX-011` |
| Index console and provenance UI | Todo | `CB-IDX-012`, `CB-IDX-013` |
| Reindexing, benchmarks, quarantine | Todo | `CB-IDX-014` through `CB-IDX-016` |

## V0 MVP scope status

| MVP item | Status |
|---|---|
| Matter workspace | Partial |
| File upload | Partial / V0 binary wired |
| Document text extraction | Partial |
| Fact extraction | Partial / source spans visible |
| Timeline | Partial |
| Parties | Partial |
| Claim builder | Partial |
| Evidence matrix | Partial |
| Legacy drafting studio | Partial |
| WorkProduct Builder | Partial / canonical AST backend wired |
| Fact-checking | Partial |
| ORS authority search | Partial / attach wired |
| Complaint profile entry point | Partial / facade over WorkProduct AST and Case History |
| Case History snapshots/compare/restore | Partial / V0 text layer wired |
| Export to DOCX/PDF later | Deferred |

## Deferred by design

- Court e-filing.
- Multi-user collaboration.
- Attorney review mode.
- Broad court-rule integration beyond the first Oregon complaint rule pack.
- Case-law integration.
- Audio/video transcription.
- Full OCR.
- Advanced strategy scoring.
- Production RBAC and sharing.
