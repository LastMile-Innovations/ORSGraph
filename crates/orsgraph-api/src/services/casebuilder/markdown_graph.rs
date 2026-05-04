use super::*;
use pulldown_cmark::{Event, Options, Parser};
use std::ops::Range;

const MARKDOWN_AST_PARSER_ID: &str = "pulldown-cmark";
const MARKDOWN_AST_PARSER_VERSION: &str = "pulldown-cmark-0.13";
pub(super) const MARKDOWN_GRAPH_SCHEMA_VERSION: &str = "casebuilder-markdown-graph-v2";

#[derive(Debug, Clone)]
struct AstStackEntry {
    node_id: String,
    path: String,
    child_count: u64,
    last_child_id: Option<String>,
    structure_path: Option<String>,
}

pub(super) fn build_markdown_ast_graph(
    matter_id: &str,
    document_id: &str,
    source_context: &SourceContext,
    index_run_id: &str,
    text: &str,
    source_sha256: &str,
    text_chunks: &[TextChunk],
    evidence_spans: &[EvidenceSpan],
    source_spans: &[SourceSpan],
    search_index_records: &[SearchIndexRecord],
) -> (MarkdownAstDocument, Vec<MarkdownAstNode>) {
    let version_seed = source_context
        .document_version_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(document_id);
    let ast_document_id = format!(
        "markdown-ast:{}:{}",
        sanitize_path_segment(document_id),
        hex_prefix(format!("{version_seed}:{source_sha256}").as_bytes(), 20)
    );
    let root_node_id =
        markdown_ast_node_id(document_id, version_seed, "0", "document", 0, 0, text.len());
    let mut nodes = vec![markdown_ast_node(
        matter_id,
        document_id,
        source_context,
        index_run_id,
        &ast_document_id,
        &root_node_id,
        None,
        None,
        "document",
        "root",
        0,
        0,
        "0",
        0,
        None,
        text,
        0..text.len(),
        text_chunks,
        evidence_spans,
        source_spans,
        search_index_records,
    )];
    let mut stack = vec![AstStackEntry {
        node_id: root_node_id.clone(),
        path: "0".to_string(),
        child_count: 0,
        last_child_id: None,
        structure_path: None,
    }];
    let mut ordinal = 1_u64;

    for (event, range) in Parser::new_ext(text, Options::all()).into_offset_iter() {
        match &event {
            Event::Start(tag) => {
                let tag_text = format!("{tag:?}");
                let node_kind = markdown_start_node_kind(&tag_text);
                let (node_id, parent_id, previous_id, path, depth, structure_path) =
                    next_child_identity(
                        &mut stack,
                        document_id,
                        version_seed,
                        node_kind,
                        text_chunks,
                        &range,
                    );
                nodes.push(markdown_ast_node(
                    matter_id,
                    document_id,
                    source_context,
                    index_run_id,
                    &ast_document_id,
                    &node_id,
                    Some(parent_id.as_str()),
                    previous_id.as_deref(),
                    node_kind,
                    &tag_text,
                    ordinal,
                    depth,
                    &path,
                    path.rsplit('.')
                        .next()
                        .and_then(|value| value.parse::<u64>().ok())
                        .unwrap_or(0),
                    structure_path.as_deref(),
                    text,
                    range.clone(),
                    text_chunks,
                    evidence_spans,
                    source_spans,
                    search_index_records,
                ));
                stack.push(AstStackEntry {
                    node_id,
                    path,
                    child_count: 0,
                    last_child_id: None,
                    structure_path,
                });
                ordinal += 1;
            }
            Event::End(_) => {
                if stack.len() > 1 {
                    stack.pop();
                }
            }
            _ => {
                let node_kind = markdown_leaf_node_kind(&event);
                if node_kind == "empty" {
                    continue;
                }
                let (node_id, parent_id, previous_id, path, depth, structure_path) =
                    next_child_identity(
                        &mut stack,
                        document_id,
                        version_seed,
                        node_kind,
                        text_chunks,
                        &range,
                    );
                nodes.push(markdown_ast_node(
                    matter_id,
                    document_id,
                    source_context,
                    index_run_id,
                    &ast_document_id,
                    &node_id,
                    Some(parent_id.as_str()),
                    previous_id.as_deref(),
                    node_kind,
                    &format!("{event:?}"),
                    ordinal,
                    depth,
                    &path,
                    path.rsplit('.')
                        .next()
                        .and_then(|value| value.parse::<u64>().ok())
                        .unwrap_or(0),
                    structure_path.as_deref(),
                    text,
                    range,
                    text_chunks,
                    evidence_spans,
                    source_spans,
                    search_index_records,
                ));
                ordinal += 1;
            }
        }
    }

    enrich_markdown_ast_nodes(&mut nodes, text);
    let document_stats = markdown_ast_document_stats(&nodes);
    let document = MarkdownAstDocument {
        markdown_ast_document_id: ast_document_id.clone(),
        id: ast_document_id,
        matter_id: matter_id.to_string(),
        document_id: document_id.to_string(),
        document_version_id: source_context.document_version_id.clone(),
        object_blob_id: source_context.object_blob_id.clone(),
        ingestion_run_id: source_context.ingestion_run_id.clone(),
        index_run_id: Some(index_run_id.to_string()),
        parser_id: MARKDOWN_AST_PARSER_ID.to_string(),
        parser_version: MARKDOWN_AST_PARSER_VERSION.to_string(),
        source_sha256: source_sha256.to_string(),
        root_node_id,
        node_count: nodes.len() as u64,
        semantic_unit_count: 0,
        heading_count: document_stats.heading_count,
        block_count: document_stats.block_count,
        inline_count: document_stats.inline_count,
        reference_count: document_stats.reference_count,
        max_depth: document_stats.max_depth,
        entity_mention_count: 0,
        citation_count: 0,
        date_count: 0,
        money_count: 0,
        graph_schema_version: MARKDOWN_GRAPH_SCHEMA_VERSION.to_string(),
        status: "indexed".to_string(),
        created_at: now_string(),
    };
    (document, nodes)
}

pub(super) fn build_markdown_semantic_units(
    matter_id: &str,
    document_id: &str,
    document: &mut MarkdownAstDocument,
    nodes: &[MarkdownAstNode],
) -> Vec<MarkdownSemanticUnit> {
    let now = now_string();
    let mut units = BTreeMap::<String, MarkdownSemanticUnit>::new();
    for node in nodes.iter().filter(|node| is_semantic_unit_node(node)) {
        let Some(semantic_unit_id) = node.semantic_unit_id.clone() else {
            continue;
        };
        let semantic_role = node
            .semantic_role
            .clone()
            .unwrap_or_else(|| "markdown_node".to_string());
        let canonical_label = node
            .heading_text
            .clone()
            .or_else(|| node.text_excerpt.clone())
            .or_else(|| node.structure_path.clone())
            .unwrap_or_else(|| node.node_kind.clone());
        let normalized_key = normalized_entity_key(&canonical_label);
        let entry = units
            .entry(semantic_unit_id.clone())
            .or_insert_with(|| MarkdownSemanticUnit {
                semantic_unit_id: semantic_unit_id.clone(),
                id: semantic_unit_id.clone(),
                matter_id: matter_id.to_string(),
                document_id: document_id.to_string(),
                document_version_id: node.document_version_id.clone(),
                markdown_ast_document_id: node.markdown_ast_document_id.clone(),
                unit_kind: node.node_kind.clone(),
                semantic_role: semantic_role.clone(),
                canonical_label: text_excerpt(&canonical_label, 240),
                normalized_key: normalized_key.clone(),
                structure_path: node.structure_path.clone(),
                section_path: node.section_path.clone(),
                section_ast_node_id: node.section_ast_node_id.clone(),
                text_hash: node.text_hash.clone(),
                semantic_fingerprint: node
                    .semantic_fingerprint
                    .clone()
                    .unwrap_or_else(|| semantic_unit_id.clone()),
                markdown_ast_node_ids: Vec::new(),
                entity_mention_ids: Vec::new(),
                citation_texts: Vec::new(),
                date_texts: Vec::new(),
                money_texts: Vec::new(),
                occurrence_count: 0,
                evidence_span_count: 0,
                text_chunk_count: 0,
                source_span_count: 0,
                review_status: node.review_status.clone(),
                created_at: now.clone(),
                updated_at: now.clone(),
            });
        push_unique(
            &mut entry.markdown_ast_node_ids,
            node.markdown_ast_node_id.clone(),
        );
        for id in &node.entity_mention_ids {
            push_unique(&mut entry.entity_mention_ids, id.clone());
        }
        for value in &node.citation_texts {
            push_unique(&mut entry.citation_texts, value.clone());
        }
        for value in &node.date_texts {
            push_unique(&mut entry.date_texts, value.clone());
        }
        for value in &node.money_texts {
            push_unique(&mut entry.money_texts, value.clone());
        }
        entry.occurrence_count += 1;
        entry.evidence_span_count += node.evidence_span_ids.len() as u64;
        entry.text_chunk_count += node.text_chunk_ids.len() as u64;
        entry.source_span_count += node.source_span_ids.len() as u64;
        entry.updated_at = now.clone();
    }
    let units = units.into_values().collect::<Vec<_>>();
    document.semantic_unit_count = units.len() as u64;
    document.entity_mention_count = nodes
        .iter()
        .flat_map(|node| node.entity_mention_ids.iter())
        .collect::<HashSet<_>>()
        .len() as u64;
    document.citation_count = nodes
        .iter()
        .flat_map(|node| node.citation_texts.iter())
        .collect::<HashSet<_>>()
        .len() as u64;
    document.date_count = nodes
        .iter()
        .flat_map(|node| node.date_texts.iter())
        .collect::<HashSet<_>>()
        .len() as u64;
    document.money_count = nodes
        .iter()
        .flat_map(|node| node.money_texts.iter())
        .collect::<HashSet<_>>()
        .len() as u64;
    units
}

pub(super) fn canonical_entities_for_mentions(
    matter_id: &str,
    mentions: &mut [EntityMention],
    parties: &[CaseParty],
) -> Vec<CaseEntity> {
    let now = now_string();
    let mut groups: BTreeMap<(String, String), CaseEntity> = BTreeMap::new();
    for mention in mentions.iter_mut() {
        let normalized_key = normalized_entity_key(&mention.mention_text);
        if normalized_key.is_empty() {
            continue;
        }
        let key = (mention.entity_type.clone(), normalized_key.clone());
        let entity = groups.entry(key).or_insert_with(|| {
            let seed = format!("{}:{}:{}", matter_id, mention.entity_type, normalized_key);
            let entity_id = format!(
                "case-entity:{}:{}",
                sanitize_path_segment(matter_id),
                hex_prefix(seed.as_bytes(), 20)
            );
            CaseEntity {
                entity_id: entity_id.clone(),
                id: entity_id,
                matter_id: matter_id.to_string(),
                entity_type: mention.entity_type.clone(),
                canonical_name: mention.mention_text.clone(),
                normalized_key: normalized_key.clone(),
                confidence: mention.confidence,
                review_status: "unreviewed".to_string(),
                mention_ids: Vec::new(),
                party_match_ids: party_matches_for_entity(
                    &mention.entity_type,
                    &normalized_key,
                    parties,
                ),
                created_at: now.clone(),
                updated_at: now.clone(),
            }
        });
        if mention.confidence > entity.confidence {
            entity.confidence = mention.confidence;
            entity.canonical_name = mention.mention_text.clone();
            entity.updated_at = now.clone();
        }
        push_unique(&mut entity.mention_ids, mention.entity_mention_id.clone());
        mention.entity_id = Some(entity.entity_id.clone());
    }
    groups.into_values().collect()
}

pub(super) fn attach_markdown_ast_node_ids_to_records(
    ast_nodes: &mut [MarkdownAstNode],
    chunks: &mut [ExtractedTextChunk],
    text_chunks: &mut [TextChunk],
    evidence_spans: &mut [EvidenceSpan],
    entity_mentions: &mut [EntityMention],
    facts: &mut [CaseFact],
    suggestions: &mut [TimelineSuggestion],
) {
    for chunk in chunks {
        chunk.markdown_ast_node_ids =
            markdown_ast_node_ids_for_range(ast_nodes, chunk.byte_start, chunk.byte_end);
    }
    for chunk in text_chunks {
        chunk.markdown_ast_node_ids =
            markdown_ast_node_ids_for_range(ast_nodes, chunk.byte_start, chunk.byte_end);
    }
    for span in evidence_spans {
        span.markdown_ast_node_ids =
            markdown_ast_node_ids_for_range(ast_nodes, span.byte_start, span.byte_end);
    }
    for mention in entity_mentions {
        mention.markdown_ast_node_ids =
            markdown_ast_node_ids_for_range(ast_nodes, mention.byte_start, mention.byte_end);
        mark_nodes_for_entity_mention(ast_nodes, mention);
    }
    for fact in facts {
        let source_span_ids = fact
            .source_spans
            .iter()
            .map(|span| span.source_span_id.clone())
            .collect::<Vec<_>>();
        let text_chunk_ids = fact
            .source_spans
            .iter()
            .filter_map(|span| span.chunk_id.clone())
            .collect::<Vec<_>>();
        fact.markdown_ast_node_ids =
            markdown_ast_node_ids_for_refs(ast_nodes, &source_span_ids, &text_chunk_ids);
        for node_id in &fact.markdown_ast_node_ids {
            if let Some(node) = ast_nodes
                .iter_mut()
                .find(|node| &node.markdown_ast_node_id == node_id)
            {
                push_unique(&mut node.fact_ids, fact.fact_id.clone());
            }
        }
    }
    for suggestion in suggestions {
        suggestion.markdown_ast_node_ids = markdown_ast_node_ids_for_refs(
            ast_nodes,
            &suggestion.source_span_ids,
            &suggestion.text_chunk_ids,
        );
        for node_id in &suggestion.markdown_ast_node_ids {
            if let Some(node) = ast_nodes
                .iter_mut()
                .find(|node| &node.markdown_ast_node_id == node_id)
            {
                push_unique(
                    &mut node.timeline_suggestion_ids,
                    suggestion.suggestion_id.clone(),
                );
            }
        }
    }
}

pub(super) fn markdown_ast_node_ids_for_refs(
    ast_nodes: &[MarkdownAstNode],
    source_span_ids: &[String],
    text_chunk_ids: &[String],
) -> Vec<String> {
    let source_span_ids = source_span_ids.iter().collect::<HashSet<_>>();
    let text_chunk_ids = text_chunk_ids.iter().collect::<HashSet<_>>();
    let mut out = Vec::new();
    for node in ast_nodes.iter().filter(|node| node.node_kind != "document") {
        let source_match = node
            .source_span_ids
            .iter()
            .any(|id| source_span_ids.contains(id));
        let chunk_match = node
            .text_chunk_ids
            .iter()
            .any(|id| text_chunk_ids.contains(id));
        if source_match || chunk_match {
            push_unique(&mut out, node.markdown_ast_node_id.clone());
        }
    }
    out
}

fn mark_nodes_for_entity_mention(ast_nodes: &mut [MarkdownAstNode], mention: &EntityMention) {
    for node_id in &mention.markdown_ast_node_ids {
        let Some(node) = ast_nodes
            .iter_mut()
            .find(|node| &node.markdown_ast_node_id == node_id)
        else {
            continue;
        };
        push_unique(
            &mut node.entity_mention_ids,
            mention.entity_mention_id.clone(),
        );
        node.contains_entity_mention = true;
        match mention.entity_type.as_str() {
            "date" => {
                node.contains_date = true;
                push_unique(&mut node.date_texts, mention.mention_text.clone());
            }
            "money" => {
                node.contains_money = true;
                push_unique(&mut node.money_texts, mention.mention_text.clone());
            }
            "statute" | "rule" | "constitutional_authority" | "session_law" => {
                node.contains_citation = true;
                push_unique(&mut node.citation_texts, mention.mention_text.clone());
            }
            _ => {}
        }
    }
}

fn next_child_identity(
    stack: &mut [AstStackEntry],
    document_id: &str,
    version_seed: &str,
    node_kind: &str,
    text_chunks: &[TextChunk],
    range: &Range<usize>,
) -> (String, String, Option<String>, String, u64, Option<String>) {
    let depth = stack.len() as u64;
    let parent = stack
        .last_mut()
        .expect("markdown AST stack should always contain the root");
    parent.child_count += 1;
    let path = format!("{}.{}", parent.path, parent.child_count);
    let previous = parent.last_child_id.clone();
    let structure_path =
        structure_path_for_range(text_chunks, range).or_else(|| parent.structure_path.clone());
    let parent_id = parent.node_id.clone();
    let node_id = markdown_ast_node_id(
        document_id,
        version_seed,
        &path,
        node_kind,
        parent.child_count,
        range.start,
        range.end,
    );
    parent.last_child_id = Some(node_id.clone());
    (node_id, parent_id, previous, path, depth, structure_path)
}

fn markdown_ast_node(
    matter_id: &str,
    document_id: &str,
    source_context: &SourceContext,
    index_run_id: &str,
    ast_document_id: &str,
    node_id: &str,
    parent_id: Option<&str>,
    previous_id: Option<&str>,
    node_kind: &str,
    tag: &str,
    ordinal: u64,
    depth: u64,
    ast_path: &str,
    sibling_index: u64,
    structure_path: Option<&str>,
    text: &str,
    range: Range<usize>,
    text_chunks: &[TextChunk],
    evidence_spans: &[EvidenceSpan],
    source_spans: &[SourceSpan],
    search_index_records: &[SearchIndexRecord],
) -> MarkdownAstNode {
    let byte_start = range.start.min(text.len());
    let byte_end = range.end.min(text.len());
    let excerpt = text
        .get(byte_start..byte_end)
        .map(|value| text_excerpt(value, 320))
        .filter(|value| !value.trim().is_empty());
    let text_hash = text
        .get(byte_start..byte_end)
        .map(|value| sha256_hex(value.as_bytes()));
    MarkdownAstNode {
        markdown_ast_node_id: node_id.to_string(),
        id: node_id.to_string(),
        matter_id: matter_id.to_string(),
        document_id: document_id.to_string(),
        document_version_id: source_context.document_version_id.clone(),
        object_blob_id: source_context.object_blob_id.clone(),
        ingestion_run_id: source_context.ingestion_run_id.clone(),
        index_run_id: Some(index_run_id.to_string()),
        markdown_ast_document_id: ast_document_id.to_string(),
        parent_ast_node_id: parent_id.map(str::to_string),
        previous_ast_node_id: previous_id.map(str::to_string),
        semantic_unit_id: None,
        node_kind: node_kind.to_string(),
        tag: text_excerpt(tag, 500),
        ordinal,
        depth,
        ast_path: ast_path.to_string(),
        sibling_index,
        child_count: 0,
        structure_path: structure_path.map(str::to_string),
        section_ast_node_id: None,
        section_path: None,
        heading_level: None,
        heading_text: None,
        semantic_role: None,
        semantic_fingerprint: None,
        text_hash,
        text_excerpt: excerpt,
        byte_start: Some(byte_start as u64),
        byte_end: Some(byte_end as u64),
        char_start: Some(text[..byte_start].chars().count() as u64),
        char_end: Some(text[..byte_end].chars().count() as u64),
        source_span_ids: overlapping_source_span_ids(source_spans, byte_start, byte_end),
        text_chunk_ids: overlapping_text_chunk_ids(text_chunks, byte_start, byte_end),
        evidence_span_ids: overlapping_evidence_span_ids(evidence_spans, byte_start, byte_end),
        search_index_record_ids: overlapping_search_index_record_ids(
            search_index_records,
            text_chunks,
            byte_start,
            byte_end,
        ),
        entity_mention_ids: Vec::new(),
        fact_ids: Vec::new(),
        timeline_suggestion_ids: Vec::new(),
        citation_texts: Vec::new(),
        date_texts: Vec::new(),
        money_texts: Vec::new(),
        contains_entity_mention: false,
        contains_citation: false,
        contains_date: false,
        contains_money: false,
        review_status: "unreviewed".to_string(),
    }
}

#[derive(Default)]
struct MarkdownAstDocumentStats {
    heading_count: u64,
    block_count: u64,
    inline_count: u64,
    reference_count: u64,
    max_depth: u64,
}

fn enrich_markdown_ast_nodes(nodes: &mut [MarkdownAstNode], text: &str) {
    let mut child_counts = BTreeMap::<String, u64>::new();
    for node in nodes.iter().filter(|node| node.node_kind != "document") {
        if let Some(parent_id) = &node.parent_ast_node_id {
            *child_counts.entry(parent_id.clone()).or_default() += 1;
        }
    }
    for node in nodes.iter_mut() {
        node.child_count = child_counts
            .get(&node.markdown_ast_node_id)
            .copied()
            .unwrap_or_default();
        node.heading_level = if node.node_kind == "heading" {
            markdown_heading_level(&node.tag, node.text_excerpt.as_deref())
        } else {
            None
        };
        node.heading_text = node
            .heading_level
            .and_then(|_| heading_text_from_node(node, text));
    }

    let mut heading_stack: Vec<(u64, String, String)> = Vec::new();
    for index in 0..nodes.len() {
        let node_kind = nodes[index].node_kind.clone();
        if node_kind == "heading" {
            let level = nodes[index].heading_level.unwrap_or(1);
            while heading_stack
                .last()
                .is_some_and(|(existing_level, _, _)| *existing_level >= level)
            {
                heading_stack.pop();
            }
            nodes[index].section_ast_node_id = heading_stack.last().map(|(_, id, _)| id.clone());
            let section_path = heading_stack
                .iter()
                .map(|(_, _, title)| title.clone())
                .chain(nodes[index].heading_text.clone())
                .collect::<Vec<_>>()
                .join(" / ");
            nodes[index].section_path = (!section_path.is_empty()).then_some(section_path);
            if let Some(title) = nodes[index].heading_text.clone() {
                heading_stack.push((level, nodes[index].markdown_ast_node_id.clone(), title));
            }
        } else if let Some((_, id, _)) = heading_stack.last() {
            nodes[index].section_ast_node_id = Some(id.clone());
            nodes[index].section_path = Some(
                heading_stack
                    .iter()
                    .map(|(_, _, title)| title.clone())
                    .collect::<Vec<_>>()
                    .join(" / "),
            );
        }

        nodes[index].semantic_role = Some(markdown_semantic_role(&nodes[index]).to_string());
        nodes[index].semantic_fingerprint = semantic_fingerprint(&nodes[index]);
        if is_semantic_unit_node(&nodes[index]) {
            nodes[index].semantic_unit_id = Some(markdown_semantic_unit_id(&nodes[index]));
        }
    }
}

fn markdown_ast_document_stats(nodes: &[MarkdownAstNode]) -> MarkdownAstDocumentStats {
    let mut stats = MarkdownAstDocumentStats::default();
    for node in nodes {
        stats.max_depth = stats.max_depth.max(node.depth);
        if node.node_kind == "heading" {
            stats.heading_count += 1;
        }
        if markdown_node_is_reference(node) {
            stats.reference_count += 1;
        }
        if markdown_node_is_inline(node) {
            stats.inline_count += 1;
        } else {
            stats.block_count += 1;
        }
    }
    stats
}

fn heading_text_from_node(node: &MarkdownAstNode, text: &str) -> Option<String> {
    let start = node.byte_start? as usize;
    let end = node.byte_end? as usize;
    let raw = text.get(start.min(text.len())..end.min(text.len()))?;
    let cleaned = raw
        .lines()
        .map(|line| {
            line.trim()
                .trim_start_matches('#')
                .trim()
                .trim_matches(|ch| ch == '[' || ch == ']')
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(" ");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        node.text_excerpt.clone()
    } else {
        Some(text_excerpt(&cleaned, 160))
    }
}

fn markdown_heading_level(tag: &str, excerpt: Option<&str>) -> Option<u64> {
    for (needle, level) in [
        ("H1", 1),
        ("H2", 2),
        ("H3", 3),
        ("H4", 4),
        ("H5", 5),
        ("H6", 6),
    ] {
        if tag.contains(needle) {
            return Some(level);
        }
    }
    excerpt.and_then(|value| {
        let hashes = value.chars().take_while(|ch| *ch == '#').count();
        (1..=6).contains(&hashes).then_some(hashes as u64)
    })
}

fn markdown_semantic_role(node: &MarkdownAstNode) -> &'static str {
    match node.node_kind.as_str() {
        "document" => "document_root",
        "heading" => "section_heading",
        "paragraph" => "paragraph",
        "quote" => "quoted_source",
        "list" => "list",
        "list_item" => "list_item",
        "table" | "table_row" | "table_cell" | "table_head" => "table",
        "code_block" | "inline_code" => "code",
        "link" | "image" | "footnote_reference" => "reference",
        "text" => "text",
        "thematic_break" => "divider",
        _ if markdown_node_is_inline(node) => "inline",
        _ => "markdown_block",
    }
}

fn markdown_node_is_inline(node: &MarkdownAstNode) -> bool {
    matches!(
        node.node_kind.as_str(),
        "text" | "inline" | "inline_code" | "break" | "link" | "image" | "footnote_reference"
    )
}

fn markdown_node_is_reference(node: &MarkdownAstNode) -> bool {
    matches!(
        node.node_kind.as_str(),
        "link" | "image" | "footnote_reference"
    ) || node.contains_citation
}

fn is_semantic_unit_node(node: &MarkdownAstNode) -> bool {
    matches!(
        node.node_kind.as_str(),
        "heading" | "paragraph" | "quote" | "list_item" | "table" | "code_block" | "link" | "image"
    )
}

fn semantic_fingerprint(node: &MarkdownAstNode) -> Option<String> {
    let label = node
        .heading_text
        .as_deref()
        .or(node.text_excerpt.as_deref())
        .unwrap_or(node.node_kind.as_str());
    let normalized = normalized_entity_key(label);
    if normalized.is_empty() && node.text_hash.is_none() {
        return None;
    }
    let seed = format!(
        "{}:{}:{}:{}",
        node.node_kind,
        node.section_path.clone().unwrap_or_default(),
        normalized,
        node.text_hash.clone().unwrap_or_default()
    );
    Some(hex_prefix(seed.as_bytes(), 24))
}

fn markdown_semantic_unit_id(node: &MarkdownAstNode) -> String {
    let fingerprint = node
        .semantic_fingerprint
        .clone()
        .unwrap_or_else(|| hex_prefix(node.markdown_ast_node_id.as_bytes(), 24));
    let seed = format!("{}:{}:{}", node.document_id, node.node_kind, fingerprint);
    format!(
        "markdown-unit:{}:{}",
        sanitize_path_segment(&node.document_id),
        hex_prefix(seed.as_bytes(), 24)
    )
}

fn markdown_ast_node_id(
    document_id: &str,
    version_seed: &str,
    path: &str,
    node_kind: &str,
    ordinal: u64,
    byte_start: usize,
    byte_end: usize,
) -> String {
    let seed = format!("{version_seed}:{path}:{node_kind}:{ordinal}:{byte_start}:{byte_end}");
    format!(
        "markdown-node:{}:{}",
        sanitize_path_segment(document_id),
        hex_prefix(seed.as_bytes(), 20)
    )
}

fn markdown_start_node_kind(tag: &str) -> &'static str {
    if tag.starts_with("Paragraph") {
        "paragraph"
    } else if tag.starts_with("Heading") {
        "heading"
    } else if tag.starts_with("BlockQuote") {
        "quote"
    } else if tag.starts_with("CodeBlock") {
        "code_block"
    } else if tag.starts_with("List") {
        "list"
    } else if tag.starts_with("Item") {
        "list_item"
    } else if tag.starts_with("Table") {
        "table"
    } else if tag.starts_with("TableHead") {
        "table_head"
    } else if tag.starts_with("TableRow") {
        "table_row"
    } else if tag.starts_with("TableCell") {
        "table_cell"
    } else if tag.starts_with("Link") {
        "link"
    } else if tag.starts_with("Image") {
        "image"
    } else if tag.starts_with("Emphasis")
        || tag.starts_with("Strong")
        || tag.starts_with("Strikethrough")
    {
        "inline"
    } else {
        "block"
    }
}

fn markdown_leaf_node_kind(event: &Event<'_>) -> &'static str {
    match event {
        Event::Text(value) if value.trim().is_empty() => "empty",
        Event::Text(_) => "text",
        Event::Code(_) => "inline_code",
        Event::Html(_) | Event::InlineHtml(_) => "html",
        Event::SoftBreak | Event::HardBreak => "break",
        Event::Rule => "thematic_break",
        Event::FootnoteReference(_) => "footnote_reference",
        Event::TaskListMarker(_) => "task_marker",
        _ => "inline",
    }
}

fn structure_path_for_range(text_chunks: &[TextChunk], range: &Range<usize>) -> Option<String> {
    let start = range.start as u64;
    let end = range.end as u64;
    text_chunks
        .iter()
        .find(|chunk| ranges_overlap(start, end, chunk.byte_start, chunk.byte_end))
        .and_then(|chunk| chunk.structure_path.clone())
}

fn overlapping_source_span_ids(spans: &[SourceSpan], start: usize, end: usize) -> Vec<String> {
    spans
        .iter()
        .filter(|span| ranges_overlap(start as u64, end as u64, span.byte_start, span.byte_end))
        .map(|span| span.source_span_id.clone())
        .collect()
}

fn overlapping_text_chunk_ids(chunks: &[TextChunk], start: usize, end: usize) -> Vec<String> {
    chunks
        .iter()
        .filter(|chunk| ranges_overlap(start as u64, end as u64, chunk.byte_start, chunk.byte_end))
        .map(|chunk| chunk.text_chunk_id.clone())
        .collect()
}

fn overlapping_evidence_span_ids(spans: &[EvidenceSpan], start: usize, end: usize) -> Vec<String> {
    spans
        .iter()
        .filter(|span| ranges_overlap(start as u64, end as u64, span.byte_start, span.byte_end))
        .map(|span| span.evidence_span_id.clone())
        .collect()
}

fn overlapping_search_index_record_ids(
    records: &[SearchIndexRecord],
    chunks: &[TextChunk],
    start: usize,
    end: usize,
) -> Vec<String> {
    let chunk_ids = overlapping_text_chunk_ids(chunks, start, end)
        .into_iter()
        .collect::<HashSet<_>>();
    records
        .iter()
        .filter(|record| {
            record
                .text_chunk_id
                .as_ref()
                .is_some_and(|chunk_id| chunk_ids.contains(chunk_id))
        })
        .map(|record| record.search_index_record_id.clone())
        .collect()
}

fn markdown_ast_node_ids_for_range(
    nodes: &[MarkdownAstNode],
    start: Option<u64>,
    end: Option<u64>,
) -> Vec<String> {
    let mut out = Vec::new();
    for node in nodes.iter().filter(|node| node.node_kind != "document") {
        if ranges_overlap(
            node.byte_start.unwrap_or_default(),
            node.byte_end.unwrap_or_default(),
            start,
            end,
        ) {
            push_unique(&mut out, node.markdown_ast_node_id.clone());
        }
    }
    out
}

fn ranges_overlap(start: u64, end: u64, other_start: Option<u64>, other_end: Option<u64>) -> bool {
    let Some(other_start) = other_start else {
        return false;
    };
    let Some(other_end) = other_end else {
        return false;
    };
    start < other_end && end > other_start
}

fn normalized_entity_key(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn party_matches_for_entity(
    entity_type: &str,
    normalized_key: &str,
    parties: &[CaseParty],
) -> Vec<String> {
    if entity_type != "party" {
        return Vec::new();
    }
    parties
        .iter()
        .filter(|party| normalized_entity_key(&party.name) == normalized_key)
        .map(|party| party.party_id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> SourceContext {
        SourceContext {
            document_version_id: Some("version:doc:a".to_string()),
            object_blob_id: Some("blob:doc".to_string()),
            ingestion_run_id: Some("ingestion:doc".to_string()),
        }
    }

    fn chunk(id: &str, start: u64, end: u64, structure_path: &str) -> TextChunk {
        TextChunk {
            text_chunk_id: id.to_string(),
            id: id.to_string(),
            matter_id: "matter:1".to_string(),
            document_id: "doc:1".to_string(),
            document_version_id: Some("version:doc:a".to_string()),
            object_blob_id: None,
            page_id: None,
            source_span_id: Some(format!("span:{id}")),
            ingestion_run_id: None,
            index_run_id: Some("index:1".to_string()),
            ordinal: 1,
            page: 1,
            text_hash: "hash".to_string(),
            text_excerpt: "excerpt".to_string(),
            token_count: 1,
            unit_type: Some("paragraph".to_string()),
            structure_path: Some(structure_path.to_string()),
            markdown_ast_node_ids: Vec::new(),
            byte_start: Some(start),
            byte_end: Some(end),
            char_start: Some(start),
            char_end: Some(end),
            status: "indexed".to_string(),
        }
    }

    #[test]
    fn markdown_ast_graph_covers_commonmark_nodes_and_source_links() {
        let text = "# Facts\n\n> Quote line\n\n1. Paid $1,200 on April 2, 2026.\n\n- item\n\n```text\ncode\n```\n\n[ORS](https://example.test)\n\n| A | B |\n| - | - |\n| 1 | 2 |\n";
        let chunks = vec![chunk("chunk:1", 0, text.len() as u64, "Facts")];
        let source_spans = vec![SourceSpan {
            source_span_id: "span:chunk:1".to_string(),
            id: "span:chunk:1".to_string(),
            matter_id: "matter:1".to_string(),
            document_id: "doc:1".to_string(),
            document_version_id: Some("version:doc:a".to_string()),
            object_blob_id: None,
            ingestion_run_id: None,
            page: Some(1),
            chunk_id: Some("chunk:1".to_string()),
            byte_start: Some(0),
            byte_end: Some(text.len() as u64),
            char_start: Some(0),
            char_end: Some(text.chars().count() as u64),
            time_start_ms: None,
            time_end_ms: None,
            speaker_label: None,
            quote: Some(text.to_string()),
            extraction_method: "test".to_string(),
            confidence: 1.0,
            review_status: "unreviewed".to_string(),
            unavailable_reason: None,
        }];
        let (document, nodes) = build_markdown_ast_graph(
            "matter:1",
            "doc:1",
            &context(),
            "index:1",
            text,
            &sha256_hex(text.as_bytes()),
            &chunks,
            &[],
            &source_spans,
            &[],
        );
        assert_eq!(document.node_count, nodes.len() as u64);
        for expected in [
            "document",
            "heading",
            "quote",
            "list",
            "list_item",
            "code_block",
            "link",
            "table",
            "text",
        ] {
            assert!(
                nodes.iter().any(|node| node.node_kind == expected),
                "missing markdown AST node kind {expected}"
            );
        }
        assert!(nodes.iter().all(|node| node.byte_start <= node.byte_end));
        assert!(nodes.iter().any(|node| {
            node.node_kind == "heading"
                && node.structure_path.as_deref() == Some("Facts")
                && node.source_span_ids.contains(&"span:chunk:1".to_string())
        }));
    }

    #[test]
    fn markdown_semantic_units_collect_section_and_reference_signals() {
        let text = "# Facts\n\nDebra Paynter paid $1,250 on April 1, 2026 under ORS 90.320.\n";
        let text_chunks = vec![chunk("chunk:1", 0, text.len() as u64, "Facts")];
        let (mut document, mut nodes) = build_markdown_ast_graph(
            "matter:1",
            "doc:1",
            &context(),
            "index:1",
            text,
            &sha256_hex(text.as_bytes()),
            &text_chunks,
            &[],
            &[],
            &[],
        );
        let mut chunks = Vec::<ExtractedTextChunk>::new();
        let mut text_chunks = text_chunks;
        let mut evidence_spans = Vec::<EvidenceSpan>::new();
        let mut mentions = vec![
            mention("mention:party", "party", "Debra Paynter", text, 0),
            mention("mention:money", "money", "$1,250", text, 0),
            mention("mention:date", "date", "April 1, 2026", text, 0),
            mention("mention:statute", "statute", "ORS 90.320", text, 0),
        ];
        let mut facts = Vec::<CaseFact>::new();
        let mut suggestions = Vec::<TimelineSuggestion>::new();
        attach_markdown_ast_node_ids_to_records(
            &mut nodes,
            &mut chunks,
            &mut text_chunks,
            &mut evidence_spans,
            &mut mentions,
            &mut facts,
            &mut suggestions,
        );
        let units = build_markdown_semantic_units("matter:1", "doc:1", &mut document, &nodes);
        assert!(document.semantic_unit_count >= 2);
        assert!(document.entity_mention_count > 0);
        assert!(document.citation_count > 0);
        assert!(
            units
                .iter()
                .any(|unit| unit.semantic_role == "section_heading")
        );
        assert!(units.iter().any(|unit| {
            unit.semantic_role == "paragraph"
                && unit
                    .entity_mention_ids
                    .contains(&"mention:money".to_string())
                && unit.citation_texts.contains(&"ORS 90.320".to_string())
                && unit.date_texts.contains(&"April 1, 2026".to_string())
                && unit.money_texts.contains(&"$1,250".to_string())
        }));
        assert!(nodes.iter().any(|node| {
            node.node_kind == "paragraph"
                && node.contains_money
                && node.contains_date
                && node.contains_citation
                && node.section_path.as_deref() == Some("Facts")
        }));
    }

    #[test]
    fn markdown_ast_node_ids_are_version_aware() {
        let text = "# Facts\n\nAlpha beta gamma.";
        let chunks = vec![chunk("chunk:1", 0, text.len() as u64, "Facts")];
        let (first, first_nodes) = build_markdown_ast_graph(
            "matter:1",
            "doc:1",
            &context(),
            "index:1",
            text,
            &sha256_hex(text.as_bytes()),
            &chunks,
            &[],
            &[],
            &[],
        );
        let mut second_context = context();
        second_context.document_version_id = Some("version:doc:b".to_string());
        let (second, second_nodes) = build_markdown_ast_graph(
            "matter:1",
            "doc:1",
            &second_context,
            "index:2",
            text,
            &sha256_hex(text.as_bytes()),
            &chunks,
            &[],
            &[],
            &[],
        );
        assert_ne!(
            first.markdown_ast_document_id,
            second.markdown_ast_document_id
        );
        assert_ne!(
            first_nodes[1].markdown_ast_node_id,
            second_nodes[1].markdown_ast_node_id
        );
    }

    #[test]
    fn canonical_entities_group_mentions_and_keep_party_matches_reviewable() {
        let mut mentions = vec![
            EntityMention {
                entity_mention_id: "mention:1".to_string(),
                id: "mention:1".to_string(),
                matter_id: "matter:1".to_string(),
                document_id: "doc:1".to_string(),
                text_chunk_id: Some("chunk:1".to_string()),
                source_span_id: None,
                entity_id: None,
                markdown_ast_node_ids: Vec::new(),
                mention_text: "Debra Paynter".to_string(),
                entity_type: "party".to_string(),
                confidence: 0.76,
                byte_start: Some(0),
                byte_end: Some(13),
                char_start: Some(0),
                char_end: Some(13),
                review_status: "unreviewed".to_string(),
            },
            EntityMention {
                entity_mention_id: "mention:2".to_string(),
                id: "mention:2".to_string(),
                matter_id: "matter:1".to_string(),
                document_id: "doc:1".to_string(),
                text_chunk_id: Some("chunk:1".to_string()),
                source_span_id: None,
                entity_id: None,
                markdown_ast_node_ids: Vec::new(),
                mention_text: "Debra Paynter".to_string(),
                entity_type: "party".to_string(),
                confidence: 0.8,
                byte_start: Some(20),
                byte_end: Some(33),
                char_start: Some(20),
                char_end: Some(33),
                review_status: "unreviewed".to_string(),
            },
        ];
        let parties = vec![CaseParty {
            id: "party:debra".to_string(),
            party_id: "party:debra".to_string(),
            matter_id: "matter:1".to_string(),
            name: "Debra Paynter".to_string(),
            role: "plaintiff".to_string(),
            party_type: "individual".to_string(),
            represented_by: None,
            contact_email: None,
            contact_phone: None,
            notes: None,
        }];
        let entities = canonical_entities_for_mentions("matter:1", &mut mentions, &parties);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].mention_ids.len(), 2);
        assert_eq!(entities[0].party_match_ids, vec!["party:debra"]);
        assert!(
            mentions
                .iter()
                .all(|mention| mention.entity_id == Some(entities[0].entity_id.clone()))
        );
    }

    fn mention(
        id: &str,
        entity_type: &str,
        mention_text: &str,
        text: &str,
        start_hint: usize,
    ) -> EntityMention {
        let start = text[start_hint..]
            .find(mention_text)
            .map(|offset| start_hint + offset)
            .expect("mention text");
        let end = start + mention_text.len();
        EntityMention {
            entity_mention_id: id.to_string(),
            id: id.to_string(),
            matter_id: "matter:1".to_string(),
            document_id: "doc:1".to_string(),
            text_chunk_id: Some("chunk:1".to_string()),
            source_span_id: Some("span:chunk:1".to_string()),
            entity_id: None,
            markdown_ast_node_ids: Vec::new(),
            mention_text: mention_text.to_string(),
            entity_type: entity_type.to_string(),
            confidence: 0.9,
            byte_start: Some(start as u64),
            byte_end: Some(end as u64),
            char_start: Some(start as u64),
            char_end: Some(end as u64),
            review_status: "unreviewed".to_string(),
        }
    }
}
