// materialize_history_edges.cypher
// Materialize legislative history and temporal currentness relationships

CALL {
    MATCH (sn:SourceNote)
    MATCH (ltv:LegalTextVersion {version_id: sn.version_id})
    MERGE (ltv)-[:HAS_SOURCE_NOTE]->(sn)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (sn:SourceNote)
    MATCH (p:Provision {provision_id: sn.provision_id})
    MERGE (p)-[:HAS_SOURCE_NOTE]->(sn)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (sl:SessionLaw)
    WHERE sl.source_note_id IS NOT NULL
    MATCH (sn:SourceNote {source_note_id: sl.source_note_id})
    MERGE (sn)-[:MENTIONS_SESSION_LAW]->(sl)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (am:Amendment)
    MATCH (sl:SessionLaw {session_law_id: am.session_law_id})
    MERGE (sl)-[:ENACTS]->(am)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (am:Amendment)
    WHERE am.affected_canonical_id IS NOT NULL
    MATCH (lti:LegalTextIdentity {canonical_id: am.affected_canonical_id})
    MERGE (am)-[:AFFECTS]->(lti)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (am:Amendment)
    WHERE am.affected_version_id IS NOT NULL
    MATCH (ltv:LegalTextVersion {version_id: am.affected_version_id})
    MERGE (am)-[:AFFECTS_VERSION]->(ltv)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (se:StatusEvent)
    MATCH (lti:LegalTextIdentity {canonical_id: se.canonical_id})
    MERGE (lti)-[:HAS_STATUS_EVENT]->(se)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (se:StatusEvent)
    WHERE se.version_id IS NOT NULL
    MATCH (ltv:LegalTextVersion {version_id: se.version_id})
    MERGE (ltv)-[:HAS_STATUS_EVENT]->(se)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (te:TemporalEffect)
    WHERE te.version_id IS NOT NULL
    MATCH (ltv:LegalTextVersion {version_id: te.version_id})
    MERGE (ltv)-[:HAS_TEMPORAL_EFFECT]->(te)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (te:TemporalEffect)
    WHERE te.source_note_id IS NOT NULL
    MATCH (sn:SourceNote {source_note_id: te.source_note_id})
    MERGE (te)-[:SUPPORTED_BY]->(sn)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (le:LineageEvent)
    MATCH (lti:LegalTextIdentity {canonical_id: le.current_canonical_id})
    MERGE (lti)-[:HAS_LINEAGE_EVENT]->(le)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (le:LineageEvent {lineage_type: 'renumbered_from'})
    WHERE le.from_canonical_id IS NOT NULL
    MATCH (old:LegalTextIdentity {canonical_id: le.from_canonical_id})
    MATCH (new:LegalTextIdentity {canonical_id: le.current_canonical_id})
    MERGE (old)-[:RENUMBERED_TO]->(new)
    MERGE (new)-[:FORMERLY]->(old)
} IN TRANSACTIONS OF 5000 ROWS;
