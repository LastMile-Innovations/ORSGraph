// 1. Identity & Breadcrumb Context
// Parameters: $chunk_id
MATCH (c:RetrievalChunk {chunk_id: $chunk_id})
OPTIONAL MATCH (c)-[:DERIVED_FROM]->(p:Provision)
OPTIONAL MATCH (p)-[:PART_OF]->(ltv:LegalTextVersion)
RETURN c.chunk_id AS chunk_id,
       c.breadcrumb AS breadcrumb,
       ltv.edition_year AS edition_year,
       ltv.status AS status,
       ltv.official_status AS official_status;

// 2. Citation Context: Follow RESOLVES_TO_PROVISION to get cited text
// Parameters: $chunk_id
MATCH (c:RetrievalChunk {chunk_id: $chunk_id})
MATCH (c)-[:DERIVED_FROM]->(p:Provision)
MATCH (p)-[:MENTIONS_CITATION]->(cm:CitationMention)
MATCH (cm)-[:RESOLVES_TO_PROVISION]->(target:Provision)
RETURN cm.normalized_citation AS citation,
       target.display_citation AS target_citation,
       target.text AS target_text
LIMIT 5;

// 3. Definition Context: Definitions extracted from the same provision
// Parameters: $chunk_id
MATCH (c:RetrievalChunk {chunk_id: $chunk_id})
MATCH (c)-[:DERIVED_FROM]->(p:Provision)
MATCH (p)-[:DEFINES]->(d:Definition)
MATCH (d)-[:DEFINES_TERM]->(dt:DefinedTerm)
RETURN dt.term AS term,
       d.definition_text AS definition;

// 4. Global Definition Context: Definitions applicable to this provision's scope
// This follows DEFINITION_SCOPE relationships if implemented.
// Parameters: $chunk_id
MATCH (c:RetrievalChunk {chunk_id: $chunk_id})
MATCH (c)-[:DERIVED_FROM]->(p:Provision)
MATCH (p)-[:IN_SCOPE_OF]->(ds:DefinitionScope)<-[:HAS_SCOPE]-(d:Definition)
MATCH (d)-[:DEFINES_TERM]->(dt:DefinedTerm)
RETURN dt.term AS term,
       d.definition_text AS definition,
       ds.scope_type AS scope_type;

// 5. V3 retrieval candidate filter and ranking preference
// Parameters: none
MATCH (c:RetrievalChunk)
WHERE c.embedding_policy IN ["embed_primary", "embed_special"]
  AND c.token_count <= 30000
RETURN c
ORDER BY
  CASE c.chunk_type
    WHEN "contextual_provision" THEN 1.20
    WHEN "definition_block" THEN 1.15
    WHEN "exception_block" THEN 1.15
    WHEN "deadline_block" THEN 1.10
    WHEN "penalty_block" THEN 1.10
    WHEN "citation_context" THEN 1.05
    ELSE 1.00
  END * coalesce(c.search_weight, 1.0) DESC;
