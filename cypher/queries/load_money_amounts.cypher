UNWIND $rows AS row
MERGE (m:MoneyAmount:LegalSemanticNode {money_amount_id: row.money_amount_id})
SET m += row { .amount_text, .amount_value, .percent_value, .amount_type,
               .source_provision_id, .confidence }
SET m.semantic_id = row.money_amount_id,
    m.semantic_type = 'MoneyAmount',
    m.id = row.money_amount_id,
    m.graph_kind = 'legal_semantic_node',
    m.schema_version = '1.0.0',
    m.source_system = 'ors_crawler',
    m.updated_at = datetime()
SET m.created_at = coalesce(m.created_at, datetime())
WITH row, m
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(m)
    MERGE (m)-[:SUPPORTED_BY]->(p)
)
