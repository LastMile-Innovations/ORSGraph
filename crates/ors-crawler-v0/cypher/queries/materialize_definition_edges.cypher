// materialize_definition_edges.cypher
// Materialize DEFINES, DEFINES_TERM, HAS_SCOPE, and APPLIES_TO relationships

CALL {
    MATCH (d:Definition)
    MATCH (p:Provision {provision_id: d.source_provision_id})
    MERGE (p)-[:DEFINES]->(d)
    MERGE (d)-[:SUPPORTED_BY]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (d:Definition)
    WHERE d.defined_term_id IS NOT NULL
    MATCH (dt:DefinedTerm {defined_term_id: d.defined_term_id})
    MERGE (d)-[:DEFINES_TERM]->(dt)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (d:Definition)
    WHERE d.definition_scope_id IS NOT NULL
    MATCH (ds:DefinitionScope {definition_scope_id: d.definition_scope_id})
    MERGE (d)-[:HAS_SCOPE]->(ds)
} IN TRANSACTIONS OF 5000 ROWS;

// DefinitionScope -> Target Identity (Specific Section)
CALL {
    MATCH (ds:DefinitionScope {scope_type: 'section'})
    WHERE ds.target_canonical_id IS NOT NULL
    MATCH (lti:LegalTextIdentity {canonical_id: ds.target_canonical_id})
    MERGE (ds)-[:APPLIES_TO]->(lti)
} IN TRANSACTIONS OF 5000 ROWS;

// DefinitionScope -> Target Chapter
CALL {
    MATCH (ds:DefinitionScope {scope_type: 'chapter'})
    WHERE ds.target_chapter_id IS NOT NULL
    MATCH (cv:ChapterVersion {chapter_id: ds.target_chapter_id})
    MERGE (ds)-[:APPLIES_TO_CHAPTER]->(cv)
} IN TRANSACTIONS OF 5000 ROWS;

// DefinitionScope -> Range
CALL {
    MATCH (ds:DefinitionScope)
    WHERE ds.scope_type IN ['range', 'subchapter'] 
      AND ds.target_range_start IS NOT NULL
    MATCH (start:LegalTextIdentity {canonical_id: ds.target_range_start})
    MERGE (ds)-[:APPLIES_TO_RANGE_START]->(start)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (ds:DefinitionScope)
    WHERE ds.scope_type IN ['range', 'subchapter'] 
      AND ds.target_range_end IS NOT NULL
    MATCH (end:LegalTextIdentity {canonical_id: ds.target_range_end})
    MERGE (ds)-[:APPLIES_TO_RANGE_END]->(end)
} IN TRANSACTIONS OF 5000 ROWS;
