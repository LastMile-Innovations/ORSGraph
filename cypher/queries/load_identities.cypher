// Load legal text identity nodes into the graph
// Parameter: $rows (array of legal text identities)

UNWIND $rows AS row
MERGE (lti:LegalTextIdentity {canonical_id: row.canonical_id})
SET lti += row { .citation, .jurisdiction_id, .authority_family, .corpus_id,
               .authority_type, .authority_level, .effective_date, .title, .chapter, .status }
SET lti.id = row.canonical_id,
    lti.graph_kind = 'authority',
    lti.schema_version = '1.0.0',
    lti.source_system = 'ors_crawler',
    lti.authority_level = coalesce(row.authority_level, lti.authority_level, 90),
    lti.official_status = coalesce(lti.official_status, CASE WHEN row.authority_family IN ['UTCR', 'SLR'] THEN 'official_pdf' ELSE 'official_online_not_official_print' END),
    lti.disclaimer_required = coalesce(lti.disclaimer_required, NOT (row.authority_family IN ['UTCR', 'SLR'])),
    lti.parser_profile = coalesce(lti.parser_profile, CASE WHEN row.authority_family = 'UTCR' THEN 'utcr_pdf_parser_v1' WHEN row.authority_family = 'SLR' THEN 'local_rule_pdf_parser_v1' ELSE 'ors_dom_parser_v1' END),
    lti.parser_confidence = coalesce(lti.parser_confidence, CASE WHEN row.authority_family IN ['UTCR', 'SLR'] THEN 0.90 ELSE 0.98 END),
    lti.updated_at = datetime()
SET lti.created_at = coalesce(lti.created_at, datetime())
FOREACH (_ IN CASE WHEN row.authority_family IN ['UTCR', 'SLR'] THEN [1] ELSE [] END | SET lti:CourtRule)
FOREACH (_ IN CASE WHEN row.authority_family = 'UTCR' THEN [1] ELSE [] END | SET lti:UTCRRule)
FOREACH (_ IN CASE WHEN row.authority_family = 'SLR' THEN [1] ELSE [] END | SET lti:SLRRule:SupplementaryLocalRule)
FOREACH (_ IN CASE WHEN row.authority_family = 'ORS' THEN [1] ELSE [] END | SET lti:ORSSectionIdentity:Statute)
