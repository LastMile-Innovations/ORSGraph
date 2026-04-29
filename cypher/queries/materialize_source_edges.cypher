// Create relationships between source documents and derived entities
// This consolidated method creates all source-related edges in a single operation:
// - Public body to source documents
// - Legal text versions to source documents
// - Provisions to source documents (via versions)
// - Citation mentions to source documents (via provisions)

CALL {
    // Public body to all source documents
    MATCH (pb:PublicBody)
    FILTER pb.public_body_id = 'or:legislature'
    MATCH (sd:SourceDocument)
    MERGE (pb)-[:PUBLISHED]->(sd)
    MERGE (pb)-[:PUBLISHES]->(sd)
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
CALL {
    // LegalTextVersion to SourceDocument
    MATCH (ltv:LegalTextVersion)
    MATCH (sd:SourceDocument)
    FILTER sd.source_document_id = ltv.source_document_id
    MERGE (sd)-[:SOURCE_FOR]->(ltv)
    MERGE (ltv)-[:DERIVED_FROM]->(sd)
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
CALL {
    // Provision to SourceDocument (via LegalTextVersion)
    MATCH (p:Provision)
    MATCH (ltv:LegalTextVersion)
    FILTER ltv.version_id = p.version_id
    MATCH (sd:SourceDocument)
    FILTER sd.source_document_id = ltv.source_document_id
    MERGE (sd)-[:SOURCE_FOR]->(p)
    MERGE (p)-[:DERIVED_FROM]->(sd)
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
CALL {
    // CitationMention to SourceDocument (via Provision -> LegalTextVersion)
    MATCH (cm:CitationMention)
    MATCH (p:Provision)
    FILTER p.provision_id = cm.source_provision_id
    MATCH (ltv:LegalTextVersion)
    FILTER ltv.version_id = p.version_id
    MATCH (sd:SourceDocument)
    FILTER sd.source_document_id = ltv.source_document_id
    MERGE (sd)-[:SOURCE_FOR]->(cm)
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
CALL {
    // SourceNote to SourceDocument and LegalTextVersion
    MATCH (sn:SourceNote)
    OPTIONAL MATCH (sd:SourceDocument {source_document_id: sn.source_document_id})
    FOREACH (_ IN CASE WHEN sd IS NULL THEN [] ELSE [1] END |
        MERGE (sd)-[:SOURCE_FOR]->(sn)
        MERGE (sn)-[:DERIVED_FROM]->(sd)
    )
    WITH sn
    OPTIONAL MATCH (ltv:LegalTextVersion {version_id: sn.version_id})
    FOREACH (_ IN CASE WHEN ltv IS NULL THEN [] ELSE [1] END |
        MERGE (ltv)-[:HAS_SOURCE_NOTE]->(sn)
        MERGE (sn)-[:ANNOTATES]->(ltv)
    )
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
