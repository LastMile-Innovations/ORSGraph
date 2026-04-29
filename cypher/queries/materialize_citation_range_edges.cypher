// Create range citation edges
// Links citation mentions to range start and end identities

CALL {
    MATCH (cm:CitationMention)
    WHERE cm.target_start_canonical_id IS NOT NULL
    MATCH (start:LegalTextIdentity {canonical_id: cm.target_start_canonical_id})
    MERGE (cm)-[:RESOLVES_TO_RANGE_START]->(start)
    WITH cm
    MATCH (end:LegalTextIdentity {canonical_id: cm.target_end_canonical_id})
    MERGE (cm)-[:RESOLVES_TO_RANGE_END]->(end)
} IN TRANSACTIONS OF 1000 ROWS
