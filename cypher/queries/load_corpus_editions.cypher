// Load a specific corpus edition (e.g., ORS 2025)
// Parameter: $edition_id, $edition_year

MERGE (e:CorpusEdition {edition_id: $edition_id})
SET e += {
    id: $edition_id, graph_kind: 'corpus', schema_version: '1.0.0',
    source_system: 'ors_crawler', corpus_id: 'or:ors',
    edition_year: $edition_year, current: true, updated_at: datetime()
}
SET e.created_at = coalesce(e.created_at, datetime())
WITH e
MATCH (c:LegalCorpus {corpus_id: 'or:ors'})
MERGE (c)-[:HAS_EDITION]->(e)
