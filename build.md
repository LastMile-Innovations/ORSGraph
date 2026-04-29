# Final Graph Spec: Oregon Legal Graph OS

This should be built as a **time-aware, authority-aware, jurisdiction-aware, source-provenance-backed legal knowledge graph**.

Not:

```text
documents → chunks → embeddings
```

But:

```text
jurisdictions
→ issuers
→ legal authorities
→ versions
→ provisions
→ citations
→ amendments
→ interpretations
→ enforcement
→ geography
→ retrieval views
→ sourced legal answers
```

The Oregon Legislature’s ORS page confirms why this matters: the ORS is the codified law, the 2025 edition is current, the ORS is published every two years, and the online database text is not the official legal text, so legal accuracy must be verified against official Oregon law sources. That disclaimer becomes a first-class graph property, not a UI footnote. ([Oregon Legislature][1])

---

# 0. Highest-level architecture

Use **six graph layers**.

```text
L0 Source / provenance graph
L1 Jurisdiction / geography graph
L2 Authority / issuer graph
L3 Legal text identity + version graph
L4 Legal relationship graph
L5 Retrieval / embedding / agent graph
```

The rule:

> **Only authoritative legal text nodes are treated as law. Chunks, embeddings, summaries, and LLM extractions are derived artifacts.**

---

# 1. Core graph principle

Every legal object gets split into:

```text
Identity node = stable thing over time
Version node = exact text/state at a specific time
Provision node = smallest legally meaningful unit
RetrievalChunk = search/embedding view over one or more provisions
SourceDocument = where the text came from
```

Example:

```text
(:LegalTextIdentity {canonical_id: "or:ors:1.002"})
  -[:HAS_VERSION]->
(:LegalTextVersion {version_id: "or:ors:1.002@2025"})
  -[:CONTAINS]->
(:Provision {provision_id: "or:ors:1.002@2025::subsection:3:a"})
```

This is what makes the graph capable of answering:

```text
What is current?
What was effective on a past date?
What changed?
What authority controls?
What cites it?
What interprets it?
What local rule implements or conflicts with it?
```

---

# 2. Root ontology

## 2.1 Base labels

Use these universal labels across the whole system:

```text
:Entity
:Jurisdiction
:PublicBody
:Source
:LegalAuthority
:LegalTextIdentity
:LegalTextVersion
:Provision
:CitationMention
:ChangeEvent
:Interpretation
:RetrievalArtifact
:GeoEntity
:TemporalEntity
:IssueTaxonomyNode
```

Specialized labels attach on top.

Example:

```cypher
(:LegalAuthority:Statute:ORSSectionVersion)
(:LegalAuthority:AdministrativeRule:OARRuleVersion)
(:LegalAuthority:CaseLaw:Opinion)
(:LegalAuthority:LocalLaw:CityCodeSectionVersion)
```

---

# 3. Jurisdiction and geography graph

This is the root of the whole database.

Oregon has 36 counties, and Oregon’s state site describes county government structures across those counties. ([Oregon][2]) Oregon also has 241 incorporated cities, and the Oregon Blue Book notes that city councils pass ordinances, adopt resolutions, and set local public policy. ([Oregon Secretary of State][3])

## 3.1 Jurisdiction labels

```text
:State
:County
:City
:MetroGovernment
:SpecialDistrict
:SchoolDistrict
:FireDistrict
:WaterDistrict
:SanitaryDistrict
:PortDistrict
:TransitDistrict
:LibraryDistrict
:ParksDistrict
:CourtDistrict
:JudicialDistrict
:LegislativeDistrict
:ZoningDistrict
:OverlayDistrict
:UrbanGrowthBoundary
:ServiceArea
:Parcel
:Address
```

## 3.2 Jurisdiction properties

```json
{
  "jurisdiction_id": "or:city:portland",
  "name": "Portland",
  "kind": "city",
  "state": "OR",
  "county_ids": ["or:county:multnomah", "or:county:washington", "or:county:clackamas"],
  "source_url": "...",
  "official_status": "official",
  "geo_ref": "s3://geo/or/cities/portland.geojson",
  "geo_hash": "sha256:...",
  "effective_start": "1851-02-08",
  "effective_end": null
}
```

## 3.3 Geography relationships

```text
(:City)-[:IN_COUNTY]->(:County)
(:County)-[:IN_STATE]->(:State)
(:Parcel)-[:LOCATED_IN]->(:City)
(:Parcel)-[:LOCATED_IN]->(:County)
(:Parcel)-[:WITHIN]->(:SpecialDistrict)
(:Parcel)-[:WITHIN]->(:SchoolDistrict)
(:Parcel)-[:WITHIN]->(:ZoningDistrict)
(:Parcel)-[:WITHIN]->(:OverlayDistrict)
(:LegalAuthority)-[:APPLIES_IN]->(:Jurisdiction)
(:LegalAuthority)-[:APPLIES_TO_GEO]->(:GeoEntity)
(:PublicBody)-[:HAS_JURISDICTION_OVER]->(:Jurisdiction)
```

Use Neo4j for the graph and keep large geometry in PostGIS or object storage. Store the geometry pointer, hash, bounding box, and simplified geometry in Neo4j.

---

# 4. Public body / issuer graph

Every law, rule, order, policy, fee schedule, permit standard, or guidance document must have an issuer.

## 4.1 Public body labels

```text
:PublicBody
:Legislature
:StateAgency
:CountyBoard
:CityCouncil
:MayorOffice
:CityManagerOffice
:CountyDepartment
:CityDepartment
:PlanningCommission
:HearingsOfficer
:SchoolBoard
:SpecialDistrictBoard
:Court
:AdministrativeTribunal
:AttorneyGeneralOffice
:EthicsCommission
:LawLibrary
:ClerkOffice
:AuditorOffice
:RecorderOffice
```

## 4.2 Public body properties

```json
{
  "body_id": "or:city:portland:council",
  "name": "Portland City Council",
  "body_type": "city_council",
  "jurisdiction_id": "or:city:portland",
  "parent_body_id": "or:city:portland",
  "official_url": "...",
  "active": true
}
```

## 4.3 Public body relationships

```text
(:PublicBody)-[:PART_OF]->(:PublicBody)
(:PublicBody)-[:SERVES]->(:Jurisdiction)
(:PublicBody)-[:ISSUED]->(:LegalAuthority)
(:PublicBody)-[:ENFORCES]->(:LegalAuthority)
(:PublicBody)-[:ADMINISTERS]->(:Program)
(:PublicBody)-[:HOLDS_MEETING]->(:Meeting)
(:PublicBody)-[:HAS_OFFICIAL_SOURCE]->(:SourcePublisher)
```

---

# 5. Source and provenance graph

This is mandatory. Every node must trace back to source bytes.

## 5.1 Source labels

```text
:SourcePublisher
:SourceCollection
:SourceDocument
:SourcePage
:RawArtifact
:NormalizedArtifact
:ParsedArtifact
:ParserRun
:FetcherRun
:CrawlerRun
:SourceLicense
:RobotsPolicy
:TermsSnapshot
:Checksum
```

## 5.2 SourceDocument properties

```json
{
  "source_document_id": "src:orleg:ors:chapter:001@2025",
  "publisher": "Oregon Legislature",
  "source_type": "official_online_database",
  "url": "...",
  "canonical_priority": 100,
  "official_status": "official_online_not_official_print",
  "disclaimer_required": true,
  "retrieved_at": "2026-04-27T00:00:00Z",
  "content_type": "text/html",
  "raw_hash": "sha256:...",
  "normalized_hash": "sha256:...",
  "license_status": "needs_review",
  "robots_status": "allowed|disallowed|unknown",
  "terms_status": "reviewed|unknown|restricted"
}
```

## 5.3 Provenance relationships

```text
(:SourcePublisher)-[:PUBLISHED]->(:SourceDocument)
(:CrawlerRun)-[:DISCOVERED]->(:SourceDocument)
(:FetcherRun)-[:FETCHED]->(:RawArtifact)
(:RawArtifact)-[:NORMALIZED_TO]->(:NormalizedArtifact)
(:ParserRun)-[:PARSED]->(:NormalizedArtifact)
(:ParserRun)-[:CREATED]->(:LegalTextVersion)
(:LegalTextVersion)-[:DERIVED_FROM]->(:SourceDocument)
(:Provision)-[:DERIVED_FROM]->(:SourceDocument)
(:RetrievalChunk)-[:DERIVED_FROM]->(:Provision)
```

## 5.4 Why this matters

The pasted ORS Chapter 1 sample shows the parser has to preserve title/chapter structure, section tables, active statutes, notes, amendment brackets, repealed sections, renumbered sections, and internal cross-references. 

So the graph must support:

```text
source page
→ raw text
→ normalized text
→ parsed chapter
→ section version
→ provision tree
→ citation mentions
→ amendment notes
→ retrieval chunks
```

---

# 6. Legal authority taxonomy

Everything legal goes under `:LegalAuthority`.

## 6.1 State authority labels

```text
:ConstitutionProvision
:Statute
:ORSChapter
:ORSSectionIdentity
:ORSSectionVersion
:SessionLaw
:OregonLawChapter
:Bill
:BillVersion
:BillAmendment
:LegislativeMeasure
:LegislativeHistoryDocument
:MeasureSummary
:FiscalImpactStatement
:RevenueImpactStatement
:CommitteeRecord
:VoteRecord
:BallotMeasure
:VoterPamphletEntry
```

## 6.2 Administrative authority labels

```text
:AdministrativeRule
:OARChapter
:OARDivision
:OARRuleIdentity
:OARRuleVersion
:AdministrativeOrder
:RuleFiling
:NoticeOfProposedRulemaking
:TemporaryRule
:PermanentRule
:StatutoryMinorCorrection
:FiveYearRuleReview
:AgencyGuidance
:AgencyManual
:DeclaratoryRuling
:FinalOrder
:DisciplinaryOrder
:PermitDecision
:LicenseDecision
```

OARD is the Oregon Administrative Rules Database, and the Secretary of State says it houses rules and filings, including official copies. OARs are used by rulemaking entities to implement or interpret statutes, and rules may be adopted, amended, repealed, renumbered, or temporarily suspended through administrative processes. ([Oregon Secretary of State][4]) ([Oregon Secretary of State][5])

## 6.3 Court authority labels

```text
:Case
:CaseIdentity
:CaseVersion
:Opinion
:OpinionVersion
:OpinionParagraph
:Holding
:Issue
:RuleStatement
:StandardOfReview
:Disposition
:ProceduralPosture
:Docket
:Citation
:CaseTreatment
:CourtRule
:ORCPRule
:UTCRRule
:ORAPRule
:SupplementaryLocalRule
:StandingOrder
:CourtForm
:CourtFeeSchedule
```

Oregon Supreme Court, Court of Appeals, and Tax Court decisions are posted weekly or as soon as available on the day issued, with final publication available through the State of Oregon Law Library Digital Collection. ([Oregon Courts][6])

## 6.4 County authority labels

```text
:CountyCharter
:CountyCode
:CountyCodeTitle
:CountyCodeChapter
:CountyCodeSectionIdentity
:CountyCodeSectionVersion
:CountyOrdinance
:CountyResolution
:CountyBoardOrder
:CountyPolicy
:CountyManual
:CountyFeeSchedule
:CountyZoningCode
:CountyDevelopmentCode
:CountyComprehensivePlan
:CountyPublicWorksStandard
:CountySheriffPolicy
:CountyJailPolicy
:CountyPublicHealthRule
:CountyProcurementRule
```

## 6.5 City authority labels

```text
:CityCharter
:CityCode
:CityCodeTitle
:CityCodeChapter
:CityCodeSectionIdentity
:CityCodeSectionVersion
:CityOrdinance
:CityResolution
:CityCouncilRule
:CityPolicy
:CityAdministrativeRule
:CityFeeSchedule
:CityZoningCode
:CityDevelopmentCode
:CityComprehensivePlan
:CityPublicWorksStandard
:BusinessLicenseRule
:ShortTermRentalRule
:ParkingRule
:NoiseCode
:NuisanceCode
:TreeCode
:BuildingPermitRule
:FireCodeLocalAmendment
```

## 6.6 District / school / local authority labels

```text
:SpecialDistrictRule
:SpecialDistrictPolicy
:SpecialDistrictResolution
:RateSchedule
:ServiceRule
:SchoolBoardPolicy
:AdministrativeRegulation
:StudentHandbook
:EmployeeHandbook
:FacilityUseRule
:TransitRule
:WaterServiceRule
:SanitaryRule
:FireDistrictPolicy
:LibraryPolicy
:PortDistrictRule
```

---

# 7. Universal legal text model

## 7.1 LegalTextIdentity

Stable ID across time.

```json
{
  "canonical_id": "or:ors:1.002",
  "authority_family": "ORS",
  "jurisdiction_id": "or:state",
  "issuer_id": "or:legislature",
  "citation": "ORS 1.002",
  "title": "Supreme Court; Chief Justice as administrative head...",
  "status": "active",
  "created_date": null,
  "retired_date": null
}
```

## 7.2 LegalTextVersion

Exact version at a time.

```json
{
  "version_id": "or:ors:1.002@2025",
  "canonical_id": "or:ors:1.002",
  "edition": "2025",
  "effective_start": "2025-01-01",
  "effective_end": null,
  "publication_date": null,
  "text_hash": "sha256:...",
  "source_document_id": "src:orleg:ors:chapter:001@2025",
  "official_status": "official_online_not_official_print",
  "authority_level": 90,
  "current": true,
  "superseded": false
}
```

## 7.3 Provision

The smallest legally meaningful unit.

```json
{
  "provision_id": "or:ors:1.002@2025::p:3:a",
  "version_id": "or:ors:1.002@2025",
  "display_citation": "ORS 1.002(3)(a)",
  "local_path": ["3", "a"],
  "provision_type": "paragraph",
  "text": "...",
  "normalized_text": "...",
  "order_index": 41,
  "depth": 2,
  "text_hash": "sha256:...",
  "is_definition": false,
  "is_exception_candidate": false,
  "is_deadline_candidate": false,
  "is_penalty_candidate": false,
  "is_procedural": true,
  "source_start_offset": 12345,
  "source_end_offset": 12890
}
```

## 7.4 Provision relationships

```text
(:LegalTextVersion)-[:CONTAINS]->(:Provision)
(:Provision)-[:CONTAINS]->(:Provision)
(:Provision)-[:NEXT]->(:Provision)
(:Provision)-[:PREVIOUS]->(:Provision)
(:Provision)-[:HAS_PARENT]->(:Provision)
(:Provision)-[:PART_OF_VERSION]->(:LegalTextVersion)
```

---

# 8. Citation graph

Do **not** just create `CITES` edges. Use citation mention nodes.

## 8.1 CitationMention

```json
{
  "citation_mention_id": "cite:01HX...",
  "raw_text": "ORS 192.311 to 192.478",
  "normalized_citation": "ORS 192.311-192.478",
  "citation_type": "range",
  "target_type": "statute_range",
  "source_offset_start": 244,
  "source_offset_end": 266,
  "resolver_status": "resolved_partial",
  "confidence": 0.94
}
```

## 8.2 Citation relationships

```text
(:Provision)-[:MENTIONS_CITATION]->(:CitationMention)
(:OpinionParagraph)-[:MENTIONS_CITATION]->(:CitationMention)
(:CitationMention)-[:RESOLVES_TO]->(:LegalTextIdentity)
(:CitationMention)-[:RESOLVES_TO_VERSION]->(:LegalTextVersion)
(:CitationMention)-[:RESOLVES_TO_RANGE]->(:LegalTextIdentity)
(:CitationMention)-[:UNRESOLVED_TARGET]->(:UnresolvedCitation)
```

## 8.3 Materialized convenience edges

These are derived from citation mentions:

```text
(:LegalAuthority)-[:CITES {basis:"citation_mention"}]->(:LegalAuthority)
(:Opinion)-[:CITES_STATUTE]->(:ORSSectionIdentity)
(:Rule)-[:CITES_STATUTE]->(:ORSSectionIdentity)
(:LocalCodeSectionVersion)-[:CITES_OR_SUPPLEMENTS]->(:ORSSectionIdentity)
```

The mention node remains the audit trail.

---

# 9. Definition graph

Definitions are scoped. Never use one global `Term` node without scope.

## 9.1 Labels

```text
:DefinedTerm
:Definition
:DefinitionScope
:TermUse
```

## 9.2 Definition properties

```json
{
  "definition_id": "def:or:ors:1.194@2025:payment",
  "term": "Payment",
  "normalized_term": "payment",
  "definition_text": "...",
  "definition_style": "means",
  "scope_type": "section_range",
  "scope_citation": "ORS 1.194 to 1.200",
  "confidence": 1.0
}
```

## 9.3 Definition relationships

```text
(:Provision)-[:DEFINES]->(:Definition)
(:Definition)-[:DEFINES_TERM]->(:DefinedTerm)
(:Definition)-[:HAS_SCOPE]->(:DefinitionScope)
(:DefinitionScope)-[:COVERS]->(:LegalTextIdentity)
(:Provision)-[:USES_TERM]->(:DefinedTerm)
(:Provision)-[:GOVERNED_BY_DEFINITION]->(:Definition)
```

This lets the graph answer:

```text
Which definition controls this use of the term?
Is this definition chapter-wide, section-only, title-wide, or local?
```

---

# 10. Amendment / change-event graph

Amendments must be modeled as events, not as text notes.

## 10.1 Labels

```text
:ChangeEvent
:AmendmentEvent
:RepealEvent
:RenumberEvent
:CodificationEvent
:SunsetEvent
:RevivalEvent
:TemporaryProvisionEvent
:EffectiveDateEvent
:OrdinanceAdoptionEvent
:RulemakingEvent
:CaseTreatmentEvent
```

## 10.2 ChangeEvent properties

```json
{
  "change_event_id": "chg:or:laws:2025:ch88:sec1",
  "change_type": "amend",
  "source_authority_id": "or:laws:2025:ch88",
  "source_section": "Section 1",
  "target_canonical_id": "or:ors:1.002",
  "effective_start": "2025-09-15",
  "effective_end": null,
  "confidence": 1.0,
  "extraction_method": "statutes_affected_table+session_law_parser"
}
```

## 10.3 Change relationships

```text
(:Bill)-[:BECAME]->(:OregonLawChapter)
(:OregonLawChapter)-[:HAS_SECTION]->(:SessionLawProvision)
(:SessionLawProvision)-[:CAUSES]->(:ChangeEvent)
(:ChangeEvent)-[:AMENDS]->(:LegalTextIdentity)
(:ChangeEvent)-[:REPEALS]->(:LegalTextIdentity)
(:ChangeEvent)-[:RENUMBERS]->(:LegalTextIdentity)
(:ChangeEvent)-[:CREATES_VERSION]->(:LegalTextVersion)
(:LegalTextVersion)-[:SUPERSEDES]->(:LegalTextVersion)
(:LegalTextIdentity)-[:RENAMED_TO]->(:LegalTextIdentity)
(:LegalTextIdentity)-[:RENAMED_FROM]->(:LegalTextIdentity)
```

This enables:

```text
What changed?
When did it change?
Which bill/ordinance/order changed it?
What version was in effect on a date?
Was it repealed, renumbered, sunsetted, or temporary?
```

---

# 11. Case law graph

## 11.1 Case labels

```text
:CaseIdentity
:CaseVersion
:Opinion
:OpinionVersion
:OpinionParagraph
:OpinionSection
:Syllabus
:Headnote
:Issue
:Holding
:RuleStatement
:FactPattern
:ProceduralPosture
:Disposition
:StandardOfReview
:Party
:Attorney
:Judge
:Court
:Docket
:ReporterCitation
:ParallelCitation
:PinpointCitation
:CaseTreatment
```

## 11.2 Opinion properties

```json
{
  "opinion_id": "or:case:sct:S069123:2026-04-23",
  "case_name": "...",
  "court_id": "or:court:supreme",
  "decision_date": "2026-04-23",
  "publication_status": "published",
  "precedential_status": "binding",
  "official_citation": null,
  "docket_number": "...",
  "source_url": "...",
  "authority_level": 95
}
```

## 11.3 Opinion paragraph

```json
{
  "paragraph_id": "or:case:sct:S069123:2026-04-23::para:42",
  "paragraph_number": 42,
  "text": "...",
  "text_hash": "sha256:...",
  "page": 12,
  "pin_cite": "12"
}
```

## 11.4 Case relationships

```text
(:Opinion)-[:DECIDED_BY]->(:Court)
(:Opinion)-[:AUTHORED_BY]->(:Judge)
(:Opinion)-[:HAS_PARAGRAPH]->(:OpinionParagraph)
(:Opinion)-[:HAS_HOLDING]->(:Holding)
(:Holding)-[:SUPPORTED_BY]->(:OpinionParagraph)
(:Opinion)-[:HAS_ISSUE]->(:Issue)
(:Opinion)-[:HAS_DISPOSITION]->(:Disposition)
(:Opinion)-[:APPLIES_STANDARD]->(:StandardOfReview)

(:OpinionParagraph)-[:MENTIONS_CITATION]->(:CitationMention)
(:Opinion)-[:INTERPRETS]->(:LegalTextIdentity)
(:Opinion)-[:APPLIES]->(:LegalTextIdentity)
(:Opinion)-[:CONSTRUES]->(:LegalTextIdentity)

(:Opinion)-[:FOLLOWS]->(:Opinion)
(:Opinion)-[:DISTINGUISHES]->(:Opinion)
(:Opinion)-[:OVERRULES]->(:Opinion)
(:Opinion)-[:LIMITS]->(:Opinion)
(:Opinion)-[:ABROGATED_BY_STATUTE]->(:LegalTextIdentity)
```

## 11.5 Treatment edge properties

```json
{
  "treatment_type": "overruled|distinguished|followed|limited|criticized|superseded_by_statute",
  "confidence": 0.82,
  "extraction_method": "citation_context_classifier",
  "supporting_paragraph_id": "...",
  "review_status": "machine_extracted|human_reviewed"
}
```

---

# 12. Local law graph

Local law is where this becomes valuable.

## 12.1 Local code hierarchy

```text
(:CityCode)
  -[:HAS_TITLE]->
(:LocalCodeTitle)
  -[:HAS_CHAPTER]->
(:LocalCodeChapter)
  -[:HAS_SECTION]->
(:LocalCodeSectionIdentity)
  -[:HAS_VERSION]->
(:LocalCodeSectionVersion)
  -[:CONTAINS]->
(:Provision)
```

Same for county, district, and school policy manuals.

## 12.2 Local law relationships

```text
(:CityOrdinance)-[:ADOPTED_BY]->(:CityCouncil)
(:CityOrdinance)-[:AMENDS]->(:CityCodeSectionIdentity)
(:CityOrdinance)-[:CREATES_VERSION]->(:CityCodeSectionVersion)
(:CityCodeSectionVersion)-[:AUTHORIZED_BY]->(:ORSSectionIdentity)
(:CityCodeSectionVersion)-[:IMPLEMENTS]->(:ORSSectionIdentity)
(:CityCodeSectionVersion)-[:PREEMPTED_BY]->(:ORSSectionIdentity)
(:CityCodeSectionVersion)-[:ENFORCED_BY]->(:PublicBody)
(:CityCodeSectionVersion)-[:APPLIES_IN]->(:City)
(:CityCodeSectionVersion)-[:APPLIES_TO_GEO]->(:Zone)
```

## 12.3 Policy/manual graph

```text
(:PolicyManual)-[:HAS_SECTION]->(:PolicySection)
(:PolicySection)-[:IMPLEMENTS]->(:LegalTextIdentity)
(:PolicySection)-[:ISSUED_BY]->(:PublicBody)
(:PolicySection)-[:APPLIES_TO_ROLE]->(:LegalActor)
(:PolicySection)-[:CREATES_PROCEDURE]->(:Procedure)
(:PolicySection)-[:ENFORCED_BY]->(:PublicBody)
```

---

# 13. Agency / administrative order graph

## 13.1 OAR model

```text
(:OARChapter)
  -[:HAS_DIVISION]->
(:OARDivision)
  -[:HAS_RULE]->
(:OARRuleIdentity)
  -[:HAS_VERSION]->
(:OARRuleVersion)
  -[:CONTAINS]->
(:Provision)
```

## 13.2 Rulemaking relationships

```text
(:Agency)-[:PROMULGATED]->(:OARRuleVersion)
(:AdministrativeOrder)-[:ADOPTS]->(:OARRuleIdentity)
(:AdministrativeOrder)-[:AMENDS]->(:OARRuleIdentity)
(:AdministrativeOrder)-[:REPEALS]->(:OARRuleIdentity)
(:AdministrativeOrder)-[:TEMPORARILY_SUSPENDS]->(:OARRuleIdentity)
(:OARRuleVersion)-[:IMPLEMENTS]->(:ORSSectionIdentity)
(:OARRuleVersion)-[:INTERPRETS]->(:ORSSectionIdentity)
(:OARRuleVersion)-[:AUTHORIZED_BY]->(:ORSSectionIdentity)
(:OARRuleVersion)-[:SUPERSEDES]->(:OARRuleVersion)
```

---

# 14. Legal semantics graph

This is derived. Never treat it as authoritative unless linked back to source text.

## 14.1 Semantic labels

```text
:LegalActor
:RegulatedEntity
:ProtectedClass
:LegalAction
:Obligation
:Permission
:Prohibition
:Right
:Power
:Duty
:Condition
:Exception
:Deadline
:Penalty
:Remedy
:Procedure
:NoticeRequirement
:FilingRequirement
:Standard
:Threshold
:Fee
:Tax
:License
:Permit
:AppealRight
:EnforcementMechanism
:Defense
:Element
:BurdenOfProof
:StandardOfReview
```

## 14.2 Semantic node properties

```json
{
  "semantic_id": "obl:...",
  "semantic_type": "obligation",
  "modality": "must|shall|required|may|prohibited",
  "plain_language": "...",
  "confidence": 0.86,
  "extraction_method": "rules+llm",
  "source_model": "model-name",
  "prompt_hash": "sha256:...",
  "review_status": "machine_extracted",
  "created_at": "..."
}
```

## 14.3 Semantic relationships

```text
(:Provision)-[:EXPRESSES]->(:Obligation)
(:Provision)-[:EXPRESSES]->(:Permission)
(:Provision)-[:EXPRESSES]->(:Prohibition)
(:Obligation)-[:IMPOSED_ON]->(:LegalActor)
(:Obligation)-[:OWED_TO]->(:LegalActor)
(:Obligation)-[:REQUIRES_ACTION]->(:LegalAction)
(:Obligation)-[:HAS_DEADLINE]->(:Deadline)
(:Obligation)-[:SUBJECT_TO]->(:Condition)
(:Exception)-[:EXCEPTS]->(:Obligation)
(:Penalty)-[:TRIGGERED_BY]->(:Violation)
(:Remedy)-[:AVAILABLE_TO]->(:LegalActor)
(:AppealRight)-[:FROM_DECISION_BY]->(:PublicBody)
(:Procedure)-[:HAS_STEP]->(:ProcedureStep)
```

Every semantic node must have:

```text
(:SemanticNode)-[:SUPPORTED_BY]->(:Provision or :OpinionParagraph)
```

---

# 15. Conflict, preemption, and hierarchy graph

This is where the graph gets powerful.

## 15.1 Authority hierarchy

```text
(:ConstitutionProvision)-[:SUPERIOR_TO]->(:Statute)
(:Statute)-[:SUPERIOR_TO]->(:AdministrativeRule)
(:Statute)-[:SUPERIOR_TO]->(:LocalCodeSectionVersion)
(:AdministrativeRule)-[:SUPERIOR_TO]->(:AgencyGuidance)
(:CityCharter)-[:SUPERIOR_TO]->(:CityCodeSectionVersion)
(:CountyCharter)-[:SUPERIOR_TO]->(:CountyCodeSectionVersion)
(:CaseOpinion)-[:INTERPRETS]->(:LegalAuthority)
```

## 15.2 Preemption / conflict

```text
(:LegalAuthority)-[:PREEMPTS]->(:LegalAuthority)
(:LegalAuthority)-[:CONFLICTS_WITH]->(:LegalAuthority)
(:LegalAuthority)-[:EXCEPTION_TO]->(:LegalAuthority)
(:LegalAuthority)-[:SUBJECT_TO]->(:LegalAuthority)
(:LegalAuthority)-[:NOTWITHSTANDING]->(:LegalAuthority)
(:LegalAuthority)-[:LIMITS_APPLICATION_OF]->(:LegalAuthority)
```

Relationship properties:

```json
{
  "basis": "explicit_text|case_law|attorney_general|inferred",
  "supporting_authority_id": "...",
  "confidence": 0.72,
  "review_status": "machine_extracted"
}
```

---

# 16. Authority scoring

Every authority node gets an explicit score and scope.

```text
100  Oregon Constitution
95   Oregon Supreme Court binding opinion
92   Oregon Court of Appeals binding opinion
90   Current ORS
88   Oregon Laws / session laws
85   Current OAR
80   Court rules
75   City/county charter
72   City/county code
70   Adopted ordinance / resolution
68   Special district rule
65   Agency final order
60   AG formal opinion
55   DOJ public records order
50   School board policy
45   Administrative policy/manual
35   Official guidance/FAQ
25   Secondary mirror
10   News/commentary
```

Properties:

```json
{
  "authority_level": 90,
  "binding_scope": "statewide|county|city|district|agency|court",
  "jurisdiction_id": "or:state",
  "official_status": "official|official_online_not_official_print|secondary|unknown",
  "precedential_status": "binding|persuasive|nonprecedential|guidance|unknown",
  "current": true,
  "effective_start": "...",
  "effective_end": null
}
```

---

# 17. Retrieval and embedding graph

Neo4j supports vector indexes for querying embeddings stored on nodes or relationships, and it supports full-text indexes over string properties for legal text search. ([Graph Database & Analytics][7]) ([Graph Database & Analytics][8])

## 17.1 RetrievalArtifact labels

```text
:RetrievalChunk
:EmbeddingRun
:EmbeddingModel
:SearchProfile
:QueryRun
:AnswerRun
:ContextPack
:EvaluationCase
:RetrievalEvaluation
```

## 17.2 RetrievalChunk properties

```json
{
  "chunk_id": "chunk:or:ors:1.002@2025:p3a:contextual:v1",
  "chunk_type": "contextual_provision",
  "text": "...",
  "breadcrumb": "Oregon > ORS > Chapter 1 > ORS 1.002 > subsection (3)(a)",
  "authority_level": 90,
  "jurisdiction_id": "or:state",
  "effective_start": "2025-01-01",
  "effective_end": null,
  "embedding": [0.123, -0.04],
  "embedding_model": "bge-m3",
  "embedding_dim": 1024,
  "embedding_input_hash": "sha256:...",
  "retrieval_profile": "legal-context-v1"
}
```

## 17.3 Retrieval relationships

```text
(:RetrievalChunk)-[:DERIVED_FROM]->(:Provision)
(:RetrievalChunk)-[:DERIVED_FROM]->(:OpinionParagraph)
(:RetrievalChunk)-[:DERIVED_FROM]->(:PolicySection)
(:RetrievalChunk)-[:HAS_PARENT_AUTHORITY]->(:LegalTextVersion)
(:EmbeddingRun)-[:EMBEDDED]->(:RetrievalChunk)
(:QueryRun)-[:RETURNED]->(:RetrievalChunk)
(:AnswerRun)-[:CITED]->(:LegalAuthority)
(:AnswerRun)-[:USED_CONTEXT]->(:ContextPack)
```

## 17.4 Chunk types

```text
atomic_provision
contextual_provision
definition_block
exception_block
deadline_block
penalty_block
citation_context
case_paragraph
case_holding
case_rule_statement
statute_interpretation_block
agency_order_finding
agency_order_conclusion
policy_section
procedure_step
fee_schedule_item
permit_requirement
zoning_standard
property_overlay_rule
meeting_agenda_item
ordinance_adoption_event
```

---

# 18. Neo4j constraints and indexes

## 18.1 Uniqueness constraints

```cypher
CREATE CONSTRAINT legal_text_identity_id IF NOT EXISTS
FOR (n:LegalTextIdentity)
REQUIRE n.canonical_id IS UNIQUE;

CREATE CONSTRAINT legal_text_version_id IF NOT EXISTS
FOR (n:LegalTextVersion)
REQUIRE n.version_id IS UNIQUE;

CREATE CONSTRAINT provision_id IF NOT EXISTS
FOR (n:Provision)
REQUIRE n.provision_id IS UNIQUE;

CREATE CONSTRAINT jurisdiction_id IF NOT EXISTS
FOR (n:Jurisdiction)
REQUIRE n.jurisdiction_id IS UNIQUE;

CREATE CONSTRAINT public_body_id IF NOT EXISTS
FOR (n:PublicBody)
REQUIRE n.body_id IS UNIQUE;

CREATE CONSTRAINT source_document_id IF NOT EXISTS
FOR (n:SourceDocument)
REQUIRE n.source_document_id IS UNIQUE;

CREATE CONSTRAINT citation_mention_id IF NOT EXISTS
FOR (n:CitationMention)
REQUIRE n.citation_mention_id IS UNIQUE;

CREATE CONSTRAINT retrieval_chunk_id IF NOT EXISTS
FOR (n:RetrievalChunk)
REQUIRE n.chunk_id IS UNIQUE;

CREATE CONSTRAINT change_event_id IF NOT EXISTS
FOR (n:ChangeEvent)
REQUIRE n.change_event_id IS UNIQUE;
```

## 18.2 Lookup indexes

```cypher
CREATE INDEX authority_citation IF NOT EXISTS
FOR (n:LegalTextIdentity)
ON (n.citation);

CREATE INDEX authority_scope IF NOT EXISTS
FOR (n:LegalTextVersion)
ON (n.jurisdiction_id, n.effective_start, n.effective_end);

CREATE INDEX provision_path IF NOT EXISTS
FOR (n:Provision)
ON (n.display_citation, n.version_id, n.order_index);

CREATE INDEX source_url IF NOT EXISTS
FOR (n:SourceDocument)
ON (n.url);

CREATE INDEX public_body_jurisdiction IF NOT EXISTS
FOR (n:PublicBody)
ON (n.jurisdiction_id, n.body_type);

CREATE INDEX current_authority IF NOT EXISTS
FOR (n:LegalTextVersion)
ON (n.current, n.authority_level, n.jurisdiction_id);
```

## 18.3 Full-text index

```cypher
CREATE FULLTEXT INDEX legal_text_fulltext IF NOT EXISTS
FOR (n:LegalTextVersion|Provision|OpinionParagraph|RetrievalChunk|Definition)
ON EACH [
  n.citation,
  n.display_citation,
  n.title,
  n.caption,
  n.text,
  n.normalized_text,
  n.breadcrumb,
  n.definition_text
];
```

## 18.4 Vector index

```cypher
CREATE VECTOR INDEX retrieval_chunk_embedding IF NOT EXISTS
FOR (n:RetrievalChunk)
ON n.embedding
OPTIONS {
  indexConfig: {
    `vector.dimensions`: 1024,
    `vector.similarity_function`: 'cosine'
  }
};
```

---

# 19. Critical relationship catalog

Use these relationship families.

## 19.1 Structure

```text
HAS_VERSION
VERSION_OF
CONTAINS
PART_OF
NEXT
PREVIOUS
HAS_PARENT
HAS_CHILD
HAS_TITLE
HAS_CHAPTER
HAS_SECTION
HAS_DIVISION
HAS_RULE
```

## 19.2 Source/provenance

```text
PUBLISHED
FETCHED
NORMALIZED_TO
PARSED
CREATED
DERIVED_FROM
HAS_SOURCE
HAS_LICENSE
HAS_TERMS
HAS_ROBOTS_POLICY
```

## 19.3 Jurisdiction/geography

```text
IN_STATE
IN_COUNTY
LOCATED_IN
WITHIN
OVERLAPS
ADJACENT_TO
APPLIES_IN
APPLIES_TO_GEO
HAS_JURISDICTION_OVER
```

## 19.4 Authority/legal effect

```text
ISSUED_BY
ADOPTED_BY
ENFORCED_BY
ADMINISTERED_BY
AUTHORIZED_BY
IMPLEMENTS
INTERPRETS
APPLIES
CONSTRUES
SUPERIOR_TO
PREEMPTS
CONFLICTS_WITH
SUBJECT_TO
EXCEPTION_TO
LIMITS_APPLICATION_OF
NOTWITHSTANDING
```

## 19.5 Citation

```text
MENTIONS_CITATION
RESOLVES_TO
RESOLVES_TO_VERSION
RESOLVES_TO_RANGE
CITES
CITES_STATUTE
CITES_RULE
CITES_CASE
```

## 19.6 Change/versioning

```text
AMENDS
REPEALS
RENUMBERS
CREATES
CREATES_VERSION
SUPERSEDES
REVIVES
SUNSETS
TEMPORARILY_SUSPENDS
EFFECTIVE_ON
EXPIRES_ON
```

## 19.7 Case law

```text
DECIDED_BY
AUTHORED_BY
HAS_PARAGRAPH
HAS_HOLDING
SUPPORTED_BY
HAS_ISSUE
HAS_DISPOSITION
FOLLOWS
DISTINGUISHES
OVERRULES
LIMITS
CRITICIZES
ABROGATED_BY_STATUTE
```

## 19.8 Semantics

```text
EXPRESSES
IMPOSED_ON
OWED_TO
REQUIRES_ACTION
PERMITS_ACTION
PROHIBITS_ACTION
HAS_DEADLINE
HAS_CONDITION
TRIGGERED_BY
AVAILABLE_TO
CREATES_RIGHT
CREATES_REMEDY
CREATES_PENALTY
HAS_PROCEDURE
HAS_STEP
```

## 19.9 Retrieval/AI

```text
DERIVED_FROM
EMBEDDED
RETURNED
RERANKED
USED_CONTEXT
CITED
SUPPORTED_ANSWER
FAILED_EVAL
PASSED_EVAL
```

---

# 20. Source registry schema

Every source gets registered before crawling.

```json
{
  "source_id": "orleg_ors_2025",
  "name": "Oregon Revised Statutes 2025",
  "publisher": "Oregon Legislature",
  "source_family": "statutes",
  "jurisdiction_id": "or:state",
  "start_url": "...",
  "source_type": "official_online_database",
  "official_status": "official_online_not_official_print",
  "canonical_priority": 100,
  "authority_level_default": 90,
  "crawl_method": "html",
  "parser_profile": "ors_chapter_html_v1",
  "update_frequency": "biennial_with_session_updates",
  "robots_status": "unknown",
  "terms_status": "needs_review",
  "redistribution_status": "needs_review"
}
```

Build adapters:

```text
OregonLegislatureORSAdapter
OregonLawsSessionAdapter
OLISBillsAdapter
OARDRulesAdapter
OARDFilingsAdapter
OJDOwnOpinionsAdapter
CourtListenerAdapter
CountyCodeAdapter
CityCodeAdapter
MunicodeAdapter
CivicPlusAdapter
BoardDocsAdapter
GranicusAgendaAdapter
ArcGISBoundaryAdapter
LocalPdfAdapter
SchoolPolicyAdapter
SpecialDistrictAdapter
```

---

# 21. Legal answer context pack

A graph-backed answer should not send raw top-k chunks straight to the model. It should build a **ContextPack**.

## 21.1 ContextPack contents

```text
Primary authority
Current version
Effective date range
Jurisdiction
Issuer
Source document
Official-status warning
Parent hierarchy
Definitions
Exceptions
Deadlines
Penalties/remedies
Cross-references
Amendment history
Interpreting cases
Implementing rules
Local overlays
Enforcement body
Confidence and gaps
```

## 21.2 ContextPack graph

```text
(:QueryRun)-[:CREATED]->(:ContextPack)
(:ContextPack)-[:PRIMARY_AUTHORITY]->(:LegalAuthority)
(:ContextPack)-[:INCLUDES_DEFINITION]->(:Definition)
(:ContextPack)-[:INCLUDES_EXCEPTION]->(:Exception)
(:ContextPack)-[:INCLUDES_CASE]->(:Opinion)
(:ContextPack)-[:INCLUDES_RULE]->(:OARRuleVersion)
(:ContextPack)-[:INCLUDES_LOCAL_OVERLAY]->(:LocalCodeSectionVersion)
(:AnswerRun)-[:USED_CONTEXT]->(:ContextPack)
```

---

# 22. Core traversal patterns

## 22.1 “What law applies to this address?”

```cypher
MATCH (a:Address {address_id: $address_id})-[:LOCATED_IN|WITHIN*1..3]->(j:Jurisdiction)
MATCH (auth:LegalAuthority)-[:APPLIES_IN]->(j)
WHERE auth.current = true
RETURN auth
ORDER BY auth.authority_level DESC;
```

Expand:

```text
state law
county law
city law
special district law
zoning/overlay
agency rules
case interpretations
enforcement body
```

## 22.2 “What cases interpret this statute?”

```cypher
MATCH (s:LegalTextIdentity {citation: $citation})
MATCH (o:Opinion)-[:INTERPRETS|APPLIES|CONSTRUES]->(s)
RETURN o
ORDER BY o.decision_date DESC, o.authority_level DESC;
```

## 22.3 “What changed in this statute?”

```cypher
MATCH (id:LegalTextIdentity {citation: $citation})-[:HAS_VERSION]->(v:LegalTextVersion)
OPTIONAL MATCH (chg:ChangeEvent)-[:AMENDS|REPEALS|RENUMBERS|CREATES_VERSION]->(id)
RETURN v, chg
ORDER BY v.effective_start DESC;
```

## 22.4 “What local law implements this ORS?”

```cypher
MATCH (s:ORSSectionIdentity {citation: $citation})
MATCH (local:LegalAuthority)-[:IMPLEMENTS|AUTHORIZED_BY|SUPPLEMENTS]->(s)
RETURN local
ORDER BY local.authority_level DESC;
```

## 22.5 “Find controlling definitions”

```cypher
MATCH (p:Provision {provision_id: $provision_id})
MATCH (p)-[:GOVERNED_BY_DEFINITION]->(d:Definition)
RETURN d.term, d.definition_text, d.scope_type;
```

---

# 23. Build order

## Phase 0 — Source registry and jurisdiction base

```text
1. SourceRegistry
2. Oregon state node
3. 36 county nodes
4. 241 city nodes
5. public body registry
6. source/provenance tables
7. crawler/fetcher run tracking
```

## Phase 1 — State law core

```text
1. Oregon Constitution
2. ORS current edition
3. ORS archives
4. Oregon Laws/session laws
5. statutes affected by measures
6. bills/amendments/measure summaries
7. amendment/change-event graph
```

## Phase 2 — Rules and agencies

```text
1. OAR current rules
2. OARD filings
3. Oregon Bulletin
4. administrative orders
5. agency guidance/manuals
6. final orders/declaratory rulings
```

## Phase 3 — Courts

```text
1. Oregon Supreme Court opinions
2. Oregon Court of Appeals opinions
3. Tax Court decisions
4. ORCP / UTCR / ORAP / SLR
5. case citation graph
6. holdings/rule statements/treatments
```

## Phase 4 — Local law

```text
1. county codes
2. city codes
3. charters
4. ordinances/resolutions
5. policies/manuals
6. fee schedules
7. public works standards
8. zoning/development codes
```

## Phase 5 — Geography/property

```text
1. city/county boundaries
2. parcels
3. zoning districts
4. overlays
5. UGBs
6. special districts
7. school districts
8. property-law applicability engine
```

## Phase 6 — Legal semantics and AI

```text
1. obligations
2. permissions
3. prohibitions
4. conditions
5. exceptions
6. deadlines
7. penalties
8. remedies
9. enforcement path
10. appeal path
11. answer/context/eval graph
```

---

# 24. The final mental model

The graph should be able to move in every direction:

```text
Address → jurisdiction → applicable law
Statute → rules → cases → local implementation
Ordinance → state authorization → preemption risk
Case → cited statutes → holdings → later treatment
Rule → agency order → statutory authority → filings
Policy → issuing body → source authority → enforcement path
Question → retrieved chunks → provisions → context pack → cited answer
```

That is how you take advantage of Neo4j.

The ultimate schema is:

```text
Jurisdiction
+ PublicBody
+ SourceDocument
+ LegalTextIdentity
+ LegalTextVersion
+ Provision
+ CitationMention
+ Definition
+ ChangeEvent
+ CaseInterpretation
+ LocalImplementation
+ GeoApplicability
+ LegalSemantics
+ RetrievalChunk
+ ContextPack
+ AnswerRun
```

The core invariant:

> **Every legal answer must trace from question → jurisdiction → authority → version → provision → source → interpretation → effective date → enforcement path.**

[1]: https://www.oregonlegislature.gov/bills_laws/pages/ors.aspx "
	
            Bills and Laws
            
            
            Oregon Revised Statutes
            
        
"
[2]: https://www.oregon.gov/pages/counties.aspx?utm_source=chatgpt.com "Oregon Counties : State of Oregon"
[3]: https://sos.oregon.gov/blue-book/Pages/local/cities/about.aspx "
	State of Oregon: Blue Book - About City Government
"
[4]: https://sos.oregon.gov/archives/Pages/oregon_administrative_rules.aspx "
	State of Oregon: State Archives - Oregon Administrative Rules (OARs)
"
[5]: https://sos.oregon.gov/archives/Pages/oard-faq.aspx "
	State of Oregon: State Archives - OARD Filing FAQ
"
[6]: https://www.courts.oregon.gov/publications/pages/default.aspx "
	Oregon Judicial Department : Publications Program of the State of Oregon Law Library : Publications : State of Oregon
"
[7]: https://neo4j.com/docs/cypher-manual/current/indexes/semantic-indexes/vector-indexes/ "Vector indexes - Cypher Manual"
[8]: https://neo4j.com/docs/cypher-manual/current/indexes/semantic-indexes/full-text-indexes/ "Full-text indexes - Cypher Manual"
The best way is **not** to start with embeddings or Neo4j. Start with a **legal-source ingestion factory** that produces clean, versioned, auditable artifacts. Then load those artifacts into Neo4j, then embed retrieval views.

The build order should be:

```text id="c76kcx"
1. Source registry
2. Crawl frontier
3. Fetch/cache raw artifacts
4. Normalize into canonical text
5. Parse into legal structure
6. Extract citations, definitions, dates, amendments
7. Build graph import files
8. Load Neo4j
9. Generate retrieval chunks
10. Embed chunks
11. Build hybrid search / answer engine
12. Add evals and monitoring
```

The core rule:

> **Use APIs and bulk data first. Crawl only when no API or bulk feed exists. Scrape only through source-specific adapters. Never rely on one generic scraper.**

---

# 1. Start with a source registry

Before crawling anything, create a `source_registry.yaml`.

Each source gets:

```yaml id="x3m6fk"
source_id: or_leg_ors_2025
name: Oregon Revised Statutes 2025
jurisdiction_id: or:state
publisher: Oregon Legislature
source_family: statutes
source_type: official_online_database
start_url: https://www.oregonlegislature.gov/bills_laws/ors/
canonical_priority: 100
authority_level_default: 90
official_status: official_online_not_official_print
disclaimer_required: true
crawl_method: html
parser_profile: ors_chapter_html_v1
update_frequency: biennial_plus_session_updates
robots_status: unknown
terms_status: needs_review
redistribution_status: needs_review
```

Why: Oregon’s ORS page says users should verify legal accuracy against official Oregon law sources, so your graph needs source disclaimers and verification flags from day one. ([Oregon Legislature][1])

---

# 2. Use source adapters, not a universal crawler

Build one crawler engine, but many adapters.

```text id="ygbnyx"
Crawler engine:
  queue, fetch, cache, retry, hash, schedule

Source adapters:
  know how each source is structured
```

Initial adapters:

```text id="7uf2us"
OregonLegislatureORSAdapter
PublicLawOregonAdapter
OregonLawsSessionAdapter
OLISODataAdapter
OARDAdapter
OJDCasesAdapter
CourtListenerAdapter
CityCodeAdapter
CountyCodeAdapter
MunicodeAdapter
CivicPlusAdapter
BoardDocsAdapter
GranicusAdapter
ArcGISGeoAdapter
PDFAdapter
```

The adapter decides:

```text id="oov5l7"
what URLs to discover
what pages are canonical
what pages are indexes
what content is legal text
what content is metadata
what parser profile to use
what update frequency applies
```

---

# 3. Crawl in priority order

## Priority 1 — State legal core

Start with:

```text id="3s6doc"
ORS current
ORS archive
Oregon Laws / session laws
OLIS bills and amendments
OAR current rules
OAR filings
Oregon Constitution
OJD opinions
Court rules
```

The Oregon Legislature provides an OData API for legislative measure, committee, and legislative member data, so use that instead of scraping OLIS wherever possible. ([Oregon Legislature][2])

OARD should be a first-class source because the Secretary of State describes it as containing Oregon Administrative Rules and filings in one searchable location, and the OARD FAQ says it houses official copies of OARs and filings. ([Oregon Secretary of State][3])

Oregon Judicial Department opinions should be ingested from OJD first because OJD says Supreme Court, Court of Appeals, and Tax Court decisions are posted weekly or as soon as available on the day issued. ([Oregon Courts][4])

## Priority 2 — Case law bulk backfill

Use CourtListener / Free Law Project for case-law scale. CourtListener describes itself as a free legal research website with millions of legal opinions, and Free Law Project describes CourtListener as an accessible archive of opinions, oral arguments, judges, financial disclosures, and federal filings. ([CourtListener][5])

Use CourtListener for:

```text id="ejdqvz"
historical Oregon cases
federal cases interpreting Oregon law
citation graph expansion
case metadata
bulk opinion ingestion
later citation treatment analysis
```

## Priority 3 — Jurisdiction registry

Build all Oregon jurisdictions before deep local scraping.

```text id="q7urx2"
State
36 counties
241 incorporated cities
special districts
school districts
court districts
service districts
zoning districts
```

Oregon’s Blue Book says Oregon has 241 incorporated cities. ([Oregon Secretary of State][6]) The League of Oregon Cities’ handbook describes Oregon’s local-government landscape as including 36 counties, 241 cities, 197 school districts, at least seven regional governments, and roughly 1,000 special districts. ([Oregon Cities][7])

## Priority 4 — Local law

Do not try to scrape all 241 cities equally at first.

Go deep first:

```text id="c7y6c4"
Portland
Multnomah County
Washington County
Clackamas County
Gresham
Beaverton
Hillsboro
Oregon City
Lake Oswego
Metro
TriMet
major school districts
major fire/water/sanitary districts
```

Then expand.

## Priority 5 — Geography/property

Add:

```text id="ak071y"
city limits
county boundaries
tax lots
parcels
zoning
overlays
urban growth boundaries
special district boundaries
school district boundaries
```

Oregon GEOHub exposes city limit data, and ORMAP provides statewide tax lot viewing and assessor map downloads. ([Oregon GEOHub][8])

---

# 4. Store immutable raw artifacts first

Never parse directly into Neo4j.

For every fetched item, store:

```text id="elzrqc"
raw HTML / PDF / JSON / XML / CSV
HTTP headers
source URL
fetched_at
ETag
Last-Modified
content hash
normalized text hash
parser version
source registry version
```

Use this directory shape:

```text id="o7infb"
data/
  raw/
    or_leg/ors/2025/ors001.html
    oard/rules/2026/...
    ojd/opinions/2026/...
  normalized/
    or_leg/ors/2025/chapter_001.json
  parsed/
    ors/sections/ors_1.002@2025.json
  graph_import/
    nodes/
    relationships/
  chunks/
  embeddings/
```

Also keep WARC or WARC-like records if possible. You want courtroom-grade reproducibility:

```text id="fk37wq"
URL → raw bytes → normalized text → parsed legal nodes → graph records
```

---

# 5. Normalize into canonical legal JSON

Every source adapter should output the same general object model.

```json id="zbz63c"
{
  "source_document_id": "src:orleg:ors:chapter:001@2025",
  "jurisdiction_id": "or:state",
  "authority_family": "ORS",
  "edition": "2025",
  "canonical_priority": 100,
  "official_status": "official_online_not_official_print",
  "items": [
    {
      "canonical_id": "or:ors:1.002",
      "version_id": "or:ors:1.002@2025",
      "citation": "ORS 1.002",
      "title": "Supreme Court; Chief Justice as administrative head...",
      "status": "active",
      "text": "...",
      "history": "...",
      "provisions": [],
      "citations": [],
      "definitions": [],
      "notes": []
    }
  ]
}
```

Your uploaded Chapter 1 sample confirms why this parser has to handle more than normal sections: it contains chapter headings, a section table, full section text, notes, amendment histories, repealed entries, renumbered entries, and ORS cross-references. 

---

# 6. Use parser profiles

Each source gets a parser profile.

```text id="ptl2i4"
ors_chapter_html_v1
public_law_section_html_v1
oard_rule_html_v1
ojd_opinion_pdf_v1
courtlistener_json_v1
portland_code_html_v1
municode_html_v1
civicplus_html_v1
boarddocs_agenda_v1
arcgis_geojson_v1
generic_pdf_policy_v1
```

Each parser profile must emit:

```text id="dpaxij"
LegalTextIdentity
LegalTextVersion
Provision tree
CitationMention nodes
Definition nodes
ChangeEvent candidates
Source provenance links
RetrievalChunk candidates
```

---

# 7. Make crawling incremental

Do not recrawl everything constantly.

Use this pattern:

```text id="6qbhxi"
discover URLs
  ↓
compare against known URL registry
  ↓
HEAD request if supported
  ↓
check ETag / Last-Modified
  ↓
fetch only if changed
  ↓
hash raw bytes
  ↓
skip parse if raw_hash unchanged
  ↓
skip embed if chunk_input_hash unchanged
```

Recommended schedules:

```text id="4us2m2"
ORS current: weekly during session, monthly otherwise
ORS archive: once, then annual check
Oregon Laws/session laws: daily during session, weekly after session
OLIS/OData: daily during session
OAR current: daily or weekly
OAR filings/Bulletin: daily or weekly
OJD opinions: daily, especially Wed/Thu
CourtListener: daily/weekly via API/bulk
City/county codes: weekly/monthly
Ordinances/resolutions/agendas: daily/weekly
Policies/manuals: monthly
Geo boundaries/parcels: monthly/quarterly
```

---

# 8. Separate discovery from fetching

Use a crawl frontier.

```sql id="vrrhqu"
crawl_url(
  url,
  source_id,
  jurisdiction_id,
  discovered_from,
  url_type,
  priority,
  canonical_priority,
  status,
  next_fetch_at,
  last_fetch_at,
  last_status_code,
  last_hash,
  parser_profile
)
```

URL types:

```text id="hae3qb"
index
chapter
section
rule
opinion
pdf
ordinance
agenda
minutes
policy
geojson
api_endpoint
bulk_file
```

---

# 9. Build a change-detection pipeline

Every parsed object gets content hashes.

```text id="2ciowl"
raw_hash
normalized_hash
legal_text_hash
provision_hash
citation_hash
definition_hash
chunk_input_hash
embedding_hash
```

If a city code section changes, emit:

```text id="uh9jrk"
new LegalTextVersion
SUPERSEDES old version
ChangeEvent candidate
re-embed affected chunks only
invalidate retrieval cache
run regression evals
```

---

# 10. Load Neo4j through import files, not direct scraping writes

Best pattern:

```text id="m9mujg"
crawler/fetcher writes raw artifacts
parser writes canonical JSON
normalizer writes graph records
loader writes Neo4j
```

Generate deterministic JSONL/CSV:

```text id="8b5hy7"
nodes_jurisdiction.jsonl
nodes_public_body.jsonl
nodes_source_document.jsonl
nodes_legal_identity.jsonl
nodes_legal_version.jsonl
nodes_provision.jsonl
nodes_citation_mention.jsonl
nodes_definition.jsonl
nodes_change_event.jsonl
nodes_retrieval_chunk.jsonl

rels_contains.jsonl
rels_has_version.jsonl
rels_derived_from.jsonl
rels_mentions_citation.jsonl
rels_resolves_to.jsonl
rels_defines.jsonl
rels_amends.jsonl
rels_supersedes.jsonl
rels_applies_in.jsonl
```

Then load with idempotent Cypher or Neo4j batch import.

---

# 11. Best system architecture

Use a Rust-first architecture.

```text id="jiejdr"
services/
  source-registry
  crawler-frontier
  fetcher
  artifact-store
  parser-workers
  citation-resolver
  definition-resolver
  change-detector
  graph-record-builder
  neo4j-loader
  chunk-builder
  embedding-worker
  search-api
  legal-context-api
  eval-runner
```

Infrastructure:

```text id="zwluvv"
Postgres:
  crawl frontier, source registry, job state, hash index

Object storage:
  raw artifacts, normalized JSON, PDFs, WARC, chunks

Neo4j:
  legal graph, citation graph, authority graph, retrieval graph

PostGIS:
  geometry-heavy data, parcels, boundaries

Queue:
  NATS, Redpanda, Kafka, or Postgres queue initially

Search:
  Neo4j full-text + vector indexes first
  optional OpenSearch later for heavy document search
```

---

# 12. Crawler worker flow

```text id="f92klc"
1. Pop URL job from crawl frontier
2. Check robots/terms/source policy
3. Rate limit by host/source
4. Fetch with stable User-Agent
5. Save raw artifact
6. Compute raw hash
7. If unchanged, mark skipped
8. Normalize content
9. Compute normalized hash
10. Parse with source-specific parser profile
11. Validate canonical JSON schema
12. Extract citations/definitions/amendments
13. Emit graph records
14. Load or stage to Neo4j
15. Generate retrieval chunks
16. Embed changed chunks
17. Run source-specific evals
18. Mark job complete
```

---

# 13. Use strict source priority

For the same legal text, choose canonical text in this order:

```text id="3k6r8m"
1. Official government source
2. Official government API/bulk file
3. Official government PDF
4. Official government HTML
5. Official third-party hosted code platform
6. Nonprofit bulk source
7. Public mirror
8. Secondary commentary
```

Example:

```text id="98gg31"
ORS text:
  Oregon Legislature = canonical
  Public.Law = secondary parser aid / comparison

Cases:
  Oregon Judicial Department = canonical current Oregon opinions
  CourtListener = bulk backfill / citation network / secondary source

OAR:
  OARD = canonical

Geography:
  official agency GIS source = canonical
  third-party map = secondary
```

---

# 14. Cross-validate important sources

For ORS:

```text id="it8ylq"
Oregon Legislature text
vs.
Public.Law text
vs.
your parsed text
```

For cases:

```text id="hlxdl0"
OJD PDF
vs.
CourtListener opinion
vs.
your paragraph parser
```

For local codes:

```text id="8qou7t"
city clerk/city code page
vs.
Municode/CivicPlus mirror
vs.
ordinance history
```

Classify diffs:

```text id="xrfoj5"
MATCH
FORMAT_ONLY_DIFF
MINOR_TEXT_DIFF
SUBSTANTIVE_DIFF
MISSING_IN_CANONICAL
MISSING_IN_SECONDARY
NEEDS_REVIEW
```

Substantive diffs go into a review queue.

---

# 15. Build extraction in layers

Do not try to extract everything with an LLM first.

Use this order:

```text id="dzxj8o"
1. Regex / structural parser
2. Deterministic citation resolver
3. Deterministic date/effective-date parser
4. Deterministic definition extractor
5. Deterministic amendment/event parser
6. Lightweight classifier
7. LLM extraction only for hard semantic objects
```

Machine-extracted semantic objects need:

```text id="s6jy97"
confidence
extraction_method
model
prompt_hash
source provision
review_status
```

Never let an LLM-created `Obligation` or `Exception` become “law” without a `SUPPORTED_BY` edge back to exact text.

---

# 16. Chunk after graph parsing

Bad order:

```text id="eouid8"
HTML → text chunks → embeddings → graph
```

Correct order:

```text id="vhdc7g"
HTML/PDF/API
→ canonical legal JSON
→ provision tree
→ citations/definitions/amendments
→ graph
→ retrieval chunks
→ embeddings
```

Chunk types:

```text id="mtfp2g"
atomic_provision
contextual_provision
definition_block
exception_block
deadline_block
penalty_block
citation_context
case_holding
case_rule_statement
case_paragraph
agency_order_conclusion
policy_section
ordinance_adoption_event
zoning_standard
permit_requirement
fee_schedule_item
```

Every chunk maps back to authoritative nodes:

```text id="svrj0e"
(:RetrievalChunk)-[:DERIVED_FROM]->(:Provision)
(:RetrievalChunk)-[:DERIVED_FROM]->(:OpinionParagraph)
(:RetrievalChunk)-[:HAS_PARENT_AUTHORITY]->(:LegalTextVersion)
```

---

# 17. Build retrieval as hybrid graph traversal

The search API should not be “top 20 vector chunks.”

It should run:

```text id="28zsoz"
1. Citation parser
2. Exact legal citation lookup
3. Full-text search
4. Vector search
5. Jurisdiction filter
6. Effective-date filter
7. Authority ranking
8. Graph expansion
9. Reranking
10. ContextPack assembly
```

Graph expansion should pull:

```text id="74dw0q"
parent authority
definitions
exceptions
citations
implementing rules
interpreting cases
amendment history
local overlays
enforcement body
source provenance
```

---

# 18. Do Portland metro first

The fastest useful path:

## V0 — ORS + graph skeleton

```text id="i2o27v"
Oregon Legislature ORS
Public.Law comparison
Section/provision parser
Citation resolver
Neo4j loader
Basic search
```

## V1 — OAR + cases

```text id="k18q26"
OARD current rules
OAR filings
OJD opinions
CourtListener backfill
Case citation graph
```

## V2 — Legislative amendments

```text id="9qdltn"
Oregon Laws
OLIS OData
Bills/amendments/votes
ChangeEvent graph
Time-travel law lookup
```

## V3 — Portland metro local law

```text id="bo5wrt"
Portland code/charter/policies
Multnomah County code
Washington County code
Clackamas County code
Metro
TriMet
major school districts
major special districts
```

## V4 — Property/geography

```text id="eltcwc"
city/county boundaries
parcels/tax lots
zoning
overlays
permit rules
land-use decisions
LUBA
```

## V5 — all Oregon local law

```text id="ihyf1j"
remaining counties
remaining cities
special districts
school policies
agency manuals
fee schedules
ordinances
minutes
```

---

# 19. Minimal MVP you can build now

Start with this exact MVP:

```text id="xsq17c"
1. Source registry
2. ORS chapter URL discovery
3. Fetch Oregon Legislature ORS chapter HTML
4. Fetch matching Public.Law section/chapter pages
5. Parse ORS chapter into sections
6. Parse sections into provisions
7. Extract ORS citations
8. Extract definition blocks
9. Detect repealed/renumbered/status sections
10. Load Neo4j:
    Edition
    Chapter
    LegalTextIdentity
    LegalTextVersion
    Provision
    CitationMention
    Definition
    SourceDocument
    RetrievalChunk
11. Create full-text index
12. Create vector index
13. Build search API:
    /lookup?citation=ORS 1.002
    /search?q=...
    /citations/incoming?citation=...
    /context?citation=...
```

This is the first real foundation. Everything else plugs into the same pipeline.

---

# 20. Practical anti-failure rules

```text id="1yysuo"
Do not scrape faster than humans need.
Do not ignore robots or terms.
Do not embed before parsing.
Do not use generic chunks as legal authority.
Do not overwrite old versions.
Do not delete repealed or renumbered sections.
Do not treat Public.Law as canonical over the Legislature.
Do not treat CourtListener as canonical over OJD for current Oregon opinions.
Do not mix city/county/state law without jurisdiction filters.
Do not answer legal questions without source provenance.
Do not rely on LLM extraction without source support.
```

---

# 21. The winning build strategy

Build it as a **repeatable legal ingestion factory**:

```text id="mxejna"
source registry
→ adapter
→ raw artifact
→ normalized artifact
→ parsed legal JSON
→ graph records
→ Neo4j
→ retrieval chunks
→ embeddings
→ context pack
→ cited answer
```

The graph will be good only if the ingestion is good.

The best first milestone is:

> **Crawl and parse all current ORS into versioned sections, provisions, citations, definitions, source provenance, and retrieval chunks.**

Then add OAR.
Then add Oregon Laws/amendments.
Then add OJD/CourtListener cases.
Then add Portland metro local law.
Then scale to every city, county, district, policy, code, and geography layer.

[1]: https://www.oregonlegislature.gov/bills_laws/pages/ors.aspx?utm_source=chatgpt.com "Bills and Laws Oregon Revised Statutes"
[2]: https://www.oregonlegislature.gov/citizen_engagement/Pages/data.aspx?utm_source=chatgpt.com "Citizen Engagement Oregon Legislative Data"
[3]: https://sos.oregon.gov/archives/Pages/oregon_administrative_rules.aspx?utm_source=chatgpt.com "State Archives - Oregon Administrative Rules (OARs)"
[4]: https://www.courts.oregon.gov/publications/pages/default.aspx?utm_source=chatgpt.com "Publications Program of the State of Oregon Law Library"
[5]: https://www.courtlistener.com/?utm_source=chatgpt.com "CourtListener.com: Non-Profit Free Legal Search Engine and ..."
[6]: https://sos.oregon.gov/blue-book/Pages/local/cities/about.aspx?utm_source=chatgpt.com "Blue Book - About City Government - State of Oregon"
[7]: https://www.orcities.org/application/files/5817/2797/7376/Handbook_-_Chapter_1_Nature_of_Cities.pdf?utm_source=chatgpt.com "Oregon Municipal Handbook – Chapter 1: Nature of Cities"
[8]: https://geohub.oregon.gov/datasets/oregon-geo%3A%3Acity-limits?utm_source=chatgpt.com "City Limits | Oregon GEOHub"
