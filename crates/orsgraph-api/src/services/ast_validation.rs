use crate::error::{ApiError, ApiResult};
use crate::models::casebuilder::*;
use crate::services::work_product_ast::{
    ast_block_spec, expected_formatting_profile_id, expected_profile_id, expected_rule_pack_id,
    find_ast_block, flatten_work_product_blocks, normalize_work_product_type_lossy,
    registered_template_product_type, required_role_specs_for_work_product,
    role_spec_matches_block, validate_optional_text_range, SUPPORTED_BLOCK_TYPES,
};
use std::collections::HashSet;

pub(crate) fn validate_work_product_document(product: &WorkProduct) -> AstValidationResponse {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let document = &product.document_ast;

    if document.schema_version.trim().is_empty() {
        errors.push(ast_issue(
            "missing_schema_version",
            "WorkProduct AST is missing schema_version.",
            "error",
            Some("document"),
            Some(&document.document_id),
        ));
    }
    if document.matter_id != product.matter_id {
        errors.push(ast_issue(
            "ast_matter_mismatch",
            "WorkProduct AST matter_id does not match the work product.",
            "error",
            Some("document"),
            Some(&document.document_id),
        ));
    }
    if document.work_product_id != product.work_product_id {
        errors.push(ast_issue(
            "ast_work_product_mismatch",
            "WorkProduct AST work_product_id does not match the work product.",
            "error",
            Some("document"),
            Some(&document.document_id),
        ));
    }
    if document.document_type.trim().is_empty() {
        errors.push(ast_issue(
            "missing_document_type",
            "WorkProduct AST is missing document_type.",
            "error",
            Some("document"),
            Some(&document.document_id),
        ));
    }
    if document.document_type != product.product_type
        || document.product_type != product.product_type
    {
        errors.push(ast_issue(
            "ast_document_type_mismatch",
            "WorkProduct AST document_type/product_type does not match the work product.",
            "error",
            Some("document"),
            Some(&document.document_id),
        ));
    }
    if document.metadata.status.trim().is_empty() {
        warnings.push(ast_issue(
            "missing_metadata_status",
            "WorkProduct AST metadata is missing status.",
            "warning",
            Some("document"),
            Some(&document.document_id),
        ));
    }
    validate_registry_references(product, &mut errors, &mut warnings);

    let mut seen_blocks = HashSet::new();
    let mut parent_ids = HashSet::new();
    validate_ast_blocks(
        &document.blocks,
        None,
        &mut Vec::new(),
        &mut seen_blocks,
        &mut parent_ids,
        &mut errors,
        &mut warnings,
    );
    for parent_id in parent_ids {
        if !seen_blocks.contains(&parent_id) {
            errors.push(ast_issue(
                "missing_parent",
                &format!("Parent block {parent_id} does not exist."),
                "error",
                Some("block"),
                Some(&parent_id),
            ));
        }
    }

    validate_unique_ids(
        document.links.iter().map(|link| link.link_id.as_str()),
        "duplicate_link_id",
        "link",
        &mut errors,
    );
    validate_unique_ids(
        document
            .citations
            .iter()
            .map(|citation| citation.citation_use_id.as_str()),
        "duplicate_citation_use_id",
        "citation",
        &mut errors,
    );
    validate_unique_ids(
        document
            .exhibits
            .iter()
            .map(|exhibit| exhibit.exhibit_reference_id.as_str()),
        "duplicate_exhibit_reference_id",
        "exhibit",
        &mut errors,
    );
    validate_unique_ids(
        document
            .rule_findings
            .iter()
            .map(|finding| finding.finding_id.as_str()),
        "duplicate_rule_finding_id",
        "rule_finding",
        &mut errors,
    );

    validate_links(document, &mut errors);
    validate_citations(document, &mut errors, &mut warnings);
    validate_exhibits(document, &mut errors, &mut warnings);
    validate_block_refs(document, &mut errors);
    validate_rule_findings(product, &mut errors);
    validate_required_blocks(product, &mut warnings);

    AstValidationResponse {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

pub(crate) fn ensure_work_product_ast_valid(product: &WorkProduct, context: &str) -> ApiResult<()> {
    let validation = validate_work_product_document(product);
    if validation.errors.is_empty() {
        return Ok(());
    }
    let codes = validation
        .errors
        .iter()
        .map(|issue| issue.code.clone())
        .collect::<Vec<_>>()
        .join(",");
    Err(ApiError::BadRequest(format!(
        "{context} failed AST validation: issue_codes={codes}"
    )))
}

fn validate_ast_blocks(
    blocks: &[WorkProductBlock],
    parent_id: Option<&str>,
    path: &mut Vec<String>,
    seen: &mut HashSet<String>,
    parent_ids: &mut HashSet<String>,
    errors: &mut Vec<AstValidationIssue>,
    warnings: &mut Vec<AstValidationIssue>,
) {
    let mut order_indexes = HashSet::new();
    for block in blocks {
        if block.block_id.trim().is_empty() {
            errors.push(ast_issue(
                "missing_block_id",
                "AST block is missing block_id.",
                "error",
                Some("block"),
                None,
            ));
        } else if !seen.insert(block.block_id.clone()) {
            errors.push(ast_issue(
                "duplicate_block_id",
                &format!("Duplicate block id {}.", block.block_id),
                "error",
                Some("block"),
                Some(&block.block_id),
            ));
        }
        if block.block_type.trim().is_empty() {
            errors.push(ast_issue(
                "missing_block_type",
                &format!("Block {} is missing type.", block.block_id),
                "error",
                Some("block"),
                Some(&block.block_id),
            ));
        } else {
            match ast_block_spec(&block.block_type) {
                Some(spec) => validate_block_against_spec(block, spec, errors, warnings),
                None => {
                    warnings.push(ast_issue(
                        "unknown_block_type",
                        &format!(
                            "Block {} uses non-MVP block type {}.",
                            block.block_id, block.block_type
                        ),
                        "warning",
                        Some("block"),
                        Some(&block.block_id),
                    ));
                    if !SUPPORTED_BLOCK_TYPES.contains(&block.block_type.as_str()) {
                        warnings.push(ast_issue(
                            "unregistered_block_type",
                            &format!(
                                "Block {} is not registered in the WorkProduct AST block registry.",
                                block.block_id
                            ),
                            "warning",
                            Some("block"),
                            Some(&block.block_id),
                        ));
                    }
                }
            }
        }
        if block.ordinal == 0 || !order_indexes.insert(block.ordinal) {
            warnings.push(ast_issue(
                "order_index_review",
                &format!(
                    "Block {} has a duplicate or zero order_index.",
                    block.block_id
                ),
                "warning",
                Some("block"),
                Some(&block.block_id),
            ));
        }
        if let Some(parent_id) = parent_id.or(block.parent_block_id.as_deref()) {
            parent_ids.insert(parent_id.to_string());
            if parent_id == block.block_id || path.iter().any(|id| id == &block.block_id) {
                errors.push(ast_issue(
                    "block_cycle",
                    &format!(
                        "Block {} participates in a parent/child cycle.",
                        block.block_id
                    ),
                    "error",
                    Some("block"),
                    Some(&block.block_id),
                ));
            }
        }
        path.push(block.block_id.clone());
        validate_ast_blocks(
            &block.children,
            Some(&block.block_id),
            path,
            seen,
            parent_ids,
            errors,
            warnings,
        );
        path.pop();
    }
}

fn validate_block_against_spec(
    block: &WorkProductBlock,
    spec: crate::services::work_product_ast::AstBlockSpec,
    errors: &mut Vec<AstValidationIssue>,
    warnings: &mut Vec<AstValidationIssue>,
) {
    if block.block_type != spec.block_type {
        warnings.push(ast_issue(
            "block_type_not_canonical",
            &format!(
                "Block {} type should be canonicalized as {}.",
                block.block_id, spec.block_type
            ),
            "warning",
            Some("block"),
            Some(&block.block_id),
        ));
    }
    if spec.requires_title && block.title.trim().is_empty() {
        warnings.push(ast_issue(
            "block_title_missing",
            &format!(
                "{} block {} is missing a title.",
                spec.block_type, block.block_id
            ),
            "warning",
            Some("block"),
            Some(&block.block_id),
        ));
    }
    if !spec.allows_text && !block.text.trim().is_empty() {
        warnings.push(ast_issue(
            "block_text_not_allowed",
            &format!(
                "{} block {} should not contain text.",
                spec.block_type, block.block_id
            ),
            "warning",
            Some("block"),
            Some(&block.block_id),
        ));
    }
    if !spec.allows_children && !block.children.is_empty() {
        errors.push(ast_issue(
            "block_children_not_allowed",
            &format!(
                "{} block {} cannot contain child blocks.",
                spec.block_type, block.block_id
            ),
            "error",
            Some("block"),
            Some(&block.block_id),
        ));
    }
    if spec.requires_paragraph_number && block.paragraph_number.is_none() {
        warnings.push(ast_issue(
            "paragraph_number_missing",
            &format!(
                "Numbered paragraph block {} is missing paragraph_number.",
                block.block_id
            ),
            "warning",
            Some("block"),
            Some(&block.block_id),
        ));
    }
    if spec.requires_sentence_id && block.sentence_id.is_none() {
        warnings.push(ast_issue(
            "sentence_id_missing",
            &format!("Sentence block {} is missing sentence_id.", block.block_id),
            "warning",
            Some("block"),
            Some(&block.block_id),
        ));
    }
    if spec.block_type == "count" && block.count_number.is_none() {
        warnings.push(ast_issue(
            "count_number_missing",
            &format!("Count block {} is missing count_number.", block.block_id),
            "warning",
            Some("block"),
            Some(&block.block_id),
        ));
    }
}

fn validate_unique_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    code: &str,
    target_type: &str,
    errors: &mut Vec<AstValidationIssue>,
) {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            errors.push(ast_issue(
                &format!("missing_{target_type}_id"),
                &format!("AST {target_type} is missing an id."),
                "error",
                Some(target_type),
                None,
            ));
        } else if !seen.insert(id.to_string()) {
            errors.push(ast_issue(
                code,
                &format!("Duplicate {target_type} id {id}."),
                "error",
                Some(target_type),
                Some(id),
            ));
        }
    }
}

fn validate_links(document: &WorkProductDocument, errors: &mut Vec<AstValidationIssue>) {
    for link in &document.links {
        match find_ast_block(&document.blocks, &link.source_block_id) {
            Some(block) => {
                if validate_optional_text_range(&block.text, link.source_text_range.as_ref())
                    .is_err()
                {
                    errors.push(ast_issue(
                        "invalid_link_text_range",
                        "Link text range does not match the source block.",
                        "error",
                        Some("link"),
                        Some(&link.link_id),
                    ));
                }
            }
            None => errors.push(ast_issue(
                "missing_link_source_block",
                "Link source block does not exist.",
                "error",
                Some("link"),
                Some(&link.link_id),
            )),
        }
    }
}

fn validate_citations(
    document: &WorkProductDocument,
    errors: &mut Vec<AstValidationIssue>,
    warnings: &mut Vec<AstValidationIssue>,
) {
    for citation in &document.citations {
        match find_ast_block(&document.blocks, &citation.source_block_id) {
            Some(block) => {
                if validate_optional_text_range(&block.text, citation.source_text_range.as_ref())
                    .is_err()
                {
                    errors.push(ast_issue(
                        "invalid_citation_text_range",
                        "Citation text range does not match the source block.",
                        "error",
                        Some("citation"),
                        Some(&citation.citation_use_id),
                    ));
                }
            }
            None => errors.push(ast_issue(
                "missing_citation_source_block",
                "Citation source block does not exist.",
                "error",
                Some("citation"),
                Some(&citation.citation_use_id),
            )),
        }
        if matches!(
            citation.status.as_str(),
            "unresolved" | "ambiguous" | "stale" | "currentness_warning" | "needs_review"
        ) {
            warnings.push(ast_issue(
                "citation_needs_review",
                &format!("Citation '{}' needs review.", citation.raw_text),
                "warning",
                Some("citation"),
                Some(&citation.citation_use_id),
            ));
        }
    }
}

fn validate_exhibits(
    document: &WorkProductDocument,
    errors: &mut Vec<AstValidationIssue>,
    warnings: &mut Vec<AstValidationIssue>,
) {
    for exhibit in &document.exhibits {
        match find_ast_block(&document.blocks, &exhibit.source_block_id) {
            Some(block) => {
                if validate_optional_text_range(&block.text, exhibit.source_text_range.as_ref())
                    .is_err()
                {
                    errors.push(ast_issue(
                        "invalid_exhibit_text_range",
                        "Exhibit text range does not match the source block.",
                        "error",
                        Some("exhibit"),
                        Some(&exhibit.exhibit_reference_id),
                    ));
                }
            }
            None => errors.push(ast_issue(
                "missing_exhibit_source_block",
                "Exhibit source block does not exist.",
                "error",
                Some("exhibit"),
                Some(&exhibit.exhibit_reference_id),
            )),
        }
        if exhibit.status != "attached" {
            warnings.push(ast_issue(
                "exhibit_needs_review",
                &format!("Exhibit reference '{}' is not attached.", exhibit.label),
                "warning",
                Some("exhibit"),
                Some(&exhibit.exhibit_reference_id),
            ));
        }
    }
}

fn validate_block_refs(document: &WorkProductDocument, errors: &mut Vec<AstValidationIssue>) {
    let link_ids = document
        .links
        .iter()
        .map(|link| link.link_id.clone())
        .collect::<HashSet<_>>();
    let citation_ids = document
        .citations
        .iter()
        .map(|citation| citation.citation_use_id.clone())
        .collect::<HashSet<_>>();
    let exhibit_ids = document
        .exhibits
        .iter()
        .map(|exhibit| exhibit.exhibit_reference_id.clone())
        .collect::<HashSet<_>>();
    let finding_ids = document
        .rule_findings
        .iter()
        .map(|finding| finding.finding_id.clone())
        .collect::<HashSet<_>>();
    for block in flatten_work_product_blocks(&document.blocks) {
        for link_id in &block.links {
            if !link_ids.contains(link_id) {
                errors.push(ast_issue(
                    "broken_block_link",
                    &format!(
                        "Block {} references missing link {link_id}.",
                        block.block_id
                    ),
                    "error",
                    Some("block"),
                    Some(&block.block_id),
                ));
            }
        }
        for citation_id in &block.citations {
            if !citation_ids.contains(citation_id) {
                errors.push(ast_issue(
                    "broken_block_citation",
                    &format!(
                        "Block {} references missing citation {citation_id}.",
                        block.block_id
                    ),
                    "error",
                    Some("block"),
                    Some(&block.block_id),
                ));
            }
        }
        for exhibit_id in &block.exhibits {
            if !exhibit_ids.contains(exhibit_id) {
                errors.push(ast_issue(
                    "broken_block_exhibit",
                    &format!(
                        "Block {} references missing exhibit {exhibit_id}.",
                        block.block_id
                    ),
                    "error",
                    Some("block"),
                    Some(&block.block_id),
                ));
            }
        }
        for finding_id in &block.rule_finding_ids {
            if !finding_ids.contains(finding_id) {
                errors.push(ast_issue(
                    "broken_block_rule_finding",
                    &format!(
                        "Block {} references missing rule finding {finding_id}.",
                        block.block_id
                    ),
                    "error",
                    Some("block"),
                    Some(&block.block_id),
                ));
            }
        }
    }
}

fn validate_rule_findings(product: &WorkProduct, errors: &mut Vec<AstValidationIssue>) {
    let block_ids = flatten_work_product_blocks(&product.document_ast.blocks)
        .into_iter()
        .map(|block| block.block_id)
        .collect::<HashSet<_>>();
    let citation_ids = product
        .document_ast
        .citations
        .iter()
        .map(|citation| citation.citation_use_id.clone())
        .collect::<HashSet<_>>();
    let exhibit_ids = product
        .document_ast
        .exhibits
        .iter()
        .map(|exhibit| exhibit.exhibit_reference_id.clone())
        .collect::<HashSet<_>>();
    let link_ids = product
        .document_ast
        .links
        .iter()
        .map(|link| link.link_id.clone())
        .collect::<HashSet<_>>();
    let document_targets = HashSet::from([
        product.work_product_id.clone(),
        product.document_ast.document_id.clone(),
        product.formatting_profile.profile_id.clone(),
    ]);
    for finding in &product.document_ast.rule_findings {
        let block_target = matches!(
            finding.target_type.as_str(),
            "block" | "paragraph" | "section" | "count" | "caption" | "sentence"
        );
        if block_target && !block_ids.contains(&finding.target_id) {
            errors.push(ast_issue(
                "missing_rule_finding_target",
                "Rule finding target does not resolve to an AST node.",
                "error",
                Some("rule_finding"),
                Some(&finding.finding_id),
            ));
        }
        if matches!(
            finding.target_type.as_str(),
            "work_product" | "document" | "formatting"
        ) && !document_targets.contains(&finding.target_id)
        {
            errors.push(ast_issue(
                "missing_rule_finding_target",
                "Rule finding target does not resolve to the work product document.",
                "error",
                Some("rule_finding"),
                Some(&finding.finding_id),
            ));
        }
        let valid_record_target = match finding.target_type.as_str() {
            "citation" | "citation_use" => citation_ids.contains(&finding.target_id),
            "exhibit" | "exhibit_reference" => exhibit_ids.contains(&finding.target_id),
            "link" | "support" | "support_link" => link_ids.contains(&finding.target_id),
            _ => true,
        };
        if !valid_record_target {
            errors.push(ast_issue(
                "missing_rule_finding_target",
                "Rule finding target does not resolve to an AST record.",
                "error",
                Some("rule_finding"),
                Some(&finding.finding_id),
            ));
        }
    }
}

fn validate_registry_references(
    product: &WorkProduct,
    errors: &mut Vec<AstValidationIssue>,
    warnings: &mut Vec<AstValidationIssue>,
) {
    let product_type = normalize_work_product_type_lossy(&product.product_type);
    let metadata = &product.document_ast.metadata;

    if let Some(metadata_type) = metadata.work_product_type.as_deref() {
        if normalize_work_product_type_lossy(metadata_type) != product_type {
            errors.push(ast_issue(
                "metadata_work_product_type_mismatch",
                "AST metadata work_product_type does not match the WorkProduct type.",
                "error",
                Some("document"),
                Some(&product.document_ast.document_id),
            ));
        }
    } else {
        warnings.push(ast_issue(
            "missing_metadata_work_product_type",
            "AST metadata is missing work_product_type.",
            "warning",
            Some("document"),
            Some(&product.document_ast.document_id),
        ));
    }

    if normalize_work_product_type_lossy(&product.profile.product_type) != product_type {
        errors.push(ast_issue(
            "profile_product_type_mismatch",
            "WorkProduct profile product_type does not match the WorkProduct type.",
            "error",
            Some("profile"),
            Some(&product.profile.profile_id),
        ));
    }

    let expected_profile = expected_profile_id(&product_type);
    if product.profile.profile_id != expected_profile {
        warnings.push(ast_issue(
            "profile_id_not_registered_default",
            &format!(
                "Profile {} does not match the registered default profile {}.",
                product.profile.profile_id, expected_profile
            ),
            "warning",
            Some("profile"),
            Some(&product.profile.profile_id),
        ));
    }

    match metadata.formatting_profile_id.as_deref() {
        Some(profile_id) if profile_id != product.formatting_profile.profile_id => {
            errors.push(ast_issue(
                "metadata_formatting_profile_mismatch",
                "AST metadata formatting_profile_id does not match the WorkProduct formatting profile.",
                "error",
                Some("formatting_profile"),
                Some(profile_id),
            ));
        }
        Some(_) => {}
        None => warnings.push(ast_issue(
            "missing_formatting_profile_id",
            "AST metadata is missing formatting_profile_id.",
            "warning",
            Some("document"),
            Some(&product.document_ast.document_id),
        )),
    }

    let expected_format = expected_formatting_profile_id(&product_type);
    if product.formatting_profile.profile_id != expected_format {
        warnings.push(ast_issue(
            "formatting_profile_not_registered_default",
            &format!(
                "Formatting profile {} does not match the registered default profile {}.",
                product.formatting_profile.profile_id, expected_format
            ),
            "warning",
            Some("formatting_profile"),
            Some(&product.formatting_profile.profile_id),
        ));
    }

    match metadata.rule_pack_id.as_deref() {
        Some(rule_pack_id) if rule_pack_id != product.rule_pack.rule_pack_id => {
            errors.push(ast_issue(
                "metadata_rule_pack_mismatch",
                "AST metadata rule_pack_id does not match the WorkProduct rule pack.",
                "error",
                Some("rule_pack"),
                Some(rule_pack_id),
            ));
        }
        Some(_) => {}
        None => warnings.push(ast_issue(
            "missing_rule_pack_id",
            "AST metadata is missing rule_pack_id.",
            "warning",
            Some("document"),
            Some(&product.document_ast.document_id),
        )),
    }

    let expected_rule_pack = expected_rule_pack_id(&product_type);
    if product.rule_pack.rule_pack_id != expected_rule_pack {
        warnings.push(ast_issue(
            "rule_pack_not_registered_default",
            &format!(
                "Rule pack {} does not match the registered default rule pack {}.",
                product.rule_pack.rule_pack_id, expected_rule_pack
            ),
            "warning",
            Some("rule_pack"),
            Some(&product.rule_pack.rule_pack_id),
        ));
    }

    if let Some(template_id) = metadata.template_id.as_deref() {
        match registered_template_product_type(template_id) {
            Some(template_product_type)
                if normalize_work_product_type_lossy(template_product_type) != product_type =>
            {
                errors.push(ast_issue(
                    "template_product_type_mismatch",
                    "AST metadata template_id belongs to a different WorkProduct type.",
                    "error",
                    Some("template"),
                    Some(template_id),
                ));
            }
            Some(_) => {}
            None => warnings.push(ast_issue(
                "unknown_template_id",
                &format!("AST metadata template_id {template_id} is not registered."),
                "warning",
                Some("template"),
                Some(template_id),
            )),
        }
    }
}

fn validate_required_blocks(product: &WorkProduct, warnings: &mut Vec<AstValidationIssue>) {
    let flat = flatten_work_product_blocks(&product.document_ast.blocks);
    let specs = required_role_specs_for_work_product(&product.product_type);
    let mut checked_roles = HashSet::new();
    for spec in specs {
        checked_roles.insert(spec.role.to_string());
        if !flat
            .iter()
            .any(|block| role_spec_matches_block(spec, block))
        {
            warnings.push(ast_issue(
                "required_block_missing",
                &format!("Required block role {} is missing or empty.", spec.role),
                "warning",
                Some("document"),
                Some(&product.document_ast.document_id),
            ));
        }
    }

    for role in &product.profile.required_block_roles {
        if checked_roles.contains(role) {
            continue;
        }
        let missing = flat
            .iter()
            .find(|block| &block.role == role)
            .map(|block| block.text.trim().is_empty())
            .unwrap_or(true);
        if missing {
            warnings.push(ast_issue(
                "profile_required_block_missing",
                &format!("Profile-required block role {role} is missing or empty."),
                "warning",
                Some("document"),
                Some(&product.document_ast.document_id),
            ));
        }
    }
}

pub(crate) fn ast_issue(
    code: &str,
    message: &str,
    severity: &str,
    target_type: Option<&str>,
    target_id: Option<&str>,
) -> AstValidationIssue {
    AstValidationIssue {
        code: code.to_string(),
        message: message.to_string(),
        severity: Some(severity.to_string()),
        blocking: severity == "error" || severity == "blocking",
        target_type: target_type.map(str::to_string),
        target_id: target_id.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_product(blocks: Vec<WorkProductBlock>) -> WorkProduct {
        WorkProduct {
            work_product_id: "wp:test".to_string(),
            id: "wp:test".to_string(),
            matter_id: "matter:test".to_string(),
            title: "Test".to_string(),
            product_type: "custom".to_string(),
            status: "draft".to_string(),
            review_status: "needs_review".to_string(),
            setup_stage: "test".to_string(),
            source_draft_id: None,
            source_complaint_id: None,
            created_at: "1".to_string(),
            updated_at: "2".to_string(),
            profile: WorkProductProfile {
                profile_id: "custom".to_string(),
                product_type: "custom".to_string(),
                name: "Custom".to_string(),
                jurisdiction: "Oregon".to_string(),
                version: "test".to_string(),
                route_slug: "custom".to_string(),
                required_block_roles: Vec::new(),
                optional_block_roles: Vec::new(),
                supports_rich_text: true,
            },
            document_ast: WorkProductDocument {
                schema_version: default_work_product_schema_version(),
                document_id: "wp:test:document".to_string(),
                work_product_id: "wp:test".to_string(),
                matter_id: "matter:test".to_string(),
                document_type: "custom".to_string(),
                product_type: "custom".to_string(),
                title: "Test".to_string(),
                blocks,
                ..WorkProductDocument::default()
            },
            blocks: Vec::new(),
            marks: Vec::new(),
            anchors: Vec::new(),
            findings: Vec::new(),
            artifacts: Vec::new(),
            history: Vec::new(),
            ai_commands: Vec::new(),
            formatting_profile: FormattingProfile {
                profile_id: "fmt".to_string(),
                name: "Format".to_string(),
                jurisdiction: "Oregon".to_string(),
                line_numbers: true,
                double_spaced: true,
                first_page_top_blank_inches: 1.0,
                margin_top_inches: 1.0,
                margin_bottom_inches: 1.0,
                margin_left_inches: 1.0,
                margin_right_inches: 1.0,
                font_family: "Times".to_string(),
                font_size_pt: 12,
            },
            rule_pack: RulePack {
                rule_pack_id: "rules".to_string(),
                name: "Rules".to_string(),
                jurisdiction: "Oregon".to_string(),
                version: "test".to_string(),
                effective_date: "2026-01-01".to_string(),
                rule_profile: RuleProfileSummary {
                    jurisdiction_id: "or".to_string(),
                    resolver_endpoint: "test".to_string(),
                    ..RuleProfileSummary::default()
                },
                rules: Vec::new(),
            },
        }
    }

    fn block(id: &str) -> WorkProductBlock {
        WorkProductBlock {
            block_id: id.to_string(),
            id: id.to_string(),
            matter_id: "matter:test".to_string(),
            work_product_id: "wp:test".to_string(),
            block_type: "paragraph".to_string(),
            role: "custom".to_string(),
            title: "Block".to_string(),
            text: "Some text".to_string(),
            ordinal: 1,
            ..WorkProductBlock::default()
        }
    }

    fn finding(id: &str, target_type: &str, target_id: &str) -> WorkProductFinding {
        WorkProductFinding {
            finding_id: id.to_string(),
            id: id.to_string(),
            matter_id: "matter:test".to_string(),
            work_product_id: "wp:test".to_string(),
            rule_id: "rule:test".to_string(),
            rule_pack_id: Some("rules".to_string()),
            source_citation: None,
            source_url: None,
            category: "qc".to_string(),
            severity: "warning".to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            message: "Review target.".to_string(),
            explanation: String::new(),
            suggested_fix: String::new(),
            auto_fix_available: false,
            primary_action: WorkProductAction {
                action_id: format!("{id}:action"),
                label: "Review".to_string(),
                action_type: "inspect".to_string(),
                href: None,
                target_type: target_type.to_string(),
                target_id: target_id.to_string(),
            },
            status: "open".to_string(),
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        }
    }

    fn role_block(
        id: &str,
        block_type: &str,
        role: &str,
        title: &str,
        text: &str,
        ordinal: u64,
    ) -> WorkProductBlock {
        WorkProductBlock {
            block_id: id.to_string(),
            id: id.to_string(),
            matter_id: "matter:test".to_string(),
            work_product_id: "wp:test".to_string(),
            block_type: block_type.to_string(),
            role: role.to_string(),
            title: title.to_string(),
            text: text.to_string(),
            ordinal,
            count_number: if block_type == "count" { Some(1) } else { None },
            ..WorkProductBlock::default()
        }
    }

    fn complaint_product(blocks: Vec<WorkProductBlock>) -> WorkProduct {
        let mut product = test_product(blocks);
        product.product_type = "complaint".to_string();
        product.profile = WorkProductProfile {
            profile_id: expected_profile_id("complaint"),
            product_type: "complaint".to_string(),
            name: "Structured Complaint".to_string(),
            jurisdiction: "Oregon".to_string(),
            version: "test".to_string(),
            route_slug: "complaint".to_string(),
            required_block_roles: vec![
                "caption".to_string(),
                "jurisdiction".to_string(),
                "facts".to_string(),
                "count".to_string(),
                "relief".to_string(),
                "signature".to_string(),
            ],
            optional_block_roles: Vec::new(),
            supports_rich_text: true,
        };
        product.formatting_profile.profile_id = expected_formatting_profile_id("complaint");
        product.rule_pack.rule_pack_id = expected_rule_pack_id("complaint");
        product.document_ast.document_type = "complaint".to_string();
        product.document_ast.product_type = "complaint".to_string();
        product.document_ast.metadata.work_product_type = Some("complaint".to_string());
        product.document_ast.metadata.rule_pack_id = Some(product.rule_pack.rule_pack_id.clone());
        product.document_ast.metadata.formatting_profile_id =
            Some(product.formatting_profile.profile_id.clone());
        product
    }

    #[test]
    fn rejects_duplicate_block_ids() {
        let product = test_product(vec![block("b1"), block("b1")]);
        let validation = validate_work_product_document(&product);
        assert!(validation
            .errors
            .iter()
            .any(|issue| issue.code == "duplicate_block_id"));
    }

    #[test]
    fn catches_broken_block_refs() {
        let mut b = block("b1");
        b.links.push("missing".to_string());
        let product = test_product(vec![b]);
        let validation = validate_work_product_document(&product);
        assert!(validation
            .errors
            .iter()
            .any(|issue| issue.code == "broken_block_link"));
    }

    #[test]
    fn complaint_seed_alias_roles_satisfy_required_blocks() {
        let product = complaint_product(vec![
            role_block("b1", "caption", "caption", "Caption", "Court caption", 1),
            role_block(
                "b2",
                "section",
                "jurisdiction_venue",
                "Jurisdiction and venue",
                "Venue is proper.",
                2,
            ),
            role_block(
                "b3",
                "section",
                "factual_paragraph",
                "Factual allegations",
                "Facts go here.",
                3,
            ),
            role_block("b4", "count", "count", "Count I", "Claim text.", 4),
            role_block(
                "b5",
                "section",
                "prayer_for_relief",
                "Prayer",
                "Relief requested.",
                5,
            ),
            role_block(
                "b6",
                "signature",
                "signature_block",
                "Signature",
                "Signature block.",
                6,
            ),
        ]);
        let validation = validate_work_product_document(&product);
        assert!(!validation.warnings.iter().any(|issue| {
            issue.code == "required_block_missing" || issue.code == "profile_required_block_missing"
        }));
    }

    #[test]
    fn catches_template_product_type_mismatch() {
        let mut product = complaint_product(vec![role_block(
            "b1",
            "caption",
            "caption",
            "Caption",
            "Court caption",
            1,
        )]);
        product.document_ast.metadata.template_id = Some("answer-response-grid".to_string());
        let validation = validate_work_product_document(&product);
        assert!(validation
            .errors
            .iter()
            .any(|issue| issue.code == "template_product_type_mismatch"));
    }

    #[test]
    fn rejects_children_on_leaf_block_types() {
        let mut parent = block("b1");
        parent.children.push(role_block(
            "b2",
            "paragraph",
            "custom",
            "Child",
            "Child text",
            1,
        ));
        let product = test_product(vec![parent]);
        let validation = validate_work_product_document(&product);
        assert!(validation
            .errors
            .iter()
            .any(|issue| issue.code == "block_children_not_allowed"));
    }

    #[test]
    fn rule_findings_resolve_non_block_ast_records() {
        let mut product = test_product(vec![block("b1")]);
        product.document_ast.links.push(WorkProductLink {
            link_id: "link:1".to_string(),
            source_block_id: "b1".to_string(),
            source_text_range: None,
            target_type: "fact".to_string(),
            target_id: "fact:1".to_string(),
            relation: "supports".to_string(),
            confidence: Some(0.8),
            created_by: "tester".to_string(),
            created_at: "1".to_string(),
        });
        product.document_ast.citations.push(WorkProductCitationUse {
            citation_use_id: "cite:1".to_string(),
            source_block_id: "b1".to_string(),
            source_text_range: None,
            raw_text: "ORS 90.100".to_string(),
            normalized_citation: Some("ORS 90.100".to_string()),
            target_type: "legal_authority".to_string(),
            target_id: Some("ors:90.100".to_string()),
            pinpoint: None,
            status: "resolved".to_string(),
            resolver_message: None,
            created_at: "1".to_string(),
        });
        product
            .document_ast
            .exhibits
            .push(WorkProductExhibitReference {
                exhibit_reference_id: "exhibit:1".to_string(),
                source_block_id: "b1".to_string(),
                source_text_range: None,
                label: "Exhibit 1".to_string(),
                exhibit_id: Some("exhibit:attached:1".to_string()),
                document_id: None,
                page_range: None,
                status: "attached".to_string(),
                created_at: "1".to_string(),
            });
        product.document_ast.rule_findings = vec![
            finding("finding:link", "link", "link:1"),
            finding("finding:citation", "citation", "cite:1"),
            finding("finding:exhibit", "exhibit_reference", "exhibit:1"),
        ];

        let validation = validate_work_product_document(&product);
        assert!(!validation
            .errors
            .iter()
            .any(|issue| issue.code == "missing_rule_finding_target"));
    }

    #[test]
    fn rule_findings_reject_missing_non_block_ast_records() {
        let mut product = test_product(vec![block("b1")]);
        product.document_ast.rule_findings = vec![finding(
            "finding:missing-citation",
            "citation",
            "missing:cite",
        )];

        let validation = validate_work_product_document(&product);
        assert!(validation.errors.iter().any(|issue| {
            issue.code == "missing_rule_finding_target"
                && issue.target_id.as_deref() == Some("finding:missing-citation")
        }));
    }
}
