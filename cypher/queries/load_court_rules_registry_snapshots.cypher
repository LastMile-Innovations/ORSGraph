UNWIND $rows AS row
MERGE (snap:CourtRulesRegistrySnapshot {registry_snapshot_id: row.registry_snapshot_id})
SET snap += row { .registry_source_id, .snapshot_date, .jurisdiction_id, .source_url,
                  .parser_profile, .entry_count, .input_hash }
SET snap.id = row.registry_snapshot_id,
    snap.graph_kind = 'court_rules_registry',
    snap.schema_version = '1.0.0',
    snap.source_system = 'ors_crawler',
    snap.updated_at = datetime()
SET snap.created_at = coalesce(snap.created_at, datetime())
WITH snap, row
MATCH (src:CourtRulesRegistrySource {registry_source_id: row.registry_source_id})
MERGE (src)-[:HAS_SNAPSHOT]->(snap)
