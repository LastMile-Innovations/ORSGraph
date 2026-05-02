UNWIND $rows AS row
MERGE (rr:ReservedRange {reserved_range_id: row.reserved_range_id})
SET rr += row { .source_document_id, .chapter, .edition_year, .range_text, .start_chapter,
                .end_chapter, .start_title, .end_title, .source_paragraph_order, .confidence }
SET rr.id = row.reserved_range_id,
    rr.graph_kind = 'reserved_range',
    rr.schema_version = '1.0.0',
    rr.source_system = 'ors_crawler',
    rr.updated_at = datetime()
SET rr.created_at = coalesce(rr.created_at, datetime())
WITH row, rr
OPTIONAL MATCH (cv:ChapterVersion {chapter_id: 'or:ors:chapter:' + row.chapter + '@' + toString(row.edition_year)})
FOREACH (_ IN CASE WHEN cv IS NULL THEN [] ELSE [1] END |
    MERGE (cv)-[:HAS_RESERVED_RANGE]->(rr)
)
WITH row, rr
OPTIONAL MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
    MERGE (rr)-[:DERIVED_FROM]->(sd)
)
