UNWIND $rows AS row
MERGE (doc:RuleAuthorityDocument {authority_document_id: row.authority_document_id})
SET doc:SupplementaryLocalRuleEdition
SET doc += row { .edition_id, .corpus_id, .supplements_corpus_id, .jurisdiction_id, .court_id,
                 .edition_year, .title, .effective_start_date, .effective_end_date, .date_status }
SET doc.id = row.authority_document_id,
    doc.graph_kind = 'rule_authority_document',
    doc.schema_version = '1.0.0',
    doc.source_system = 'ors_crawler',
    doc.updated_at = datetime()
SET doc.created_at = coalesce(doc.created_at, datetime())
