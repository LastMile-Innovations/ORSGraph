UNWIND $rows AS row
MERGE (la:LegalAction {action_id: row.action_id})
SET la += row { .verb, .object, .normalized_action }
SET la.id = row.action_id,
    la.graph_kind = 'legal_action',
    la.schema_version = '1.0.0',
    la.source_system = 'ors_crawler',
    la.updated_at = datetime()
SET la.created_at = coalesce(la.created_at, datetime())
