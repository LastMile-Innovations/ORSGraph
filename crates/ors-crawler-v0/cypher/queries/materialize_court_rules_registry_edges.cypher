CALL {
    MATCH (src:CourtRulesRegistrySource)
    MATCH (entry:RulePublicationEntry {registry_source_id: src.registry_source_id})
    MERGE (src)-[:HAS_ENTRY]->(entry)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (snap:CourtRulesRegistrySnapshot)
    MATCH (entry:RulePublicationEntry {registry_snapshot_id: snap.registry_snapshot_id})
    MERGE (snap)-[:HAS_ENTRY]->(entry)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (entry:RulePublicationEntry)
    MATCH (doc:RuleAuthorityDocument {authority_document_id: entry.authority_document_id})
    MERGE (entry)-[:DESCRIBES]->(doc)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (entry:RulePublicationEntry)
    MATCH (j:Jurisdiction {jurisdiction_id: entry.jurisdiction_id})
    MERGE (entry)-[:APPLIES_TO_JURISDICTION]->(j)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (doc:RuleAuthorityDocument)
    MATCH (interval:EffectiveInterval {authority_document_id: doc.authority_document_id})
    MERGE (doc)-[:EFFECTIVE_DURING]->(interval)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (doc:RuleAuthorityDocument)
    UNWIND coalesce(doc.topic_ids, []) AS topic_id
    MATCH (topic:RuleTopic {rule_topic_id: topic_id})
    MERGE (doc)-[:HAS_TOPIC]->(topic)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (slr:SupplementaryLocalRuleEdition)
    WHERE slr.supplements_corpus_id IS NOT NULL
    MATCH (base:LegalCorpus {corpus_id: slr.supplements_corpus_id})
    MERGE (slr)-[:SUPPLEMENTS]->(base)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (amendment:OutOfCycleAmendment)
    WHERE amendment.amends_authority_document_id IS NOT NULL
    MATCH (edition:SupplementaryLocalRuleEdition {authority_document_id: amendment.amends_authority_document_id})
    MERGE (amendment)-[:AMENDS]->(edition)
} IN TRANSACTIONS OF 5000 ROWS;
