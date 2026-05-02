UNWIND $rows AS row
MERGE (entry:RulePublicationEntry {publication_entry_id: row.publication_entry_id})
SET entry += row { .registry_source_id, .registry_snapshot_id, .authority_document_id,
                   .effective_interval_id, .title, .jurisdiction, .jurisdiction_id,
                   .subcategory, .authority_kind, .publication_bucket, .table_section,
                   .row_index, .effective_start_date, .effective_end_date, .date_status,
                   .status_flags, .authority_identifier }
SET entry.id = row.publication_entry_id,
    entry.graph_kind = 'court_rules_registry',
    entry.schema_version = '1.0.0',
    entry.source_system = 'ors_crawler',
    entry.updated_at = datetime()
SET entry.created_at = coalesce(entry.created_at, datetime())
