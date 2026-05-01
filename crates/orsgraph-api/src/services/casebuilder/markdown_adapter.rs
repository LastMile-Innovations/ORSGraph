use super::work_product_ast::{
    find_ast_block_mut, normalize_work_product_type_lossy, prosemirror_doc_for_text,
    work_product_document_from_projection,
};
use crate::models::casebuilder::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
struct MarkdownFrontmatter {
    schema_version: Option<String>,
    matter_id: Option<String>,
    work_product_id: Option<String>,
    document_type: Option<String>,
    title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct MarkdownDocumentMeta {
    #[serde(default)]
    links: Vec<WorkProductLink>,
    #[serde(default)]
    citations: Vec<WorkProductCitationUse>,
    #[serde(default)]
    exhibits: Vec<WorkProductExhibitReference>,
    #[serde(default)]
    rule_findings: Vec<WorkProductFinding>,
    #[serde(default)]
    tombstones: Vec<WorkProductBlock>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct MarkdownBlockMeta {
    block_id: String,
    #[serde(rename = "type", default)]
    block_type: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    order_index: u64,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    paragraph_number: Option<u64>,
    #[serde(default)]
    section_kind: Option<String>,
    #[serde(default)]
    count_number: Option<u64>,
    #[serde(default)]
    links: Vec<String>,
    #[serde(default)]
    citations: Vec<String>,
    #[serde(default)]
    exhibits: Vec<String>,
    #[serde(default)]
    rule_finding_ids: Vec<String>,
}

#[derive(Debug, Default)]
struct PendingMarkdownBlock {
    meta: Option<MarkdownBlockMeta>,
    title: Option<String>,
    text: Vec<String>,
    block_type: Option<String>,
    role: Option<String>,
    paragraph_number: Option<u64>,
    heading_level: Option<usize>,
}

pub(crate) fn work_product_markdown(product: &WorkProduct) -> String {
    let document_meta = MarkdownDocumentMeta {
        links: product.document_ast.links.clone(),
        citations: product.document_ast.citations.clone(),
        exhibits: product.document_ast.exhibits.clone(),
        rule_findings: product.document_ast.rule_findings.clone(),
        tombstones: product.document_ast.tombstones.clone(),
    };
    let mut lines = vec![
        "---".to_string(),
        format!("schema_version: {}", product.document_ast.schema_version),
        format!("matter_id: {}", product.matter_id),
        format!("work_product_id: {}", product.work_product_id),
        format!("document_type: {}", product.product_type),
        format!("title: {}", product.title),
        "---".to_string(),
        format!(
            "<!-- wp-ast-document {} -->",
            serde_json::to_string(&document_meta).unwrap_or_else(|_| "{}".to_string())
        ),
        format!("# {}", product.title),
    ];

    for block in &product.document_ast.blocks {
        render_markdown_block(block, &mut lines);
    }
    lines.push(String::new());
    lines.push("> Review needed; not legal advice or filing-ready.".to_string());
    lines.join("\n")
}

fn render_markdown_block(block: &WorkProductBlock, lines: &mut Vec<String>) {
    let meta = MarkdownBlockMeta {
        block_id: block.block_id.clone(),
        block_type: block.block_type.clone(),
        role: block.role.clone(),
        title: block.title.clone(),
        order_index: block.ordinal,
        parent_id: block.parent_block_id.clone(),
        paragraph_number: block.paragraph_number,
        section_kind: block.section_kind.clone(),
        count_number: block.count_number,
        links: block.links.clone(),
        citations: block.citations.clone(),
        exhibits: block.exhibits.clone(),
        rule_finding_ids: block.rule_finding_ids.clone(),
    };
    lines.push(String::new());
    lines.push(format!(
        "<!-- wp-ast-block {} -->",
        serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string())
    ));
    match block.block_type.as_str() {
        "heading" | "section" | "count" | "caption" => {
            let level = block
                .section_kind
                .as_deref()
                .and_then(|value| value.strip_prefix("level_"))
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(2)
                .clamp(1, 4);
            lines.push(format!("{} {}", "#".repeat(level), block.title));
            if !block.text.trim().is_empty() && block.text.trim() != block.title.trim() {
                lines.push(block.text.clone());
            }
        }
        "numbered_paragraph" => {
            let number = block.paragraph_number.unwrap_or(block.ordinal);
            lines.push(format!("{number}. {}", block.text));
        }
        "quote" => {
            for line in block.text.lines() {
                lines.push(format!("> {line}"));
            }
        }
        "page_break" => lines.push("<!-- page-break -->".to_string()),
        _ => lines.push(block.text.clone()),
    }
    for child in &block.children {
        render_markdown_block(child, lines);
    }
}

pub(crate) fn markdown_to_work_product_ast(
    product: &WorkProduct,
    markdown: &str,
) -> (WorkProductDocument, Vec<String>) {
    let mut blocks = Vec::new();
    let mut warnings = Vec::new();
    let mut current = PendingMarkdownBlock::default();
    let mut document_meta = MarkdownDocumentMeta::default();
    let mut ordinal = 1_u64;
    let mut in_frontmatter = false;
    let mut frontmatter_seen = false;
    let mut frontmatter = MarkdownFrontmatter::default();
    let mut document_meta_seen = false;
    let mut block_meta_seen = false;

    for raw_line in markdown.lines() {
        let line = raw_line.trim_end();
        if line.trim() == "---" && !frontmatter_seen {
            in_frontmatter = !in_frontmatter;
            if !in_frontmatter {
                frontmatter_seen = true;
            }
            continue;
        }
        if in_frontmatter {
            parse_frontmatter_line(line, &mut frontmatter);
            continue;
        }
        if let Some(payload) = hidden_comment_payload(line, "wp-ast-document") {
            document_meta_seen = true;
            match serde_json::from_str::<MarkdownDocumentMeta>(payload) {
                Ok(meta) => document_meta = meta,
                Err(_) => {
                    warnings.push("Markdown AST document metadata could not be parsed.".to_string())
                }
            }
            continue;
        }
        if let Some(payload) = hidden_comment_payload(line, "wp-ast-block") {
            block_meta_seen = true;
            flush_pending(product, &mut blocks, &mut current, &mut ordinal);
            match serde_json::from_str::<MarkdownBlockMeta>(payload) {
                Ok(meta) => current.meta = Some(meta),
                Err(_) => {
                    warnings.push("Markdown AST block metadata could not be parsed.".to_string())
                }
            }
            continue;
        }
        if line.trim_start().starts_with("<!--") {
            continue;
        }
        if line.trim().is_empty() {
            if !current.text.is_empty() {
                current.text.push(String::new());
            }
            continue;
        }
        if let Some((level, heading)) = markdown_heading(line) {
            if level == 1
                && blocks.is_empty()
                && current.meta.is_none()
                && current.title.is_none()
                && current.text.is_empty()
                && heading == product.title
            {
                continue;
            }
            if current.meta.is_none() || current.title.is_some() || !current.text.is_empty() {
                flush_pending(product, &mut blocks, &mut current, &mut ordinal);
            }
            current.title = Some(heading.to_string());
            current.heading_level = Some(level);
            if current.meta.is_none() {
                current.block_type = Some(if heading.to_ascii_uppercase().starts_with("COUNT ") {
                    "count".to_string()
                } else {
                    "heading".to_string()
                });
                current.role = current.block_type.clone();
            }
            continue;
        }
        if let Some((number, text)) = markdown_numbered_paragraph(line) {
            if current.meta.is_none() || current.title.is_some() || !current.text.is_empty() {
                flush_pending(product, &mut blocks, &mut current, &mut ordinal);
            }
            current.block_type = Some("numbered_paragraph".to_string());
            current.role = Some("factual_allegation".to_string());
            current.paragraph_number = Some(number);
            current.title = Some(format!("Paragraph {number}"));
            current.text.push(text.to_string());
            flush_pending(product, &mut blocks, &mut current, &mut ordinal);
            continue;
        }
        if current.meta.is_none() && current.block_type.is_none() {
            current.block_type = Some("paragraph".to_string());
            current.role = Some("custom".to_string());
        }
        current.text.push(line.trim().to_string());
    }
    flush_pending(product, &mut blocks, &mut current, &mut ordinal);

    if blocks.is_empty() {
        warnings.push(
            "Markdown did not contain recognizable blocks; created an empty AST.".to_string(),
        );
    }
    if !document_meta_seen && product_has_sidecar_metadata(product) {
        warnings.push("Markdown round trip did not include AST sidecar metadata.".to_string());
    }
    if !block_meta_seen && !product.document_ast.blocks.is_empty() {
        warnings.push(
            "Markdown round trip did not include block metadata comments; stable block IDs may be regenerated."
                .to_string(),
        );
    }

    let mut document = work_product_document_from_projection(product, blocks);
    apply_frontmatter(&mut document, product, &frontmatter, &mut warnings);
    if !document_meta.links.is_empty() {
        document.links = document_meta.links;
    }
    if !document_meta.citations.is_empty() {
        document.citations = document_meta.citations;
    }
    if !document_meta.exhibits.is_empty() {
        document.exhibits = document_meta.exhibits;
    }
    if !document_meta.rule_findings.is_empty() {
        document.rule_findings = document_meta.rule_findings;
    }
    if !document_meta.tombstones.is_empty() {
        document.tombstones = document_meta.tombstones;
    }
    rehydrate_block_refs_from_document_records(&mut document);
    (document, warnings)
}

fn parse_frontmatter_line(line: &str, frontmatter: &mut MarkdownFrontmatter) {
    let Some((key, value)) = line.split_once(':') else {
        return;
    };
    let value = value.trim().trim_matches('"').to_string();
    match key.trim() {
        "schema_version" if !value.is_empty() => frontmatter.schema_version = Some(value),
        "matter_id" if !value.is_empty() => frontmatter.matter_id = Some(value),
        "work_product_id" if !value.is_empty() => frontmatter.work_product_id = Some(value),
        "document_type" if !value.is_empty() => frontmatter.document_type = Some(value),
        "title" if !value.is_empty() => frontmatter.title = Some(value),
        _ => {}
    }
}

fn apply_frontmatter(
    document: &mut WorkProductDocument,
    product: &WorkProduct,
    frontmatter: &MarkdownFrontmatter,
    warnings: &mut Vec<String>,
) {
    if let Some(schema_version) = frontmatter.schema_version.as_deref() {
        document.schema_version = schema_version.to_string();
    }
    if let Some(matter_id) = frontmatter.matter_id.as_deref() {
        if matter_id != product.matter_id {
            warnings.push(
                "Markdown frontmatter matter_id differed from the current matter; kept current matter_id."
                    .to_string(),
            );
        }
    }
    if let Some(work_product_id) = frontmatter.work_product_id.as_deref() {
        if work_product_id != product.work_product_id {
            warnings.push(
                "Markdown frontmatter work_product_id differed from the current WorkProduct; kept current work_product_id."
                    .to_string(),
            );
        }
    }
    if let Some(document_type) = frontmatter.document_type.as_deref() {
        let document_type = normalize_work_product_type_lossy(document_type);
        document.document_type = document_type.clone();
        document.product_type = document_type.clone();
        document.metadata.work_product_type = Some(document_type);
    }
    if let Some(title) = frontmatter.title.as_deref() {
        document.title = title.to_string();
        document.metadata.document_title = Some(title.to_string());
    }
}

fn product_has_sidecar_metadata(product: &WorkProduct) -> bool {
    !product.document_ast.links.is_empty()
        || !product.document_ast.citations.is_empty()
        || !product.document_ast.exhibits.is_empty()
        || !product.document_ast.rule_findings.is_empty()
        || !product.document_ast.tombstones.is_empty()
}

fn rehydrate_block_refs_from_document_records(document: &mut WorkProductDocument) {
    let link_refs = document
        .links
        .iter()
        .map(|link| (link.source_block_id.clone(), link.link_id.clone()))
        .collect::<Vec<_>>();
    for (block_id, link_id) in link_refs {
        if let Some(block) = find_ast_block_mut(&mut document.blocks, &block_id) {
            push_unique_ref(&mut block.links, link_id);
        }
    }

    let citation_refs = document
        .citations
        .iter()
        .map(|citation| {
            (
                citation.source_block_id.clone(),
                citation.citation_use_id.clone(),
            )
        })
        .collect::<Vec<_>>();
    for (block_id, citation_id) in citation_refs {
        if let Some(block) = find_ast_block_mut(&mut document.blocks, &block_id) {
            push_unique_ref(&mut block.citations, citation_id);
        }
    }

    let exhibit_refs = document
        .exhibits
        .iter()
        .map(|exhibit| {
            (
                exhibit.source_block_id.clone(),
                exhibit.exhibit_reference_id.clone(),
            )
        })
        .collect::<Vec<_>>();
    for (block_id, exhibit_id) in exhibit_refs {
        if let Some(block) = find_ast_block_mut(&mut document.blocks, &block_id) {
            push_unique_ref(&mut block.exhibits, exhibit_id);
        }
    }

    let finding_refs = document
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
    for (block_id, finding_id) in finding_refs {
        if let Some(block) = find_ast_block_mut(&mut document.blocks, &block_id) {
            push_unique_ref(&mut block.rule_finding_ids, finding_id);
        }
    }
}

fn push_unique_ref(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn flush_pending(
    product: &WorkProduct,
    blocks: &mut Vec<WorkProductBlock>,
    pending: &mut PendingMarkdownBlock,
    ordinal: &mut u64,
) {
    if pending.meta.is_none() && pending.title.is_none() && pending.text.is_empty() {
        return;
    }
    let meta = pending.meta.take().unwrap_or_default();
    let block_type = non_empty(
        &meta.block_type,
        pending.block_type.as_deref().unwrap_or("paragraph"),
    );
    let role = non_empty(&meta.role, pending.role.as_deref().unwrap_or("custom"));
    let title = non_empty(
        &meta.title,
        pending.title.as_deref().unwrap_or_else(|| {
            if block_type == "paragraph" {
                "Paragraph"
            } else {
                "Block"
            }
        }),
    );
    let text = pending.text.join("\n").trim_matches('\n').to_string();
    let text = if text.is_empty() && block_type == "heading" {
        title.clone()
    } else {
        text
    };
    let block_id = if meta.block_id.is_empty() {
        format!("{}:block:{}", product.work_product_id, *ordinal)
    } else {
        meta.block_id
    };
    blocks.push(WorkProductBlock {
        id: block_id.clone(),
        block_id,
        matter_id: product.matter_id.clone(),
        work_product_id: product.work_product_id.clone(),
        block_type,
        role,
        title,
        text: text.clone(),
        ordinal: if meta.order_index == 0 {
            *ordinal
        } else {
            meta.order_index
        },
        parent_block_id: meta.parent_id,
        links: meta.links,
        citations: meta.citations,
        exhibits: meta.exhibits,
        rule_finding_ids: meta.rule_finding_ids,
        paragraph_number: meta.paragraph_number.or(pending.paragraph_number),
        section_kind: meta.section_kind.or_else(|| {
            pending
                .heading_level
                .map(|level| format!("level_{}", level.clamp(1, 4)))
        }),
        count_number: meta.count_number.or_else(|| {
            pending
                .title
                .as_deref()
                .and_then(roman_or_number_after_count)
        }),
        review_status: "needs_review".to_string(),
        prosemirror_json: Some(prosemirror_doc_for_text(&text)),
        ..WorkProductBlock::default()
    });
    *ordinal += 1;
    *pending = PendingMarkdownBlock::default();
}

fn hidden_comment_payload<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let trimmed = line.trim();
    let prefix = format!("<!-- {marker} ");
    trimmed
        .strip_prefix(&prefix)?
        .strip_suffix("-->")
        .map(str::trim)
}

fn non_empty(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn markdown_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|value| *value == '#').count();
    if (1..=4).contains(&level) && trimmed.chars().nth(level) == Some(' ') {
        Some((level, trimmed[level + 1..].trim()))
    } else {
        None
    }
}

fn markdown_numbered_paragraph(line: &str) -> Option<(u64, &str)> {
    let trimmed = line.trim_start();
    let dot = trimmed.find('.')?;
    let number = trimmed[..dot].parse::<u64>().ok()?;
    let text = trimmed[dot + 1..].trim();
    if text.is_empty() {
        None
    } else {
        Some((number, text))
    }
}

fn roman_or_number_after_count(text: &str) -> Option<u64> {
    let rest = text.trim_start_matches(|c: char| c != ' ').trim();
    let token = rest.split_whitespace().next().unwrap_or_default();
    token.parse::<u64>().ok().or_else(|| {
        match token
            .trim_matches(|c: char| !c.is_ascii_alphabetic())
            .to_uppercase()
            .as_str()
        {
            "I" => Some(1),
            "II" => Some(2),
            "III" => Some(3),
            "IV" => Some(4),
            "V" => Some(5),
            "VI" => Some(6),
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product() -> WorkProduct {
        let block = WorkProductBlock {
            block_id: "wp:test:block:1".to_string(),
            id: "wp:test:block:1".to_string(),
            matter_id: "matter:test".to_string(),
            work_product_id: "wp:test".to_string(),
            block_type: "numbered_paragraph".to_string(),
            role: "factual_allegation".to_string(),
            title: "Paragraph 7".to_string(),
            text: "A supported fact.".to_string(),
            ordinal: 1,
            paragraph_number: Some(7),
            citations: vec!["cite:1".to_string()],
            ..WorkProductBlock::default()
        };
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
                blocks: vec![block],
                citations: vec![WorkProductCitationUse {
                    citation_use_id: "cite:1".to_string(),
                    source_block_id: "wp:test:block:1".to_string(),
                    source_text_range: None,
                    raw_text: "ORS 90.320".to_string(),
                    normalized_citation: Some("ORS 90.320".to_string()),
                    target_type: "provision".to_string(),
                    target_id: Some("or:ors:90.320".to_string()),
                    pinpoint: None,
                    status: "resolved".to_string(),
                    resolver_message: None,
                    created_at: "1".to_string(),
                }],
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

    #[test]
    fn markdown_round_trip_preserves_block_and_citation_ids() {
        let product = product();
        let markdown = work_product_markdown(&product);
        let (document, warnings) = markdown_to_work_product_ast(&product, &markdown);
        assert!(warnings.is_empty());
        assert_eq!(document.blocks[0].block_id, "wp:test:block:1");
        assert_eq!(document.blocks[0].paragraph_number, Some(7));
        assert_eq!(document.citations[0].citation_use_id, "cite:1");
    }

    #[test]
    fn markdown_frontmatter_updates_safe_document_fields() {
        let product = product();
        let markdown = work_product_markdown(&product).replace("title: Test", "title: Revised");
        let (document, warnings) = markdown_to_work_product_ast(&product, &markdown);
        assert!(warnings.is_empty());
        assert_eq!(document.title, "Revised");
        assert_eq!(document.metadata.document_title.as_deref(), Some("Revised"));
    }

    #[test]
    fn markdown_sidecar_rehydrates_block_record_refs() {
        let product = product();
        let markdown = work_product_markdown(&product)
            .replace("\"citations\":[\"cite:1\"]", "\"citations\":[]");
        let (document, warnings) = markdown_to_work_product_ast(&product, &markdown);
        assert!(warnings.is_empty());
        assert_eq!(document.blocks[0].citations, vec!["cite:1".to_string()]);
    }

    #[test]
    fn markdown_without_sidecar_warns_about_metadata_loss() {
        let product = product();
        let markdown = "# Test\n\n1. A supported fact.";
        let (_document, warnings) = markdown_to_work_product_ast(&product, markdown);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("sidecar metadata")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("block metadata comments")));
    }
}
