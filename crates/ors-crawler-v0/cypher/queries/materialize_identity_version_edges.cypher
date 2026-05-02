// Create relationships between legal text identities and their versions
// Creates HAS_VERSION and VERSION_OF edges

CALL {
    MATCH (ltv:LegalTextVersion)
    MATCH (lti:LegalTextIdentity {canonical_id: ltv.canonical_id})
    MERGE (lti)-[:HAS_VERSION]->(ltv)
    MERGE (ltv)-[:VERSION_OF]->(lti)
} IN TRANSACTIONS OF 1000 ROWS
