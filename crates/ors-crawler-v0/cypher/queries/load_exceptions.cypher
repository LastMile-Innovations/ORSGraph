UNWIND $rows AS row
MERGE (e:Exception:LegalSemanticNode {exception_id: row.exception_id})
SET e += row { .text, .trigger_phrase, .exception_type, .source_provision_id, .confidence,
               .target_provision_id, .target_canonical_id, .target_obligation_id }
SET e.semantic_id = row.exception_id,
    e.semantic_type = 'Exception',
    e.id = row.exception_id,
    e.graph_kind = 'legal_semantic_node',
    e.schema_version = '1.0.0',
    e.source_system = 'ors_crawler',
    e.updated_at = datetime()
SET e.created_at = coalesce(e.created_at, datetime())
WITH row, e
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(e)
    MERGE (e)-[:SUPPORTED_BY]->(p)
)
WITH row, e
OPTIONAL MATCH (target:Provision {provision_id: row.target_provision_id})
FOREACH (_ IN CASE WHEN target IS NULL THEN [] ELSE [1] END |
    MERGE (e)-[:EXCEPTION_TO]->(target)
)
