UNWIND $rows AS row
MERGE (o:Obligation:LegalSemanticNode {obligation_id: row.obligation_id})
SET o += row { .text, .actor_text, .action_text, .object_text, .condition_text,
               .source_provision_id, .confidence, .actor_id, .action_id, .deadline_id,
               .exception_id, .penalty_id }
SET o.semantic_id = row.obligation_id,
    o.semantic_type = 'Obligation',
    o.id = row.obligation_id,
    o.graph_kind = 'legal_semantic_node',
    o.schema_version = '1.0.0',
    o.source_system = 'ors_crawler',
    o.updated_at = datetime()
SET o.created_at = coalesce(o.created_at, datetime())
WITH row, o
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(o)
    MERGE (o)-[:SUPPORTED_BY]->(p)
)
WITH row, o
OPTIONAL MATCH (a:LegalActor {actor_id: row.actor_id})
FOREACH (_ IN CASE WHEN a IS NULL THEN [] ELSE [1] END |
    MERGE (o)-[:IMPOSED_ON]->(a)
)
WITH row, o
OPTIONAL MATCH (la:LegalAction {action_id: row.action_id})
FOREACH (_ IN CASE WHEN la IS NULL THEN [] ELSE [1] END |
    MERGE (o)-[:REQUIRES_ACTION]->(la)
)
WITH row, o
OPTIONAL MATCH (d:Deadline {deadline_id: row.deadline_id})
FOREACH (_ IN CASE WHEN d IS NULL THEN [] ELSE [1] END |
    MERGE (o)-[:HAS_DEADLINE]->(d)
)
WITH row, o
OPTIONAL MATCH (e:Exception {exception_id: row.exception_id})
FOREACH (_ IN CASE WHEN e IS NULL THEN [] ELSE [1] END |
    MERGE (o)-[:SUBJECT_TO]->(e)
)
WITH row, o
OPTIONAL MATCH (pnl:Penalty {penalty_id: row.penalty_id})
FOREACH (_ IN CASE WHEN pnl IS NULL THEN [] ELSE [1] END |
    MERGE (o)-[:VIOLATION_PENALIZED_BY]->(pnl)
)
