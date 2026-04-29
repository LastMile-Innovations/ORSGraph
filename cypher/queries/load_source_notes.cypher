UNWIND $rows AS row
FILTER row.source_note_id IS NOT NULL
CALL (row) {
    LET noteId = row.source_note_id
    MERGE (sn:SourceNote {source_note_id: noteId})
    SET sn += row { .note_type, .text, .source_document_id, .canonical_id, .version_id,
                    .provision_id, .citation, .paragraph_start_order, .paragraph_end_order,
                    .source_paragraph_order, .source_paragraph_ids, .normalized_text,
                    .confidence, .extraction_method }
    SET sn.id = noteId,
        sn.graph_kind = 'source_note',
        sn.schema_version = '1.0.0',
        sn.source_system = 'ors_crawler',
        sn.updated_at = datetime()
    SET sn.created_at = coalesce(sn.created_at, datetime())
    WITH row, sn
    OPTIONAL MATCH (ltv:LegalTextVersion {version_id: row.version_id})
    FOREACH (_ IN CASE WHEN ltv IS NULL THEN [] ELSE [1] END |
        MERGE (ltv)-[:HAS_SOURCE_NOTE]->(sn)
        MERGE (sn)-[:ANNOTATES]->(ltv)
    )
    WITH row, sn
    OPTIONAL MATCH (p:Provision {provision_id: row.provision_id})
    FOREACH (_ IN CASE WHEN p IS NULL THEN [] ELSE [1] END |
        MERGE (p)-[:HAS_SOURCE_NOTE]->(sn)
        MERGE (sn)-[:ANNOTATES]->(p)
    )
    WITH row, sn
    OPTIONAL MATCH (sd:SourceDocument {source_document_id: row.source_document_id})
    FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
        MERGE (sd)-[:SOURCE_FOR]->(sn)
        MERGE (sn)-[:DERIVED_FROM]->(sd)
    )
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
