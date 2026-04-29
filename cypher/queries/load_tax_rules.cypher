UNWIND $rows AS row
MERGE (tr:TaxRule:LegalSemanticNode {tax_rule_id: row.tax_rule_id})
SET tr += row { .tax_type, .rate_text, .base, .cap, .recipient, .fund_name,
                .source_provision_id, .confidence }
SET tr.semantic_id = row.tax_rule_id,
    tr.semantic_type = 'TaxRule',
    tr.id = row.tax_rule_id,
    tr.graph_kind = 'legal_semantic_node',
    tr.schema_version = '1.0.0',
    tr.source_system = 'ors_crawler',
    tr.updated_at = datetime()
SET tr.created_at = coalesce(tr.created_at, datetime())
WITH row, tr
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(tr)
    MERGE (tr)-[:SUPPORTED_BY]->(p)
)
WITH row, tr
MATCH (rl:RateLimit {source_provision_id: row.source_provision_id})
MERGE (tr)-[:HAS_RATE_LIMIT]->(rl)
