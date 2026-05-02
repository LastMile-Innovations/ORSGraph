UNWIND $rows AS row
MERGE (d:Deadline:LegalSemanticNode {deadline_id: row.deadline_id})
SET d += row { .text, .duration, .date_text, .trigger_event, .actor, .action_required,
               .source_provision_id, .confidence, .obligation_id }
SET d.semantic_id = row.deadline_id,
    d.semantic_type = 'Deadline',
    d.id = row.deadline_id,
    d.graph_kind = 'legal_semantic_node',
    d.schema_version = '1.0.0',
    d.source_system = 'ors_crawler',
    d.updated_at = datetime()
SET d.created_at = coalesce(d.created_at, datetime())
WITH row, d
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(d)
    MERGE (d)-[:SUPPORTED_BY]->(p)
)
WITH row, d
OPTIONAL MATCH (o:Obligation {obligation_id: row.obligation_id})
FOREACH (_ IN CASE WHEN o IS NULL THEN [] ELSE [1] END |
    MERGE (d)-[:APPLIES_TO]->(o)
)
