# Source Registry Schema

This file defines the documentation schema for `source-registry.md`. It is not a Rust type contract yet. It is the checklist future connector configs, source manifests, and QC reports should satisfy.

## YAML Shape

```yaml
source_id: or_leg_odata
name: Oregon Legislature OData API
owner: Oregon Legislature
jurisdiction: or:state
source_type: api
access: free
official_status: official
data_types:
  - legislative_sessions
  - measures
  - committees
  - votes
update_frequency: active sessions update frequently; historical sessions stable
rate_limits_terms: Use acceptable-use agreement; cache by session and entity set.
robots_acceptable_use: Open Data is preferred for OLIS data; do not regularly scrape covered OLIS pages.
preferred_ingestion_method: OData API with metadata-driven mapping
fallback_ingestion_method: Static crawl only for documents not covered by API
graph_nodes_created:
  - LegislativeSession
  - Measure
  - MeasureDocument
  - VoteEvent
graph_edges_created:
  - IN_SESSION
  - HAS_DOCUMENT
  - CAST_VOTE
connector_status: planned
priority: P0
risks:
  - Legacy OData shape and composite keys.
source_url: https://api.oregonlegislature.gov/odata/ODataService.svc/
docs_url: https://www.oregonlegislature.gov/citizen_engagement/Pages/data.aspx
provenance:
  required_artifacts:
    - raw_artifact
    - source_metadata
    - content_hash
    - retrieval_timestamp
    - parser_diagnostics
    - jsonl_nodes
    - jsonl_edges
    - qc_report
  preserve_disclaimers: true
  official_source_precedence: true
```

## Required Fields

| Field | Type | Required | Allowed values / guidance |
| --- | --- | --- | --- |
| `source_id` | string | yes | Stable snake_case ID. Do not include year unless the source itself is edition-specific. |
| `name` | string | yes | Human-readable source name. |
| `owner` | string | yes | Publishing agency, nonprofit, vendor, court, or government body. |
| `jurisdiction` | string | yes | Existing or planned graph jurisdiction ID such as `or:state`, `us`, `or:linn`, or `global`. |
| `source_type` | enum | yes | `api`, `bulk`, `static_html`, `pdf`, `socrata`, `arcgis`, `search_page`. |
| `access` | enum | yes | `free`, `free_key_required`, `public_search`, `mixed`. |
| `official_status` | enum | yes | `official`, `nonprofit`, `secondary`, `unknown`. |
| `data_types` | list[string] | yes | Source-specific nouns, not graph labels. |
| `update_frequency` | string | yes | Natural-language cadence or `unknown`. |
| `rate_limits_terms` | string | yes | Known API key, throttle, license, attribution, or terms notes. Use `needs_review` if unknown. |
| `robots_acceptable_use` | string | yes | Robots/acceptable-use notes. Use `needs_review` if unknown. |
| `preferred_ingestion_method` | string | yes | Must follow API-first policy where an API exists. |
| `fallback_ingestion_method` | string | yes | Must not authorize broad scraping when terms are unknown. |
| `graph_nodes_created` | list[string] | yes | Planned or existing graph labels. Prefer existing ORSGraph labels. |
| `graph_edges_created` | list[string] | yes | Planned or existing relationship names. |
| `connector_status` | enum | yes | `not_started`, `planned`, `partial`, `implemented`, `blocked`, `deferred`. |
| `priority` | enum | yes | `P0`, `P1`, `P2`. |
| `risks` | list[string] | yes | At least one operational, legal, quality, or mapping risk. |
| `source_url` | string | yes | Primary source URL, or `varies` for source families. No tracking parameters. |
| `docs_url` | string | yes | API/docs/terms/reference URL, or `varies`. No tracking parameters. |

## Enum Definitions

### source_type

```yaml
source_type:
  - api
  - bulk
  - static_html
  - pdf
  - socrata
  - arcgis
  - search_page
```

Use the primary method as `source_type`. Record alternates in `fallback_ingestion_method`.

### access

```yaml
access:
  - free
  - free_key_required
  - public_search
  - mixed
```

- `free`: no key or account needed for normal use.
- `free_key_required`: free key, token, or registration needed.
- `public_search`: browser/search interface intended for targeted public lookup, not bulk ingestion.
- `mixed`: free metadata exists but documents, advanced use, or some datasets may require terms review, fees, or accounts.

### official_status

```yaml
official_status:
  - official
  - nonprofit
  - secondary
  - unknown
```

- `official`: published by the responsible government, court, legislature, agency, or official data portal.
- `nonprofit`: public-interest source such as Free Law Project or Open States.
- `secondary`: commercial, academic, or reference source that is not the official publisher.
- `unknown`: source needs review before ingestion.

### connector_status

```yaml
connector_status:
  - not_started
  - planned
  - partial
  - implemented
  - blocked
  - deferred
```

- `implemented`: connector/parser exists and is part of the current workflow.
- `partial`: some parser or source support exists, but coverage or automation is incomplete.
- `planned`: P0/P1 source intended for implementation.
- `deferred`: useful source intentionally left for later expansion.
- `blocked`: cannot proceed until terms, access, schema, or product scope is resolved.

## Identifier Conventions

Use stable, short, lowercase IDs:

```text
or_leg_odata
or_leg_ors_html
or_sos_oar
ojd_utcr
ojd_slr_registry
or_business_registry
govinfo_api_bulk
federal_register_api
courtlistener_api
recap_archive_api
oregon_geohub
```

Rules:

- Prefix Oregon state sources with `or_`.
- Prefix Oregon Legislature sources with `or_leg_`.
- Prefix Oregon Judicial Department sources with `ojd_`.
- Prefix federal sources with the source name or agency where clearer, such as `govinfo_`, `ecfr_`, `congress_gov_`, or `sec_`.
- Use source-family IDs for county/city sources until a specific connector exists, such as `county_assessor_property`.
- Do not encode implementation state, connector version, or priority in `source_id`.

## Provenance Requirements

Every connector output must include:

```yaml
raw_artifact:
  path: data/<source_id>/raw/...
  content_type: text/html
  raw_hash: sha256:...
  retrieved_at: 2026-05-01T00:00:00Z
source_metadata:
  source_id: or_leg_odata
  source_url: https://...
  docs_url: https://...
  owner: Oregon Legislature
  official_status: official
  disclaimer_required: true
  terms_status: reviewed
  robots_status: allowed
parser_diagnostics:
  path: data/<source_id>/graph/parser_diagnostics.jsonl
graph_outputs:
  nodes:
    - data/<source_id>/graph/source_documents.jsonl
  edges:
    - data/<source_id>/graph/source_edges.jsonl
qc_report:
  path: data/<source_id>/qc/report.json
```

Minimum provenance fields:

- `source_id`
- `source_url`
- `docs_url`
- `retrieved_at`
- `raw_hash`
- `normalized_hash` when normalization occurs
- `content_type`
- `official_status`
- `license_or_terms_note`
- `robots_or_acceptable_use_note`
- `parser_profile`
- `connector_version`
- `disclaimer_required`

## Connector Contract

Target Rust abstraction:

```rust
pub trait DataConnector {
    fn source_id(&self) -> &'static str;
    fn source_kind(&self) -> SourceKind;
    async fn discover(&self) -> Result<Vec<SourceItem>>;
    async fn fetch(&self, item: &SourceItem) -> Result<RawArtifact>;
    async fn parse(&self, artifact: &RawArtifact) -> Result<GraphBatch>;
    async fn qc(&self, batch: &GraphBatch) -> Result<QcReport>;
}
```

The method contract:

| Method | Responsibility |
| --- | --- |
| `discover` | Find candidate API records, pages, PDFs, packages, datasets, or search targets without mutating the graph. |
| `fetch` | Retrieve and hash immutable raw artifacts with retrieval metadata. |
| `parse` | Convert raw/normalized artifacts into source-backed JSONL nodes and edges. |
| `qc` | Verify provenance, graph ID stability, record counts, diagnostics, and source-specific invariants. |

## Validation Rules

- Registry rows must contain all required fields.
- `source_url` and `docs_url` must not include tracking parameters such as `utm_source`.
- API sources must use API or bulk ingestion as the preferred method unless the source API does not cover the needed artifact.
- Search-page sources must be marked `public_search` or `mixed` unless an explicit bulk/API permission exists.
- Official-source cautions must be preserved in `robots_acceptable_use`, `rate_limits_terms`, or `risks`.
- P0 sources must include specific graph node and edge mappings, not generic `unknown` placeholders.
- `official_status=secondary` sources must not be assigned higher canonical priority than official sources for the same legal text.
- Oregon Legislature OLIS data covered by OData must use `or_leg_odata` as the preferred source.
- OJD public records search must be treated as targeted public lookup, not bulk docket ingestion.
- GovInfo API sources must record that a free API key is required.
- Federal Register API sources must record API access separately from GovInfo Federal Register packages.
