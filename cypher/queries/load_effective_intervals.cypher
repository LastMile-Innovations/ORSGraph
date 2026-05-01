UNWIND $rows AS row
MERGE (interval:EffectiveInterval {effective_interval_id: row.effective_interval_id})
SET interval += row { .authority_document_id, .start_date, .end_date, .label, .certainty }
SET interval.id = row.effective_interval_id,
    interval.graph_kind = 'effective_interval',
    interval.schema_version = '1.0.0',
    interval.source_system = 'ors_crawler',
    interval.updated_at = datetime()
SET interval.created_at = coalesce(interval.created_at, datetime())
