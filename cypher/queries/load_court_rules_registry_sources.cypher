UNWIND $rows AS row
MERGE (src:CourtRulesRegistrySource {registry_source_id: row.registry_source_id})
SET src += row { .source_type, .jurisdiction, .jurisdiction_id, .source_url,
                 .snapshot_date, .contains_current_future, .contains_prior }
SET src.id = row.registry_source_id,
    src.graph_kind = 'court_rules_registry',
    src.schema_version = '1.0.0',
    src.source_system = 'ors_crawler',
    src.updated_at = datetime()
SET src.created_at = coalesce(src.created_at, datetime())
WITH src, row
MATCH (j:Jurisdiction {jurisdiction_id: row.jurisdiction_id})
MERGE (src)-[:APPLIES_TO_JURISDICTION]->(j)
