// materialize_specialized_edges.cypher
// Materialize relationships for specialized extracts (Money, Tax, Form, Notice)

CALL {
    MATCH (ft:FormText)
    MATCH (p:Provision {provision_id: ft.source_provision_id})
    MERGE (p)-[:HAS_FORM_TEXT]->(ft)
    MERGE (ft)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (rn:RequiredNotice)
    MATCH (p:Provision {provision_id: rn.source_provision_id})
    MERGE (p)-[:REQUIRES_NOTICE]->(rn)
    MERGE (rn)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (ma:MoneyAmount)
    MATCH (p:Provision {provision_id: ma.source_provision_id})
    MERGE (p)-[:EXPRESSES]->(ma)
    MERGE (ma)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (tr:TaxRule)
    MATCH (p:Provision {provision_id: tr.source_provision_id})
    MERGE (p)-[:EXPRESSES]->(tr)
    MERGE (tr)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (rl:RateLimit)
    MATCH (p:Provision {provision_id: rl.source_provision_id})
    MERGE (p)-[:EXPRESSES]->(rl)
    MERGE (rl)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

// Connect TaxRules to associated Money/Rate if fields match (often they are extracted together)
// This is more heuristic but useful if the IDs are aligned or cross-linked in the parser
CALL {
    MATCH (tr:TaxRule)
    MATCH (p:Provision {provision_id: tr.source_provision_id})
    MATCH (p)-[:EXPRESSES]->(ma:MoneyAmount)
    MERGE (tr)-[:HAS_AMOUNT]->(ma)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (tr:TaxRule)
    MATCH (p:Provision {provision_id: tr.source_provision_id})
    MATCH (p)-[:EXPRESSES]->(rl:RateLimit)
    MERGE (tr)-[:HAS_RATE_LIMIT]->(rl)
} IN TRANSACTIONS OF 5000 ROWS;
