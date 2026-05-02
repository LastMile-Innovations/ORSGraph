UNWIND $rows AS row
MERGE (te:TemporalEffect {temporal_effect_id: row.temporal_effect_id})
SET te += row { .source_note_id, .source_provision_id, .version_id, .canonical_id, .effect_type,
                .trigger_text, .effective_date, .operative_date, .repeal_date,
                .expiration_date, .session_law_ref, .confidence }
SET te.id = row.temporal_effect_id,
    te.graph_kind = 'temporal_effect',
    te.schema_version = '1.0.0',
    te.source_system = 'ors_crawler',
    te.updated_at = datetime()
SET te.created_at = coalesce(te.created_at, datetime())
WITH row, te
OPTIONAL MATCH (ltv:LegalTextVersion {version_id: row.version_id})
FOREACH (_ IN CASE WHEN ltv IS NULL THEN [] ELSE [1] END |
    MERGE (ltv)-[:HAS_TEMPORAL_EFFECT]->(te)
)
WITH row, te
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:HAS_TEMPORAL_EFFECT]->(te)
    MERGE (te)-[:SUPPORTED_BY]->(p)
)
WITH row, te
OPTIONAL MATCH (sn:SourceNote {source_note_id: row.source_note_id})
FOREACH (_ IN CASE WHEN sn IS NULL THEN [] ELSE [1] END |
    MERGE (te)-[:SUPPORTED_BY]->(sn)
)
WITH row, te
OPTIONAL MATCH (sl:SessionLaw {citation: row.session_law_ref})
FOREACH (_ IN CASE WHEN sl IS NULL THEN [] ELSE [1] END |
    MERGE (te)-[:REFERENCES_SESSION_LAW]->(sl)
)
