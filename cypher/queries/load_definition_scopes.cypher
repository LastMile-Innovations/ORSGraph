UNWIND $rows AS row
MERGE (ds:DefinitionScope {definition_scope_id: row.definition_scope_id})
SET ds += row { .scope_type, .scope_citation, .target_canonical_id, .target_chapter_id,
                .target_range_start, .target_range_end }
SET ds.id = row.definition_scope_id,
    ds.graph_kind = 'definition_scope',
    ds.schema_version = '1.0.0',
    ds.source_system = 'ors_crawler',
    ds.updated_at = datetime()
SET ds.created_at = coalesce(ds.created_at, datetime())
WITH row, ds
OPTIONAL MATCH (lti:LegalTextIdentity {canonical_id: row.target_canonical_id})
FOREACH (_ IN CASE WHEN lti IS NULL THEN [] ELSE [1] END |
    MERGE (ds)-[:APPLIES_TO]->(lti)
)
WITH row, ds
OPTIONAL MATCH (cv:ChapterVersion {chapter_id: row.target_chapter_id})
FOREACH (_ IN CASE WHEN cv IS NULL THEN [] ELSE [1] END |
    MERGE (ds)-[:APPLIES_TO]->(cv)
)
