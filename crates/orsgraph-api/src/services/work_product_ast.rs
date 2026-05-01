use crate::error::{ApiError, ApiResult};
use crate::models::casebuilder::*;
use crate::services::citation_resolver::work_product_citations_for_text;
use serde_json::json;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

pub const SUPPORTED_WORK_PRODUCT_TYPES: &[&str] = &[
    "complaint",
    "answer",
    "motion",
    "declaration",
    "affidavit",
    "memo",
    "notice",
    "letter",
    "exhibit_list",
    "proposed_order",
    "custom",
];

pub const SUPPORTED_BLOCK_TYPES: &[&str] = &[
    "caption",
    "heading",
    "section",
    "count",
    "paragraph",
    "numbered_paragraph",
    "sentence",
    "quote",
    "list",
    "table",
    "signature",
    "certificate",
    "exhibit_reference",
    "page_break",
    "markdown",
];

#[derive(Debug, Clone, Copy)]
pub(crate) struct AstBlockSpec {
    pub block_type: &'static str,
    pub requires_title: bool,
    pub allows_text: bool,
    pub allows_children: bool,
    pub requires_paragraph_number: bool,
    pub requires_sentence_id: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RequiredRoleSpec {
    pub role: &'static str,
    pub aliases: &'static [&'static str],
    pub allowed_block_types: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WorkProductTemplateSpec {
    pub template_id: &'static str,
    pub product_type: &'static str,
}

pub(crate) const REGISTERED_TEMPLATES: &[WorkProductTemplateSpec] = &[
    WorkProductTemplateSpec {
        template_id: "complaint-oregon-civil",
        product_type: "complaint",
    },
    WorkProductTemplateSpec {
        template_id: "answer-response-grid",
        product_type: "answer",
    },
    WorkProductTemplateSpec {
        template_id: "motion-standard",
        product_type: "motion",
    },
    WorkProductTemplateSpec {
        template_id: "declaration-support",
        product_type: "declaration",
    },
    WorkProductTemplateSpec {
        template_id: "legal-memo",
        product_type: "memo",
    },
    WorkProductTemplateSpec {
        template_id: "demand-letter",
        product_type: "letter",
    },
    WorkProductTemplateSpec {
        template_id: "notice",
        product_type: "notice",
    },
    WorkProductTemplateSpec {
        template_id: "exhibit-list",
        product_type: "exhibit_list",
    },
    WorkProductTemplateSpec {
        template_id: "proposed-order",
        product_type: "proposed_order",
    },
];

const COMPLAINT_REQUIRED_ROLES: &[RequiredRoleSpec] = &[
    RequiredRoleSpec {
        role: "caption",
        aliases: &["caption"],
        allowed_block_types: &["caption", "section"],
    },
    RequiredRoleSpec {
        role: "jurisdiction",
        aliases: &["jurisdiction", "jurisdiction_venue", "venue"],
        allowed_block_types: &["section", "paragraph", "numbered_paragraph"],
    },
    RequiredRoleSpec {
        role: "facts",
        aliases: &[
            "facts",
            "factual_paragraph",
            "factual_allegation",
            "background",
        ],
        allowed_block_types: &["section", "paragraph", "numbered_paragraph"],
    },
    RequiredRoleSpec {
        role: "count",
        aliases: &["count", "claim", "claims", "claim_for_relief"],
        allowed_block_types: &["count", "section"],
    },
    RequiredRoleSpec {
        role: "relief",
        aliases: &["relief", "prayer", "prayer_for_relief"],
        allowed_block_types: &["section", "paragraph", "numbered_paragraph"],
    },
    RequiredRoleSpec {
        role: "signature",
        aliases: &["signature", "signature_block"],
        allowed_block_types: &["signature", "section", "paragraph"],
    },
];

const MOTION_REQUIRED_ROLES: &[RequiredRoleSpec] = &[
    RequiredRoleSpec {
        role: "notice_motion",
        aliases: &["notice_motion", "motion_notice", "heading"],
        allowed_block_types: &["heading", "section"],
    },
    RequiredRoleSpec {
        role: "relief_requested",
        aliases: &["relief_requested", "relief", "requested_relief"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "legal_standard",
        aliases: &["legal_standard", "standard"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "argument",
        aliases: &["argument", "analysis"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "conclusion",
        aliases: &["conclusion"],
        allowed_block_types: &["section", "paragraph"],
    },
];

const ANSWER_REQUIRED_ROLES: &[RequiredRoleSpec] = &[
    RequiredRoleSpec {
        role: "responses",
        aliases: &["responses", "allegation_responses"],
        allowed_block_types: &["section", "paragraph", "table"],
    },
    RequiredRoleSpec {
        role: "affirmative_defenses",
        aliases: &["affirmative_defenses", "defenses"],
        allowed_block_types: &["section", "paragraph", "list"],
    },
    RequiredRoleSpec {
        role: "prayer",
        aliases: &["prayer", "relief", "prayer_for_relief"],
        allowed_block_types: &["section", "paragraph"],
    },
];

const DECLARATION_REQUIRED_ROLES: &[RequiredRoleSpec] = &[
    RequiredRoleSpec {
        role: "declarant",
        aliases: &["declarant", "affiant", "witness"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "facts",
        aliases: &["facts", "declaration_facts", "factual_statement"],
        allowed_block_types: &["section", "paragraph", "numbered_paragraph"],
    },
    RequiredRoleSpec {
        role: "signature",
        aliases: &["signature", "signature_block"],
        allowed_block_types: &["signature", "section", "paragraph"],
    },
];

const MEMO_REQUIRED_ROLES: &[RequiredRoleSpec] = &[
    RequiredRoleSpec {
        role: "question",
        aliases: &["question", "question_presented"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "brief_answer",
        aliases: &["brief_answer", "short_answer"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "facts",
        aliases: &["facts", "relevant_facts"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "analysis",
        aliases: &["analysis", "argument"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "conclusion",
        aliases: &["conclusion"],
        allowed_block_types: &["section", "paragraph"],
    },
];

const NOTICE_LETTER_REQUIRED_ROLES: &[RequiredRoleSpec] = &[
    RequiredRoleSpec {
        role: "recipient",
        aliases: &["recipient", "addressee"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "purpose",
        aliases: &["purpose", "subject"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "body",
        aliases: &["body", "notice_body", "letter_body"],
        allowed_block_types: &["section", "paragraph", "markdown"],
    },
    RequiredRoleSpec {
        role: "signature",
        aliases: &["signature", "signature_block"],
        allowed_block_types: &["signature", "section", "paragraph"],
    },
];

const PROPOSED_ORDER_REQUIRED_ROLES: &[RequiredRoleSpec] = &[
    RequiredRoleSpec {
        role: "caption",
        aliases: &["caption"],
        allowed_block_types: &["caption", "section"],
    },
    RequiredRoleSpec {
        role: "findings",
        aliases: &["findings", "recitals"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "order",
        aliases: &["order", "ordered_relief"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "signature",
        aliases: &["signature", "judge_signature"],
        allowed_block_types: &["signature", "section", "paragraph"],
    },
];

const EXHIBIT_LIST_REQUIRED_ROLES: &[RequiredRoleSpec] = &[RequiredRoleSpec {
    role: "exhibits",
    aliases: &["exhibits", "exhibit_list"],
    allowed_block_types: &["section", "list", "table", "exhibit_reference"],
}];

const CUSTOM_REQUIRED_ROLES: &[RequiredRoleSpec] = &[
    RequiredRoleSpec {
        role: "summary",
        aliases: &["summary", "overview"],
        allowed_block_types: &["section", "paragraph", "markdown"],
    },
    RequiredRoleSpec {
        role: "facts",
        aliases: &["facts", "relevant_facts"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "analysis",
        aliases: &["analysis", "argument"],
        allowed_block_types: &["section", "paragraph"],
    },
    RequiredRoleSpec {
        role: "conclusion",
        aliases: &["conclusion"],
        allowed_block_types: &["section", "paragraph"],
    },
];

pub(crate) fn ast_block_spec(block_type: &str) -> Option<AstBlockSpec> {
    match block_type.trim().to_ascii_lowercase().as_str() {
        "caption" => Some(AstBlockSpec {
            block_type: "caption",
            requires_title: false,
            allows_text: true,
            allows_children: true,
            requires_paragraph_number: false,
            requires_sentence_id: false,
        }),
        "heading" => Some(AstBlockSpec {
            block_type: "heading",
            requires_title: true,
            allows_text: true,
            allows_children: false,
            requires_paragraph_number: false,
            requires_sentence_id: false,
        }),
        "section" => Some(AstBlockSpec {
            block_type: "section",
            requires_title: true,
            allows_text: true,
            allows_children: true,
            requires_paragraph_number: false,
            requires_sentence_id: false,
        }),
        "count" => Some(AstBlockSpec {
            block_type: "count",
            requires_title: true,
            allows_text: true,
            allows_children: true,
            requires_paragraph_number: false,
            requires_sentence_id: false,
        }),
        "paragraph" | "quote" | "markdown" => {
            let normalized = block_type.trim().to_ascii_lowercase();
            Some(AstBlockSpec {
                block_type: if normalized == "quote" {
                    "quote"
                } else if normalized == "markdown" {
                    "markdown"
                } else {
                    "paragraph"
                },
                requires_title: false,
                allows_text: true,
                allows_children: false,
                requires_paragraph_number: false,
                requires_sentence_id: false,
            })
        }
        "numbered_paragraph" => Some(AstBlockSpec {
            block_type: "numbered_paragraph",
            requires_title: false,
            allows_text: true,
            allows_children: false,
            requires_paragraph_number: true,
            requires_sentence_id: false,
        }),
        "sentence" => Some(AstBlockSpec {
            block_type: "sentence",
            requires_title: false,
            allows_text: true,
            allows_children: false,
            requires_paragraph_number: false,
            requires_sentence_id: true,
        }),
        "list" | "table" => {
            let normalized = block_type.trim().to_ascii_lowercase();
            Some(AstBlockSpec {
                block_type: if normalized == "table" {
                    "table"
                } else {
                    "list"
                },
                requires_title: false,
                allows_text: true,
                allows_children: true,
                requires_paragraph_number: false,
                requires_sentence_id: false,
            })
        }
        "signature" | "certificate" | "exhibit_reference" => Some(AstBlockSpec {
            block_type: match block_type {
                "certificate" => "certificate",
                "exhibit_reference" => "exhibit_reference",
                _ => "signature",
            },
            requires_title: false,
            allows_text: true,
            allows_children: false,
            requires_paragraph_number: false,
            requires_sentence_id: false,
        }),
        "page_break" => Some(AstBlockSpec {
            block_type: "page_break",
            requires_title: false,
            allows_text: false,
            allows_children: false,
            requires_paragraph_number: false,
            requires_sentence_id: false,
        }),
        _ => None,
    }
}

pub(crate) fn required_role_specs_for_work_product(
    product_type: &str,
) -> &'static [RequiredRoleSpec] {
    match normalize_work_product_type_lossy(product_type).as_str() {
        "complaint" => COMPLAINT_REQUIRED_ROLES,
        "motion" => MOTION_REQUIRED_ROLES,
        "answer" => ANSWER_REQUIRED_ROLES,
        "declaration" | "affidavit" => DECLARATION_REQUIRED_ROLES,
        "memo" => MEMO_REQUIRED_ROLES,
        "notice" | "letter" => NOTICE_LETTER_REQUIRED_ROLES,
        "proposed_order" => PROPOSED_ORDER_REQUIRED_ROLES,
        "exhibit_list" => EXHIBIT_LIST_REQUIRED_ROLES,
        _ => CUSTOM_REQUIRED_ROLES,
    }
}

pub(crate) fn role_spec_matches_block(spec: &RequiredRoleSpec, block: &WorkProductBlock) -> bool {
    let role = canonical_role_key(&block.role);
    let role_matches = spec
        .aliases
        .iter()
        .any(|alias| canonical_role_key(alias) == role);
    let type_matches = spec.allowed_block_types.is_empty()
        || spec
            .allowed_block_types
            .iter()
            .any(|block_type| block.block_type == *block_type);
    role_matches && type_matches && !block.text.trim().is_empty()
}

pub(crate) fn registered_template_product_type(template_id: &str) -> Option<&'static str> {
    REGISTERED_TEMPLATES
        .iter()
        .find(|template| template.template_id == template_id)
        .map(|template| template.product_type)
}

pub(crate) fn expected_profile_id(product_type: &str) -> String {
    format!(
        "work-product-{}-v1",
        normalize_work_product_type_lossy(product_type)
    )
}

pub(crate) fn expected_formatting_profile_id(product_type: &str) -> String {
    format!(
        "oregon-circuit-civil-{}",
        normalize_work_product_type_lossy(product_type)
    )
}

pub(crate) fn expected_rule_pack_id(product_type: &str) -> String {
    let product_type = normalize_work_product_type_lossy(product_type);
    match product_type.as_str() {
        "complaint" => "oregon-circuit-civil-complaint-orcp-utcr".to_string(),
        "motion" => "oregon-circuit-civil-motion-orcp-utcr".to_string(),
        other => format!("oregon-circuit-civil-{other}-baseline"),
    }
}

fn canonical_role_key(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_")
}

pub(crate) fn now_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

pub(crate) fn canonical_work_product_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "complaint" => Some("complaint"),
        "answer" => Some("answer"),
        "motion" => Some("motion"),
        "declaration" => Some("declaration"),
        "affidavit" => Some("affidavit"),
        "memo" | "legal_memo" | "brief" => Some("memo"),
        "notice" => Some("notice"),
        "letter" | "demand_letter" => Some("letter"),
        "exhibit_list" => Some("exhibit_list"),
        "proposed_order" => Some("proposed_order"),
        "custom" => Some("custom"),
        _ => None,
    }
}

pub(crate) fn normalize_work_product_type_lossy(value: &str) -> String {
    canonical_work_product_type(value)
        .unwrap_or("custom")
        .to_string()
}

pub(crate) fn sanitize_path_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if sanitized.is_empty() {
        "item".to_string()
    } else {
        sanitized
    }
}

pub(crate) fn humanize_product_type(value: &str) -> String {
    value
        .replace('_', " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn prosemirror_doc_for_text(text: &str) -> serde_json::Value {
    json!({
        "type": "doc",
        "content": text
            .split("\n\n")
            .filter(|part| !part.trim().is_empty())
            .map(|part| json!({
                "type": "paragraph",
                "content": [{ "type": "text", "text": part.trim() }]
            }))
            .collect::<Vec<_>>()
    })
}

pub(crate) fn validate_optional_text_range(text: &str, range: Option<&TextRange>) -> ApiResult<()> {
    let Some(range) = range else {
        return Ok(());
    };
    let len = text.chars().count() as u64;
    if range.start_offset > range.end_offset || range.end_offset > len {
        return Err(ApiError::BadRequest(
            "AST text range is outside the source block.".to_string(),
        ));
    }
    if let Some(quote) = range.quote.as_deref() {
        let selected = text
            .chars()
            .skip(range.start_offset as usize)
            .take((range.end_offset - range.start_offset) as usize)
            .collect::<String>();
        if !quote.is_empty() && selected != quote {
            return Err(ApiError::BadRequest(
                "AST text range quote does not match the source block.".to_string(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

pub(crate) fn rebuild_work_product_ast_from_projection(product: &mut WorkProduct) {
    let blocks = product.blocks.clone();
    product.document_ast = work_product_document_from_projection(product, blocks);
}

pub(crate) fn normalize_work_product_ast(product: &mut WorkProduct) {
    let now = if product.updated_at.is_empty() {
        now_string()
    } else {
        product.updated_at.clone()
    };
    let document_type_source = if product.document_ast.document_type.trim().is_empty()
        || product.document_ast.document_type == "custom"
    {
        product.product_type.as_str()
    } else {
        product.document_ast.document_type.as_str()
    };
    let document_type = normalize_work_product_type_lossy(document_type_source);
    product.product_type = document_type.clone();
    if product.document_ast.schema_version.trim().is_empty() {
        product.document_ast.schema_version = default_work_product_schema_version();
    }
    if product.document_ast.document_id.trim().is_empty() {
        product.document_ast.document_id = format!("{}:document", product.work_product_id);
    }
    product.document_ast.work_product_id = product.work_product_id.clone();
    product.document_ast.matter_id = product.matter_id.clone();
    product.document_ast.draft_id = product.source_draft_id.clone();
    product.document_ast.document_type = document_type.clone();
    product.document_ast.product_type = document_type.clone();
    product.document_ast.title = product.title.clone();
    product.document_ast.metadata.work_product_type = Some(document_type);
    product.document_ast.metadata.document_title = Some(product.title.clone());
    product.document_ast.metadata.status = product.status.clone();
    product.document_ast.metadata.rule_pack_id = Some(product.rule_pack.rule_pack_id.clone());
    product.document_ast.metadata.formatting_profile_id =
        Some(product.formatting_profile.profile_id.clone());
    if product.document_ast.metadata.created_at.is_none() && !product.created_at.is_empty() {
        product.document_ast.metadata.created_at = Some(product.created_at.clone());
    }
    product.document_ast.metadata.updated_at = Some(now.clone());
    if product.document_ast.created_at.is_empty() {
        product.document_ast.created_at = product.created_at.clone();
    }
    product.document_ast.updated_at = now.clone();
    if product.document_ast.rule_findings.is_empty() && !product.findings.is_empty() {
        product.document_ast.rule_findings = product.findings.clone();
    }
    product.findings = product.document_ast.rule_findings.clone();
    for block in &mut product.document_ast.blocks {
        normalize_ast_block(block, &product.matter_id, &product.work_product_id, &now);
    }
    sync_inferred_citations(&mut product.document_ast);
    sync_rule_finding_refs(&mut product.document_ast);
    for block in &mut product.document_ast.tombstones {
        normalize_ast_block(block, &product.matter_id, &product.work_product_id, &now);
        block.tombstoned = true;
    }
    product.blocks = flatten_work_product_blocks(&product.document_ast.blocks);
}

fn normalize_ast_block(
    block: &mut WorkProductBlock,
    matter_id: &str,
    work_product_id: &str,
    now: &str,
) {
    if block.block_id.is_empty() {
        block.block_id = block.id.clone();
    }
    if block.id.is_empty() {
        block.id = block.block_id.clone();
    }
    block.matter_id = matter_id.to_string();
    block.work_product_id = work_product_id.to_string();
    if block.created_at.is_empty() {
        block.created_at = now.to_string();
    }
    block.updated_at = now.to_string();
    if block.review_status.is_empty() {
        block.review_status = "needs_review".to_string();
    }
    if block.block_type.is_empty() {
        block.block_type = "paragraph".to_string();
    }
    if block.block_type == "sentence" && block.sentence_id.is_none() {
        block.sentence_id = Some(format!("{}:sentence:{}", block.block_id, block.ordinal));
    }
    for child in &mut block.children {
        child.parent_block_id = Some(block.block_id.clone());
        normalize_ast_block(child, matter_id, work_product_id, now);
    }
}

fn sync_inferred_citations(document: &mut WorkProductDocument) {
    let mut existing = document
        .citations
        .iter()
        .map(citation_match_key)
        .collect::<HashSet<_>>();
    let mut inferred = Vec::new();
    sync_inferred_citations_for_blocks(
        &mut document.blocks,
        &document.work_product_id,
        &document.updated_at,
        &mut existing,
        &mut inferred,
    );
    document.citations.extend(inferred);
}

fn sync_inferred_citations_for_blocks(
    blocks: &mut [WorkProductBlock],
    work_product_id: &str,
    created_at: &str,
    existing: &mut HashSet<String>,
    inferred: &mut Vec<WorkProductCitationUse>,
) {
    for block in blocks {
        for citation in work_product_citations_for_text(
            work_product_id,
            &block.block_id,
            &block.text,
            created_at,
        ) {
            let key = citation_match_key(&citation);
            if existing.insert(key) {
                push_unique(&mut block.citations, citation.citation_use_id.clone());
                inferred.push(citation);
            }
        }
        sync_inferred_citations_for_blocks(
            &mut block.children,
            work_product_id,
            created_at,
            existing,
            inferred,
        );
    }
}

fn citation_match_key(citation: &WorkProductCitationUse) -> String {
    let range = citation
        .source_text_range
        .as_ref()
        .map(|range| format!("{}:{}", range.start_offset, range.end_offset))
        .unwrap_or_else(|| "block".to_string());
    format!(
        "{}:{}:{}",
        citation.source_block_id,
        range,
        citation.raw_text.trim().to_ascii_uppercase()
    )
}

fn sync_rule_finding_refs(document: &mut WorkProductDocument) {
    let refs = document
        .rule_findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.target_type.as_str(),
                "block" | "paragraph" | "section" | "count" | "caption" | "sentence"
            )
        })
        .map(|finding| (finding.target_id.clone(), finding.finding_id.clone()))
        .collect::<Vec<_>>();
    for (block_id, finding_id) in refs {
        if let Some(block) = find_ast_block_mut(&mut document.blocks, &block_id) {
            push_unique(&mut block.rule_finding_ids, finding_id);
        }
    }
}

pub(crate) fn work_product_document_from_projection(
    product: &WorkProduct,
    blocks: Vec<WorkProductBlock>,
) -> WorkProductDocument {
    let now = if product.updated_at.is_empty() {
        product.created_at.clone()
    } else {
        product.updated_at.clone()
    };
    let mut flat_blocks = blocks
        .into_iter()
        .enumerate()
        .map(|(index, mut block)| {
            if block.block_id.is_empty() {
                block.block_id = block.id.clone();
            }
            if block.id.is_empty() {
                block.id = block.block_id.clone();
            }
            if block.created_at.is_empty() {
                block.created_at = product.created_at.clone();
            }
            block.updated_at = now.clone();
            block.ordinal = if block.ordinal == 0 {
                index as u64 + 1
            } else {
                block.ordinal
            };
            block.children.clear();
            block.links.clear();
            block.citations.clear();
            block.exhibits.clear();
            block.rule_finding_ids = product
                .findings
                .iter()
                .filter(|finding| finding.target_id == block.block_id)
                .map(|finding| finding.finding_id.clone())
                .collect();
            block
        })
        .collect::<Vec<_>>();
    flat_blocks.sort_by_key(|block| block.ordinal);

    let mut links = Vec::new();
    let mut citations = Vec::new();
    let exhibits = product.document_ast.exhibits.clone();
    for block in &mut flat_blocks {
        for fact_id in &block.fact_ids {
            let link_id = format!(
                "{}:link:fact:{}",
                block.block_id,
                sanitize_path_segment(fact_id)
            );
            push_unique(&mut block.links, link_id.clone());
            links.push(WorkProductLink {
                link_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                target_type: "fact".to_string(),
                target_id: fact_id.clone(),
                relation: "supports".to_string(),
                confidence: None,
                created_by: "system".to_string(),
                created_at: now.clone(),
            });
        }
        for evidence_id in &block.evidence_ids {
            let link_id = format!(
                "{}:link:evidence:{}",
                block.block_id,
                sanitize_path_segment(evidence_id)
            );
            push_unique(&mut block.links, link_id.clone());
            links.push(WorkProductLink {
                link_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                target_type: "evidence".to_string(),
                target_id: evidence_id.clone(),
                relation: "supports".to_string(),
                confidence: None,
                created_by: "system".to_string(),
                created_at: now.clone(),
            });
        }
        for authority in &block.authorities {
            let link_id = format!(
                "{}:link:authority:{}",
                block.block_id,
                sanitize_path_segment(&authority.canonical_id)
            );
            let citation_use_id = format!(
                "{}:citation:{}",
                block.block_id,
                sanitize_path_segment(&authority.citation)
            );
            push_unique(&mut block.links, link_id.clone());
            push_unique(&mut block.citations, citation_use_id.clone());
            links.push(WorkProductLink {
                link_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                target_type: "legal_authority".to_string(),
                target_id: authority.canonical_id.clone(),
                relation: "cites".to_string(),
                confidence: None,
                created_by: "system".to_string(),
                created_at: now.clone(),
            });
            citations.push(WorkProductCitationUse {
                citation_use_id,
                source_block_id: block.block_id.clone(),
                source_text_range: None,
                raw_text: authority.citation.clone(),
                normalized_citation: Some(authority.citation.clone()),
                target_type: "provision".to_string(),
                target_id: Some(authority.canonical_id.clone()),
                pinpoint: authority.pinpoint.clone(),
                status: "resolved".to_string(),
                resolver_message: authority.reason.clone(),
                created_at: now.clone(),
            });
        }
    }

    for anchor in &product.anchors {
        let link_id = anchor.anchor_id.clone();
        links.push(WorkProductLink {
            link_id: link_id.clone(),
            source_block_id: anchor.block_id.clone(),
            source_text_range: anchor.quote.as_ref().map(|quote| TextRange {
                start_offset: 0,
                end_offset: quote.chars().count() as u64,
                quote: Some(quote.clone()),
            }),
            target_type: anchor.target_type.clone(),
            target_id: anchor.target_id.clone(),
            relation: anchor.relation.clone(),
            confidence: None,
            created_by: "user".to_string(),
            created_at: now.clone(),
        });
        if let Some(block) = flat_blocks
            .iter_mut()
            .find(|block| block.block_id == anchor.block_id)
        {
            push_unique(&mut block.links, link_id.clone());
            if anchor.anchor_type == "authority" || anchor.citation.is_some() {
                let citation_use_id = format!("{link_id}:citation");
                push_unique(&mut block.citations, citation_use_id.clone());
                citations.push(WorkProductCitationUse {
                    citation_use_id,
                    source_block_id: anchor.block_id.clone(),
                    source_text_range: None,
                    raw_text: anchor
                        .citation
                        .clone()
                        .unwrap_or_else(|| anchor.target_id.clone()),
                    normalized_citation: anchor.citation.clone(),
                    target_type: "provision".to_string(),
                    target_id: anchor
                        .canonical_id
                        .clone()
                        .or_else(|| Some(anchor.target_id.clone())),
                    pinpoint: anchor.pinpoint.clone(),
                    status: if anchor.status == "resolved" {
                        "resolved".to_string()
                    } else {
                        "needs_review".to_string()
                    },
                    resolver_message: None,
                    created_at: now.clone(),
                });
            }
        }
    }

    WorkProductDocument {
        schema_version: default_work_product_schema_version(),
        document_id: format!("{}:document", product.work_product_id),
        work_product_id: product.work_product_id.clone(),
        matter_id: product.matter_id.clone(),
        draft_id: product.source_draft_id.clone(),
        document_type: product.product_type.clone(),
        product_type: product.product_type.clone(),
        title: product.title.clone(),
        metadata: WorkProductMetadata {
            work_product_type: Some(product.product_type.clone()),
            document_title: Some(product.title.clone()),
            jurisdiction: Some(product.profile.jurisdiction.clone()),
            court: product.document_ast.metadata.court.clone(),
            county: product.document_ast.metadata.county.clone(),
            case_number: product.document_ast.metadata.case_number.clone(),
            rule_pack_id: Some(product.rule_pack.rule_pack_id.clone()),
            template_id: product.document_ast.metadata.template_id.clone(),
            formatting_profile_id: Some(product.formatting_profile.profile_id.clone()),
            parties: product.document_ast.metadata.parties.clone(),
            status: product.status.clone(),
            created_at: Some(product.created_at.clone()),
            updated_at: Some(now.clone()),
            created_by: product.document_ast.metadata.created_by.clone(),
            last_modified_by: product.document_ast.metadata.last_modified_by.clone(),
        },
        blocks: build_work_product_block_tree(&flat_blocks),
        links,
        citations,
        exhibits,
        rule_findings: product.findings.clone(),
        tombstones: product.document_ast.tombstones.clone(),
        created_at: product.created_at.clone(),
        updated_at: now,
    }
}

pub(crate) fn build_work_product_block_tree(
    flat_blocks: &[WorkProductBlock],
) -> Vec<WorkProductBlock> {
    let ids = flat_blocks
        .iter()
        .map(|block| block.block_id.clone())
        .collect::<HashSet<_>>();
    flat_blocks
        .iter()
        .filter(|block| {
            block
                .parent_block_id
                .as_ref()
                .map(|parent_id| !ids.contains(parent_id))
                .unwrap_or(true)
        })
        .cloned()
        .map(|mut block| {
            attach_work_product_children(&mut block, flat_blocks);
            block
        })
        .collect()
}

fn attach_work_product_children(block: &mut WorkProductBlock, flat_blocks: &[WorkProductBlock]) {
    block.children = flat_blocks
        .iter()
        .filter(|candidate| candidate.parent_block_id.as_deref() == Some(&block.block_id))
        .cloned()
        .map(|mut child| {
            attach_work_product_children(&mut child, flat_blocks);
            child
        })
        .collect();
}

pub(crate) fn flatten_work_product_blocks(blocks: &[WorkProductBlock]) -> Vec<WorkProductBlock> {
    let mut flattened = Vec::new();
    for block in blocks {
        flatten_work_product_block(block, &mut flattened);
    }
    flattened.sort_by_key(|block| block.ordinal);
    flattened
}

fn flatten_work_product_block(block: &WorkProductBlock, flattened: &mut Vec<WorkProductBlock>) {
    let mut current = block.clone();
    current.children.clear();
    flattened.push(current);
    for child in &block.children {
        flatten_work_product_block(child, flattened);
    }
}

pub(crate) fn canonical_work_product_blocks(product: &WorkProduct) -> Vec<WorkProductBlock> {
    flatten_work_product_blocks(&product.document_ast.blocks)
}

pub(crate) fn find_ast_block<'a>(
    blocks: &'a [WorkProductBlock],
    block_id: &str,
) -> Option<&'a WorkProductBlock> {
    for block in blocks {
        if block.block_id == block_id {
            return Some(block);
        }
        if let Some(found) = find_ast_block(&block.children, block_id) {
            return Some(found);
        }
    }
    None
}

pub(crate) fn find_ast_block_mut<'a>(
    blocks: &'a mut [WorkProductBlock],
    block_id: &str,
) -> Option<&'a mut WorkProductBlock> {
    for block in blocks {
        if block.block_id == block_id {
            return Some(block);
        }
        if let Some(found) = find_ast_block_mut(&mut block.children, block_id) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_tree_round_trip_keeps_children() {
        let parent = WorkProductBlock {
            block_id: "p".to_string(),
            id: "p".to_string(),
            ordinal: 1,
            ..WorkProductBlock::default()
        };
        let child = WorkProductBlock {
            block_id: "c".to_string(),
            id: "c".to_string(),
            parent_block_id: Some("p".to_string()),
            ordinal: 2,
            ..WorkProductBlock::default()
        };
        let tree = build_work_product_block_tree(&[parent, child]);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].children[0].block_id, "c");
        assert_eq!(flatten_work_product_blocks(&tree).len(), 2);
    }

    #[test]
    fn normalize_infers_citation_uses_from_block_text_once() {
        let block = WorkProductBlock {
            block_id: "wp:test:block:1".to_string(),
            id: "wp:test:block:1".to_string(),
            matter_id: "matter:test".to_string(),
            work_product_id: "wp:test".to_string(),
            block_type: "paragraph".to_string(),
            role: "argument".to_string(),
            text: "ORS 90.320 supplies the standard.".to_string(),
            ordinal: 1,
            ..WorkProductBlock::default()
        };
        let mut product = WorkProduct {
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
                blocks: vec![block],
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
        };
        normalize_work_product_ast(&mut product);
        normalize_work_product_ast(&mut product);
        assert_eq!(product.document_ast.citations.len(), 1);
        assert_eq!(product.document_ast.citations[0].status, "resolved");
        assert_eq!(product.document_ast.blocks[0].citations.len(), 1);
    }
}
