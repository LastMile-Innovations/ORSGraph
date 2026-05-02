UNWIND $rows AS row
MERGE (rn:RequiredNotice:LegalSemanticNode {required_notice_id: row.required_notice_id})
SET rn += row { .notice_type, .text, .required_recipient, .required_sender,
                .trigger_event, .source_provision_id, .confidence }
SET rn.semantic_id = row.required_notice_id,
    rn.semantic_type = 'RequiredNotice',
    rn.id = row.required_notice_id,
    rn.graph_kind = 'legal_semantic_node',
    rn.schema_version = '1.0.0',
    rn.source_system = 'ors_crawler',
    rn.updated_at = datetime()
SET rn.created_at = coalesce(rn.created_at, datetime())
WITH row, rn
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(rn)
    MERGE (p)-[:REQUIRES_NOTICE]->(rn)
    MERGE (rn)-[:SUPPORTED_BY]->(p)
)
