// Load citation mention nodes into the graph
// Parameter: $rows (array of citation mentions)

UNWIND $rows AS row
FILTER row.citation_mention_id IS NOT NULL
CALL (row) {
    LET cmId = row.citation_mention_id
    MERGE (cm:CitationMention {citation_mention_id: cmId})
    SET cm += row { .source_provision_id, .raw_text, .normalized_citation, .citation_type,
                 .target_canonical_id, .target_start_canonical_id, .target_end_canonical_id,
                 .target_provision_id, .unresolved_subpath, .external_citation_id,
                 .resolver_status, .confidence, .qc_severity }
    SET cm.id = cmId,
        cm.graph_kind = 'citation',
        cm.schema_version = '1.0.0',
        cm.source_system = 'ors_crawler',
        cm.updated_at = datetime()
    SET cm.created_at = coalesce(cm.created_at, datetime())
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
