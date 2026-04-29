// Load source document nodes into the graph
// Parameter: $rows (array of source documents)

UNWIND $rows AS row
MERGE (sd:SourceDocument {source_document_id: row.source_document_id})
SET sd += row { .source_provider, .source_kind, .url, .chapter, .edition_year,
             .chapter_title, .html_encoding, .source_path, .paragraph_count,
             .first_body_paragraph_index, .parser_profile, .official_status,
             .disclaimer_required, .raw_hash, .normalized_hash }
SET sd.id = row.source_document_id,
    sd.graph_kind = 'source',
    sd.schema_version = '1.0.0',
    sd.source_system = 'ors_crawler',
    sd.updated_at = datetime()
SET sd.created_at = coalesce(sd.created_at, datetime())
