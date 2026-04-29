UNWIND $rows AS row
MERGE (le:LineageEvent {lineage_event_id: row.lineage_event_id})
SET le += row { .source_note_id, .from_canonical_id, .to_canonical_id, .current_canonical_id,
                .lineage_type, .raw_text, .year, .confidence }
SET le.id = row.lineage_event_id,
    le.graph_kind = 'lineage_event',
    le.schema_version = '1.0.0',
    le.source_system = 'ors_crawler',
    le.updated_at = datetime()
SET le.created_at = coalesce(le.created_at, datetime())
WITH row, le
OPTIONAL MATCH (cur:LegalTextIdentity {canonical_id: row.current_canonical_id})
FOREACH (_ IN CASE WHEN cur IS NULL THEN [] ELSE [1] END |
    MERGE (cur)-[:HAS_LINEAGE_EVENT]->(le)
)
WITH row, le
OPTIONAL MATCH (fromId:LegalTextIdentity {canonical_id: row.from_canonical_id})
OPTIONAL MATCH (toId:LegalTextIdentity {canonical_id: row.to_canonical_id})
FOREACH (_ IN CASE WHEN row.lineage_type STARTS WITH 'renumbered' AND fromId IS NOT NULL AND toId IS NOT NULL THEN [1] ELSE [] END |
    MERGE (fromId)-[:RENUMBERED_TO]->(toId)
)
FOREACH (_ IN CASE WHEN row.lineage_type = 'formerly' AND fromId IS NOT NULL AND toId IS NOT NULL THEN [1] ELSE [] END |
    MERGE (toId)-[:FORMERLY]->(fromId)
)
WITH row, le
OPTIONAL MATCH (sn:SourceNote {source_note_id: row.source_note_id})
FOREACH (_ IN CASE WHEN sn IS NULL THEN [] ELSE [1] END |
    MERGE (le)-[:SUPPORTED_BY]->(sn)
)
