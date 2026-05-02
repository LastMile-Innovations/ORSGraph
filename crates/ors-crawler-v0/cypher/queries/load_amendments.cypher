UNWIND $rows AS row
MERGE (a:Amendment {amendment_id: row.amendment_id})
SET a += row { .amendment_type, .session_law_citation, .effective_date, .text,
               .raw_text, .source_document_id, .confidence, .canonical_id, .version_id,
               .session_law_id, .affected_canonical_id, .affected_version_id, .source_note_id,
               .proposal_method, .proposal_id, .measure_number, .resolution_chamber,
               .resolution_number, .filed_date, .proposed_year, .adopted_date,
               .election_date, .resolution_status }
SET a.id = row.amendment_id,
    a.graph_kind = 'amendment',
    a.schema_version = '1.0.0',
    a.source_system = 'ors_crawler',
    a.updated_at = datetime()
SET a.created_at = coalesce(a.created_at, datetime())
WITH row, a
OPTIONAL MATCH (lti:LegalTextIdentity {canonical_id: row.canonical_id})
FOREACH (_ IN CASE WHEN lti IS NULL THEN [] ELSE [1] END |
    MERGE (a)-[:AFFECTS]->(lti)
)
WITH row, a
OPTIONAL MATCH (ltv:LegalTextVersion {version_id: row.version_id})
FOREACH (_ IN CASE WHEN ltv IS NULL THEN [] ELSE [1] END |
    MERGE (a)-[:AFFECTS_VERSION]->(ltv)
)
WITH row, a
OPTIONAL MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
    MERGE (a)-[:SUPPORTED_BY]->(sd)
)
WITH row, a
OPTIONAL MATCH (sl:SessionLaw {session_law_id: row.session_law_id})
FOREACH (_ IN CASE WHEN sl IS NULL THEN [] ELSE [1] END |
    MERGE (sl)-[:ENACTS]->(a)
)
WITH row, a
OPTIONAL MATCH (sn:SourceNote {source_note_id: row.source_note_id})
FOREACH (_ IN CASE WHEN sn IS NULL THEN [] ELSE [1] END |
    MERGE (a)-[:SUPPORTED_BY]->(sn)
)
