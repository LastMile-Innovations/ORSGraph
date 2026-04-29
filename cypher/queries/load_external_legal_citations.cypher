UNWIND $rows AS row
MERGE (elc:ExternalLegalCitation {external_citation_id: row.external_citation_id})
SET elc += row { .citation, .normalized_citation, .citation_type, .jurisdiction_id, .source_system }
SET elc.id = row.external_citation_id,
    elc.graph_kind = 'citation',
    elc.schema_version = '1.0.0',
    elc.updated_at = datetime()
SET elc.created_at = coalesce(elc.created_at, datetime())
