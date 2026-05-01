# Oregon Legislature OData Ingestion

This document describes the Oregon Legislature OData service as a source for measures, sessions, committees, legislative actions, votes, testimony, and bill-document metadata. This source is the missing bridge between ORSGraph's current ORS text graph and richer legislative history/currentness data.

## Current Implementation Status

`or_leg_odata` is implemented as a registry-driven connector in `crates/ors-crawler-v0/src/oregon_leg_odata.rs`. It is selected by `source-ingest --source-id or_leg_odata` through `crates/ors-crawler-v0/src/connectors/mod.rs`.

The connector currently:

- Discovers `$metadata`, `LegislativeSessions`, and session-scoped entity sets for measures, documents, history actions, sponsors, committees, legislators, meetings, and votes.
- Uses `--session-key` for explicit sessions such as `2025R1`; if absent, it defaults to `<edition_year>R1`.
- Preserves raw artifacts under `data/sources/or_leg_odata/raw/`.
- Parses legacy OData JSON shapes including `d.results`, `d`, `value`, `results`, top-level arrays, and object keys matching the entity set.
- Emits normalized legislative JSONL rows and source-backed projections into existing `SourceDocument`, `SessionLaw`, `StatusEvent`, `LineageEvent`, and `LegalActor` contracts.
- Records `odata_entity_set_stats.jsonl` row counts and parser diagnostics.
- Detects OData next links and warns, but does not yet follow pagination automatically.

## Source Service

Service root:

```text
https://api.oregonlegislature.gov/odata/ODataService.svc/
```

Metadata endpoint:

```text
https://api.oregonlegislature.gov/odata/ODataService.svc/$metadata
```

The service document advertises these entity sets:

```text
LegislativeSessions
Measures
Committees
CommitteeMeetings
CommitteeAgendaItems
CommitteeStaffMembers
CommitteeMeetingDocuments
ConveneTimes
FloorSessionAgendaItems
Legislators
MeasureAnalysisDocuments
MeasureDocuments
MeasureHistoryActions
MeasureSponsors
CommitteeProposedAmendments
FloorLetters
CommitteeVotes
MeasureVotes
CommitteeMembers
CommitteePublicTestimonies
```

The local `OLOData-Model.pdf` diagram is useful for relationship review, but the parser contract should be generated from `$metadata` and verified against sampled entity responses.

## OData Version

Although OData v4.01 is the modern standard, the Oregon Legislature endpoint exposes legacy Microsoft WCF Data Services metadata:

```xml
<edmx:Edmx Version="1.0">
  <edmx:DataServices
    m:DataServiceVersion="1.0"
    m:MaxDataServiceVersion="3.0">
```

Implementation should use OData v2/v3-compatible conventions:

- Keep the `$` prefix on system query options such as `$filter`, `$select`, `$expand`, `$orderby`, `$top`, and `$skip`.
- Use case-sensitive property and entity-set names from `$metadata`.
- Use parentheses key syntax, including named composite keys.
- Use legacy `Edm.DateTime` filter literals when filtering by dates.
- Normalize both likely legacy JSON shapes and any service-specific variants during response parsing.

Do not rely on OData v4-only features such as `$compute`, key-as-segment URLs, unprefixed case-insensitive system query options, the `in` operator, JSON batch, or v4-style payload control information until the live service proves support.

## Core Entity Keys

The high-value entity keys from `$metadata` are:

```text
LegislativeSession
  SessionKey

Measure
  MeasureNumber
  MeasurePrefix
  SessionKey

Committee
  CommitteeCode
  SessionKey

CommitteeMeeting
  CommitteeCode
  MeetingDate
  SessionKey

CommitteeAgendaItem
  CommitteeAgendaItemId

CommitteeStaff
  CommitteeStaffId

CommitteeMeetingDocument
  CommitteeMeetingDocumentId

ConveneTime
  Chamber
  SessionDate
  SessionKey

FloorSessionAgendaItem
  AgendaId

Legislator
  LegislatorCode
  SessionKey

MeasureAnalysisDocument
  MeasureAnalysisId

MeasureDocument
  MeasureNumber
  MeasurePrefix
  SessionKey
  VersionDescription

MeasureHistoryAction
  MeasureHistoryId

MeasureSponsor
  MeasureSponsorId

CommitteeProposedAmendment
  ProposedAmendmentId

FloorLetter
  FloorLetterId

CommitteeVote
  CommitteeVoteId

MeasureVote
  MeasureVoteId

CommitteeMember
  CommitteeCode
  CreatedDate
  Title

CommitteePublicTestimony
  CommitteeCode
  CommTestId
  CreatedDate
  MeetingDate
  PdfCreatedFlag
  SubmitterFirstName
  SubmitterLastName
```

Composite measure key example:

```text
Measures(MeasureNumber=2001,MeasurePrefix='HB',SessionKey='2025R1')
```

Session key example:

```text
LegislativeSessions('2025R1')
```

## Query Patterns

Start by discovering sessions:

```text
GET /odata/ODataService.svc/LegislativeSessions?$orderby=BeginDate desc
GET /odata/ODataService.svc/LegislativeSessions?$filter=DefaultSession eq true
```

Fetch measures for one session:

```text
GET /odata/ODataService.svc/Measures?$filter=SessionKey eq '2025R1'&$select=SessionKey,MeasurePrefix,MeasureNumber,CatchLine,MeasureSummary,ChapterNumber,CurrentLocation,CurrentCommitteeCode,EffectiveDate,EmergencyClause,Vetoed,CreatedDate,ModifiedDate
```

Fetch measure history and documents for one session:

```text
GET /odata/ODataService.svc/MeasureHistoryActions?$filter=SessionKey eq '2025R1'&$orderby=ActionDate
GET /odata/ODataService.svc/MeasureDocuments?$filter=SessionKey eq '2025R1'
GET /odata/ODataService.svc/MeasureAnalysisDocuments?$filter=SessionKey eq '2025R1'
```

Fetch sponsors, votes, and committee context:

```text
GET /odata/ODataService.svc/MeasureSponsors?$filter=SessionKey eq '2025R1'
GET /odata/ODataService.svc/MeasureVotes?$filter=SessionKey eq '2025R1'
GET /odata/ODataService.svc/CommitteeVotes?$filter=SessionKey eq '2025R1'
GET /odata/ODataService.svc/Committees?$filter=SessionKey eq '2025R1'
GET /odata/ODataService.svc/Legislators?$filter=SessionKey eq '2025R1'
```

For date filters, use the legacy `datetime'...'` literal form if the service rejects plain ISO strings:

```text
GET /odata/ODataService.svc/MeasureHistoryActions?$filter=ActionDate ge datetime'2025-01-01T00:00:00'
```

## Graph Role

This source should not replace the official ORS HTML corpus. It should enrich it.

Primary uses:

- Link ORS source notes and derived `SessionLaw` nodes to the underlying measure records.
- Improve bill-number, chapter-number, effective-date, emergency-clause, veto, and current-location data.
- Add source-backed measure history actions as legislative `StatusEvent` or `LineageEvent` inputs.
- Add committee and floor votes for legislative-history explanations.
- Add document URLs for enrolled measures, measure versions, staff measure summaries, fiscal impact, revenue impact, amendments, exhibits, floor letters, and public testimony.

Recommended graph mapping:

```text
LegislativeSessions      -> LegislativeSession or CorpusEdition support rows
Measures                 -> LegislativeMeasure, plus links to SessionLaw where ChapterNumber is present
MeasureDocuments         -> SourceDocument rows with measure/version provenance
MeasureAnalysisDocuments -> SourceDocument rows with analysis/fiscal/revenue provenance
MeasureHistoryActions    -> StatusEvent and LineageEvent candidates
MeasureSponsors          -> LegalActor links or dedicated sponsorship rows
Legislators              -> LegalActor rows
Committees               -> PublicBody or LegislativeCommittee rows
CommitteeMeetings        -> LegislativeEvent rows
CommitteeAgendaItems     -> LegislativeEvent agenda/action rows
MeasureVotes             -> LegislativeVote rows
CommitteeVotes           -> LegislativeVote rows
CommitteePublicTestimonies -> SourceDocument rows and testimony provenance
```

The first implementation can stay conservative: cache raw OData responses, emit a small set of normalized legislative JSONL rows, and only project into existing `SessionLaw`, `SourceDocument`, `StatusEvent`, `LineageEvent`, and `LegalActor` rows where the mapping is unambiguous. Dedicated legislative nodes can follow once the sampled data shape is verified.

## Output Shape

The registry-driven output folder is:

```text
data/sources/or_leg_odata/
  raw/
  normalized/
  graph/
  qc/report.json
  manifest.json
  stats.json
```

Current normalized graph files:

```text
legislative_sessions.jsonl
legislative_measures.jsonl
legislative_measure_documents.jsonl
legislative_measure_versions.jsonl
legislative_measure_history_actions.jsonl
legislative_measure_sponsors.jsonl
legislative_committees.jsonl
legislative_legislators.jsonl
legislative_committee_meetings.jsonl
legislative_votes.jsonl
vote_events.jsonl
vote_records.jsonl
source_documents.jsonl
session_laws.jsonl
status_events.jsonl
lineage_events.jsonl
legal_actors.jsonl
legislative_edges.jsonl
odata_entity_sets.jsonl
odata_metadata_summary.jsonl
odata_entity_set_stats.jsonl
parser_diagnostics.jsonl
```

Stable IDs should be deterministic and include the session key:

```text
orleg:session:2025R1
orleg:measure:2025R1:HB:2001
orleg:measure-document:2025R1:HB:2001:A-engrossed
orleg:history-action:2025R1:123456
orleg:legislator:2025R1:SMITH
orleg:committee:2025R1:HJD
orleg:vote:measure:2025R1:987654
```

## Parser Pipeline

1. Fetch and cache the service document and `$metadata`.
2. Parse `$metadata` into a schema summary with entity sets, keys, properties, navigation properties, and referential constraints.
3. Discover sessions and choose either `DefaultSession` or an explicit `--session-key`.
4. Fetch session-scoped entity sets as separate raw artifacts. Current code records diagnostics if a next link is present; automatic paging is still pending.
5. Write raw responses before normalization.
6. Normalize property names and scalar types, especially `Edm.DateTime`, `Edm.Decimal`, nullable booleans, and URLs.
7. Build deterministic IDs.
8. Join measures to documents, history actions, sponsors, votes, committees, and legislators by `SessionKey`, `MeasurePrefix`, `MeasureNumber`, and related keys.
9. Reconcile measures with existing `SessionLaw` rows using chapter number, year/session, and bill number.
10. Emit normalized JSONL and diagnostics.
11. Add loader and materialization queries only after the normalized graph rows have stable contracts.

## QC Checks

Minimum QC for a session ingest:

- Every row has `SessionKey` or an explicit diagnostic explaining why not.
- Every measure has `MeasurePrefix`, `MeasureNumber`, and `SessionKey`.
- Every document URL is either absolute or marked invalid in `parser_diagnostics.jsonl`.
- Every history action with a measure key joins to a measure.
- Every vote with a measure key joins to a measure.
- Every sponsor with a legislator or committee code either joins to a known actor/body or emits a diagnostic.
- Session-law reconciliation is confidence-scored and never silently overwrites ORS-derived history.
- Raw response count and normalized row count are reported per entity set.

## Implementation Notes

This is implemented through the registry-driven source ingest command:

```text
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- source-ingest \
  --source-id or_leg_odata \
  --out data/sources \
  --session-key 2025R1 \
  --mode all
```

Use the existing project conventions:

- Rust-first parser/client under `crates/ors-crawler-v0/src/`.
- Raw artifacts before graph output.
- JSONL graph outputs under `data/sources/or_leg_odata/graph/`.
- `parser_diagnostics.jsonl` for non-fatal data shape and join issues.
- Seed dry-run before Neo4j writes.
- Live Neo4j materialization only after the JSONL contract is stable.

Offline fixture command:

```text
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- source-ingest \
  --source-id or_leg_odata \
  --fixture-dir /private/tmp/orsgraph-odata-fixture \
  --out /private/tmp/orsgraph-odata-out \
  --session-key 2025R1 \
  --mode all \
  --allow-network false \
  --fail-on-qc
```

Fixture lookup currently supports `.json`, `.html`, `.txt`, and `.pdf`. Use `metadata.txt` for `$metadata` XML fixtures.

## Known Metadata Issues To Preserve

The metadata contains source spelling quirks that should be preserved in raw data and normalized only in derived fields:

```text
CommitteeAgendaItem.CommitteCode
MeasureAnalysisDocument.CommittteCode
MeasureSponsor.LegislatoreCode
State.Or.Leg.API.Entites
```

Do not "fix" these names in requests. The service property names are the request contract.
