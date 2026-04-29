// Create relationships from citation mentions to their resolved targets

CALL {
    MATCH (cm:CitationMention)
    MATCH (p:Provision {provision_id: cm.source_provision_id})
    MERGE (p)-[:MENTIONS_CITATION]->(cm)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (cm:CitationMention)
    WHERE cm.target_canonical_id IS NOT NULL
    MATCH (lti:LegalTextIdentity {canonical_id: cm.target_canonical_id})
    MERGE (cm)-[:RESOLVES_TO]->(lti)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (cm:CitationMention)
    WHERE cm.target_canonical_id IS NOT NULL
    MATCH (ltv:LegalTextVersion {canonical_id: cm.target_canonical_id})
    MERGE (cm)-[:RESOLVES_TO_VERSION]->(ltv)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (cm:CitationMention)
    WHERE cm.target_provision_id IS NOT NULL
    MATCH (p:Provision {provision_id: cm.target_provision_id})
    MERGE (cm)-[:RESOLVES_TO_PROVISION]->(p)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (cm:CitationMention)
    WHERE cm.citation_type = 'statute_chapter'
    WITH cm, replace(toLower(cm.normalized_citation), 'ors chapter ', '') AS chapter_num
    MATCH (cv:ChapterVersion {chapter: chapter_num})
    MERGE (cm)-[:RESOLVES_TO_CHAPTER]->(cv)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (cm:CitationMention)
    WHERE cm.resolver_status = 'resolved_external' AND cm.external_citation_id IS NOT NULL
    MATCH (elc:ExternalLegalCitation {external_citation_id: cm.external_citation_id})
    MERGE (cm)-[:RESOLVES_TO_EXTERNAL]->(elc)
} IN TRANSACTIONS OF 5000 ROWS;
