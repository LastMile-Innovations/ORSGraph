// Load provision nodes into the graph
// Parameter: $rows (array of provisions)

UNWIND $rows AS row
FILTER row.provision_id IS NOT NULL
CALL (row) {
    LET provisionId = row.provision_id
    MERGE (p:Provision:LegalAuthorityUnit {provision_id: provisionId})
    SET p += row { .version_id, .canonical_id, .citation, .display_citation, .chapter,
                .corpus_id, .edition_id, .authority_family, .authority_type, .authority_level,
                .effective_date, .source_page_start, .source_page_end, .local_path,
                .provision_type, .text, .original_text, .normalized_text, .order_index, .depth, .text_hash,
                .paragraph_start_order, .paragraph_end_order, .source_paragraph_ids,
                .heading_path, .structural_context,
                .is_implied, .is_definition_candidate, .is_exception_candidate,
                .is_deadline_candidate, .is_penalty_candidate }
    SET p.id = provisionId,
        p.graph_kind = 'provision',
        p.schema_version = '1.0.0',
        p.source_system = 'ors_crawler',
        p.jurisdiction_id = CASE
            WHEN coalesce(row.authority_family, 'ORS') IN ['USCONST', 'CONAN'] THEN 'us'
            WHEN coalesce(row.authority_family, 'ORS') = 'SLR' THEN coalesce(split(row.corpus_id, ':slr')[0], 'or:linn')
            ELSE 'or:state'
        END,
        p.authority_family = coalesce(row.authority_family, p.authority_family, 'ORS'),
        p.authority_level = coalesce(row.authority_level, p.authority_level, 90),
        p.updated_at = datetime()
    SET p.created_at = coalesce(p.created_at, datetime())
    FOREACH (_ IN CASE WHEN row.authority_family IN ['UTCR', 'SLR'] THEN [1] ELSE [] END | SET p:CourtRuleProvision)
    FOREACH (_ IN CASE WHEN row.authority_family = 'UTCR' THEN [1] ELSE [] END | SET p:UTCRProvision)
    FOREACH (_ IN CASE WHEN row.authority_family = 'SLR' THEN [1] ELSE [] END | SET p:SLRProvision:SupplementaryLocalRuleProvision)
    FOREACH (_ IN CASE WHEN coalesce(row.authority_family, 'ORS') = 'ORS' THEN [1] ELSE [] END | SET p:ORSProvision)
    FOREACH (_ IN CASE WHEN row.authority_family = 'USCONST' THEN [1] ELSE [] END | SET p:USConstitutionProvision:ConstitutionProvision:PrimaryLaw)
    FOREACH (_ IN CASE WHEN row.authority_family = 'CONAN' THEN [1] ELSE [] END | SET p:ConstitutionAnnotatedProvision:OfficialCommentary)
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
