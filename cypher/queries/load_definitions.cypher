UNWIND $rows AS row
MERGE (d:Definition {definition_id: row.definition_id})
SET d += row { .term, .normalized_term, .definition_text, .scope_type, .scope_citation,
               .source_provision_id, .confidence, .review_status, .extraction_method,
               .defined_term_id, .definition_scope_id }
SET d.id = row.definition_id,
    d.graph_kind = 'definition',
    d.schema_version = '1.0.0',
    d.source_system = 'ors_crawler',
    d.updated_at = datetime()
SET d.created_at = coalesce(d.created_at, datetime())
WITH row, d
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:DEFINES]->(d)
    MERGE (d)-[:SUPPORTED_BY]->(p)
)
WITH row, d
OPTIONAL MATCH (dt:DefinedTerm {defined_term_id: row.defined_term_id})
FOREACH (_ IN CASE WHEN dt IS NULL THEN [] ELSE [1] END |
    MERGE (d)-[:DEFINES_TERM]->(dt)
)
WITH row, d
OPTIONAL MATCH (ds:DefinitionScope {definition_scope_id: row.definition_scope_id})
FOREACH (_ IN CASE WHEN ds IS NULL THEN [] ELSE [1] END |
    MERGE (d)-[:HAS_SCOPE]->(ds)
)
