// materialize_semantic_edges.cypher
// Materialize EXPRESSES and SUPPORTED_BY relationships for all semantic nodes

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
