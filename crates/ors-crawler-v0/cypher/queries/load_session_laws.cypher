UNWIND $rows AS row
MERGE (sl:SessionLaw {session_law_id: row.session_law_id})
SET sl += row { .jurisdiction_id, .citation, .year, .chapter, .section, .bill_number,
                .effective_date, .text, .raw_text, .source_document_id, .source_note_id,
                .confidence }
SET sl.id = row.session_law_id,
    sl.graph_kind = 'session_law',
    sl.schema_version = '1.0.0',
    sl.source_system = 'ors_crawler',
    sl.updated_at = datetime()
SET sl.created_at = coalesce(sl.created_at, datetime())
WITH row, sl
OPTIONAL MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
    MERGE (sl)-[:DERIVED_FROM]->(sd)
)
WITH row, sl
OPTIONAL MATCH (sn:SourceNote {source_note_id: row.source_note_id})
FOREACH (_ IN CASE WHEN sn IS NULL THEN [] ELSE [1] END |
    MERGE (sn)-[:MENTIONS_SESSION_LAW]->(sl)
)
