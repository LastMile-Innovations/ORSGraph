// Load legal text version nodes into the graph
// Parameter: $rows (array of legal text versions)

UNWIND $rows AS row
FILTER row.version_id IS NOT NULL
CALL (row) {
    LET versionId = row.version_id
    MERGE (ltv:LegalTextVersion:ORSSectionVersion:LegalAuthority {version_id: versionId})
    SET ltv += row { .canonical_id, .citation, .title, .chapter, .edition_year, .status,
                 .status_text, .text, .original_text, .text_hash, .paragraph_start_order,
                 .paragraph_end_order, .source_paragraph_ids, .source_document_id,
                 .official_status, .disclaimer_required }
    SET ltv.id = versionId,
        ltv.graph_kind = 'authority',
        ltv.schema_version = '1.0.0',
        ltv.source_system = 'ors_crawler',
        ltv.jurisdiction_id = 'or:state',
        ltv.authority_family = 'ORS',
        ltv.authority_level = 90,
        ltv.current = true,
        ltv.parser_profile = 'ors_dom_parser_v1',
        ltv.parser_confidence = 0.98,
        ltv.updated_at = datetime()
    SET ltv.created_at = coalesce(ltv.created_at, datetime())
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
