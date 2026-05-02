UNWIND $rows AS row
MATCH (from:RuleAuthorityDocument {authority_document_id: row.from_authority_document_id})
MATCH (to:RuleAuthorityDocument {authority_document_id: row.to_authority_document_id})
MERGE (from)-[r:SUPERSEDES {edge_id: row.edge_id}]->(to)
SET r.relationship_type = row.relationship_type,
    r.reason = row.reason,
    r.confidence = row.confidence,
    r.updated_at = datetime()
