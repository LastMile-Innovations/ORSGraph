use crate::error::{ApiError, ApiResult};
use crate::models::casebuilder::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

struct ComparableLayerItem {
    layer: &'static str,
    target_type: String,
    target_id: String,
    title: String,
    summary: String,
    value: serde_json::Value,
}

pub(crate) fn diff_work_product_layers(
    from: &WorkProduct,
    to: &WorkProduct,
    layers: &[String],
) -> ApiResult<Vec<VersionLayerDiff>> {
    let mut diffs = Vec::new();
    if layers.iter().any(|layer| layer == "support") {
        diffs.extend(diff_layer_items(
            support_layer_items(from),
            support_layer_items(to),
        )?);
    }
    if layers.iter().any(|layer| layer == "citations") {
        diffs.extend(diff_layer_items(
            citation_layer_items(from),
            citation_layer_items(to),
        )?);
    }
    if layers.iter().any(|layer| layer == "exhibits") {
        diffs.extend(diff_layer_items(
            exhibit_layer_items(from),
            exhibit_layer_items(to),
        )?);
    }
    if layers.iter().any(|layer| layer == "rule_findings") {
        diffs.extend(diff_layer_items(
            rule_finding_layer_items(from),
            rule_finding_layer_items(to),
        )?);
    }
    if layers.iter().any(|layer| layer == "formatting") {
        diffs.extend(diff_layer_items(
            formatting_layer_items(from)?,
            formatting_layer_items(to)?,
        )?);
    }
    if layers.iter().any(|layer| layer == "exports") {
        diffs.extend(diff_layer_items(
            export_layer_items(from),
            export_layer_items(to),
        )?);
    }
    Ok(diffs)
}

fn diff_layer_items(
    from: Vec<ComparableLayerItem>,
    to: Vec<ComparableLayerItem>,
) -> ApiResult<Vec<VersionLayerDiff>> {
    let from_map = from
        .into_iter()
        .map(|item| (item.target_id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let to_map = to
        .into_iter()
        .map(|item| (item.target_id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let mut diffs = Vec::new();
    for (target_id, before) in &from_map {
        match to_map.get(target_id) {
            Some(after) => {
                let before_hash = json_hash(&before.value)?;
                let after_hash = json_hash(&after.value)?;
                if before_hash != after_hash {
                    diffs.push(VersionLayerDiff {
                        layer: before.layer.to_string(),
                        target_type: after.target_type.to_string(),
                        target_id: target_id.clone(),
                        title: after.title.clone(),
                        status: "modified".to_string(),
                        before_hash: Some(before_hash),
                        after_hash: Some(after_hash),
                        before_summary: Some(before.summary.clone()),
                        after_summary: Some(after.summary.clone()),
                    });
                }
            }
            None => diffs.push(VersionLayerDiff {
                layer: before.layer.to_string(),
                target_type: before.target_type.to_string(),
                target_id: target_id.clone(),
                title: before.title.clone(),
                status: "removed".to_string(),
                before_hash: Some(json_hash(&before.value)?),
                after_hash: None,
                before_summary: Some(before.summary.clone()),
                after_summary: None,
            }),
        }
    }
    for (target_id, after) in &to_map {
        if !from_map.contains_key(target_id) {
            diffs.push(VersionLayerDiff {
                layer: after.layer.to_string(),
                target_type: after.target_type.to_string(),
                target_id: target_id.clone(),
                title: after.title.clone(),
                status: "added".to_string(),
                before_hash: None,
                after_hash: Some(json_hash(&after.value)?),
                before_summary: None,
                after_summary: Some(after.summary.clone()),
            });
        }
    }
    Ok(diffs)
}

fn support_layer_items(product: &WorkProduct) -> Vec<ComparableLayerItem> {
    product
        .document_ast
        .links
        .iter()
        .map(|link| ComparableLayerItem {
            layer: "support",
            target_type: match link.target_type.as_str() {
                "authority" | "legal_authority" | "provision" | "legal_text" => "legal_authority",
                value => value,
            }
            .to_string(),
            target_id: link.link_id.clone(),
            title: "Support link".to_string(),
            summary: format!(
                "{} {} on {}",
                link.relation, link.target_type, link.source_block_id
            ),
            value: json!({
                "source_block_id": link.source_block_id,
                "target_type": link.target_type,
                "target_id": link.target_id,
                "relation": link.relation,
                "confidence": link.confidence,
            }),
        })
        .collect()
}

fn citation_layer_items(product: &WorkProduct) -> Vec<ComparableLayerItem> {
    product
        .document_ast
        .citations
        .iter()
        .map(|citation| ComparableLayerItem {
            layer: "citations",
            target_type: "citation".to_string(),
            target_id: citation.citation_use_id.clone(),
            title: "Citation use".to_string(),
            summary: format!(
                "{} citation on {}",
                citation.status, citation.source_block_id
            ),
            value: json!({
                "source_block_id": citation.source_block_id,
                "normalized_citation": citation.normalized_citation,
                "target_type": citation.target_type,
                "target_id": citation.target_id,
                "pinpoint": citation.pinpoint,
                "status": citation.status,
            }),
        })
        .collect()
}

fn exhibit_layer_items(product: &WorkProduct) -> Vec<ComparableLayerItem> {
    product
        .document_ast
        .exhibits
        .iter()
        .map(|exhibit| ComparableLayerItem {
            layer: "exhibits",
            target_type: "exhibit_reference".to_string(),
            target_id: exhibit.exhibit_reference_id.clone(),
            title: "Exhibit reference".to_string(),
            summary: format!("{} exhibit on {}", exhibit.status, exhibit.source_block_id),
            value: json!({
                "source_block_id": exhibit.source_block_id,
                "exhibit_id": exhibit.exhibit_id,
                "document_id": exhibit.document_id,
                "page_range": exhibit.page_range,
                "status": exhibit.status,
            }),
        })
        .collect()
}

fn rule_finding_layer_items(product: &WorkProduct) -> Vec<ComparableLayerItem> {
    product
        .document_ast
        .rule_findings
        .iter()
        .map(|finding| ComparableLayerItem {
            layer: "rule_findings",
            target_type: "rule_finding".to_string(),
            target_id: finding.finding_id.clone(),
            title: "Rule finding".to_string(),
            summary: format!(
                "{} {} finding on {}",
                finding.severity, finding.category, finding.target_id
            ),
            value: json!({
                "rule_id": finding.rule_id,
                "rule_pack_id": finding.rule_pack_id,
                "severity": finding.severity,
                "status": finding.status,
                "target_type": finding.target_type,
                "target_id": finding.target_id,
                "message": finding.message,
            }),
        })
        .collect()
}

fn formatting_layer_items(product: &WorkProduct) -> ApiResult<Vec<ComparableLayerItem>> {
    Ok(vec![ComparableLayerItem {
        layer: "formatting",
        target_type: "formatting".to_string(),
        target_id: product.formatting_profile.profile_id.clone(),
        title: "Formatting profile".to_string(),
        summary: product.formatting_profile.name.clone(),
        value: serde_json::to_value(&product.formatting_profile)
            .map_err(|error| ApiError::Internal(error.to_string()))?,
    }])
}

fn export_layer_items(product: &WorkProduct) -> Vec<ComparableLayerItem> {
    product
        .artifacts
        .iter()
        .map(|artifact| ComparableLayerItem {
            layer: "exports",
            target_type: "export".to_string(),
            target_id: artifact.artifact_id.clone(),
            title: format!("{} export", artifact.format.to_uppercase()),
            summary: artifact.status.clone(),
            value: json!({
                "format": artifact.format,
                "mode": artifact.mode,
                "profile": artifact.profile,
                "artifact_hash": artifact.artifact_hash,
                "snapshot_id": artifact.snapshot_id,
                "generated_at": artifact.generated_at,
            }),
        })
        .collect()
}

fn json_hash(value: &serde_json::Value) -> ApiResult<String> {
    let bytes = serde_json::to_vec(value).map_err(|error| ApiError::Internal(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok(out)
}
