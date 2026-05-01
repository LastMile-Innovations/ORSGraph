# 2025 UTCR Graph Ingestion

This document describes the 2025 Oregon Uniform Trial Court Rules ingestion path. UTCR is a first-class legal corpus in ORSGraph, not an ORS attachment and not a flat PDF artifact.

## Purpose

ORSGraph now has two different kinds of Oregon legal authority:

- ORS answers what substantive statutory law says.
- UTCR answers how Oregon trial-court work must be filed, formatted, served, captioned, redacted, presented, and checked.

The UTCR corpus is the procedural law brain for CaseBuilder WorkProduct Builder. It powers court-paper formatting, complaint/motion/answer QC, filing-packet assembly, certificate of service generation, eFiling readiness, exhibit checks, protected-information warnings, and sanctions severity.

## Corpus Identity

The parser creates these top-level graph records:

```json
{
  "corpus_id": "or:utcr",
  "name": "Oregon Uniform Trial Court Rules",
  "short_name": "UTCR",
  "authority_family": "UTCR",
  "authority_type": "court_rule",
  "jurisdiction_id": "or:state"
}
```

```json
{
  "edition_id": "or:utcr@2025",
  "corpus_id": "or:utcr",
  "edition_year": 2025,
  "effective_date": "2025-08-01",
  "source_label": "2025 Uniform Trial Court Rules"
}
```

```json
{
  "source_document_id": "or:utcr:source:2025_pdf",
  "title": "2025 Uniform Trial Court Rules",
  "source_type": "official_pdf",
  "file_name": "2025_UTCR.pdf",
  "page_count": 185,
  "effective_date": "2025-08-01"
}
```

Canonical identifiers follow this shape:

```text
LegalTextIdentity: or:utcr:2.010
LegalTextVersion:  or:utcr:2.010@2025-08-01
Provision:         or:utcr:2.010@2025-08-01:4:a
Display citation:  UTCR 2.010(4)(a)
```

## Parse Command

The parser reads the local official PDF and writes an isolated UTCR graph folder:

```bash
cargo run --release -p ors-crawler-v0 --bin ors-crawler-v0 -- parse-utcr-pdf \
  --input /Users/grey/Downloads/2025_UTCR.pdf \
  --out data/utcr_2025 \
  --edition-year 2025 \
  --effective-date 2025-08-01 \
  --source-url https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf
```

The output stays separate from the ORS output:

```text
data/utcr_2025/graph/
data/utcr_2025/stats.json
```

Do not mix generated UTCR JSONL into `data/graph/` until a combined ORS-plus-UTCR seed path is intentionally created.

## Current Parse Results

The latest successful parse produced:

```text
source pages:              185
chapters:                  24
rules:                     239
versions:                  239
provisions:                1,738
TOC entries:               263
citation mentions:         491
cites edges:               366
external legal citations:  53
procedural requirements:   2,565
retrieval chunks:          4,900
WorkProduct rule packs:    6
QC errors:                 0
QC warnings:               0
```

High-value rules are present as graph identities, including:

- `UTCR 2.010` document form, captions, spacing, margins, exhibits, title, and citation format.
- `UTCR 2.020` certificate of service.
- `UTCR 2.100` and `UTCR 2.110` protected personal information.
- Chapter 5 civil motion and proposed-order rules.
- Chapter 6 trial and exhibit rules.
- `UTCR 19.020` contempt initiating instruments and sanctions.
- `UTCR 21.040`, `21.090`, `21.100`, `21.110`, and `21.140` eFiling, signatures, service, hyperlinks, and mandatory eFiling.

## Output Files

The parser writes:

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
reporter_notes.jsonl
commentaries.jsonl
citation_mentions.jsonl
external_legal_citations.jsonl
cites_edges.jsonl
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
retrieval_chunks.jsonl
parser_diagnostics.jsonl
```

`parser_diagnostics.jsonl` should be empty for a clean parse. `stats.json` summarizes the parse and QC status.

## Parser Pipeline

The parser is implemented in `crates/ors-crawler-v0/src/utcr_pdf_parser.rs` and exposed from `crates/ors-crawler-v0/src/lib.rs`.

Pipeline:

1. Extract PDF bytes and page text with `lopdf`.
2. Normalize PDF control characters, line-break artifacts, smart punctuation, headers, footers, and page labels.
3. Store `SourcePage` rows with page number, normalized text, and text hash.
4. Detect body start and parse the front-matter TOC into `SourceTocEntry` rows.
5. Detect chapter starts, including split PDF lines.
6. Detect rule headings such as `2.010 FORM OF DOCUMENTS`, including split rule-number artifacts.
7. Assemble `LegalTextIdentity` and `LegalTextVersion` rows for each rule.
8. Parse provision hierarchy from `(1)`, `(a)`, `(i)`, and related markers.
9. Split reporter notes and commentary into dedicated rows.
10. Extract UTCR, ORS, ORCP, SLR, ORAP, order, and URL citations.
11. Resolve UTCR citations internally.
12. Resolve ORS citations when matching ORS graph identity data exists.
13. Emit ORCP, SLR, ORAP, orders, and form URLs as external legal citation placeholders.
14. Materialize procedural requirement nodes.
15. Build WorkProduct rule packs and formatting profile.
16. Create retrieval chunks optimized for rule checking and work-product generation.
17. Run structural, citation, rule-pack, and output QC.

## Graph Loading

Seed dry-run validates JSONL structure and row deserialization without requiring Neo4j credentials:

```bash
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- seed-neo4j \
  --graph-dir data/utcr_2025/graph \
  --neo4j-uri bolt://localhost:7687 \
  --neo4j-user neo4j \
  --neo4j-password-env NEO4J_PASSWORD \
  --dry-run
```

The latest dry-run validated `20,965` JSONL rows.

Live seed requires `NEO4J_PASSWORD` to be set:

```bash
export NEO4J_PASSWORD=...

cargo run --release -p ors-crawler-v0 --bin ors-crawler-v0 -- seed-neo4j \
  --graph-dir data/utcr_2025/graph \
  --neo4j-uri bolt://localhost:7687 \
  --neo4j-user neo4j \
  --neo4j-password-env NEO4J_PASSWORD
```

Embeddings are intentionally disabled for this phase. Do not add `--embed`, `--embed-chunks`, `--embed-provisions`, or `--embed-versions` until parse QC, seed dry-run, live Neo4j seed, and Neo4j QC all pass.

## Loader Behavior

The loader now accepts the shared legal graph spine for non-ORS authority families:

```text
LegalCorpus
-> CorpusEdition
-> SourceDocument
-> SourcePage
-> CourtRuleChapter
-> LegalTextIdentity
-> LegalTextVersion
-> Provision
-> RetrievalChunk
```

UTCR nodes receive court-rule labels at load time:

```text
LegalTextIdentity:UTCRRule:CourtRule
LegalTextVersion:UTCRRuleVersion:CourtRule
Provision:UTCRProvision
ProceduralRequirement plus specialized requirement labels
```

Important relationships:

```text
(:Jurisdiction)-[:HAS_CORPUS]->(:LegalCorpus)
(:LegalCorpus)-[:HAS_EDITION]->(:CorpusEdition)
(:CorpusEdition)-[:HAS_SOURCE_DOCUMENT]->(:SourceDocument)
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
(:Provision)-[:EXPRESSES]->(:ProceduralRequirement)
(:WorkProductRulePack)-[:INCLUDES_RULE]->(:ProceduralRequirement)
```

## Procedural Semantics

The parser emits procedural semantic rows as `ProceduralRequirement` records with specialized `semantic_type` values:

```text
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
```

Each requirement carries:

```text
requirement_id
semantic_type
requirement_type
label
text
normalized_text
source_provision_id
source_citation
applies_to
value
severity_default
authority_family
effective_date
confidence
extraction_method
```

Every WorkProduct-facing requirement should link back to a source provision. This allows a QC finding to cite both the procedural rule node and the actual UTCR provision.

## Rule Packs

Generated rule packs:

```text
or:utcr:2025:oregon_circuit_general_document
or:utcr:2025:oregon_circuit_civil_complaint
or:utcr:2025:oregon_circuit_civil_motion
or:utcr:2025:oregon_circuit_answer
or:utcr:2025:oregon_circuit_declaration
or:utcr:2025:oregon_circuit_filing_packet
```

Rule-pack membership rows link rule packs to procedural requirements and include default applicability and severity:

```text
rule_pack_id
requirement_id
requirement_type
source_provision_id
source_citation
applies_to
severity_default
```

The initial formatting profile is:

```text
or:utcr:2025:oregon_circuit_court_paper
```

## Retrieval Chunks

UTCR chunks are written for rule checking and WorkProduct generation. Chunk types include:

```text
full_rule
contextual_provision
formatting_requirement
filing_requirement
service_requirement
efiling_requirement
certificate_requirement
exhibit_requirement
protected_info_requirement
sanction_context
rule_pack_context
citation_context
```

Chunk headers include the corpus, edition, effective date, chapter, rule citation, source page, and requirement type when available.

## Search And API Behavior

Search understands explicit UTCR citations and ranges:

```text
UTCR 2.010
UTCR 2.010(4)(a)
UTCR 21.040 to 21.140
```

Search requests may filter by authority family:

```text
GET /api/v1/search?q=numbered%20lines&authority_family=UTCR
GET /api/v1/search?q=UTCR%202.010
GET /api/v1/search?q=mandatory%20electronic%20filing&authority_family=UTCR
```

Supported authority-family filter values include:

```text
ORS
UTCR
all
```

Bare section-like queries such as `90.300` still default to ORS. A bare UTCR rule number can be forced with `authority_family=UTCR`.

UTCR result hrefs use the court-rule route family:

```text
/rules/utcr/UTCR 2.010
/rules/utcr/UTCR 2.010(4)(a)?provision=...
```

Frontend support for these hrefs may still need route implementation if no `/rules/utcr` page exists yet.

## CaseBuilder Behavior

CaseBuilder authority search passes `authority_family` through to ORSGraph search. When the UTCR graph is seeded, UTCR citations should resolve to graph records instead of source-backed external placeholders.

Citation canonicalization now maps UTCR pin citations to the rule identity:

```text
UTCR 2.010(4) -> or:utcr:2.010
```

ORCP, ORAP, Supreme Court Orders, and court forms remain external placeholders until those corpora are added. The separate Court Rules Registry layer now models SLR/CJO/PJO publication currentness, and the local SLR PDF parser can ingest source-backed SLR text for counties as those editions are added.

## Embedding Strategy

Planned embedding targets after live seed and Neo4j QC:

```text
RetrievalChunk
Provision
LegalTextVersion
ProceduralRequirement
WorkProductRulePack summary chunks
```

Planned profiles:

```text
legal_rule_chunk_primary_v1
legal_rule_provision_primary_v1
legal_procedural_requirement_primary_v1
legal_rule_pack_primary_v1
```

Model configuration:

```text
voyage-4-large
1024 dimensions
float vectors
Neo4j vector index
```

Do not embed source pages, source documents, corpus editions, court-rule chapters, citation mentions, or parser diagnostics.

## Verification Commands

Run these after parser or loader changes:

```bash
cargo fmt --check -p ors-crawler-v0 -p orsgraph-api
cargo check -p ors-crawler-v0
cargo check -p orsgraph-api
cargo test -p ors-crawler-v0 utcr
cargo test -p orsgraph-api services::search::tests
cargo test -p orsgraph-api complaint_import_parser_preserves_labels_counts_and_citations
cargo test -p orsgraph-api citation_canonical_ids_cover_ors_orcp_and_utcr
```

Regenerate and dry-run seed:

```bash
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- parse-utcr-pdf \
  --input /Users/grey/Downloads/2025_UTCR.pdf \
  --out data/utcr_2025 \
  --edition-year 2025 \
  --effective-date 2025-08-01 \
  --source-url https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf

cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- seed-neo4j \
  --graph-dir data/utcr_2025/graph \
  --neo4j-uri bolt://localhost:7687 \
  --neo4j-user neo4j \
  --neo4j-password-env NEO4J_PASSWORD \
  --dry-run
```

## Known Limits

- PDF extraction can preserve some awkward source spacing from the official PDF. The parser normalizes the text enough for IDs, chunking, citations, and rule checks, but display copy may still need source-aware cleanup in UI surfaces.
- Procedural semantic extraction is deterministic and broad. High-value rules are explicitly materialized, but deeper domain semantics should be refined as WorkProduct checks become stricter.
- ORCP, ORAP, broader orders, and forms are placeholders in this phase. SLR/CJO/PJO currentness has moved into the Court Rules Registry layer, and local SLR PDFs can now be ingested as source-backed local rule corpora.
- Live Neo4j seed was not run in the latest verification because `NEO4J_PASSWORD` was not set in the shell.
- Embeddings were not run by design.
