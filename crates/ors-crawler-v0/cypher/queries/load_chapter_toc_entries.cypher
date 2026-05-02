UNWIND $rows AS row
MERGE (toc:ChapterTocEntry {toc_entry_id: row.toc_entry_id})
SET toc += row { .source_document_id, .chapter, .edition_year, .citation, .canonical_id,
                 .caption, .heading_path, .toc_order, .source_paragraph_order, .confidence }
SET toc.id = row.toc_entry_id,
    toc.graph_kind = 'chapter_toc_entry',
    toc.schema_version = '1.0.0',
    toc.source_system = 'ors_crawler',
    toc.updated_at = datetime()
SET toc.created_at = coalesce(toc.created_at, datetime())
WITH row, toc
OPTIONAL MATCH (cv:ChapterVersion {chapter_id: 'or:ors:chapter:' + row.chapter + '@' + toString(row.edition_year)})
FOREACH (_ IN CASE WHEN cv IS NULL THEN [] ELSE [1] END |
    MERGE (cv)-[:HAS_TOC_ENTRY]->(toc)
)
WITH row, toc
OPTIONAL MATCH (lti:LegalTextIdentity {canonical_id: row.canonical_id})
FOREACH (_ IN CASE WHEN lti IS NULL THEN [] ELSE [1] END |
    MERGE (toc)-[:POINTS_TO]->(lti)
)
WITH row, toc
OPTIONAL MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
    MERGE (toc)-[:DERIVED_FROM]->(sd)
)
