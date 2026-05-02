UNWIND $rows AS row
MERGE (ft:FormText:LegalSemanticNode {form_text_id: row.form_text_id})
SET ft += row { .form_type, .text, .source_provision_id, .source_paragraph_ids,
                .confidence }
SET ft.semantic_id = row.form_text_id,
    ft.semantic_type = 'FormText',
    ft.id = row.form_text_id,
    ft.graph_kind = 'legal_semantic_node',
    ft.schema_version = '1.0.0',
    ft.source_system = 'ors_crawler',
    ft.updated_at = datetime()
SET ft.created_at = coalesce(ft.created_at, datetime())
WITH row, ft
OPTIONAL MATCH (p:Provision {provision_id: row.source_provision_id})
FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
    MERGE (p)-[:EXPRESSES]->(ft)
    MERGE (p)-[:HAS_FORM_TEXT]->(ft)
    MERGE (ft)-[:SUPPORTED_BY]->(p)
)
