// Create relationships between chunks and their source provisions/versions.
// The Rust loader formats the transaction batch size at runtime.

CALL {
    MATCH (c:RetrievalChunk)
    FILTER c.chunk_type <> 'full_statute'
    MATCH (p:Provision)
    FILTER p.provision_id = coalesce(c.source_provision_id, c.source_id)
    MERGE (c)-[:DERIVED_FROM]->(p)
    MERGE (p)-[:HAS_CHUNK]->(c)
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS

CALL {
    MATCH (c:RetrievalChunk)
    FILTER c.chunk_type = 'full_statute'
    MATCH (ltv:LegalTextVersion)
    FILTER ltv.version_id = coalesce(c.source_version_id, c.parent_version_id, c.source_id)
    MERGE (c)-[:DERIVED_FROM]->(ltv)
    MERGE (ltv)-[:HAS_STATUTE_CHUNK]->(c)
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
