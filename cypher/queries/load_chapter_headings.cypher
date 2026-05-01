// Load chapter heading nodes into the graph
// Parameter: $rows (array of chapter headings)

UNWIND $rows AS row
WITH row,
     CASE
        WHEN row.heading_id CONTAINS ':slr@' THEN 'SLR'
        WHEN row.heading_id STARTS WITH 'or:utcr:' THEN 'UTCR'
        ELSE 'ORS'
     END AS authority_family
MERGE (h:ChapterHeading {heading_id: row.heading_id})
SET h += row { .chapter, .text, .order_index }
SET h.id = row.heading_id,
    h.graph_kind = 'authority',
    h.schema_version = '1.0.0',
    h.source_system = 'ors_crawler',
    h.jurisdiction_id = CASE
        WHEN authority_family = 'SLR' THEN split(row.heading_id, ':slr@')[0]
        ELSE 'or:state'
    END,
    h.authority_family = authority_family,
    h.updated_at = datetime()
SET h.created_at = coalesce(h.created_at, datetime())
FOREACH (_ IN CASE WHEN authority_family IN ['UTCR', 'SLR'] THEN [1] ELSE [] END | SET h:CourtRuleHeading)
FOREACH (_ IN CASE WHEN authority_family = 'SLR' THEN [1] ELSE [] END | SET h:SLRHeading)
