UNWIND $rows AS row
MERGE (fm:ChapterFrontMatter {front_matter_id: row.front_matter_id})
SET fm += row { .source_document_id, .chapter, .edition_year, .title_number,
                 .title_name, .chapter_number, .chapter_name, .text,
                 .source_paragraph_order, .front_matter_type, .confidence }
SET fm.id = row.front_matter_id,
    fm.graph_kind = 'chapter_front_matter',
    fm.schema_version = '1.0.0',
    fm.source_system = 'ors_crawler',
    fm.updated_at = datetime()
SET fm.created_at = coalesce(fm.created_at, datetime())
WITH row, fm
OPTIONAL MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
    MERGE (sd)-[:HAS_FRONT_MATTER]->(fm)
    MERGE (fm)-[:DERIVED_FROM]->(sd)
)
WITH row, fm
OPTIONAL MATCH (cv:ChapterVersion {chapter_id: 'or:ors:chapter:' + row.chapter + '@' + toString(row.edition_year)})
FOREACH (_ IN CASE WHEN cv IS NULL THEN [] ELSE [1] END |
    MERGE (cv)-[:HAS_FRONT_MATTER]->(fm)
)
WITH row, fm
OPTIONAL MATCH (hp:HtmlParagraph {source_document_id: row.source_document_id, order_index: row.source_paragraph_order})
FOREACH (_ IN CASE WHEN hp IS NULL THEN [] ELSE [1] END |
    MERGE (fm)-[:DERIVED_FROM]->(hp)
)
