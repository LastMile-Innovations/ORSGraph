UNWIND $rows AS row
MERGE (pd:ParserDiagnostic {parser_diagnostic_id: row.parser_diagnostic_id})
SET pd += row { .source_document_id, .chapter, .edition_year, .severity, .diagnostic_type,
                .message, .source_paragraph_order, .related_id, .parser_profile }
SET pd.id = row.parser_diagnostic_id,
    pd.graph_kind = 'parser_diagnostic',
    pd.schema_version = '1.0.0',
    pd.source_system = 'ors_crawler',
    pd.updated_at = datetime()
SET pd.created_at = coalesce(pd.created_at, datetime())
WITH row, pd
OPTIONAL MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
    MERGE (pd)-[:DERIVED_FROM]->(sd)
    MERGE (pd)-[:WARNED_ON]->(sd)
)
WITH row, pd
OPTIONAL MATCH (hp:HtmlParagraph {source_document_id: row.source_document_id, order_index: row.source_paragraph_order})
FOREACH (_ IN CASE WHEN hp IS NULL THEN [] ELSE [1] END |
    MERGE (pd)-[:WARNED_ON]->(hp)
)
