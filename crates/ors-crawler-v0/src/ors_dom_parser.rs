use crate::chunks::{build_chunks_for_provision, build_full_statute_chunks};
use crate::citations::extract_citation_mentions;
use crate::hash::{sha256_hex, stable_id};
use crate::models::{
    Amendment, ChapterFrontMatter, ChapterHeading, ChapterTocEntry, HtmlParagraph,
    LegalTextIdentity, LegalTextVersion, ParsedChapter, ParserDiagnostic, ParserDiagnostics,
    Provision, ReservedRange, SourceDocument, SourceNote, TitleChapterEntry,
};
use crate::text::{
    clean_parser_text, is_all_caps_heading, is_blank, is_reserved_expansion_text,
    is_reserved_tail_heading, is_rule_line, normalize_for_hash, normalize_ws,
    strip_trailing_period,
};
use anyhow::{Result, anyhow};
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static SECTION_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\d{1,3})\.(\d{3,4})\s+(.*)$").unwrap());

static SECTION_ONLY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\d{1,3})\.(\d{3,4})\.?\s*$").unwrap());

static STATUS_REPEALED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\brepealed\b").unwrap());

static STATUS_RENUMBERED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\brenumbered\b").unwrap());

static STATUS_FORMERLY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bformerly\b").unwrap());
static STATUS_NOTE_REF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bsee note(?:\s+following|\s+under)?\b").unwrap());

static LEADING_MARKER_CHAIN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*((?:\([0-9A-Za-z]+\))+)\s*(.*)$"#).unwrap());

static MARKER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\(([0-9A-Za-z]+)\)"#).unwrap());
static ARTICLE_HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^ARTICLE\s+[IVXLC0-9]+(?:\s*[.\-]\s*.+)?$").unwrap());
static ROMAN_NUMERAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:x{0,3}(?:ix|iv|vi{0,3}|i{1,3}))$").unwrap());
static NOTE_HEADING_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^note:?$").unwrap());
static ENACTED_NOTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\bwas enacted into law\b.*\b(not added to|preface to oregon revised statutes)\b",
    )
    .unwrap()
});
static PARENTHETICAL_HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\(([^)]+)\)$").unwrap());
static EDITION_MARKER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b20\d{2}\s+EDITION\b").unwrap());
static TITLE_NUMBER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^TITLE\s+(\d+)\b").unwrap());
static CHAPTER_ENTRY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^Chapter\s+(\d{1,3})\s+(.+)$").unwrap());
static LEGISLATIVE_HISTORY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*(\[[12]\d{3}\s+c\.[^\]]+\])\s*$").unwrap());
static SESSION_LAW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([12]\d{3})\s+c\.([0-9A-Za-z]+)(?:\s+\x{00A7}+\s*([0-9A-Za-z.\-]+))?").unwrap()
});
static DATE_NOTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(becomes operative|is operative|becomes effective|takes effect|applies to|is repealed|expires|sunsets|notwithstanding the effective date)\b").unwrap()
});

#[derive(Debug, Clone)]
struct Para {
    order: usize,
    paragraph_id: String,
    raw_html: String,
    raw_text: String,
    text: String,
    bold_text: String,
    has_bold: bool,
    has_underline: bool,
    has_italic: bool,
    align: Option<String>,
    margin_left: Option<String>,
    text_indent: Option<String>,
    style_raw: Option<String>,
    class_hint: Option<String>,
    is_empty: bool,
    is_heading: bool,
}

#[derive(Debug, Clone)]
enum SectionParaKind {
    Content,
    StructuralHeading,
    Note,
    Separator,
    ReservedTail,
}

#[derive(Debug, Clone)]
struct SectionPara {
    text: String,
    kind: SectionParaKind,
    order: usize,
    paragraph_id: String,
    heading_path: Vec<String>,
}

#[derive(Debug, Clone)]
struct SectionDraft {
    citation: String,
    caption: Option<String>,
    heading: Option<String>,
    paragraphs: Vec<SectionPara>,
    status_text: Option<String>,
    paragraph_start_order: usize,
    paragraph_end_order: usize,
}

#[derive(Debug, Clone, Default)]
struct BodyParseResult {
    sections: Vec<SectionDraft>,
    diagnostics: ParserDiagnostics,
}

pub fn parse_ors_chapter_html(
    html: &str,
    source_url: &str,
    chapter: &str,
    edition_year: i32,
) -> Result<ParsedChapter> {
    let raw_hash = sha256_hex(html);
    let normalized_full_text = normalize_ws(&html_to_text(html)?);
    let normalized_hash = sha256_hex(normalized_full_text.as_bytes());

    let source_document_id = format!("src:orleg:ors:chapter:{:0>3}@{}", chapter, edition_year);

    let mut source_document = SourceDocument {
        source_document_id: source_document_id.clone(),
        source_provider: "oregon_legislature".to_string(),
        source_kind: "official_online_database".to_string(),
        url: source_url.to_string(),
        chapter: chapter.to_string(),
        corpus_id: Some("or:ors".to_string()),
        edition_id: Some(format!("or:ors@{}", edition_year)),
        authority_family: Some("ORS".to_string()),
        authority_type: Some("statute".to_string()),
        title: None,
        source_type: Some("official_online_database".to_string()),
        file_name: None,
        page_count: None,
        effective_date: None,
        copyright_status: None,
        chapter_title: None,
        edition_year,
        html_encoding: Some("windows-1252".to_string()),
        source_path: Some(source_url.to_string()),
        paragraph_count: None,
        first_body_paragraph_index: None,
        parser_profile: Some("ors_html_semantics_v2".to_string()),
        official_status: "official_online_not_official_print".to_string(),
        disclaimer_required: true,
        raw_hash,
        normalized_hash,
    };

    let paras = html_to_paragraphs(html)?;
    source_document.paragraph_count = Some(paras.len());
    source_document.first_body_paragraph_index = paras
        .iter()
        .find(|p| is_body_section_start(p, chapter))
        .map(|p| p.order);
    source_document.chapter_title = derive_chapter_title(&paras, chapter);
    let html_paragraphs_debug =
        build_html_paragraph_debug(&paras, chapter, edition_year, &source_document_id);
    let (chapter_front_matter, title_chapter_entries) =
        parse_title_front_matter(&paras, chapter, edition_year, &source_document_id);
    let toc_captions = parse_toc_captions(&paras, chapter);
    let chapter_toc_entries =
        parse_chapter_toc_entries(&paras, chapter, edition_year, &source_document_id);
    let reserved_ranges = parse_reserved_ranges(&paras, chapter, edition_year, &source_document_id);
    let body = parse_body_sections(&paras, chapter, &toc_captions)?;
    let sections = body.sections;
    let mut parser_diagnostics = body.diagnostics;
    parser_diagnostics.chapter = chapter.to_string();
    parser_diagnostics.edition_year = edition_year;
    let parser_diagnostic_rows = build_parser_diagnostic_rows(
        &parser_diagnostics,
        &source_document_id,
        chapter,
        edition_year,
    );

    let mut identities = Vec::new();
    let mut versions = Vec::new();
    let mut provisions = Vec::new();
    let mut citations = Vec::new();
    let mut chunks = Vec::new();
    let mut headings = Vec::new();
    let mut source_notes = Vec::new();
    let mut amendments = Vec::new();

    let mut heading_seen: HashMap<String, usize> = HashMap::new();
    let primary_section_indices = select_primary_section_indices(&sections);
    let mut primary_content_hashes: HashMap<String, String> = HashMap::new();
    for idx in &primary_section_indices {
        if let Some(draft) = sections.get(*idx) {
            primary_content_hashes.insert(draft.citation.clone(), section_content_hash(draft));
        }
    }

    let mut selected_section_order = 0usize;

    for (section_order, draft) in sections.iter().enumerate() {
        let canonical_id = format!("or:ors:{}", draft.citation);
        let version_id = format!("{}@{}", canonical_id, edition_year);

        if !primary_section_indices.contains(&section_order) {
            let primary_hash = primary_content_hashes
                .get(&draft.citation)
                .map(String::as_str);
            source_notes.push(build_duplicate_section_source_note(
                draft,
                &canonical_id,
                &version_id,
                &source_document_id,
                primary_hash,
            ));
            continue;
        }

        // Extract heading from draft.heading if present
        if let Some(h) = &draft.heading {
            if !heading_seen.contains_key(h) {
                let order = heading_seen.len();
                heading_seen.insert(h.clone(), order);
                headings.push(ChapterHeading {
                    heading_id: format!(
                        "or:ors:chapter:{}@{}::heading:{}",
                        chapter,
                        edition_year,
                        stable_id(h)
                    ),
                    chapter: chapter.to_string(),
                    text: h.clone(),
                    order_index: order,
                });
            }
        }

        // Also extract structural headings from within section paragraphs
        for p in &draft.paragraphs {
            if matches!(p.kind, SectionParaKind::StructuralHeading) {
                if !heading_seen.contains_key(&p.text) {
                    let order = heading_seen.len();
                    heading_seen.insert(p.text.clone(), order);
                    headings.push(ChapterHeading {
                        heading_id: format!(
                            "or:ors:chapter:{}@{}::heading:{}",
                            chapter,
                            edition_year,
                            stable_id(&p.text)
                        ),
                        chapter: chapter.to_string(),
                        text: p.text.clone(),
                        order_index: order,
                    });
                }
            }
        }

        let original_full_text = section_content_text(draft);
        let (full_text, history_items) = extract_legislative_history(&original_full_text);
        let status = classify_status(&full_text, draft.status_text.as_deref());

        let title = draft
            .caption
            .clone()
            .or_else(|| toc_captions.get(&draft.citation).cloned())
            .or_else(|| derive_fallback_title(draft));

        identities.push(LegalTextIdentity {
            canonical_id: canonical_id.clone(),
            citation: format!("ORS {}", draft.citation),
            jurisdiction_id: "or:state".to_string(),
            authority_family: "ORS".to_string(),
            corpus_id: Some("or:ors".to_string()),
            authority_type: Some("statute".to_string()),
            authority_level: Some(90),
            effective_date: None,
            title: title.clone(),
            chapter: chapter.to_string(),
            status: status.clone(),
        });

        versions.push(LegalTextVersion {
            version_id: version_id.clone(),
            canonical_id: canonical_id.clone(),
            citation: format!("ORS {}", draft.citation),
            title,
            chapter: chapter.to_string(),
            edition_year,
            status: status.clone(),
            status_text: draft.status_text.clone(),
            text: full_text.clone(),
            original_text: if original_full_text != full_text {
                Some(original_full_text.clone())
            } else {
                None
            },
            text_hash: sha256_hex(normalize_for_hash(&full_text).as_bytes()),
            paragraph_start_order: Some(draft.paragraph_start_order),
            paragraph_end_order: Some(draft.paragraph_end_order),
            source_paragraph_ids: draft
                .paragraphs
                .iter()
                .filter(|p| matches!(p.kind, SectionParaKind::Content))
                .map(|p| p.paragraph_id.clone())
                .collect(),
            source_document_id: source_document_id.clone(),
            official_status: "official_online_not_official_print".to_string(),
            disclaimer_required: true,
            ..Default::default()
        });

        let version_id_for_notes = version_id.clone();
        for note in build_source_notes_for_section(
            draft,
            &canonical_id,
            &version_id_for_notes,
            &source_document_id,
        ) {
            source_notes.push(note);
        }

        for history in history_items {
            let history_note = build_legislative_history_source_note(
                &history,
                draft,
                &canonical_id,
                &version_id,
                &source_document_id,
            );
            amendments.extend(build_amendments_from_history(
                &history,
                &canonical_id,
                &version_id,
                &source_document_id,
            ));
            source_notes.push(history_note);
        }

        let section_provisions = build_provisions_for_section(
            draft,
            &canonical_id,
            &version_id,
            edition_year,
            selected_section_order,
        );
        selected_section_order += 1;

        let mut root_provision_id: Option<String> = None;
        for p in section_provisions {
            if root_provision_id.is_none() {
                root_provision_id = Some(p.provision_id.clone());
            }
            let citation_mentions = extract_citation_mentions(&p);
            let provision_chunks = build_chunks_for_provision(&p, edition_year, 90);

            citations.extend(citation_mentions);
            chunks.extend(provision_chunks);
            provisions.push(p);
        }

        if status == "active" {
            let version = versions.last().unwrap();
            let root_id = root_provision_id.as_deref().unwrap_or(&version_id);
            chunks.extend(build_full_statute_chunks(
                version,
                root_id,
                edition_year,
                90,
            ));
        }
    }

    Ok(ParsedChapter {
        chapter: chapter.to_string(),
        edition_year,
        source_document,
        identities,
        versions,
        provisions,
        citations,
        chunks,
        headings,
        html_paragraphs_debug,
        chapter_front_matter,
        title_chapter_entries,
        source_notes,
        amendments,
        chapter_toc_entries,
        reserved_ranges,
        time_intervals: Vec::new(),
        parser_diagnostic_rows,
        parser_diagnostics,
    })
}

fn html_to_text(html: &str) -> Result<String> {
    let doc = Html::parse_document(html);
    let body_sel = Selector::parse("body").map_err(|e| anyhow!("selector error: {e:?}"))?;
    let mut out = String::new();

    for body in doc.select(&body_sel) {
        let text = body.text().collect::<Vec<_>>().join(" ");
        out.push_str(&text);
    }

    Ok(normalize_ws(&out))
}

fn html_to_paragraphs(html: &str) -> Result<Vec<Para>> {
    let doc = Html::parse_document(html);
    let p_sel = Selector::parse("p.MsoNormal").map_err(|e| anyhow!("selector error: {e:?}"))?;
    let b_sel = Selector::parse("b").map_err(|e| anyhow!("selector error: {e:?}"))?;
    let u_sel = Selector::parse("u").map_err(|e| anyhow!("selector error: {e:?}"))?;
    let i_sel = Selector::parse("i").map_err(|e| anyhow!("selector error: {e:?}"))?;

    let mut paras = Vec::new();

    for (order, p) in doc.select(&p_sel).enumerate() {
        let raw_text = p.text().collect::<Vec<_>>().join(" ");
        let text = clean_parser_text(&raw_text);
        let bold_text = p
            .select(&b_sel)
            .map(|b| clean_parser_text(&b.text().collect::<Vec<_>>().join(" ")))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        let has_bold = !bold_text.is_empty();
        let has_underline = p.select(&u_sel).next().is_some();
        let has_italic = p.select(&i_sel).next().is_some();
        let style_raw = p.value().attr("style").map(ToString::to_string);
        let class_hint = p.value().attr("class").map(ToString::to_string);
        let align = p
            .value()
            .attr("align")
            .map(ToString::to_string)
            .or_else(|| {
                style_raw
                    .as_deref()
                    .and_then(|style| extract_style_value(style, "text-align"))
            });
        let margin_left = style_raw
            .as_deref()
            .and_then(|style| extract_style_value(style, "margin-left"));
        let text_indent = style_raw
            .as_deref()
            .and_then(|style| extract_style_value(style, "text-indent"));

        paras.push(Para {
            order,
            paragraph_id: format!("p:{order}"),
            raw_html: p.html(),
            raw_text,
            is_empty: is_blank(&text),
            is_heading: is_structural_heading_text(&text, has_underline),
            text,
            bold_text,
            has_bold,
            has_underline,
            has_italic,
            align,
            margin_left,
            text_indent,
            style_raw,
            class_hint,
        });
    }

    Ok(paras)
}

fn extract_style_value(style: &str, key: &str) -> Option<String> {
    style.split(';').find_map(|part| {
        let (name, value) = part.split_once(':')?;
        if name.trim().eq_ignore_ascii_case(key) {
            let value = normalize_ws(value);
            if value.is_empty() { None } else { Some(value) }
        } else {
            None
        }
    })
}

fn build_html_paragraph_debug(
    paras: &[Para],
    chapter: &str,
    edition_year: i32,
    source_document_id: &str,
) -> Vec<HtmlParagraph> {
    paras
        .iter()
        .map(|p| HtmlParagraph {
            paragraph_id: p.paragraph_id.clone(),
            chapter: chapter.to_string(),
            edition_year,
            order_index: p.order,
            raw_html: p.raw_html.clone(),
            raw_text: p.raw_text.clone(),
            cleaned_text: p.text.clone(),
            normalized_text: normalize_ws(&p.text).to_lowercase(),
            bold_text: if p.bold_text.is_empty() {
                None
            } else {
                Some(p.bold_text.clone())
            },
            has_bold: p.has_bold,
            has_underline: p.has_underline,
            has_italic: p.has_italic,
            align: p.align.clone(),
            margin_left: p.margin_left.clone(),
            text_indent: p.text_indent.clone(),
            style_raw: p.style_raw.clone(),
            style_hint: p.style_raw.clone(),
            class_hint: p.class_hint.clone(),
            source_document_id: source_document_id.to_string(),
        })
        .collect()
}

fn parse_toc_captions(paras: &[Para], chapter: &str) -> HashMap<String, String> {
    let mut captions = HashMap::new();
    let mut before_body = true;

    for p in paras {
        if is_body_section_start(p, chapter) {
            before_body = false;
        }

        if !before_body || p.has_bold || p.is_empty {
            continue;
        }

        if let Some(caps) = SECTION_LINE_RE.captures(&p.text) {
            let chap = caps.get(1).unwrap().as_str();
            if chap != chapter {
                continue;
            }

            let section = caps.get(2).unwrap().as_str();
            let caption = normalize_ws(caps.get(3).unwrap().as_str());

            if !caption.is_empty() && !caption.starts_with('[') {
                captions.insert(format!("{}.{}", chap, section), caption);
            }
        }
    }

    captions
}

fn parse_chapter_toc_entries(
    paras: &[Para],
    chapter: &str,
    edition_year: i32,
    source_document_id: &str,
) -> Vec<ChapterTocEntry> {
    let mut entries = Vec::new();
    let mut heading_path: Vec<String> = Vec::new();
    let mut before_body = true;

    for p in paras {
        if is_body_section_start(p, chapter) {
            before_body = false;
        }
        if !before_body || p.is_empty {
            continue;
        }
        if p.is_heading && !EDITION_MARKER_RE.is_match(&p.text) {
            update_heading_stack(&mut heading_path, &p.text);
            continue;
        }

        if let Some(caps) = SECTION_LINE_RE.captures(&p.text) {
            let chap = caps.get(1).unwrap().as_str();
            if chap != chapter {
                continue;
            }
            let citation = format!("{}.{}", chap, caps.get(2).unwrap().as_str());
            let caption = normalize_ws(caps.get(3).unwrap().as_str());
            if caption.is_empty() || caption.starts_with('[') {
                continue;
            }
            let toc_order = entries.len();
            entries.push(ChapterTocEntry {
                toc_entry_id: format!(
                    "toc_entry:{}",
                    stable_id(&format!(
                        "{source_document_id}::{toc_order}::{citation}::{caption}"
                    ))
                ),
                source_document_id: source_document_id.to_string(),
                chapter: chapter.to_string(),
                edition_year,
                citation: Some(format!("ORS {citation}")),
                canonical_id: Some(format!("or:ors:{citation}")),
                caption,
                heading_path: heading_path.clone(),
                toc_order,
                source_paragraph_order: p.order,
                confidence: 0.85,
            });
        } else if p.is_heading {
            update_heading_stack(&mut heading_path, &p.text);
        }
    }

    entries
}

fn parse_title_front_matter(
    paras: &[Para],
    chapter: &str,
    edition_year: i32,
    source_document_id: &str,
) -> (Vec<ChapterFrontMatter>, Vec<TitleChapterEntry>) {
    let mut front_matter = Vec::new();
    let mut entries = Vec::new();
    let mut before_body = true;
    let mut title_number: Option<String> = None;
    let mut title_name: Option<String> = None;
    let mut pending_title_name = false;

    for p in paras {
        if is_body_section_start(p, chapter) {
            before_body = false;
        }
        if !before_body {
            break;
        }
        if p.is_empty {
            continue;
        }

        if let Some(caps) = TITLE_NUMBER_RE.captures(&p.text) {
            title_number = Some(caps.get(1).unwrap().as_str().to_string());
            pending_title_name = true;
            front_matter.push(ChapterFrontMatter {
                front_matter_id: format!(
                    "chapter_front_matter:{}",
                    stable_id(&format!(
                        "{source_document_id}::{}::title::{}",
                        p.order, p.text
                    ))
                ),
                source_document_id: source_document_id.to_string(),
                chapter: chapter.to_string(),
                edition_year,
                title_number: title_number.clone(),
                title_name: title_name.clone(),
                chapter_number: None,
                chapter_name: None,
                text: p.text.clone(),
                source_paragraph_order: p.order,
                front_matter_type: "title_number".to_string(),
                confidence: 0.9,
            });
            continue;
        }

        if pending_title_name
            && title_name.is_none()
            && p.is_heading
            && !EDITION_MARKER_RE.is_match(&p.text)
            && !TITLE_NUMBER_RE.is_match(&p.text)
        {
            title_name = Some(p.text.clone());
            pending_title_name = false;
            front_matter.push(ChapterFrontMatter {
                front_matter_id: format!(
                    "chapter_front_matter:{}",
                    stable_id(&format!(
                        "{source_document_id}::{}::title_name::{}",
                        p.order, p.text
                    ))
                ),
                source_document_id: source_document_id.to_string(),
                chapter: chapter.to_string(),
                edition_year,
                title_number: title_number.clone(),
                title_name: title_name.clone(),
                chapter_number: None,
                chapter_name: None,
                text: p.text.clone(),
                source_paragraph_order: p.order,
                front_matter_type: "title_name".to_string(),
                confidence: 0.82,
            });
            continue;
        }

        if let Some(caps) = CHAPTER_ENTRY_RE.captures(&p.text) {
            let chapter_number = caps.get(1).unwrap().as_str().to_string();
            let chapter_name = strip_trailing_period(caps.get(2).unwrap().as_str());
            let chapter_list_order = entries.len();
            entries.push(TitleChapterEntry {
                title_chapter_entry_id: format!(
                    "title_chapter_entry:{}",
                    stable_id(&format!(
                        "{source_document_id}::{chapter_list_order}::{chapter_number}::{chapter_name}"
                    ))
                ),
                source_document_id: source_document_id.to_string(),
                chapter: chapter.to_string(),
                edition_year,
                title_number: title_number.clone(),
                title_name: title_name.clone(),
                chapter_number: chapter_number.clone(),
                chapter_name: chapter_name.clone(),
                chapter_list_order,
                source_paragraph_order: p.order,
                confidence: 0.85,
            });
            front_matter.push(ChapterFrontMatter {
                front_matter_id: format!(
                    "chapter_front_matter:{}",
                    stable_id(&format!(
                        "{source_document_id}::{}::chapter_entry::{}::{}",
                        p.order, chapter_number, chapter_name
                    ))
                ),
                source_document_id: source_document_id.to_string(),
                chapter: chapter.to_string(),
                edition_year,
                title_number: title_number.clone(),
                title_name: title_name.clone(),
                chapter_number: Some(chapter_number),
                chapter_name: Some(chapter_name),
                text: p.text.clone(),
                source_paragraph_order: p.order,
                front_matter_type: "title_chapter_entry".to_string(),
                confidence: 0.85,
            });
        }
    }

    (front_matter, entries)
}

fn derive_chapter_title(paras: &[Para], chapter: &str) -> Option<String> {
    let chapter_re = Regex::new(&format!(r"(?i)^Chapter\s+0?{}\b\s*(.*)$", chapter)).ok()?;
    for (idx, p) in paras.iter().enumerate() {
        if let Some(caps) = chapter_re.captures(&p.text) {
            let rest = caps
                .get(1)
                .map(|m| normalize_ws(m.as_str()))
                .unwrap_or_default();
            let rest = rest
                .trim_start_matches(|c: char| c == '-' || c == '—' || c.is_whitespace())
                .trim()
                .to_string();
            if !rest.is_empty() {
                return Some(strip_trailing_period(&rest));
            }
            return paras
                .iter()
                .skip(idx + 1)
                .find(|next| !next.is_empty && !EDITION_MARKER_RE.is_match(&next.text))
                .map(|next| strip_trailing_period(&next.text));
        }
    }
    None
}

fn parse_reserved_ranges(
    paras: &[Para],
    chapter: &str,
    edition_year: i32,
    source_document_id: &str,
) -> Vec<ReservedRange> {
    paras
        .iter()
        .filter(|p| is_reserved_tail_heading(&p.text) || is_reserved_expansion_text(&p.text))
        .map(|p| ReservedRange {
            reserved_range_id: format!(
                "reserved_range:{}",
                stable_id(&format!("{source_document_id}::{}::{}", p.order, p.text))
            ),
            source_document_id: source_document_id.to_string(),
            chapter: chapter.to_string(),
            edition_year,
            range_text: p.text.clone(),
            start_chapter: Some(chapter.to_string()),
            end_chapter: None,
            start_title: None,
            end_title: None,
            source_paragraph_order: p.order,
            confidence: 0.8,
        })
        .collect()
}

fn build_parser_diagnostic_rows(
    diagnostics: &ParserDiagnostics,
    source_document_id: &str,
    chapter: &str,
    edition_year: i32,
) -> Vec<ParserDiagnostic> {
    let mut rows = Vec::new();
    let mut push = |diagnostic_type: &str, message: String| {
        rows.push(ParserDiagnostic {
            parser_diagnostic_id: format!(
                "parser_diagnostic:{}",
                stable_id(&format!(
                    "{source_document_id}::{diagnostic_type}::{message}"
                ))
            ),
            source_document_id: source_document_id.to_string(),
            chapter: chapter.to_string(),
            edition_year,
            severity: "info".to_string(),
            diagnostic_type: diagnostic_type.to_string(),
            message,
            source_paragraph_order: None,
            related_id: None,
            parser_profile: "ors_html_semantics_v1".to_string(),
        });
    };

    if diagnostics.reserved_tail_stops > 0 {
        push(
            "reserved_tail",
            format!("reserved tail stops: {}", diagnostics.reserved_tail_stops),
        );
    }
    if diagnostics.skipped_structural_headings > 0 {
        push(
            "structural_heading",
            format!(
                "structural heading paragraphs skipped from legal text: {}",
                diagnostics.skipped_structural_headings
            ),
        );
    }
    if diagnostics.skipped_note_paragraphs > 0 {
        push(
            "source_note",
            format!(
                "source note paragraphs skipped from legal text: {}",
                diagnostics.skipped_note_paragraphs
            ),
        );
    }
    if diagnostics.section_starts_detected == 0 {
        push("no_sections", "no section starts detected".to_string());
    }

    rows
}

fn parse_body_sections(
    paras: &[Para],
    chapter: &str,
    toc_captions: &HashMap<String, String>,
) -> Result<BodyParseResult> {
    let mut sections: Vec<SectionDraft> = Vec::new();
    let mut current: Option<SectionDraft> = None;
    let mut in_body = false;
    let mut in_note_block = false;
    let mut seen_edition_marker = false;
    let document_has_edition_marker = paras.iter().any(|p| EDITION_MARKER_RE.is_match(&p.text));
    let mut heading_stack: Vec<String> = Vec::new();
    let mut diagnostics = ParserDiagnostics {
        total_mso_normal: paras.len(),
        ..Default::default()
    };

    for p in paras {
        if p.is_empty {
            continue;
        }

        if EDITION_MARKER_RE.is_match(&p.text) {
            seen_edition_marker = true;
        }

        if in_body && (is_end_of_chapter(p, chapter) || is_other_chapter_section_start(p, chapter))
        {
            if is_reserved_tail_heading(&p.text) || is_reserved_expansion_text(&p.text) {
                diagnostics.reserved_tail_stops += 1;
            }
            break;
        }

        if !in_body {
            if p.is_heading {
                update_heading_stack(&mut heading_stack, &p.text);
            }

            let body_context_ready =
                !document_has_edition_marker || (seen_edition_marker && !heading_stack.is_empty());
            if body_context_ready && is_body_section_start(p, chapter) {
                in_body = true;
            } else {
                diagnostics.paragraphs_ignored_before_body_start += 1;
                continue;
            }
        }

        if is_body_section_start(p, chapter) {
            in_note_block = false;
            diagnostics.section_starts_detected += 1;
            if let Some(sec) = current.take() {
                sections.push(sec);
            }

            let mut sec = parse_body_section_header(p, chapter, toc_captions)?;
            sec.heading = heading_stack.last().cloned();
            if let Some(last) = heading_stack.last().cloned() {
                sec.paragraphs.push(SectionPara {
                    text: last,
                    kind: SectionParaKind::StructuralHeading,
                    order: p.order,
                    paragraph_id: p.paragraph_id.clone(),
                    heading_path: heading_stack.clone(),
                });
            }
            current = Some(sec);
            continue;
        }

        if let Some(sec) = current.as_mut() {
            if p.is_heading {
                in_note_block = false;
                diagnostics.skipped_structural_headings += 1;
                update_heading_stack(&mut heading_stack, &p.text);
                sec.paragraphs.push(SectionPara {
                    text: p.text.clone(),
                    kind: SectionParaKind::StructuralHeading,
                    order: p.order,
                    paragraph_id: p.paragraph_id.clone(),
                    heading_path: heading_stack.clone(),
                });
                continue;
            }

            let mut kind = classify_section_para(p);
            if in_note_block && is_inline_section_continuation(p, &sec.citation) {
                in_note_block = false;
            }
            if in_note_block && matches!(kind, SectionParaKind::Content) {
                kind = SectionParaKind::Note;
            }
            if matches!(kind, SectionParaKind::ReservedTail) {
                diagnostics.reserved_tail_stops += 1;
                break;
            }
            if matches!(kind, SectionParaKind::Note) {
                diagnostics.skipped_note_paragraphs += 1;
                in_note_block = true;
            }
            if matches!(kind, SectionParaKind::StructuralHeading) {
                in_note_block = false;
                diagnostics.skipped_structural_headings += 1;
                update_heading_stack(&mut heading_stack, &p.text);
            }

            sec.paragraphs.push(SectionPara {
                text: p.text.clone(),
                kind,
                order: p.order,
                paragraph_id: p.paragraph_id.clone(),
                heading_path: heading_stack.clone(),
            });
        }
    }

    if let Some(sec) = current.take() {
        sections.push(sec);
    }

    for sec in &mut sections {
        if let Some(min_order) = sec.paragraphs.iter().map(|p| p.order).min() {
            sec.paragraph_start_order = sec.paragraph_start_order.min(min_order);
        }
        if let Some(max_order) = sec.paragraphs.iter().map(|p| p.order).max() {
            sec.paragraph_end_order = sec.paragraph_end_order.max(max_order);
        }
    }

    Ok(BodyParseResult {
        sections,
        diagnostics,
    })
}

fn select_primary_section_indices(sections: &[SectionDraft]) -> HashSet<usize> {
    let mut best_by_citation: HashMap<&str, (usize, i64)> = HashMap::new();

    for (idx, draft) in sections.iter().enumerate() {
        let score = section_primary_score(draft);
        best_by_citation
            .entry(draft.citation.as_str())
            .and_modify(|current| {
                if score > current.1 {
                    *current = (idx, score);
                }
            })
            .or_insert((idx, score));
    }

    best_by_citation.into_values().map(|(idx, _)| idx).collect()
}

fn section_primary_score(draft: &SectionDraft) -> i64 {
    let text = section_content_text(draft);
    let status = classify_status(&text, draft.status_text.as_deref());
    let status_score = match status.as_str() {
        "active" => 1_000_000,
        "formerly" => 500_000,
        "renumbered" | "repealed" => 100_000,
        _ => 0,
    };

    status_score + normalize_for_hash(&text).len() as i64
}

fn section_content_text(draft: &SectionDraft) -> String {
    draft
        .paragraphs
        .iter()
        .filter(|p| matches!(p.kind, SectionParaKind::Content))
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn section_content_hash(draft: &SectionDraft) -> String {
    sha256_hex(normalize_for_hash(&section_content_text(draft)).as_bytes())
}

fn is_structural_heading_text(text: &str, has_underline: bool) -> bool {
    ARTICLE_HEADING_RE.is_match(text)
        || is_all_caps_heading(text)
        || (has_underline && !text.ends_with('.') && !text.ends_with(':'))
}

fn update_heading_stack(stack: &mut Vec<String>, heading: &str) {
    let heading = normalize_ws(heading);
    if heading.is_empty() {
        return;
    }

    if ARTICLE_HEADING_RE.is_match(&heading) {
        while stack
            .last()
            .is_some_and(|h| ARTICLE_HEADING_RE.is_match(h) || !is_all_caps_heading(h))
        {
            stack.pop();
        }
        stack.push(heading);
    } else if is_all_caps_heading(&heading) {
        stack.clear();
        stack.push(heading);
    } else {
        while stack
            .last()
            .is_some_and(|h| !ARTICLE_HEADING_RE.is_match(h) && !is_all_caps_heading(h))
        {
            stack.pop();
        }
        stack.push(heading);
    }
}

fn classify_section_para(p: &Para) -> SectionParaKind {
    if is_rule_line(&p.text) {
        return SectionParaKind::Separator;
    }

    if NOTE_HEADING_RE.is_match(&p.bold_text) || ENACTED_NOTE_RE.is_match(&p.text) {
        return SectionParaKind::Note;
    }

    if is_reserved_tail_heading(&p.text) || is_reserved_expansion_text(&p.text) {
        return SectionParaKind::ReservedTail;
    }

    if p.is_heading
        || p.has_underline
        || ARTICLE_HEADING_RE.is_match(&p.text)
        || p.text
            .eq_ignore_ascii_case("MANDATORY BOATING SAFETY EDUCATION")
    {
        return SectionParaKind::StructuralHeading;
    }

    SectionParaKind::Content
}

fn is_body_section_start(p: &Para, expected_chapter: &str) -> bool {
    if !p.has_bold {
        return false;
    }

    let bold = normalize_ws(&p.bold_text);

    if let Some(caps) = SECTION_LINE_RE.captures(&bold) {
        return caps.get(1).unwrap().as_str() == expected_chapter;
    }

    if let Some(caps) = SECTION_ONLY_RE.captures(&bold) {
        return caps.get(1).unwrap().as_str() == expected_chapter;
    }

    false
}

fn is_other_chapter_section_start(p: &Para, expected_chapter: &str) -> bool {
    if !p.has_bold {
        return false;
    }

    let bold = normalize_ws(&p.bold_text);

    if let Some(caps) = SECTION_LINE_RE.captures(&bold) {
        return caps.get(1).unwrap().as_str() != expected_chapter;
    }

    if let Some(caps) = SECTION_ONLY_RE.captures(&bold) {
        return caps.get(1).unwrap().as_str() != expected_chapter;
    }

    false
}

fn is_inline_section_continuation(p: &Para, citation: &str) -> bool {
    if !p.has_bold {
        return false;
    }
    let text = normalize_ws(&p.text);
    text.starts_with(&format!("{citation}. ")) || text.starts_with(&format!("{citation} "))
}

fn is_end_of_chapter(p: &Para, chapter: &str) -> bool {
    let text_upper = p.text.to_uppercase();
    if text_upper.starts_with("ENROLLED HOUSE BILL")
        || text_upper.starts_with("ENROLLED SENATE BILL")
    {
        return true;
    }
    if is_reserved_tail_heading(&p.text) || is_reserved_expansion_text(&p.text) {
        return true;
    }
    if text_upper.starts_with("CHAPTER ") && text_upper.len() < 20 {
        if !text_upper.contains(&format!("CHAPTER {}", chapter)) {
            return true;
        }
    }
    false
}

fn parse_body_section_header(
    p: &Para,
    chapter: &str,
    toc_captions: &HashMap<String, String>,
) -> Result<SectionDraft> {
    let bold = normalize_ws(&p.bold_text);
    let full = clean_parser_text(&p.text);

    let (section_number, caption_from_bold) = if let Some(caps) = SECTION_LINE_RE.captures(&bold) {
        let chap = caps.get(1).unwrap().as_str();
        if chap != chapter {
            return Err(anyhow!("wrong chapter in section start: {}", bold));
        }
        let sec = caps.get(2).unwrap().as_str();
        let caption = strip_trailing_period(caps.get(3).unwrap().as_str());
        (sec.to_string(), Some(caption))
    } else if let Some(caps) = SECTION_ONLY_RE.captures(&bold) {
        let chap = caps.get(1).unwrap().as_str();
        if chap != chapter {
            return Err(anyhow!("wrong chapter in section start: {}", bold));
        }
        let sec = caps.get(2).unwrap().as_str();
        (sec.to_string(), None)
    } else {
        return Err(anyhow!("not a section header: {}", bold));
    };

    let citation = format!("{}.{}", chapter, section_number);
    let caption = caption_from_bold
        .filter(|s| !s.is_empty())
        .or_else(|| toc_captions.get(&citation).cloned());

    let mut paragraphs = Vec::new();
    let after_bold = full.strip_prefix(&bold).unwrap_or("").trim().to_string();

    let status_text = if full.contains('[') && full.contains(']') && caption.is_none() {
        Some(full.clone())
    } else {
        None
    };

    if !after_bold.is_empty() {
        paragraphs.push(SectionPara {
            text: after_bold,
            kind: SectionParaKind::Content,
            order: p.order,
            paragraph_id: p.paragraph_id.clone(),
            heading_path: Vec::new(),
        });
    } else if caption.is_none() && !full.is_empty() {
        paragraphs.push(SectionPara {
            text: full.clone(),
            kind: SectionParaKind::Content,
            order: p.order,
            paragraph_id: p.paragraph_id.clone(),
            heading_path: Vec::new(),
        });
    }

    Ok(SectionDraft {
        citation,
        caption,
        heading: None,
        paragraphs,
        status_text,
        paragraph_start_order: p.order,
        paragraph_end_order: p.order,
    })
}

fn classify_status(full_text: &str, status_text: Option<&str>) -> String {
    if let Some(t) = status_text {
        if STATUS_NOTE_REF_RE.is_match(t) {
            return status_from_text(t).unwrap_or("status_only").to_string();
        }

        if let Some(status) = status_from_text(t) {
            let normalized_full = normalize_ws(full_text);
            let normalized_status = normalize_status_text(t);
            let remainder = normalized_full
                .strip_prefix(&normalized_status)
                .map(str::trim)
                .unwrap_or_else(|| normalized_full.trim());

            if remainder.is_empty()
                || is_parenthetical_heading_text(remainder)
                || is_heading_like_status_remainder(remainder)
                || remainder.starts_with('(')
                || remainder.starts_with("Sec. ")
            {
                return status.to_string();
            }
        }
    }

    let t = status_text.unwrap_or(full_text);

    let stripped = full_text
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();

    let only_bracketed = full_text.trim().starts_with('[') && full_text.trim().ends_with(']');

    // Check for status keywords regardless of length when they appear at the very start
    let stripped_upper = stripped.to_ascii_uppercase();
    if stripped_upper.starts_with("REPEALED") || stripped_upper.starts_with("RENUMBERED") {
        if STATUS_RENUMBERED_RE.is_match(t) {
            return "renumbered".to_string();
        }
        return "repealed".to_string();
    }
    if stripped_upper.starts_with("FORMERLY") {
        return "formerly".to_string();
    }

    if only_bracketed || stripped.len() < 80 {
        if STATUS_RENUMBERED_RE.is_match(t) {
            return "renumbered".to_string();
        }
        if STATUS_REPEALED_RE.is_match(t) {
            return "repealed".to_string();
        }
        if STATUS_FORMERLY_RE.is_match(t) {
            return "formerly".to_string();
        }
        return "status_only".to_string();
    }

    "active".to_string()
}

fn status_from_text(text: &str) -> Option<&'static str> {
    if STATUS_RENUMBERED_RE.is_match(text) {
        return Some("renumbered");
    }
    if STATUS_REPEALED_RE.is_match(text) {
        return Some("repealed");
    }
    if STATUS_FORMERLY_RE.is_match(text) {
        return Some("formerly");
    }
    None
}

fn normalize_status_text(text: &str) -> String {
    let normalized = normalize_ws(text);
    if let Some(start) = normalized.find('[') {
        normalized[start..].to_string()
    } else {
        normalized
    }
}

fn is_parenthetical_heading_text(text: &str) -> bool {
    let normalized = normalize_ws(text);
    let Some(caps) = PARENTHETICAL_HEADING_RE.captures(&normalized) else {
        return false;
    };
    let inner = normalize_ws(caps.get(1).unwrap().as_str());
    !inner.is_empty() && !inner.contains('[') && !inner.contains(']')
}

fn is_heading_like_status_remainder(text: &str) -> bool {
    let normalized = normalize_ws(text);
    if normalized.is_empty() || normalized.len() > 120 || normalized.contains(". ") {
        return false;
    }

    let trimmed = normalized
        .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
        .to_string();
    if trimmed.is_empty() {
        return false;
    }

    let letters = trimmed
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<Vec<_>>();
    letters.len() >= 3 && letters.iter().all(|c| !c.is_lowercase())
}

fn derive_fallback_title(draft: &SectionDraft) -> Option<String> {
    let candidates = draft
        .paragraphs
        .iter()
        .filter(|p| matches!(p.kind, SectionParaKind::Content))
        .filter_map(|p| {
            let normalized = normalize_ws(&p.text);
            let caps = PARENTHETICAL_HEADING_RE.captures(&normalized)?;
            Some(normalize_ws(caps.get(1)?.as_str()))
        })
        .collect::<Vec<_>>();

    if candidates.len() != 1 {
        return derive_inline_caption_title(draft);
    }

    let title = candidates[0].clone();
    if title.is_empty() {
        derive_inline_caption_title(draft)
    } else {
        Some(title)
    }
}

fn derive_inline_caption_title(draft: &SectionDraft) -> Option<String> {
    let prefix = format!("{}.", draft.citation);

    for para in draft
        .paragraphs
        .iter()
        .filter(|p| matches!(p.kind, SectionParaKind::Content))
    {
        let normalized = normalize_ws(&para.text);
        let Some(rest) = normalized.strip_prefix(&prefix) else {
            continue;
        };

        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }

        let sentence = rest
            .split_once(". ")
            .map(|(head, _)| head)
            .unwrap_or(rest)
            .trim();
        let title = strip_trailing_period(sentence);
        if !title.is_empty() {
            return Some(title);
        }
    }

    None
}

fn build_source_notes_for_section(
    draft: &SectionDraft,
    canonical_id: &str,
    version_id: &str,
    source_document_id: &str,
) -> Vec<SourceNote> {
    let mut groups: Vec<Vec<&SectionPara>> = Vec::new();
    for para in draft
        .paragraphs
        .iter()
        .filter(|p| matches!(p.kind, SectionParaKind::Note))
    {
        let starts_note = normalize_ws(&para.text)
            .to_ascii_lowercase()
            .starts_with("note:");
        if starts_note || groups.is_empty() {
            groups.push(Vec::new());
        }
        groups.last_mut().unwrap().push(para);
    }

    groups
        .into_iter()
        .filter_map(|note_paras| {
            let text = note_paras
                .iter()
                .map(|p| normalize_ws(&p.text))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            if text.is_empty() {
                return None;
            }

            let start = note_paras.iter().map(|p| p.order).min().unwrap_or(0);
            let end = note_paras.iter().map(|p| p.order).max().unwrap_or(start);
            Some(SourceNote {
                source_note_id: format!(
                    "source_note:{}",
                    stable_id(&format!("{version_id}::{start}::{end}::{text}"))
                ),
                note_type: classify_source_note_type(&text).to_string(),
                normalized_text: normalize_ws(&text).to_lowercase(),
                text,
                source_document_id: source_document_id.to_string(),
                canonical_id: canonical_id.to_string(),
                version_id: Some(version_id.to_string()),
                provision_id: None,
                citation: format!("ORS {}", draft.citation),
                paragraph_start_order: start,
                paragraph_end_order: end,
                source_paragraph_order: start,
                source_paragraph_ids: note_paras.iter().map(|p| p.paragraph_id.clone()).collect(),
                confidence: 0.8,
                extraction_method: "ors_dom_note_parser_v1".to_string(),
            })
        })
        .collect()
}

fn build_duplicate_section_source_note(
    draft: &SectionDraft,
    canonical_id: &str,
    version_id: &str,
    source_document_id: &str,
    primary_hash: Option<&str>,
) -> SourceNote {
    let text = section_content_text(draft);
    let normalized_text = normalize_ws(&text);
    let duplicate_hash = sha256_hex(normalize_for_hash(&text).as_bytes());
    let note_type = if primary_hash == Some(duplicate_hash.as_str()) {
        "duplicate_section_text"
    } else {
        "duplicate_section_variant"
    };
    let paragraph_ids = draft
        .paragraphs
        .iter()
        .filter(|p| matches!(p.kind, SectionParaKind::Content))
        .map(|p| p.paragraph_id.clone())
        .collect::<Vec<_>>();

    SourceNote {
        source_note_id: format!(
            "source_note:{}",
            stable_id(&format!(
                "{version_id}::duplicate-section::{}::{}::{}",
                draft.paragraph_start_order, draft.paragraph_end_order, duplicate_hash
            ))
        ),
        note_type: note_type.to_string(),
        normalized_text: normalized_text.to_lowercase(),
        text: if normalized_text.is_empty() {
            format!("Duplicate source section header for ORS {}", draft.citation)
        } else {
            normalized_text
        },
        source_document_id: source_document_id.to_string(),
        canonical_id: canonical_id.to_string(),
        version_id: Some(version_id.to_string()),
        provision_id: None,
        citation: format!("ORS {}", draft.citation),
        paragraph_start_order: draft.paragraph_start_order,
        paragraph_end_order: draft.paragraph_end_order,
        source_paragraph_order: draft.paragraph_start_order,
        source_paragraph_ids: paragraph_ids,
        confidence: 0.7,
        extraction_method: "ors_dom_duplicate_section_parser_v1".to_string(),
    }
}

fn classify_source_note_type(text: &str) -> &'static str {
    let lowered = normalize_ws(text).to_lowercase();
    if lowered.contains("not added to or made a part of")
        && (lowered.contains("series") || lowered.contains("any series therein"))
    {
        "not_added_to_series_note"
    } else if lowered.contains("not added to or made a part of") {
        "not_added_to_chapter_note"
    } else if lowered.contains("added to and made a part of") {
        "added_to_chapter_note"
    } else if DATE_NOTE_RE.is_match(text) {
        "operative_note"
    } else if status_from_text(text).is_some() {
        "status_note"
    } else if LEGISLATIVE_HISTORY_RE.is_match(text) || SESSION_LAW_RE.is_match(text) {
        "session_law_note"
    } else if lowered.contains("oregon laws") || lowered.starts_with("note: section ") {
        "session_law_note"
    } else {
        "unknown_note"
    }
}

fn build_legislative_history_source_note(
    history: &str,
    draft: &SectionDraft,
    canonical_id: &str,
    version_id: &str,
    source_document_id: &str,
) -> SourceNote {
    let order = draft.paragraph_end_order;
    let text = normalize_ws(history);
    SourceNote {
        source_note_id: format!(
            "source_note:{}",
            stable_id(&format!("{version_id}::history::{order}::{text}"))
        ),
        note_type: "session_law_note".to_string(),
        normalized_text: text.to_lowercase(),
        text,
        source_document_id: source_document_id.to_string(),
        canonical_id: canonical_id.to_string(),
        version_id: Some(version_id.to_string()),
        provision_id: None,
        citation: format!("ORS {}", draft.citation),
        paragraph_start_order: order,
        paragraph_end_order: order,
        source_paragraph_order: order,
        source_paragraph_ids: vec![format!("p:{order}")],
        confidence: 0.85,
        extraction_method: "ors_dom_legislative_history_parser_v1".to_string(),
    }
}

fn extract_legislative_history(text: &str) -> (String, Vec<String>) {
    let normalized = normalize_ws(text);
    let Some(caps) = LEGISLATIVE_HISTORY_RE.captures(&normalized) else {
        return (normalized, Vec::new());
    };
    let Some(history_match) = caps.get(1) else {
        return (normalized, Vec::new());
    };
    let cleaned = normalized[..history_match.start()].trim().to_string();
    (cleaned, vec![history_match.as_str().to_string()])
}

fn build_amendments_from_history(
    history: &str,
    canonical_id: &str,
    version_id: &str,
    source_document_id: &str,
) -> Vec<Amendment> {
    SESSION_LAW_RE
        .captures_iter(history)
        .filter_map(|caps| {
            let year = caps.get(1)?.as_str();
            let chapter = caps.get(2)?.as_str();
            let section = caps.get(3).map(|m| m.as_str());
            let session_law_citation = match section {
                Some(sec) => format!("{year} c.{chapter} sec. {sec}"),
                None => format!("{year} c.{chapter}"),
            };
            let session_law_id = match section {
                Some(sec) => format!("or:laws:{year}:c:{chapter}:s:{sec}"),
                None => format!("or:laws:{year}:c:{chapter}"),
            };
            Some(Amendment {
                amendment_id: format!(
                    "amendment:{}",
                    stable_id(&format!("{version_id}::{session_law_citation}::{history}"))
                ),
                amendment_type: "legislative_history".to_string(),
                session_law_citation: Some(session_law_citation),
                effective_date: None,
                text: history.to_string(),
                raw_text: Some(history.to_string()),
                source_document_id: Some(source_document_id.to_string()),
                confidence: 0.85,
                canonical_id: Some(canonical_id.to_string()),
                version_id: Some(version_id.to_string()),
                session_law_id: Some(session_law_id),
                affected_canonical_id: Some(canonical_id.to_string()),
                affected_version_id: Some(version_id.to_string()),
                source_note_id: None,
                ..Default::default()
            })
        })
        .collect()
}

fn is_roman_numeral(label: &str) -> bool {
    !label.is_empty()
        && label.chars().all(|c| c.is_ascii_lowercase())
        && ROMAN_NUMERAL_RE.is_match(label)
}

fn marker_level(label: &str) -> usize {
    if label.chars().all(|c| c.is_ascii_digit()) {
        1
    } else if label.len() == 1 && label.chars().all(|c| c.is_ascii_uppercase()) {
        3
    } else if is_roman_numeral(label) {
        4
    } else if label.len() == 1 && label.chars().all(|c| c.is_ascii_lowercase()) {
        2
    } else {
        4
    }
}

fn build_provisions_for_section(
    draft: &SectionDraft,
    canonical_id: &str,
    version_id: &str,
    edition_year: i32,
    _section_order: usize,
) -> Vec<Provision> {
    let mut provisions = Vec::new();
    let mut current_path: Vec<String> = Vec::new();
    let mut active_path: Option<Vec<String>> = None;
    let mut current_text = String::new();
    let mut current_original_text = String::new();
    let mut current_para_start: Option<usize> = None;
    let mut current_para_end: Option<usize> = None;
    let mut current_para_ids: Vec<String> = Vec::new();
    let mut current_heading_path: Vec<String> =
        draft.heading.clone().map(|h| vec![h]).unwrap_or_default();
    let mut order_index = 0usize;
    let mut seen_paths: HashMap<Vec<String>, usize> = HashMap::new();

    let mut flush = |provisions: &mut Vec<Provision>,
                     active_path: &mut Option<Vec<String>>,
                     current_text: &mut String,
                     current_original_text: &mut String,
                     current_para_start: &mut Option<usize>,
                     current_para_end: &mut Option<usize>,
                     current_para_ids: &mut Vec<String>,
                     current_heading_path: &[String],
                     order_index: &mut usize| {
        let original_text = normalize_ws(current_original_text);
        let (text, _) = extract_legislative_history(&normalize_ws(current_text));
        if text.is_empty() {
            return;
        }

        let mut path = active_path
            .clone()
            .unwrap_or_else(|| vec!["root".to_string()]);

        let count = seen_paths.entry(path.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            let last_idx = path.len() - 1;
            path[last_idx] = format!("{}_v{}", path[last_idx], count);
        }

        let display = display_citation(&draft.citation, &path);
        let provision_id = format!(
            "{}@{}::p:{}",
            canonical_id,
            edition_year,
            path.join(".").replace(' ', "_")
        );

        let normalized = normalize_ws(&text);

        provisions.push(Provision {
            provision_id,
            version_id: version_id.to_string(),
            canonical_id: canonical_id.to_string(),
            citation: format!("ORS {}", draft.citation),
            display_citation: display,
            local_path: path.clone(),
            provision_type: provision_type_for_path(&path).to_string(),
            text: text.clone(),
            original_text: if !original_text.is_empty() && original_text != text {
                Some(original_text)
            } else {
                None
            },
            normalized_text: normalized.clone(),
            order_index: *order_index,
            depth: path.len(),
            text_hash: sha256_hex(normalize_for_hash(&normalized).as_bytes()),
            is_implied: false,
            is_definition_candidate: is_definition_candidate(&text),
            is_exception_candidate: is_exception_candidate(&text),
            is_deadline_candidate: is_deadline_candidate(&text),
            is_penalty_candidate: is_penalty_candidate(&text),
            paragraph_start_order: *current_para_start,
            paragraph_end_order: *current_para_end,
            source_paragraph_ids: current_para_ids.clone(),
            heading_path: current_heading_path.to_vec(),
            structural_context: if current_heading_path.is_empty() {
                None
            } else {
                Some(current_heading_path.join(" > "))
            },
            ..Default::default()
        });

        *order_index += 1;
        current_text.clear();
        current_original_text.clear();
        *current_para_start = None;
        *current_para_end = None;
        current_para_ids.clear();
    };

    for para in &draft.paragraphs {
        let text = normalize_ws(&para.text);
        if text.is_empty() {
            continue;
        }

        match para.kind {
            SectionParaKind::Separator | SectionParaKind::Note | SectionParaKind::ReservedTail => {
                continue;
            }
            SectionParaKind::StructuralHeading => {
                current_heading_path = para.heading_path.clone();
                continue;
            }
            SectionParaKind::Content => {}
        }

        if let Some((chain, rest)) = parse_leading_marker_chain(&text) {
            flush(
                &mut provisions,
                &mut active_path,
                &mut current_text,
                &mut current_original_text,
                &mut current_para_start,
                &mut current_para_end,
                &mut current_para_ids,
                &current_heading_path,
                &mut order_index,
            );

            // Apply the chain to the cumulative current_path
            for marker in &chain {
                let level = marker_level(marker);
                if level == 0 {
                    continue;
                }
                while current_path.len() >= level {
                    current_path.pop();
                }
                // If we are jumping from level 1 to level 3, we should pad, but the old parser didn't.
                // We just push the marker to current_path.
                current_path.push(marker.clone());
            }

            active_path = Some(current_path.clone());
            current_text.push_str(&normalize_ws(&rest));
            current_original_text.push_str(&text);
        } else {
            if !current_text.is_empty() {
                current_text.push(' ');
                current_original_text.push(' ');
            }
            current_text.push_str(&text);
            current_original_text.push_str(&text);
        }
        current_para_start =
            Some(current_para_start.map_or(para.order, |start| start.min(para.order)));
        current_para_end = Some(current_para_end.map_or(para.order, |end| end.max(para.order)));
        if !current_para_ids.contains(&para.paragraph_id) {
            current_para_ids.push(para.paragraph_id.clone());
        }
    }

    flush(
        &mut provisions,
        &mut active_path,
        &mut current_text,
        &mut current_original_text,
        &mut current_para_start,
        &mut current_para_end,
        &mut current_para_ids,
        &current_heading_path,
        &mut order_index,
    );

    if provisions.is_empty() {
        let text = joined_root_text(draft);
        if !text.is_empty() {
            provisions.push(make_root_provision(
                draft,
                canonical_id,
                version_id,
                edition_year,
                &text,
            ));
        }
    }

    provisions
}

fn joined_root_text(draft: &SectionDraft) -> String {
    draft
        .paragraphs
        .iter()
        .filter(|p| matches!(p.kind, SectionParaKind::Content))
        .map(|p| normalize_ws(&p.text))
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn make_root_provision(
    draft: &SectionDraft,
    canonical_id: &str,
    version_id: &str,
    edition_year: i32,
    text: &str,
) -> Provision {
    let path = vec!["root".to_string()];
    let normalized = normalize_ws(text);
    let content_paras = draft
        .paragraphs
        .iter()
        .filter(|p| matches!(p.kind, SectionParaKind::Content))
        .collect::<Vec<_>>();
    let heading_path = draft.heading.clone().map(|h| vec![h]).unwrap_or_default();
    Provision {
        provision_id: format!("{}@{}::p:root", canonical_id, edition_year),
        version_id: version_id.to_string(),
        canonical_id: canonical_id.to_string(),
        citation: format!("ORS {}", draft.citation),
        display_citation: format!("ORS {}", draft.citation),
        local_path: path,
        provision_type: "section_text".to_string(),
        text: normalized.clone(),
        original_text: None,
        normalized_text: normalized.clone(),
        order_index: 0,
        depth: 1,
        text_hash: sha256_hex(normalize_for_hash(&normalized).as_bytes()),
        is_implied: false,
        is_definition_candidate: is_definition_candidate(&normalized),
        is_exception_candidate: is_exception_candidate(&normalized),
        is_deadline_candidate: is_deadline_candidate(&normalized),
        is_penalty_candidate: is_penalty_candidate(&normalized),
        paragraph_start_order: content_paras.iter().map(|p| p.order).min(),
        paragraph_end_order: content_paras.iter().map(|p| p.order).max(),
        source_paragraph_ids: content_paras
            .iter()
            .map(|p| p.paragraph_id.clone())
            .collect(),
        heading_path: heading_path.clone(),
        structural_context: if heading_path.is_empty() {
            None
        } else {
            Some(heading_path.join(" > "))
        },
        ..Default::default()
    }
}

fn parse_leading_marker_chain(s: &str) -> Option<(Vec<String>, String)> {
    let caps = LEADING_MARKER_CHAIN_RE.captures(s)?;
    let marker_chain = caps.get(1)?.as_str();
    let rest = caps
        .get(2)
        .map(|m| m.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    let path = MARKER_RE
        .captures_iter(marker_chain)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>();

    if path.is_empty() {
        None
    } else {
        Some((path, rest))
    }
}

fn display_citation(section: &str, path: &[String]) -> String {
    if path.is_empty() || path == ["root"] {
        return format!("ORS {}", section);
    }

    let suffix = path
        .iter()
        .map(|p| format!("({})", p))
        .collect::<Vec<_>>()
        .join("");

    format!("ORS {}{}", section, suffix)
}

fn provision_type_for_path(path: &[String]) -> &'static str {
    if path == ["root"] {
        return "section_text";
    }

    match path.len() {
        1 => "subsection",
        2 => "paragraph",
        3 => "subparagraph",
        _ => "clause",
    }
}

fn is_definition_candidate(text: &str) -> bool {
    static DEF_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"(?i)(^|[.;]\s+)(as used in (this (section|subsection|paragraph)|ORS [0-9A-Za-z.\s]+)|[^.]{0,80}\bmeans\b|[^.]{0,80}\bincludes\b)"#,
        )
        .unwrap()
    });

    let text = normalize_ws(text);
    if text.is_empty() {
        return false;
    }

    if text.starts_with("As used in ") {
        return true;
    }

    DEF_RE.is_match(&text)
}

fn is_exception_candidate(text: &str) -> bool {
    static EXC_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(except as provided|except that|unless|notwithstanding|does not apply|is exempt from)\b",
        )
        .unwrap()
    });
    static SUBJECT_TO_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(^|[.;]\s+)subject to\s+(ORS|this |subsection|paragraph|section)").unwrap()
    });

    let text = normalize_ws(text);
    EXC_RE.is_match(&text) || SUBJECT_TO_RE.is_match(&text)
}

fn is_deadline_candidate(text: &str) -> bool {
    static DEADLINE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(no later than|within \d+|not later than|before [A-Z][a-z]+ \d+|after \d+ days|for \d+ days|on or before)\b").unwrap()
    });

    DEADLINE_RE.is_match(text)
}

fn is_penalty_candidate(text: &str) -> bool {
    static PENALTY_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(penalty|punishable|fine|civil penalty|criminal penalty|misdemeanor|felony|contempt|class [a-z] violation)\b",
        )
        .unwrap()
    });

    let text = normalize_ws(text);
    if text.contains("civil liability") && !text.contains("civil penalty") {
        return false;
    }

    PENALTY_RE.is_match(&text)
}

#[cfg(test)]
mod tests {
    use super::parse_ors_chapter_html;
    use std::fs;

    fn load_html(name: &str) -> String {
        let path = format!("/Users/grey/ORSGraph/data/raw/official/{name}");
        let bytes = fs::read(path).expect("fixture html");
        let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
        cow.to_string()
    }

    #[test]
    fn uses_requested_edition_year_in_ids_and_chunks() {
        let html = load_html("ors002.html");
        let parsed = parse_ors_chapter_html(
            &html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors002.html",
            "2",
            2024,
        )
        .expect("parse chapter 2");

        let root = parsed
            .provisions
            .iter()
            .find(|p| p.canonical_id == "or:ors:2.010")
            .expect("root provision");
        assert_eq!(root.provision_id, "or:ors:2.010@2024::p:root");

        let chunk = parsed
            .chunks
            .iter()
            .find(|c| {
                c.source_provision_id.as_deref() == Some(root.provision_id.as_str())
                    && c.chunk_type == "contextual_provision"
            })
            .expect("contextual chunk");
        assert!(
            chunk
                .text
                .starts_with("Oregon Revised Statutes. 2024 Edition.")
        );
    }

    #[test]
    fn strips_tail_artifacts_from_note_heavy_sections() {
        let html = load_html("ors830.html");
        let parsed = parse_ors_chapter_html(
            &html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors830.html",
            "830",
            2025,
        )
        .expect("parse chapter 830");

        let compact = parsed
            .versions
            .iter()
            .find(|v| v.canonical_id == "or:ors:830.080")
            .expect("830.080");
        assert!(!compact.text.contains("Note:"));
        assert!(!compact.text.contains("MANDATORY BOATING SAFETY EDUCATION"));
        assert!(!compact.text.contains("_______________"));

        let final_section = parsed
            .versions
            .iter()
            .find(|v| v.canonical_id == "or:ors:830.999")
            .expect("830.999");
        assert!(!final_section.text.contains("CHAPTERS 831 TO 834"));
        assert!(!final_section.text.contains("[Reserved for expansion]"));
    }

    #[test]
    fn strips_reserved_expansion_tail_from_last_section() {
        let html = load_html("ors838.html");
        let parsed = parse_ors_chapter_html(
            &html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors838.html",
            "838",
            2025,
        )
        .expect("parse chapter 838");

        let section = parsed
            .versions
            .iter()
            .find(|v| v.canonical_id == "or:ors:838.075")
            .expect("838.075");
        assert!(!section.text.contains("TITLES 63 et seq."));
        assert!(!section.text.contains("CHAPTERS 839 et seq."));
        assert!(!section.text.contains("[Reserved for expansion]"));
    }

    #[test]
    fn keeps_status_only_sections_with_parenthetical_titles_out_of_active_status() {
        let html = load_html("ors025.html");
        let parsed = parse_ors_chapter_html(
            &html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors025.html",
            "25",
            2025,
        )
        .expect("parse chapter 25");

        let section = parsed
            .versions
            .iter()
            .find(|v| v.canonical_id == "or:ors:25.500")
            .expect("25.500");
        assert_eq!(section.status, "repealed");
        assert_eq!(
            section.title.as_deref(),
            Some("Child Support Determination and Compliance")
        );
    }

    #[test]
    fn preserves_repeated_level_one_paths_after_structural_heading() {
        let html = r#"
            <html><body>
              <p class="MsoNormal"><b>1.001 Test section.</b></p>
              <p class="MsoNormal">(1) First heading group text.</p>
              <p class="MsoNormal"><u>SECOND GROUP</u></p>
              <p class="MsoNormal">(1) Second heading group text.</p>
              <p class="MsoNormal">(2) Third text.</p>
            </body></html>
        "#;
        let parsed = parse_ors_chapter_html(
            html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors001.html",
            "1",
            2025,
        )
        .expect("parse synthetic chapter");

        let paths = parsed
            .provisions
            .iter()
            .map(|p| p.local_path.clone())
            .collect::<Vec<_>>();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], vec!["1".to_string()]);
        assert_eq!(paths[1], vec!["1_v2".to_string()]);
        assert_eq!(paths[2], vec!["2".to_string()]);
        assert!(!parsed.provisions.iter().any(|p| p.local_path == ["root"]));
    }

    #[test]
    fn keeps_note_following_sections_out_of_active_status() {
        let html = load_html("ors188.html");
        let parsed = parse_ors_chapter_html(
            &html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors188.html",
            "188",
            2025,
        )
        .expect("parse chapter 188");

        let section = parsed
            .versions
            .iter()
            .find(|v| v.canonical_id == "or:ors:188.135")
            .expect("188.135");
        assert_eq!(section.status, "status_only");
    }

    #[test]
    fn keeps_repealed_sections_with_session_law_notes_out_of_active_status() {
        let html = load_html("ors199.html");
        let parsed = parse_ors_chapter_html(
            &html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors199.html",
            "199",
            2025,
        )
        .expect("parse chapter 199");

        let section = parsed
            .versions
            .iter()
            .find(|v| v.canonical_id == "or:ors:199.310")
            .expect("199.310");
        assert_eq!(section.status, "repealed");
    }

    #[test]
    fn keeps_repealed_sections_with_heading_only_remainders_out_of_active_status() {
        let html = load_html("ors441.html");
        let parsed = parse_ors_chapter_html(
            &html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors441.html",
            "441",
            2025,
        )
        .expect("parse chapter 441");

        let section = parsed
            .versions
            .iter()
            .find(|v| v.canonical_id == "or:ors:441.110")
            .expect("441.110");
        assert_eq!(section.status, "repealed");
    }

    #[test]
    fn derives_titles_from_inline_repeated_section_lines() {
        let html = load_html("ors679.html");
        let parsed = parse_ors_chapter_html(
            &html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors679.html",
            "679",
            2025,
        )
        .expect("parse chapter 679");

        let section = parsed
            .versions
            .iter()
            .find(|v| v.canonical_id == "or:ors:679.530")
            .expect("679.530");
        assert_eq!(
            section.title.as_deref(),
            Some("Information about oral prosthetic devices")
        );
    }

    #[test]
    fn captures_notes_diagnostics_provenance_and_history() {
        let html = r#"
            <html><body>
              <p class="MsoNormal">2025 EDITION</p>
              <p class="MsoNormal">GENERAL PROVISIONS</p>
              <p class="MsoNormal">1.001 Table of contents row</p>
              <p class="MsoNormal"><b>1.001 Test section.</b> Statutory text. [2013 c.685 &#167;46; 2015 c.10 &#167;2]</p>
              <p class="MsoNormal"><b>Note:</b> 1.001 was enacted into law by the Legislative Assembly but was not added to or made a part of ORS chapter 1.</p>
              <p class="MsoNormal">This is continuation note text.</p>
            </body></html>
        "#;
        let parsed = parse_ors_chapter_html(
            html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors001.html",
            "1",
            2025,
        )
        .expect("parse synthetic chapter");

        assert_eq!(parsed.versions.len(), 1);
        let version = &parsed.versions[0];
        assert_eq!(version.text, "Statutory text.");
        assert!(
            version
                .original_text
                .as_deref()
                .is_some_and(|text| text.contains("2013 c.685"))
        );
        assert_eq!(version.paragraph_start_order, Some(3));
        assert_eq!(version.paragraph_end_order, Some(5));

        assert_eq!(parsed.provisions.len(), 1);
        assert_eq!(parsed.provisions[0].text, "Statutory text.");
        assert_eq!(parsed.provisions[0].paragraph_start_order, Some(3));

        assert!(parsed.source_notes.len() >= 2);
        let official_note = parsed
            .source_notes
            .iter()
            .find(|note| note.note_type == "not_added_to_chapter_note")
            .expect("official note");
        assert!(official_note.text.contains("continuation note text"));

        assert_eq!(parsed.amendments.len(), 2);
        assert!(
            parsed
                .amendments
                .iter()
                .any(|a| a.session_law_citation.as_deref() == Some("2013 c.685 sec. 46"))
        );

        assert_eq!(parsed.parser_diagnostics.total_mso_normal, 6);
        assert_eq!(parsed.parser_diagnostics.section_starts_detected, 1);
        assert_eq!(parsed.parser_diagnostics.skipped_note_paragraphs, 2);
        assert!(
            parsed
                .parser_diagnostics
                .paragraphs_ignored_before_body_start
                >= 3
        );
    }

    #[test]
    fn emits_toc_reserved_temporal_lineage_and_session_law_surfaces() {
        let html = r#"
            <html><body>
              <p class="MsoNormal">2025 EDITION</p>
              <p class="MsoNormal">GENERAL PROVISIONS</p>
              <p class="MsoNormal">1.001 Test section</p>
              <p class="MsoNormal"><b>1.001 Test section.</b> Text. [2013 c.685 &#167;46; Formerly 2.001; renumbered 1.002 in 2020]</p>
              <p class="MsoNormal"><b>Note:</b> Section becomes operative July 1, 2026 and is repealed January 2, 2027. Oregon Laws 2025, chapter 88, section 3 applies.</p>
              <p class="MsoNormal">CHAPTERS 2 TO 4</p>
              <p class="MsoNormal">[Reserved for expansion]</p>
            </body></html>
        "#;
        let parsed = parse_ors_chapter_html(
            html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors001.html",
            "1",
            2025,
        )
        .expect("parse synthetic chapter");

        assert_eq!(parsed.chapter_toc_entries.len(), 1);
        assert_eq!(
            parsed.chapter_toc_entries[0].canonical_id.as_deref(),
            Some("or:ors:1.001")
        );
        assert!(!parsed.reserved_ranges.is_empty());
        assert!(!parsed.parser_diagnostic_rows.is_empty());

        let note_semantics = crate::semantic::derive_note_semantics(
            &parsed.source_notes,
            &parsed.source_document,
            parsed.edition_year,
        );
        assert!(
            note_semantics
                .temporal_effects
                .iter()
                .any(|e| e.effect_type == "operative")
        );
        assert!(
            note_semantics
                .temporal_effects
                .iter()
                .any(|e| e.effect_type == "repeal")
        );
        assert!(
            note_semantics
                .lineage_events
                .iter()
                .any(|e| e.lineage_type == "formerly")
        );
        assert!(
            note_semantics
                .lineage_events
                .iter()
                .any(|e| e.lineage_type == "renumbered_to")
        );
        assert!(
            note_semantics
                .session_laws
                .iter()
                .any(|law| law.session_law_id == "or:laws:2025:c:88:s:3")
        );
        assert!(
            parsed
                .amendments
                .iter()
                .any(|a| a.session_law_id.as_deref() == Some("or:laws:2013:c:685:s:46"))
        );
    }

    #[test]
    fn semantic_extractor_adds_penalty_details_and_definition_scope() {
        let html = r#"
            <html><body>
              <p class="MsoNormal"><b>1.001 Definitions.</b></p>
              <p class="MsoNormal">(1) As used in ORS 1.001 to 1.009, "agency" means a public body.</p>
              <p class="MsoNormal">(2) Violation of ORS 1.001 is a Class A misdemeanor and is punishable by a civil penalty not to exceed $10,000. A license may be suspended.</p>
            </body></html>
        "#;
        let parsed = parse_ors_chapter_html(
            html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors001.html",
            "1",
            2025,
        )
        .expect("parse synthetic chapter");
        let semantic = crate::semantic::derive_semantic_nodes(&parsed.provisions);

        assert!(
            semantic
                .definition_scopes
                .iter()
                .any(|scope| scope.scope_type == "range"
                    && scope.target_range_start.as_deref() == Some("or:ors:1.001"))
        );
        let penalty = semantic.penalties.first().expect("penalty");
        assert_eq!(
            penalty.criminal_class.as_deref(),
            Some("Class A misdemeanor")
        );
        assert_eq!(penalty.max_amount.as_deref(), Some("$10,000"));
        assert_eq!(penalty.license_suspension, Some(true));
    }

    #[test]
    fn keeps_toc_section_rows_before_heading_out_of_body() {
        let html = r#"
            <html><body>
              <p class="MsoNormal">2025 EDITION</p>
              <p class="MsoNormal"><b>1.001 TOC row that should not start body.</b></p>
              <p class="MsoNormal">GENERAL PROVISIONS</p>
              <p class="MsoNormal"><b>1.001 Actual section.</b> Body text.</p>
            </body></html>
        "#;
        let parsed = parse_ors_chapter_html(
            html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors001.html",
            "1",
            2025,
        )
        .expect("parse synthetic chapter");

        assert_eq!(parsed.versions.len(), 1);
        assert_eq!(parsed.versions[0].title.as_deref(), Some("Actual section"));
        assert_eq!(parsed.versions[0].text, "Body text.");
    }

    #[test]
    fn attaches_structural_heading_context_to_provisions_and_chunks() {
        let html = r#"
            <html><body>
              <p class="MsoNormal"><b>1.001 Test section.</b></p>
              <p class="MsoNormal"><u>ARTICLE I</u></p>
              <p class="MsoNormal">(1) Article text.</p>
            </body></html>
        "#;
        let parsed = parse_ors_chapter_html(
            html,
            "https://www.oregonlegislature.gov/bills_laws/ors/ors001.html",
            "1",
            2025,
        )
        .expect("parse synthetic chapter");

        let provision = parsed.provisions.first().expect("provision");
        assert_eq!(provision.heading_path, vec!["ARTICLE I".to_string()]);
        assert_eq!(provision.structural_context.as_deref(), Some("ARTICLE I"));

        let chunk = parsed
            .chunks
            .iter()
            .find(|c| c.source_provision_id.as_deref() == Some(provision.provision_id.as_str()))
            .expect("chunk");
        assert_eq!(chunk.heading_path, vec!["ARTICLE I".to_string()]);
        assert_eq!(chunk.structural_context.as_deref(), Some("ARTICLE I"));
    }
}
