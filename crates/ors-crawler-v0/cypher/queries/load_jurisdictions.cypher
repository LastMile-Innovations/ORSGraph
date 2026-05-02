// Load jurisdiction nodes into the graph (US and Oregon state)
// Creates the federal US jurisdiction and Oregon state jurisdiction with proper hierarchy

MERGE (us:Jurisdiction:FederalJurisdiction {jurisdiction_id: 'us'})
SET us += {
    id: 'us', graph_kind: 'jurisdiction', schema_version: '1.0.0',
    source_system: 'ors_crawler', name: 'United States', kind: 'country',
    country: 'US', updated_at: datetime()
}
SET us.created_at = coalesce(us.created_at, datetime())
MERGE (orState:Jurisdiction:StateJurisdiction {jurisdiction_id: 'or:state'})
SET orState += {
    id: 'or:state', graph_kind: 'jurisdiction', schema_version: '1.0.0',
    source_system: 'ors_crawler', name: 'Oregon', kind: 'state',
    country: 'US', parent_jurisdiction_id: 'us', updated_at: datetime()
}
SET orState.created_at = coalesce(orState.created_at, datetime())
MERGE (orState)-[:PART_OF]->(us)
