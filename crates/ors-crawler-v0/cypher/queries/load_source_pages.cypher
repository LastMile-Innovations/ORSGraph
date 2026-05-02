UNWIND $rows AS row
MERGE (sp:SourcePage {source_page_id: row.source_page_id})
SET sp += row { .source_document_id, .page_number, .printed_label, .text, .normalized_text, .text_hash }
SET sp.id = row.source_page_id,
    sp.graph_kind = 'source',
    sp.schema_version = '1.0.0',
    sp.source_system = 'ors_crawler',
    sp.updated_at = datetime()
SET sp.created_at = coalesce(sp.created_at, datetime())
WITH sp, row
MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
MERGE (sd)-[:HAS_PAGE]->(sp)
