UNWIND $rows AS row
MERGE (c:Commentary {commentary_id: row.commentary_id})
SET c += row { .source_document_id, .canonical_id, .version_id, .source_provision_id,
             .target_canonical_id, .target_provision_id, .citation, .authority_family,
             .corpus_id, .authority_level, .source_role, .commentary_type, .text,
             .normalized_text, .source_page_start, .source_page_end, .confidence,
             .extraction_method }
SET c.id = row.commentary_id,
    c.graph_kind = 'commentary',
    c.schema_version = '1.0.0',
    c.source_system = 'ors_crawler',
    c.authority_level = coalesce(row.authority_level, c.authority_level, 65),
    c.source_role = coalesce(row.source_role, c.source_role, 'official_commentary'),
    c.updated_at = datetime()
SET c.created_at = coalesce(c.created_at, datetime())
