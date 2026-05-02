use crate::artifact_store::{ArtifactMetadata, RawArtifact};
use crate::authority_taxonomy::{
    AUTHORITY_LEVEL_OFFICIAL_COMMENTARY, AUTHORITY_LEVEL_STATE_CONSTITUTION,
};
use crate::connectors::{ConnectorOptions, DataConnector, SourceItem};
use crate::graph_batch::GraphBatch;
use crate::hash::{sha256_hex, stable_id};
use crate::models::{
    Amendment, CitationMention, Commentary, CorpusEdition, ExternalLegalCitation, Jurisdiction,
    LegalCorpus, LegalTextIdentity, LegalTextVersion, LineageEvent, ParserDiagnostic, Provision,
    RetrievalChunk, SessionLaw, SourceDocument, SourceNote, SourcePage, SourceTocEntry,
    StatusEvent, TemporalEffect,
};
use crate::source_qc::{QcReport, qc_source_batch};
use crate::source_registry::{SourceKind, SourceRegistryEntry};
use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::{BTreeMap, BTreeSet};

const SOURCE_ID: &str = "or_leg_constitution";
const CONSTITUTION_URL: &str = "https://www.oregonlegislature.gov/bills_laws/ors/orcons.html";
const PREAMBLE_URL: &str = "https://www.oregonlegislature.gov/bills_laws/ors/ocapream.html";
const ANNOTATIONS_INDEX_URL: &str =
    "https://www.oregonlegislature.gov/bills_laws/Pages/Annotations.aspx";
const OR_CONSTITUTION_CORPUS_ID: &str = "or:constitution";
const OR_CONSTITUTION_AUTHORITY_FAMILY: &str = "ORCONST";
const OR_CONSTITUTION_AUTHORITY_TYPE: &str = "constitution";
const OR_CONSTITUTION_EDITION_SOURCE: &str = "Oregon Legislature Constitution web edition";
const OR_JURISDICTION_ID: &str = "or:state";
const US_JURISDICTION_ID: &str = "us";
const PARSER_PROFILE: &str = "oregon_constitution_html_v1";
const ANNOTATION_PARSER_PROFILE: &str = "oregon_constitution_annotations_html_v1";
const SECTION_SYMBOL: char = '\u{00a7}';

static ANNOTATION_LINK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)\banc(0[0-9]{2})\.html\b"#).expect("valid annotation link"));
static ARTICLE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^ARTICLE\s+([IVXLCDM]+(?:-[A-Z]+(?:\([0-9]+\))?)?)(?:\s+\((Amended|Original)\))?$",
    )
    .expect("valid article regex")
});
static SECTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^Section\s+([0-9]+[a-z]?)\.\s+([^\.]+)\.\s*(.*)$")
        .expect("valid section regex")
});
static SUBSECTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\(([0-9A-Za-z]+)\)\s+(.+)$").expect("valid subsection regex"));
static BRACKETED_SOURCE_NOTE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s*\[([^\]]+)\]\s*$").expect("valid source note regex"));
static NOTE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^Note:\s*(.+)$").expect("valid note regex"));
static CREATED_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\bCreated through ([^;\.]+?)(?: and adopted by the people ([^;\.]+))?(?:;|\.|$)",
    )
    .expect("valid created regex")
});
static AMENDMENT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bAmendment proposed by ([A-Z]\.J\.R\.\s*[0-9A-Z.-]+),?\s*([0-9]{4})?,?\s*and adopted by the people ([^;\.]+)")
        .expect("valid amendment regex")
});
static CASE_CITATION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{1,3}\s+Or(?:\s+App)?\s+\d{1,4}\b").expect("valid Oregon case citation")
});
static ORS_CITATION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bORS\s+[0-9]{1,3}[A-Z]?(?:\.[0-9]{3,4})?(?:\([^)]+\))*")
        .expect("valid ORS citation")
});

#[derive(Debug, Clone)]
pub struct OregonConstitutionConnector {
    entry: SourceRegistryEntry,
    options: ConnectorOptions,
}

impl OregonConstitutionConnector {
    pub fn new(entry: SourceRegistryEntry, options: ConnectorOptions) -> Self {
        Self { entry, options }
    }
}

#[async_trait]
impl DataConnector for OregonConstitutionConnector {
    fn source_id(&self) -> &'static str {
        SOURCE_ID
    }

    fn source_kind(&self) -> SourceKind {
        SourceKind::StaticHtml
    }

    async fn discover(&self) -> Result<Vec<SourceItem>> {
        let mut items = vec![
            source_item(
                "constitution-text",
                CONSTITUTION_URL,
                "Constitution of Oregon",
                "text/html",
                "constitution_text",
            ),
            source_item(
                "constitution-preamble",
                PREAMBLE_URL,
                "Preamble to the Constitution of Oregon",
                "text/html",
                "preamble",
            ),
            source_item(
                "constitution-annotations-index",
                ANNOTATIONS_INDEX_URL,
                "Oregon Constitution Annotations Index",
                "text/html",
                "annotations_index",
            ),
        ];

        let mut annotation_items = discover_annotation_items(self.options.max_items).await;
        if annotation_items.is_empty() {
            annotation_items = fallback_annotation_items();
        }
        for item in annotation_items {
            if !items
                .iter()
                .any(|existing| existing.item_id == item.item_id)
            {
                items.push(item);
            }
        }

        if self.options.max_items > 0 {
            items.truncate(self.options.max_items.max(1));
        }
        Ok(items)
    }

    async fn parse(&self, artifact: &RawArtifact) -> Result<GraphBatch> {
        let item_kind = artifact
            .metadata
            .item_id
            .to_ascii_lowercase()
            .replace('_', "-");
        if item_kind.contains("annotation-article") || artifact.metadata.url.contains("/anc") {
            parse_annotation_html(artifact, self.options.edition_year)
        } else if item_kind.contains("preamble") || artifact.metadata.url.contains("ocapream") {
            parse_preamble_html(artifact, self.options.edition_year)
        } else if item_kind.contains("annotations-index") {
            parse_annotations_index_html(artifact, self.options.edition_year)
        } else {
            parse_constitution_html(artifact, self.options.edition_year)
        }
    }

    async fn qc(&self, artifacts: &[ArtifactMetadata], batch: &GraphBatch) -> Result<QcReport> {
        Ok(qc_source_batch(&self.entry, artifacts, batch))
    }
}

fn source_item(
    item_id: &str,
    url: &str,
    title: &str,
    content_type: &str,
    item_kind: &str,
) -> SourceItem {
    let mut metadata = BTreeMap::new();
    metadata.insert("kind".to_string(), item_kind.to_string());
    SourceItem {
        item_id: item_id.to_string(),
        url: Some(url.to_string()),
        title: Some(title.to_string()),
        content_type: Some(content_type.to_string()),
        metadata,
    }
}

async fn discover_annotation_items(max_items: usize) -> Vec<SourceItem> {
    if max_items > 0 && max_items <= 3 {
        return Vec::new();
    }
    let client = match reqwest::Client::builder()
        .user_agent("ORSGraph Oregon Constitution source registry crawler")
        .build()
    {
        Ok(client) => client,
        Err(_) => return Vec::new(),
    };
    let Ok(response) = client.get(ANNOTATIONS_INDEX_URL).send().await else {
        return Vec::new();
    };
    let Ok(html) = response.text().await else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    ANNOTATION_LINK_RE
        .captures_iter(&html)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .filter(|id| seen.insert(id.clone()))
        .map(|id| annotation_item(&id))
        .collect()
}

fn fallback_annotation_items() -> Vec<SourceItem> {
    [
        "001", "002", "003", "004", "005", "006", "007", "008", "009", "010", "011", "012", "014",
        "015", "016", "017", "018",
    ]
    .into_iter()
    .map(annotation_item)
    .collect()
}

fn annotation_item(id: &str) -> SourceItem {
    source_item(
        &format!("constitution-annotation-article-{id}"),
        &format!("https://www.oregonlegislature.gov/bills_laws/ors/anc{id}.html"),
        &format!("Oregon Constitution Annotations Article {id}"),
        "text/html",
        "annotation",
    )
}

fn parse_annotations_index_html(artifact: &RawArtifact, edition_year: i32) -> Result<GraphBatch> {
    let html = decode_html(&artifact.bytes);
    let document = Html::parse_document(&html);
    let mut batch = GraphBatch::default();
    append_base_corpus(&mut batch, artifact, edition_year)?;
    let source_doc = source_document(
        artifact,
        "annotations_index",
        "Oregon Constitution Annotations Index",
        edition_year,
        ANNOTATION_PARSER_PROFILE,
        Some("official_commentary_index"),
    );
    let source_document_id = source_doc.source_document_id.clone();
    let page_text = normalized_body_text(&document);
    batch.push("source_documents.jsonl", &source_doc)?;
    append_source_page(&mut batch, &source_document_id, "HTML", &page_text)?;
    for (order, caps) in ANNOTATION_LINK_RE.captures_iter(&html).enumerate() {
        let Some(id) = caps.get(1).map(|m| m.as_str()) else {
            continue;
        };
        let article_key = annotation_article_key(id);
        batch.push(
            "source_toc_entries.jsonl",
            &SourceTocEntry {
                source_toc_entry_id: format!("{source_document_id}:toc:{article_key}"),
                source_document_id: source_document_id.clone(),
                citation: Some(article_citation(&article_key)),
                canonical_id: Some(format!("{OR_CONSTITUTION_CORPUS_ID}:{article_key}")),
                title: format!("Oregon Constitution annotations {article_key}"),
                chapter: Some(article_key),
                page_label: None,
                page_number: Some(1),
                toc_order: order + 1,
                entry_type: "annotation_article".to_string(),
                confidence: 0.82,
            },
        )?;
    }
    Ok(batch)
}

fn parse_preamble_html(artifact: &RawArtifact, edition_year: i32) -> Result<GraphBatch> {
    let html = decode_html(&artifact.bytes);
    let document = Html::parse_document(&html);
    let mut batch = GraphBatch::default();
    append_base_corpus(&mut batch, artifact, edition_year)?;
    let source_doc = source_document(
        artifact,
        "preamble",
        "Preamble to the Constitution of Oregon",
        edition_year,
        PARSER_PROFILE,
        Some("primary_law"),
    );
    let source_document_id = source_doc.source_document_id.clone();
    let page_text = normalized_body_text(&document);
    batch.push("source_documents.jsonl", &source_doc)?;
    append_source_page(&mut batch, &source_document_id, "HTML", &page_text)?;

    let lines = html_lines(&document);
    let mut after_preamble_heading = false;
    for (order, line) in lines.iter().enumerate() {
        if line.eq_ignore_ascii_case("PREAMBLE") {
            after_preamble_heading = true;
            continue;
        }
        if after_preamble_heading && is_preamble_text(line) {
            let draft = AuthorityDraft {
                canonical_id: "or:constitution:preamble".to_string(),
                citation: "Or. Const. pmbl.".to_string(),
                title: "Preamble".to_string(),
                chapter: "preamble".to_string(),
                provision_type: "preamble".to_string(),
                local_path: vec!["Preamble".to_string()],
                text: line.clone(),
                order_index: order + 1,
                heading_path: vec!["Preamble".to_string()],
            };
            append_authority_rows(
                &mut batch,
                &draft,
                &source_document_id,
                edition_year,
                OR_CONSTITUTION_AUTHORITY_TYPE,
                AUTHORITY_LEVEL_STATE_CONSTITUTION,
                "primary_law",
            )?;
            break;
        }
    }

    for (order, toc) in article_toc_entries(&lines).into_iter().enumerate() {
        batch.push(
            "source_toc_entries.jsonl",
            &SourceTocEntry {
                source_toc_entry_id: format!("{source_document_id}:toc:{}", toc.article_key),
                source_document_id: source_document_id.clone(),
                citation: Some(article_citation(&toc.article_key)),
                canonical_id: Some(format!("{OR_CONSTITUTION_CORPUS_ID}:{}", toc.article_key)),
                title: toc.title,
                chapter: Some(toc.article_key),
                page_label: None,
                page_number: Some(1),
                toc_order: order + 1,
                entry_type: "constitution_article".to_string(),
                confidence: 0.88,
            },
        )?;
    }
    Ok(batch)
}

fn parse_constitution_html(artifact: &RawArtifact, edition_year: i32) -> Result<GraphBatch> {
    let html = decode_html(&artifact.bytes);
    let document = Html::parse_document(&html);
    let mut batch = GraphBatch::default();
    append_base_corpus(&mut batch, artifact, edition_year)?;
    let source_doc = source_document(
        artifact,
        "constitution",
        "Constitution of Oregon",
        edition_year,
        PARSER_PROFILE,
        Some("primary_law"),
    );
    let source_document_id = source_doc.source_document_id.clone();
    let page_text = normalized_body_text(&document);
    batch.push("source_documents.jsonl", &source_doc)?;
    append_source_page(&mut batch, &source_document_id, "HTML", &page_text)?;

    let lines = html_lines(&document);
    let mut current_article: Option<ArticleContext> = None;
    let mut article_rows = BTreeSet::new();
    let mut current_section: Option<SectionDraft> = None;
    let mut order_index = 0usize;

    for (line_index, line) in lines.iter().enumerate() {
        if let Some(article) = parse_article_heading(line) {
            if let Some(section) = current_section.take() {
                append_section(&mut batch, section, &source_document_id, edition_year)?;
            }
            if article_rows.insert(article.key.clone()) {
                order_index += 1;
                append_article(
                    &mut batch,
                    &article,
                    &source_document_id,
                    edition_year,
                    order_index,
                )?;
            }
            current_article = Some(article);
            continue;
        }

        let Some(article) = current_article.clone() else {
            continue;
        };

        if is_toc_line(line) || is_article_title_only(line) {
            continue;
        }

        if let Some((section_number, title, rest)) = parse_section_start(line) {
            if let Some(section) = current_section.take() {
                append_section(&mut batch, section, &source_document_id, edition_year)?;
            }
            order_index += 1;
            let mut section = SectionDraft::new(article, &section_number, &title, order_index);
            if !rest.trim().is_empty() {
                section.push_text(rest.trim(), line_index + 1);
            }
            current_section = Some(section);
            continue;
        }

        let Some(section) = current_section.as_mut() else {
            continue;
        };

        if let Some(note_text) = parse_note_line(line) {
            section.push_source_note("note", &note_text, line_index + 1);
        } else if line.starts_with('[') && line.ends_with(']') {
            section.push_source_note(
                "source_history",
                line.trim_start_matches('[').trim_end_matches(']'),
                line_index + 1,
            );
        } else if !line.trim().is_empty() {
            section.push_text(line, line_index + 1);
        }
    }

    if let Some(section) = current_section.take() {
        append_section(&mut batch, section, &source_document_id, edition_year)?;
    }

    Ok(batch)
}

fn parse_annotation_html(artifact: &RawArtifact, edition_year: i32) -> Result<GraphBatch> {
    let html = decode_html(&artifact.bytes);
    let document = Html::parse_document(&html);
    let mut batch = GraphBatch::default();
    append_base_corpus(&mut batch, artifact, edition_year)?;
    let source_doc = source_document(
        artifact,
        "constitution_annotations",
        "Oregon Constitution Annotations",
        edition_year,
        ANNOTATION_PARSER_PROFILE,
        Some("official_commentary"),
    );
    let source_document_id = source_doc.source_document_id.clone();
    let page_text = normalized_body_text(&document);
    batch.push("source_documents.jsonl", &source_doc)?;
    append_source_page(&mut batch, &source_document_id, "HTML", &page_text)?;

    let mut current_article_key = article_key_from_annotation_url(&artifact.metadata.url);
    let mut current_article_label = current_article_key
        .as_deref()
        .map(article_label_from_key)
        .unwrap_or_else(|| "Article I".to_string());
    let mut current_section: Option<String> = None;
    let mut commentary_type = "notes_of_decisions".to_string();
    let mut seen_commentaries = BTreeSet::new();

    for (line_index, line) in html_lines(&document).iter().enumerate() {
        if let Some(article) = parse_annotation_article_line(line) {
            current_article_key = Some(article.key.clone());
            current_article_label = article.label;
            current_section = None;
            continue;
        }
        if let Some((article, section)) = parse_annotation_section_line(line) {
            if let Some(article) = article {
                current_article_key = Some(article.key.clone());
                current_article_label = article.label;
            }
            current_section = Some(section);
            continue;
        }
        if line.eq_ignore_ascii_case("NOTES OF DECISIONS") {
            commentary_type = "notes_of_decisions".to_string();
            continue;
        }
        if line.to_ascii_uppercase().starts_with("ATTY. GEN. OPINIONS") {
            commentary_type = "attorney_general_opinion_note".to_string();
        } else if line
            .to_ascii_uppercase()
            .starts_with("LAW REVIEW CITATIONS")
        {
            commentary_type = "law_review_citation_note".to_string();
        }

        let Some(article_key) = current_article_key.as_deref() else {
            continue;
        };
        let Some(section_number) = current_section.as_deref() else {
            continue;
        };
        if !is_annotation_body_line(line) {
            continue;
        }

        let target = section_canonical_id(article_key, section_number);
        let citation = section_citation(&current_article_label, section_number);
        let normalized = normalize_ws(line);
        let commentary_id = format!(
            "or:constitution:annotation:{}:{}",
            target.trim_start_matches("or:constitution:"),
            stable_id(&normalized)
        );
        if !seen_commentaries.insert(commentary_id.clone()) {
            continue;
        }
        batch.push(
            "commentaries.jsonl",
            &Commentary {
                commentary_id: commentary_id.clone(),
                source_document_id: source_document_id.clone(),
                canonical_id: None,
                version_id: None,
                source_provision_id: None,
                target_canonical_id: Some(target.clone()),
                target_provision_id: Some(target.clone()),
                citation: Some(citation),
                authority_family: Some(OR_CONSTITUTION_AUTHORITY_FAMILY.to_string()),
                corpus_id: Some(OR_CONSTITUTION_CORPUS_ID.to_string()),
                authority_level: Some(AUTHORITY_LEVEL_OFFICIAL_COMMENTARY),
                source_role: Some("official_commentary".to_string()),
                commentary_type: commentary_type.clone(),
                text: normalized.clone(),
                normalized_text: normalized.clone(),
                source_page_start: Some(1),
                source_page_end: Some(1),
                confidence: 0.82,
                extraction_method: ANNOTATION_PARSER_PROFILE.to_string(),
            },
        )?;
        append_annotation_chunk(
            &mut batch,
            &commentary_id,
            &target,
            &normalized,
            &source_document_id,
            edition_year,
            line_index + 1,
        )?;
        append_external_citations(&mut batch, &commentary_id, &normalized)?;
    }
    Ok(batch)
}

fn append_base_corpus(
    batch: &mut GraphBatch,
    artifact: &RawArtifact,
    edition_year: i32,
) -> Result<()> {
    batch.push(
        "jurisdictions.jsonl",
        &Jurisdiction {
            jurisdiction_id: US_JURISDICTION_ID.to_string(),
            name: "United States".to_string(),
            jurisdiction_type: "federal".to_string(),
            parent_jurisdiction_id: None,
            country: Some("US".to_string()),
        },
    )?;
    batch.push(
        "jurisdictions.jsonl",
        &Jurisdiction {
            jurisdiction_id: OR_JURISDICTION_ID.to_string(),
            name: "Oregon".to_string(),
            jurisdiction_type: "state".to_string(),
            parent_jurisdiction_id: Some(US_JURISDICTION_ID.to_string()),
            country: Some("US".to_string()),
        },
    )?;
    batch.push(
        "legal_corpora.jsonl",
        &LegalCorpus {
            corpus_id: OR_CONSTITUTION_CORPUS_ID.to_string(),
            name: "Constitution of Oregon".to_string(),
            short_name: "Oregon Constitution".to_string(),
            authority_family: OR_CONSTITUTION_AUTHORITY_FAMILY.to_string(),
            authority_type: OR_CONSTITUTION_AUTHORITY_TYPE.to_string(),
            jurisdiction_id: OR_JURISDICTION_ID.to_string(),
        },
    )?;
    batch.push(
        "corpus_editions.jsonl",
        &CorpusEdition {
            edition_id: edition_id(edition_year),
            corpus_id: OR_CONSTITUTION_CORPUS_ID.to_string(),
            edition_year,
            effective_date: None,
            source_label: Some(format!(
                "{OR_CONSTITUTION_EDITION_SOURCE} retrieved {}",
                artifact.metadata.retrieved_at.to_rfc3339()
            )),
            current: Some(true),
        },
    )
}

fn source_document(
    artifact: &RawArtifact,
    chapter: &str,
    title: &str,
    edition_year: i32,
    parser_profile: &str,
    source_type: Option<&str>,
) -> SourceDocument {
    let text = decode_html(&artifact.bytes);
    let normalized = normalize_ws(&html_to_text(&text));
    SourceDocument {
        source_document_id: format!("src:{SOURCE_ID}:{}", stable_id(&artifact.metadata.url)),
        source_provider: "Oregon Legislature".to_string(),
        source_kind: "static_html".to_string(),
        url: artifact.metadata.url.clone(),
        chapter: chapter.to_string(),
        corpus_id: Some(OR_CONSTITUTION_CORPUS_ID.to_string()),
        edition_id: Some(edition_id(edition_year)),
        authority_family: Some(OR_CONSTITUTION_AUTHORITY_FAMILY.to_string()),
        authority_type: Some(OR_CONSTITUTION_AUTHORITY_TYPE.to_string()),
        title: Some(title.to_string()),
        source_type: source_type.map(ToString::to_string),
        file_name: artifact
            .metadata
            .path
            .rsplit('/')
            .next()
            .map(ToOwned::to_owned),
        page_count: Some(1),
        effective_date: None,
        copyright_status: Some("free".to_string()),
        chapter_title: Some(title.to_string()),
        edition_year,
        html_encoding: Some("windows-1252".to_string()),
        source_path: Some(artifact.metadata.path.clone()),
        paragraph_count: Some(normalized.lines().filter(|line| !line.is_empty()).count()),
        first_body_paragraph_index: Some(0),
        parser_profile: Some(parser_profile.to_string()),
        official_status: "official".to_string(),
        disclaimer_required: true,
        raw_hash: artifact.metadata.raw_hash.clone(),
        normalized_hash: sha256_hex(normalized.as_bytes()),
    }
}

fn append_source_page(
    batch: &mut GraphBatch,
    source_document_id: &str,
    label: &str,
    text: &str,
) -> Result<()> {
    let normalized = normalize_ws(text);
    batch.push(
        "source_pages.jsonl",
        &SourcePage {
            source_page_id: format!("{source_document_id}:page:1"),
            source_document_id: source_document_id.to_string(),
            page_number: 1,
            printed_label: Some(label.to_string()),
            text: text.to_string(),
            normalized_text: normalized.clone(),
            text_hash: sha256_hex(normalized.as_bytes()),
        },
    )
}

fn append_article(
    batch: &mut GraphBatch,
    article: &ArticleContext,
    source_document_id: &str,
    edition_year: i32,
    order_index: usize,
) -> Result<()> {
    let title = article
        .title
        .as_deref()
        .unwrap_or(article.label.as_str())
        .to_string();
    let draft = AuthorityDraft {
        canonical_id: format!("{OR_CONSTITUTION_CORPUS_ID}:{}", article.key),
        citation: article_citation(&article.key),
        title: title.clone(),
        chapter: article.key.clone(),
        provision_type: "article".to_string(),
        local_path: vec![article.label.clone()],
        text: title,
        order_index,
        heading_path: vec![article.label.clone()],
    };
    append_authority_rows(
        batch,
        &draft,
        source_document_id,
        edition_year,
        OR_CONSTITUTION_AUTHORITY_TYPE,
        AUTHORITY_LEVEL_STATE_CONSTITUTION,
        "primary_law",
    )
}

fn append_section(
    batch: &mut GraphBatch,
    mut section: SectionDraft,
    source_document_id: &str,
    edition_year: i32,
) -> Result<()> {
    section.flush_pending_source_notes();
    let text = section.text_lines.join("\n\n");
    let draft = AuthorityDraft {
        canonical_id: section.canonical_id(),
        citation: section.citation(),
        title: section.title.clone(),
        chapter: section.article.key.clone(),
        provision_type: "section".to_string(),
        local_path: vec![
            section.article.label.clone(),
            format!("Section {}", section.number),
        ],
        text,
        order_index: section.order_index,
        heading_path: vec![section.article.label.clone(), section.title.clone()],
    };
    append_authority_rows(
        batch,
        &draft,
        source_document_id,
        edition_year,
        OR_CONSTITUTION_AUTHORITY_TYPE,
        AUTHORITY_LEVEL_STATE_CONSTITUTION,
        "primary_law",
    )?;
    for subsection in &section.subsections {
        append_subsection(batch, &section, subsection, edition_year)?;
    }
    for source_note in &section.source_notes {
        append_source_note_and_history(
            batch,
            &section,
            source_note,
            source_document_id,
            edition_year,
        )?;
    }
    append_citation_mentions(batch, &section, &draft.text)?;
    Ok(())
}

fn append_authority_rows(
    batch: &mut GraphBatch,
    draft: &AuthorityDraft,
    source_document_id: &str,
    edition_year: i32,
    authority_type: &str,
    authority_level: i32,
    source_role: &str,
) -> Result<()> {
    let normalized_text = normalize_ws(&draft.text);
    let version_id = format!("{}@{edition_year}", draft.canonical_id);
    batch.push(
        "legal_text_identities.jsonl",
        &LegalTextIdentity {
            canonical_id: draft.canonical_id.clone(),
            citation: draft.citation.clone(),
            jurisdiction_id: OR_JURISDICTION_ID.to_string(),
            authority_family: OR_CONSTITUTION_AUTHORITY_FAMILY.to_string(),
            corpus_id: Some(OR_CONSTITUTION_CORPUS_ID.to_string()),
            authority_type: Some(authority_type.to_string()),
            authority_level: Some(authority_level),
            effective_date: None,
            title: Some(draft.title.clone()),
            chapter: draft.chapter.clone(),
            status: "active".to_string(),
        },
    )?;
    batch.push(
        "legal_text_versions.jsonl",
        &LegalTextVersion {
            version_id: version_id.clone(),
            canonical_id: draft.canonical_id.clone(),
            citation: draft.citation.clone(),
            title: Some(draft.title.clone()),
            chapter: draft.chapter.clone(),
            corpus_id: Some(OR_CONSTITUTION_CORPUS_ID.to_string()),
            edition_id: Some(edition_id(edition_year)),
            authority_family: Some(OR_CONSTITUTION_AUTHORITY_FAMILY.to_string()),
            authority_type: Some(authority_type.to_string()),
            authority_level: Some(authority_level),
            effective_date: None,
            source_page_start: Some(1),
            source_page_end: Some(1),
            edition_year,
            status: "active".to_string(),
            status_text: Some("current".to_string()),
            text: draft.text.clone(),
            text_hash: sha256_hex(normalized_text.as_bytes()),
            original_text: Some(draft.text.clone()),
            paragraph_start_order: Some(draft.order_index),
            paragraph_end_order: Some(draft.order_index),
            source_paragraph_ids: Vec::new(),
            source_document_id: source_document_id.to_string(),
            official_status: "official".to_string(),
            disclaimer_required: true,
            embedding_model: None,
            embedding_dim: None,
            embedding: None,
            embedding_input_hash: None,
            embedding_input_type: None,
            embedding_output_dtype: None,
            embedded_at: None,
            embedding_profile: None,
            embedding_strategy: None,
            embedding_source_dimension: None,
        },
    )?;
    batch.push(
        "provisions.jsonl",
        &Provision {
            provision_id: draft.canonical_id.clone(),
            version_id: version_id.clone(),
            canonical_id: draft.canonical_id.clone(),
            citation: draft.citation.clone(),
            display_citation: draft.citation.clone(),
            chapter: Some(draft.chapter.clone()),
            corpus_id: Some(OR_CONSTITUTION_CORPUS_ID.to_string()),
            edition_id: Some(edition_id(edition_year)),
            authority_family: Some(OR_CONSTITUTION_AUTHORITY_FAMILY.to_string()),
            authority_type: Some(authority_type.to_string()),
            authority_level: Some(authority_level),
            effective_date: None,
            source_page_start: Some(1),
            source_page_end: Some(1),
            local_path: draft.local_path.clone(),
            provision_type: draft.provision_type.clone(),
            text: draft.text.clone(),
            original_text: Some(draft.text.clone()),
            normalized_text: normalized_text.clone(),
            order_index: draft.order_index,
            depth: draft.local_path.len().max(1),
            text_hash: sha256_hex(normalized_text.as_bytes()),
            is_implied: false,
            is_definition_candidate: false,
            is_exception_candidate: false,
            is_deadline_candidate: false,
            is_penalty_candidate: false,
            paragraph_start_order: Some(draft.order_index),
            paragraph_end_order: Some(draft.order_index),
            source_paragraph_ids: Vec::new(),
            heading_path: draft.heading_path.clone(),
            structural_context: Some(source_role.to_string()),
            embedding_model: None,
            embedding_dim: None,
            embedding: None,
            embedding_input_hash: None,
            embedding_input_type: None,
            embedding_output_dtype: None,
            embedded_at: None,
            embedding_profile: None,
            embedding_source_dimension: None,
        },
    )?;
    append_chunk(
        batch,
        &format!(
            "chunk:{}:{}",
            draft.canonical_id,
            stable_id(&normalized_text)
        ),
        "constitutional_text",
        &draft.text,
        &draft.title,
        Some(&draft.canonical_id),
        &version_id,
        &draft.canonical_id,
        &draft.citation,
        authority_level,
        authority_type,
        Some(&draft.chapter),
        edition_year,
    )
}

fn append_subsection(
    batch: &mut GraphBatch,
    section: &SectionDraft,
    subsection: &SubsectionDraft,
    edition_year: i32,
) -> Result<()> {
    let canonical_id = section.canonical_id();
    let version_id = format!("{canonical_id}@{edition_year}");
    let path = vec![
        section.article.label.clone(),
        format!("Section {}", section.number),
        format!("({})", subsection.marker),
    ];
    let provision_id = format!(
        "{}:{}",
        canonical_id,
        clean_id_part(&subsection.marker.to_ascii_lowercase())
    );
    let citation = format!("{}({})", section.citation(), subsection.marker);
    let normalized = normalize_ws(&subsection.text);
    batch.push(
        "provisions.jsonl",
        &Provision {
            provision_id: provision_id.clone(),
            version_id: version_id.clone(),
            canonical_id,
            citation: section.citation(),
            display_citation: citation.clone(),
            chapter: Some(section.article.key.clone()),
            corpus_id: Some(OR_CONSTITUTION_CORPUS_ID.to_string()),
            edition_id: Some(edition_id(edition_year)),
            authority_family: Some(OR_CONSTITUTION_AUTHORITY_FAMILY.to_string()),
            authority_type: Some(OR_CONSTITUTION_AUTHORITY_TYPE.to_string()),
            authority_level: Some(AUTHORITY_LEVEL_STATE_CONSTITUTION),
            effective_date: None,
            source_page_start: Some(1),
            source_page_end: Some(1),
            local_path: path.clone(),
            provision_type: "subsection".to_string(),
            text: subsection.text.clone(),
            original_text: Some(subsection.text.clone()),
            normalized_text: normalized.clone(),
            order_index: subsection.order_index,
            depth: path.len(),
            text_hash: sha256_hex(normalized.as_bytes()),
            is_implied: false,
            is_definition_candidate: false,
            is_exception_candidate: false,
            is_deadline_candidate: false,
            is_penalty_candidate: false,
            paragraph_start_order: Some(subsection.source_order),
            paragraph_end_order: Some(subsection.source_order),
            source_paragraph_ids: Vec::new(),
            heading_path: path,
            structural_context: Some("primary_law".to_string()),
            embedding_model: None,
            embedding_dim: None,
            embedding: None,
            embedding_input_hash: None,
            embedding_input_type: None,
            embedding_output_dtype: None,
            embedded_at: None,
            embedding_profile: None,
            embedding_source_dimension: None,
        },
    )?;
    append_chunk(
        batch,
        &format!("chunk:{provision_id}:{}", stable_id(&normalized)),
        "constitutional_subsection",
        &subsection.text,
        &citation,
        Some(&provision_id),
        &version_id,
        &section.canonical_id(),
        &citation,
        AUTHORITY_LEVEL_STATE_CONSTITUTION,
        OR_CONSTITUTION_AUTHORITY_TYPE,
        Some(&section.article.key),
        edition_year,
    )?;
    let phantom = SectionDraft {
        text_lines: vec![subsection.text.clone()],
        ..section.clone_for_mentions()
    };
    append_citation_mentions(batch, &phantom, &subsection.text)
}

fn append_source_note_and_history(
    batch: &mut GraphBatch,
    section: &SectionDraft,
    note: &SourceNoteDraft,
    source_document_id: &str,
    edition_year: i32,
) -> Result<()> {
    let canonical_id = section.canonical_id();
    let version_id = format!("{canonical_id}@{edition_year}");
    let normalized = normalize_ws(&note.text);
    let source_note_id = format!(
        "source-note:{}:{}:{}",
        canonical_id,
        note.note_type,
        stable_id(&normalized)
    );
    batch.push(
        "source_notes.jsonl",
        &SourceNote {
            source_note_id: source_note_id.clone(),
            note_type: note.note_type.clone(),
            text: note.text.clone(),
            normalized_text: normalized.clone(),
            source_document_id: source_document_id.to_string(),
            canonical_id: canonical_id.clone(),
            version_id: Some(version_id.clone()),
            provision_id: Some(canonical_id.clone()),
            citation: section.citation(),
            paragraph_start_order: note.source_order,
            paragraph_end_order: note.source_order,
            source_paragraph_order: note.source_order,
            source_paragraph_ids: Vec::new(),
            confidence: 0.86,
            extraction_method: PARSER_PROFILE.to_string(),
        },
    )?;
    append_history_rows(
        batch,
        section,
        &source_note_id,
        &normalized,
        source_document_id,
        edition_year,
    )
}

fn append_history_rows(
    batch: &mut GraphBatch,
    section: &SectionDraft,
    source_note_id: &str,
    text: &str,
    source_document_id: &str,
    edition_year: i32,
) -> Result<()> {
    let canonical_id = section.canonical_id();
    let version_id = format!("{canonical_id}@{edition_year}");
    let mut emitted_any = false;

    for (index, caps) in CREATED_RE.captures_iter(text).enumerate() {
        let proposal = caps.get(1).map(|m| normalize_ws(m.as_str()));
        let adopted = caps.get(2).map(|m| normalize_ws(m.as_str()));
        append_amendment_row(
            batch,
            section,
            source_note_id,
            source_document_id,
            "created",
            proposal.as_deref(),
            adopted.as_deref(),
            None,
            edition_year,
            index + 1,
        )?;
        emitted_any = true;
    }

    for (index, caps) in AMENDMENT_RE.captures_iter(text).enumerate() {
        let resolution = caps.get(1).map(|m| normalize_ws(m.as_str()));
        let proposed_year = caps.get(2).and_then(|m| m.as_str().parse::<i32>().ok());
        let adopted = caps.get(3).map(|m| normalize_ws(m.as_str()));
        append_amendment_row(
            batch,
            section,
            source_note_id,
            source_document_id,
            "amended",
            resolution.as_deref(),
            adopted.as_deref(),
            proposed_year,
            edition_year,
            index + 1,
        )?;
        if let Some(year) = proposed_year {
            batch.push(
                "lineage_events.jsonl",
                &LineageEvent {
                    lineage_event_id: format!(
                        "lineage:{}:{}",
                        canonical_id,
                        stable_id(&format!("{source_note_id}:proposed:{year}"))
                    ),
                    source_note_id: Some(source_note_id.to_string()),
                    from_canonical_id: Some(canonical_id.clone()),
                    to_canonical_id: Some(canonical_id.clone()),
                    current_canonical_id: canonical_id.clone(),
                    lineage_type: "constitutional_amendment_proposed".to_string(),
                    raw_text: text.to_string(),
                    year: Some(year),
                    confidence: 0.74,
                },
            )?;
        }
        emitted_any = true;
    }

    let lower = text.to_ascii_lowercase();
    if lower.contains("repealed")
        || lower.contains("supplanted")
        || lower.contains("superseded")
        || lower.contains("declared void")
        || lower.contains("void")
    {
        let status_type = if lower.contains("declared void") || lower.contains("void") {
            "voided"
        } else if lower.contains("supplanted") || lower.contains("superseded") {
            "superseded"
        } else {
            "repealed"
        };
        batch.push(
            "status_events.jsonl",
            &StatusEvent {
                status_event_id: format!(
                    "status:{}:{}",
                    canonical_id,
                    stable_id(&format!("{source_note_id}:{status_type}"))
                ),
                status_type: status_type.to_string(),
                status_text: Some(text.to_string()),
                source_document_id: Some(source_document_id.to_string()),
                canonical_id: canonical_id.clone(),
                version_id: Some(version_id.clone()),
                event_year: first_year(text),
                effective_date: None,
                source_note_id: Some(source_note_id.to_string()),
                effect_type: Some(status_type.to_string()),
                trigger_text: Some(text.to_string()),
                operative_date: None,
                repeal_date: None,
                session_law_ref: None,
                confidence: 0.78,
                extraction_method: PARSER_PROFILE.to_string(),
            },
        )?;
        batch.push(
            "temporal_effects.jsonl",
            &TemporalEffect {
                temporal_effect_id: format!(
                    "temporal:{}:{}",
                    canonical_id,
                    stable_id(&format!("{source_note_id}:{status_type}"))
                ),
                source_note_id: Some(source_note_id.to_string()),
                source_provision_id: Some(canonical_id.clone()),
                version_id: Some(version_id),
                canonical_id: Some(canonical_id),
                effect_type: status_type.to_string(),
                trigger_text: text.to_string(),
                effective_date: None,
                operative_date: None,
                repeal_date: None,
                expiration_date: None,
                session_law_ref: None,
                confidence: 0.72,
            },
        )?;
        emitted_any = true;
    }

    if !emitted_any {
        batch.push(
            "parser_diagnostics.jsonl",
            &ParserDiagnostic {
                parser_diagnostic_id: format!(
                    "diag:{SOURCE_ID}:{}",
                    stable_id(&format!("{source_note_id}:unparsed_history"))
                ),
                source_document_id: source_document_id.to_string(),
                chapter: section.article.key.clone(),
                edition_year,
                severity: "info".to_string(),
                diagnostic_type: "constitution_history_note_unclassified".to_string(),
                message: "Constitution source note preserved but not classified as amendment, status, or temporal effect.".to_string(),
                source_paragraph_order: None,
                related_id: Some(source_note_id.to_string()),
                parser_profile: PARSER_PROFILE.to_string(),
            },
        )?;
    }
    Ok(())
}

fn append_amendment_row(
    batch: &mut GraphBatch,
    section: &SectionDraft,
    source_note_id: &str,
    source_document_id: &str,
    amendment_type: &str,
    proposal_text: Option<&str>,
    adopted_date: Option<&str>,
    proposed_year_hint: Option<i32>,
    edition_year: i32,
    occurrence: usize,
) -> Result<()> {
    let canonical_id = section.canonical_id();
    let version_id = format!("{canonical_id}@{edition_year}");
    let proposal_method = proposal_text.map(infer_proposal_method);
    let (resolution_chamber, resolution_number) = proposal_text
        .and_then(parse_resolution)
        .map(|(chamber, number)| (Some(chamber), Some(number)))
        .unwrap_or((None, None));
    let amendment_id = format!(
        "amendment:{}:{}:{}",
        canonical_id,
        amendment_type,
        stable_id(&format!("{source_note_id}:{occurrence}"))
    );
    batch.push(
        "amendments.jsonl",
        &Amendment {
            amendment_id,
            amendment_type: amendment_type.to_string(),
            session_law_citation: proposal_text.map(ToString::to_string),
            effective_date: adopted_date.map(ToString::to_string),
            text: proposal_text
                .or(adopted_date)
                .unwrap_or(amendment_type)
                .to_string(),
            raw_text: proposal_text.map(ToString::to_string),
            source_document_id: Some(source_document_id.to_string()),
            confidence: 0.78,
            canonical_id: Some(canonical_id.clone()),
            version_id: Some(version_id.clone()),
            session_law_id: None,
            affected_canonical_id: Some(canonical_id.clone()),
            affected_version_id: Some(version_id),
            source_note_id: Some(source_note_id.to_string()),
            proposal_method,
            proposal_id: proposal_text.map(|text| clean_id_part(text)),
            measure_number: proposal_text.and_then(parse_measure_number),
            resolution_chamber,
            resolution_number,
            filed_date: proposal_text.and_then(parse_filed_date),
            proposed_year: proposed_year_hint.or_else(|| proposal_text.and_then(first_year)),
            adopted_date: adopted_date.map(ToString::to_string),
            election_date: adopted_date.map(ToString::to_string),
            resolution_status: Some("source_note_extracted".to_string()),
        },
    )?;
    if let Some(text) = proposal_text {
        if text.to_ascii_lowercase().contains("oregon laws") {
            batch.push(
                "session_laws.jsonl",
                &SessionLaw {
                    session_law_id: format!("or:constitution:session-law:{}", stable_id(text)),
                    jurisdiction_id: Some(OR_JURISDICTION_ID.to_string()),
                    citation: text.to_string(),
                    year: first_year(text).unwrap_or(0),
                    chapter: None,
                    section: None,
                    bill_number: None,
                    effective_date: adopted_date.map(ToString::to_string),
                    text: Some(text.to_string()),
                    raw_text: Some(text.to_string()),
                    source_document_id: Some(source_document_id.to_string()),
                    source_note_id: Some(source_note_id.to_string()),
                    confidence: 0.62,
                },
            )?;
        }
    }
    Ok(())
}

fn append_chunk(
    batch: &mut GraphBatch,
    chunk_id: &str,
    chunk_type: &str,
    text: &str,
    breadcrumb: &str,
    source_provision_id: Option<&str>,
    parent_version_id: &str,
    canonical_id: &str,
    citation: &str,
    authority_level: i32,
    authority_type: &str,
    chapter: Option<&str>,
    edition_year: i32,
) -> Result<()> {
    let normalized = normalize_ws(text);
    batch.push(
        "retrieval_chunks.jsonl",
        &RetrievalChunk {
            chunk_id: chunk_id.to_string(),
            chunk_type: chunk_type.to_string(),
            text: text.to_string(),
            breadcrumb: breadcrumb.to_string(),
            source_provision_id: source_provision_id.map(ToString::to_string),
            source_version_id: Some(parent_version_id.to_string()),
            parent_version_id: parent_version_id.to_string(),
            canonical_id: canonical_id.to_string(),
            citation: citation.to_string(),
            jurisdiction_id: OR_JURISDICTION_ID.to_string(),
            authority_level,
            authority_family: Some(OR_CONSTITUTION_AUTHORITY_FAMILY.to_string()),
            corpus_id: Some(OR_CONSTITUTION_CORPUS_ID.to_string()),
            authority_type: Some(authority_type.to_string()),
            effective_date: None,
            chapter: chapter.map(ToString::to_string),
            source_page_start: Some(1),
            source_page_end: Some(1),
            edition_year,
            embedding_model: None,
            embedding_dim: None,
            embedding: None,
            embedding_input_hash: sha256_hex(normalized.as_bytes()),
            embedding_policy: Some("source_backed".to_string()),
            answer_policy: Some("cite_source".to_string()),
            chunk_schema_version: Some("1.0.0".to_string()),
            retrieval_profile: Some("legal_authority_v1".to_string()),
            search_weight: Some(if authority_type == OR_CONSTITUTION_AUTHORITY_TYPE {
                1.9
            } else {
                0.72
            }),
            embedding_input_type: Some("legal_text".to_string()),
            embedding_output_dtype: None,
            embedded_at: None,
            source_kind: Some("official".to_string()),
            source_id: Some(SOURCE_ID.to_string()),
            token_count: Some(normalized.split_whitespace().count()),
            max_tokens: None,
            context_window: None,
            chunking_strategy: Some("constitution_section_or_annotation".to_string()),
            chunk_version: Some("1".to_string()),
            overlap_tokens: Some(0),
            split_reason: None,
            part_index: Some(1),
            part_count: Some(1),
            is_definition_candidate: false,
            is_exception_candidate: false,
            is_penalty_candidate: false,
            heading_path: vec![breadcrumb.to_string()],
            structural_context: Some(authority_type.to_string()),
            embedding_profile: None,
            embedding_source_dimension: None,
        },
    )
}

fn append_annotation_chunk(
    batch: &mut GraphBatch,
    commentary_id: &str,
    target_canonical_id: &str,
    text: &str,
    _source_document_id: &str,
    edition_year: i32,
    order_index: usize,
) -> Result<()> {
    let version_id = format!("{target_canonical_id}@{edition_year}");
    append_chunk(
        batch,
        &format!("chunk:{commentary_id}:{order_index}"),
        "constitution_annotation",
        text,
        "Oregon Constitution annotation",
        None,
        &version_id,
        target_canonical_id,
        target_canonical_id,
        AUTHORITY_LEVEL_OFFICIAL_COMMENTARY,
        "official_commentary",
        target_canonical_id
            .strip_prefix("or:constitution:")
            .and_then(|rest| rest.split(":section-").next()),
        edition_year,
    )
}

fn append_citation_mentions(
    batch: &mut GraphBatch,
    section: &SectionDraft,
    text: &str,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for caps in ORS_CITATION_RE.captures_iter(text) {
        let Some(raw) = caps.get(0).map(|m| m.as_str()) else {
            continue;
        };
        let normalized = normalize_ws(raw);
        if !seen.insert(normalized.clone()) {
            continue;
        }
        let target = normalized
            .split_whitespace()
            .nth(1)
            .and_then(|value| value.split('(').next())
            .map(|section| format!("or:ors:{section}"));
        batch.push(
            "citation_mentions.jsonl",
            &CitationMention {
                citation_mention_id: format!(
                    "cite:{}:{}",
                    section.canonical_id(),
                    stable_id(&normalized)
                ),
                source_provision_id: section.canonical_id(),
                raw_text: raw.to_string(),
                normalized_citation: normalized,
                citation_type: "statute".to_string(),
                target_canonical_id: target,
                target_start_canonical_id: None,
                target_end_canonical_id: None,
                target_provision_id: None,
                unresolved_subpath: None,
                external_citation_id: None,
                resolver_status: "resolved_by_pattern".to_string(),
                confidence: 0.72,
                qc_severity: None,
            },
        )?;
    }
    Ok(())
}

fn append_external_citations(batch: &mut GraphBatch, source_id: &str, text: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for caps in CASE_CITATION_RE.captures_iter(text) {
        let Some(raw) = caps.get(0).map(|m| m.as_str()) else {
            continue;
        };
        let normalized = normalize_ws(raw);
        if !seen.insert(normalized.clone()) {
            continue;
        }
        batch.push(
            "external_legal_citations.jsonl",
            &ExternalLegalCitation {
                external_citation_id: format!(
                    "external:orconst:{}:{}",
                    clean_id_part(source_id),
                    stable_id(&normalized)
                ),
                citation: raw.to_string(),
                normalized_citation: normalized,
                citation_type: "case_law".to_string(),
                jurisdiction_id: OR_JURISDICTION_ID.to_string(),
                source_system: ANNOTATION_PARSER_PROFILE.to_string(),
                url: None,
            },
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct AuthorityDraft {
    canonical_id: String,
    citation: String,
    title: String,
    chapter: String,
    provision_type: String,
    local_path: Vec<String>,
    text: String,
    order_index: usize,
    heading_path: Vec<String>,
}

#[derive(Debug, Clone)]
struct ArticleContext {
    key: String,
    label: String,
    title: Option<String>,
}

#[derive(Debug, Clone)]
struct SectionDraft {
    article: ArticleContext,
    number: String,
    title: String,
    text_lines: Vec<String>,
    subsections: Vec<SubsectionDraft>,
    source_notes: Vec<SourceNoteDraft>,
    pending_source_note: Option<SourceNoteDraft>,
    order_index: usize,
    first_source_order: usize,
}

#[derive(Debug, Clone)]
struct SubsectionDraft {
    marker: String,
    text: String,
    order_index: usize,
    source_order: usize,
}

#[derive(Debug, Clone)]
struct SourceNoteDraft {
    note_type: String,
    text: String,
    source_order: usize,
}

#[derive(Debug)]
struct TocEntry {
    article_key: String,
    title: String,
}

impl SectionDraft {
    fn new(article: ArticleContext, number: &str, title: &str, order_index: usize) -> Self {
        Self {
            article,
            number: number.to_string(),
            title: title.to_string(),
            text_lines: Vec::new(),
            subsections: Vec::new(),
            source_notes: Vec::new(),
            pending_source_note: None,
            order_index,
            first_source_order: order_index,
        }
    }

    fn canonical_id(&self) -> String {
        section_canonical_id(&self.article.key, &self.number)
    }

    fn citation(&self) -> String {
        section_citation(&self.article.label, &self.number)
    }

    fn push_text(&mut self, line: &str, source_order: usize) {
        self.flush_pending_source_notes();
        let mut text = normalize_ws(line);
        if let Some(caps) = BRACKETED_SOURCE_NOTE_RE.captures(&text) {
            if let Some(note) = caps.get(1).map(|m| normalize_ws(m.as_str())) {
                text = BRACKETED_SOURCE_NOTE_RE
                    .replace(&text, "")
                    .trim()
                    .to_string();
                self.push_source_note("source_history", &note, source_order);
            }
        }
        if text.is_empty() {
            return;
        }
        if self.first_source_order == self.order_index {
            self.first_source_order = source_order;
        }
        if let Some((marker, body)) = parse_subsection_line(&text) {
            self.subsections.push(SubsectionDraft {
                marker,
                text: body,
                order_index: self.order_index + self.subsections.len() + 1,
                source_order,
            });
        } else {
            self.text_lines.push(text);
        }
    }

    fn push_source_note(&mut self, note_type: &str, text: &str, source_order: usize) {
        self.flush_pending_source_notes();
        self.pending_source_note = Some(SourceNoteDraft {
            note_type: note_type.to_string(),
            text: normalize_ws(text),
            source_order,
        });
    }

    fn flush_pending_source_notes(&mut self) {
        if let Some(note) = self.pending_source_note.take() {
            self.source_notes.push(note);
        }
    }

    fn clone_for_mentions(&self) -> Self {
        Self {
            article: self.article.clone(),
            number: self.number.clone(),
            title: self.title.clone(),
            text_lines: Vec::new(),
            subsections: Vec::new(),
            source_notes: Vec::new(),
            pending_source_note: None,
            order_index: self.order_index,
            first_source_order: self.first_source_order,
        }
    }
}

fn parse_article_heading(line: &str) -> Option<ArticleContext> {
    let caps = ARTICLE_RE.captures(line.trim())?;
    let token = caps.get(1)?.as_str().to_ascii_uppercase();
    let variant = caps.get(2).map(|m| m.as_str().to_ascii_lowercase());
    let key = article_key(&token, variant.as_deref());
    let mut label = format!("Article {token}");
    if let Some(variant) = variant {
        label.push_str(&format!(" ({})", title_case(&variant)));
    }
    Some(ArticleContext {
        key,
        label,
        title: None,
    })
}

fn parse_section_start(line: &str) -> Option<(String, String, String)> {
    let caps = SECTION_RE.captures(line.trim())?;
    Some((
        caps.get(1)?.as_str().to_string(),
        normalize_ws(caps.get(2)?.as_str()),
        caps.get(3)
            .map(|m| normalize_ws(m.as_str()))
            .unwrap_or_default(),
    ))
}

fn parse_subsection_line(line: &str) -> Option<(String, String)> {
    let caps = SUBSECTION_RE.captures(line)?;
    Some((
        caps.get(1)?.as_str().to_string(),
        normalize_ws(caps.get(2)?.as_str()),
    ))
}

fn parse_note_line(line: &str) -> Option<String> {
    NOTE_RE
        .captures(line.trim())
        .and_then(|caps| caps.get(1).map(|m| normalize_ws(m.as_str())))
}

fn parse_annotation_article_line(line: &str) -> Option<ArticleContext> {
    let trimmed = line.trim();
    let caps = Regex::new(r"(?i)^Article\s+([IVXLCDM]+(?:-[A-Z]+(?:\([0-9]+\))?)?)$")
        .ok()?
        .captures(trimmed)?;
    let token = caps.get(1)?.as_str().to_ascii_uppercase();
    Some(ArticleContext {
        key: article_key(&token, None),
        label: format!("Article {token}"),
        title: None,
    })
}

fn parse_annotation_section_line(line: &str) -> Option<(Option<ArticleContext>, String)> {
    let trimmed = line.trim();
    if let Some(caps) = Regex::new(r"(?i)^Section\s+([0-9]+[a-z]?)$")
        .ok()?
        .captures(trimmed)
    {
        return Some((None, caps.get(1)?.as_str().to_string()));
    }
    let caps = Regex::new(
        r"(?i)^Art\.?\s+([IVXLCDM]+(?:-[A-Z]+(?:\([0-9]+\))?)?),?\s+Section\s+([0-9]+[a-z]?)$",
    )
    .ok()?
    .captures(trimmed)?;
    let token = caps.get(1)?.as_str().to_ascii_uppercase();
    let article = ArticleContext {
        key: article_key(&token, None),
        label: format!("Article {token}"),
        title: None,
    };
    Some((Some(article), caps.get(2)?.as_str().to_string()))
}

fn is_annotation_body_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let upper = trimmed.to_ascii_uppercase();
    !upper.starts_with("OREGON CONSTITUTION ANNOTATIONS")
        && !upper.starts_with("CONSTITUTION OF OREGON")
        && !upper.starts_with("ARTICLE ")
        && !upper.starts_with("SECTION ")
        && !upper.starts_with("ART. ")
        && upper != "NOTES OF DECISIONS"
}

fn is_toc_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("Sec.")
        || Regex::new(r"^[0-9]+[a-z]?\.\s+").unwrap().is_match(trimmed)
        || trimmed.eq_ignore_ascii_case("Sec.")
}

fn is_article_title_only(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && trimmed.chars().any(|ch| ch.is_ascii_alphabetic())
        && trimmed.chars().all(|ch| {
            ch.is_ascii_uppercase() || ch.is_ascii_whitespace() || matches!(ch, '-' | '(' | ')')
        })
        && !trimmed.starts_with("ARTICLE")
}

fn is_preamble_text(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("we the people of the state of oregon")
}

fn article_toc_entries(lines: &[String]) -> Vec<TocEntry> {
    let mut entries = Vec::new();
    let toc_re =
        Regex::new(r"(?i)^Article\s+([IVXLCDM]+(?:-[A-Z]+(?:\([0-9]+\))?)?)\s+(.+)$").unwrap();
    for line in lines {
        if let Some(caps) = toc_re.captures(line) {
            let token = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            let title = caps
                .get(2)
                .map(|m| normalize_ws(m.as_str()))
                .unwrap_or_default();
            entries.push(TocEntry {
                article_key: article_key(&token.to_ascii_uppercase(), None),
                title,
            });
        }
    }
    entries
}

fn annotation_article_key(id: &str) -> String {
    match id.trim_start_matches('0').parse::<u32>().unwrap_or(1) {
        1 => "article-i".to_string(),
        2 => "article-ii".to_string(),
        3 => "article-iii".to_string(),
        4 => "article-iv".to_string(),
        5 => "article-v".to_string(),
        6 => "article-vi".to_string(),
        7 => "article-vii".to_string(),
        8 => "article-viii".to_string(),
        9 => "article-ix".to_string(),
        10 => "article-x".to_string(),
        11 => "article-xi".to_string(),
        12 => "article-xii".to_string(),
        14 => "article-xiv".to_string(),
        15 => "article-xv".to_string(),
        16 => "article-xvi".to_string(),
        17 => "article-xvii".to_string(),
        18 => "article-xviii".to_string(),
        other => format!("article-{other}"),
    }
}

fn article_key_from_annotation_url(url: &str) -> Option<String> {
    ANNOTATION_LINK_RE
        .captures(url)
        .and_then(|caps| caps.get(1).map(|m| annotation_article_key(m.as_str())))
}

fn article_label_from_key(key: &str) -> String {
    let roman = key
        .strip_prefix("article-")
        .unwrap_or(key)
        .replace("-amended", "")
        .replace("-original", "")
        .split('-')
        .map(|part| {
            if part.chars().all(|ch| ch.is_ascii_digit()) {
                format!("({part})")
            } else {
                part.to_ascii_uppercase()
            }
        })
        .collect::<Vec<_>>()
        .join("-");
    format!("Article {roman}")
}

fn article_key(token: &str, variant: Option<&str>) -> String {
    let mut key = format!("article-{}", clean_article_token(token));
    if let Some(variant) = variant {
        key.push('-');
        key.push_str(&clean_id_part(variant));
    }
    key
}

fn clean_article_token(token: &str) -> String {
    token
        .trim()
        .to_ascii_lowercase()
        .replace('(', "-")
        .replace(')', "")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .replace("--", "-")
}

fn article_citation(article_key: &str) -> String {
    let label = article_label_from_key(article_key);
    format!("Or. Const. art. {}", label.trim_start_matches("Article "))
}

fn section_canonical_id(article_key: &str, section_number: &str) -> String {
    format!(
        "{OR_CONSTITUTION_CORPUS_ID}:{article_key}:section-{}",
        clean_id_part(section_number)
    )
}

fn section_citation(article_label: &str, section_number: &str) -> String {
    format!(
        "Or. Const. art. {}, {SECTION_SYMBOL} {section_number}",
        article_label.trim_start_matches("Article ")
    )
}

fn edition_id(edition_year: i32) -> String {
    format!("{OR_CONSTITUTION_CORPUS_ID}@{edition_year}")
}

fn infer_proposal_method(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    if lower.contains("initiative petition") {
        "initiative_petition".to_string()
    } else if lower.contains("h.j.r.") || lower.contains("s.j.r.") {
        "legislative_resolution".to_string()
    } else {
        "source_note".to_string()
    }
}

fn parse_resolution(value: &str) -> Option<(String, String)> {
    let caps = Regex::new(r"(?i)\b([HS])\.J\.R\.\s*([0-9A-Z.-]+)")
        .ok()?
        .captures(value)?;
    let chamber = match caps.get(1)?.as_str().to_ascii_uppercase().as_str() {
        "H" => "house",
        "S" => "senate",
        _ => "unknown",
    };
    Some((chamber.to_string(), caps.get(2)?.as_str().to_string()))
}

fn parse_measure_number(value: &str) -> Option<String> {
    Regex::new(r"(?i)\bMeasure\s+(?:No\.\s*)?([0-9A-Z.-]+)")
        .ok()?
        .captures(value)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

fn parse_filed_date(value: &str) -> Option<String> {
    Regex::new(r"(?i)\bfiled\s+([^;,\]]+?[0-9]{4})")
        .ok()?
        .captures(value)
        .and_then(|caps| caps.get(1).map(|m| normalize_ws(m.as_str())))
}

fn first_year(value: &str) -> Option<i32> {
    Regex::new(r"\b(18|19|20)[0-9]{2}\b")
        .ok()?
        .find(value)
        .and_then(|m| m.as_str().parse::<i32>().ok())
}

fn html_lines(document: &Html) -> Vec<String> {
    let selector = Selector::parse("h1, h2, h3, p, li").expect("valid html text selector");
    document
        .select(&selector)
        .map(|element| normalize_constitution_text(&element.text().collect::<Vec<_>>().join(" ")))
        .filter(|line| !line.is_empty())
        .collect()
}

fn normalized_body_text(document: &Html) -> String {
    html_lines(document).join("\n")
}

fn html_to_text(html: &str) -> String {
    let document = Html::parse_document(html);
    normalized_body_text(&document)
}

fn decode_html(bytes: &[u8]) -> String {
    let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    cow.to_string()
}

fn normalize_constitution_text(value: &str) -> String {
    normalize_ws(&value.replace('\u{00a0}', " ").replace('\u{fffd}', " "))
}

fn normalize_ws(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn clean_id_part(value: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '.' | '-' | '_' | ' ' | '/' | '(' | ')') {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn artifact(url: &str, item_id: &str, html: &str) -> RawArtifact {
        let bytes = html.as_bytes().to_vec();
        RawArtifact {
            metadata: ArtifactMetadata {
                artifact_id: "artifact:test".to_string(),
                source_id: SOURCE_ID.to_string(),
                item_id: item_id.to_string(),
                url: url.to_string(),
                path: "test.html".to_string(),
                content_type: Some("text/html".to_string()),
                etag: None,
                last_modified: None,
                retrieved_at: Utc.with_ymd_and_hms(2026, 1, 9, 0, 0, 0).unwrap(),
                raw_hash: sha256_hex(&bytes),
                byte_len: bytes.len(),
                status: "fixture".to_string(),
                skipped: false,
            },
            bytes,
        }
    }

    #[test]
    fn parses_primary_text_history_and_subsections() {
        let html = r#"
          <p>ARTICLE I</p>
          <p>BILL OF RIGHTS</p>
          <p>Sec. 1. Natural rights inherent in people</p>
          <p>Section 1. Natural rights inherent in people. We declare that all power is inherent in the people. [Constitution of 1859]</p>
          <p>Section 41. Work programs. (1) All inmates shall be actively engaged in work.</p>
          <p>(2) The Legislative Assembly shall provide by law. [Created through initiative petition filed Jan. 12, 1994, and adopted by the people Nov. 8, 1994; Amendment proposed by H.J.R. 2, 1997, and adopted by the people May 20, 1997]</p>
          <p>Note: An initiative petition (Measure No. 40, 1996) was declared void for not being enacted in compliance with section 1, Article XVII of this Constitution.</p>
          <p>ARTICLE VII (Amended)</p>
          <p>Section 2. Courts. The courts shall continue.</p>
        "#;
        let batch =
            parse_constitution_html(&artifact(CONSTITUTION_URL, "constitution-text", html), 2026)
                .unwrap();
        let provisions = batch.files.get("provisions.jsonl").unwrap();
        assert!(provisions.iter().any(|row| {
            row.get("provision_id").and_then(|value| value.as_str())
                == Some("or:constitution:article-i:section-41")
        }));
        assert!(provisions.iter().any(|row| {
            row.get("provision_id").and_then(|value| value.as_str())
                == Some("or:constitution:article-i:section-41:1")
        }));
        assert!(provisions.iter().any(|row| {
            row.get("provision_id").and_then(|value| value.as_str())
                == Some("or:constitution:article-vii-amended:section-2")
        }));
        assert!(
            batch
                .files
                .get("amendments.jsonl")
                .map(|rows| rows.len() >= 2)
                .unwrap_or(false)
        );
        assert!(
            batch
                .files
                .get("status_events.jsonl")
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
        );
    }

    #[test]
    fn parses_preamble_and_annotation_commentary() {
        let preamble = r#"
          <p>PREAMBLE</p>
          <p>We the people of the State of Oregon to the end that Justice be established, order maintained, and liberty perpetuated, do ordain this Constitution.</p>
        "#;
        let batch = parse_preamble_html(
            &artifact(PREAMBLE_URL, "constitution-preamble", preamble),
            2026,
        )
        .unwrap();
        assert!(
            batch
                .files
                .get("provisions.jsonl")
                .unwrap()
                .iter()
                .any(|row| {
                    row.get("provision_id").and_then(|value| value.as_str())
                        == Some("or:constitution:preamble")
                })
        );

        let annotations = r#"
          <h1>Oregon Constitution Annotations</h1>
          <p>Article XVII</p>
          <p>Section 1</p>
          <p>NOTES OF DECISIONS</p>
          <p>Requirement that two or more amendments must be voted upon separately is applicable to amendments submitted by initiative petition. Armatta v. Kitzhaber, 327 Or 250, 959 P2d 49 (1998)</p>
          <p>LAW REVIEW CITATIONS: 87 OLR 717 (2008)</p>
        "#;
        let batch = parse_annotation_html(
            &artifact(
                "https://www.oregonlegislature.gov/bills_laws/ors/anc017.html",
                "constitution-annotation-article-017",
                annotations,
            ),
            2026,
        )
        .unwrap();
        assert!(
            batch
                .files
                .get("commentaries.jsonl")
                .unwrap()
                .iter()
                .any(|row| {
                    row.get("target_canonical_id")
                        .and_then(|value| value.as_str())
                        == Some("or:constitution:article-xvii:section-1")
                })
        );
        assert!(
            batch
                .files
                .get("external_legal_citations.jsonl")
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
        );
    }
}
