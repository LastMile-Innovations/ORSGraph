# Full ORSGraph Data Model

This document is the project-level data model reference for ORSGraph and CaseBuilder. It describes the current implemented graph and the intended top-down expansion path:

```text
United States
-> federal law
-> state law
-> county/local court overlays
-> court/location/procedure overlays
-> matter/work-product rule profiles
```

The core rule is simple: every legal answer, rule check, filing warning, export decision, and CaseBuilder finding should be traceable to source-backed graph records with jurisdiction, effective date, and provenance.

## Layer Map

ORSGraph has five major graph layers:

```text
Jurisdiction and courts
Legal corpora and source provenance
Legal text, provisions, citations, and semantics
Court rules registry and currentness overlays
CaseBuilder matters, evidence, WorkProducts, history, and rule profiles
```

The layers are intentionally reusable. Oregon is the first full state build-out, but the model must support:

- federal law and federal court rules;
- all Oregon statewide law and court rules;
- all Oregon county SLR/currentness overlays;
- other states with different court structures;
- local municipal/county codes and agency rules;
- matter-specific rule resolution by filing date.

## Jurisdiction And Court Model

Jurisdiction nodes provide the top-down legal scope.

Primary node labels:

```text
Jurisdiction
FederalJurisdiction
StateJurisdiction
CountyJurisdiction
JudicialDistrict
CircuitCourt
Court
```

Core records:

```json
{
  "jurisdiction_id": "us",
  "name": "United States",
  "jurisdiction_type": "federal",
  "parent_jurisdiction_id": null,
  "country": "US"
}
```

```json
{
  "jurisdiction_id": "or:state",
  "name": "Oregon",
  "jurisdiction_type": "state",
  "parent_jurisdiction_id": "us",
  "country": "US"
}
```

```json
{
  "jurisdiction_id": "or:linn",
  "name": "Linn County",
  "jurisdiction_type": "county",
  "parent_jurisdiction_id": "or:state",
  "country": "US"
}
```

```json
{
  "court_id": "or:linn:circuit_court",
  "name": "Linn County Circuit Court",
  "court_type": "circuit_court",
  "jurisdiction_id": "or:linn",
  "county_jurisdiction_id": "or:linn",
  "judicial_district_id": "or:judicial_district:23",
  "judicial_district": "23rd Judicial District"
}
```

Jurisdiction edges:

```text
(:Jurisdiction)-[:PART_OF]->(:Jurisdiction)
(:Court)-[:SERVES_JURISDICTION]->(:Jurisdiction)
(:Court)-[:LOCATED_IN]->(:CountyJurisdiction)
(:Court)-[:PART_OF_JUDICIAL_DISTRICT]->(:JudicialDistrict)
```

Resolver scope rule:

```text
For a county/court matter, load active authorities from:
local county jurisdiction
-> parent state jurisdiction
-> federal jurisdiction
```

For Oregon Linn County, that means:

```text
or:linn
or:state
us
```

## Legal Corpus Spine

All statutory, rule, and local court-rule corpora share the same source-backed spine.

Primary node labels:

```text
LegalCorpus
CorpusEdition
SourceDocument
SourcePage
SourceTocEntry
CourtRuleChapter
ChapterHeading
LegalTextIdentity
LegalTextVersion
Provision
CitationMention
ExternalLegalCitation
RetrievalChunk
ParserDiagnostic
```

Core edges:

```text
(:Jurisdiction)-[:HAS_CORPUS]->(:LegalCorpus)
(:LegalCorpus)-[:HAS_EDITION]->(:CorpusEdition)
(:CorpusEdition)-[:HAS_SOURCE_DOCUMENT]->(:SourceDocument)
(:SourceDocument)-[:HAS_PAGE]->(:SourcePage)
(:SourceDocument)-[:HAS_TOC_ENTRY]->(:SourceTocEntry)
(:CorpusEdition)-[:HAS_CHAPTER]->(:CourtRuleChapter)
(:CourtRuleChapter)-[:HAS_RULE]->(:LegalTextIdentity)
(:LegalTextIdentity)-[:HAS_VERSION]->(:LegalTextVersion)
(:LegalTextVersion)-[:CONTAINS]->(:Provision)
(:Provision)-[:HAS_CHILD]->(:Provision)
(:Provision)-[:NEXT]->(:Provision)
(:Provision)-[:PREVIOUS]->(:Provision)
(:Provision)-[:APPEARS_ON_PAGE]->(:SourcePage)
(:Provision)-[:DERIVED_FROM]->(:SourceDocument)
(:Provision)-[:MENTIONS_CITATION]->(:CitationMention)
(:CitationMention)-[:RESOLVES_TO]->(:LegalTextIdentity|Provision)
(:Provision)-[:CITES]->(:LegalTextIdentity|Provision)
(:Provision)-[:CITES_EXTERNAL]->(:ExternalLegalCitation)
(:RetrievalChunk)-[:CHUNKS]->(:LegalTextVersion|Provision|ProceduralRequirement)
```

Implemented Oregon corpus IDs:

```text
or:ors
or:utcr
or:linn:slr
```

Identifier shape:

```text
LegalCorpus:       or:utcr
CorpusEdition:     or:utcr@2025
LegalTextIdentity: or:utcr:2.010
LegalTextVersion:  or:utcr:2.010@2025-08-01
Provision:         or:utcr:2.010@2025-08-01:4:a
RetrievalChunk:    chunk:<stable hash>
```

For local SLRs:

```text
LegalCorpus:       or:linn:slr
CorpusEdition:     or:linn:slr@2026
LegalTextIdentity: or:linn:slr:13.095
LegalTextVersion:  or:linn:slr:13.095@2026
Provision:         or:linn:slr:13.095@2026:a
```

## Authority Families

Every legal text should carry both source identity and authority classification.

Current families:

```text
ORS
UTCR
SLR
RuleAuthorityDocument
```

Planned families:

```text
ORCP
ORAP
OAR
SessionLaw
MunicipalCode
CountyCode
FederalStatute
CFR
FederalRule
CaseLaw
CourtForm
AdministrativeOrder
```

Common authority properties:

```text
jurisdiction_id
authority_family
authority_type
authority_level
effective_date or effective_start_date
effective_end_date
source_document_id
source_url
official_status
disclaimer_required
```

Authority-level ordering is used for conflict and overlay reasoning. Lower-level local overlays should not replace statewide law; they supplement, localize, or modify procedure inside their own jurisdiction and date range.

## Temporal, Lineage, And Legislative Model

Statutory and rule text changes over time. The graph keeps current text queryable while preserving historical signals.

Primary node/row types:

```text
TimeInterval
TemporalEffect
LineageEvent
StatusEvent
Amendment
SessionLaw
SourceNote
ReporterNote
Commentary
ReservedRange
ChapterFrontMatter
ChapterTocEntry
TitleChapterEntry
```

Core relationships:

```text
(:LegalTextVersion)-[:EFFECTIVE_DURING]->(:TimeInterval)
(:LegalTextVersion)-[:HAS_STATUS_EVENT]->(:StatusEvent)
(:LegalTextVersion)-[:HAS_LINEAGE_EVENT]->(:LineageEvent)
(:SessionLaw)-[:AMENDS]->(:LegalTextIdentity|LegalTextVersion)
(:TemporalEffect)-[:AFFECTS]->(:LegalTextIdentity|LegalTextVersion)
(:Provision)-[:HAS_SOURCE_NOTE]->(:SourceNote)
(:LegalTextVersion)-[:HAS_REPORTER_NOTE]->(:ReporterNote)
(:LegalTextVersion)-[:HAS_COMMENTARY]->(:Commentary)
```

Session-law and temporal records should include:

```text
jurisdiction_id
citation
year
chapter
section
effective_date
affected_canonical_id
affected_version_id
source_document_id
source_note_id
confidence
```

For currentness-sensitive work, do not infer active law from latest text alone. Use the version/effective-date model plus any registry/order overlays.

## Legal Semantics Model

Legal semantics are extracted from provisions so search, issue spotting, rule checks, and WorkProduct QC can reason over more than raw text.

Primary semantic node/row types:

```text
DefinedTerm
Definition
DefinitionScope
LegalSemanticNode
Obligation
Exception
Deadline
Penalty
Remedy
LegalActor
LegalAction
MoneyAmount
TaxRule
RateLimit
RequiredNotice
FormText
```

Core relationships:

```text
(:Provision)-[:DEFINES]->(:Definition)
(:Definition)-[:DEFINES_TERM]->(:DefinedTerm)
(:Definition)-[:HAS_SCOPE]->(:DefinitionScope)
(:Provision)-[:EXPRESSES]->(:LegalSemanticNode)
(:Provision)-[:IMPOSES]->(:Obligation)
(:Provision)-[:CREATES_EXCEPTION]->(:Exception)
(:Provision)-[:SETS_DEADLINE]->(:Deadline)
(:Provision)-[:CREATES_PENALTY]->(:Penalty)
(:Provision)-[:CREATES_REMEDY]->(:Remedy)
(:Obligation)-[:APPLIES_TO_ACTOR]->(:LegalActor)
(:Obligation)-[:REQUIRES_ACTION]->(:LegalAction)
(:Penalty|Remedy|TaxRule)-[:HAS_AMOUNT]->(:MoneyAmount)
(:Provision)-[:REQUIRES_NOTICE]->(:RequiredNotice)
(:Provision)-[:HAS_FORM_TEXT]->(:FormText)
```

Semantic extraction should keep source traceability:

```text
source_provision_id
source_citation
text
normalized_text
confidence
extraction_method
jurisdiction_id
authority_family
effective_date
```

For CaseBuilder, semantic nodes are not enough on their own. A work-product finding should cite the semantic node and the source provision that produced it.

## Court Rules Registry Layer

The registry layer is not a PDF corpus. It is a publication/currentness index that says which court rules and orders exist, where they apply, and when they are active.

Primary node labels:

```text
CourtRulesRegistrySource
CourtRulesRegistrySnapshot
RulePublicationEntry
RuleAuthorityDocument
ChiefJusticeOrder
PresidingJudgeOrder
SupplementaryLocalRuleEdition
OutOfCycleAmendment
EffectiveInterval
RuleTopic
```

Registry provenance:

```text
(:CourtRulesRegistrySource)-[:HAS_ENTRY]->(:RulePublicationEntry)
(:CourtRulesRegistrySnapshot)-[:HAS_ENTRY]->(:RulePublicationEntry)
(:RulePublicationEntry)-[:DESCRIBES]->(:RuleAuthorityDocument)
(:RulePublicationEntry)-[:APPLIES_TO_JURISDICTION]->(:Jurisdiction)
```

Authority applicability:

```text
(:RuleAuthorityDocument)-[:APPLIES_TO]->(:Jurisdiction)
(:RuleAuthorityDocument)-[:GOVERNS_COURT]->(:Court)
(:RuleAuthorityDocument)-[:EFFECTIVE_DURING]->(:EffectiveInterval)
(:RuleAuthorityDocument)-[:HAS_TOPIC]->(:RuleTopic)
(:SupplementaryLocalRuleEdition)-[:SUPPLEMENTS]->(:LegalCorpus)
(:OutOfCycleAmendment)-[:AMENDS]->(:SupplementaryLocalRuleEdition)
(:RuleAuthorityDocument)-[:SUPERSEDES]->(:RuleAuthorityDocument)
```

Registry row example:

```json
{
  "authority_document_id": "or:linn:pjo:25-005",
  "title": "PJO 25-005 Order to Set Aside Judgments and Seal Eligible Residential Landlord Tenant Cases",
  "jurisdiction_id": "or:linn",
  "subcategory": "PJO",
  "authority_kind": "PresidingJudgeOrder",
  "effective_start_date": "2025-11-10",
  "effective_end_date": null,
  "publication_bucket": "current_future",
  "date_status": "current",
  "status_flags": ["current", "open_ended"]
}
```

Currentness is computed from dates, not just the source table bucket:

```text
effective_start_date <= filing_date
and
(effective_end_date is null or effective_end_date >= filing_date)
```

Status flags:

```text
current
future
prior
expired
open_ended
one_day_only
out_of_cycle
superseded
```

## Procedural Semantics And Rule Packs

Procedural semantics turn source rules into machine-checkable requirements.

Primary node labels:

```text
ProceduralRequirement
ProceduralRule
FormattingRequirement
FilingRequirement
CaptionRequirement
SignatureRequirement
CertificateOfServiceRequirement
ExhibitRequirement
ProtectedInformationRequirement
EfilingRequirement
ServiceRequirement
DeadlineRule
SanctionRule
ExceptionRule
WorkProductRulePack
RulePackMembership
FormattingProfile
```

Core edges:

```text
(:Provision)-[:EXPRESSES]->(:ProceduralRequirement)
(:WorkProductRulePack)-[:INCLUDES_RULE]->(:ProceduralRequirement)
(:WorkProductRulePack)-[:INCLUDES_AUTHORITY]->(:RuleAuthorityDocument)
(:WorkProductRulePack)-[:USES_FORMATTING_PROFILE]->(:FormattingProfile)
```

Current Oregon UTCR rule packs:

```text
or:utcr:2025:oregon_circuit_general_document
or:utcr:2025:oregon_circuit_civil_complaint
or:utcr:2025:oregon_circuit_civil_motion
or:utcr:2025:oregon_circuit_answer
or:utcr:2025:oregon_circuit_declaration
or:utcr:2025:oregon_circuit_filing_packet
```

Registry authorities extend rule packs by filing date and jurisdiction. For a Linn complaint filed on February 15, 2026, the rule profile should include:

```text
or:utcr@2025
or:linn:slr@2026
active statewide CJOs
active Linn PJOs
active out-of-cycle amendments
complaint rule-pack requirements
```

## Rule Applicability Resolver

`RuleApplicabilityResolver` is the backend service that answers:

```text
For this matter, in this court, on this filing date, for this work product type:
which rules and orders apply?
```

Inputs:

```json
{
  "jurisdiction": "Linn",
  "court": "Linn County Circuit Court",
  "workProductType": "complaint",
  "filingDate": "2026-02-15"
}
```

Output groups:

```text
utcr
slr_edition
statewide_orders
local_orders
out_of_cycle_amendments
currentness_warnings
```

API routes:

```text
GET /api/v1/rules/registry
GET /api/v1/rules/jurisdictions/:jurisdictionId/current
GET /api/v1/rules/jurisdictions/:jurisdictionId/history
GET /api/v1/rules/applicable?jurisdiction=Linn&date=2026-02-15&type=complaint
GET /api/v1/rules/orders/:authorityDocumentId
GET /api/v1/rules/slr/:jurisdictionId/:year
```

The resolver should never silently use expired or future authorities for applicability. History endpoints can return expired/prior rules.

## CaseBuilder Matter Model

CaseBuilder is the matter/workbench layer over the legal graph.

Primary matter nodes:

```text
Matter
CaseParty
CaseDocument
ObjectBlob
DocumentVersion
IngestionRun
SourceSpan
ExtractedTextChunk
CaseFact
CaseTimelineEvent
CaseEvidence
CaseClaim
CaseDefense
CaseElement
CaseDeadline
CaseTask
AuthorityRef
```

Core relationships:

```text
(:Matter)-[:HAS_PARTY]->(:CaseParty)
(:Matter)-[:HAS_DOCUMENT]->(:CaseDocument)
(:CaseDocument)-[:HAS_VERSION]->(:DocumentVersion)
(:DocumentVersion)-[:STORED_AS]->(:ObjectBlob)
(:DocumentVersion)-[:HAS_INGESTION_RUN]->(:IngestionRun)
(:IngestionRun)-[:PRODUCED_CHUNK]->(:ExtractedTextChunk)
(:ExtractedTextChunk)-[:HAS_SOURCE_SPAN]->(:SourceSpan)
(:Matter)-[:HAS_FACT]->(:CaseFact)
(:CaseFact)-[:SUPPORTED_BY]->(:SourceSpan|CaseEvidence|CaseDocument)
(:Matter)-[:HAS_TIMELINE_EVENT]->(:CaseTimelineEvent)
(:Matter)-[:HAS_EVIDENCE]->(:CaseEvidence)
(:Matter)-[:HAS_CLAIM]->(:CaseClaim)
(:CaseClaim)-[:HAS_ELEMENT]->(:CaseElement)
(:CaseElement)-[:SUPPORTED_BY]->(:CaseFact|CaseEvidence|AuthorityRef)
(:Matter)-[:HAS_DEADLINE]->(:CaseDeadline)
(:Matter)-[:HAS_TASK]->(:CaseTask)
```

The object-store boundary is deliberate:

```text
Neo4j: queryable legal meaning, summaries, source spans, hashes, IDs, relationships
R2/local ObjectStore: immutable bytes, large extraction artifacts, exports, snapshots
```

No storage key should contain names, facts, draft text, or case-sensitive labels.

## WorkProduct AST Model

`WorkProduct.document_ast` is the canonical current document model.

Primary nodes and DTOs:

```text
WorkProduct
WorkProductDocument
WorkProductMetadata
WorkProductBlock
WorkProductLink
WorkProductCitationUse
WorkProductExhibitReference
WorkProductFinding
TextRange
AstPatch
AstOperation
WorkProductArtifact
RuleProfileSummary
RuleCheckFinding
```

Pipeline:

```text
Matter
-> WorkProduct
-> WorkProductDocument AST
-> links/citations/exhibits/rule findings
-> Case History snapshots
-> export artifacts
```

Core AST rules:

- The AST is truth for current work product content.
- Flat `blocks`, `marks`, and `anchors` are compatibility projections.
- Legal support links target AST block IDs and text ranges.
- Rule findings target AST block IDs, sentence IDs, citation uses, exhibits, or document-level profile state.
- Export artifacts lock to immutable snapshots.
- AI edits should converge on `AstPatch` proposals and audited accepted changes.

Rule profile summary fields:

```text
jurisdiction_id
court_id
filing_date
utcr_edition_id
slr_edition_id
active_statewide_order_ids
active_local_order_ids
out_of_cycle_amendment_ids
currentness_warnings
```

## Case History Model

Case History is graph-native legal version control for WorkProducts and related support.

Primary nodes:

```text
ChangeSet
VersionChange
VersionSnapshot
SnapshotManifest
SnapshotEntityState
VersionBranch
LegalSupportUse
AIEditAudit
Milestone
```

Core edges:

```text
(:Matter)-[:HAS_CHANGE_SET]->(:ChangeSet)
(:ChangeSet)-[:CHANGED]->(:VersionChange)
(:WorkProduct)-[:HAS_SNAPSHOT]->(:VersionSnapshot)
(:VersionSnapshot)-[:HAS_MANIFEST]->(:SnapshotManifest)
(:VersionSnapshot)-[:HAS_ENTITY_STATE]->(:SnapshotEntityState)
(:WorkProduct)-[:HAS_BRANCH]->(:VersionBranch)
(:WorkProduct)-[:USES_SUPPORT]->(:LegalSupportUse)
(:WorkProduct)-[:HAS_AI_AUDIT]->(:AIEditAudit)
```

Snapshot and export artifacts may store large immutable state in ObjectStore, but the graph keeps IDs, hashes, summaries, and relationships.

## JSONL File Families

Core legal corpus outputs:

```text
legal_corpora.jsonl
corpus_editions.jsonl
source_documents.jsonl
source_pages.jsonl
source_toc_entries.jsonl
court_rule_chapters.jsonl
chapter_headings.jsonl
legal_text_identities.jsonl
legal_text_versions.jsonl
provisions.jsonl
citation_mentions.jsonl
external_legal_citations.jsonl
cites_edges.jsonl
retrieval_chunks.jsonl
parser_diagnostics.jsonl
```

Registry outputs:

```text
court_rules_registry_sources.jsonl
court_rules_registry_snapshots.jsonl
rule_publication_entries.jsonl
jurisdictions.jsonl
courts.jsonl
rule_authority_documents.jsonl
chief_justice_orders.jsonl
presiding_judge_orders.jsonl
supplementary_local_rule_editions.jsonl
out_of_cycle_amendments.jsonl
effective_intervals.jsonl
rule_topics.jsonl
rule_supersession_edges.jsonl
rule_applicability_edges.jsonl
work_product_rule_pack_authorities.jsonl
parser_diagnostics.jsonl
```

Procedural semantic outputs:

```text
procedural_rules.jsonl
formatting_requirements.jsonl
filing_requirements.jsonl
service_requirements.jsonl
efiling_requirements.jsonl
caption_requirements.jsonl
signature_requirements.jsonl
certificate_requirements.jsonl
exhibit_requirements.jsonl
protected_information_rules.jsonl
sanction_rules.jsonl
deadline_rules.jsonl
exception_rules.jsonl
work_product_rule_packs.jsonl
formatting_profiles.jsonl
rule_pack_memberships.jsonl
```

Temporal, lineage, and semantic outputs:

```text
source_notes.jsonl
reporter_notes.jsonl
commentaries.jsonl
chapter_toc_entries.jsonl
chapter_front_matter.jsonl
title_chapter_entries.jsonl
reserved_ranges.jsonl
temporal_effects.jsonl
lineage_events.jsonl
status_events.jsonl
amendments.jsonl
session_laws.jsonl
time_intervals.jsonl
defined_terms.jsonl
definitions.jsonl
definition_scopes.jsonl
legal_semantic_nodes.jsonl
obligations.jsonl
exceptions.jsonl
deadlines.jsonl
penalties.jsonl
remedies.jsonl
legal_actors.jsonl
legal_actions.jsonl
money_amounts.jsonl
tax_rules.jsonl
rate_limits.jsonl
required_notices.jsonl
form_texts.jsonl
```

QC and seed outputs:

```text
qc_full_report.json
stats.json
parser_diagnostics.jsonl
seed_stats.json
seed_failures.jsonl
```

## Current Oregon Implementation

Current implemented pieces:

- ORS legal text ingestion and Neo4j loading.
- 2025 UTCR PDF ingestion as `or:utcr`.
- Linn court rules registry snapshot parser with CJO/PJO/SLR/out-of-cycle currentness.
- Linn 2026 SLR PDF parser as `or:linn:slr`.
- Neo4j load/materialization support for jurisdictions, courts, registries, authorities, SLR editions, topics, intervals, applicability, supersession, and WorkProduct rule-pack authorities.
- Rules API and `RuleApplicabilityResolver`.
- CaseBuilder rule profile summary wiring for WorkProducts.
- WorkProduct AST, support links, citation uses, exhibits, rule findings, version snapshots, and export artifacts.

Important current limits:

- Oregon is the first expansion target; other states and federal rule corpora need source-specific parsers.
- ORCP, ORAP, OAR, federal rules, local municipal/county codes, forms, and case law are not complete corpora yet.
- Registry currentness is available only where registry snapshots have been ingested.
- PDF extraction is source-faithful enough for graphing and checks, but display text may need UI cleanup.
- Live filing automation is out of scope until rule profiles and court-specific safety checks are complete.

## Expansion Rules

When adding a new jurisdiction:

1. Create or ingest `Jurisdiction` records for `us`, the state, local jurisdictions, court districts, and courts.
2. Add a base legal corpus for the statewide or federal rule set.
3. Add source-backed legal text with `LegalCorpus`, `CorpusEdition`, `SourceDocument`, `LegalTextIdentity`, `LegalTextVersion`, and `Provision`.
4. Add registry/currentness sources for orders, local rules, emergency rules, and supersession.
5. Link local editions to their base rule corpus through `supplements_corpus_id`.
6. Emit effective intervals and preserve expired/future rules for history.
7. Add or extend WorkProduct rule packs by document type.
8. Verify `RuleApplicabilityResolver` returns only active authorities for the filing date.
9. Add search/citation parsing for that authority family.
10. Add UI warnings before any filing/export behavior relies on the new rules.

New state ID conventions:

```text
ca:state
ca:los_angeles
ca:los_angeles:superior_court
ca:trial_court_rules
ca:los_angeles:local_rules
```

Federal ID conventions:

```text
us
us:usc
us:cfr
us:frcp
us:fre
us:bankruptcy_rules
us:district_court:or
us:district_court:or:local_rules
```

The data model should grow by adding new authority families and source adapters, not by creating one-off matter logic.
