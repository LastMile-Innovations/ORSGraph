UNWIND $rows AS row
MERGE (dt:DefinedTerm {defined_term_id: row.defined_term_id})
SET dt += row { .term, .normalized_term, .jurisdiction_id, .authority_family }
SET dt.id = row.defined_term_id,
    dt.graph_kind = 'defined_term',
    dt.schema_version = '1.0.0',
    dt.source_system = 'ors_crawler',
    dt.updated_at = datetime()
SET dt.created_at = coalesce(dt.created_at, datetime())
