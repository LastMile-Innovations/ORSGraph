// Create relationships between source documents and derived entities
// This consolidated method creates all source-related edges in a single operation:
// - Public body to source documents
// - Legal text versions to source documents
// - Provisions to source documents (via versions)
// - Citation mentions to source documents (via provisions)

CALL {
    MATCH (sd:SourceDocument)
    WITH sd, CASE WHEN sd.authority_family = 'UTCR' THEN 'or:judicial_department' ELSE 'or:legislature' END AS public_body_id
    OPTIONAL MATCH (pb:PublicBody {public_body_id: public_body_id})
    FOREACH (_ IN CASE WHEN pb IS NULL THEN [] ELSE [1] END |
        MERGE (pb)-[:PUBLISHED]->(sd)
        MERGE (pb)-[:PUBLISHES]->(sd)
    )
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
    MATCH (sp:SourcePage)
    MATCH (sd:SourceDocument {source_document_id: sp.source_document_id})
    MERGE (sd)-[:HAS_PAGE]->(sp)
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
CALL {
    MATCH (p:Provision)
    WHERE p.source_page_start IS NOT NULL
    MATCH (ltv:LegalTextVersion {version_id: p.version_id})
    MATCH (sp:SourcePage {source_document_id: ltv.source_document_id, page_number: p.source_page_start})
    MERGE (p)-[:APPEARS_ON_PAGE]->(sp)
    MERGE (sp)-[:CONTAINS_TEXT]->(p)
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
CALL {
    MATCH (rn:ReporterNote)
    OPTIONAL MATCH (p:Provision {provision_id: rn.source_provision_id})
    OPTIONAL MATCH (ltv:LegalTextVersion {version_id: rn.version_id})
    WITH rn, coalesce(p, ltv) AS target
    FOREACH (_ IN CASE WHEN target IS NULL THEN [] ELSE [1] END |
        MERGE (target)-[:HAS_REPORTER_NOTE]->(rn)
    )
} IN 8 CONCURRENT TRANSACTIONS OF 5000 ROWS
CALL {
    MATCH (c:Commentary)
    OPTIONAL MATCH (p:Provision {provision_id: c.source_provision_id})
    OPTIONAL MATCH (ltv:LegalTextVersion {version_id: c.version_id})
    WITH c, coalesce(p, ltv) AS target
    FOREACH (_ IN CASE WHEN target IS NULL THEN [] ELSE [1] END |
        MERGE (target)-[:HAS_COMMENTARY]->(c)
    )
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
