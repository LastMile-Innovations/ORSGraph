UNWIND $rows AS row
MERGE (ti:TimeInterval {time_interval_id: row.time_interval_id})
SET ti += row { .start_date, .end_date, .label, .certainty }
SET ti.id = row.time_interval_id,
    ti.graph_kind = 'time_interval',
    ti.schema_version = '1.0.0',
    ti.source_system = 'ors_crawler',
    ti.updated_at = datetime()
SET ti.created_at = coalesce(ti.created_at, datetime())
