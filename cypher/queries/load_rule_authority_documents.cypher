UNWIND $rows AS row
MERGE (doc:RuleAuthorityDocument {authority_document_id: row.authority_document_id})
SET doc += row { .title, .jurisdiction_id, .jurisdiction, .subcategory, .authority_kind,
                 .authority_identifier, .effective_start_date, .effective_end_date,
                 .publication_bucket, .date_status, .status_flags, .topic_ids,
                 .amends_authority_document_id, .source_registry_id, .source_snapshot_id,
                 .source_url }
SET doc.id = row.authority_document_id,
    doc.graph_kind = 'rule_authority_document',
    doc.schema_version = '1.0.0',
    doc.source_system = 'ors_crawler',
    doc.updated_at = datetime()
SET doc.created_at = coalesce(doc.created_at, datetime())
FOREACH (_ IN CASE WHEN row.authority_kind = 'ChiefJusticeOrder' THEN [1] ELSE [] END | SET doc:ChiefJusticeOrder)
FOREACH (_ IN CASE WHEN row.authority_kind = 'PresidingJudgeOrder' THEN [1] ELSE [] END | SET doc:PresidingJudgeOrder)
FOREACH (_ IN CASE WHEN row.authority_kind = 'SupplementaryLocalRuleEdition' THEN [1] ELSE [] END | SET doc:SupplementaryLocalRuleEdition)
FOREACH (_ IN CASE WHEN row.authority_kind = 'OutOfCycleAmendment' THEN [1] ELSE [] END | SET doc:OutOfCycleAmendment)
FOREACH (_ IN CASE WHEN row.title =~ '(?i).*emergency.*closure.*' THEN [1] ELSE [] END | SET doc:EmergencyClosureOrder)
FOREACH (_ IN CASE WHEN row.title =~ '(?i).*fees?.*' THEN [1] ELSE [] END | SET doc:FeeOrder)
FOREACH (_ IN CASE WHEN row.title =~ '(?i).*remote.*' THEN [1] ELSE [] END | SET doc:RemoteProceedingOrder)
FOREACH (_ IN CASE WHEN row.title =~ '(?i).*court operations.*' THEN [1] ELSE [] END | SET doc:CourtOperationsOrder)
FOREACH (_ IN CASE WHEN row.title =~ '(?i).*security screening.*' THEN [1] ELSE [] END | SET doc:SecurityScreeningOrder)
FOREACH (_ IN CASE WHEN row.title =~ '(?i).*pretrial release.*' THEN [1] ELSE [] END | SET doc:PretrialReleaseOrder)
FOREACH (_ IN CASE WHEN row.title =~ '(?i).*(seal|landlord tenant).*' THEN [1] ELSE [] END | SET doc:SealingOrder)
