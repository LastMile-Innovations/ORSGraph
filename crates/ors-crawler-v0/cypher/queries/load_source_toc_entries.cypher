UNWIND $rows AS row
MERGE (toc:SourceTocEntry {source_toc_entry_id: row.source_toc_entry_id})
SET toc += row { .source_document_id, .citation, .canonical_id, .title, .chapter,
               .page_label, .page_number, .toc_order, .entry_type, .confidence }
SET toc.id = row.source_toc_entry_id,
    toc.graph_kind = 'source',
    toc.schema_version = '1.0.0',
    toc.source_system = 'ors_crawler',
    toc.updated_at = datetime()
SET toc.created_at = coalesce(toc.created_at, datetime())
WITH toc, row
MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
MERGE (sd)-[:HAS_TOC_ENTRY]->(toc)
