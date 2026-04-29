UNWIND $rows AS row
MERGE (tce:TitleChapterEntry {title_chapter_entry_id: row.title_chapter_entry_id})
SET tce += row { .source_document_id, .chapter, .edition_year, .title_number,
                  .title_name, .chapter_number, .chapter_name, .chapter_list_order,
                  .source_paragraph_order, .confidence }
SET tce.id = row.title_chapter_entry_id,
    tce.graph_kind = 'title_chapter_entry',
    tce.schema_version = '1.0.0',
    tce.source_system = 'ors_crawler',
    tce.updated_at = datetime()
SET tce.created_at = coalesce(tce.created_at, datetime())
WITH row, tce
OPTIONAL MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
    MERGE (sd)-[:HAS_TITLE_CHAPTER_ENTRY]->(tce)
    MERGE (tce)-[:DERIVED_FROM]->(sd)
)
WITH row, tce
OPTIONAL MATCH (cv:ChapterVersion {chapter_id: 'or:ors:chapter:' + row.chapter_number + '@' + toString(row.edition_year)})
FOREACH (_ IN CASE WHEN cv IS NULL THEN [] ELSE [1] END |
    MERGE (tce)-[:POINTS_TO_CHAPTER]->(cv)
)
WITH row, tce
OPTIONAL MATCH (hp:HtmlParagraph {source_document_id: row.source_document_id, order_index: row.source_paragraph_order})
FOREACH (_ IN CASE WHEN hp IS NULL THEN [] ELSE [1] END |
    MERGE (tce)-[:DERIVED_FROM]->(hp)
)
