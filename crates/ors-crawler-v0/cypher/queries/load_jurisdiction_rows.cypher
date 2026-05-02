UNWIND $rows AS row
MERGE (j:Jurisdiction {jurisdiction_id: row.jurisdiction_id})
SET j += row { .name, .jurisdiction_type, .parent_jurisdiction_id, .country }
SET j.id = row.jurisdiction_id,
    j.kind = row.jurisdiction_type,
    j.graph_kind = 'jurisdiction',
    j.schema_version = '1.0.0',
    j.source_system = 'ors_crawler',
    j.updated_at = datetime()
SET j.created_at = coalesce(j.created_at, datetime())
FOREACH (_ IN CASE WHEN row.jurisdiction_type = 'state' THEN [1] ELSE [] END | SET j:StateJurisdiction)
FOREACH (_ IN CASE WHEN row.jurisdiction_type = 'county' THEN [1] ELSE [] END | SET j:CountyJurisdiction)
FOREACH (_ IN CASE WHEN row.jurisdiction_type = 'judicial_district' THEN [1] ELSE [] END | SET j:JudicialDistrict)
WITH j, row
OPTIONAL MATCH (parent:Jurisdiction {jurisdiction_id: row.parent_jurisdiction_id})
FOREACH (_ IN CASE WHEN parent IS NULL THEN [] ELSE [1] END |
    MERGE (j)-[:PART_OF]->(parent)
)
