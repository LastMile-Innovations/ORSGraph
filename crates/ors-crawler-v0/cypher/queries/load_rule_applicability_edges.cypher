UNWIND $rows AS row
MATCH (doc:RuleAuthorityDocument {authority_document_id: row.authority_document_id})
MATCH (j:Jurisdiction {jurisdiction_id: row.jurisdiction_id})
MERGE (doc)-[applies:APPLIES_TO]->(j)
SET applies.edge_id = row.edge_id,
    applies.relationship_type = row.relationship_type,
    applies.updated_at = datetime()
WITH doc, row
OPTIONAL MATCH (court:Court {court_id: row.court_id})
FOREACH (_ IN CASE WHEN court IS NULL THEN [] ELSE [1] END |
    MERGE (doc)-[governs:GOVERNS_COURT]->(court)
    SET governs.edge_id = row.edge_id + ':court',
        governs.updated_at = datetime()
)
