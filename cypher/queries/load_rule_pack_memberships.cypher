UNWIND $rows AS row
MERGE (m:RulePackMembership {membership_id: row.membership_id})
SET m += row { .rule_pack_id, .requirement_id, .requirement_type, .source_provision_id,
             .source_citation, .applies_to, .severity_default }
SET m.id = row.membership_id,
    m.graph_kind = 'rule_pack',
    m.schema_version = '1.0.0',
    m.source_system = 'ors_crawler',
    m.updated_at = datetime()
SET m.created_at = coalesce(m.created_at, datetime())
WITH m, row
MATCH (pack:WorkProductRulePack {rule_pack_id: row.rule_pack_id})
MATCH (req:ProceduralRequirement {requirement_id: row.requirement_id})
MERGE (pack)-[:INCLUDES_RULE]->(req)
MERGE (m)-[:MEMBERSHIP_OF]->(pack)
MERGE (m)-[:INCLUDES]->(req)
WITH m, row
MATCH (p:Provision {provision_id: row.source_provision_id})
MERGE (m)-[:BASED_ON]->(p)
