use crate::models::casebuilder::*;

pub(crate) fn legal_support_use_from_anchor(
    product: &WorkProduct,
    anchor: &WorkProductAnchor,
) -> LegalSupportUse {
    let source_type = match anchor.target_type.as_str() {
        "fact" => "fact",
        "evidence" => "evidence",
        "document" => "document",
        "source_span" => "source_span",
        "authority" | "provision" | "legal_text" => "authority",
        value => value,
    }
    .to_string();
    let source_id = anchor
        .canonical_id
        .clone()
        .filter(|_| matches!(source_type.as_str(), "authority" | "provision" | "citation"))
        .unwrap_or_else(|| anchor.target_id.clone());
    LegalSupportUse {
        id: anchor.anchor_id.clone(),
        support_use_id: anchor.anchor_id.clone(),
        matter_id: product.matter_id.clone(),
        subject_id: product.work_product_id.clone(),
        branch_id: format!("{}:branch:main", product.work_product_id),
        target_type: "block".to_string(),
        target_id: anchor.block_id.clone(),
        source_type,
        source_id,
        relation: anchor.relation.clone(),
        status: anchor.status.clone(),
        quote: anchor.quote.clone(),
        pinpoint: anchor.pinpoint.clone(),
        confidence: None,
        created_snapshot_id: String::new(),
        retired_snapshot_id: None,
    }
}

pub(crate) fn support_use_label(source_type: &str) -> &'static str {
    match source_type {
        "fact" => "FactUse",
        "authority" | "provision" | "citation" => "AuthorityUse",
        "element" => "ElementSupport",
        _ => "LegalSupportUse",
    }
}
