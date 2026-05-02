UNWIND $rows AS row
MERGE (lsn:LegalSemanticNode {semantic_id: row.semantic_id})
SET lsn += row { .semantic_type, .text, .normalized_text, .source_provision_id,
                 .confidence, .review_status, .extraction_method }
SET lsn.id = row.semantic_id,
    lsn.graph_kind = 'legal_semantic_node',
    lsn.schema_version = '1.0.0',
    lsn.source_system = 'ors_crawler',
    lsn.updated_at = datetime()
SET lsn.created_at = coalesce(lsn.created_at, datetime())

// Dynamically apply specialized labels based on semantic_type without APOC
FOREACH (_ IN CASE WHEN row.semantic_type = 'Obligation' THEN [1] ELSE [] END | SET lsn:Obligation)
FOREACH (_ IN CASE WHEN row.semantic_type = 'Permission' THEN [1] ELSE [] END | SET lsn:Permission)
FOREACH (_ IN CASE WHEN row.semantic_type = 'Power' THEN [1] ELSE [] END | SET lsn:Power)
FOREACH (_ IN CASE WHEN row.semantic_type = 'AuthorityGrant' THEN [1] ELSE [] END | SET lsn:AuthorityGrant)
FOREACH (_ IN CASE WHEN row.semantic_type = 'Prohibition' THEN [1] ELSE [] END | SET lsn:Prohibition)
FOREACH (_ IN CASE WHEN row.semantic_type = 'Definition' THEN [1] ELSE [] END | SET lsn:Definition)
FOREACH (_ IN CASE WHEN row.semantic_type = 'Exception' THEN [1] ELSE [] END | SET lsn:Exception)
FOREACH (_ IN CASE WHEN row.semantic_type = 'Deadline' THEN [1] ELSE [] END | SET lsn:Deadline)
FOREACH (_ IN CASE WHEN row.semantic_type = 'Penalty' THEN [1] ELSE [] END | SET lsn:Penalty)
FOREACH (_ IN CASE WHEN row.semantic_type = 'Remedy' THEN [1] ELSE [] END | SET lsn:Remedy)
FOREACH (_ IN CASE WHEN row.semantic_type = 'MoneyAmount' THEN [1] ELSE [] END | SET lsn:MoneyAmount)
FOREACH (_ IN CASE WHEN row.semantic_type = 'TaxRule' THEN [1] ELSE [] END | SET lsn:TaxRule)
FOREACH (_ IN CASE WHEN row.semantic_type = 'BondAuthority' THEN [1] ELSE [] END | SET lsn:BondAuthority)
FOREACH (_ IN CASE WHEN row.semantic_type = 'DistributionRule' THEN [1] ELSE [] END | SET lsn:DistributionRule)
FOREACH (_ IN CASE WHEN row.semantic_type = 'RequiredNotice' THEN [1] ELSE [] END | SET lsn:RequiredNotice)
FOREACH (_ IN CASE WHEN row.semantic_type = 'FormText' THEN [1] ELSE [] END | SET lsn:FormText)
FOREACH (_ IN CASE WHEN row.semantic_type = 'TimePeriod' THEN [1] ELSE [] END | SET lsn:TimePeriod)
FOREACH (_ IN CASE WHEN row.semantic_type = 'Fee' THEN [1] ELSE [] END | SET lsn:Fee)
FOREACH (_ IN CASE WHEN row.semantic_type = 'Procedure' THEN [1] ELSE [] END | SET lsn:Procedure)

WITH row, lsn
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(lsn)
    MERGE (lsn)-[:SUPPORTED_BY]->(p)
)
