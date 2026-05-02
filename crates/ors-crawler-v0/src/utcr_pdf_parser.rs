use crate::hash::{sha256_hex, stable_id};
use crate::models::{
    ChapterHeading, CitationMention, CitesEdge, Commentary, CorpusEdition, CourtRuleChapter,
    ExternalLegalCitation, FormattingProfile, LegalCorpus, LegalTextIdentity, LegalTextVersion,
    ParserDiagnostic, ProceduralRequirement, Provision, ReporterNote, RetrievalChunk,
    RulePackMembership, SourceDocument, SourcePage, SourceTocEntry, WorkProductRulePack,
};
use crate::text::normalize_ws;
use anyhow::{Context, Result};
use lopdf::Document;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

const CORPUS_ID: &str = "or:utcr";
const EDITION_ID: &str = "or:utcr@2025";
const SOURCE_DOCUMENT_ID: &str = "or:utcr:source:2025_pdf";
const AUTHORITY_FAMILY: &str = "UTCR";
const AUTHORITY_TYPE: &str = "court_rule";
const JURISDICTION_ID: &str = "or:state";
const AUTHORITY_LEVEL: i32 = 80;
const PARSER_PROFILE: &str = "utcr_pdf_parser_v1";
const SOURCE_URL: &str = "https://www.courts.oregon.gov/rules/UTCR/2025_UTCR.pdf";
const EFFECTIVE_DATE: &str = "2025-08-01";

static CHAPTER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^CHAPTER\s+([0-9]+)\s*[-—]\s*(.+)$").expect("chapter regex"));
static CHAPTER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^CHAPTER\s+([0-9]+)$").expect("chapter number regex"));
static RULE_HEADING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([0-9]{1,2}\.[0-9]{3})\s+(.+)$").expect("rule heading regex"));
static RULE_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([0-9]{1,2}\.[0-9]{3})$").expect("rule number regex"));
static SPLIT_RULE_PREFIX_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([0-9]{1,2})\.$").expect("split rule prefix regex"));
static SPLIT_RULE_SUFFIX_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]{3}$").expect("split rule suffix regex"));
static PAGE_FOOTER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bUTCR\s+8/1/2025\s+[0-9]+\.[0-9]+\b").expect("footer regex"));
static PAGE_LABEL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]{1,2}\.[0-9]{1,2}$").expect("page label regex"));
static PDF_CONTROL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\u{0000}-\u{0008}\u{000B}\u{000C}\u{000E}-\u{001F}]").unwrap());
static LEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\.{4,}").unwrap());
static RULE_PIN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\(([0-9A-Za-z]+)\)").expect("pin regex"));
static PROVISION_MARKER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\(\s*([0-9](?:\s*[0-9])*|[A-Za-z]|[IVXLCDMivxlcdm]+)\s*\)\s*(.*)$")
        .expect("provision marker regex")
});
static COMMENTARY_START_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^[0-9]{4}\s+Commentary\b").expect("commentary regex"));
static UTCR_CITATION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bUTCR\s+([0-9]{1,2}\.[0-9]{3})((?:\([0-9A-Za-z]+\))*)")
        .expect("utcr citation regex")
});
static ORS_CITATION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\bORS\s+(?:chapters?\s+)?[0-9]{1,3}[A-Z]?(?:\.[0-9]{3,4})?(?:\s*(?:to|through)\s*[0-9]{1,3}[A-Z]?(?:\.[0-9]{3,4})?)?(?:\([^)]+\))*",
    )
    .expect("ors citation regex")
});
static ORCP_CITATION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bORCP\s+[0-9A-Z]+(?:\s+[A-Z0-9]+)*(?:\([^)]+\))*").unwrap());
static SLR_CITATION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bSLR\s+[0-9]{1,2}\.[0-9]{3}(?:\([^)]+\))*").unwrap());
static ORAP_CITATION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bORAP\s+[0-9]{1,2}\.[0-9]{2,3}(?:\([^)]+\))*").unwrap());
static ORDER_CITATION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:Chief Justice Order|Supreme Court Order|CJO|SCO)\s+(?:No\.\s*)?[0-9]{2,4}-[0-9]{2,4}\b")
        .unwrap()
});
static URL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bhttps?://[^\s)]+|\bwww\.courts\.oregon\.gov/[^\s)]+").unwrap());

#[derive(Debug, Clone)]
struct PageLine {
    page_number: usize,
    text: String,
}

#[derive(Debug, Clone)]
struct RuleDraft {
    citation: String,
    title: String,
    chapter: String,
    chapter_title: String,
    start_page: usize,
    end_page: usize,
    lines: Vec<PageLine>,
}

#[derive(Debug, Default)]
pub struct ParsedUtcrCorpus {
    pub legal_corpora: Vec<LegalCorpus>,
    pub corpus_editions: Vec<CorpusEdition>,
    pub source_documents: Vec<SourceDocument>,
    pub source_pages: Vec<SourcePage>,
    pub source_toc_entries: Vec<SourceTocEntry>,
    pub court_rule_chapters: Vec<CourtRuleChapter>,
    pub chapter_headings: Vec<ChapterHeading>,
    pub identities: Vec<LegalTextIdentity>,
    pub versions: Vec<LegalTextVersion>,
    pub provisions: Vec<Provision>,
    pub reporter_notes: Vec<ReporterNote>,
    pub commentaries: Vec<Commentary>,
    pub citation_mentions: Vec<CitationMention>,
    pub external_legal_citations: Vec<ExternalLegalCitation>,
    pub cites_edges: Vec<CitesEdge>,
    pub procedural_rules: Vec<ProceduralRequirement>,
    pub formatting_requirements: Vec<ProceduralRequirement>,
    pub filing_requirements: Vec<ProceduralRequirement>,
    pub service_requirements: Vec<ProceduralRequirement>,
    pub efiling_requirements: Vec<ProceduralRequirement>,
    pub caption_requirements: Vec<ProceduralRequirement>,
    pub signature_requirements: Vec<ProceduralRequirement>,
    pub certificate_requirements: Vec<ProceduralRequirement>,
    pub exhibit_requirements: Vec<ProceduralRequirement>,
    pub protected_information_rules: Vec<ProceduralRequirement>,
    pub sanction_rules: Vec<ProceduralRequirement>,
    pub deadline_rules: Vec<ProceduralRequirement>,
    pub exception_rules: Vec<ProceduralRequirement>,
    pub work_product_rule_packs: Vec<WorkProductRulePack>,
    pub formatting_profiles: Vec<FormattingProfile>,
    pub rule_pack_memberships: Vec<RulePackMembership>,
    pub retrieval_chunks: Vec<RetrievalChunk>,
    pub parser_diagnostics: Vec<ParserDiagnostic>,
}

#[derive(Debug, Clone)]
pub struct UtcrParseConfig {
    pub edition_year: i32,
    pub effective_date: String,
    pub source_url: String,
}

impl Default for UtcrParseConfig {
    fn default() -> Self {
        Self {
            edition_year: 2025,
            effective_date: EFFECTIVE_DATE.to_string(),
            source_url: SOURCE_URL.to_string(),
        }
    }
}

pub fn parse_utcr_pdf(path: &Path, config: UtcrParseConfig) -> Result<ParsedUtcrCorpus> {
    let raw_bytes = std::fs::read(path)
        .with_context(|| format!("failed to read UTCR PDF {}", path.display()))?;
    let doc = Document::load(path)
        .with_context(|| format!("failed to load UTCR PDF {}", path.display()))?;
    let mut page_numbers = doc.get_pages().keys().copied().collect::<Vec<_>>();
    page_numbers.sort_unstable();
    let mut pages = Vec::new();
    for page_number in page_numbers {
        let raw_text = doc
            .extract_text(&[page_number])
            .with_context(|| format!("failed to extract PDF text from page {page_number}"))?;
        let normalized_text = normalize_page_text(&raw_text);
        pages.push(SourcePage {
            source_page_id: format!("{SOURCE_DOCUMENT_ID}:page:{page_number}"),
            source_document_id: SOURCE_DOCUMENT_ID.to_string(),
            page_number: page_number as usize,
            printed_label: extract_printed_label(&raw_text),
            text: raw_text,
            normalized_text: normalized_text.clone(),
            text_hash: sha256_hex(normalized_text.as_bytes()),
        });
    }

    let normalized_full_text = pages
        .iter()
        .map(|page| page.normalized_text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let source_document = SourceDocument {
        source_document_id: SOURCE_DOCUMENT_ID.to_string(),
        source_provider: "oregon_judicial_department".to_string(),
        source_kind: "official_pdf".to_string(),
        url: config.source_url.clone(),
        chapter: "UTCR".to_string(),
        corpus_id: Some(CORPUS_ID.to_string()),
        edition_id: Some(EDITION_ID.to_string()),
        authority_family: Some(AUTHORITY_FAMILY.to_string()),
        authority_type: Some(AUTHORITY_TYPE.to_string()),
        title: Some("2025 Uniform Trial Court Rules".to_string()),
        source_type: Some("official_pdf".to_string()),
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string()),
        page_count: Some(pages.len()),
        effective_date: Some(config.effective_date.clone()),
        copyright_status: Some("no_copyright_reproducible".to_string()),
        chapter_title: Some("Oregon Uniform Trial Court Rules".to_string()),
        edition_year: config.edition_year,
        html_encoding: None,
        source_path: Some(path.display().to_string()),
        paragraph_count: Some(
            normalized_full_text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count(),
        ),
        first_body_paragraph_index: None,
        parser_profile: Some(PARSER_PROFILE.to_string()),
        official_status: "official_pdf".to_string(),
        disclaimer_required: false,
        raw_hash: sha256_hex(&raw_bytes),
        normalized_hash: sha256_hex(normalized_full_text.as_bytes()),
    };

    let page_lines = page_lines(&pages);
    let body_start = body_start_index(&page_lines).unwrap_or(0);
    let toc_entries = parse_toc_entries(&page_lines[..body_start.min(page_lines.len())], &pages);
    let body_lines = logical_body_lines(&page_lines[body_start..]);
    let (chapters, rule_drafts, mut diagnostics) = parse_body_rules(&body_lines, &config);
    let chapter_headings = chapter_headings_from_chapters(&chapters);

    let mut parsed = ParsedUtcrCorpus {
        legal_corpora: vec![LegalCorpus {
            corpus_id: CORPUS_ID.to_string(),
            name: "Oregon Uniform Trial Court Rules".to_string(),
            short_name: AUTHORITY_FAMILY.to_string(),
            authority_family: AUTHORITY_FAMILY.to_string(),
            authority_type: AUTHORITY_TYPE.to_string(),
            jurisdiction_id: JURISDICTION_ID.to_string(),
        }],
        corpus_editions: vec![CorpusEdition {
            edition_id: EDITION_ID.to_string(),
            corpus_id: CORPUS_ID.to_string(),
            edition_year: config.edition_year,
            effective_date: Some(config.effective_date.clone()),
            source_label: Some("2025 Uniform Trial Court Rules".to_string()),
            current: Some(true),
        }],
        source_documents: vec![source_document],
        source_pages: pages,
        source_toc_entries: toc_entries,
        court_rule_chapters: chapters,
        chapter_headings,
        ..Default::default()
    };

    let mut provision_lookup = HashMap::<(String, Vec<String>), String>::new();
    for draft in &rule_drafts {
        push_rule(&mut parsed, draft, &config, &mut provision_lookup);
    }

    extract_citations(&mut parsed, &provision_lookup);
    extract_requirements(&mut parsed, &config);
    build_rule_packs(&mut parsed, &config);
    build_retrieval_chunks(&mut parsed, &config);
    diagnostics.extend(validate_utcr_parse(&parsed, &config));
    parsed.parser_diagnostics = diagnostics;
    Ok(parsed)
}

fn page_lines(pages: &[SourcePage]) -> Vec<PageLine> {
    pages
        .iter()
        .flat_map(|page| {
            page.normalized_text.lines().filter_map(move |line| {
                let text = normalize_line(line);
                (!text.is_empty()).then_some(PageLine {
                    page_number: page.page_number,
                    text,
                })
            })
        })
        .collect()
}

fn normalize_page_text(text: &str) -> String {
    let mut out = text
        .replace('\u{00a0}', " ")
        .replace('“', "\"")
        .replace('”', "\"")
        .replace('‘', "'")
        .replace('’', "'")
        .replace('–', "-")
        .replace('—', "-");
    out = PDF_CONTROL_RE.replace_all(&out, " ").to_string();
    out.lines()
        .map(normalize_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_line(line: &str) -> String {
    let mut text = PAGE_FOOTER_RE.replace_all(line, "").to_string();
    text = text.replace('\u{00a0}', " ");
    normalize_ws(&text)
}

fn extract_printed_label(raw_text: &str) -> Option<String> {
    PAGE_FOOTER_RE
        .find(raw_text)
        .map(|m| m.as_str().replace("UTCR 8/1/2025", "").trim().to_string())
}

fn body_start_index(lines: &[PageLine]) -> Option<usize> {
    let mut by_page = BTreeMap::<usize, Vec<&str>>::new();
    for line in lines {
        by_page
            .entry(line.page_number)
            .or_default()
            .push(line.text.as_str());
    }

    let body_page = by_page.iter().find_map(|(page, page_lines)| {
        let has_contents = page_lines
            .iter()
            .any(|line| line.eq_ignore_ascii_case("CONTENTS"));
        let has_chapter_one = page_lines.iter().any(|line| {
            CHAPTER_NUMBER_RE
                .captures(line)
                .is_some_and(|caps| &caps[1] == "1")
        });
        let has_rule_one = page_lines.iter().any(|line| line.trim() == "1.010");
        (has_chapter_one && has_rule_one && !has_contents).then_some(*page)
    })?;

    lines.iter().position(|line| line.page_number == body_page)
}

fn parse_toc_entries(lines: &[PageLine], pages: &[SourcePage]) -> Vec<SourceTocEntry> {
    let mut entries = Vec::new();
    let mut order = 0usize;
    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if let Some((chapter, title, next_i)) = parse_chapter_start(lines, i) {
            order += 1;
            let page_label = toc_page_label_between(lines, i, next_i);
            entries.push(SourceTocEntry {
                source_toc_entry_id: format!("toc:utcr:2025:chapter:{chapter}"),
                source_document_id: SOURCE_DOCUMENT_ID.to_string(),
                citation: Some(format!("UTCR Chapter {chapter}")),
                canonical_id: Some(format!("or:utcr:chapter:{chapter}")),
                title,
                chapter: Some(chapter),
                page_label,
                page_number: Some(line.page_number),
                toc_order: order,
                entry_type: "chapter".to_string(),
                confidence: 0.86,
            });
            i = next_i.max(i + 1);
            continue;
        }

        if let Some((citation, title, page_label, next_i)) = parse_toc_rule_entry(lines, i) {
            order += 1;
            entries.push(SourceTocEntry {
                source_toc_entry_id: format!("toc:utcr:2025:rule:{}", safe_id(&citation)),
                source_document_id: SOURCE_DOCUMENT_ID.to_string(),
                citation: Some(format!("UTCR {citation}")),
                canonical_id: Some(format!("or:utcr:{citation}")),
                title,
                chapter: citation.split('.').next().map(ToString::to_string),
                page_label: page_label.clone(),
                page_number: pages
                    .iter()
                    .find(|page| page.printed_label == page_label)
                    .map(|page| page.page_number),
                toc_order: order,
                entry_type: "rule".to_string(),
                confidence: 0.82,
            });
            i = next_i.max(i + 1);
            continue;
        }

        i += 1;
    }
    entries
}

fn toc_page_label_between(lines: &[PageLine], start: usize, end: usize) -> Option<String> {
    lines[start.saturating_add(1)..end.min(lines.len())]
        .iter()
        .find(|line| PAGE_LABEL_RE.is_match(&line.text))
        .map(|line| line.text.clone())
}

fn parse_toc_rule_entry(
    lines: &[PageLine],
    index: usize,
) -> Option<(String, String, Option<String>, usize)> {
    let citation = RULE_NUMBER_RE
        .captures(&lines.get(index)?.text)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))?;
    let mut title_parts = Vec::<String>::new();
    let mut page_label = None;
    let mut i = index + 1;
    while i < lines.len() {
        let text = lines[i].text.trim();
        if is_noise_line(text) || text == "-" {
            i += 1;
            continue;
        }
        if PAGE_LABEL_RE.is_match(text) && !title_parts.is_empty() {
            page_label = Some(text.to_string());
            i += 1;
            break;
        }
        if CHAPTER_NUMBER_RE.is_match(text) || RULE_NUMBER_RE.is_match(text) {
            break;
        }
        if LEADER_RE.is_match(text) {
            i += 1;
            continue;
        }
        title_parts.push(strip_toc_fragment(text));
        i += 1;
    }
    let title = normalize_ws(&title_parts.join(" "));
    (!title.is_empty()).then_some((citation, title, page_label, i))
}

fn strip_toc_fragment(value: &str) -> String {
    let before_leaders = LEADER_RE.split(value).next().unwrap_or(value);
    normalize_ws(before_leaders)
}

fn logical_body_lines(lines: &[PageLine]) -> Vec<PageLine> {
    let mut filtered = Vec::<PageLine>::new();
    let mut page_index = 0usize;
    let mut current_page = None::<usize>;
    let mut skipped_page_label = false;

    for line in lines {
        if current_page != Some(line.page_number) {
            current_page = Some(line.page_number);
            page_index = 0;
            skipped_page_label = false;
        }
        page_index += 1;

        let text = line.text.trim();
        if is_body_header_fragment(text, page_index) {
            continue;
        }
        if !skipped_page_label && page_index <= 8 && PAGE_LABEL_RE.is_match(text) {
            skipped_page_label = true;
            continue;
        }
        if LEADER_RE.is_match(text) {
            continue;
        }
        filtered.push(line.clone());
    }

    let mut merged = Vec::<PageLine>::new();
    let mut i = 0usize;
    while i < filtered.len() {
        let line = &filtered[i];
        if let Some(prefix) = SPLIT_RULE_PREFIX_RE
            .captures(&line.text)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        {
            if let Some(next) = filtered.get(i + 1) {
                if SPLIT_RULE_SUFFIX_RE.is_match(&next.text) {
                    merged.push(PageLine {
                        page_number: line.page_number,
                        text: format!("{prefix}.{}", next.text),
                    });
                    i += 2;
                    continue;
                }
            }
        }
        if line.text.starts_with('(') && !line.text.ends_with(')') {
            if line.text == "(" {
                if let (Some(marker), Some(close)) = (filtered.get(i + 1), filtered.get(i + 2)) {
                    if close.text == ")" {
                        merged.push(PageLine {
                            page_number: line.page_number,
                            text: format!("({})", marker.text),
                        });
                        i += 3;
                        continue;
                    }
                }
            }
            if let Some(next) = filtered.get(i + 1) {
                if next.text == ")" {
                    merged.push(PageLine {
                        page_number: line.page_number,
                        text: format!("{})", line.text),
                    });
                    i += 2;
                    continue;
                }
            }
        }
        if line.text.eq_ignore_ascii_case("REPORTER") {
            if let (Some(apostrophe), Some(rest)) = (filtered.get(i + 1), filtered.get(i + 2)) {
                if apostrophe.text == "'" && rest.text.eq_ignore_ascii_case("S NOTE") {
                    merged.push(PageLine {
                        page_number: line.page_number,
                        text: "REPORTER'S NOTE".to_string(),
                    });
                    i += 3;
                    continue;
                }
            }
        }
        merged.push(line.clone());
        i += 1;
    }
    merged
}

fn is_body_header_fragment(text: &str, page_index: usize) -> bool {
    if text == "UTCR" || text.starts_with("UTCR 8/1/") || text == "8/1/202" || text == "8/1/2025" {
        return true;
    }
    page_index <= 6 && matches!(text, "20" | "2" | "5" | "2025")
}

fn is_noise_line(text: &str) -> bool {
    text.is_empty()
        || text.eq_ignore_ascii_case("Page")
        || text.eq_ignore_ascii_case("CONTENTS")
        || text.eq_ignore_ascii_case("UNIFORM TRIAL COURT RULES")
        || LEADER_RE.is_match(text)
}

fn parse_body_rules(
    lines: &[PageLine],
    config: &UtcrParseConfig,
) -> (Vec<CourtRuleChapter>, Vec<RuleDraft>, Vec<ParserDiagnostic>) {
    let mut chapters = Vec::<CourtRuleChapter>::new();
    let mut rules = Vec::<RuleDraft>::new();
    let mut diagnostics = Vec::<ParserDiagnostic>::new();
    let mut current_chapter = String::new();
    let mut current_chapter_title = String::new();
    let mut current_rule: Option<RuleDraft> = None;

    let mut i = 0usize;
    while i < lines.len() {
        if let Some((chapter, title, next_i)) = parse_chapter_start(lines, i) {
            if let Some(rule) = current_rule.take() {
                rules.push(rule);
            }
            current_chapter = chapter;
            current_chapter_title = title;
            chapters.push(CourtRuleChapter {
                chapter_id: format!(
                    "or:utcr:chapter:{}@{}",
                    current_chapter, config.edition_year
                ),
                corpus_id: CORPUS_ID.to_string(),
                edition_id: EDITION_ID.to_string(),
                chapter: current_chapter.clone(),
                title: current_chapter_title.clone(),
                citation: format!("UTCR Chapter {current_chapter}"),
                edition_year: config.edition_year,
                effective_date: config.effective_date.clone(),
                source_page_start: Some(lines[i].page_number),
                source_page_end: Some(lines[i].page_number),
            });
            i = next_i.max(i + 1);
            continue;
        }

        if let Some((citation, title, next_i)) = parse_rule_start(lines, i) {
            if let Some(rule) = current_rule.take() {
                rules.push(rule);
            }
            let rule_chapter = citation.split('.').next().unwrap_or_default().to_string();
            if current_chapter.is_empty() || current_chapter != rule_chapter {
                current_chapter = rule_chapter;
                diagnostics.push(diagnostic(
                    "warning",
                    "rule_before_chapter",
                    &format!("Rule {citation} appeared before a parsed chapter heading"),
                    Some(lines[i].page_number),
                    Some(format!("or:utcr:{citation}")),
                ));
            }
            current_rule = Some(RuleDraft {
                citation,
                title,
                chapter: current_chapter.clone(),
                chapter_title: current_chapter_title.clone(),
                start_page: lines[i].page_number,
                end_page: lines[i].page_number,
                lines: Vec::new(),
            });
            i = next_i.max(i + 1);
            continue;
        }

        if let Some(rule) = current_rule.as_mut() {
            rule.end_page = lines[i].page_number;
            rule.lines.push(lines[i].clone());
        }
        if let Some(chapter) = chapters.last_mut() {
            chapter.source_page_end = Some(lines[i].page_number);
        }
        i += 1;
    }

    if let Some(rule) = current_rule {
        rules.push(rule);
    }

    (chapters, rules, diagnostics)
}

fn chapter_headings_from_chapters(chapters: &[CourtRuleChapter]) -> Vec<ChapterHeading> {
    chapters
        .iter()
        .enumerate()
        .map(|(idx, chapter)| ChapterHeading {
            heading_id: format!("{}:heading", chapter.chapter_id),
            chapter: chapter.chapter.clone(),
            text: format!("CHAPTER {} - {}", chapter.chapter, chapter.title),
            order_index: idx + 1,
        })
        .collect()
}

fn parse_chapter_start(lines: &[PageLine], index: usize) -> Option<(String, String, usize)> {
    let line = lines.get(index)?;
    if let Some(caps) = CHAPTER_RE.captures(&line.text) {
        let chapter = caps.get(1)?.as_str().to_string();
        let title = normalize_ws(caps.get(2)?.as_str());
        return Some((chapter, title, index + 1));
    }

    let caps = CHAPTER_NUMBER_RE.captures(&line.text)?;
    let chapter = caps.get(1)?.as_str().to_string();
    let mut title_parts = Vec::<String>::new();
    let mut i = index + 1;
    while i < lines.len() {
        let text = lines[i].text.trim();
        if is_noise_line(text) || text == "-" || PAGE_LABEL_RE.is_match(text) {
            i += 1;
            continue;
        }
        if RULE_NUMBER_RE.is_match(text)
            || SPLIT_RULE_PREFIX_RE.is_match(text)
            || CHAPTER_NUMBER_RE.is_match(text)
            || text.starts_with('(')
            || text.eq_ignore_ascii_case("REPORTER'S NOTE")
            || text
                .to_ascii_lowercase()
                .starts_with("this chapter reserved")
        {
            break;
        }
        title_parts.push(text.to_string());
        i += 1;
    }
    let title = normalize_ws(&title_parts.join(" "));
    Some((chapter, title, i))
}

fn parse_rule_start(lines: &[PageLine], index: usize) -> Option<(String, String, usize)> {
    let line = lines.get(index)?;
    if index > 0
        && matches!(
            lines[index - 1].text.to_ascii_uppercase().as_str(),
            "ORS" | "ORCP" | "ORAP" | "UTCR" | "SLR"
        )
    {
        return None;
    }
    if let Some((citation, title)) = parse_rule_heading(&line.text) {
        return Some((citation, title, index + 1));
    }

    let citation = RULE_NUMBER_RE
        .captures(&line.text)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))?;
    let mut title_parts = Vec::<String>::new();
    let mut i = index + 1;
    while i < lines.len() {
        let text = lines[i].text.trim();
        if is_noise_line(text) || text == "-" || PAGE_LABEL_RE.is_match(text) {
            i += 1;
            continue;
        }
        if PROVISION_MARKER_RE.is_match(text)
            || CHAPTER_NUMBER_RE.is_match(text)
            || RULE_NUMBER_RE.is_match(text)
            || SPLIT_RULE_PREFIX_RE.is_match(text)
            || text.eq_ignore_ascii_case("REPORTER'S NOTE")
        {
            break;
        }
        if !is_title_line(text) {
            break;
        }
        title_parts.push(text.to_string());
        i += 1;
    }
    let title = normalize_ws(&title_parts.join(" "));
    (!title.is_empty()).then_some((citation, title, i))
}

fn parse_rule_heading(line: &str) -> Option<(String, String)> {
    if LEADER_RE.is_match(line) {
        return None;
    }
    let caps = RULE_HEADING_RE.captures(line)?;
    let citation = caps.get(1)?.as_str().to_string();
    let title = normalize_ws(caps.get(2)?.as_str());
    if title.len() < 3 || !is_title_line(&title) {
        return None;
    }
    Some((citation, title))
}

fn is_title_line(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.starts_with(',') || trimmed.starts_with(';') {
        return false;
    }
    if trimmed.len() <= 1 {
        return matches!(trimmed, "'" | "\"" | "-" | ":");
    }
    if trimmed.contains("(Repealed)") || trimmed.contains("(Reserved)") {
        return true;
    }
    let alpha_count = trimmed
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .count();
    if alpha_count == 0 {
        return false;
    }
    let uppercase_count = trimmed
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic() && ch.is_ascii_uppercase())
        .count();
    uppercase_count * 100 / alpha_count >= 65
}

fn push_rule(
    parsed: &mut ParsedUtcrCorpus,
    draft: &RuleDraft,
    config: &UtcrParseConfig,
    provision_lookup: &mut HashMap<(String, Vec<String>), String>,
) {
    let canonical_id = format!("or:utcr:{}", draft.citation);
    let version_id = format!("{}@{}", canonical_id, config.effective_date);
    let body_text = normalize_ws(
        &draft
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let status = if draft.title.to_ascii_lowercase().contains("repealed") {
        "repealed"
    } else {
        "active"
    };

    parsed.identities.push(LegalTextIdentity {
        canonical_id: canonical_id.clone(),
        citation: format!("UTCR {}", draft.citation),
        jurisdiction_id: JURISDICTION_ID.to_string(),
        authority_family: AUTHORITY_FAMILY.to_string(),
        corpus_id: Some(CORPUS_ID.to_string()),
        authority_type: Some(AUTHORITY_TYPE.to_string()),
        authority_level: Some(AUTHORITY_LEVEL),
        effective_date: Some(config.effective_date.clone()),
        title: Some(draft.title.clone()),
        chapter: draft.chapter.clone(),
        status: status.to_string(),
    });

    parsed.versions.push(LegalTextVersion {
        version_id: version_id.clone(),
        canonical_id: canonical_id.clone(),
        citation: format!("UTCR {}", draft.citation),
        title: Some(draft.title.clone()),
        chapter: draft.chapter.clone(),
        corpus_id: Some(CORPUS_ID.to_string()),
        edition_id: Some(EDITION_ID.to_string()),
        authority_family: Some(AUTHORITY_FAMILY.to_string()),
        authority_type: Some(AUTHORITY_TYPE.to_string()),
        authority_level: Some(AUTHORITY_LEVEL),
        effective_date: Some(config.effective_date.clone()),
        source_page_start: Some(draft.start_page),
        source_page_end: Some(draft.end_page),
        edition_year: config.edition_year,
        status: status.to_string(),
        status_text: (status != "active").then(|| draft.title.clone()),
        text: body_text.clone(),
        text_hash: sha256_hex(body_text.as_bytes()),
        original_text: Some(
            draft
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        paragraph_start_order: None,
        paragraph_end_order: None,
        source_paragraph_ids: Vec::new(),
        source_document_id: SOURCE_DOCUMENT_ID.to_string(),
        official_status: "official_pdf".to_string(),
        disclaimer_required: false,
        embedding_model: None,
        embedding_dim: None,
        embedding: None,
        embedding_input_hash: None,
        embedding_input_type: Some("document".to_string()),
        embedding_output_dtype: Some("float".to_string()),
        embedded_at: None,
        embedding_profile: Some("legal_rule_provision_primary_v1".to_string()),
        embedding_strategy: Some("utcr_rule_v1".to_string()),
        embedding_source_dimension: None,
    });

    let (body_lines, notes, commentaries) = split_notes_and_commentary(&draft.lines);
    for note in notes {
        parsed.reporter_notes.push(ReporterNote {
            reporter_note_id: format!(
                "reporter_note:{}",
                stable_id(&format!("{}::{note}", draft.citation))
            ),
            source_document_id: SOURCE_DOCUMENT_ID.to_string(),
            canonical_id: Some(canonical_id.clone()),
            version_id: Some(version_id.clone()),
            source_provision_id: None,
            citation: Some(format!("UTCR {}", draft.citation)),
            normalized_text: normalize_ws(&note),
            text: note,
            source_page_start: Some(draft.start_page),
            source_page_end: Some(draft.end_page),
            confidence: 0.82,
            extraction_method: PARSER_PROFILE.to_string(),
        });
    }
    for commentary in commentaries {
        parsed.commentaries.push(Commentary {
            commentary_id: format!(
                "commentary:{}",
                stable_id(&format!("{}::{commentary}", draft.citation))
            ),
            source_document_id: SOURCE_DOCUMENT_ID.to_string(),
            canonical_id: Some(canonical_id.clone()),
            version_id: Some(version_id.clone()),
            source_provision_id: None,
            target_canonical_id: Some(canonical_id.clone()),
            target_provision_id: None,
            citation: Some(format!("UTCR {}", draft.citation)),
            authority_family: Some("UTCR".to_string()),
            corpus_id: Some(CORPUS_ID.to_string()),
            authority_level: Some(80),
            source_role: Some("official_commentary".to_string()),
            commentary_type: "historical_commentary".to_string(),
            normalized_text: normalize_ws(&commentary),
            text: commentary,
            source_page_start: Some(draft.start_page),
            source_page_end: Some(draft.end_page),
            confidence: 0.78,
            extraction_method: PARSER_PROFILE.to_string(),
        });
    }

    let mut provisions = parse_provisions(draft, &body_lines, &canonical_id, &version_id, config);
    disambiguate_duplicate_provision_paths(&mut provisions, &draft.citation, &version_id);
    for provision in provisions {
        provision_lookup.insert(
            (provision.canonical_id.clone(), provision.local_path.clone()),
            provision.provision_id.clone(),
        );
        parsed.provisions.push(provision);
    }
}

fn split_notes_and_commentary(lines: &[PageLine]) -> (Vec<PageLine>, Vec<String>, Vec<String>) {
    let mut body = Vec::new();
    let mut notes = Vec::new();
    let mut commentaries = Vec::new();
    let mut current_note = Vec::new();
    let mut current_commentary = Vec::new();
    let mut in_note = false;
    let mut in_commentary = false;
    for line in lines {
        if line
            .text
            .to_ascii_uppercase()
            .starts_with("REPORTER'S NOTE")
        {
            if !current_commentary.is_empty() {
                commentaries.push(current_commentary.join(" "));
                current_commentary.clear();
            }
            if !current_note.is_empty() {
                notes.push(current_note.join(" "));
                current_note.clear();
            }
            in_note = true;
            in_commentary = false;
            current_note.push(line.text.clone());
            continue;
        }
        if COMMENTARY_START_RE.is_match(&line.text) {
            if !current_note.is_empty() {
                notes.push(current_note.join(" "));
                current_note.clear();
            }
            if !current_commentary.is_empty() {
                commentaries.push(current_commentary.join(" "));
                current_commentary.clear();
            }
            in_note = false;
            in_commentary = true;
            current_commentary.push(line.text.clone());
            continue;
        }
        if in_note {
            current_note.push(line.text.clone());
        } else if in_commentary {
            current_commentary.push(line.text.clone());
        } else {
            body.push(line.clone());
        }
    }
    if !current_note.is_empty() {
        notes.push(current_note.join(" "));
    }
    if !current_commentary.is_empty() {
        commentaries.push(current_commentary.join(" "));
    }
    (body, notes, commentaries)
}

fn parse_provisions(
    draft: &RuleDraft,
    lines: &[PageLine],
    canonical_id: &str,
    version_id: &str,
    config: &UtcrParseConfig,
) -> Vec<Provision> {
    let mut provisions = Vec::new();
    let mut current_path = Vec::<String>::new();
    let mut current_text = String::new();
    let mut current_start_page = draft.start_page;
    let mut order_index = 0usize;

    let flush = |provisions: &mut Vec<Provision>,
                 current_path: &mut Vec<String>,
                 current_text: &mut String,
                 current_start_page: usize,
                 current_end_page: usize,
                 order_index: &mut usize| {
        let text = normalize_ws(current_text);
        if text.is_empty() || current_path.is_empty() {
            current_text.clear();
            return;
        }
        let original = current_text.trim().to_string();
        let display = display_citation(&draft.citation, current_path);
        let provision_id = format!(
            "{}:{}",
            version_id,
            current_path
                .iter()
                .map(|segment| safe_id(segment))
                .collect::<Vec<_>>()
                .join(":")
        );
        provisions.push(Provision {
            provision_id,
            version_id: version_id.to_string(),
            canonical_id: canonical_id.to_string(),
            citation: format!("UTCR {}", draft.citation),
            display_citation: display,
            chapter: Some(draft.chapter.clone()),
            corpus_id: Some(CORPUS_ID.to_string()),
            edition_id: Some(EDITION_ID.to_string()),
            authority_family: Some(AUTHORITY_FAMILY.to_string()),
            authority_type: Some(AUTHORITY_TYPE.to_string()),
            authority_level: Some(AUTHORITY_LEVEL),
            effective_date: Some(config.effective_date.clone()),
            source_page_start: Some(current_start_page),
            source_page_end: Some(current_end_page),
            local_path: current_path.clone(),
            provision_type: provision_type(current_path).to_string(),
            text: text.clone(),
            original_text: Some(original),
            normalized_text: text.clone(),
            order_index: *order_index,
            depth: current_path.len(),
            text_hash: sha256_hex(text.as_bytes()),
            is_implied: false,
            is_definition_candidate: is_definition_candidate(&text),
            is_exception_candidate: is_exception_candidate(&text),
            is_deadline_candidate: is_deadline_candidate(&text),
            is_penalty_candidate: is_penalty_candidate(&text),
            paragraph_start_order: None,
            paragraph_end_order: None,
            source_paragraph_ids: Vec::new(),
            heading_path: vec![format!(
                "Chapter {} - {}",
                draft.chapter, draft.chapter_title
            )],
            structural_context: Some(format!(
                "Oregon Uniform Trial Court Rules > Chapter {} > UTCR {}",
                draft.chapter, draft.citation
            )),
            embedding_model: None,
            embedding_dim: None,
            embedding: None,
            embedding_input_hash: None,
            embedding_input_type: Some("document".to_string()),
            embedding_output_dtype: Some("float".to_string()),
            embedded_at: None,
            embedding_profile: Some("legal_rule_provision_primary_v1".to_string()),
            embedding_source_dimension: None,
        });
        *order_index += 1;
        current_text.clear();
    };

    let mut current_end_page = draft.start_page;
    for line in lines {
        current_end_page = line.page_number;
        if let Some(caps) = PROVISION_MARKER_RE.captures(&line.text) {
            flush(
                &mut provisions,
                &mut current_path,
                &mut current_text,
                current_start_page,
                current_end_page,
                &mut order_index,
            );
            let marker = normalize_provision_marker(caps.get(1).unwrap().as_str());
            current_path = next_path(&current_path, &marker);
            current_start_page = line.page_number;
            current_text.push_str(caps.get(2).map(|m| m.as_str()).unwrap_or_default());
        } else {
            if current_path.is_empty() && !line.text.trim().is_empty() {
                current_path = vec!["root".to_string()];
                current_start_page = line.page_number;
            }
            if !current_text.is_empty() {
                current_text.push(' ');
            }
            current_text.push_str(&line.text);
        }
    }
    flush(
        &mut provisions,
        &mut current_path,
        &mut current_text,
        current_start_page,
        current_end_page,
        &mut order_index,
    );

    if provisions.is_empty() {
        let text = normalize_ws(
            &lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        );
        if !text.is_empty() {
            provisions.push(Provision {
                provision_id: format!("{version_id}:root"),
                version_id: version_id.to_string(),
                canonical_id: canonical_id.to_string(),
                citation: format!("UTCR {}", draft.citation),
                display_citation: format!("UTCR {}", draft.citation),
                chapter: Some(draft.chapter.clone()),
                corpus_id: Some(CORPUS_ID.to_string()),
                edition_id: Some(EDITION_ID.to_string()),
                authority_family: Some(AUTHORITY_FAMILY.to_string()),
                authority_type: Some(AUTHORITY_TYPE.to_string()),
                authority_level: Some(AUTHORITY_LEVEL),
                effective_date: Some(config.effective_date.clone()),
                source_page_start: Some(draft.start_page),
                source_page_end: Some(draft.end_page),
                local_path: vec!["root".to_string()],
                provision_type: "rule_text".to_string(),
                text: text.clone(),
                original_text: Some(text.clone()),
                normalized_text: text.clone(),
                order_index: 0,
                depth: 1,
                text_hash: sha256_hex(text.as_bytes()),
                is_implied: false,
                is_definition_candidate: is_definition_candidate(&text),
                is_exception_candidate: is_exception_candidate(&text),
                is_deadline_candidate: is_deadline_candidate(&text),
                is_penalty_candidate: is_penalty_candidate(&text),
                paragraph_start_order: None,
                paragraph_end_order: None,
                source_paragraph_ids: Vec::new(),
                heading_path: vec![format!(
                    "Chapter {} - {}",
                    draft.chapter, draft.chapter_title
                )],
                structural_context: Some(format!(
                    "Oregon Uniform Trial Court Rules > Chapter {} > UTCR {}",
                    draft.chapter, draft.citation
                )),
                embedding_model: None,
                embedding_dim: None,
                embedding: None,
                embedding_input_hash: None,
                embedding_input_type: Some("document".to_string()),
                embedding_output_dtype: Some("float".to_string()),
                embedded_at: None,
                embedding_profile: Some("legal_rule_provision_primary_v1".to_string()),
                embedding_source_dimension: None,
            });
        }
    }
    provisions
}

fn disambiguate_duplicate_provision_paths(
    provisions: &mut [Provision],
    rule_citation: &str,
    version_id: &str,
) {
    let mut seen = HashMap::<Vec<String>, usize>::new();
    for provision in provisions {
        let count = seen.entry(provision.local_path.clone()).or_insert(0);
        *count += 1;
        if *count == 1 {
            continue;
        }
        provision.local_path.push(format!("repeat{count}"));
        provision.display_citation = display_citation(rule_citation, &provision.local_path);
        provision.provision_id = format!(
            "{}:{}",
            version_id,
            provision
                .local_path
                .iter()
                .map(|segment| safe_id(segment))
                .collect::<Vec<_>>()
                .join(":")
        );
        provision.depth = provision.local_path.len();
        provision.provision_type = provision_type(&provision.local_path).to_string();
    }
}

fn normalize_provision_marker(marker: &str) -> String {
    marker
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn next_path(current: &[String], marker: &str) -> Vec<String> {
    if marker.chars().all(|ch| ch.is_ascii_digit()) {
        return vec![marker.to_string()];
    }
    if is_roman(marker) && current.len() >= 2 {
        let mut path = current[..2].to_vec();
        path.push(marker.to_ascii_lowercase());
        return path;
    }
    if marker.chars().all(|ch| ch.is_ascii_alphabetic()) {
        let mut path = if current.is_empty() {
            Vec::new()
        } else {
            current[..1].to_vec()
        };
        path.push(marker.to_ascii_lowercase());
        return path;
    }
    let mut path = current.to_vec();
    path.push(marker.to_string());
    path
}

fn is_roman(marker: &str) -> bool {
    matches!(
        marker.to_ascii_lowercase().as_str(),
        "i" | "ii"
            | "iii"
            | "iv"
            | "v"
            | "vi"
            | "vii"
            | "viii"
            | "ix"
            | "x"
            | "xi"
            | "xii"
            | "xiii"
            | "xiv"
            | "xv"
    )
}

fn display_citation(rule_citation: &str, path: &[String]) -> String {
    if path.len() == 1 && path[0] == "root" {
        return format!("UTCR {rule_citation}");
    }
    let pins = path
        .iter()
        .map(|segment| format!("({segment})"))
        .collect::<Vec<_>>()
        .join("");
    format!("UTCR {rule_citation}{pins}")
}

fn provision_type(path: &[String]) -> &'static str {
    if path.len() == 1 && path[0] == "root" {
        return "rule_text";
    }
    match path.len() {
        0 => "rule_text",
        1 => "subsection",
        2 => "paragraph",
        3 => "subparagraph",
        _ => "clause",
    }
}

fn extract_citations(
    parsed: &mut ParsedUtcrCorpus,
    provision_lookup: &HashMap<(String, Vec<String>), String>,
) {
    let known_rules = parsed
        .identities
        .iter()
        .map(|identity| identity.canonical_id.clone())
        .collect::<HashSet<_>>();
    let mut seen_mentions = HashSet::new();
    let mut external_by_id = HashMap::<String, ExternalLegalCitation>::new();
    let mut edges = Vec::new();

    for provision in &parsed.provisions {
        extract_utcr_citations(
            provision,
            provision_lookup,
            &known_rules,
            &mut seen_mentions,
            &mut parsed.citation_mentions,
            &mut edges,
        );
        extract_ors_citations(
            provision,
            &mut seen_mentions,
            &mut parsed.citation_mentions,
            &mut edges,
        );
        extract_external_citations(
            provision,
            &mut seen_mentions,
            &mut parsed.citation_mentions,
            &mut external_by_id,
        );
    }
    parsed.external_legal_citations = external_by_id.into_values().collect();
    parsed.cites_edges = edges;
}

fn extract_utcr_citations(
    provision: &Provision,
    provision_lookup: &HashMap<(String, Vec<String>), String>,
    known_rules: &HashSet<String>,
    seen: &mut HashSet<String>,
    mentions: &mut Vec<CitationMention>,
    edges: &mut Vec<CitesEdge>,
) {
    for caps in UTCR_CITATION_RE.captures_iter(&provision.text) {
        let raw = caps.get(0).unwrap().as_str().trim().to_string();
        let rule = caps.get(1).unwrap().as_str();
        let pins = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
        let path = RULE_PIN_RE
            .captures_iter(pins)
            .filter_map(|pin| pin.get(1).map(|m| m.as_str().to_ascii_lowercase()))
            .collect::<Vec<_>>();
        let target_canonical_id = format!("or:utcr:{rule}");
        let target_provision_id = if path.is_empty() {
            None
        } else {
            provision_lookup
                .get(&(target_canonical_id.clone(), path.clone()))
                .cloned()
        };
        let mention_id = format!(
            "cite:{}",
            stable_id(&format!("{}::{raw}", provision.provision_id))
        );
        if !seen.insert(mention_id.clone()) {
            continue;
        }
        let resolved = known_rules.contains(&target_canonical_id);
        mentions.push(CitationMention {
            citation_mention_id: mention_id.clone(),
            source_provision_id: provision.provision_id.clone(),
            raw_text: raw.clone(),
            normalized_citation: format!("UTCR {rule}{pins}"),
            citation_type: if path.is_empty() {
                "utcr_rule".to_string()
            } else {
                "utcr_provision".to_string()
            },
            target_canonical_id: resolved.then(|| target_canonical_id.clone()),
            target_start_canonical_id: None,
            target_end_canonical_id: None,
            target_provision_id: target_provision_id.clone(),
            unresolved_subpath: (!path.is_empty() && target_provision_id.is_none()).then_some(path),
            external_citation_id: None,
            resolver_status: if resolved {
                "resolved_internal".to_string()
            } else {
                "unresolved_target_not_in_corpus".to_string()
            },
            confidence: 0.94,
            qc_severity: (!resolved).then(|| "warning".to_string()),
        });
        if resolved {
            edges.push(CitesEdge {
                edge_id: format!("edge:{mention_id}:CITES:{target_canonical_id}"),
                edge_type: "CITES".to_string(),
                source_provision_id: provision.provision_id.clone(),
                target_canonical_id: Some(target_canonical_id.clone()),
                target_version_id: None,
                target_provision_id,
                target_chapter_id: None,
                citation_kind: Some("utcr".to_string()),
                citation_mention_id: mention_id,
            });
        }
    }
}

fn extract_ors_citations(
    provision: &Provision,
    seen: &mut HashSet<String>,
    mentions: &mut Vec<CitationMention>,
    edges: &mut Vec<CitesEdge>,
) {
    for mat in ORS_CITATION_RE.find_iter(&provision.text) {
        let raw = mat.as_str().trim().trim_end_matches('.').to_string();
        let mention_id = format!(
            "cite:{}",
            stable_id(&format!("{}::{raw}", provision.provision_id))
        );
        if !seen.insert(mention_id.clone()) {
            continue;
        }
        let target = ors_target_canonical_id(&raw);
        mentions.push(CitationMention {
            citation_mention_id: mention_id.clone(),
            source_provision_id: provision.provision_id.clone(),
            raw_text: raw.clone(),
            normalized_citation: raw.clone(),
            citation_type: "statute_reference".to_string(),
            target_canonical_id: target.clone(),
            target_start_canonical_id: None,
            target_end_canonical_id: None,
            target_provision_id: None,
            unresolved_subpath: None,
            external_citation_id: None,
            resolver_status: "parsed_unverified".to_string(),
            confidence: 0.88,
            qc_severity: None,
        });
        if let Some(target_canonical_id) = target {
            edges.push(CitesEdge {
                edge_id: format!("edge:{mention_id}:CITES:{target_canonical_id}"),
                edge_type: "CITES".to_string(),
                source_provision_id: provision.provision_id.clone(),
                target_canonical_id: Some(target_canonical_id),
                target_version_id: None,
                target_provision_id: None,
                target_chapter_id: None,
                citation_kind: Some("ors".to_string()),
                citation_mention_id: mention_id,
            });
        }
    }
}

fn ors_target_canonical_id(raw: &str) -> Option<String> {
    let section = Regex::new(r"(?i)\bORS\s+([0-9]{1,3}[A-Z]?\.[0-9]{3,4})")
        .unwrap()
        .captures(raw)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))?;
    Some(format!("or:ors:{section}"))
}

fn extract_external_citations(
    provision: &Provision,
    seen: &mut HashSet<String>,
    mentions: &mut Vec<CitationMention>,
    external_by_id: &mut HashMap<String, ExternalLegalCitation>,
) {
    for (citation_type, regex) in [
        ("orcp", &*ORCP_CITATION_RE),
        ("slr", &*SLR_CITATION_RE),
        ("orap", &*ORAP_CITATION_RE),
        ("court_order", &*ORDER_CITATION_RE),
        ("url", &*URL_RE),
    ] {
        for mat in regex.find_iter(&provision.text) {
            let raw = mat.as_str().trim().trim_end_matches('.').to_string();
            let mention_id = format!(
                "cite:{}",
                stable_id(&format!("{}::{raw}", provision.provision_id))
            );
            if !seen.insert(mention_id.clone()) {
                continue;
            }
            let external_id = format!("external:{}:{}", citation_type, stable_id(&raw));
            external_by_id
                .entry(external_id.clone())
                .or_insert_with(|| ExternalLegalCitation {
                    external_citation_id: external_id.clone(),
                    citation: raw.clone(),
                    normalized_citation: normalize_ws(&raw),
                    citation_type: citation_type.to_string(),
                    jurisdiction_id: JURISDICTION_ID.to_string(),
                    source_system: PARSER_PROFILE.to_string(),
                    url: (citation_type == "url").then(|| normalize_url(&raw)),
                });
            mentions.push(CitationMention {
                citation_mention_id: mention_id,
                source_provision_id: provision.provision_id.clone(),
                raw_text: raw.clone(),
                normalized_citation: normalize_ws(&raw),
                citation_type: citation_type.to_string(),
                target_canonical_id: None,
                target_start_canonical_id: None,
                target_end_canonical_id: None,
                target_provision_id: None,
                unresolved_subpath: None,
                external_citation_id: Some(external_id),
                resolver_status: "resolved_external_placeholder".to_string(),
                confidence: 0.84,
                qc_severity: None,
            });
        }
    }
}

fn normalize_url(raw: &str) -> String {
    if raw.to_ascii_lowercase().starts_with("www.") {
        format!("https://{raw}")
    } else {
        raw.to_string()
    }
}

fn extract_requirements(parsed: &mut ParsedUtcrCorpus, config: &UtcrParseConfig) {
    let mut generated = Vec::<(&'static str, ProceduralRequirement)>::new();
    for provision in &parsed.provisions {
        let lower = provision.text.to_ascii_lowercase();
        let mut types = Vec::<(&str, &str, &str, Vec<&str>, Option<&str>, &str)>::new();
        if contains_any(
            &lower,
            &[
                "double-spaced",
                "numbered lines",
                "margin",
                "caption",
                "exhibit",
                "font",
                "page number",
                "two inches",
                "one-inch",
            ],
        ) {
            types.push((
                "FormattingRequirement",
                "formatting",
                "formatting",
                vec!["pleading", "motion", "court_document"],
                None,
                "blocking_for_export",
            ));
        }
        if contains_any(
            &lower,
            &[
                "file ",
                "filed",
                "filing",
                "submit",
                "submitted",
                "presented to the court",
            ],
        ) {
            types.push((
                "FilingRequirement",
                "filing",
                "filing",
                vec!["filing_packet", "court_document"],
                None,
                "blocking_for_filing",
            ));
        }
        if lower.contains("exhibit") {
            types.push((
                "ExhibitRequirement",
                "exhibit",
                "exhibit",
                vec!["filing_packet", "trial_document"],
                None,
                "serious",
            ));
        }
        if contains_any(&lower, &["caption", "case number", "party", "title"]) {
            types.push((
                "CaptionRequirement",
                "caption",
                "caption",
                vec!["pleading", "motion", "answer"],
                None,
                "blocking_for_export",
            ));
        }
        if contains_any(
            &lower,
            &["sign", "signature", "dated", "address", "telephone"],
        ) {
            types.push((
                "SignatureRequirement",
                "signature",
                "signature",
                vec!["court_document"],
                None,
                "serious",
            ));
        }
        if lower.contains("certificate of service") || lower.contains("certification of service") {
            types.push((
                "CertificateOfServiceRequirement",
                "certificate",
                "service_certificate",
                vec!["filing_packet", "motion", "answer"],
                None,
                "blocking_for_filing",
            ));
        }
        if contains_any(
            &lower,
            &[
                "served",
                "service",
                "mail",
                "email",
                "facsimile",
                "electronic service",
            ],
        ) {
            types.push((
                "ServiceRequirement",
                "service",
                "service",
                vec!["filing_packet"],
                None,
                "serious",
            ));
        }
        if contains_any(
            &lower,
            &[
                "electronic filing",
                "electronically file",
                "pdf",
                "pdf/a",
                "25 mb",
                "25mb",
                "hyperlink",
                "electronic signature",
            ],
        ) {
            types.push((
                "EfilingRequirement",
                "efiling",
                "efiling",
                vec!["filing_packet"],
                None,
                "blocking_for_filing",
            ));
        }
        if contains_any(
            &lower,
            &[
                "protected personal information",
                "confidential",
                "segregate",
                "redact",
            ],
        ) {
            types.push((
                "ProtectedInformationRequirement",
                "protected_info",
                "protected_information",
                vec!["filing_packet", "court_document"],
                None,
                "blocking_for_filing",
            ));
        }
        if contains_any(
            &lower,
            &["sanction", "strike", "costs", "attorney fees", "failure to"],
        ) {
            types.push((
                "SanctionRule",
                "sanction",
                "sanction",
                vec!["court_document"],
                None,
                "serious",
            ));
        }
        if contains_any(
            &lower,
            &[
                "within",
                "not later than",
                "no later than",
                "days after",
                "days before",
                "by 5:00",
            ],
        ) {
            types.push((
                "DeadlineRule",
                "deadline",
                "deadline",
                vec!["filing_packet", "motion"],
                None,
                "serious",
            ));
        }
        if contains_any(
            &lower,
            &["except", "unless", "waiver", "does not apply", "not apply"],
        ) {
            types.push((
                "ExceptionRule",
                "exception",
                "exception",
                vec!["court_document"],
                None,
                "warning",
            ));
        }
        if contains_any(&lower, &["must", "shall", "required"]) {
            types.push((
                "ProceduralRule",
                "procedural",
                "procedural",
                vec!["court_document"],
                None,
                "warning",
            ));
        }

        for (semantic_type, bucket, requirement_type, applies_to, value, severity) in types {
            let req = requirement(
                provision,
                semantic_type,
                requirement_type,
                applies_to,
                value,
                severity,
                config,
            );
            generated.push((bucket, req));
        }
    }

    for (bucket, req) in generated {
        route_requirement(parsed, bucket, req);
    }

    // Dedicated high-value materializations that power WorkProduct Builder directly.
    for (citation, semantic_type, requirement_type, applies_to, value, severity) in [
        (
            "UTCR 2.010(4)(a)",
            "FormattingRequirement",
            "line_spacing",
            vec!["pleading", "motion", "requested_instruction"],
            Some("double_spaced_numbered_lines"),
            "blocking_for_export",
        ),
        (
            "UTCR 2.010(4)(c)",
            "FormattingRequirement",
            "first_page_top_blank",
            vec!["pleading", "motion"],
            Some("two_inches"),
            "blocking_for_export",
        ),
        (
            "UTCR 2.010(4)(d)",
            "FormattingRequirement",
            "side_margins",
            vec!["court_document"],
            Some("one_inch"),
            "blocking_for_export",
        ),
        (
            "UTCR 2.020",
            "CertificateOfServiceRequirement",
            "certificate_of_service",
            vec!["filing_packet"],
            None,
            "blocking_for_filing",
        ),
        (
            "UTCR 2.100",
            "ProtectedInformationRequirement",
            "protected_personal_information",
            vec!["filing_packet"],
            None,
            "blocking_for_filing",
        ),
        (
            "UTCR 2.110",
            "ProtectedInformationRequirement",
            "existing_protected_information",
            vec!["filing_packet"],
            None,
            "serious",
        ),
        (
            "UTCR 21.040",
            "EfilingRequirement",
            "pdf_format_size",
            vec!["filing_packet"],
            Some("pdf_or_pdfa_max_25_mb"),
            "blocking_for_filing",
        ),
        (
            "UTCR 21.090",
            "EfilingRequirement",
            "electronic_signature",
            vec!["court_document"],
            None,
            "serious",
        ),
        (
            "UTCR 21.100",
            "EfilingRequirement",
            "electronic_service",
            vec!["filing_packet"],
            None,
            "serious",
        ),
        (
            "UTCR 21.110",
            "EfilingRequirement",
            "hyperlink",
            vec!["court_document"],
            None,
            "warning",
        ),
        (
            "UTCR 21.140",
            "EfilingRequirement",
            "mandatory_efiling",
            vec!["filing_packet"],
            None,
            "blocking_for_filing",
        ),
        (
            "UTCR 5.010",
            "MotionRequirement",
            "motion_conferral",
            vec!["motion"],
            None,
            "serious",
        ),
        (
            "UTCR 5.100",
            "OrderRequirement",
            "proposed_order_submission",
            vec!["motion", "proposed_order"],
            None,
            "serious",
        ),
        (
            "UTCR 6.050",
            "ExhibitRequirement",
            "trial_exhibit_submission",
            vec!["trial_document", "exhibit"],
            None,
            "serious",
        ),
        (
            "UTCR 6.080",
            "ExhibitRequirement",
            "marking_exhibits",
            vec!["trial_document", "exhibit"],
            None,
            "serious",
        ),
        (
            "UTCR 19.020",
            "ProceduralRule",
            "contempt_initiating_instrument",
            vec!["contempt_filing"],
            None,
            "blocking_for_filing",
        ),
    ] {
        if let Some(provision) = find_source_provision(&parsed.provisions, citation).cloned() {
            let req = requirement(
                &provision,
                semantic_type,
                requirement_type,
                applies_to,
                value,
                severity,
                config,
            );
            route_requirement(parsed, requirement_bucket(semantic_type), req);
        }
    }

    dedupe_requirements(&mut parsed.procedural_rules);
    dedupe_requirements(&mut parsed.formatting_requirements);
    dedupe_requirements(&mut parsed.filing_requirements);
    dedupe_requirements(&mut parsed.service_requirements);
    dedupe_requirements(&mut parsed.efiling_requirements);
    dedupe_requirements(&mut parsed.caption_requirements);
    dedupe_requirements(&mut parsed.signature_requirements);
    dedupe_requirements(&mut parsed.certificate_requirements);
    dedupe_requirements(&mut parsed.exhibit_requirements);
    dedupe_requirements(&mut parsed.protected_information_rules);
    dedupe_requirements(&mut parsed.sanction_rules);
    dedupe_requirements(&mut parsed.deadline_rules);
    dedupe_requirements(&mut parsed.exception_rules);
}

fn requirement(
    provision: &Provision,
    semantic_type: &str,
    requirement_type: &str,
    applies_to: Vec<&str>,
    value: Option<&str>,
    severity: &str,
    config: &UtcrParseConfig,
) -> ProceduralRequirement {
    let requirement_id = format!(
        "or:utcr:2025:{}:{}",
        requirement_type,
        stable_id(&format!(
            "{}::{requirement_type}::{}",
            provision.provision_id, provision.text
        ))
    );
    ProceduralRequirement {
        requirement_id,
        semantic_type: semantic_type.to_string(),
        requirement_type: requirement_type.to_string(),
        label: label_for_requirement(requirement_type),
        text: provision.text.clone(),
        normalized_text: provision.normalized_text.clone(),
        source_provision_id: provision.provision_id.clone(),
        source_citation: provision.display_citation.clone(),
        applies_to: applies_to.into_iter().map(ToString::to_string).collect(),
        value: value.map(ToString::to_string),
        severity_default: Some(severity.to_string()),
        authority_family: AUTHORITY_FAMILY.to_string(),
        effective_date: config.effective_date.clone(),
        confidence: 0.72,
        extraction_method: "utcr_procedural_semantics_v1".to_string(),
    }
}

fn route_requirement(parsed: &mut ParsedUtcrCorpus, bucket: &str, req: ProceduralRequirement) {
    match bucket {
        "formatting" => parsed.formatting_requirements.push(req),
        "caption" => parsed.caption_requirements.push(req),
        "signature" => parsed.signature_requirements.push(req),
        "certificate" => parsed.certificate_requirements.push(req),
        "service" => parsed.service_requirements.push(req),
        "efiling" => parsed.efiling_requirements.push(req),
        "protected_info" => parsed.protected_information_rules.push(req),
        "sanction" => parsed.sanction_rules.push(req),
        "deadline" => parsed.deadline_rules.push(req),
        "exception" => parsed.exception_rules.push(req),
        "filing" => parsed.filing_requirements.push(req),
        "exhibit" => parsed.exhibit_requirements.push(req),
        _ => parsed.procedural_rules.push(req),
    }
}

fn requirement_bucket(semantic_type: &str) -> &'static str {
    match semantic_type {
        "FormattingRequirement" => "formatting",
        "CaptionRequirement" => "caption",
        "SignatureRequirement" => "signature",
        "CertificateOfServiceRequirement" => "certificate",
        "ExhibitRequirement" => "exhibit",
        "ServiceRequirement" => "service",
        "EfilingRequirement" => "efiling",
        "ProtectedInformationRequirement" => "protected_info",
        "SanctionRule" => "sanction",
        "DeadlineRule" => "deadline",
        "ExceptionRule" => "exception",
        _ => "procedural",
    }
}

fn label_for_requirement(requirement_type: &str) -> String {
    requirement_type
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_source_provision<'a>(provisions: &'a [Provision], citation: &str) -> Option<&'a Provision> {
    provisions
        .iter()
        .find(|p| p.display_citation == citation || p.citation == citation)
        .or_else(|| {
            provisions.iter().find(|p| {
                citation.starts_with(&p.citation) && p.display_citation.starts_with(citation)
            })
        })
        .or_else(|| {
            let base = citation
                .split('(')
                .next()
                .unwrap_or(citation)
                .trim()
                .to_string();
            provisions.iter().find(|p| p.citation == base)
        })
}

fn dedupe_requirements(rows: &mut Vec<ProceduralRequirement>) {
    let mut seen = HashSet::new();
    rows.retain(|row| seen.insert(row.requirement_id.clone()));
}

fn build_rule_packs(parsed: &mut ParsedUtcrCorpus, config: &UtcrParseConfig) {
    let packs = vec![
        (
            "or:utcr:2025:oregon_circuit_general_document",
            "Oregon Circuit Court General Document Rules",
            vec!["court_document"],
        ),
        (
            "or:utcr:2025:oregon_circuit_civil_complaint",
            "Oregon Circuit Civil Complaint Rule Pack",
            vec!["complaint"],
        ),
        (
            "or:utcr:2025:oregon_circuit_civil_motion",
            "Oregon Circuit Civil Motion Rule Pack",
            vec!["motion"],
        ),
        (
            "or:utcr:2025:oregon_circuit_answer",
            "Oregon Circuit Answer Rule Pack",
            vec!["answer"],
        ),
        (
            "or:utcr:2025:oregon_circuit_declaration",
            "Oregon Circuit Declaration Rule Pack",
            vec!["declaration"],
        ),
        (
            "or:utcr:2025:oregon_circuit_filing_packet",
            "Oregon Circuit Filing Packet Rule Pack",
            vec!["filing_packet"],
        ),
    ];
    parsed.work_product_rule_packs = packs
        .into_iter()
        .map(|(id, name, work_product_types)| WorkProductRulePack {
            rule_pack_id: id.to_string(),
            name: name.to_string(),
            jurisdiction: "Oregon".to_string(),
            court_system: "circuit_court".to_string(),
            effective_date: config.effective_date.clone(),
            source_corpus_id: CORPUS_ID.to_string(),
            source_edition_id: EDITION_ID.to_string(),
            work_product_types: work_product_types.into_iter().map(ToString::to_string).collect(),
            inherits: if id.ends_with("general_document") {
                Vec::new()
            } else {
                vec!["or:utcr:2025:oregon_circuit_general_document".to_string()]
            },
            description: Some("Source-backed Oregon circuit court procedural rule pack derived from the 2025 UTCR.".to_string()),
        })
        .collect();

    let mut profile_properties = BTreeMap::new();
    profile_properties.insert("line_spacing".to_string(), "double".to_string());
    profile_properties.insert("numbered_lines".to_string(), "true".to_string());
    profile_properties.insert("first_page_top_blank_inches".to_string(), "2".to_string());
    profile_properties.insert("side_margin_inches".to_string(), "1".to_string());
    profile_properties.insert("source".to_string(), "UTCR 2.010".to_string());
    parsed.formatting_profiles = vec![FormattingProfile {
        formatting_profile_id: "or:utcr:2025:oregon_circuit_court_paper".to_string(),
        name: "Oregon Circuit Court Paper Formatting Profile".to_string(),
        source_corpus_id: CORPUS_ID.to_string(),
        source_edition_id: EDITION_ID.to_string(),
        effective_date: config.effective_date.clone(),
        properties: profile_properties,
    }];

    let all_requirements = all_requirements(parsed);
    for pack in &parsed.work_product_rule_packs {
        for req in &all_requirements {
            if pack_matches_requirement(pack, req) {
                parsed.rule_pack_memberships.push(RulePackMembership {
                    membership_id: format!(
                        "rule_pack_membership:{}",
                        stable_id(&format!("{}::{}", pack.rule_pack_id, req.requirement_id))
                    ),
                    rule_pack_id: pack.rule_pack_id.clone(),
                    requirement_id: req.requirement_id.clone(),
                    requirement_type: req.requirement_type.clone(),
                    source_provision_id: req.source_provision_id.clone(),
                    source_citation: req.source_citation.clone(),
                    applies_to: req.applies_to.clone(),
                    severity_default: req.severity_default.clone(),
                });
            }
        }
    }
}

fn all_requirements(parsed: &ParsedUtcrCorpus) -> Vec<ProceduralRequirement> {
    [
        parsed.procedural_rules.clone(),
        parsed.formatting_requirements.clone(),
        parsed.filing_requirements.clone(),
        parsed.service_requirements.clone(),
        parsed.efiling_requirements.clone(),
        parsed.caption_requirements.clone(),
        parsed.signature_requirements.clone(),
        parsed.certificate_requirements.clone(),
        parsed.exhibit_requirements.clone(),
        parsed.protected_information_rules.clone(),
        parsed.sanction_rules.clone(),
        parsed.deadline_rules.clone(),
        parsed.exception_rules.clone(),
    ]
    .concat()
}

fn pack_matches_requirement(pack: &WorkProductRulePack, req: &ProceduralRequirement) -> bool {
    if pack.rule_pack_id.ends_with("general_document") {
        return matches!(
            req.semantic_type.as_str(),
            "FormattingRequirement"
                | "CaptionRequirement"
                | "SignatureRequirement"
                | "ProceduralRule"
        );
    }
    if pack.rule_pack_id.ends_with("filing_packet") {
        return matches!(
            req.semantic_type.as_str(),
            "CertificateOfServiceRequirement"
                | "ServiceRequirement"
                | "EfilingRequirement"
                | "ProtectedInformationRequirement"
                | "DeadlineRule"
        );
    }
    pack.work_product_types.iter().any(|typ| {
        req.applies_to
            .iter()
            .any(|applies| applies == typ || applies == "court_document")
    })
}

fn build_retrieval_chunks(parsed: &mut ParsedUtcrCorpus, config: &UtcrParseConfig) {
    for version in &parsed.versions {
        parsed.retrieval_chunks.push(version_chunk(version, config));
    }
    for provision in &parsed.provisions {
        parsed
            .retrieval_chunks
            .push(provision_chunk(provision, "contextual_provision", config));
        if provision_has_citation(&provision.text) {
            parsed
                .retrieval_chunks
                .push(provision_chunk(provision, "citation_context", config));
        }
    }
    for req in all_requirements(parsed) {
        parsed
            .retrieval_chunks
            .push(requirement_chunk(&req, config));
    }
    for pack in &parsed.work_product_rule_packs {
        parsed.retrieval_chunks.push(rule_pack_chunk(pack, config));
    }
}

fn version_chunk(version: &LegalTextVersion, config: &UtcrParseConfig) -> RetrievalChunk {
    let text = format!(
        "Oregon Uniform Trial Court Rules. {} Edition.\nRule: {} - {}\nEffective: {}\nChapter: {}\nSource pages: {}-{}\n\n{}",
        config.edition_year,
        version.citation,
        version.title.clone().unwrap_or_default(),
        config.effective_date,
        version.chapter,
        version.source_page_start.unwrap_or_default(),
        version.source_page_end.unwrap_or_default(),
        version.text
    );
    RetrievalChunk {
        chunk_id: format!("chunk:{}:full_rule:v1", safe_id(&version.version_id)),
        chunk_type: "full_rule".to_string(),
        text: text.clone(),
        breadcrumb: format!(
            "Oregon > UTCR > Chapter {} > {}",
            version.chapter, version.citation
        ),
        source_provision_id: None,
        source_version_id: Some(version.version_id.clone()),
        parent_version_id: version.version_id.clone(),
        canonical_id: version.canonical_id.clone(),
        citation: version.citation.clone(),
        jurisdiction_id: JURISDICTION_ID.to_string(),
        authority_level: AUTHORITY_LEVEL,
        authority_family: Some(AUTHORITY_FAMILY.to_string()),
        corpus_id: Some(CORPUS_ID.to_string()),
        authority_type: Some(AUTHORITY_TYPE.to_string()),
        effective_date: Some(config.effective_date.clone()),
        chapter: Some(version.chapter.clone()),
        source_page_start: version.source_page_start,
        source_page_end: version.source_page_end,
        edition_year: config.edition_year,
        embedding_model: None,
        embedding_dim: None,
        embedding: None,
        embedding_input_hash: sha256_hex(text.as_bytes()),
        embedding_policy: Some("embed_primary".to_string()),
        answer_policy: Some("authoritative_support".to_string()),
        chunk_schema_version: Some("1.0.0".to_string()),
        retrieval_profile: Some("legal_rule_chunk_primary_v1".to_string()),
        search_weight: Some(1.1),
        embedding_input_type: Some("document".to_string()),
        embedding_output_dtype: Some("float".to_string()),
        embedded_at: None,
        source_kind: Some("LegalTextVersion".to_string()),
        source_id: Some(version.version_id.clone()),
        token_count: Some(estimate_tokens(&text)),
        max_tokens: Some(28_000),
        context_window: Some(32_000),
        chunking_strategy: Some("utcr_rule_adaptive_v1".to_string()),
        chunk_version: Some("1.0.0".to_string()),
        overlap_tokens: Some(0),
        split_reason: Some("none".to_string()),
        part_index: Some(1),
        part_count: Some(1),
        is_definition_candidate: false,
        is_exception_candidate: false,
        is_penalty_candidate: false,
        heading_path: Vec::new(),
        structural_context: None,
        embedding_profile: Some("legal_rule_chunk_primary_v1".to_string()),
        embedding_source_dimension: None,
    }
}

fn provision_chunk(
    provision: &Provision,
    chunk_type: &str,
    config: &UtcrParseConfig,
) -> RetrievalChunk {
    let header = format!(
        "Oregon Uniform Trial Court Rules. {} Edition.\nRule provision: {}\nEffective: {}\nChapter: {}\nSource pages: {}-{}\n\n",
        config.edition_year,
        provision.display_citation,
        config.effective_date,
        provision.chapter.clone().unwrap_or_default(),
        provision.source_page_start.unwrap_or_default(),
        provision.source_page_end.unwrap_or_default()
    );
    let text = format!("{header}{}", provision.text);
    RetrievalChunk {
        chunk_id: format!(
            "chunk:{}:{}:v1",
            safe_id(&provision.provision_id),
            chunk_type
        ),
        chunk_type: chunk_type.to_string(),
        text: text.clone(),
        breadcrumb: format!(
            "Oregon > UTCR > Chapter {} > {}",
            provision.chapter.clone().unwrap_or_default(),
            provision.display_citation
        ),
        source_provision_id: Some(provision.provision_id.clone()),
        source_version_id: None,
        parent_version_id: provision.version_id.clone(),
        canonical_id: provision.canonical_id.clone(),
        citation: provision.citation.clone(),
        jurisdiction_id: JURISDICTION_ID.to_string(),
        authority_level: AUTHORITY_LEVEL,
        authority_family: Some(AUTHORITY_FAMILY.to_string()),
        corpus_id: Some(CORPUS_ID.to_string()),
        authority_type: Some(AUTHORITY_TYPE.to_string()),
        effective_date: Some(config.effective_date.clone()),
        chapter: provision.chapter.clone(),
        source_page_start: provision.source_page_start,
        source_page_end: provision.source_page_end,
        edition_year: config.edition_year,
        embedding_model: None,
        embedding_dim: None,
        embedding: None,
        embedding_input_hash: sha256_hex(text.as_bytes()),
        embedding_policy: Some("embed_primary".to_string()),
        answer_policy: Some("authoritative_support".to_string()),
        chunk_schema_version: Some("1.0.0".to_string()),
        retrieval_profile: Some(format!("legal_rule_{chunk_type}_v1")),
        search_weight: Some(if chunk_type == "citation_context" {
            0.9
        } else {
            1.0
        }),
        embedding_input_type: Some("document".to_string()),
        embedding_output_dtype: Some("float".to_string()),
        embedded_at: None,
        source_kind: Some("Provision".to_string()),
        source_id: Some(provision.provision_id.clone()),
        token_count: Some(estimate_tokens(&text)),
        max_tokens: Some(26_000),
        context_window: Some(32_000),
        chunking_strategy: Some("utcr_rule_adaptive_v1".to_string()),
        chunk_version: Some("1.0.0".to_string()),
        overlap_tokens: Some(0),
        split_reason: Some("none".to_string()),
        part_index: Some(1),
        part_count: Some(1),
        is_definition_candidate: provision.is_definition_candidate,
        is_exception_candidate: provision.is_exception_candidate,
        is_penalty_candidate: provision.is_penalty_candidate,
        heading_path: provision.heading_path.clone(),
        structural_context: provision.structural_context.clone(),
        embedding_profile: Some("legal_rule_chunk_primary_v1".to_string()),
        embedding_source_dimension: None,
    }
}

fn requirement_chunk(req: &ProceduralRequirement, config: &UtcrParseConfig) -> RetrievalChunk {
    let chunk_type = match req.semantic_type.as_str() {
        "FormattingRequirement" => "formatting_requirement",
        "FilingRequirement" => "filing_requirement",
        "ServiceRequirement" => "service_requirement",
        "EfilingRequirement" => "efiling_requirement",
        "CertificateOfServiceRequirement" => "certificate_requirement",
        "ExhibitRequirement" => "exhibit_requirement",
        "ProtectedInformationRequirement" => "protected_info_requirement",
        "SanctionRule" => "sanction_context",
        _ => "contextual_provision",
    };
    let text = format!(
        "Oregon Uniform Trial Court Rules. {} Edition.\nRequirement type: {}\nSource: {}\nEffective: {}\nApplies to: {}\nSeverity: {}\n\n{}",
        config.edition_year,
        req.requirement_type,
        req.source_citation,
        config.effective_date,
        req.applies_to.join(", "),
        req.severity_default.clone().unwrap_or_default(),
        req.text
    );
    RetrievalChunk {
        chunk_id: format!("chunk:{}:{chunk_type}:v1", safe_id(&req.requirement_id)),
        chunk_type: chunk_type.to_string(),
        text: text.clone(),
        breadcrumb: format!("Oregon > UTCR > Requirements > {}", req.source_citation),
        source_provision_id: Some(req.source_provision_id.clone()),
        source_version_id: None,
        parent_version_id: String::new(),
        canonical_id: req.source_provision_id.clone(),
        citation: req.source_citation.clone(),
        jurisdiction_id: JURISDICTION_ID.to_string(),
        authority_level: AUTHORITY_LEVEL,
        authority_family: Some(AUTHORITY_FAMILY.to_string()),
        corpus_id: Some(CORPUS_ID.to_string()),
        authority_type: Some(AUTHORITY_TYPE.to_string()),
        effective_date: Some(config.effective_date.clone()),
        chapter: chapter_from_citation(&req.source_citation),
        source_page_start: None,
        source_page_end: None,
        edition_year: config.edition_year,
        embedding_model: None,
        embedding_dim: None,
        embedding: None,
        embedding_input_hash: sha256_hex(text.as_bytes()),
        embedding_policy: Some("embed_special".to_string()),
        answer_policy: Some("authoritative_support".to_string()),
        chunk_schema_version: Some("1.0.0".to_string()),
        retrieval_profile: Some("legal_procedural_requirement_primary_v1".to_string()),
        search_weight: Some(1.25),
        embedding_input_type: Some("document".to_string()),
        embedding_output_dtype: Some("float".to_string()),
        embedded_at: None,
        source_kind: Some(req.semantic_type.clone()),
        source_id: Some(req.requirement_id.clone()),
        token_count: Some(estimate_tokens(&text)),
        max_tokens: Some(16_000),
        context_window: Some(32_000),
        chunking_strategy: Some("utcr_requirement_v1".to_string()),
        chunk_version: Some("1.0.0".to_string()),
        overlap_tokens: Some(0),
        split_reason: Some("none".to_string()),
        part_index: Some(1),
        part_count: Some(1),
        is_definition_candidate: false,
        is_exception_candidate: req.semantic_type == "ExceptionRule",
        is_penalty_candidate: req.semantic_type == "SanctionRule",
        heading_path: Vec::new(),
        structural_context: Some("UTCR procedural requirement".to_string()),
        embedding_profile: Some("legal_procedural_requirement_primary_v1".to_string()),
        embedding_source_dimension: None,
    }
}

fn rule_pack_chunk(pack: &WorkProductRulePack, config: &UtcrParseConfig) -> RetrievalChunk {
    let text = format!(
        "Oregon Uniform Trial Court Rules. {} Edition.\nRule pack: {}\nEffective: {}\nWork product types: {}\n\n{}",
        config.edition_year,
        pack.name,
        config.effective_date,
        pack.work_product_types.join(", "),
        pack.description.clone().unwrap_or_default()
    );
    RetrievalChunk {
        chunk_id: format!("chunk:{}:rule_pack_context:v1", safe_id(&pack.rule_pack_id)),
        chunk_type: "rule_pack_context".to_string(),
        text: text.clone(),
        breadcrumb: format!("Oregon > UTCR > Rule Packs > {}", pack.name),
        source_provision_id: None,
        source_version_id: None,
        parent_version_id: String::new(),
        canonical_id: pack.rule_pack_id.clone(),
        citation: pack.name.clone(),
        jurisdiction_id: JURISDICTION_ID.to_string(),
        authority_level: AUTHORITY_LEVEL,
        authority_family: Some(AUTHORITY_FAMILY.to_string()),
        corpus_id: Some(CORPUS_ID.to_string()),
        authority_type: Some("work_product_rule_pack".to_string()),
        effective_date: Some(config.effective_date.clone()),
        chapter: None,
        source_page_start: None,
        source_page_end: None,
        edition_year: config.edition_year,
        embedding_model: None,
        embedding_dim: None,
        embedding: None,
        embedding_input_hash: sha256_hex(text.as_bytes()),
        embedding_policy: Some("embed_special".to_string()),
        answer_policy: Some("supporting".to_string()),
        chunk_schema_version: Some("1.0.0".to_string()),
        retrieval_profile: Some("legal_rule_pack_primary_v1".to_string()),
        search_weight: Some(0.75),
        embedding_input_type: Some("document".to_string()),
        embedding_output_dtype: Some("float".to_string()),
        embedded_at: None,
        source_kind: Some("WorkProductRulePack".to_string()),
        source_id: Some(pack.rule_pack_id.clone()),
        token_count: Some(estimate_tokens(&text)),
        max_tokens: Some(8_000),
        context_window: Some(32_000),
        chunking_strategy: Some("utcr_rule_pack_v1".to_string()),
        chunk_version: Some("1.0.0".to_string()),
        overlap_tokens: Some(0),
        split_reason: Some("none".to_string()),
        part_index: Some(1),
        part_count: Some(1),
        is_definition_candidate: false,
        is_exception_candidate: false,
        is_penalty_candidate: false,
        heading_path: Vec::new(),
        structural_context: Some("UTCR WorkProduct rule pack".to_string()),
        embedding_profile: Some("legal_rule_pack_primary_v1".to_string()),
        embedding_source_dimension: None,
    }
}

fn validate_utcr_parse(
    parsed: &ParsedUtcrCorpus,
    config: &UtcrParseConfig,
) -> Vec<ParserDiagnostic> {
    let mut diagnostics = Vec::new();
    if parsed.source_pages.len() != 185 {
        diagnostics.push(diagnostic(
            "warning",
            "unexpected_page_count",
            &format!("Expected 185 pages, parsed {}", parsed.source_pages.len()),
            None,
            Some(SOURCE_DOCUMENT_ID.to_string()),
        ));
    }
    for required in ["UTCR 2.010", "UTCR 2.020", "UTCR 21.040", "UTCR 21.140"] {
        if !parsed.identities.iter().any(|row| row.citation == required) {
            diagnostics.push(diagnostic(
                "error",
                "required_rule_missing",
                &format!("Required UTCR rule missing: {required}"),
                None,
                None,
            ));
        }
    }
    if parsed.work_product_rule_packs.len() < 6 {
        diagnostics.push(diagnostic(
            "error",
            "rule_pack_missing",
            "Expected general, complaint, motion, answer, declaration, and filing packet rule packs",
            None,
            Some(EDITION_ID.to_string()),
        ));
    }
    let mut seen_provision_paths = HashSet::new();
    for provision in &parsed.provisions {
        if !seen_provision_paths
            .insert((provision.version_id.clone(), provision.local_path.clone()))
        {
            diagnostics.push(diagnostic(
                "error",
                "duplicate_provision_path",
                &format!(
                    "Duplicate provision path for {} {:?}",
                    provision.version_id, provision.local_path
                ),
                provision.source_page_start,
                Some(provision.provision_id.clone()),
            ));
        }
    }
    if parsed.citation_mentions.iter().all(|cm| {
        !cm.citation_type.starts_with("utcr") || cm.resolver_status != "resolved_internal"
    }) {
        diagnostics.push(diagnostic(
            "warning",
            "no_internal_utcr_citations",
            "No internal UTCR citations resolved",
            None,
            Some(config.effective_date.clone()),
        ));
    }
    diagnostics
}

fn diagnostic(
    severity: &str,
    diagnostic_type: &str,
    message: &str,
    source_page: Option<usize>,
    related_id: Option<String>,
) -> ParserDiagnostic {
    ParserDiagnostic {
        parser_diagnostic_id: format!(
            "parser_diagnostic:{}",
            stable_id(&format!(
                "{diagnostic_type}::{message}::{source_page:?}::{related_id:?}"
            ))
        ),
        source_document_id: SOURCE_DOCUMENT_ID.to_string(),
        chapter: "UTCR".to_string(),
        edition_year: 2025,
        severity: severity.to_string(),
        diagnostic_type: diagnostic_type.to_string(),
        message: message.to_string(),
        source_paragraph_order: source_page,
        related_id,
        parser_profile: PARSER_PROFILE.to_string(),
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn is_definition_candidate(text: &str) -> bool {
    text.to_ascii_lowercase().contains(" means ")
}

fn is_exception_candidate(text: &str) -> bool {
    contains_any(
        &text.to_ascii_lowercase(),
        &["except", "unless", "does not apply", "notwithstanding"],
    )
}

fn is_deadline_candidate(text: &str) -> bool {
    contains_any(
        &text.to_ascii_lowercase(),
        &[
            "within",
            "not later than",
            "no later than",
            "days after",
            "days before",
        ],
    )
}

fn is_penalty_candidate(text: &str) -> bool {
    contains_any(
        &text.to_ascii_lowercase(),
        &["sanction", "strike", "costs", "attorney fees", "penalty"],
    )
}

fn provision_has_citation(text: &str) -> bool {
    UTCR_CITATION_RE.is_match(text)
        || ORS_CITATION_RE.is_match(text)
        || ORCP_CITATION_RE.is_match(text)
        || SLR_CITATION_RE.is_match(text)
        || ORAP_CITATION_RE.is_match(text)
}

fn estimate_tokens(text: &str) -> usize {
    (text.split_whitespace().count() as f32 * 1.35).ceil() as usize
}

fn chapter_from_citation(citation: &str) -> Option<String> {
    Regex::new(r"(?i)^UTCR\s+([0-9]{1,2})\.")
        .unwrap()
        .captures(citation)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, ':' | '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rule_heading_without_toc_leaders() {
        assert_eq!(
            parse_rule_heading("2.010 FORM OF DOCUMENTS"),
            Some(("2.010".to_string(), "FORM OF DOCUMENTS".to_string()))
        );
        assert_eq!(
            parse_rule_heading("2.010 FORM OF DOCUMENTS........2.1"),
            None
        );
    }

    #[test]
    fn builds_nested_provision_paths() {
        assert_eq!(next_path(&[], "1"), vec!["1"]);
        assert_eq!(next_path(&["1".to_string()], "a"), vec!["1", "a"]);
        assert_eq!(
            next_path(&["1".to_string(), "a".to_string()], "i"),
            vec!["1", "a", "i"]
        );
    }

    #[test]
    fn formats_utcr_pin_citations() {
        assert_eq!(
            display_citation("2.010", &["4".to_string(), "a".to_string()]),
            "UTCR 2.010(4)(a)"
        );
    }
}
