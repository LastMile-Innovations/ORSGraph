# 08 - Case File Indexing Harness Spec

This spec defines the production indexing harness for CaseBuilder matters with hundreds to thousands of mixed files. The harness turns private case materials into searchable, provenance-backed graph intelligence without putting heavy artifacts in Neo4j.

## Goal

Build a repeatable indexing system that can answer:

- What files exist in this matter?
- Which files are duplicates or near-duplicates?
- What text, pages, entities, dates, citations, facts, and evidence spans came from each file?
- What claims, elements, deadlines, and draft paragraphs are supported by each source?
- What must be re-indexed when a parser, OCR engine, model, or graph schema changes?

## Core Architecture

### Storage Split

- R2 is the evidence and artifact lake.
  - Original uploads.
  - Normalized text JSON.
  - Page/layout JSON.
  - OCR output.
  - Thumbnails/previews.
  - Redacted copies.
  - Extraction and indexing manifests.
  - Export bundles.
- Neo4j is the meaning graph.
  - Matter, CaseDocument, DocumentVersion, ObjectBlob.
  - Page, TextChunk, EvidenceSpan.
  - Entity, Party, EntityMention.
  - Fact, TimelineEvent, DeadlineInstance.
  - Claim, Element, Defense.
  - DraftParagraph, DraftSentence.
  - Authority refs and validation findings.
  - IngestionRun and IndexRun.

### Indexing Layers

1. Inventory index: file metadata, hashes, MIME sniffing, duplicate detection, storage status.
2. Extraction index: extracted text, page layout, OCR, sheet/email/archive structure.
3. Retrieval index: chunks, full-text tokens, embeddings, citation/entity/date facets.
4. Graph index: facts, evidence spans, mentions, timeline events, claims/elements/deadlines.
5. Review index: proposed items, confidence, unsupported/contradicted state, human approval.

Each layer must be independently retryable and tied to a versioned manifest.

## Data Model

### Required Nodes / DTOs

- `ObjectBlob`: content-addressed object identity keyed by `sha256`.
- `CaseDocument`: user-facing document record within a matter.
- `DocumentVersion`: original, normalized, OCR, redacted, exhibit, or generated representation.
- `IngestionRun`: upload/extraction pipeline execution.
- `IndexRun`: indexing execution for a document version or whole matter.
- `Page`: page/sheet/message/frame unit.
- `TextChunk`: retrieval-ready text region.
- `EvidenceSpan`: exact quote/span used as support.
- `EntityMention`: extracted person/org/date/money/address/citation mention.
- `SearchIndexRecord`: durable record that a chunk/version was indexed into full-text/vector stores.

### Provenance Rule

Every derived node must be able to trace back to:

```text
Matter
→ CaseDocument
→ DocumentVersion
→ ObjectBlob
→ Page or structural unit
→ TextChunk
→ byte/char span or cell/message coordinate
→ IngestionRun / IndexRun
```

If a source coordinate is unavailable, the node must say why: unsupported file type, OCR unavailable, parser failed, redacted source, or user-entered fact.

## File Type Strategy

### V0 Required

- TXT, Markdown, HTML.
- PDF with embedded text.
- DOCX.
- CSV and XLSX.
- Images as stored/previewable artifacts with OCR-deferred state.
- ZIP or folder upload as an archive manifest with child files queued individually.

### V0.1 Required

- Scanned PDFs and image OCR.
- EML/MBOX email exports.
- JSON/text message exports.
- Common image formats with thumbnails.
- Receipts/invoices through OCR plus metadata extraction.

### V0.2+

- Audio/video transcription.
- PST/MSG where supported by a safe parser.
- Redacted derivative generation.
- Court filing package import.
- Near-duplicate and version-family detection.

## Pipeline

### Stages

1. Intake
   - Receive file or object notification.
   - Create `ObjectBlob`, `CaseDocument`, `DocumentVersion`, and `IngestionRun`.
   - Store opaque R2 key and preserve original filename only as private metadata.
2. Fingerprint
   - Compute sha256, size, MIME type, extension, magic bytes, page estimates.
   - Detect exact duplicates by `ObjectBlob`.
3. Classify
   - Identify document type, parser, sensitivity hints, likely OCR need, archive children.
4. Extract
   - Write normalized artifacts to R2.
   - Produce page/layout/text manifests.
5. Chunk
   - Create retrieval-ready chunks with stable IDs and source coordinates.
   - Use page/section/table/email boundaries before token-size splitting.
6. Enrich
   - Extract dates, parties, entities, citations, money amounts, deadlines, candidate facts.
   - Provider-gate AI enrichments and label deterministic/template/live mode.
7. Index
   - Upsert Neo4j pages/chunks/spans/mentions/facts.
   - Update full-text and vector indexes.
   - Record `SearchIndexRecord` rows and index versions.
8. Review
   - Surface proposed facts/entities/deadlines/issues for approval.
   - Keep rejected items traceable but inactive.

### State Machine

```text
queued
→ stored
→ fingerprinted
→ classified
→ extracting
→ extracted
→ chunked
→ enriching
→ indexed
→ review_ready
→ approved / needs_attention
```

Failure states:

```text
unsupported
ocr_required
parse_failed
index_failed
partial_success
quarantined
```

Failures must include a retryable/non-retryable flag, error code, failed stage, and next action.

## Indexing Strategy

### Full-Text

- Index document title, filename metadata, extracted text chunks, citations, parties, and tags.
- Use matter-scoped queries by default.
- Store index version and chunk IDs so stale records can be reindexed.

### Vector / Semantic

- Embed retrieval chunks, not whole documents.
- Store embedding model, dimension, chunking strategy, and source span.
- Reindex only chunks affected by parser/chunker/model changes.
- Keep vector search abstracted behind a search adapter so Neo4j vector, external vector stores, or hybrid search can be swapped.

### Graph

- Neo4j stores relationships users need to reason:

```cypher
(:Matter)-[:HAS_DOCUMENT]->(:CaseDocument)
(:CaseDocument)-[:HAS_VERSION]->(:DocumentVersion)
(:DocumentVersion)-[:STORED_AS]->(:ObjectBlob)
(:DocumentVersion)-[:HAS_PAGE]->(:Page)
(:Page)-[:HAS_CHUNK]->(:TextChunk)
(:TextChunk)-[:MENTIONS]->(:EntityMention)
(:EntityMention)-[:RESOLVES_TO]->(:Party)
(:Fact)-[:SUPPORTED_BY]->(:EvidenceSpan)
(:EvidenceSpan)-[:QUOTES]->(:TextChunk)
(:Claim)-[:HAS_ELEMENT]->(:Element)
(:Element)-[:SATISFIED_BY]->(:Fact)
(:IngestionRun)-[:PRODUCED]->(:Fact)
(:IngestionRun)-[:DERIVED_FROM]->(:ObjectBlob)
(:IndexRun)-[:INDEXED]->(:TextChunk)
```

### Reindexing

Reindex when:

- Parser version changes.
- OCR engine/model changes.
- Chunking strategy changes.
- Embedding model changes.
- Entity/fact/citation extractor changes.
- Graph schema changes.
- User requests reprocess.

Each reindex run should reuse original R2 objects, write new manifests/artifacts when needed, and mark previous index records stale rather than deleting provenance.

## Scaling Requirements

### Matter Scale

Design target per matter:

- 100 to 1,000+ files.
- 10,000+ pages.
- 100,000+ chunks for very large matters.
- 10,000+ extracted facts/mentions before review.

### Controls

- Queue-based ingestion with bounded concurrency per matter.
- Backpressure so one large matter cannot starve all work.
- Per-stage retry limits.
- Idempotent graph upserts.
- Pagination and lazy loading for document library, facts, chunks, and findings.
- Matter-level index progress summaries.

## UI Requirements

### Matter Index Console

Show:

- Total files, indexed files, failed files, OCR-needed files.
- Current queue depth and active jobs.
- Stage progress by file type.
- Duplicate groups.
- Parser/OCR failures.
- Reindex-needed warnings.
- Storage and index version summaries.

### Document Provenance Trail

For each document:

- Original object hash and storage state.
- Versions/artifacts.
- Ingestion runs and index runs.
- Extracted pages/chunks.
- Proposed facts/entities/deadlines/issues.
- Errors and retry actions.

### Search Experience

Search must support:

- Keyword search.
- Semantic search.
- Filters by file type, date, party, tag, claim, source, status, OCR-needed, reviewed/unreviewed.
- Result explanation: why this matched, which chunk/span, which file/page, and whether it is approved evidence.

## Safety / Privacy

- New object keys must not include raw filenames or case facts.
- Logs must never include raw document text.
- Indexing failures must not mark unsupported facts as supported.
- Quarantined files stay stored but are not parsed or previewed until reviewed.
- Redacted versions must never overwrite originals.
- Deleted/tombstoned matters must remove or disable access to artifacts and index records according to retention policy.

## Acceptance Criteria

- Uploading a mixed 1,000-file fixture creates stable document/blob/version/run records.
- Duplicate files share `ObjectBlob` where policy allows.
- Every indexed chunk can trace to a source object, page/unit, and span.
- Failed/unsupported files show actionable status without blocking the whole matter.
- Reindexing a parser or embedding version updates only affected records.
- Matter search remains matter-scoped and returns provenance-backed results.
- The UI can answer what is indexed, what failed, what needs review, and what changed since the last index run.
