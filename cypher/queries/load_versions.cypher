// Load legal text version nodes into the graph
// Parameter: $rows (array of legal text versions)

UNWIND $rows AS row
FILTER row.version_id IS NOT NULL
CALL (row) {
    LET versionId = row.version_id
    MERGE (ltv:LegalTextVersion:LegalAuthority {version_id: versionId})
    SET ltv += row { .canonical_id, .citation, .title, .chapter, .edition_year, .status,
                 .corpus_id, .edition_id, .authority_family, .authority_type, .authority_level,
                 .effective_date, .source_page_start, .source_page_end,
                 .status_text, .text, .original_text, .text_hash, .paragraph_start_order,
                 .paragraph_end_order, .source_paragraph_ids, .source_document_id,
                 .official_status, .disclaimer_required }
    SET ltv.id = versionId,
        ltv.graph_kind = 'authority',
        ltv.schema_version = '1.0.0',
        ltv.source_system = 'ors_crawler',
        ltv.jurisdiction_id = CASE
            WHEN coalesce(row.authority_family, 'ORS') = 'SLR' THEN coalesce(split(row.corpus_id, ':slr')[0], 'or:linn')
            ELSE 'or:state'
        END,
        ltv.authority_family = coalesce(row.authority_family, ltv.authority_family, 'ORS'),
        ltv.authority_level = coalesce(row.authority_level, ltv.authority_level, 90),
        ltv.current = true,
        ltv.parser_profile = CASE
            WHEN coalesce(row.authority_family, 'ORS') = 'UTCR' THEN 'utcr_pdf_parser_v1'
            WHEN coalesce(row.authority_family, 'ORS') = 'SLR' THEN 'local_rule_pdf_parser_v1'
            ELSE 'ors_dom_parser_v1'
        END,
        ltv.parser_confidence = CASE WHEN coalesce(row.authority_family, 'ORS') IN ['UTCR', 'SLR'] THEN 0.90 ELSE 0.98 END,
        ltv.updated_at = datetime()
    SET ltv.created_at = coalesce(ltv.created_at, datetime())
    FOREACH (_ IN CASE WHEN row.authority_family IN ['UTCR', 'SLR'] THEN [1] ELSE [] END | SET ltv:CourtRule)
    FOREACH (_ IN CASE WHEN row.authority_family = 'UTCR' THEN [1] ELSE [] END | SET ltv:UTCRRuleVersion)
    FOREACH (_ IN CASE WHEN row.authority_family = 'SLR' THEN [1] ELSE [] END | SET ltv:SLRRuleVersion:SupplementaryLocalRule)
    FOREACH (_ IN CASE WHEN coalesce(row.authority_family, 'ORS') = 'ORS' THEN [1] ELSE [] END | SET ltv:ORSSectionVersion)
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
