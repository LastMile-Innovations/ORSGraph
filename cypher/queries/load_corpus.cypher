// Load the legal corpus node (Oregon Revised Statutes)
// Creates the ORS corpus node and establishes relationships with jurisdictions and public bodies

MERGE (c:LegalCorpus {corpus_id: 'or:ors'})
SET c += {
    id: 'or:ors', graph_kind: 'corpus', schema_version: '1.0.0',
    source_system: 'ors_crawler', name: 'Oregon Revised Statutes',
    short_name: 'ORS', authority_family: 'ORS', jurisdiction_id: 'or:state',
    corpus_kind: 'statutes', updated_at: datetime()
}
SET c.created_at = coalesce(c.created_at, datetime())
WITH c
MATCH (j:Jurisdiction {jurisdiction_id: 'or:state'})
MATCH (pb:PublicBody {public_body_id: 'or:legislature'})
MERGE (j)-[:HAS_CORPUS]->(c)
MERGE (pb)-[:ISSUES]->(c)
