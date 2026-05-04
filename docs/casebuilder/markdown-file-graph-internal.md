# Markdown File Graph Internal Reference

This reference documents the implemented CaseBuilder Markdown File Graph pipeline. It is the engineering contract for the review-first graph of and from user Markdown files.

## Scope

The active indexing mode is Markdown-only.

- Markdown files are editable, extractable, promotable into `WorkProduct.document_ast`, and indexable.
- Non-Markdown files are stored and available as `view_only` sources. They remain openable/downloadable and annotatable where supported.
- DOCX, PDF, OCR, spreadsheet, email, archive, and media parsers remain future adapter work unless a dedicated route explicitly handles them.

The feature adds graph structure for Markdown documents without renaming public API routes.

## Source Files

Primary backend files:

- `crates/orsgraph-api/src/services/casebuilder/markdown_graph.rs`
- `crates/orsgraph-api/src/services/casebuilder/documents.rs`
- `crates/orsgraph-api/src/services/casebuilder/embeddings.rs`
- `crates/orsgraph-api/src/services/casebuilder/repository.rs`
- `crates/orsgraph-api/src/services/casebuilder/mod.rs`
- `crates/orsgraph-api/src/models/casebuilder.rs`
- `crates/orsgraph-api/src/services/casebuilder/indexes.rs`

Primary frontend files:

- `frontend/lib/casebuilder/types.ts`
- `frontend/lib/casebuilder/api.ts`
- `frontend/components/casebuilder/document-workspace.tsx`
- `frontend/components/casebuilder/matter-graph-view.tsx`

Contract tests:

- `crates/orsgraph-api/tests/graph_contract.rs`
- `frontend/components/casebuilder/document-workspace.test.tsx`
- `frontend/components/casebuilder/matter-graph-view.test.tsx`
- `frontend/components/casebuilder/timeline-view.test.tsx`

## Pipeline

Markdown indexing follows this order:

1. Confirm the document is Markdown-indexable.
2. Read source bytes as text.
3. Normalize index text with `markdown_index_text`.
   - Strip CaseBuilder sidecar comments.
   - Strip generated review notices.
   - Preserve useful Markdown structure for parsing and chunking.
4. Create document-version-aware chunks and source spans.
5. Build page, text chunk, evidence span, entity mention, and search index records.
6. Propose deterministic facts and deterministic-first timeline suggestions.
7. Parse the sanitized Markdown with `pulldown-cmark`.
8. Build `MarkdownAstDocument` and `MarkdownAstNode` records.
9. Attach overlapping AST node IDs to chunks, evidence spans, entity mentions, facts, and timeline suggestions.
10. Group entity mentions into canonical reviewable `CaseEntity` candidates.
11. Write normalized text/pages/manifest artifacts to the object store.
12. Upsert graph nodes and relationships into Neo4j.
13. Queue Markdown embeddings when `ORS_CASEBUILDER_EMBEDDINGS_ENABLED=true` and a Voyage key is configured.

The parsing and graph construction entrypoint is `build_markdown_ast_graph`.

## Voyage Embedding Pipeline

CaseBuilder Markdown embeddings are an asynchronous add-on to the deterministic Markdown graph. Extraction and review workflows must succeed or fail independently of the embedding provider.

Configuration:

- `ORS_CASEBUILDER_EMBEDDINGS_ENABLED=true`
- `VOYAGE_API_KEY`
- `ORS_EMBEDDING_MODEL`, default `voyage-4-large`
- `ORS_VECTOR_DIMENSION`, default `1024`

Provider profile:

- Voyage `/v1/embeddings`
- `input_type=document`
- `output_dimension=1024`
- `output_dtype=float`
- `truncation=false`
- batch size capped at 64 inputs

The embedding service creates three retrieval levels:

- Full Markdown file records with `target_kind=markdown_file`.
- Chunk records with `target_kind=text_chunk`.
- Semantic unit records with `target_kind=markdown_semantic_unit`.

Full-file source text is embedded directly only under the safe token threshold. Oversized files do not use provider truncation; they get a normalized centroid vector from chunk and semantic-unit vectors and mark `embedding_strategy=centroid_from_chunks`.

The embedding input text is built from sanitized Markdown index text. It excludes CaseBuilder sidecar comments and generated review notices, and it includes compact context such as document title, relative path, heading/structure path, semantic role, citations, dates, money hints, and exact source-span text where available.

Stale detection is version-aware. Records are current only when all of these match:

- `document_version_id`
- model
- dimension
- profile
- `chunker_version`
- Markdown graph schema version
- source/input hash

Saving a Markdown edit creates a new document version. Old records remain for lineage and become stale; search defaults to current records only.

## Markdown AST Construction

The AST parser uses:

```text
pulldown-cmark 0.13
Parser::new_ext(text, Options::all()).into_offset_iter()
```

Each parser start event becomes a structural node. Leaf events become inline/text nodes. End events close the stack.

Node kinds include:

- `document`
- `heading`
- `paragraph`
- `quote`
- `list`
- `list_item`
- `table`
- `table_head`
- `table_row`
- `table_cell`
- `code_block`
- `inline_code`
- `link`
- `image`
- `text`
- `html`
- `break`
- `thematic_break`
- `footnote_reference`
- `task_marker`
- `inline`
- `block`

Every AST node carries:

- `markdown_ast_node_id`
- `markdown_ast_document_id`
- `matter_id`
- `document_id`
- `document_version_id`
- `object_blob_id`
- `ingestion_run_id`
- `index_run_id`
- `parent_ast_node_id`
- `previous_ast_node_id`
- `node_kind`
- `tag`
- `ordinal`
- `depth`
- `structure_path`
- `text_hash`
- `text_excerpt`
- `byte_start`
- `byte_end`
- `char_start`
- `char_end`
- `source_span_ids`
- `text_chunk_ids`
- `evidence_span_ids`
- `search_index_record_ids`
- `review_status`

Full raw Markdown text remains in source/version artifacts, not graph payloads.

## Stable IDs

AST IDs are version-aware.

The AST document ID seed includes:

- `document_id`
- `document_version_id` when present
- sanitized source text hash

The AST node ID seed includes:

- `document_version_id` when present
- parser path
- node kind
- ordinal
- byte span

The result is idempotent for the same document version and cleanly separated for later document versions.

## Graph Contract

New graph node labels:

- `MarkdownAstDocument`
- `MarkdownAstNode`
- `MarkdownSemanticUnit`
- `CaseBuilderEmbeddingRun`
- `CaseBuilderEmbeddingRecord`
- `CaseEntity`

Existing provenance labels participate in the Markdown graph:

- `CaseDocument`
- `DocumentVersion`
- `ObjectBlob`
- `IngestionRun`
- `IndexRun`
- `SourceSpan`
- `TextChunk`
- `EvidenceSpan`
- `SearchIndexRecord`
- `EntityMention`
- `Fact`
- `TimelineSuggestion`
- `TimelineEvent`

Core relationships:

- `CaseDocument -[:HAS_MARKDOWN_AST_DOCUMENT]-> MarkdownAstDocument`
- `DocumentVersion -[:HAS_MARKDOWN_AST_DOCUMENT]-> MarkdownAstDocument`
- `MarkdownAstDocument -[:HAS_AST_ROOT]-> MarkdownAstNode`
- `MarkdownAstDocument -[:CONTAINS_AST_NODE]-> MarkdownAstNode`
- `CaseDocument -[:CONTAINS_AST_NODE]-> MarkdownAstNode`
- `MarkdownAstNode -[:PARENT_OF]-> MarkdownAstNode`
- `MarkdownAstNode -[:NEXT_AST_NODE]-> MarkdownAstNode`
- `MarkdownAstNode -[:OVERLAPS_SOURCE_SPAN]-> SourceSpan`
- `MarkdownAstNode -[:OVERLAPS_TEXT_CHUNK]-> TextChunk`
- `MarkdownAstNode -[:OVERLAPS_EVIDENCE_SPAN]-> EvidenceSpan`
- `MarkdownAstNode -[:INDEXED_AS]-> SearchIndexRecord`
- `MarkdownAstNode -[:HAS_ENTITY_MENTION]-> EntityMention`
- `EntityMention -[:RESOLVES_TO]-> CaseEntity`
- `CaseEntity -[:MAY_MATCH_PARTY]-> Party`
- `MarkdownAstNode -[:SUPPORTS_FACT]-> Fact`
- `MarkdownAstNode -[:PROPOSES_TIMELINE]-> TimelineSuggestion`
- `MarkdownAstNode -[:SUPPORTS_EVENT]-> TimelineEvent`
- `CaseDocument -[:HAS_EMBEDDING_RUN]-> CaseBuilderEmbeddingRun`
- `DocumentVersion -[:HAS_EMBEDDING_RUN]-> CaseBuilderEmbeddingRun`
- `IndexRun -[:PRODUCED]-> CaseBuilderEmbeddingRun`
- `CaseDocument -[:HAS_EMBEDDING_RECORD]-> CaseBuilderEmbeddingRecord`
- `DocumentVersion -[:HAS_EMBEDDING_RECORD]-> CaseBuilderEmbeddingRecord`
- `IndexRun -[:PRODUCED]-> CaseBuilderEmbeddingRecord`
- `CaseBuilderEmbeddingRun -[:PRODUCED]-> CaseBuilderEmbeddingRecord`
- `TextChunk -[:HAS_EMBEDDING_RECORD]-> CaseBuilderEmbeddingRecord`
- `MarkdownSemanticUnit -[:HAS_EMBEDDING_RECORD]-> CaseBuilderEmbeddingRecord`
- `MarkdownAstDocument -[:HAS_EMBEDDING_RECORD]-> CaseBuilderEmbeddingRecord`
- `CaseBuilderEmbeddingRecord -[:EMBEDS_SOURCE_SPAN]-> SourceSpan`
- `CaseBuilderEmbeddingRecord -[:EMBEDS_TEXT_CHUNK]-> TextChunk`
- `CaseBuilderEmbeddingRecord -[:EMBEDS_AST_NODE]-> MarkdownAstNode`
- `CaseBuilderEmbeddingRecord -[:EMBEDS_SEMANTIC_UNIT]-> MarkdownSemanticUnit`
- `CaseBuilderEmbeddingRecord -[:CENTROID_OF]-> CaseBuilderEmbeddingRecord`

Party-like entity matches are review candidates only. They must not mutate `CaseParty` automatically.

## Neo4j Constraints And Indexes

The CaseBuilder index setup includes uniqueness constraints for:

- `casebuilder_markdown_ast_document_id`
- `casebuilder_markdown_ast_node_id`
- `casebuilder_case_entity_id`
- `casebuilder_embedding_run_id`
- `casebuilder_embedding_record_id`

It also includes matter/document lookup indexes and a full-text index for AST excerpts/structure paths.

The vector index for CaseBuilder Markdown embeddings is:

```cypher
CREATE VECTOR INDEX casebuilder_markdown_embedding_1024 IF NOT EXISTS
FOR (n:CaseBuilderEmbeddingRecord) ON n.embedding
OPTIONS {indexConfig: {`vector.dimensions`: 1024, `vector.similarity_function`: 'cosine'}}
```

The vector property lives on `CaseBuilderEmbeddingRecord.embedding`. Vectors must not be duplicated into JSON payload fields.

## API Contract

Routes are unchanged. New response fields are backward-compatible optional fields or arrays.

`DocumentExtractionResponse` adds:

- `markdown_ast_document`
- `markdown_ast_nodes`
- `entities`
- `embedding_run`

`DocumentWorkspace` adds:

- `markdown_ast_document`
- `markdown_ast_nodes`
- `text_chunks`
- `evidence_spans`
- `entity_mentions`
- `entities`
- `search_index_records`
- `embedding_runs`
- `embedding_records`
- `embedding_coverage`
- `proposed_facts`
- `timeline_suggestions`

`MatterIndexRunDocumentResult` adds:

- `produced_markdown_ast_nodes`
- `produced_entities`
- `produced_embedding_records`

Embedding routes:

- `POST /matters/{matter_id}/embeddings/run`
- `POST /matters/{matter_id}/documents/{document_id}/embeddings/run`
- `POST /matters/{matter_id}/embeddings/search`

Embedding search returns score, target kind/id, document/version, excerpt, source spans, text chunks, semantic units, AST node IDs, and stale status.

These records may include `markdown_ast_node_ids`:

- `TextChunk`
- `EvidenceSpan`
- `EntityMention`
- `ExtractedTextChunk`
- `CaseFact`
- `CaseTimelineEvent`
- `TimelineSuggestion`
- create fact/timeline request payloads

Frontend normalizers must default missing new arrays to `[]` and missing AST document objects to `null`.

## Matter Graph Modes

`CaseGraphResponse.modes` now includes:

- `markdown`
- `markdown_ast`
- `markdown_embeddings`
- `entities`
- `provenance`

The matter graph uses compact `metadata` maps. It does not return full Markdown text.

Frontend rendering should support these node kinds:

- `markdown_ast_document`
- `markdown_ast_node`
- `source_span`
- `text_chunk`
- `evidence_span`
- `search_index_record`
- `embedding_run`
- `embedding_record`
- `entity_mention`
- `case_entity`
- `document_version`
- `index_run`
- `extraction_manifest`

## Document Workspace UI

The Document Workspace includes a Markdown Graph tab.

It shows:

- AST node count.
- Entity count.
- Chunk count.
- Source span count.
- Parser/root metadata.
- Embedding coverage, current/stale counts, latest run status, and retry action.
- Document-scoped Markdown embedding search with source-jump results.
- Outline nodes.
- Canonical entity candidates.
- Proposed facts and timeline suggestions.
- Selected source span details.

Clicking an AST graph item with offsets selects and scrolls to the source range in the Markdown editor.

Review actions stay explicit. The panel can show candidates, but durable approvals still go through fact, timeline, entity, or party-linking workflows.

## Entity Extraction Rules

Deterministic extraction is the base layer.

Signal types include:

- dates
- Oregon statutes
- ORCP/UTCR rules
- Oregon constitutional citations
- session laws
- money amounts
- party-like names
- organizations
- courts
- places
- local code/statute-like references

The broad capitalized-phrase detector is filtered by:

- minimum meaningful word count
- generic false-positive words
- generated notice terms
- document label phrases
- month/date-only labels
- organization suffixes
- court/place context
- party-role context

Entity mentions group into `CaseEntity` by `entity_type` plus normalized mention text. The highest-confidence mention can update the canonical display name.

## Review-First Guarantees

The pipeline must preserve these invariants:

- No extracted fact becomes operative automatically.
- No timeline suggestion becomes an event automatically.
- No entity mention mutates the party list automatically.
- Every proposed fact/timeline item generated from Markdown must retain source span, chunk, and AST-node provenance when overlap exists.
- AI/provider enrichment may improve labels, grouping, confidence, explanations, and warnings only.
- AI/provider enrichment must not invent source spans, AST nodes, facts, or events.

## Artifact Boundary

Neo4j stores:

- IDs.
- Hashes.
- source coordinates.
- capped excerpts.
- review status.
- relationship edges.
- compact metadata.

Object storage/local artifact storage keeps:

- original source bytes.
- normalized Markdown/text artifacts.
- page artifacts.
- manifest JSON.
- heavy audit payloads.

The manifest should carry heavier per-chunk structure maps and source-unit hints. Graph payloads should stay compact.

## Verification

Backend:

```bash
cargo test -p orsgraph-api markdown_graph -- --nocapture
cargo test -p orsgraph-api markdown_adapter -- --nocapture
cargo test -p orsgraph-api markdown_only -- --nocapture
cargo test -p orsgraph-api casebuilder
```

Frontend:

```bash
pnpm --dir frontend run test -- document-library document-workspace matter-graph
pnpm --dir frontend run typecheck
```

Smoke:

```bash
node --check frontend/scripts/smoke-casebuilder.mjs
curl -fsS http://127.0.0.1:8080/api/v1/health
```

Only run the live CaseBuilder smoke when `/api/v1/health` responds.

## Operational Notes

- If a Markdown document reindexes with the same `document_version_id` and content, AST IDs should remain stable.
- If a new document version is created, AST IDs should change for clean lineage.
- `view_only` files should not be rejected or deleted.
- Search text should not include CaseBuilder sidecar comments or generated review notices.
- Graph views should remain matter-scoped.
- Frontend route names and API route names should remain unchanged.
