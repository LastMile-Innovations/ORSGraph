// materialize_semantic_edges.cypher
// Materialize EXPRESSES and SUPPORTED_BY relationships for all semantic nodes

CALL {
    MATCH (sn:ProceduralRequirement)
    MATCH (p:Provision {provision_id: sn.source_provision_id})
    MERGE (p)-[:EXPRESSES]->(sn)
    MERGE (sn)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (sn:LegalSemanticNode)
    MATCH (p:Provision {provision_id: sn.source_provision_id})
    MERGE (p)-[:EXPRESSES]->(sn)
    MERGE (sn)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

// Also for specific sub-types that might have been loaded separately but inherit LegalSemanticNode
CALL {
    MATCH (sn:Obligation)
    MATCH (p:Provision {provision_id: sn.source_provision_id})
    MERGE (p)-[:EXPRESSES]->(sn)
    MERGE (sn)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (m:RulePackMembership)
    MATCH (pack:WorkProductRulePack {rule_pack_id: m.rule_pack_id})
    MATCH (req:ProceduralRequirement {requirement_id: m.requirement_id})
    MERGE (pack)-[:INCLUDES_RULE]->(req)
    MERGE (m)-[:MEMBERSHIP_OF]->(pack)
    MERGE (m)-[:INCLUDES]->(req)
    WITH m, req
    MATCH (p:Provision {provision_id: m.source_provision_id})
    MERGE (m)-[:BASED_ON]->(p)
    MERGE (req)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (pack:WorkProductRulePack)
    MATCH (c:LegalCorpus {corpus_id: pack.source_corpus_id})
    MERGE (pack)-[:DERIVED_FROM]->(c)
    WITH pack
    UNWIND coalesce(pack.work_product_types, []) AS workProductType
    MERGE (wpt:WorkProductType {work_product_type_id: workProductType})
    SET wpt.name = workProductType,
        wpt.updated_at = datetime(),
        wpt.created_at = coalesce(wpt.created_at, datetime())
    MERGE (pack)-[:APPLIES_TO]->(wpt)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (sn:Exception)
    MATCH (p:Provision {provision_id: sn.source_provision_id})
    MERGE (p)-[:EXPRESSES]->(sn)
    MERGE (sn)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (sn:Deadline)
    MATCH (p:Provision {provision_id: sn.source_provision_id})
    MERGE (p)-[:EXPRESSES]->(sn)
    MERGE (sn)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (sn:Penalty)
    MATCH (p:Provision {provision_id: sn.source_provision_id})
    MERGE (p)-[:EXPRESSES]->(sn)
    MERGE (sn)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (sn:Remedy)
    MATCH (p:Provision {provision_id: sn.source_provision_id})
    MERGE (p)-[:EXPRESSES]->(sn)
    MERGE (sn)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;
