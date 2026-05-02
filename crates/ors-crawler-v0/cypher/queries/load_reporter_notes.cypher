UNWIND $rows AS row
MERGE (rn:ReporterNote {reporter_note_id: row.reporter_note_id})
SET rn += row { .source_document_id, .canonical_id, .version_id, .source_provision_id,
              .citation, .text, .normalized_text, .source_page_start, .source_page_end,
              .confidence, .extraction_method }
SET rn.id = row.reporter_note_id,
    rn.graph_kind = 'source_note',
    rn.schema_version = '1.0.0',
    rn.source_system = 'ors_crawler',
    rn.updated_at = datetime()
SET rn.created_at = coalesce(rn.created_at, datetime())
