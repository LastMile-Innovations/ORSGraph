// Load retrieval chunk nodes into the graph
// Parameter: $rows (array of retrieval chunks)

UNWIND $rows AS row
FILTER row.chunk_id IS NOT NULL
CALL (row) {
    LET chunkId = row.chunk_id
    MERGE (c:RetrievalChunk {chunk_id: chunkId})
    SET c += row { .chunk_type, .text, .breadcrumb, .source_provision_id, .source_version_id, .parent_version_id,
                 .canonical_id, .citation, .jurisdiction_id, .authority_level, .edition_year,
                 .authority_family, .corpus_id, .authority_type, .effective_date, .chapter,
                 .source_page_start, .source_page_end,
                 .embedding_input_hash, .embedding_policy, .answer_policy, .chunk_schema_version,
                 .retrieval_profile, .search_weight, .source_kind, .source_id,
                 .token_count, .max_tokens, .context_window, .chunking_strategy, .chunk_version,
                 .overlap_tokens, .split_reason, .part_index, .part_count,
                 .is_definition_candidate, .is_exception_candidate, .is_penalty_candidate,
                 .heading_path, .structural_context }
    SET c.id = chunkId,
        c.graph_kind = 'chunk',
        c.schema_version = '1.0.0',
        c.source_system = 'ors_crawler',
        c.updated_at = datetime()
    SET c.created_at = coalesce(c.created_at, datetime())
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
