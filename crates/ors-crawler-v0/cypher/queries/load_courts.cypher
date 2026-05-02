UNWIND $rows AS row
MERGE (c:Court {court_id: row.court_id})
SET c += row { .name, .court_type, .jurisdiction_id, .county_jurisdiction_id,
               .judicial_district_id, .judicial_district }
SET c.id = row.court_id,
    c.graph_kind = 'court',
    c.schema_version = '1.0.0',
    c.source_system = 'ors_crawler',
    c.updated_at = datetime()
SET c.created_at = coalesce(c.created_at, datetime())
FOREACH (_ IN CASE WHEN row.court_type = 'circuit_court' THEN [1] ELSE [] END | SET c:CircuitCourt)
WITH c, row
MATCH (j:Jurisdiction {jurisdiction_id: row.jurisdiction_id})
MERGE (c)-[:LOCATED_IN]->(j)
MERGE (j)-[:HAS_COURT]->(c)
WITH c, row
OPTIONAL MATCH (d:Jurisdiction {jurisdiction_id: row.judicial_district_id})
FOREACH (_ IN CASE WHEN d IS NULL THEN [] ELSE [1] END |
    MERGE (c)-[:PART_OF_JUDICIAL_DISTRICT]->(d)
)
