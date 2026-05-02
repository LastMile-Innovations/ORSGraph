UNWIND $rows AS row
MERGE (c:LegalCorpus {corpus_id: row.corpus_id})
SET c += row { .name, .short_name, .authority_family, .authority_type, .jurisdiction_id }
SET c.id = row.corpus_id,
    c.graph_kind = 'corpus',
    c.schema_version = '1.0.0',
    c.source_system = 'ors_crawler',
    c.updated_at = datetime()
SET c.created_at = coalesce(c.created_at, datetime())
WITH c, row
MATCH (j:Jurisdiction {jurisdiction_id: row.jurisdiction_id})
MERGE (j)-[:HAS_CORPUS]->(c)
