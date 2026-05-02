UNWIND $rows AS row
MERGE (r:Remedy:LegalSemanticNode {remedy_id: row.remedy_id})
SET r += row { .text, .remedy_type, .available_to, .available_against,
               .source_provision_id, .confidence }
SET r.semantic_id = row.remedy_id,
    r.semantic_type = 'Remedy',
    r.id = row.remedy_id,
    r.graph_kind = 'legal_semantic_node',
    r.schema_version = '1.0.0',
    r.source_system = 'ors_crawler',
    r.updated_at = datetime()
SET r.created_at = coalesce(r.created_at, datetime())
WITH row, r
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(r)
    MERGE (r)-[:SUPPORTED_BY]->(p)
)
