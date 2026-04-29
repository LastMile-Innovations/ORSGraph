UNWIND $rows AS row
MERGE (pnl:Penalty:LegalSemanticNode {penalty_id: row.penalty_id})
SET pnl += row { .text, .penalty_type, .amount, .minimum, .maximum, .condition,
                 .source_provision_id, .confidence, .obligation_id, .criminal_class,
                 .civil_penalty_amount, .min_amount, .max_amount, .jail_term,
                 .license_suspension, .revocation, .target_conduct, .target_citation }
SET pnl.semantic_id = row.penalty_id,
    pnl.semantic_type = 'Penalty',
    pnl.id = row.penalty_id,
    pnl.graph_kind = 'legal_semantic_node',
    pnl.schema_version = '1.0.0',
    pnl.source_system = 'ors_crawler',
    pnl.updated_at = datetime()
SET pnl.created_at = coalesce(pnl.created_at, datetime())
WITH row, pnl
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(pnl)
    MERGE (pnl)-[:SUPPORTED_BY]->(p)
)
WITH row, pnl
OPTIONAL MATCH (o:Obligation {obligation_id: row.obligation_id})
FOREACH (_ IN CASE WHEN o IS NULL THEN [] ELSE [1] END |
    MERGE (pnl)-[:PENALIZES_VIOLATION_OF]->(o)
)
