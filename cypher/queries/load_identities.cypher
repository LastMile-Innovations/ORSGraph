// Load legal text identity nodes into the graph
// Parameter: $rows (array of legal text identities)

UNWIND $rows AS row
MERGE (lti:LegalTextIdentity:ORSSectionIdentity:Statute {canonical_id: row.canonical_id})
SET lti += row { .citation, .jurisdiction_id, .authority_family, .title, .chapter, .status }
SET lti.id = row.canonical_id,
    lti.graph_kind = 'authority',
    lti.schema_version = '1.0.0',
    lti.source_system = 'ors_crawler',
    lti.authority_level = 90,
    lti.official_status = 'official_online_not_official_print',
    lti.disclaimer_required = true,
    lti.parser_profile = 'ors_dom_parser_v1',
    lti.parser_confidence = 0.98,
    lti.updated_at = datetime()
SET lti.created_at = coalesce(lti.created_at, datetime())
