// Load public body nodes into the graph (Oregon Legislative Assembly)
// Creates the public body node and establishes relationships with jurisdictions

MERGE (pb:PublicBody {public_body_id: 'or:legislature'})
SET pb += {
    id: 'or:legislature', graph_kind: 'jurisdiction', schema_version: '1.0.0',
    source_system: 'ors_crawler', name: 'Oregon Legislative Assembly',
    kind: 'legislature', jurisdiction_id: 'or:state', updated_at: datetime()
}
SET pb.created_at = coalesce(pb.created_at, datetime())
WITH pb
MATCH (j:Jurisdiction {jurisdiction_id: 'or:state'})
MERGE (pb)-[:OPERATES_IN]->(j)
MERGE (j)-[:HAS_PUBLIC_BODY]->(pb)

WITH 1 AS _
MERGE (pb:PublicBody {public_body_id: 'or:judicial_department'})
SET pb += {
    id: 'or:judicial_department', graph_kind: 'jurisdiction', schema_version: '1.0.0',
    source_system: 'ors_crawler', name: 'Oregon Judicial Department',
    kind: 'judicial_department', jurisdiction_id: 'or:state', updated_at: datetime()
}
SET pb.created_at = coalesce(pb.created_at, datetime())
WITH pb
MATCH (j:Jurisdiction {jurisdiction_id: 'or:state'})
MERGE (pb)-[:OPERATES_IN]->(j)
MERGE (j)-[:HAS_PUBLIC_BODY]->(pb)
