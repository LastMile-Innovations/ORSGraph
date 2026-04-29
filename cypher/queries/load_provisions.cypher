// Load provision nodes into the graph
// Parameter: $rows (array of provisions)

UNWIND $rows AS row
FILTER row.provision_id IS NOT NULL
CALL (row) {
    LET provisionId = row.provision_id
    MERGE (p:Provision:ORSProvision:LegalAuthorityUnit {provision_id: provisionId})
    SET p += row { .version_id, .canonical_id, .citation, .display_citation, .local_path,
                .provision_type, .text, .original_text, .normalized_text, .order_index, .depth, .text_hash,
                .paragraph_start_order, .paragraph_end_order, .source_paragraph_ids,
                .heading_path, .structural_context,
                .is_implied, .is_definition_candidate, .is_exception_candidate,
                .is_deadline_candidate, .is_penalty_candidate }
    SET p.id = provisionId,
        p.graph_kind = 'provision',
        p.schema_version = '1.0.0',
        p.source_system = 'ors_crawler',
        p.jurisdiction_id = 'or:state',
        p.authority_family = 'ORS',
        p.authority_level = 90,
        p.updated_at = datetime()
    SET p.created_at = coalesce(p.created_at, datetime())
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
