UNWIND $rows AS row
MERGE (a:LegalActor {actor_id: row.actor_id})
SET a += row { .name, .normalized_name, .actor_type, .jurisdiction_id }
SET a.id = row.actor_id,
    a.graph_kind = 'legal_actor',
    a.schema_version = '1.0.0',
    a.source_system = 'ors_crawler',
    a.updated_at = datetime()
SET a.created_at = coalesce(a.created_at, datetime())
