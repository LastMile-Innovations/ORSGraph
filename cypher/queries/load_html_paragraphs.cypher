UNWIND $rows AS row
MERGE (hp:HtmlParagraph {paragraph_id: row.paragraph_id})
SET hp += row { .chapter, .edition_year, .order_index, .raw_html, .raw_text,
                 .cleaned_text, .normalized_text, .bold_text, .has_bold,
                 .has_underline, .has_italic, .align, .margin_left,
                 .text_indent, .style_raw, .style_hint, .class_hint,
                 .source_document_id }
SET hp.id = row.paragraph_id,
    hp.graph_kind = 'html_paragraph',
    hp.schema_version = '1.0.0',
    hp.source_system = 'ors_crawler',
    hp.updated_at = datetime()
SET hp.created_at = coalesce(hp.created_at, datetime())
WITH row, hp
OPTIONAL MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
    MERGE (sd)-[:HAS_PARAGRAPH]->(hp)
    MERGE (hp)-[:DERIVED_FROM]->(sd)
)
