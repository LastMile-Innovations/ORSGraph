# 09 - Indexing Harness Backlog

This backlog implements the case-file indexing harness described in [08-case-file-indexing-harness-spec.md](08-case-file-indexing-harness-spec.md).

## CB-IDX-001 - Indexing harness DTOs and graph constraints
- Priority: P0
- Area: Data model
- Problem: Large matter indexing needs stable records beyond the current document/fact payloads.
- Expected behavior: Add DTOs and constraints for `ObjectBlob`, `DocumentVersion`, `IngestionRun`, `IndexRun`, `Page`, `TextChunk`, `EvidenceSpan`, `EntityMention`, and `SearchIndexRecord`.
- Implementation notes: Keep IDs opaque and matter-scoped where relevant; key `ObjectBlob` by sha256 and index document/version/run status fields.
- Acceptance checks: Contract tests cover serialization, frontend normalization, Neo4j constraints, and no raw filename leakage in IDs/object keys.
- Dependencies: `CB-V0F-014`, `CB-V0F-015`, `CB-X-013`.
- Status: Partial
- Progress: First provenance DTO/constraint slice is implemented for `ObjectBlob`, `DocumentVersion`, `IngestionRun`, and `SourceSpan`; backend/frontend contract tests cover serialization, normalization references, graph constraints, and opaque object key behavior.
- Still needed: Add `IndexRun`, `Page`, `TextChunk`, `EvidenceSpan`, `EntityMention`, and `SearchIndexRecord`; expand constraints/indexes for large-matter indexing; and connect DTOs to parser registry, manifests, search indexes, and provenance UI.

## CB-IDX-002 - Parser registry and file classifier
- Priority: P0
- Area: Extraction
- Problem: Mixed matters need deterministic routing to the correct parser or unsupported/OCR state.
- Expected behavior: Add a parser registry that maps MIME, extension, and magic bytes to parser capability, artifact outputs, OCR requirement, and failure policy.
- Implementation notes: Classify before extraction; never trust extension alone.
- Acceptance checks: Fixtures classify TXT, PDF, DOCX, HTML, CSV, XLSX, ZIP, image, unknown binary, and malformed files correctly.
- Dependencies: `CB-IDX-001`, `CB-V0-021`.
- Status: Todo

## CB-IDX-003 - Inventory and fingerprint index
- Priority: P0
- Area: Intake
- Problem: Users may upload 100 to 1,000+ files and need immediate inventory, duplicate, and status feedback.
- Expected behavior: Intake computes hash, size, MIME, storage state, duplicate group, initial document type, and parser route before extraction.
- Implementation notes: Create inventory records even when extraction is delayed or unsupported.
- Acceptance checks: A 1,000-file fixture produces an inventory summary with duplicate groups, unsupported types, OCR-needed files, and no matter leakage.
- Dependencies: `CB-IDX-001`, `CB-IDX-002`.
- Status: Todo

## CB-IDX-004 - R2 artifact writer for normalized outputs
- Priority: P0
- Area: Artifact storage
- Problem: Normalized text, pages, OCR, and manifests should not live only in Neo4j.
- Expected behavior: Extraction writes `text.normalized.json`, `pages.json`, `manifest.json`, and optional `ocr.json`/thumbnail artifacts to R2 under opaque keys.
- Implementation notes: Store artifact object keys and hashes in `DocumentVersion`, `ObjectBlob`, and run records.
- Acceptance checks: Extracting a supported file creates R2 artifacts and graph references; deleting/tombstoning a matter disables artifact access.
- Dependencies: `CB-V0F-017`, `CB-X-017`.
- Status: Todo

## CB-IDX-005 - Idempotent manifest-to-graph upserter
- Priority: P0
- Area: Graph indexing
- Problem: Indexing must be rerunnable without duplicating pages, chunks, mentions, facts, or evidence spans.
- Expected behavior: Build an upsert pipeline from ingestion manifests into `Page`, `TextChunk`, `EntityMention`, `EvidenceSpan`, proposed facts, and relationships.
- Implementation notes: Use stable IDs derived from document version, page/unit, chunk ordinal, and span coordinates, not raw text.
- Acceptance checks: Replaying the same manifest produces the same node IDs and relationship counts.
- Dependencies: `CB-IDX-004`, `CB-V0-027`.
- Status: Todo

## CB-IDX-006 - Full-text index adapter
- Priority: P1
- Area: Search
- Problem: Users need fast keyword search across all case files without loading every document from Neo4j.
- Expected behavior: Add a matter-scoped full-text indexing adapter for titles, metadata, chunks, citations, parties, tags, and statuses.
- Implementation notes: Start with the existing database/search primitives if sufficient; keep adapter boundaries so a dedicated search engine can be added later.
- Acceptance checks: Keyword search returns chunk-level results with document/page/span provenance and respects matter boundaries.
- Dependencies: `CB-IDX-005`, `CB-X-016`.
- Status: Todo

## CB-IDX-007 - Vector and hybrid retrieval adapter
- Priority: P1
- Area: Semantic search
- Problem: Legal/case search needs semantic recall across chunked evidence, not whole-file embeddings.
- Expected behavior: Embed retrieval chunks and store index metadata: model, dimension, chunker version, source span, and stale/current state.
- Implementation notes: Reuse existing vector search infrastructure where practical; abstract provider/store selection behind an adapter.
- Acceptance checks: Semantic search returns provenance-backed chunks, can be filtered by matter/file/status, and marks stale embeddings after model/chunker changes.
- Dependencies: `CB-IDX-005`, `CB-X-004`, `CB-X-010`.
- Status: Todo

## CB-IDX-008 - OCR-needed and OCR result workflow
- Priority: P1
- Area: OCR
- Problem: Scanned PDFs and images are common but must not be silently treated as indexed.
- Expected behavior: OCR-needed files show actionable state; configured OCR writes `ocr.json`, text artifacts, pages, chunks, and provenance links.
- Implementation notes: Keep OCR provider-gated and label disabled/live mode in UI and API responses.
- Acceptance checks: Image-only files become `ocr_required` when OCR is disabled and `review_ready` with source spans when OCR is enabled.
- Dependencies: `CB-IDX-004`, `CB-X-004`.
- Status: Todo

## CB-IDX-009 - Archive and folder ingestion
- Priority: P1
- Area: Intake
- Problem: Users will upload folders or ZIP archives containing many mixed files.
- Expected behavior: Archive upload creates a parent archive document/version plus child document records for supported files, preserving folder paths as private metadata.
- Implementation notes: Child object keys remain opaque; original folder paths must not leak into R2 keys.
- Acceptance checks: ZIP/folder fixture indexes children individually, reports unsupported children, and preserves archive provenance.
- Dependencies: `CB-IDX-003`, `CB-IDX-004`.
- Status: Todo

## CB-IDX-010 - Email and message export ingestion
- Priority: P1
- Area: Extraction
- Problem: Case files often include EML, MBOX, text message exports, and attachments.
- Expected behavior: Parse message containers into message units, participants, timestamps, body chunks, attachments, and entity mentions.
- Implementation notes: Treat each message as a page-like structural unit with source coordinates and attachment provenance.
- Acceptance checks: Email/message fixtures produce searchable bodies, parties/mentions, attachment links, and timeline candidates.
- Dependencies: `CB-IDX-002`, `CB-IDX-005`.
- Status: Todo

## CB-IDX-011 - Spreadsheet/table indexing
- Priority: P1
- Area: Extraction
- Problem: Spreadsheets need cell/range provenance, not only flattened text.
- Expected behavior: XLSX/CSV extraction records sheets, rows, columns, cell ranges, detected dates/amounts/entities, and chunked table summaries.
- Implementation notes: Evidence spans from spreadsheets should point to sheet/range coordinates.
- Acceptance checks: Spreadsheet fixture supports search by value, date, amount, and source range.
- Dependencies: `CB-IDX-002`, `CB-IDX-005`.
- Status: Todo

## CB-IDX-012 - Matter index console UI
- Priority: P1
- Area: Frontend
- Problem: Users need to understand indexing progress and failures across hundreds of files.
- Expected behavior: Add matter-level index console showing totals, active jobs, queue depth, failures, OCR-needed files, duplicate groups, stale indexes, and retry actions.
- Implementation notes: Keep the console operational and dense, not marketing-style.
- Acceptance checks: A large fixture clearly shows what is indexed, failed, pending, stale, and review-ready.
- Dependencies: `CB-IDX-003`, `CB-IDX-005`, `CB-X-010`.
- Status: Todo

## CB-IDX-013 - Document provenance trail UI
- Priority: P1
- Area: Frontend
- Problem: Users need trustable source trails from facts/search results back to exact files and spans.
- Expected behavior: Document detail shows original blob, versions, artifacts, ingestion/index runs, pages/chunks, produced facts/mentions, and retry/error state.
- Implementation notes: Use the same provenance component in facts, evidence, draft support, QC, and search results.
- Acceptance checks: Opening any search result or fact support shows exact object/version/page/chunk/span provenance.
- Dependencies: `CB-V0-020`, `CB-IDX-005`.
- Status: Todo

## CB-IDX-014 - Reindex scheduler and stale-index detection
- Priority: P1
- Area: Operations
- Problem: Parser, OCR, chunker, embedding, and graph schema changes require targeted reindexing.
- Expected behavior: Track index versions and mark affected records stale; schedule reindex by matter, document, version, parser, or embedding model.
- Implementation notes: Preserve old provenance and run records; do not destructively rewrite history.
- Acceptance checks: Changing a parser/chunker/model version marks only affected records stale and reindexes them idempotently.
- Dependencies: `CB-IDX-005`, `CB-IDX-006`, `CB-IDX-007`.
- Status: Todo

## CB-IDX-015 - Indexing harness large-fixture benchmark
- Priority: P0
- Area: Quality/performance
- Problem: The harness must prove it can handle 100 to 1,000+ files before users rely on it.
- Expected behavior: Add a synthetic and/or sanitized large-matter fixture with mixed files, duplicates, unsupported files, OCR-needed files, and high chunk counts.
- Implementation notes: Track throughput, failure rates, memory, queue behavior, graph upsert counts, and search response time.
- Acceptance checks: Benchmark produces a report and fails if indexing is unbounded, non-idempotent, or cross-matter unsafe.
- Dependencies: `CB-X-018`, `CB-IDX-001` through `CB-IDX-007`.
- Status: Todo

## CB-IDX-016 - Quarantine and unsafe file policy
- Priority: P0
- Area: Security
- Problem: Mixed uploads may include malformed, encrypted, executable, or unsafe files.
- Expected behavior: Quarantine unsafe or unparseable files, store them as artifacts, block parsing/preview, and show review actions.
- Implementation notes: Do not delete automatically unless retention policy says so; record reason and scanner/parser result.
- Acceptance checks: Unsafe fixture files do not enter extraction/search indexes and cannot be previewed as trusted evidence.
- Dependencies: `CB-IDX-002`, `CB-X-011`.
- Status: Todo
