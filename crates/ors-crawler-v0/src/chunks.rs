use once_cell::sync::Lazy;
use regex::Regex;

use crate::hash::sha256_hex;
use crate::models::{LegalTextVersion, Provision, RetrievalChunk};
use crate::voyage::{VOYAGE_4_LARGE, estimate_tokens};

const CHUNK_VERSION: &str = "3.0";
const CHUNKING_STRATEGY: &str = "legal_structure_adaptive_v3";
const CONTEXT_WINDOW: usize = 32_000;
const HARD_FAIL_TOKENS: usize = 30_000;
const HEADER_SAFETY_MARGIN: usize = 400;

#[derive(Debug, Clone)]
pub struct ChunkBudgetProfile {
    pub chunk_type: &'static str,
    pub target_max_tokens: usize,
    pub warning_tokens: usize,
    pub hard_fail_tokens: usize,
    pub overlap_tokens: usize,
    pub preserve_atomic: bool,
}

#[derive(Debug, Clone)]
struct TextPart {
    text: String,
    split_reason: String,
}

pub fn get_adaptive_budget(chunk_type: &str, edition_year: i32) -> ChunkBudgetProfile {
    let chunk_type_static = match chunk_type {
        "full_statute" => "full_statute",
        "contextual_provision" => "contextual_provision",
        "definition_block" => "definition_block",
        "exception_block" => "exception_block",
        "deadline_block" => "deadline_block",
        "penalty_block" => "penalty_block",
        "citation_context" => "citation_context",
        "status_note" => "status_note",
        "temporal_effect" => "temporal_effect",
        "lineage_context" => "lineage_context",
        "session_law_context" => "session_law_context",
        "source_note_context" => "source_note_context",
        "section_outline" => "section_outline",
        "chapter_summary" => "chapter_summary",
        _ => "unknown",
    };

    let mut target: usize = match chunk_type {
        "full_statute" => 28_000,
        "contextual_provision" => 26_000,
        "definition_block" => 16_000,
        "exception_block" => 16_000,
        "penalty_block" => 16_000,
        "deadline_block" => 18_000,
        "citation_context" => 14_000,
        "status_note" => 12_000,
        "temporal_effect" => 10_000,
        "lineage_context" => 10_000,
        "session_law_context" => 12_000,
        "source_note_context" => 12_000,
        "section_outline" => 8_000,
        "chapter_summary" => 8_000,
        _ => 24_000,
    };

    if edition_year < 2020 {
        target = target.saturating_sub(1_000);
    }

    let overlap = match chunk_type {
        "full_statute" => 400,
        "contextual_provision" => 200,
        _ => 0,
    };

    ChunkBudgetProfile {
        chunk_type: chunk_type_static,
        target_max_tokens: target,
        warning_tokens: target,
        hard_fail_tokens: HARD_FAIL_TOKENS,
        overlap_tokens: overlap,
        preserve_atomic: matches!(
            chunk_type,
            "definition_block"
                | "exception_block"
                | "deadline_block"
                | "penalty_block"
                | "citation_context"
        ),
    }
}

pub fn create_retrieval_chunk(
    provision: &Provision,
    chunk_type: &str,
    text: &str,
    breadcrumb: String,
    authority_level: i32,
    edition_year: i32,
) -> RetrievalChunk {
    let budget = get_adaptive_budget(chunk_type, edition_year);
    make_provision_chunk(
        provision,
        chunk_type,
        text.to_string(),
        breadcrumb,
        authority_level,
        edition_year,
        &budget,
        1,
        1,
        "none",
        0,
    )
}

pub fn build_chunks_for_provision(
    provision: &Provision,
    edition_year: i32,
    authority_level: i32,
) -> Vec<RetrievalChunk> {
    let breadcrumb = build_provision_breadcrumb(provision);
    let mut chunks = Vec::new();

    chunks.extend(build_adaptive_provision_chunks(
        provision,
        "contextual_provision",
        build_provision_header(provision, "contextual_provision", edition_year),
        &provision.text,
        breadcrumb.clone(),
        authority_level,
        edition_year,
    ));

    let special_types = [
        (provision.is_definition_candidate, "definition_block"),
        (provision.is_exception_candidate, "exception_block"),
        (provision.is_deadline_candidate, "deadline_block"),
        (provision.is_penalty_candidate, "penalty_block"),
    ];

    for (is_candidate, chunk_type) in special_types {
        if is_candidate {
            chunks.extend(build_adaptive_provision_chunks(
                provision,
                chunk_type,
                build_provision_header(provision, chunk_type, edition_year),
                &provision.text,
                breadcrumb.clone(),
                authority_level,
                edition_year,
            ));
        }
    }

    if has_cross_reference(&provision.text) {
        chunks.extend(build_adaptive_provision_chunks(
            provision,
            "citation_context",
            build_provision_header(provision, "citation_context", edition_year),
            &provision.text,
            breadcrumb,
            authority_level,
            edition_year,
        ));
    }

    chunks
}

pub fn build_full_statute_chunks(
    version: &LegalTextVersion,
    _root_provision_id: &str,
    edition_year: i32,
    authority_level: i32,
) -> Vec<RetrievalChunk> {
    let title = version.title.clone().unwrap_or_default();
    let header = format!(
        "Oregon Revised Statutes. {} Edition.\nCitation: {}\nTitle: {}\nStatus: {}\nChapter: {}\n\nFull statute text:",
        edition_year, version.citation, title, version.status, version.chapter
    );
    let breadcrumb = format!(
        "Oregon > ORS > Chapter {} > {}",
        version.chapter, version.citation
    );
    let budget = get_adaptive_budget("full_statute", edition_year);

    let parts = build_adaptive_text_parts(&header, &version.text, &budget);
    let part_count = parts.len();

    parts
        .into_iter()
        .enumerate()
        .map(|(idx, part)| {
            let text = format_part_text(&header, &part.text, idx + 1, part_count);
            make_full_statute_chunk(
                version,
                text,
                breadcrumb.clone(),
                authority_level,
                edition_year,
                &budget,
                idx + 1,
                part_count,
                &part.split_reason,
                if part_count > 1 {
                    budget.overlap_tokens
                } else {
                    0
                },
            )
        })
        .collect()
}

fn build_adaptive_provision_chunks(
    provision: &Provision,
    chunk_type: &str,
    header: String,
    body_text: &str,
    breadcrumb: String,
    authority_level: i32,
    edition_year: i32,
) -> Vec<RetrievalChunk> {
    let mut budget = get_adaptive_budget(chunk_type, edition_year);
    let parts = build_adaptive_text_parts(&header, body_text, &budget);
    let part_count = parts.len();
    let split_overlap = if part_count > 1 {
        if budget.preserve_atomic && budget.overlap_tokens == 0 {
            budget.overlap_tokens = 100;
        }
        budget.overlap_tokens
    } else {
        0
    };

    parts
        .into_iter()
        .enumerate()
        .map(|(idx, part)| {
            let text = format_part_text(&header, &part.text, idx + 1, part_count);
            make_provision_chunk(
                provision,
                chunk_type,
                text,
                breadcrumb.clone(),
                authority_level,
                edition_year,
                &budget,
                idx + 1,
                part_count,
                &part.split_reason,
                split_overlap,
            )
        })
        .collect()
}

fn build_adaptive_text_parts(
    header: &str,
    body_text: &str,
    budget: &ChunkBudgetProfile,
) -> Vec<TextPart> {
    let full_text = format!("{header}\n\n{body_text}");
    let planned_full_tokens = planning_token_count(&full_text);

    if planned_full_tokens
        <= budget
            .target_max_tokens
            .saturating_sub(HEADER_SAFETY_MARGIN)
    {
        return vec![TextPart {
            text: body_text.trim().to_string(),
            split_reason: "none".to_string(),
        }];
    }

    let full_tokens = token_count(&full_text);
    if full_tokens <= budget.target_max_tokens {
        return vec![TextPart {
            text: body_text.trim().to_string(),
            split_reason: "none".to_string(),
        }];
    }

    let header_tokens = planning_token_count(header);
    let body_budget = budget
        .target_max_tokens
        .saturating_sub(header_tokens)
        .saturating_sub(HEADER_SAFETY_MARGIN)
        .max(1_000);

    let overlap = if budget.preserve_atomic && budget.overlap_tokens == 0 {
        100
    } else {
        budget.overlap_tokens
    };

    split_text_by_legal_boundaries(body_text, body_budget, overlap)
}

fn make_provision_chunk(
    provision: &Provision,
    chunk_type: &str,
    text: String,
    breadcrumb: String,
    authority_level: i32,
    edition_year: i32,
    budget: &ChunkBudgetProfile,
    part_index: usize,
    part_count: usize,
    split_reason: &str,
    overlap_tokens: usize,
) -> RetrievalChunk {
    let token_count = token_count(&text);
    let source_id = provision.provision_id.clone();
    let chunk_id = build_chunk_id(&source_id, chunk_type, part_index, part_count, &text);

    RetrievalChunk {
        chunk_id,
        chunk_type: chunk_type.to_string(),
        text: text.clone(),
        breadcrumb,
        source_provision_id: Some(provision.provision_id.clone()),
        source_version_id: None,
        parent_version_id: provision.version_id.clone(),
        canonical_id: provision.canonical_id.clone(),
        citation: provision.citation.clone(),
        jurisdiction_id: "or:state".to_string(),
        authority_level,
        authority_family: provision
            .authority_family
            .clone()
            .or_else(|| Some("ORS".to_string())),
        corpus_id: provision
            .corpus_id
            .clone()
            .or_else(|| Some("or:ors".to_string())),
        authority_type: provision
            .authority_type
            .clone()
            .or_else(|| Some("statute".to_string())),
        effective_date: provision.effective_date.clone(),
        chapter: provision
            .chapter
            .clone()
            .or_else(|| Some(chapter_from_citation(&provision.citation))),
        source_page_start: provision.source_page_start,
        source_page_end: provision.source_page_end,
        edition_year,
        embedding_model: None,
        embedding_dim: None,
        embedding: None,
        embedding_input_hash: sha256_hex(text.as_bytes()),
        embedding_policy: embedding_policy(chunk_type).map(str::to_string),
        answer_policy: Some("authoritative_support".to_string()),
        chunk_schema_version: Some("1.0.0".to_string()),
        retrieval_profile: Some(format!("ors_{}_v3", chunk_type)),
        search_weight: Some(search_weight(chunk_type)),
        embedding_input_type: Some("document".to_string()),
        embedding_output_dtype: Some("float".to_string()),
        embedded_at: None,
        embedding_profile: None,
        embedding_source_dimension: None,
        source_kind: Some("Provision".to_string()),
        source_id: Some(source_id),
        token_count: Some(token_count),
        max_tokens: Some(budget.target_max_tokens),
        context_window: Some(CONTEXT_WINDOW),
        chunking_strategy: Some(CHUNKING_STRATEGY.to_string()),
        chunk_version: Some(CHUNK_VERSION.to_string()),
        overlap_tokens: Some(overlap_tokens),
        split_reason: Some(split_reason.to_string()),
        part_index: Some(part_index),
        part_count: Some(part_count),
        is_definition_candidate: provision.is_definition_candidate,
        is_exception_candidate: provision.is_exception_candidate,
        is_penalty_candidate: provision.is_penalty_candidate,
        heading_path: provision.heading_path.clone(),
        structural_context: provision.structural_context.clone(),
    }
}

fn make_full_statute_chunk(
    version: &LegalTextVersion,
    text: String,
    breadcrumb: String,
    authority_level: i32,
    edition_year: i32,
    budget: &ChunkBudgetProfile,
    part_index: usize,
    part_count: usize,
    split_reason: &str,
    overlap_tokens: usize,
) -> RetrievalChunk {
    let token_count = token_count(&text);
    let chunk_id = build_chunk_id(
        &version.version_id,
        "full_statute",
        part_index,
        part_count,
        &text,
    );

    RetrievalChunk {
        chunk_id,
        chunk_type: "full_statute".to_string(),
        text: text.clone(),
        breadcrumb,
        source_provision_id: None,
        source_version_id: Some(version.version_id.clone()),
        parent_version_id: version.version_id.clone(),
        canonical_id: version.canonical_id.clone(),
        citation: version.citation.clone(),
        jurisdiction_id: "or:state".to_string(),
        authority_level,
        authority_family: version
            .authority_family
            .clone()
            .or_else(|| Some("ORS".to_string())),
        corpus_id: version
            .corpus_id
            .clone()
            .or_else(|| Some("or:ors".to_string())),
        authority_type: version
            .authority_type
            .clone()
            .or_else(|| Some("statute".to_string())),
        effective_date: version.effective_date.clone(),
        chapter: Some(version.chapter.clone()),
        source_page_start: version.source_page_start,
        source_page_end: version.source_page_end,
        edition_year,
        embedding_model: None,
        embedding_dim: None,
        embedding: None,
        embedding_input_hash: sha256_hex(text.as_bytes()),
        embedding_policy: Some("embed_primary".to_string()),
        answer_policy: Some("authoritative_support".to_string()),
        chunk_schema_version: Some("1.0.0".to_string()),
        retrieval_profile: Some("ors_full_statute_v3".to_string()),
        search_weight: Some(1.0),
        embedding_input_type: Some("document".to_string()),
        embedding_output_dtype: Some("float".to_string()),
        embedded_at: None,
        embedding_profile: None,
        embedding_source_dimension: None,
        source_kind: Some("LegalTextVersion".to_string()),
        source_id: Some(version.version_id.clone()),
        token_count: Some(token_count),
        max_tokens: Some(budget.target_max_tokens),
        context_window: Some(CONTEXT_WINDOW),
        chunking_strategy: Some(CHUNKING_STRATEGY.to_string()),
        chunk_version: Some(CHUNK_VERSION.to_string()),
        overlap_tokens: Some(overlap_tokens),
        split_reason: Some(split_reason.to_string()),
        part_index: Some(part_index),
        part_count: Some(part_count),
        is_definition_candidate: false,
        is_exception_candidate: false,
        is_penalty_candidate: false,
        heading_path: Vec::new(),
        structural_context: None,
    }
}

fn format_part_text(header: &str, body: &str, part_index: usize, part_count: usize) -> String {
    if part_count == 1 {
        format!("{header}\n\n{}", body.trim())
    } else {
        format!(
            "{header}\n\nPart {} of {}.\n\n{}",
            part_index,
            part_count,
            body.trim()
        )
    }
}

fn build_chunk_id(
    source_id: &str,
    chunk_type: &str,
    part_index: usize,
    part_count: usize,
    text: &str,
) -> String {
    let safe_source_id = safe_id(source_id);
    let base = format!("chunk:{safe_source_id}:{chunk_type}:v{CHUNK_VERSION}");

    if part_count == 1 {
        base
    } else {
        format!(
            "{base}:part:{}-of-{}:{}",
            part_index,
            part_count,
            short_hash(text)
        )
    }
}

fn safe_id(id: &str) -> String {
    id.replace("::", ":")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, ':' | '.' | '_' | '-' | '@') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn build_provision_header(provision: &Provision, chunk_type: &str, edition_year: i32) -> String {
    let chapter = chapter_from_citation(&provision.citation);
    match chunk_type {
        "contextual_provision" => format!(
            "Oregon Revised Statutes. {} Edition.\nCitation: {}\nParent statute: {}\nProvision type: {}\nChapter: {}\nBreadcrumb: {}\n\nProvision text:",
            edition_year,
            provision.display_citation,
            provision.citation,
            provision.provision_type,
            chapter,
            build_provision_breadcrumb(provision)
        ),
        "definition_block" => format!(
            "Oregon Revised Statutes. {} Edition.\nDefinition block.\nTerm: {}\nScope: {}\nSource citation: {}\nChapter: {}\n\nDefinition text:",
            edition_year,
            provision.display_citation,
            provision
                .structural_context
                .clone()
                .unwrap_or_else(|| "source provision".to_string()),
            provision.citation,
            chapter
        ),
        "exception_block" => format!(
            "Oregon Revised Statutes. {} Edition.\nException, exclusion, limitation, unless or notwithstanding block.\nSource citation: {}\nProvision: {}\nChapter: {}\nBreadcrumb: {}\n\nException text:",
            edition_year,
            provision.citation,
            provision.display_citation,
            chapter,
            build_provision_breadcrumb(provision)
        ),
        "deadline_block" => format!(
            "Oregon Revised Statutes. {} Edition.\nDeadline, time limit, operative date or filing period block.\nSource citation: {}\nProvision: {}\nChapter: {}\nBreadcrumb: {}\n\nDeadline text:",
            edition_year,
            provision.citation,
            provision.display_citation,
            chapter,
            build_provision_breadcrumb(provision)
        ),
        "penalty_block" => format!(
            "Oregon Revised Statutes. {} Edition.\nPenalty, fine, revocation, sanction or license consequence block.\nSource citation: {}\nProvision: {}\nChapter: {}\nBreadcrumb: {}\n\nPenalty text:",
            edition_year,
            provision.citation,
            provision.display_citation,
            chapter,
            build_provision_breadcrumb(provision)
        ),
        "citation_context" => format!(
            "Oregon Revised Statutes. {} Edition.\nCitation context.\nSource citation: {}\nProvision: {}\nChapter: {}\nBreadcrumb: {}\n\nText around statutory cross-references:",
            edition_year,
            provision.citation,
            provision.display_citation,
            chapter,
            build_provision_breadcrumb(provision)
        ),
        _ => format!(
            "Oregon Revised Statutes. {} Edition.\nCitation: {}\nChapter: {}\n\nText:",
            edition_year, provision.display_citation, chapter
        ),
    }
}

fn build_provision_breadcrumb(provision: &Provision) -> String {
    if provision.heading_path.is_empty() {
        format!(
            "Oregon > ORS > {} > {}",
            provision.citation, provision.display_citation
        )
    } else {
        format!(
            "Oregon > ORS > {} > {} > {}",
            provision.citation,
            provision.heading_path.join(" > "),
            provision.display_citation
        )
    }
}

fn chapter_from_citation(citation: &str) -> String {
    citation
        .strip_prefix("ORS ")
        .and_then(|s| s.split('.').next())
        .unwrap_or_default()
        .to_string()
}

fn embedding_policy(chunk_type: &str) -> Option<&'static str> {
    match chunk_type {
        "full_statute" | "contextual_provision" => Some("embed_primary"),
        "definition_block" | "exception_block" | "deadline_block" | "penalty_block"
        | "citation_context" => Some("embed_special"),
        _ => None,
    }
}

fn search_weight(chunk_type: &str) -> f32 {
    match chunk_type {
        "contextual_provision" => 1.20,
        "definition_block" | "exception_block" => 1.15,
        "deadline_block" | "penalty_block" => 1.10,
        "citation_context" => 1.05,
        _ => 1.0,
    }
}

fn token_count(text: &str) -> usize {
    estimate_tokens(text, VOYAGE_4_LARGE.model)
}

fn planning_token_count(text: &str) -> usize {
    ((text.chars().count() as f64) / 5.2).ceil() as usize
}

fn has_cross_reference(text: &str) -> bool {
    static ORS_REF: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\bORS\s+\d+[A-Z]?\.\d+").expect("valid ORS regex"));
    ORS_REF.is_match(text)
}

fn split_text_by_legal_boundaries(
    text: &str,
    body_budget_tokens: usize,
    overlap_tokens: usize,
) -> Vec<TextPart> {
    if planning_token_count(text) <= body_budget_tokens {
        return vec![TextPart {
            text: text.trim().to_string(),
            split_reason: "none".to_string(),
        }];
    }

    if let Some(parts) = split_with_units(
        split_legal_units(text),
        body_budget_tokens,
        "legal_boundary",
    ) {
        return apply_overlap(parts, overlap_tokens);
    }

    if let Some(parts) = split_with_units(
        split_paragraphs(text),
        body_budget_tokens,
        "paragraph_boundary",
    ) {
        return apply_overlap(parts, overlap_tokens);
    }

    if let Some(parts) = split_with_units(
        split_sentences(text),
        body_budget_tokens,
        "sentence_boundary",
    ) {
        return apply_overlap(parts, overlap_tokens);
    }

    if let Some(parts) =
        split_with_units(split_clauses(text), body_budget_tokens, "clause_boundary")
    {
        return apply_overlap(parts, overlap_tokens);
    }

    apply_overlap(
        split_whitespace_fallback(text, body_budget_tokens),
        overlap_tokens,
    )
}

fn split_with_units(
    units: Vec<String>,
    budget_tokens: usize,
    split_reason: &str,
) -> Option<Vec<TextPart>> {
    if units.len() <= 1
        || units
            .iter()
            .any(|unit| planning_token_count(unit) > budget_tokens)
    {
        return None;
    }

    let mut parts = Vec::new();
    let mut current = String::new();

    for unit in units {
        let candidate = if current.is_empty() {
            unit.clone()
        } else {
            format!("{}\n\n{}", current.trim_end(), unit.trim_start())
        };

        if planning_token_count(&candidate) <= budget_tokens {
            current = candidate;
        } else {
            if !current.trim().is_empty() {
                parts.push(TextPart {
                    text: current.trim().to_string(),
                    split_reason: split_reason.to_string(),
                });
            }
            current = unit;
        }
    }

    if !current.trim().is_empty() {
        parts.push(TextPart {
            text: current.trim().to_string(),
            split_reason: split_reason.to_string(),
        });
    }

    Some(parts)
}

fn split_legal_units(text: &str) -> Vec<String> {
    static LEGAL_MARKER: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^\s*(?:\(\d+\)|\([a-z]\)|\([A-Z]\)|Sec\.\s+\d+\.)").unwrap());
    let paragraphs = split_paragraphs(text);
    if paragraphs.len() > 1 && paragraphs.iter().any(|p| LEGAL_MARKER.is_match(p)) {
        paragraphs
    } else {
        vec![text.trim().to_string()]
    }
}

fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut units = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();

    for (idx, (byte_idx, ch)) in chars.iter().enumerate() {
        if !matches!(ch, '.' | ';' | ':') {
            continue;
        }
        let next_is_space = chars
            .get(idx + 1)
            .map(|(_, next)| next.is_whitespace())
            .unwrap_or(false);
        if next_is_space {
            let end = byte_idx + ch.len_utf8();
            let unit = text[start..end].trim();
            if !unit.is_empty() {
                units.push(unit.to_string());
            }
            start = end;
            while start < text.len() {
                let next = text[start..].chars().next().unwrap();
                if !next.is_whitespace() {
                    break;
                }
                start += next.len_utf8();
            }
        }
    }

    let tail = text[start..].trim();
    if !tail.is_empty() {
        units.push(tail.to_string());
    }

    units
}

fn split_clauses(text: &str) -> Vec<String> {
    static CLAUSE_BOUNDARY: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(provided that|except that|unless|notwithstanding|subject to|or|and)\b")
            .expect("valid clause regex")
    });
    split_by_regex_boundary(text, &CLAUSE_BOUNDARY)
}

fn split_by_regex_boundary(text: &str, regex: &Regex) -> Vec<String> {
    let mut units = Vec::new();
    let mut last = 0;
    for mat in regex.find_iter(text) {
        if mat.start() > last {
            let unit = text[last..mat.start()].trim();
            if !unit.is_empty() {
                units.push(unit.to_string());
            }
        }
        last = mat.start();
    }
    let tail = text[last..].trim();
    if !tail.is_empty() {
        units.push(tail.to_string());
    }
    if units.is_empty() && !text.trim().is_empty() {
        units.push(text.trim().to_string());
    }
    units
}

fn split_whitespace_fallback(text: &str, budget_tokens: usize) -> Vec<TextPart> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return split_char_fallback(text, budget_tokens);
    }

    let mut parts = Vec::new();
    let mut current = String::new();

    for word in words {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };

        if planning_token_count(&candidate) <= budget_tokens {
            current = candidate;
        } else if current.is_empty() {
            parts.extend(split_char_fallback(word, budget_tokens));
        } else {
            parts.push(TextPart {
                text: current,
                split_reason: "whitespace_fallback".to_string(),
            });
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        parts.push(TextPart {
            text: current,
            split_reason: "whitespace_fallback".to_string(),
        });
    }

    parts
}

fn split_char_fallback(text: &str, budget_tokens: usize) -> Vec<TextPart> {
    let max_chars = (budget_tokens.max(1) * 5).max(1);
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(max_chars)
        .map(|chunk| TextPart {
            text: chunk.iter().collect(),
            split_reason: "char_fallback".to_string(),
        })
        .collect()
}

fn apply_overlap(parts: Vec<TextPart>, overlap_tokens: usize) -> Vec<TextPart> {
    if overlap_tokens == 0 || parts.len() <= 1 {
        return parts;
    }

    let mut overlapped = Vec::with_capacity(parts.len());
    let mut previous_text: Option<String> = None;

    for mut part in parts {
        if let Some(previous) = previous_text {
            let overlap = trailing_words(&previous, overlap_tokens);
            if !overlap.is_empty() {
                part.text = format!("{overlap}\n\n{}", part.text);
            }
        }
        previous_text = Some(part.text.clone());
        overlapped.push(part);
    }

    overlapped
}

fn trailing_words(text: &str, count: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= count {
        words.join(" ")
    } else {
        words[words.len() - count..].join(" ")
    }
}

fn short_hash(s: &str) -> String {
    sha256_hex(s.as_bytes()).replace("sha256:", "")[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        build_full_statute_chunks, build_provision_header, get_adaptive_budget,
        split_text_by_legal_boundaries,
    };
    use crate::hash::sha256_hex;
    use crate::models::{LegalTextVersion, Provision, RetrievalChunk};

    fn test_provision() -> Provision {
        Provision {
            provision_id: "or:ors:1.001@2025::p:root".to_string(),
            version_id: "or:ors:1.001@2025".to_string(),
            canonical_id: "or:ors:1.001".to_string(),
            citation: "ORS 1.001".to_string(),
            display_citation: "ORS 1.001".to_string(),
            local_path: vec!["root".to_string()],
            provision_type: "section".to_string(),
            text: "test".to_string(),
            normalized_text: "test".to_string(),
            order_index: 0,
            depth: 0,
            text_hash: "hash".to_string(),
            is_implied: false,
            is_definition_candidate: false,
            is_exception_candidate: false,
            is_deadline_candidate: false,
            is_penalty_candidate: false,
            ..Default::default()
        }
    }

    fn test_version() -> LegalTextVersion {
        LegalTextVersion {
            version_id: "or:ors:1.001@2025".to_string(),
            canonical_id: "or:ors:1.001".to_string(),
            citation: "ORS 1.001".to_string(),
            title: Some("Test section".to_string()),
            chapter: "1".to_string(),
            edition_year: 2025,
            status: "active".to_string(),
            status_text: None,
            text: "test".to_string(),
            text_hash: "hash".to_string(),
            source_document_id: "source:1".to_string(),
            official_status: "official".to_string(),
            disclaimer_required: true,
            ..Default::default()
        }
    }

    #[test]
    fn budgets_match_v3_spec() {
        assert_eq!(
            get_adaptive_budget("full_statute", 2025).target_max_tokens,
            28_000
        );
        assert_eq!(
            get_adaptive_budget("contextual_provision", 2025).target_max_tokens,
            26_000
        );
        assert_eq!(
            get_adaptive_budget("definition_block", 2025).target_max_tokens,
            16_000
        );
        assert_eq!(
            get_adaptive_budget("exception_block", 2025).target_max_tokens,
            16_000
        );
        assert_eq!(
            get_adaptive_budget("deadline_block", 2025).target_max_tokens,
            18_000
        );
        assert_eq!(
            get_adaptive_budget("penalty_block", 2025).target_max_tokens,
            16_000
        );
        assert_eq!(
            get_adaptive_budget("citation_context", 2025).target_max_tokens,
            14_000
        );
    }

    #[test]
    fn split_full_statute_has_valid_part_metadata() {
        let mut version = test_version();
        let paragraph = format!(
            "(1) {}",
            "alpha beta gamma delta epsilon zeta eta theta ".repeat(120)
        );
        version.text = (0..40)
            .map(|_| paragraph.clone())
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunks = build_full_statute_chunks(&version, "ignored-root", 2025, 1);

        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|c| c.part_count == Some(chunks.len())));
        assert_eq!(chunks[0].part_index, Some(1));
        assert!(chunks[0].chunk_id.contains(":part:1-of-"));
    }

    #[test]
    fn full_statute_chunks_use_version_source() {
        let mut version = test_version();
        version.text =
            "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu".to_string();
        let chunks = build_full_statute_chunks(&version, "ignored-root", 2025, 1);
        let chunk = chunks.first().expect("full statute chunk");

        assert_eq!(chunk.source_provision_id, None);
        assert_eq!(
            chunk.source_version_id.as_deref(),
            Some("or:ors:1.001@2025")
        );
        assert_eq!(chunk.source_kind.as_deref(), Some("LegalTextVersion"));
        assert_eq!(chunk.source_id.as_deref(), Some("or:ors:1.001@2025"));
        assert_eq!(chunk.max_tokens, Some(28_000));
        assert_eq!(chunk.context_window, Some(32_000));
        assert_eq!(chunk.chunk_version.as_deref(), Some("3.0"));
    }

    #[test]
    fn embedding_hash_changes_when_header_changes() {
        let provision = test_provision();
        let h1 = build_provision_header(&provision, "contextual_provision", 2025);
        let h2 = build_provision_header(&provision, "contextual_provision", 2019);
        let text1 = format!("{h1}\n\n{}", provision.text);
        let text2 = format!("{h2}\n\n{}", provision.text);

        assert_ne!(sha256_hex(text1.as_bytes()), sha256_hex(text2.as_bytes()));
    }

    #[test]
    fn paragraph_first_splitter_preserves_paragraphs() {
        let text =
            "First paragraph has text.\n\nSecond paragraph has text.\n\nThird paragraph has text.";
        let parts = split_text_by_legal_boundaries(text, 5, 0);

        assert!(parts.len() > 1);
        assert!(parts.iter().all(|p| !p.text.contains("\n\n")));
        assert!(parts.iter().all(|p| p.split_reason == "paragraph_boundary"));
    }

    #[test]
    fn sentence_fallback_activates_for_oversized_paragraph() {
        let text = "First sentence has many words. Second sentence has many words. Third sentence has many words.";
        let parts = split_text_by_legal_boundaries(text, 6, 0);

        assert!(parts.len() > 1);
        assert!(parts.iter().all(|p| p.split_reason == "sentence_boundary"));
    }

    #[test]
    fn whitespace_fallback_activates_for_no_boundary_text() {
        let text = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda";
        let parts = split_text_by_legal_boundaries(text, 3, 0);

        assert!(parts.len() > 1);
        assert!(
            parts
                .iter()
                .all(|p| p.split_reason == "whitespace_fallback")
        );
    }

    #[test]
    fn retrieval_chunk_deserializes_legacy_source_shape() {
        let json = r#"{
            "chunk_id":"chunk:legacy",
            "chunk_type":"contextual_provision",
            "text":"text",
            "breadcrumb":"crumb",
            "source_provision_id":"or:ors:1.001@2025::p:root",
            "parent_version_id":"or:ors:1.001@2025",
            "canonical_id":"or:ors:1.001",
            "citation":"ORS 1.001",
            "jurisdiction_id":"or:state",
            "authority_level":1,
            "edition_year":2025,
            "embedding_model":null,
            "embedding_dim":null,
            "embedding":null,
            "embedding_input_hash":"hash",
            "embedding_policy":"embed_primary",
            "answer_policy":"authoritative_support",
            "chunk_schema_version":"1.0.0",
            "retrieval_profile":"ors_contextual_provision_v1",
            "search_weight":1.0,
            "embedding_input_type":"document",
            "embedding_output_dtype":"float",
            "embedded_at":null,
            "source_kind":null,
            "source_id":null,
            "is_definition_candidate": false,
            "is_exception_candidate": false,
            "is_penalty_candidate": false
        }"#;

        let chunk: RetrievalChunk = serde_json::from_str(json).expect("legacy chunk");
        assert_eq!(
            chunk.source_provision_id.as_deref(),
            Some("or:ors:1.001@2025::p:root")
        );
        assert_eq!(chunk.source_version_id, None);
        assert_eq!(chunk.token_count, None);
        assert_eq!(chunk.chunk_version, None);
    }
}
