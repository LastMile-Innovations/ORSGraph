UNWIND $rows AS row
MERGE (wpa:WorkProductRulePackAuthority {rule_pack_authority_id: row.rule_pack_authority_id})
SET wpa += row { .rule_pack_id, .authority_document_id, .work_product_type, .jurisdiction_id,
                 .inclusion_reason }
SET wpa.id = row.rule_pack_authority_id,
    wpa.graph_kind = 'rule_pack_authority',
    wpa.schema_version = '1.0.0',
    wpa.source_system = 'ors_crawler',
    wpa.updated_at = datetime()
SET wpa.created_at = coalesce(wpa.created_at, datetime())
WITH wpa, row
MATCH (doc:RuleAuthorityDocument {authority_document_id: row.authority_document_id})
MERGE (wpa)-[:INCLUDES_AUTHORITY]->(doc)
WITH wpa, row, doc
OPTIONAL MATCH (pack:WorkProductRulePack {rule_pack_id: row.rule_pack_id})
FOREACH (_ IN CASE WHEN pack IS NULL THEN [] ELSE [1] END |
    MERGE (pack)-[:INCLUDES_AUTHORITY]->(doc)
)
