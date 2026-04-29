UNWIND $rows AS row
MERGE (se:StatusEvent {status_event_id: row.status_event_id})
SET se += row { .status_type, .status_text, .source_document_id, .canonical_id, .version_id,
                .event_year, .effective_date, .source_note_id, .effect_type, .trigger_text,
                .operative_date, .repeal_date, .session_law_ref, .confidence, .extraction_method }
SET se.id = row.status_event_id,
    se.graph_kind = 'status_event',
    se.schema_version = '1.0.0',
    se.source_system = 'ors_crawler',
    se.updated_at = datetime()
SET se.created_at = coalesce(se.created_at, datetime())
WITH row, se
OPTIONAL MATCH (lti:LegalTextIdentity {canonical_id: row.canonical_id})
FOREACH (_ IN CASE WHEN lti IS NULL THEN [] ELSE [1] END |
    MERGE (lti)-[:HAS_STATUS_EVENT]->(se)
)
WITH row, se
OPTIONAL MATCH (ltv:LegalTextVersion {version_id: row.version_id})
FOREACH (_ IN CASE WHEN ltv IS NULL THEN [] ELSE [1] END |
    MERGE (ltv)-[:HAS_STATUS_EVENT]->(se)
)
WITH row, se
OPTIONAL MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
    MERGE (se)-[:SUPPORTED_BY]->(sd)
)
WITH row, se
OPTIONAL MATCH (sn:SourceNote {source_note_id: row.source_note_id})
FOREACH (_ IN CASE WHEN sn IS NULL THEN [] ELSE [1] END |
    MERGE (se)-[:SUPPORTED_BY]->(sn)
)
