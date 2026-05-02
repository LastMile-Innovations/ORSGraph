use crate::hash::{sha256_hex, stable_id};
use crate::models::{
    ChapterHeading, CitationMention, CorpusEdition, Court, CourtRuleChapter, ExternalLegalCitation,
    Jurisdiction, LegalCorpus, LegalTextIdentity, LegalTextVersion, ParserDiagnostic, Provision,
    RetrievalChunk, SourceDocument, SourcePage, SourceTocEntry,
};
use crate::text::normalize_ws;
use anyhow::{Context, Result};
use lopdf::Document;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeSet;
use std::path::Path;

const PARSER_PROFILE: &str = "local_rule_pdf_parser_v1";
const AUTHORITY_TYPE: &str = "court_rule";
const AUTHORITY_FAMILY: &str = "SLR";
const AUTHORITY_LEVEL: i32 = 75;

static CHAPTER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^CHAPTER\s+([0-9]+)$").unwrap());
static APPENDIX_HEADING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^APPENDIX\s+([A-Z])$").unwrap());
static RULE_HEADING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([0-9]{1,2}\.[0-9]{3})\s+(.+)$").unwrap());
static RULE_ID_ONLY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([0-9]{1,2}\.[0-9]{3})$").unwrap());
static RULE_ID_PREFIX_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[0-9]{1,2}$").unwrap());
static RULE_ID_SUFFIX_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\.[0-9]{3}$").unwrap());
static PROVISION_MARKER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\(([0-9]+|[A-Za-z]|[ivxlcdmIVXLCDM]+)\)\s*(.*)$").unwrap());
static HEADER_DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]+\s+February\s+1,\s+2026$").unwrap());
static JUDICIAL_DISTRICT_HEADER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^[0-9]{1,2}(?:st|nd|rd|th)\s+Judicial District$").unwrap());
static PDF_CONTROL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\u{0000}-\u{0008}\u{000B}\u{000C}\u{000E}-\u{001F}]").unwrap());
static LEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\.{4,}").unwrap());
static TOC_RULE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([0-9]{1,2}\.[0-9]{3})\s+(.+?)(?:\s+\.{4,}\s*|\s{2,})([0-9]+)?$").unwrap()
});
static SLR_CITATION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(?:SLR\s+)?([0-9]{1,2}\.[0-9]{3})(?:\([^)]+\))*\b").unwrap());
static UTCR_CITATION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bUTCR\s+([0-9]{1,2}\.[0-9]{3})(?:\([^)]+\))*\b").unwrap());
static ORS_CITATION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bORS\s+[0-9]{1,3}[A-Z]?(?:\.[0-9]{3,4})?(?:\([^)]+\))*").unwrap()
});
static ORCP_CITATION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bORCP\s+[0-9A-Z]+(?:\s+[A-Z0-9]+)*(?:\([^)]+\))*").unwrap());

#[derive(Debug, Clone)]
pub struct LocalRulePdfParseConfig {
    pub state_id: String,
    pub state_name: String,
    pub jurisdiction_id: String,
    pub jurisdiction_name: String,
    pub court_id: String,
    pub court_name: String,
    pub judicial_district: String,
    pub edition_year: i32,
    pub effective_date: String,
    pub source_url: String,
}

impl LocalRulePdfParseConfig {
    pub fn oregon(
        jurisdiction_id: String,
        jurisdiction_name: String,
        court_id: String,
        court_name: String,
        judicial_district: String,
        edition_year: i32,
        effective_date: String,
        source_url: String,
    ) -> Self {
        Self {
            state_id: "or:state".to_string(),
            state_name: "Oregon".to_string(),
            jurisdiction_id,
            jurisdiction_name,
            court_id,
            court_name,
            judicial_district,
            edition_year,
            effective_date,
            source_url,
        }
    }
}

#[derive(Debug, Default)]
pub struct ParsedLocalRuleCorpus {
    pub jurisdictions: Vec<Jurisdiction>,
    pub courts: Vec<Court>,
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
    pub citation_mentions: Vec<CitationMention>,
    pub external_legal_citations: Vec<ExternalLegalCitation>,
    pub retrieval_chunks: Vec<RetrievalChunk>,
    pub parser_diagnostics: Vec<ParserDiagnostic>,
}

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
    body_lines: Vec<String>,
}

pub fn parse_local_rule_pdf(
    path: &Path,
    config: LocalRulePdfParseConfig,
) -> Result<ParsedLocalRuleCorpus> {
    let corpus_id = format!("{}:slr", config.jurisdiction_id);
    let edition_id = format!("{}@{}", corpus_id, config.edition_year);
    let source_document_id = format!("{}:source:{}_pdf", corpus_id, config.edition_year);
    let raw_bytes = std::fs::read(path)
        .with_context(|| format!("failed to read local rule PDF {}", path.display()))?;
    let doc = Document::load(path)
        .with_context(|| format!("failed to load local rule PDF {}", path.display()))?;
    let mut page_numbers = doc.get_pages().keys().copied().collect::<Vec<_>>();
    page_numbers.sort_unstable();
    let mut source_pages = Vec::new();
    for page_number in page_numbers {
        let raw_text = doc
            .extract_text(&[page_number])
            .with_context(|| format!("failed to extract PDF text from page {page_number}"))?;
        let normalized_text = normalize_page_text(&raw_text);
        source_pages.push(SourcePage {
            source_page_id: format!("{source_document_id}:page:{page_number}"),
            source_document_id: source_document_id.clone(),
            page_number: page_number as usize,
            printed_label: extract_printed_label(&normalized_text),
            text: raw_text,
            normalized_text: normalized_text.clone(),
            text_hash: sha256_hex(normalized_text.as_bytes()),
        });
    }

    let normalized_full_text = source_pages
        .iter()
        .map(|page| page.normalized_text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let source_document = SourceDocument {
        source_document_id: source_document_id.clone(),
        source_provider: "oregon_judicial_department".to_string(),
        source_kind: "official_pdf".to_string(),
        url: config.source_url.clone(),
        chapter: "SLR".to_string(),
        corpus_id: Some(corpus_id.clone()),
        edition_id: Some(edition_id.clone()),
        authority_family: Some(AUTHORITY_FAMILY.to_string()),
        authority_type: Some(AUTHORITY_TYPE.to_string()),
        title: Some(format!(
            "{} Supplementary Local Court Rules {}",
            config.court_name, config.edition_year
        )),
        source_type: Some("official_pdf".to_string()),
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string()),
        page_count: Some(source_pages.len()),
        effective_date: Some(config.effective_date.clone()),
        copyright_status: Some("no_copyright_reproducible".to_string()),
        chapter_title: Some(format!(
            "{} Supplementary Local Court Rules",
            config.court_name
        )),
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

    let page_lines = page_lines(&source_pages);
    let body_start = body_start_index(&page_lines).unwrap_or(0);
    let toc_rule_ids = toc_rule_ids(&page_lines[..body_start.min(page_lines.len())]);
    let (chapters, rule_drafts) = parse_body_rules(&page_lines[body_start..], &toc_rule_ids);
    let judicial_district_id = judicial_district_id(&config.judicial_district);

    let local_rule_prefix = local_rule_citation_prefix(&config.jurisdiction_name);
    let mut parsed = ParsedLocalRuleCorpus {
        jurisdictions: vec![
            Jurisdiction {
                jurisdiction_id: "us".to_string(),
                name: "United States".to_string(),
                jurisdiction_type: "federal".to_string(),
                parent_jurisdiction_id: None,
                country: Some("US".to_string()),
            },
            Jurisdiction {
                jurisdiction_id: config.state_id.clone(),
                name: config.state_name.clone(),
                jurisdiction_type: "state".to_string(),
                parent_jurisdiction_id: Some("us".to_string()),
                country: Some("US".to_string()),
            },
            Jurisdiction {
                jurisdiction_id: config.jurisdiction_id.clone(),
                name: config.jurisdiction_name.clone(),
                jurisdiction_type: "county".to_string(),
                parent_jurisdiction_id: Some(config.state_id.clone()),
                country: Some("US".to_string()),
            },
            Jurisdiction {
                jurisdiction_id: judicial_district_id.clone(),
                name: config.judicial_district.clone(),
                jurisdiction_type: "judicial_district".to_string(),
                parent_jurisdiction_id: Some(config.state_id.clone()),
                country: Some("US".to_string()),
            },
        ],
        courts: vec![Court {
            court_id: config.court_id.clone(),
            name: config.court_name.clone(),
            court_type: "circuit_court".to_string(),
            jurisdiction_id: config.jurisdiction_id.clone(),
            county_jurisdiction_id: Some(config.jurisdiction_id.clone()),
            judicial_district_id: Some(judicial_district_id),
            judicial_district: Some(config.judicial_district.clone()),
        }],
        legal_corpora: vec![LegalCorpus {
            corpus_id: corpus_id.clone(),
            name: format!("{} Supplementary Local Court Rules", config.court_name),
            short_name: local_rule_prefix.clone(),
            authority_family: AUTHORITY_FAMILY.to_string(),
            authority_type: AUTHORITY_TYPE.to_string(),
            jurisdiction_id: config.jurisdiction_id.clone(),
        }],
        corpus_editions: vec![CorpusEdition {
            edition_id: edition_id.clone(),
            corpus_id: corpus_id.clone(),
            edition_year: config.edition_year,
            effective_date: Some(config.effective_date.clone()),
            source_label: Some(format!(
                "{} SLR {}",
                config.jurisdiction_name, config.edition_year
            )),
            current: Some(true),
        }],
        source_documents: vec![source_document],
        source_pages,
        source_toc_entries: Vec::new(),
        court_rule_chapters: Vec::new(),
        chapter_headings: Vec::new(),
        identities: Vec::new(),
        versions: Vec::new(),
        provisions: Vec::new(),
        citation_mentions: Vec::new(),
        external_legal_citations: Vec::new(),
        retrieval_chunks: Vec::new(),
        parser_diagnostics: Vec::new(),
    };

    for (order_index, (chapter, title, start_page, end_page)) in chapters.iter().enumerate() {
        let chapter_key = safe_id(chapter);
        let chapter_citation = chapter_citation(chapter, &local_rule_prefix);
        parsed.court_rule_chapters.push(CourtRuleChapter {
            chapter_id: format!("{edition_id}:chapter:{chapter_key}"),
            corpus_id: corpus_id.clone(),
            edition_id: edition_id.clone(),
            chapter: chapter.clone(),
            title: title.clone(),
            citation: chapter_citation.clone(),
            edition_year: config.edition_year,
            effective_date: config.effective_date.clone(),
            source_page_start: Some(*start_page),
            source_page_end: Some(*end_page),
        });
        parsed.chapter_headings.push(ChapterHeading {
            heading_id: format!("{edition_id}:chapter:{chapter_key}:heading"),
            chapter: chapter.clone(),
            text: title.clone(),
            order_index: order_index + 1,
        });
        parsed.source_toc_entries.push(SourceTocEntry {
            source_toc_entry_id: format!("toc:{}:chapter:{chapter_key}", safe_id(&edition_id)),
            source_document_id: source_document_id.clone(),
            citation: Some(chapter_citation),
            canonical_id: Some(format!("{corpus_id}:chapter:{chapter_key}")),
            title: title.clone(),
            chapter: Some(chapter.clone()),
            page_label: Some(start_page.to_string()),
            page_number: Some(*start_page),
            toc_order: order_index + 1,
            entry_type: "chapter".to_string(),
            confidence: 0.86,
        });
    }

    let mut provision_order = 0usize;
    for draft in &rule_drafts {
        push_rule(
            &mut parsed,
            draft,
            &config,
            &corpus_id,
            &edition_id,
            &source_document_id,
            &mut provision_order,
        );
    }
    build_chunks(&mut parsed, &config, &corpus_id);
    parsed.parser_diagnostics = validate_parse(&parsed, &source_document_id, config.edition_year);
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
    let lines = out
        .lines()
        .map(normalize_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    repair_split_rule_ids(lines)
        .into_iter()
        .filter(|line| !is_page_artifact_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_line(line: &str) -> String {
    let text = normalize_ws(line);
    let lower = text.to_ascii_lowercase();
    if (lower.ends_with("circuit court") && !lower.starts_with("in the "))
        || JUDICIAL_DISTRICT_HEADER_RE.is_match(&text)
        || HEADER_DATE_RE.is_match(&text)
    {
        String::new()
    } else {
        text
    }
}

fn repair_split_rule_ids(lines: Vec<String>) -> Vec<String> {
    let mut repaired = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        if i + 1 < lines.len()
            && RULE_ID_PREFIX_RE.is_match(&lines[i])
            && RULE_ID_SUFFIX_RE.is_match(&lines[i + 1])
        {
            repaired.push(format!("{}{}", lines[i], lines[i + 1]));
            i += 2;
        } else {
            repaired.push(lines[i].clone());
            i += 1;
        }
    }
    repaired
}

fn is_page_artifact_line(line: &str) -> bool {
    line == "-"
        || line == "rd"
        || line == "Judicial District"
        || line == "February 1, 20"
        || (line.chars().all(|ch| ch.is_ascii_digit()) && line.len() <= 2)
}

fn extract_printed_label(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        HEADER_DATE_RE.find(line).map(|m| {
            m.as_str()
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
        })
    })
}

fn body_start_index(lines: &[PageLine]) -> Option<usize> {
    lines.iter().enumerate().position(|(idx, line)| {
        CHAPTER_NUMBER_RE
            .captures(&line.text)
            .is_some_and(|caps| &caps[1] == "1")
            && lines.get(idx + 1).is_some_and(|next| {
                !next.text.eq("-") && !next.text.eq_ignore_ascii_case("GENERAL")
            })
    })
}

fn toc_rule_ids(lines: &[PageLine]) -> BTreeSet<String> {
    lines
        .iter()
        .filter_map(|line| {
            TOC_RULE_RE
                .captures(&line.text)
                .or_else(|| RULE_HEADING_RE.captures(&line.text))
                .or_else(|| RULE_ID_ONLY_RE.captures(&line.text))
                .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        })
        .collect()
}

fn parse_body_rules(
    lines: &[PageLine],
    toc_rule_ids: &BTreeSet<String>,
) -> (Vec<(String, String, usize, usize)>, Vec<RuleDraft>) {
    let mut chapters = Vec::<(String, String, usize, usize)>::new();
    let mut rules = Vec::<RuleDraft>::new();
    let mut current_chapter = String::new();
    let mut current_chapter_title = String::new();
    let mut current_rule: Option<RuleDraft> = None;
    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if let Some(caps) = APPENDIX_HEADING_RE.captures(&line.text) {
            if let Some(rule) = current_rule.take() {
                rules.push(rule);
            }
            if let Some(last) = chapters.last_mut() {
                last.3 = line.page_number.saturating_sub(1).max(last.2);
            }
            let appendix_letter = caps[1].to_ascii_uppercase();
            current_chapter = format!("Appendix {appendix_letter}");
            current_chapter_title = current_chapter.clone();
            chapters.push((
                current_chapter.clone(),
                current_chapter_title.clone(),
                line.page_number,
                line.page_number,
            ));
            current_rule = Some(RuleDraft {
                citation: current_chapter.clone(),
                title: appendix_title(lines, i + 1, &current_chapter),
                chapter: current_chapter.clone(),
                chapter_title: current_chapter_title.clone(),
                start_page: line.page_number,
                end_page: line.page_number,
                body_lines: Vec::new(),
            });
            i += 1;
            continue;
        }
        if let Some(caps) = CHAPTER_NUMBER_RE.captures(&line.text) {
            if let Some(rule) = current_rule.take() {
                rules.push(rule);
            }
            if let Some(last) = chapters.last_mut() {
                last.3 = line.page_number.saturating_sub(1).max(last.2);
            }
            current_chapter = caps[1].to_string();
            current_chapter_title = lines
                .get(i + 1)
                .map(|next| next.text.clone())
                .unwrap_or_default();
            chapters.push((
                current_chapter.clone(),
                current_chapter_title.clone(),
                line.page_number,
                line.page_number,
            ));
            i += 2;
            continue;
        }
        if let Some((citation, title, next_i)) = parse_rule_heading_at(lines, i, toc_rule_ids) {
            if let Some(rule) = current_rule.take() {
                rules.push(rule);
            }
            current_rule = Some(RuleDraft {
                citation,
                title,
                chapter: current_chapter.clone(),
                chapter_title: current_chapter_title.clone(),
                start_page: line.page_number,
                end_page: line.page_number,
                body_lines: Vec::new(),
            });
            i = next_i;
            continue;
        }
        if let Some(rule) = current_rule.as_mut() {
            rule.end_page = line.page_number;
            rule.body_lines.push(line.text.clone());
        }
        i += 1;
    }
    if let Some(rule) = current_rule {
        rules.push(rule);
    }
    if let (Some(last), Some(last_line)) = (chapters.last_mut(), lines.last()) {
        last.3 = last_line.page_number;
    }
    (chapters, rules)
}

fn parse_rule_heading_at(
    lines: &[PageLine],
    index: usize,
    toc_rule_ids: &BTreeSet<String>,
) -> Option<(String, String, usize)> {
    let line = lines.get(index)?;
    let (citation, mut title, mut next_i) = if let Some(caps) = RULE_HEADING_RE.captures(&line.text)
    {
        (
            caps[1].to_string(),
            strip_toc_leader(caps.get(2).map(|m| m.as_str()).unwrap_or("")),
            index + 1,
        )
    } else if let Some(caps) = RULE_ID_ONLY_RE.captures(&line.text) {
        (caps[1].to_string(), String::new(), index + 1)
    } else {
        return None;
    };

    if !toc_rule_ids.is_empty() && !toc_rule_ids.contains(&citation) {
        return None;
    }

    while let Some(next) = lines.get(next_i) {
        if CHAPTER_NUMBER_RE.is_match(&next.text)
            || APPENDIX_HEADING_RE.is_match(&next.text)
            || PROVISION_MARKER_RE.is_match(&next.text)
            || is_known_rule_heading_start(&next.text, toc_rule_ids)
            || !is_upper_heading_continuation(&next.text)
        {
            break;
        }
        if title.is_empty() {
            title = next.text.clone();
        } else {
            title = format!("{title} {}", next.text);
        }
        next_i += 1;
    }

    (!title.trim().is_empty()).then_some((citation, normalize_ws(&title), next_i))
}

fn is_known_rule_heading_start(text: &str, toc_rule_ids: &BTreeSet<String>) -> bool {
    let citation = RULE_HEADING_RE
        .captures(text)
        .or_else(|| RULE_ID_ONLY_RE.captures(text))
        .and_then(|caps| caps.get(1).map(|m| m.as_str()));
    citation.is_some_and(|rule| toc_rule_ids.is_empty() || toc_rule_ids.contains(rule))
}

fn strip_toc_leader(text: &str) -> String {
    LEADER_RE
        .replace(text, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn appendix_title(lines: &[PageLine], start: usize, fallback: &str) -> String {
    let mut title_parts = Vec::new();
    for line in lines.iter().skip(start).take(18) {
        if CHAPTER_NUMBER_RE.is_match(&line.text)
            || APPENDIX_HEADING_RE.is_match(&line.text)
            || RULE_HEADING_RE.is_match(&line.text)
        {
            break;
        }
        let upper = line.text.to_ascii_uppercase();
        if upper.contains("WAIVER") || upper.contains("DECLARATION") || upper.contains("PENALTY") {
            title_parts.push(line.text.clone());
        }
    }
    if title_parts.is_empty() {
        fallback.to_string()
    } else {
        normalize_ws(&title_parts.join(" "))
    }
}

fn is_upper_heading_continuation(text: &str) -> bool {
    let letters = text
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect::<Vec<_>>();
    !letters.is_empty()
        && letters.iter().all(|c| c.is_ascii_uppercase())
        && !text.ends_with('.')
        && text.len() <= 90
}

fn push_rule(
    parsed: &mut ParsedLocalRuleCorpus,
    draft: &RuleDraft,
    config: &LocalRulePdfParseConfig,
    corpus_id: &str,
    edition_id: &str,
    source_document_id: &str,
    provision_order: &mut usize,
) {
    let canonical_id = canonical_id_for_rule(corpus_id, &draft.citation);
    let version_id = format!("{canonical_id}@{}", config.edition_year);
    let display_citation = format!(
        "{} {}",
        local_rule_citation_prefix(&config.jurisdiction_name),
        draft.citation
    );
    let body_text = draft.body_lines.join("\n");
    let version_text = format!("{}\n{}", draft.title, body_text).trim().to_string();
    parsed.identities.push(LegalTextIdentity {
        canonical_id: canonical_id.clone(),
        citation: display_citation.clone(),
        jurisdiction_id: config.jurisdiction_id.clone(),
        authority_family: AUTHORITY_FAMILY.to_string(),
        corpus_id: Some(corpus_id.to_string()),
        authority_type: Some(AUTHORITY_TYPE.to_string()),
        authority_level: Some(AUTHORITY_LEVEL),
        effective_date: Some(config.effective_date.clone()),
        title: Some(draft.title.clone()),
        chapter: draft.chapter.clone(),
        status: "current".to_string(),
    });
    parsed.versions.push(LegalTextVersion {
        version_id: version_id.clone(),
        canonical_id: canonical_id.clone(),
        citation: display_citation.clone(),
        title: Some(draft.title.clone()),
        chapter: draft.chapter.clone(),
        corpus_id: Some(corpus_id.to_string()),
        edition_id: Some(edition_id.to_string()),
        authority_family: Some(AUTHORITY_FAMILY.to_string()),
        authority_type: Some(AUTHORITY_TYPE.to_string()),
        authority_level: Some(AUTHORITY_LEVEL),
        effective_date: Some(config.effective_date.clone()),
        source_page_start: Some(draft.start_page),
        source_page_end: Some(draft.end_page),
        edition_year: config.edition_year,
        status: "current".to_string(),
        status_text: None,
        text: version_text.clone(),
        text_hash: sha256_hex(version_text.as_bytes()),
        original_text: None,
        paragraph_start_order: Some(*provision_order + 1),
        paragraph_end_order: None,
        source_paragraph_ids: Vec::new(),
        source_document_id: source_document_id.to_string(),
        official_status: "official_pdf".to_string(),
        disclaimer_required: false,
        embedding_model: None,
        embedding_dim: None,
        embedding: None,
        embedding_input_hash: None,
        embedding_input_type: None,
        embedding_output_dtype: None,
        embedded_at: None,
        embedding_profile: None,
        embedding_strategy: Some("retrieval_chunks".to_string()),
        embedding_source_dimension: None,
    });
    push_provision(
        parsed,
        ProvisionDraft {
            provision_id: format!("{version_id}:rule"),
            version_id: version_id.clone(),
            canonical_id: canonical_id.clone(),
            citation: display_citation.clone(),
            display_citation: display_citation.clone(),
            chapter: draft.chapter.clone(),
            corpus_id: corpus_id.to_string(),
            edition_id: edition_id.to_string(),
            effective_date: config.effective_date.clone(),
            source_page_start: draft.start_page,
            source_page_end: draft.end_page,
            local_path: vec![draft.citation.clone()],
            provision_type: "rule".to_string(),
            text: version_text.clone(),
            order_index: {
                *provision_order += 1;
                *provision_order
            },
            depth: 0,
            heading_path: vec![draft.chapter_title.clone(), draft.title.clone()],
        },
    );

    for (marker, text, order_hint) in marker_provisions(&draft.body_lines) {
        push_provision(
            parsed,
            ProvisionDraft {
                provision_id: format!("{version_id}:{}", safe_id(&marker)),
                version_id: version_id.clone(),
                canonical_id: canonical_id.clone(),
                citation: format!("{}({marker})", display_citation),
                display_citation: format!("{}({marker})", display_citation),
                chapter: draft.chapter.clone(),
                corpus_id: corpus_id.to_string(),
                edition_id: edition_id.to_string(),
                effective_date: config.effective_date.clone(),
                source_page_start: draft.start_page,
                source_page_end: draft.end_page,
                local_path: vec![draft.citation.clone(), marker.clone()],
                provision_type: "paragraph".to_string(),
                text,
                order_index: {
                    *provision_order += 1;
                    *provision_order
                },
                depth: marker_depth(&marker),
                heading_path: vec![draft.chapter_title.clone(), draft.title.clone(), order_hint],
            },
        );
    }

    extract_citations(
        parsed,
        &version_id,
        &canonical_id,
        &display_citation,
        &version_text,
        corpus_id,
        config,
    );
    parsed.source_toc_entries.push(SourceTocEntry {
        source_toc_entry_id: format!(
            "toc:{}:rule:{}",
            safe_id(edition_id),
            safe_id(&draft.citation)
        ),
        source_document_id: source_document_id.to_string(),
        citation: Some(display_citation),
        canonical_id: Some(canonical_id),
        title: draft.title.clone(),
        chapter: Some(draft.chapter.clone()),
        page_label: Some(draft.start_page.to_string()),
        page_number: Some(draft.start_page),
        toc_order: parsed.source_toc_entries.len() + 1,
        entry_type: "rule".to_string(),
        confidence: 0.84,
    });
}

struct ProvisionDraft {
    provision_id: String,
    version_id: String,
    canonical_id: String,
    citation: String,
    display_citation: String,
    chapter: String,
    corpus_id: String,
    edition_id: String,
    effective_date: String,
    source_page_start: usize,
    source_page_end: usize,
    local_path: Vec<String>,
    provision_type: String,
    text: String,
    order_index: usize,
    depth: usize,
    heading_path: Vec<String>,
}

fn push_provision(parsed: &mut ParsedLocalRuleCorpus, draft: ProvisionDraft) {
    let normalized_text = normalize_ws(&draft.text);
    parsed.provisions.push(Provision {
        provision_id: draft.provision_id,
        version_id: draft.version_id,
        canonical_id: draft.canonical_id,
        citation: draft.citation,
        display_citation: draft.display_citation,
        chapter: Some(draft.chapter),
        corpus_id: Some(draft.corpus_id),
        edition_id: Some(draft.edition_id),
        authority_family: Some(AUTHORITY_FAMILY.to_string()),
        authority_type: Some(AUTHORITY_TYPE.to_string()),
        authority_level: Some(AUTHORITY_LEVEL),
        effective_date: Some(draft.effective_date),
        source_page_start: Some(draft.source_page_start),
        source_page_end: Some(draft.source_page_end),
        local_path: draft.local_path,
        provision_type: draft.provision_type,
        text: draft.text,
        original_text: None,
        normalized_text: normalized_text.clone(),
        order_index: draft.order_index,
        depth: draft.depth,
        text_hash: sha256_hex(normalized_text.as_bytes()),
        is_implied: false,
        is_definition_candidate: false,
        is_exception_candidate: normalized_text.to_ascii_lowercase().contains("except"),
        is_deadline_candidate: has_deadline_signal(&normalized_text),
        is_penalty_candidate: has_penalty_signal(&normalized_text),
        paragraph_start_order: Some(draft.order_index),
        paragraph_end_order: Some(draft.order_index),
        source_paragraph_ids: Vec::new(),
        heading_path: draft.heading_path,
        structural_context: Some("supplementary local rule".to_string()),
        embedding_model: None,
        embedding_dim: None,
        embedding: None,
        embedding_input_hash: None,
        embedding_input_type: None,
        embedding_output_dtype: None,
        embedded_at: None,
        embedding_profile: None,
        embedding_source_dimension: None,
    });
}

fn marker_provisions(lines: &[String]) -> Vec<(String, String, String)> {
    let mut rows = Vec::<(String, Vec<String>)>::new();
    for line in lines {
        if let Some(caps) = PROVISION_MARKER_RE.captures(line) {
            let marker = caps[1].to_string();
            let rest = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
            rows.push((marker, vec![rest]));
        } else if let Some((_, body)) = rows.last_mut() {
            body.push(line.clone());
        }
    }
    rows.into_iter()
        .map(|(marker, body)| {
            let label = format!("paragraph {marker}");
            (marker, normalize_ws(&body.join(" ")), label)
        })
        .collect()
}

fn marker_depth(marker: &str) -> usize {
    if marker.chars().all(|c| c.is_ascii_digit()) {
        1
    } else if marker.len() == 1 && marker.chars().all(|c| c.is_ascii_alphabetic()) {
        2
    } else {
        3
    }
}

fn extract_citations(
    parsed: &mut ParsedLocalRuleCorpus,
    version_id: &str,
    canonical_id: &str,
    display_citation: &str,
    text: &str,
    corpus_id: &str,
    config: &LocalRulePdfParseConfig,
) {
    let mut seen = BTreeSet::new();
    for (raw, normalized, citation_type, target) in citation_candidates(text, corpus_id, config) {
        if raw == display_citation || !seen.insert((raw.clone(), normalized.clone())) {
            continue;
        }
        let mention_id = format!(
            "citation_mention:{}",
            stable_id(&format!("{version_id}::{raw}::{normalized}"))
        );
        parsed.citation_mentions.push(CitationMention {
            citation_mention_id: mention_id.clone(),
            source_provision_id: format!("{version_id}:rule"),
            raw_text: raw.clone(),
            normalized_citation: normalized.clone(),
            citation_type: citation_type.clone(),
            target_canonical_id: target,
            target_start_canonical_id: None,
            target_end_canonical_id: None,
            target_provision_id: None,
            unresolved_subpath: None,
            external_citation_id: None,
            resolver_status: "candidate".to_string(),
            confidence: 0.78,
            qc_severity: None,
        });
        if citation_type == "external" {
            parsed.external_legal_citations.push(ExternalLegalCitation {
                external_citation_id: format!("external_citation:{}", stable_id(&normalized)),
                citation: raw,
                normalized_citation: normalized,
                citation_type: "external_rule".to_string(),
                jurisdiction_id: config.state_id.clone(),
                source_system: PARSER_PROFILE.to_string(),
                url: None,
            });
        }
    }
    if text.to_ascii_lowercase().contains("appendix a") {
        let normalized = format!(
            "{} Appendix A",
            local_rule_citation_prefix(&config.jurisdiction_name)
        );
        parsed.citation_mentions.push(CitationMention {
            citation_mention_id: format!(
                "citation_mention:{}",
                stable_id(&format!("{canonical_id}::appendix_a"))
            ),
            source_provision_id: format!("{version_id}:rule"),
            raw_text: "Appendix A".to_string(),
            normalized_citation: normalized,
            citation_type: "appendix".to_string(),
            target_canonical_id: Some(format!("{corpus_id}:appendix:a")),
            target_start_canonical_id: None,
            target_end_canonical_id: None,
            target_provision_id: None,
            unresolved_subpath: None,
            external_citation_id: None,
            resolver_status: "candidate".to_string(),
            confidence: 0.7,
            qc_severity: None,
        });
    }
}

fn citation_candidates(
    text: &str,
    corpus_id: &str,
    config: &LocalRulePdfParseConfig,
) -> Vec<(String, String, String, Option<String>)> {
    let mut rows = Vec::new();
    let mut protected_ranges = Vec::<(usize, usize)>::new();
    for caps in UTCR_CITATION_RE.captures_iter(text) {
        let raw = caps.get(0).unwrap().as_str().to_string();
        let rule = caps.get(1).unwrap().as_str();
        let span = caps.get(0).unwrap();
        protected_ranges.push((span.start(), span.end()));
        rows.push((
            raw,
            format!("UTCR {rule}"),
            "court_rule".to_string(),
            Some(format!("or:utcr:{rule}")),
        ));
    }
    for caps in ORS_CITATION_RE.captures_iter(text) {
        let raw = caps.get(0).unwrap().as_str().to_string();
        let span = caps.get(0).unwrap();
        protected_ranges.push((span.start(), span.end()));
        rows.push((
            raw.clone(),
            raw.to_ascii_uppercase(),
            "external".to_string(),
            None,
        ));
    }
    for caps in ORCP_CITATION_RE.captures_iter(text) {
        let raw = caps.get(0).unwrap().as_str().to_string();
        let span = caps.get(0).unwrap();
        protected_ranges.push((span.start(), span.end()));
        rows.push((
            raw.clone(),
            raw.to_ascii_uppercase(),
            "external".to_string(),
            None,
        ));
    }
    for caps in SLR_CITATION_RE.captures_iter(text) {
        let span = caps.get(0).unwrap();
        if protected_ranges
            .iter()
            .any(|(start, end)| span.start() < *end && span.end() > *start)
        {
            continue;
        }
        let raw = span.as_str().to_string();
        let rule = caps.get(1).unwrap().as_str();
        rows.push((
            raw,
            format!(
                "{} {rule}",
                local_rule_citation_prefix(&config.jurisdiction_name)
            ),
            "local_court_rule".to_string(),
            Some(format!("{corpus_id}:{rule}")),
        ));
    }
    rows
}

fn build_chunks(
    parsed: &mut ParsedLocalRuleCorpus,
    config: &LocalRulePdfParseConfig,
    corpus_id: &str,
) {
    for version in &parsed.versions {
        let text = format!(
            "{}. {} Edition.\nRule: {}\nEffective: {}\nChapter: {}\n\n{}",
            parsed
                .legal_corpora
                .first()
                .map(|c| c.name.as_str())
                .unwrap_or("Supplementary Local Court Rules"),
            config.edition_year,
            version.citation,
            config.effective_date,
            version.chapter,
            version.text
        );
        parsed.retrieval_chunks.push(RetrievalChunk {
            chunk_id: format!(
                "chunk:{}",
                stable_id(&format!("{}::version", version.version_id))
            ),
            chunk_type: "full_rule".to_string(),
            text: text.clone(),
            breadcrumb: format!(
                "{} > {} > SLR {} > {}",
                config.state_name, config.jurisdiction_name, config.edition_year, version.citation
            ),
            source_provision_id: None,
            source_version_id: Some(version.version_id.clone()),
            parent_version_id: version.version_id.clone(),
            canonical_id: version.canonical_id.clone(),
            citation: version.citation.clone(),
            jurisdiction_id: config.jurisdiction_id.clone(),
            authority_level: AUTHORITY_LEVEL,
            authority_family: Some(AUTHORITY_FAMILY.to_string()),
            corpus_id: Some(corpus_id.to_string()),
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
            answer_policy: Some("answerable".to_string()),
            chunk_schema_version: Some("local_rule_chunk_v1".to_string()),
            retrieval_profile: Some("legal_rule_primary".to_string()),
            search_weight: Some(1.1),
            embedding_input_type: Some("passage".to_string()),
            embedding_output_dtype: Some("float".to_string()),
            embedded_at: None,
            source_kind: Some("LegalTextVersion".to_string()),
            source_id: Some(version.version_id.clone()),
            token_count: Some(estimate_tokens(&text)),
            max_tokens: Some(1200),
            context_window: Some(6000),
            chunking_strategy: Some("rule".to_string()),
            chunk_version: Some("1.0.0".to_string()),
            overlap_tokens: Some(0),
            split_reason: None,
            part_index: Some(1),
            part_count: Some(1),
            is_definition_candidate: false,
            is_exception_candidate: text.to_ascii_lowercase().contains("except"),
            is_penalty_candidate: has_penalty_signal(&text),
            heading_path: vec![
                version.chapter.clone(),
                version.title.clone().unwrap_or_default(),
            ],
            structural_context: Some(format!("{} SLR rule", config.jurisdiction_name)),
            embedding_profile: Some("legal_rule_primary_v1".to_string()),
            embedding_source_dimension: None,
        });
    }
}

fn validate_parse(
    parsed: &ParsedLocalRuleCorpus,
    source_document_id: &str,
    edition_year: i32,
) -> Vec<ParserDiagnostic> {
    let mut diagnostics = Vec::new();
    if parsed.versions.is_empty() {
        diagnostics.push(diagnostic(
            source_document_id,
            edition_year,
            "error",
            "rules_missing",
            "No supplementary local rules were parsed from the PDF.",
            None,
        ));
    } else if parsed.versions.len() < 10 {
        diagnostics.push(diagnostic(
            source_document_id,
            edition_year,
            "warning",
            "low_rule_count",
            &format!(
                "Parsed {} SLR rules; verify that this local edition has a short rule set.",
                parsed.versions.len()
            ),
            None,
        ));
    }
    diagnostics
}

fn diagnostic(
    source_document_id: &str,
    edition_year: i32,
    severity: &str,
    diagnostic_type: &str,
    message: &str,
    related_id: Option<String>,
) -> ParserDiagnostic {
    ParserDiagnostic {
        parser_diagnostic_id: format!(
            "parser_diagnostic:{}",
            stable_id(&format!(
                "{source_document_id}::{diagnostic_type}::{message}"
            ))
        ),
        source_document_id: source_document_id.to_string(),
        chapter: "SLR".to_string(),
        edition_year,
        severity: severity.to_string(),
        diagnostic_type: diagnostic_type.to_string(),
        message: message.to_string(),
        source_paragraph_order: None,
        related_id,
        parser_profile: PARSER_PROFILE.to_string(),
    }
}

fn has_deadline_signal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("days")
        || lower.contains("one week")
        || lower.contains("prior to")
        || lower.contains("no later than")
}

fn has_penalty_signal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("may result")
        || lower.contains("shall be dismissed")
        || lower.contains("default")
        || lower.contains("sanction")
}

fn canonical_id_for_rule(corpus_id: &str, citation: &str) -> String {
    if let Some(letter) = citation
        .strip_prefix("Appendix ")
        .and_then(|suffix| suffix.chars().next())
    {
        format!("{corpus_id}:appendix:{}", letter.to_ascii_lowercase())
    } else {
        format!("{corpus_id}:{citation}")
    }
}

fn judicial_district_id(name: &str) -> String {
    let number = name
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if number.is_empty() {
        format!("or:judicial_district:{}", safe_id(name))
    } else {
        format!("or:judicial_district:{number}")
    }
}

fn chapter_citation(chapter: &str, local_rule_prefix: &str) -> String {
    if chapter.starts_with("Appendix ") {
        format!("{local_rule_prefix} {chapter}")
    } else {
        format!("SLR Chapter {chapter}")
    }
}

fn local_rule_citation_prefix(jurisdiction_name: &str) -> String {
    let short_name = jurisdiction_name
        .trim()
        .strip_suffix(" County")
        .unwrap_or_else(|| jurisdiction_name.trim());
    format!("{short_name} SLR")
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn estimate_tokens(text: &str) -> usize {
    ((text.split_whitespace().count() as f32) * 1.35).ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_parser_uses_toc_to_avoid_false_rule_headings() {
        let lines = vec![
            PageLine {
                page_number: 1,
                text: "CHAPTER 13".to_string(),
            },
            PageLine {
                page_number: 1,
                text: "ARBITRATION".to_string(),
            },
            PageLine {
                page_number: 1,
                text: "13.095 ARBITRATION PANEL".to_string(),
            },
            PageLine {
                page_number: 1,
                text: "(a) Civil Panel: an attorney meeting the requirements set forth in UTCR"
                    .to_string(),
            },
            PageLine {
                page_number: 1,
                text: "13.090 with five years continuous practice including significant experience"
                    .to_string(),
            },
            PageLine {
                page_number: 2,
                text: "13.125 SUSPENSION OF PROCEEDINGS PENDING PAYMENT OF DEPOSIT".to_string(),
            },
            PageLine {
                page_number: 2,
                text: "The arbitrator may decline to begin a hearing.".to_string(),
            },
        ];
        let toc = ["13.095".to_string(), "13.125".to_string()]
            .into_iter()
            .collect();
        let (_, rules) = parse_body_rules(&lines, &toc);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].citation, "13.095");
        assert!(
            rules[0]
                .body_lines
                .iter()
                .any(|line| line.starts_with("13.090"))
        );
    }

    #[test]
    fn appendix_stops_previous_rule_and_becomes_own_unit() {
        let lines = vec![
            PageLine {
                page_number: 24,
                text: "CHAPTER 16".to_string(),
            },
            PageLine {
                page_number: 24,
                text: "VIOLATIONS".to_string(),
            },
            PageLine {
                page_number: 24,
                text: "16.005 TRAFFIC AND OTHER VIOLATION OFFENSES".to_string(),
            },
            PageLine {
                page_number: 24,
                text: "As authorized by ORS 153.080, Appendix A may be filed.".to_string(),
            },
            PageLine {
                page_number: 25,
                text: "Appendix A".to_string(),
            },
            PageLine {
                page_number: 25,
                text: "WAIVER AND DECLARATION UNDER".to_string(),
            },
            PageLine {
                page_number: 25,
                text: "PENALTY OF PERJURY".to_string(),
            },
        ];
        let toc = ["16.005".to_string()].into_iter().collect();
        let (_, rules) = parse_body_rules(&lines, &toc);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].citation, "16.005");
        assert_eq!(rules[0].end_page, 24);
        assert_eq!(rules[1].citation, "Appendix A");
        assert!(rules[1].title.contains("WAIVER AND DECLARATION"));
    }

    #[test]
    fn body_start_skips_table_of_contents_chapter_one() {
        let lines = vec![
            PageLine {
                page_number: 2,
                text: "CHAPTER 1".to_string(),
            },
            PageLine {
                page_number: 2,
                text: "GENERAL".to_string(),
            },
            PageLine {
                page_number: 2,
                text: "PROVISIONS".to_string(),
            },
            PageLine {
                page_number: 5,
                text: "CHAPTER 1".to_string(),
            },
            PageLine {
                page_number: 5,
                text: "LOCATION AND HOURS OF COURT OPERATION".to_string(),
            },
        ];
        assert_eq!(body_start_index(&lines), Some(3));
    }

    #[test]
    fn repair_split_rule_ids_joins_fragmented_rule_numbers() {
        let repaired = repair_split_rule_ids(vec![
            "1".to_string(),
            ".171".to_string(),
            "WEBSITE ADDRESS".to_string(),
        ]);
        assert_eq!(repaired, vec!["1.171", "WEBSITE ADDRESS"]);
    }

    #[test]
    fn citation_candidates_do_not_double_count_utcr_rule_numbers_as_local_rules() {
        let config = LocalRulePdfParseConfig::oregon(
            "or:linn".to_string(),
            "Linn County".to_string(),
            "or:linn:circuit_court".to_string(),
            "Linn County Circuit Court".to_string(),
            "23rd Judicial District".to_string(),
            2026,
            "2026-02-01".to_string(),
            "https://example.test/linn.pdf".to_string(),
        );
        let rows = citation_candidates(
            "Civil Panel requirements are set forth in UTCR\n13.090 and SLR 13.095.",
            "or:linn:slr",
            &config,
        );

        assert!(
            rows.iter()
                .any(|(_, normalized, _, target)| normalized == "UTCR 13.090"
                    && target.as_deref() == Some("or:utcr:13.090"))
        );
        assert!(
            rows.iter()
                .any(|(_, normalized, _, target)| normalized == "Linn SLR 13.095"
                    && target.as_deref() == Some("or:linn:slr:13.095"))
        );
        assert!(
            !rows
                .iter()
                .any(|(_, normalized, _, target)| normalized == "Linn SLR 13.090"
                    || target.as_deref() == Some("or:linn:slr:13.090"))
        );
    }
}
