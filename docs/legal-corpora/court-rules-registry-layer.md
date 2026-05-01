# Court Rules Registry Layer

The Court Rules Registry layer ingests court-published rule and order indexes. It is a currentness overlay, not a replacement for source PDFs such as UTCR or local SLR editions.

For Linn County, the provenance source is:

```text
https://www.courts.oregon.gov/courts/linn/go/pages/rules.aspx
```

The captured table tells ORSGraph:

```text
what rule/order exists
which jurisdiction it applies to
which table bucket published it
what type of authority it is
when it starts
when it ends
whether it is current, future, prior, expired, open-ended, one-day-only, out-of-cycle, or superseded
```

## Parser Command

```bash
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- parse-court-rules-registry \
  --input data/registry/linn_rules_2026_snapshot.txt \
  --out data/linn_rules_registry_2026 \
  --jurisdiction Linn \
  --snapshot-date 2026-05-01 \
  --source-url https://www.courts.oregon.gov/courts/linn/go/pages/rules.aspx
```

The parser writes graph files under:

```text
data/linn_rules_registry_2026/graph/
```

## Parsed Sections

The parser detects:

```text
Current and Future Rules
Prior Rules
```

Expected columns:

```text
Description
Jurisdiction
Subcategory
Effective Start Date
Effective End Date
```

Malformed rows become parser diagnostics. A malformed row should not drop the whole snapshot.

## Normalization

Jurisdictions:

```text
Statewide -> or:state
Linn      -> or:linn
```

Authority kinds:

```text
CJO          -> ChiefJusticeOrder
PJO          -> PresidingJudgeOrder
Rule         -> SupplementaryLocalRuleEdition
Out-of-Cycle -> OutOfCycleAmendment
```

Current Oregon defaults:

```text
state_id:            or:state
state_name:          Oregon
base_rule_corpus_id: or:utcr
local court id:      <jurisdiction_id>:circuit_court
```

Document identifiers are extracted when present:

```text
CJO 25-018
PJO 25-005
PJO 25001
SLR 6.101
Appendix B
```

Topic tags are derived from titles, such as:

```text
Emergency Closure
Court Operations
COVID-19
Remote Proceedings
Pretrial Release
Fees
Security Screening
Landlord Tenant Sealing
Immigration Enforcement
OJCIN Fees
```

## Output Files

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

The parser also writes empty shared graph files where needed so the standard seed path can run without a separate special-case directory.

## Graph Contract

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
```

Rule hierarchy and currentness:

```text
(:SupplementaryLocalRuleEdition)-[:SUPPLEMENTS]->(:LegalCorpus)
(:OutOfCycleAmendment)-[:AMENDS]->(:SupplementaryLocalRuleEdition)
(:RuleAuthorityDocument)-[:SUPERSEDES]->(:RuleAuthorityDocument)
(:WorkProductRulePack)-[:INCLUDES_AUTHORITY]->(:RuleAuthorityDocument)
```

The `SUPPLEMENTS` edge is data-driven through `supplements_corpus_id`. Oregon SLR editions currently supplement `or:utcr`. Other states can point to their own base trial-court rule corpus.

## Currentness Logic

The source table bucket is stored as `publication_bucket`:

```text
current_future
prior
```

Applicability uses date logic:

```text
effective_start_date <= filing_date
and
(effective_end_date is null or effective_end_date >= filing_date)
```

Computed flags:

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

For the May 1, 2026 snapshot, the Linn SLR edition effective February 1, 2026 through January 31, 2027 is active.

## API And Resolver

The API route module is `crates/orsgraph-api/src/routes/rules.rs`.

Routes:

```text
GET /api/v1/rules/registry
GET /api/v1/rules/jurisdictions/:jurisdictionId/current
GET /api/v1/rules/jurisdictions/:jurisdictionId/history
GET /api/v1/rules/applicable?jurisdiction=Linn&date=2026-02-15&type=complaint
GET /api/v1/rules/orders/:authorityDocumentId
GET /api/v1/rules/slr/:jurisdictionId/:year
```

`RuleApplicabilityResolver` expands jurisdiction scope through `PART_OF` edges. For local Oregon courts, it can include:

```text
or:linn
or:state
us
```

Applicability results are grouped as:

```text
utcr
slr_edition
statewide_orders
local_orders
out_of_cycle_amendments
currentness_warnings
```

Expired and future authorities stay available through history endpoints but are excluded from applicability results.

## Verification

```bash
cargo fmt --check
cargo check -p ors-crawler-v0
cargo check -p orsgraph-api
cargo test -p ors-crawler-v0 court_rules_registry
```

Run a seed dry-run after parser or loader changes:

```bash
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- seed-neo4j \
  --graph-dir data/linn_rules_registry_2026/graph \
  --neo4j-uri bolt://localhost:7687 \
  --neo4j-user neo4j \
  --neo4j-password-env NEO4J_PASSWORD \
  --dry-run
```

