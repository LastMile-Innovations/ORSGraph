UNWIND $rows AS row
MERGE (e:CorpusEdition {edition_id: row.edition_id})
SET e += row { .corpus_id, .edition_year, .effective_date, .source_label, .current }
SET e.id = row.edition_id,
    e.graph_kind = 'corpus',
    e.schema_version = '1.0.0',
    e.source_system = 'ors_crawler',
    e.updated_at = datetime()
SET e.created_at = coalesce(e.created_at, datetime())
WITH e, row
MATCH (c:LegalCorpus {corpus_id: row.corpus_id})
MERGE (c)-[:HAS_EDITION]->(e)
