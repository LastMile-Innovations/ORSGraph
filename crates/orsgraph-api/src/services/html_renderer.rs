use crate::error::ApiResult;
use crate::models::casebuilder::*;
use crate::services::markdown_adapter::work_product_markdown;
use crate::services::work_product_ast::{canonical_work_product_blocks, now_string};

pub(crate) fn render_work_product_preview(product: &WorkProduct) -> WorkProductPreviewResponse {
    let mut html = String::new();
    let blocks = canonical_work_product_blocks(product);
    html.push_str("<article class=\"work-product-preview\" data-renderer=\"work-product-ast-v1\">");
    html.push_str(&format!(
        "<header data-document-id=\"{}\"><p>{}</p><h1>{}</h1><p class=\"review\">Review needed - not legal advice or filing-ready.</p></header>",
        escape_html(&product.document_ast.document_id),
        escape_html(&product.profile.name),
        escape_html(&product.title)
    ));
    for block in &blocks {
        html.push_str(&render_block_html(block));
    }
    html.push_str("</article>");
    let plain_text = work_product_plain_text(product);
    WorkProductPreviewResponse {
        work_product_id: product.work_product_id.clone(),
        matter_id: product.matter_id.clone(),
        html,
        plain_text,
        page_count: ((count_work_product_words(product) / 450) + 1).max(1),
        warnings: work_product_export_warnings(product, "preview", false, true),
        generated_at: now_string(),
        review_label: "Review needed; not legal advice or filing-ready.".to_string(),
    }
}

fn render_block_html(block: &WorkProductBlock) -> String {
    match block.block_type.as_str() {
        "page_break" => format!(
            "<div class=\"page-break\" data-block-id=\"{}\"></div>",
            escape_html(&block.block_id)
        ),
        "quote" => format!(
            "<blockquote data-block-id=\"{}\">{}</blockquote>",
            escape_html(&block.block_id),
            escape_html(&block.text).replace('\n', "<br />")
        ),
        "numbered_paragraph" => format!(
            "<p data-block-id=\"{}\" data-paragraph-number=\"{}\"><span class=\"paragraph-number\">{}.</span> {}</p>",
            escape_html(&block.block_id),
            block.paragraph_number.unwrap_or(block.ordinal),
            block.paragraph_number.unwrap_or(block.ordinal),
            escape_html(&block.text).replace('\n', "<br />")
        ),
        "caption" => format!(
            "<section data-block-id=\"{}\" class=\"caption\"><h2>{}</h2><p>{}</p></section>",
            escape_html(&block.block_id),
            escape_html(&block.title),
            escape_html(&block.text).replace('\n', "<br />")
        ),
        _ => format!(
            "<section data-block-id=\"{}\" data-block-type=\"{}\"><h2>{}</h2><p>{}</p></section>",
            escape_html(&block.block_id),
            escape_html(&block.block_type),
            escape_html(&block.title),
            escape_html(&block.text).replace('\n', "<br />")
        ),
    }
}

pub(crate) fn work_product_plain_text(product: &WorkProduct) -> String {
    let mut lines = vec![product.title.clone()];
    for block in canonical_work_product_blocks(product) {
        lines.push(String::new());
        if block.block_type == "numbered_paragraph" {
            lines.push(format!(
                "{}. {}",
                block.paragraph_number.unwrap_or(block.ordinal),
                block.text
            ));
        } else if block.block_type == "page_break" {
            lines.push("[page break]".to_string());
        } else {
            lines.push(block.title.clone());
            if !block.text.is_empty() && block.text != block.title {
                lines.push(block.text.clone());
            }
        }
    }
    lines.push(String::new());
    lines.push("Review needed; not legal advice or filing-ready.".to_string());
    lines.join("\n")
}

pub(crate) fn render_work_product_export_content(
    product: &WorkProduct,
    format: &str,
) -> ApiResult<String> {
    Ok(match format {
        "html" => render_work_product_preview(product).html,
        "json" => serde_json::to_string_pretty(&product.document_ast)
            .map_err(|error| crate::error::ApiError::Internal(error.to_string()))?,
        "markdown" => work_product_markdown(product),
        "text" | "plain_text" => work_product_plain_text(product),
        "pdf" => super::pdf_renderer::render_pdf_placeholder(product),
        "docx" => super::docx_renderer::render_docx_placeholder(product),
        _ => work_product_plain_text(product),
    })
}

pub(crate) fn work_product_export_warnings(
    product: &WorkProduct,
    format: &str,
    include_exhibits: bool,
    include_qc_report: bool,
) -> Vec<String> {
    let mut warnings =
        vec!["Review needed; generated checks and exports are not legal advice.".to_string()];
    if product
        .findings
        .iter()
        .any(|finding| finding.status == "open")
    {
        warnings.push("Open QC findings remain.".to_string());
    }
    if product
        .findings
        .iter()
        .any(|finding| finding.status == "open" && finding.severity == "blocking")
    {
        warnings.push("Blocking QC findings remain; export is not court-ready.".to_string());
    }
    if product.document_ast.citations.iter().any(|citation| {
        matches!(
            citation.status.as_str(),
            "unresolved" | "ambiguous" | "stale" | "currentness_warning" | "needs_review"
        )
    }) {
        warnings
            .push("One or more citations are unresolved or need currentness review.".to_string());
    }
    if product
        .document_ast
        .exhibits
        .iter()
        .any(|exhibit| exhibit.status != "attached")
    {
        warnings.push("One or more exhibit references are missing or need review.".to_string());
    }
    if product.product_type == "motion" && !include_qc_report {
        warnings.push("Motion export excludes the QC report.".to_string());
    }
    let has_evidence_or_exhibit = !product.document_ast.exhibits.is_empty()
        || product.document_ast.links.iter().any(|link| {
            matches!(
                link.target_type.as_str(),
                "document" | "evidence" | "exhibit"
            )
        });
    if include_exhibits && !has_evidence_or_exhibit {
        warnings.push("No exhibit or evidence anchors are currently linked.".to_string());
    }
    if matches!(format, "pdf" | "docx") {
        warnings.push(
            "PDF/DOCX output is a deterministic skeleton until the dedicated renderer is enabled."
                .to_string(),
        );
    }
    warnings
}

pub(crate) fn count_work_product_words(product: &WorkProduct) -> u64 {
    canonical_work_product_blocks(product)
        .iter()
        .map(|block| block.text.split_whitespace().count() as u64)
        .sum()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
