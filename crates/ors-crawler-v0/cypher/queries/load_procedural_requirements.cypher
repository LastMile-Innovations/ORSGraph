UNWIND $rows AS row
MERGE (req:ProceduralRequirement {requirement_id: row.requirement_id})
SET req += row { .semantic_type, .requirement_type, .label, .text, .normalized_text,
               .source_provision_id, .source_citation, .applies_to, .value,
               .severity_default, .authority_family, .effective_date, .confidence,
               .extraction_method }
SET req.id = row.requirement_id,
    req.graph_kind = 'semantic',
    req.schema_version = '1.0.0',
    req.source_system = 'ors_crawler',
    req.review_status = coalesce(req.review_status, 'machine_extracted'),
    req.updated_at = datetime()
SET req.created_at = coalesce(req.created_at, datetime())
FOREACH (_ IN CASE WHEN row.semantic_type = 'ProceduralRule' THEN [1] ELSE [] END | SET req:ProceduralRule)
FOREACH (_ IN CASE WHEN row.semantic_type = 'ApplicabilityRule' THEN [1] ELSE [] END | SET req:ApplicabilityRule)
FOREACH (_ IN CASE WHEN row.semantic_type = 'FilingRequirement' THEN [1] ELSE [] END | SET req:FilingRequirement)
FOREACH (_ IN CASE WHEN row.semantic_type = 'FormattingRequirement' THEN [1] ELSE [] END | SET req:FormattingRequirement)
FOREACH (_ IN CASE WHEN row.semantic_type = 'CaptionRequirement' THEN [1] ELSE [] END | SET req:CaptionRequirement)
FOREACH (_ IN CASE WHEN row.semantic_type = 'SignatureRequirement' THEN [1] ELSE [] END | SET req:SignatureRequirement)
FOREACH (_ IN CASE WHEN row.semantic_type = 'CertificateOfServiceRequirement' THEN [1] ELSE [] END | SET req:CertificateOfServiceRequirement)
FOREACH (_ IN CASE WHEN row.semantic_type = 'ExhibitRequirement' THEN [1] ELSE [] END | SET req:ExhibitRequirement)
FOREACH (_ IN CASE WHEN row.semantic_type = 'ProtectedInformationRequirement' THEN [1] ELSE [] END | SET req:ProtectedInformationRequirement)
FOREACH (_ IN CASE WHEN row.semantic_type = 'EfilingRequirement' THEN [1] ELSE [] END | SET req:EfilingRequirement)
FOREACH (_ IN CASE WHEN row.semantic_type = 'ServiceRequirement' THEN [1] ELSE [] END | SET req:ServiceRequirement)
FOREACH (_ IN CASE WHEN row.semantic_type = 'DeadlineRule' THEN [1] ELSE [] END | SET req:DeadlineRule)
FOREACH (_ IN CASE WHEN row.semantic_type = 'SanctionRule' THEN [1] ELSE [] END | SET req:SanctionRule)
FOREACH (_ IN CASE WHEN row.semantic_type = 'ExceptionRule' THEN [1] ELSE [] END | SET req:ExceptionRule)
FOREACH (_ IN CASE WHEN row.semantic_type = 'MotionRequirement' THEN [1] ELSE [] END | SET req:MotionRequirement)
FOREACH (_ IN CASE WHEN row.semantic_type = 'OrderRequirement' THEN [1] ELSE [] END | SET req:OrderRequirement)
WITH req, row
MATCH (p:Provision {provision_id: row.source_provision_id})
MERGE (p)-[:EXPRESSES]->(req)
MERGE (req)-[:SUPPORTED_BY]->(p)
