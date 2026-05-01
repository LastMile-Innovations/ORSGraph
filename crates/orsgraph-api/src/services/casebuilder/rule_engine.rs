use super::work_product_ast::{
    canonical_work_product_blocks, humanize_product_type, now_string,
    required_role_specs_for_work_product, role_spec_matches_block, sanitize_path_segment,
};
use crate::models::casebuilder::*;

pub(crate) fn work_product_findings(product: &WorkProduct) -> Vec<WorkProductFinding> {
    let now = now_string();
    let mut findings = Vec::new();
    let blocks = canonical_work_product_blocks(product);
    for spec in required_role_specs_for_work_product(&product.product_type) {
        if !blocks
            .iter()
            .any(|block| role_spec_matches_block(spec, block))
        {
            findings.push(work_product_finding(
                product,
                &format!("required-block-{}", spec.role),
                "structure",
                "blocking",
                "work_product",
                &product.work_product_id,
                &format!("{} block is required.", humanize_product_type(spec.role)),
                "The active work-product profile requires this block before preview/export can be trusted.",
                "Add the missing block or complete its text.",
                &now,
            ));
        }
    }
    for block in &blocks {
        if is_factual_block(block) && !has_support_for_block(product, block) {
            findings.push(work_product_finding(
                product,
                "unsupported-factual-assertion",
                "support",
                if block_ai_generated(block) {
                    "serious"
                } else {
                    "warning"
                },
                "block",
                &block.block_id,
                "Factual assertion lacks linked support.",
                "Factual WorkProduct text should be linked to case facts or evidence before it is treated as court-ready.",
                "Link a supporting fact, evidence item, source document, or source span.",
                &now,
            ));
        }
    }
    for citation in &product.document_ast.citations {
        if matches!(
            citation.status.as_str(),
            "unresolved" | "ambiguous" | "stale" | "currentness_warning" | "needs_review"
        ) {
            findings.push(work_product_finding(
                product,
                "citation-needs-review",
                "citation",
                "warning",
                "citation",
                &citation.citation_use_id,
                "Citation needs review.",
                "The citation resolver could not mark this citation as fully resolved and current.",
                "Resolve the citation target or mark the citation review status intentionally.",
                &now,
            ));
        }
    }
    if product.product_type == "motion" {
        let has_relief = blocks.iter().any(|block| {
            block.role == "relief_requested" && block.text.split_whitespace().count() > 8
        });
        if !has_relief {
            findings.push(work_product_finding(
                product,
                "orcp-14-motion-writing-grounds-relief",
                "rules",
                "blocking",
                "block",
                "relief_requested",
                "Motion relief must be stated with particularity.",
                "ORCP 14 A requires the motion to set forth the relief or order sought.",
                "Complete the relief requested block.",
                &now,
            ));
        }
        let has_block_authority = blocks.iter().any(|block| {
            matches!(block.role.as_str(), "legal_standard" | "argument")
                && !block.authorities.is_empty()
        });
        let has_link_authority = product
            .document_ast
            .links
            .iter()
            .any(|link| matches!(link.target_type.as_str(), "authority" | "legal_authority"));
        let has_authority =
            has_block_authority || has_link_authority || !product.document_ast.citations.is_empty();
        if !has_authority {
            findings.push(work_product_finding(
                product,
                "utcr-5-020-authorities",
                "authority",
                "warning",
                "work_product",
                &product.work_product_id,
                "Motion has no linked authority.",
                "UTCR 5.020 and motion practice require human review of authorities.",
                "Link controlling authority in the legal-standard or argument block.",
                &now,
            ));
        }
        let has_conferral = blocks
            .iter()
            .any(|block| block.role == "conferral_certificate");
        if !has_conferral {
            findings.push(work_product_finding(
                product,
                "utcr-5-010-conferral",
                "rules",
                "warning",
                "work_product",
                &product.work_product_id,
                "Conferral requirement needs review.",
                "Some civil motions require conferral and a certificate under UTCR 5.010.",
                "Add a conferral certificate block or mark why it is not required.",
                &now,
            ));
        }
    }
    if !product.formatting_profile.double_spaced || !product.formatting_profile.line_numbers {
        findings.push(work_product_finding(
            product,
            "utcr-2-010-document-form",
            "formatting",
            "serious",
            "formatting",
            &product.formatting_profile.profile_id,
            "Document formatting requires review.",
            "UTCR 2.010 applies form requirements to motions and other court documents.",
            "Use court-paper formatting before export.",
            &now,
        ));
    }
    findings
}

fn is_factual_block(block: &WorkProductBlock) -> bool {
    let role = block.role.to_ascii_lowercase();
    let text_words = block.text.split_whitespace().count();
    text_words > 6
        && matches!(
            role.as_str(),
            "facts"
                | "factual_paragraph"
                | "factual_allegation"
                | "declaration_facts"
                | "background"
                | "factual_background"
        )
}

fn has_support_for_block(product: &WorkProduct, block: &WorkProductBlock) -> bool {
    !block.fact_ids.is_empty()
        || !block.evidence_ids.is_empty()
        || product.document_ast.links.iter().any(|link| {
            link.source_block_id == block.block_id
                && matches!(
                    link.target_type.as_str(),
                    "fact" | "evidence" | "document" | "source_span" | "exhibit"
                )
                && matches!(
                    link.relation.as_str(),
                    "supports" | "partially_supports" | "authenticates" | "references"
                )
        })
}

fn block_ai_generated(block: &WorkProductBlock) -> bool {
    block
        .provenance
        .as_ref()
        .and_then(|provenance| {
            provenance
                .get("source")
                .or_else(|| provenance.get("created_by"))
        })
        .map(|value| value.to_ascii_lowercase().contains("ai"))
        .unwrap_or(false)
}

pub(crate) fn work_product_finding(
    product: &WorkProduct,
    rule_id: &str,
    category: &str,
    severity: &str,
    target_type: &str,
    target_id: &str,
    message: &str,
    explanation: &str,
    suggested_fix: &str,
    now: &str,
) -> WorkProductFinding {
    let finding_id = format!(
        "{}:finding:{}:{}",
        product.work_product_id,
        sanitize_path_segment(rule_id),
        sanitize_path_segment(target_id)
    );
    WorkProductFinding {
        id: finding_id.clone(),
        finding_id,
        matter_id: product.matter_id.clone(),
        work_product_id: product.work_product_id.clone(),
        rule_id: rule_id.to_string(),
        rule_pack_id: Some(product.rule_pack.rule_pack_id.clone()),
        source_citation: None,
        source_url: None,
        category: category.to_string(),
        severity: severity.to_string(),
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        message: message.to_string(),
        explanation: explanation.to_string(),
        suggested_fix: suggested_fix.to_string(),
        auto_fix_available: false,
        primary_action: WorkProductAction {
            action_id: format!("action:{}", sanitize_path_segment(rule_id)),
            label: suggested_fix.to_string(),
            action_type: "open_editor".to_string(),
            href: None,
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
        },
        status: "open".to_string(),
        created_at: now.to_string(),
        updated_at: now.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::work_product_ast::{
        expected_formatting_profile_id, expected_profile_id, expected_rule_pack_id,
    };
    use super::*;
    use crate::models::casebuilder::{default_work_product_schema_version, RuleProfileSummary};

    fn test_product(blocks: Vec<WorkProductBlock>) -> WorkProduct {
        WorkProduct {
            work_product_id: "wp:test".to_string(),
            id: "wp:test".to_string(),
            matter_id: "matter:test".to_string(),
            title: "Test".to_string(),
            product_type: "memo".to_string(),
            status: "draft".to_string(),
            review_status: "needs_review".to_string(),
            setup_stage: "test".to_string(),
            source_draft_id: None,
            source_complaint_id: None,
            created_at: "1".to_string(),
            updated_at: "2".to_string(),
            profile: WorkProductProfile {
                profile_id: expected_profile_id("memo"),
                product_type: "memo".to_string(),
                name: "Memo".to_string(),
                jurisdiction: "Oregon".to_string(),
                version: "test".to_string(),
                route_slug: "memo".to_string(),
                required_block_roles: Vec::new(),
                optional_block_roles: Vec::new(),
                supports_rich_text: true,
            },
            document_ast: WorkProductDocument {
                schema_version: default_work_product_schema_version(),
                document_id: "wp:test:document".to_string(),
                work_product_id: "wp:test".to_string(),
                matter_id: "matter:test".to_string(),
                document_type: "memo".to_string(),
                product_type: "memo".to_string(),
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
                profile_id: expected_formatting_profile_id("memo"),
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
                rule_pack_id: expected_rule_pack_id("memo"),
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

    fn factual_block() -> WorkProductBlock {
        WorkProductBlock {
            block_id: "block:facts".to_string(),
            id: "block:facts".to_string(),
            matter_id: "matter:test".to_string(),
            work_product_id: "wp:test".to_string(),
            block_type: "section".to_string(),
            role: "facts".to_string(),
            title: "Facts".to_string(),
            text: "The tenant gave written notice and the landlord failed to repair the unit."
                .to_string(),
            ordinal: 1,
            ..WorkProductBlock::default()
        }
    }

    #[test]
    fn flags_unsupported_factual_blocks() {
        let product = test_product(vec![factual_block()]);
        let findings = work_product_findings(&product);
        assert!(findings
            .iter()
            .any(|finding| finding.rule_id == "unsupported-factual-assertion"));
    }

    #[test]
    fn support_link_satisfies_factual_block_support() {
        let mut product = test_product(vec![factual_block()]);
        product.document_ast.links.push(WorkProductLink {
            link_id: "link:1".to_string(),
            source_block_id: "block:facts".to_string(),
            source_text_range: None,
            target_type: "fact".to_string(),
            target_id: "fact:1".to_string(),
            relation: "supports".to_string(),
            confidence: Some(0.9),
            created_by: "test".to_string(),
            created_at: "1".to_string(),
        });
        let findings = work_product_findings(&product);
        assert!(!findings
            .iter()
            .any(|finding| finding.rule_id == "unsupported-factual-assertion"));
    }
}
