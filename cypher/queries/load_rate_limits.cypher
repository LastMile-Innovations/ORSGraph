UNWIND $rows AS row
MERGE (rl:RateLimit:LegalSemanticNode {rate_limit_id: row.rate_limit_id})
SET rl += row { .rate_type, .percent_value, .amount_text, .cap_text,
                .source_provision_id, .confidence }
SET rl.semantic_id = row.rate_limit_id,
    rl.semantic_type = 'RateLimit',
    rl.id = row.rate_limit_id,
    rl.graph_kind = 'legal_semantic_node',
    rl.schema_version = '1.0.0',
    rl.source_system = 'ors_crawler',
    rl.updated_at = datetime()
SET rl.created_at = coalesce(rl.created_at, datetime())
WITH row, rl
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(rl)
    MERGE (rl)-[:SUPPORTED_BY]->(p)
)
WITH row, rl
MATCH (tr:TaxRule {source_provision_id: row.source_provision_id})
MERGE (tr)-[:HAS_RATE_LIMIT]->(rl)
