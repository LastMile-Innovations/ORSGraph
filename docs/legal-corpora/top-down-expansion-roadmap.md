# Top-Down Legal Authority Expansion Roadmap

ORSGraph should expand top down:

```text
federal
-> state
-> county/local
-> court
-> document type
-> matter-specific filing date
```

The goal is not just to ingest more text. The goal is to answer, with source-backed currentness:

```text
What law applies here, in this court, for this filing, on this date?
```

## Oregon First

Oregon is the proving ground for the full graph shape.

Current Oregon pieces:

- ORS corpus and provisions.
- 2025 UTCR corpus and procedural requirement/rule-pack extraction.
- Linn SLR/CJO/PJO registry snapshot.
- Linn 2026 SLR PDF corpus.
- Rule applicability API and CaseBuilder rule profile integration.
- Oregon Legislature OData ingestion contract for sessions, measures, documents, sponsors, votes, committees, legislators, and session-law enrichment.

Oregon next sources:

```text
ORCP
ORAP
OAR
Oregon Legislature OData implementation
Oregon session laws
Oregon Tax Court rules
Oregon appellate rules/orders
all county SLR registries
all county SLR PDF editions
court forms
municipal and county codes
agency guidance where authoritative enough to cite
```

Oregon court overlay order:

```text
or:state
-> judicial district
-> county
-> circuit court
-> court location
-> work product type
```

## All Oregon County SLRs

Each Oregon county/court needs two ingestion tracks:

1. Registry/currentness index:

```text
CourtRulesRegistrySource
CourtRulesRegistrySnapshot
RulePublicationEntry
RuleAuthorityDocument
EffectiveInterval
RuleTopic
```

2. Local rule PDF corpus:

```text
LegalCorpus
CorpusEdition
SourceDocument
SourcePage
LegalTextIdentity
LegalTextVersion
Provision
CitationMention
RetrievalChunk
```

The registry chooses the active SLR edition. The PDF corpus supplies the rule text.

County ID pattern:

```text
or:benton
or:clackamas
or:deschutes
or:jackson
or:linn
or:multnomah
```

Court ID pattern:

```text
or:linn:circuit_court
or:multnomah:circuit_court
```

SLR corpus pattern:

```text
or:linn:slr
or:linn:slr@2026
or:linn:slr:13.095
```

## Other States

Every state should map into the same model but with state-specific court structure.

State ID pattern:

```text
ca:state
wa:state
ny:state
tx:state
```

Base court-rule corpus pattern:

```text
ca:trial_court_rules
wa:superior_court_rules
ny:civil_practice_rules
tx:rules_of_civil_procedure
```

Local rule corpus pattern:

```text
ca:los_angeles:local_rules
wa:king:local_rules
ny:new_york:local_rules
```

Each state adapter should define:

- jurisdiction hierarchy;
- court type names;
- base rule corpus IDs;
- local rule publication sources;
- official source/disclaimer rules;
- citation parser rules;
- effective-date/currentness rules;
- WorkProduct rule packs.

Do not hard-code Oregon assumptions into matter or WorkProduct logic. Put jurisdiction-specific behavior in parsers, resolver configuration, corpus IDs, rule packs, and formatting profiles.

## Federal Law And Federal Courts

Federal law should use `us` as the top jurisdiction.

Federal corpus IDs:

```text
us:usc
us:cfr
us:frcp
us:frcrp
us:fre
us:frap
us:bankruptcy_rules
```

Federal court IDs:

```text
us:district_court:or
us:district_court:or:portland_division
us:bankruptcy_court:or
us:court_of_appeals:9th
us:supreme_court
```

Federal local rules:

```text
us:district_court:or:local_rules
us:bankruptcy_court:or:local_rules
us:court_of_appeals:9th:local_rules
```

Federal applicability needs the same resolver shape:

```text
us
-> federal court
-> district/division local rules
-> work product type
-> filing date
```

## Required Expansion Checklist

For each new jurisdiction/source:

1. Create jurisdiction and court IDs.
2. Identify official sources and disclaimers.
3. Build parser or scraper.
4. Emit JSONL using the shared graph spine.
5. Add registry/currentness output if the source has versions or orders.
6. Add loader queries only when the shared loader cannot already handle the file.
7. Add citation parser rules.
8. Add rule-pack memberships for CaseBuilder document types.
9. Add resolver fixture tests.
10. Add docs with command, output files, graph contract, limits, and verification.

## Release Gate For Filing Use

Before any jurisdiction can power filing/export compliance:

- source provenance must be stored;
- currentness must be date-resolved;
- expired/future rules must be excluded from active checks;
- the active rule profile must be visible to the user;
- rule findings must cite source authority;
- exports must show unresolved/currentness warnings;
- no automated filing should run without explicit court-specific safety checks.
