// Create relationships between legal text versions and their provisions
// Creates CONTAINS and PART_OF_VERSION edges

CALL {
    MATCH (p:Provision)
    MATCH (ltv:LegalTextVersion {version_id: p.version_id})
    MERGE (ltv)-[:CONTAINS]->(p)
    MERGE (p)-[:PART_OF_VERSION]->(ltv)
} IN TRANSACTIONS OF 1000 ROWS
