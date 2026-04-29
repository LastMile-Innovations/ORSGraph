// Load chapter heading nodes into the graph
// Parameter: $rows (array of chapter headings)

UNWIND $rows AS row
MERGE (h:ChapterHeading {heading_id: row.heading_id})
SET h += row { .chapter, .text, .order_index }
SET h.id = row.heading_id,
    h.graph_kind = 'authority',
    h.schema_version = '1.0.0',
    h.source_system = 'ors_crawler',
    h.jurisdiction_id = 'or:state',
    h.authority_family = 'ORS',
    h.updated_at = datetime()
SET h.created_at = coalesce(h.created_at, datetime())
