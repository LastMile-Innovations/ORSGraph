# Local SLR PDF Ingestion

Local Supplementary Local Rule PDFs are source-backed rule corpora. They are different from the registry layer:

- The registry says which SLR edition is active.
- The SLR PDF parser extracts the actual local rule text, provisions, citations, source pages, and retrieval chunks.

The first implemented local SLR corpus is Linn County 2026.

## Parser Command

```bash
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- parse-local-rule-pdf \
  --input /Users/grey/Downloads/Linn_SLR_2026.pdf \
  --out data/linn_slr_2026 \
  --jurisdiction-id or:linn \
  --jurisdiction-name "Linn County" \
  --court-id or:linn:circuit_court \
  --court-name "Linn County Circuit Court" \
  --judicial-district "23rd Judicial District" \
  --edition-year 2026 \
  --effective-date 2026-02-01 \
  --source-url https://www.courts.oregon.gov/courts/linn/go/pages/rules.aspx
```

Output path:

```text
data/linn_slr_2026/graph/
```

## Corpus Identity

For Linn:

```json
{
  "corpus_id": "or:linn:slr",
  "name": "Linn County Circuit Court Supplementary Local Court Rules",
  "short_name": "Linn SLR",
  "authority_family": "SLR",
  "authority_type": "court_rule",
  "jurisdiction_id": "or:linn"
}
```

```json
{
  "edition_id": "or:linn:slr@2026",
  "corpus_id": "or:linn:slr",
  "edition_year": 2026,
  "effective_date": "2026-02-01"
}
```

Identifier shape:

```text
LegalCorpus:       or:linn:slr
CorpusEdition:     or:linn:slr@2026
LegalTextIdentity: or:linn:slr:1.151
LegalTextVersion:  or:linn:slr:1.151@2026
Provision:         or:linn:slr:1.151@2026:a
Display citation:  Linn SLR 1.151(a)
```

## Output Files

```text
jurisdictions.jsonl
courts.jsonl
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
retrieval_chunks.jsonl
parser_diagnostics.jsonl
cites_edges.jsonl
```

## Parser Behavior

The parser:

1. Extracts PDF page text with `lopdf`.
2. Normalizes PDF control characters, smart punctuation, headers, footers, and page artifacts.
3. Repairs split rule numbers such as `1` plus `.171`.
4. Parses the table of contents to avoid false body headings.
5. Detects chapter headings and appendix headings.
6. Detects rule headings such as `13.095 ARBITRATION PANEL`.
7. Creates `LegalTextIdentity`, `LegalTextVersion`, and `Provision` rows.
8. Creates `SourcePage`, `SourceTocEntry`, `CourtRuleChapter`, and `ChapterHeading` rows.
9. Extracts local SLR, UTCR, ORS, and ORCP citations.
10. Builds retrieval chunks for each rule or appendix unit.
11. Emits parser diagnostics for missing or suspiciously short parses.

Header cleanup is intentionally conservative. For example, numeric page headers such as `23rd Judicial District` can be removed, but body text such as `Twenty Third Judicial District` must be preserved.

## Current Linn 2026 Parse Result

The latest parser review of `/Users/grey/Downloads/Linn_SLR_2026.pdf` produced:

```text
jurisdictions:             4
courts:                    1
source pages:              27
chapters/appendix groups:  12
rules/authority units:     31
versions:                  31
provisions:                124
citation mentions:         22
external legal citations:  11
retrieval chunks:          31
parser diagnostics:        0
```

The citation extractor protects overlapping authority citations. `UTCR 13.090` remains a UTCR citation and does not create a fake `Linn SLR 13.090` local-rule candidate.

## Relationship To Registry

The SLR PDF parser creates local rule text:

```text
or:linn:slr:13.095
```

The registry parser creates the edition/currentness authority:

```text
or:linn:slr@2026
```

Both are needed:

- the registry resolver chooses the active edition for a filing date;
- the local SLR corpus provides the actual rule text and citations.

When both are seeded, CaseBuilder can show:

```text
Rule profile:
Oregon Circuit Court
Linn County
UTCR 2025
Linn SLR 2026
active statewide CJOs
active local PJOs
```

## Verification

```bash
cargo fmt --check
cargo check -p ors-crawler-v0
cargo test -p ors-crawler-v0 local_rule_pdf_parser
```

Regenerate the parse:

```bash
cargo run -p ors-crawler-v0 --bin ors-crawler-v0 -- parse-local-rule-pdf \
  --input /Users/grey/Downloads/Linn_SLR_2026.pdf \
  --out data/linn_slr_2026
```

