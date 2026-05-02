use crate::artifact_store::{ArtifactMetadata, RawArtifact};
use crate::authority_taxonomy::{
    AUTHORITY_LEVEL_OFFICIAL_COMMENTARY, AUTHORITY_LEVEL_US_CONSTITUTION,
};
use crate::connectors::{ConnectorOptions, DataConnector, SourceItem};
use crate::graph_batch::GraphBatch;
use crate::hash::{sha256_hex, stable_id};
use crate::models::{
    Commentary, CorpusEdition, ExternalLegalCitation, Jurisdiction, LegalCorpus, LegalTextIdentity,
    LegalTextVersion, ParserDiagnostic, Provision, RetrievalChunk, SourceDocument, SourcePage,
};
use crate::source_qc::{QcReport, qc_source_batch};
use crate::source_registry::{SourceKind, SourceRegistryEntry};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::{BTreeMap, BTreeSet};

const CONSTITUTION_SOURCE_ID: &str = "congress_gov_us_constitution";
const CONAN_SOURCE_ID: &str = "congress_gov_constitution_annotated";
const CONSTITUTION_URL: &str = "https://constitution.congress.gov/constitution/";
const CONSTITUTION_LITERAL_PRINT_URL: &str =
    "https://constitution.congress.gov/static/files/Literal_Print_of_Constitution_MCT_1.9.26.pdf";
const CONAN_BROWSE_URL: &str = "https://constitution.congress.gov/browse/";
const US_CONSTITUTION_CORPUS_ID: &str = "us:constitution";
const CONAN_CORPUS_ID: &str = "us:conan";
const US_JURISDICTION_ID: &str = "us";
const CONGRESS_PARSER_PROFILE: &str = "congress_constitution_html_v1";
const CONAN_PARSER_PROFILE: &str = "congress_conan_html_v1";

static SERIAL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b((?:Intro|Pre|Appx|Amdt\d+|Art[IVXLCDM]+|Art\d+)(?:\.[A-Za-z0-9]+)+)\b")
        .expect("valid CONAN serial regex")
});
static CASE_CITATION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b\d{1,3}\s+U\.S\.\s+\d{1,4}\b").expect("valid case regex"));

#[derive(Debug, Clone)]
pub struct CongressConstitutionConnector {
    entry: SourceRegistryEntry,
    options: ConnectorOptions,
}

impl CongressConstitutionConnector {
    pub fn new(entry: SourceRegistryEntry, options: ConnectorOptions) -> Self {
        Self { entry, options }
    }
}

#[async_trait]
impl DataConnector for CongressConstitutionConnector {
    fn source_id(&self) -> &'static str {
        match self.entry.source_id.as_str() {
            CONSTITUTION_SOURCE_ID => CONSTITUTION_SOURCE_ID,
            CONAN_SOURCE_ID => CONAN_SOURCE_ID,
            _ => "congress_constitution",
        }
    }

    fn source_kind(&self) -> SourceKind {
        self.entry.source_type
    }

    async fn discover(&self) -> Result<Vec<SourceItem>> {
        if self.entry.source_id == CONSTITUTION_SOURCE_ID {
            return Ok(vec![
                source_item(
                    "constitution-full",
                    CONSTITUTION_URL,
                    "U.S. Constitution",
                    "text/html",
                ),
                source_item(
                    "constitution-literal-print-pdf",
                    CONSTITUTION_LITERAL_PRINT_URL,
                    "U.S. Constitution Literal Print PDF",
                    "application/pdf",
                ),
            ]);
        }

        let mut items = conan_browse_seed_items();
        let discovered = discover_conan_essay_items(self.options.max_items).await;
        for item in discovered {
            if !items
                .iter()
                .any(|existing| existing.item_id == item.item_id)
            {
                items.push(item);
            }
        }
        Ok(items)
    }

    async fn parse(&self, artifact: &RawArtifact) -> Result<GraphBatch> {
        if self.entry.source_id == CONSTITUTION_SOURCE_ID {
            if artifact
                .metadata
                .content_type
                .as_deref()
                .is_some_and(|content_type| content_type.contains("pdf"))
                || artifact.metadata.url.to_ascii_lowercase().ends_with(".pdf")
            {
                parse_constitution_pdf_fallback(artifact)
            } else {
                parse_constitution_html(artifact, self.options.edition_year)
            }
        } else {
            parse_conan_html(artifact, self.options.edition_year)
        }
    }

    async fn qc(&self, artifacts: &[ArtifactMetadata], batch: &GraphBatch) -> Result<QcReport> {
        Ok(qc_source_batch(&self.entry, artifacts, batch))
    }
}

fn source_item(item_id: &str, url: &str, title: &str, content_type: &str) -> SourceItem {
    SourceItem {
        item_id: item_id.to_string(),
        url: Some(url.to_string()),
        title: Some(title.to_string()),
        content_type: Some(content_type.to_string()),
        metadata: BTreeMap::new(),
    }
}

fn conan_browse_seed_items() -> Vec<SourceItem> {
    let mut items = vec![
        source_item("browse-root", CONAN_BROWSE_URL, "CONAN Browse", "text/html"),
        source_item(
            "browse-introduction",
            "https://constitution.congress.gov/browse/introduction/",
            "CONAN Introduction",
            "text/html",
        ),
        source_item(
            "browse-preamble",
            "https://constitution.congress.gov/browse/preamble/",
            "CONAN Preamble",
            "text/html",
        ),
        source_item(
            "browse-appendix",
            "https://constitution.congress.gov/browse/appendix/",
            "CONAN Appendix",
            "text/html",
        ),
    ];
    for article in 1..=7 {
        items.push(source_item(
            &format!("browse-article-{article}"),
            &format!("https://constitution.congress.gov/browse/article-{article}/"),
            &format!("CONAN Article {article}"),
            "text/html",
        ));
    }
    for amendment in 1..=27 {
        items.push(source_item(
            &format!("browse-amendment-{amendment}"),
            &format!("https://constitution.congress.gov/browse/amendment-{amendment}/"),
            &format!("CONAN Amendment {amendment}"),
            "text/html",
        ));
    }
    items
}

async fn discover_conan_essay_items(max_items: usize) -> Vec<SourceItem> {
    if max_items > 0 && max_items <= conan_browse_seed_items().len() {
        return Vec::new();
    }

    let client = match reqwest::Client::builder()
        .user_agent("ORSGraph source registry crawler (official CONAN connector)")
        .build()
    {
        Ok(client) => client,
        Err(_) => return Vec::new(),
    };
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for item in conan_browse_seed_items() {
        let Some(url) = item.url.as_deref() else {
            continue;
        };
        let Ok(response) = client.get(url).send().await else {
            continue;
        };
        let Ok(html) = response.text().await else {
            continue;
        };
        for (serial, href, title) in extract_conan_links(&html, url) {
            if seen.insert(serial.clone()) {
                items.push(source_item(
                    &format!("essay-{}", safe_id_part(&serial)),
                    &href,
                    &title,
                    "text/html",
                ));
            }
        }
    }
    items
}

fn parse_constitution_pdf_fallback(artifact: &RawArtifact) -> Result<GraphBatch> {
    let mut batch = GraphBatch::default();
    append_base_corpora(&mut batch, artifact.metadata.retrieved_at.to_rfc3339())?;
    let doc = source_document(
        artifact,
        CONSTITUTION_SOURCE_ID,
        US_CONSTITUTION_CORPUS_ID,
        "us:constitution@current",
        "USCONST",
        "constitution",
        "U.S. Constitution Literal Print PDF",
        CONGRESS_PARSER_PROFILE,
        Some("literal_print_pdf"),
    );
    let source_page = SourcePage {
        source_page_id: format!("{}:page:1", doc.source_document_id),
        source_document_id: doc.source_document_id.clone(),
        page_number: 1,
        printed_label: Some("PDF".to_string()),
        text: "U.S. Constitution Literal Print PDF fallback source.".to_string(),
        normalized_text: "U.S. Constitution Literal Print PDF fallback source.".to_string(),
        text_hash: sha256_hex("U.S. Constitution Literal Print PDF fallback source."),
    };
    batch.push("source_documents.jsonl", &doc)?;
    batch.push("source_pages.jsonl", &source_page)?;
    Ok(batch)
}

fn parse_constitution_html(artifact: &RawArtifact, edition_year: i32) -> Result<GraphBatch> {
    let html = decode_html(&artifact.bytes);
    let document = Html::parse_document(&html);
    let mut batch = GraphBatch::default();
    append_base_corpora(&mut batch, artifact.metadata.retrieved_at.to_rfc3339())?;

    let source_doc = source_document(
        artifact,
        CONSTITUTION_SOURCE_ID,
        US_CONSTITUTION_CORPUS_ID,
        "us:constitution@current",
        "USCONST",
        "constitution",
        "Constitution of the United States",
        CONGRESS_PARSER_PROFILE,
        Some("primary_law"),
    );
    let source_document_id = source_doc.source_document_id.clone();
    let normalized_page_text = normalized_body_text(&document);
    batch.push("source_documents.jsonl", &source_doc)?;
    batch.push(
        "source_pages.jsonl",
        &SourcePage {
            source_page_id: format!("{source_document_id}:page:1"),
            source_document_id: source_document_id.clone(),
            page_number: 1,
            printed_label: Some("HTML".to_string()),
            text: normalized_page_text.clone(),
            normalized_text: normalized_page_text.clone(),
            text_hash: sha256_hex(&normalized_page_text),
        },
    )?;

    let mut drafts = BTreeMap::<String, ConstitutionDraft>::new();
    let selector = Selector::parse("h1, h2, h3, p").expect("valid selector");
    let mut current_article: Option<u32> = None;
    let mut current_amendment: Option<u32> = None;
    let mut current_section: Option<u32> = None;
    let mut in_preamble = false;
    let mut article_one_section_eight_clause = 0usize;

    for element in document.select(&selector) {
        let name = element.value().name();
        let text = normalize_ws(&element.text().collect::<Vec<_>>().join(" "));
        if text.is_empty() || text.ends_with(" Explained") {
            continue;
        }
        match name {
            "h2" => {
                if text.to_ascii_lowercase().contains("preamble") {
                    in_preamble = true;
                    current_article = None;
                    current_amendment = None;
                    current_section = None;
                } else if let Some(number) = parse_numbered_heading(&text, "Article") {
                    in_preamble = false;
                    current_article = Some(number);
                    current_amendment = None;
                    current_section = None;
                    article_one_section_eight_clause = 0;
                } else if let Some(number) = parse_numbered_heading(&text, "Amendment") {
                    in_preamble = false;
                    current_article = None;
                    current_amendment = Some(number);
                    current_section = None;
                    article_one_section_eight_clause = 0;
                }
            }
            "h3" => {
                if let Some(number) = parse_numbered_heading(&text, "Section") {
                    current_section = Some(number);
                    article_one_section_eight_clause = 0;
                }
            }
            "p" => {
                if in_preamble {
                    add_draft_text(&mut drafts, constitution_preamble_draft(), &text);
                } else if let Some(article) = current_article {
                    if let Some(section) = current_section {
                        let section_draft = constitution_article_section_draft(article, section);
                        add_draft_text(&mut drafts, section_draft, &text);
                        if article == 1 && section == 8 {
                            article_one_section_eight_clause += 1;
                            add_draft_text(
                                &mut drafts,
                                constitution_article_clause_draft(
                                    article,
                                    section,
                                    article_one_section_eight_clause,
                                ),
                                &text,
                            );
                        }
                    } else {
                        add_draft_text(&mut drafts, constitution_article_draft(article), &text);
                    }
                } else if let Some(amendment) = current_amendment {
                    if let Some(section) = current_section {
                        add_draft_text(
                            &mut drafts,
                            constitution_amendment_section_draft(amendment, section),
                            &text,
                        );
                    } else {
                        add_draft_text(&mut drafts, constitution_amendment_draft(amendment), &text);
                    }
                }
            }
            _ => {}
        }
    }

    for (order, draft) in drafts.into_values().enumerate() {
        append_authority_rows(
            &mut batch,
            &draft,
            &source_document_id,
            edition_year,
            order + 1,
            US_CONSTITUTION_CORPUS_ID,
            "us:constitution@current",
            "USCONST",
            "constitution",
            AUTHORITY_LEVEL_US_CONSTITUTION,
            "primary_law",
        )?;
    }

    Ok(batch)
}

fn parse_conan_html(artifact: &RawArtifact, edition_year: i32) -> Result<GraphBatch> {
    let html = decode_html(&artifact.bytes);
    let document = Html::parse_document(&html);
    let mut batch = GraphBatch::default();
    append_base_corpora(&mut batch, artifact.metadata.retrieved_at.to_rfc3339())?;

    let title = first_heading(&document).unwrap_or_else(|| "Constitution Annotated".to_string());
    let source_doc = source_document(
        artifact,
        CONAN_SOURCE_ID,
        CONAN_CORPUS_ID,
        &format!("us:conan@{edition_year}"),
        "CONAN",
        "official_commentary",
        &title,
        CONAN_PARSER_PROFILE,
        Some("official_commentary"),
    );
    let source_document_id = source_doc.source_document_id.clone();
    let normalized_page_text = normalized_body_text(&document);
    batch.push("source_documents.jsonl", &source_doc)?;
    batch.push(
        "source_pages.jsonl",
        &SourcePage {
            source_page_id: format!("{source_document_id}:page:1"),
            source_document_id: source_document_id.clone(),
            page_number: 1,
            printed_label: Some("HTML".to_string()),
            text: normalized_page_text.clone(),
            normalized_text: normalized_page_text.clone(),
            text_hash: sha256_hex(&normalized_page_text),
        },
    )?;

    if artifact.metadata.url.contains("/browse/essay/") {
        let serial = serial_from_essay_page(&artifact.metadata.url, &normalized_page_text)
            .unwrap_or_else(|| format!("CONAN.{}", stable_id(&artifact.metadata.url)));
        let title = title_for_serial(&serial, &title);
        append_conan_commentary(
            &mut batch,
            &serial,
            &title,
            &normalized_page_text,
            &source_document_id,
            edition_year,
            1,
        )?;
        append_external_case_citations(&mut batch, &serial, &normalized_page_text)?;
    } else {
        let mut seen = BTreeSet::new();
        for (serial, href, title) in extract_conan_links(&html, &artifact.metadata.url) {
            if seen.insert(serial.clone()) {
                append_conan_commentary(
                    &mut batch,
                    &serial,
                    &title,
                    &format!("{title}\n{href}"),
                    &source_document_id,
                    edition_year,
                    seen.len(),
                )?;
            }
        }
        if seen.is_empty() {
            let diagnostic = ParserDiagnostic {
                parser_diagnostic_id: format!(
                    "diag:conan:{}",
                    stable_id(&format!("{}:no_serials", artifact.metadata.url))
                ),
                source_document_id,
                chapter: "conan".to_string(),
                edition_year,
                severity: "warning".to_string(),
                diagnostic_type: "conan_no_serials".to_string(),
                message: "CONAN browse page parsed with no essay serials.".to_string(),
                source_paragraph_order: None,
                related_id: None,
                parser_profile: CONAN_PARSER_PROFILE.to_string(),
            };
            batch.push("parser_diagnostics.jsonl", &diagnostic)?;
        }
    }
    Ok(batch)
}

fn append_base_corpora(batch: &mut GraphBatch, retrieved_at: String) -> Result<()> {
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
        "legal_corpora.jsonl",
        &LegalCorpus {
            corpus_id: US_CONSTITUTION_CORPUS_ID.to_string(),
            name: "Constitution of the United States".to_string(),
            short_name: "U.S. Constitution".to_string(),
            authority_family: "USCONST".to_string(),
            authority_type: "constitution".to_string(),
            jurisdiction_id: US_JURISDICTION_ID.to_string(),
        },
    )?;
    batch.push(
        "legal_corpora.jsonl",
        &LegalCorpus {
            corpus_id: CONAN_CORPUS_ID.to_string(),
            name: "Constitution Annotated".to_string(),
            short_name: "CONAN".to_string(),
            authority_family: "CONAN".to_string(),
            authority_type: "official_commentary".to_string(),
            jurisdiction_id: US_JURISDICTION_ID.to_string(),
        },
    )?;
    batch.push(
        "corpus_editions.jsonl",
        &CorpusEdition {
            edition_id: "us:constitution@current".to_string(),
            corpus_id: US_CONSTITUTION_CORPUS_ID.to_string(),
            edition_year: 1789,
            effective_date: None,
            source_label: Some(format!(
                "Congress.gov Constitution text retrieved {retrieved_at}"
            )),
            current: Some(true),
        },
    )?;
    batch.push(
        "corpus_editions.jsonl",
        &CorpusEdition {
            edition_id: "us:conan@current".to_string(),
            corpus_id: CONAN_CORPUS_ID.to_string(),
            edition_year: Utc::now().year_ce().1 as i32,
            effective_date: None,
            source_label: Some(format!("CONAN web edition retrieved {retrieved_at}")),
            current: Some(true),
        },
    )?;
    Ok(())
}

fn source_document(
    artifact: &RawArtifact,
    source_id: &str,
    corpus_id: &str,
    edition_id: &str,
    authority_family: &str,
    authority_type: &str,
    title: &str,
    parser_profile: &str,
    classification: Option<&str>,
) -> SourceDocument {
    let text = decode_html(&artifact.bytes);
    let normalized = normalize_ws(&html_to_text(&text));
    SourceDocument {
        source_document_id: format!("src:{source_id}:{}", stable_id(&artifact.metadata.url)),
        source_provider: "Library of Congress / Congress.gov".to_string(),
        source_kind: "static_html".to_string(),
        url: artifact.metadata.url.clone(),
        chapter: source_id.to_string(),
        corpus_id: Some(corpus_id.to_string()),
        edition_id: Some(edition_id.to_string()),
        authority_family: Some(authority_family.to_string()),
        authority_type: Some(authority_type.to_string()),
        title: Some(title.to_string()),
        source_type: classification.map(ToString::to_string),
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
        edition_year: artifact.metadata.retrieved_at.year_ce().1 as i32,
        html_encoding: Some("utf-8".to_string()),
        source_path: Some(artifact.metadata.path.clone()),
        paragraph_count: Some(normalized.lines().filter(|line| !line.is_empty()).count()),
        first_body_paragraph_index: Some(0),
        parser_profile: Some(parser_profile.to_string()),
        official_status: "official".to_string(),
        disclaimer_required: false,
        raw_hash: artifact.metadata.raw_hash.clone(),
        normalized_hash: sha256_hex(normalized.as_bytes()),
    }
}

fn append_authority_rows(
    batch: &mut GraphBatch,
    draft: &ConstitutionDraft,
    source_document_id: &str,
    edition_year: i32,
    order_index: usize,
    corpus_id: &str,
    edition_id: &str,
    authority_family: &str,
    authority_type: &str,
    authority_level: i32,
    source_role: &str,
) -> Result<()> {
    let text = draft.texts.join("\n\n");
    let normalized_text = normalize_ws(&text);
    let version_id = format!("{}@{edition_year}", draft.canonical_id);
    let provision_id = draft.canonical_id.clone();
    batch.push(
        "legal_text_identities.jsonl",
        &LegalTextIdentity {
            canonical_id: draft.canonical_id.clone(),
            citation: draft.citation.clone(),
            jurisdiction_id: US_JURISDICTION_ID.to_string(),
            authority_family: authority_family.to_string(),
            corpus_id: Some(corpus_id.to_string()),
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
            corpus_id: Some(corpus_id.to_string()),
            edition_id: Some(edition_id.to_string()),
            authority_family: Some(authority_family.to_string()),
            authority_type: Some(authority_type.to_string()),
            authority_level: Some(authority_level),
            effective_date: None,
            source_page_start: Some(1),
            source_page_end: Some(1),
            edition_year,
            status: "active".to_string(),
            status_text: Some("current".to_string()),
            text: text.clone(),
            text_hash: sha256_hex(normalized_text.as_bytes()),
            original_text: Some(text.clone()),
            paragraph_start_order: Some(order_index),
            paragraph_end_order: Some(order_index),
            source_paragraph_ids: Vec::new(),
            source_document_id: source_document_id.to_string(),
            official_status: "official".to_string(),
            disclaimer_required: false,
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
            provision_id: provision_id.clone(),
            version_id: version_id.clone(),
            canonical_id: draft.canonical_id.clone(),
            citation: draft.citation.clone(),
            display_citation: draft.citation.clone(),
            chapter: Some(draft.chapter.clone()),
            corpus_id: Some(corpus_id.to_string()),
            edition_id: Some(edition_id.to_string()),
            authority_family: Some(authority_family.to_string()),
            authority_type: Some(authority_type.to_string()),
            authority_level: Some(authority_level),
            effective_date: None,
            source_page_start: Some(1),
            source_page_end: Some(1),
            local_path: draft.local_path.clone(),
            provision_type: draft.provision_type.clone(),
            text: text.clone(),
            original_text: Some(text.clone()),
            normalized_text: normalized_text.clone(),
            order_index,
            depth: draft.local_path.len().max(1),
            text_hash: sha256_hex(normalized_text.as_bytes()),
            is_implied: false,
            is_definition_candidate: false,
            is_exception_candidate: false,
            is_deadline_candidate: false,
            is_penalty_candidate: false,
            paragraph_start_order: Some(order_index),
            paragraph_end_order: Some(order_index),
            source_paragraph_ids: Vec::new(),
            heading_path: draft.local_path.clone(),
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
        &text,
        &draft.title,
        Some(&provision_id),
        &version_id,
        &draft.canonical_id,
        &draft.citation,
        authority_level,
        authority_family,
        corpus_id,
        authority_type,
        Some(&draft.chapter),
        edition_year,
    )
}

fn append_conan_commentary(
    batch: &mut GraphBatch,
    serial: &str,
    title: &str,
    text: &str,
    source_document_id: &str,
    edition_year: i32,
    order_index: usize,
) -> Result<()> {
    let canonical_id = format!("us:conan:{serial}");
    let version_id = format!("{canonical_id}@{edition_year}");
    let normalized_text = normalize_ws(text);
    let target = target_for_conan_serial(serial);
    let chapter = conan_chapter(serial);
    append_authority_rows(
        batch,
        &ConstitutionDraft {
            canonical_id: canonical_id.clone(),
            citation: serial.to_string(),
            title: title.to_string(),
            chapter: chapter.clone(),
            provision_type: "official_commentary".to_string(),
            local_path: vec!["CONAN".to_string(), serial.to_string()],
            texts: vec![normalized_text.clone()],
        },
        source_document_id,
        edition_year,
        order_index,
        CONAN_CORPUS_ID,
        "us:conan@current",
        "CONAN",
        "official_commentary",
        AUTHORITY_LEVEL_OFFICIAL_COMMENTARY,
        "official_commentary",
    )?;
    batch.push(
        "commentaries.jsonl",
        &Commentary {
            commentary_id: canonical_id.clone(),
            source_document_id: source_document_id.to_string(),
            canonical_id: Some(canonical_id.clone()),
            version_id: Some(version_id.clone()),
            source_provision_id: None,
            target_canonical_id: target.clone(),
            target_provision_id: target,
            citation: Some(serial.to_string()),
            authority_family: Some("CONAN".to_string()),
            corpus_id: Some(CONAN_CORPUS_ID.to_string()),
            authority_level: Some(AUTHORITY_LEVEL_OFFICIAL_COMMENTARY),
            source_role: Some("official_commentary".to_string()),
            commentary_type: "constitution_annotated_analysis".to_string(),
            text: normalized_text.clone(),
            normalized_text,
            source_page_start: Some(1),
            source_page_end: Some(1),
            confidence: 0.92,
            extraction_method: CONAN_PARSER_PROFILE.to_string(),
        },
    )?;
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
    authority_family: &str,
    corpus_id: &str,
    authority_type: &str,
    chapter: Option<&str>,
    edition_year: i32,
) -> Result<()> {
    let normalized_text = normalize_ws(text);
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
            jurisdiction_id: US_JURISDICTION_ID.to_string(),
            authority_level,
            authority_family: Some(authority_family.to_string()),
            corpus_id: Some(corpus_id.to_string()),
            authority_type: Some(authority_type.to_string()),
            effective_date: None,
            chapter: chapter.map(ToString::to_string),
            source_page_start: Some(1),
            source_page_end: Some(1),
            edition_year,
            embedding_model: None,
            embedding_dim: None,
            embedding: None,
            embedding_input_hash: sha256_hex(normalized_text.as_bytes()),
            embedding_policy: Some("source_backed".to_string()),
            answer_policy: Some("cite_source".to_string()),
            chunk_schema_version: Some("1.0.0".to_string()),
            retrieval_profile: Some("legal_authority_v1".to_string()),
            search_weight: Some(if authority_family == "USCONST" {
                2.0
            } else {
                0.9
            }),
            embedding_input_type: Some("legal_text".to_string()),
            embedding_output_dtype: None,
            embedded_at: None,
            source_kind: Some("official".to_string()),
            source_id: Some(if authority_family == "USCONST" {
                CONSTITUTION_SOURCE_ID.to_string()
            } else {
                CONAN_SOURCE_ID.to_string()
            }),
            token_count: Some(normalized_text.split_whitespace().count()),
            max_tokens: None,
            context_window: None,
            chunking_strategy: Some("provision_or_essay".to_string()),
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

fn append_external_case_citations(batch: &mut GraphBatch, serial: &str, text: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for caps in CASE_CITATION_RE.captures_iter(text) {
        let Some(raw) = caps.get(0).map(|m| m.as_str()) else {
            continue;
        };
        let normalized = normalize_ws(raw);
        if seen.insert(normalized.clone()) {
            batch.push(
                "external_legal_citations.jsonl",
                &ExternalLegalCitation {
                    external_citation_id: format!(
                        "external:conan:{}:{}",
                        safe_id_part(serial),
                        stable_id(&normalized)
                    ),
                    citation: raw.to_string(),
                    normalized_citation: normalized,
                    citation_type: "case_law".to_string(),
                    jurisdiction_id: US_JURISDICTION_ID.to_string(),
                    source_system: CONAN_PARSER_PROFILE.to_string(),
                    url: None,
                },
            )?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ConstitutionDraft {
    canonical_id: String,
    citation: String,
    title: String,
    chapter: String,
    provision_type: String,
    local_path: Vec<String>,
    texts: Vec<String>,
}

fn add_draft_text(
    drafts: &mut BTreeMap<String, ConstitutionDraft>,
    mut draft: ConstitutionDraft,
    text: &str,
) {
    drafts
        .entry(draft.canonical_id.clone())
        .and_modify(|existing| existing.texts.push(text.to_string()))
        .or_insert_with(|| {
            draft.texts.push(text.to_string());
            draft
        });
}

fn constitution_preamble_draft() -> ConstitutionDraft {
    ConstitutionDraft {
        canonical_id: "us:constitution:preamble".to_string(),
        citation: "U.S. Const. pmbl.".to_string(),
        title: "Preamble".to_string(),
        chapter: "preamble".to_string(),
        provision_type: "preamble".to_string(),
        local_path: vec!["Preamble".to_string()],
        texts: Vec::new(),
    }
}

fn constitution_article_draft(article: u32) -> ConstitutionDraft {
    let roman = to_roman(article);
    ConstitutionDraft {
        canonical_id: format!("us:constitution:article-{article}"),
        citation: format!("U.S. Const. art. {roman}"),
        title: format!("Article {roman}"),
        chapter: format!("article-{article}"),
        provision_type: "article".to_string(),
        local_path: vec![format!("Article {roman}")],
        texts: Vec::new(),
    }
}

fn constitution_article_section_draft(article: u32, section: u32) -> ConstitutionDraft {
    let roman = to_roman(article);
    ConstitutionDraft {
        canonical_id: format!("us:constitution:article-{article}:section-{section}"),
        citation: format!("U.S. Const. art. {roman}, § {section}"),
        title: format!("Article {roman}, Section {section}"),
        chapter: format!("article-{article}"),
        provision_type: "section".to_string(),
        local_path: vec![format!("Article {roman}"), format!("Section {section}")],
        texts: Vec::new(),
    }
}

fn constitution_article_clause_draft(
    article: u32,
    section: u32,
    clause: usize,
) -> ConstitutionDraft {
    let roman = to_roman(article);
    ConstitutionDraft {
        canonical_id: format!(
            "us:constitution:article-{article}:section-{section}:clause-{clause}"
        ),
        citation: format!("U.S. Const. art. {roman}, § {section}, cl. {clause}"),
        title: format!("Article {roman}, Section {section}, Clause {clause}"),
        chapter: format!("article-{article}"),
        provision_type: "clause".to_string(),
        local_path: vec![
            format!("Article {roman}"),
            format!("Section {section}"),
            format!("Clause {clause}"),
        ],
        texts: Vec::new(),
    }
}

fn constitution_amendment_draft(amendment: u32) -> ConstitutionDraft {
    let roman = to_roman(amendment);
    ConstitutionDraft {
        canonical_id: format!("us:constitution:amendment-{amendment}"),
        citation: format!("U.S. Const. amend. {roman}"),
        title: format!("Amendment {roman}"),
        chapter: format!("amendment-{amendment}"),
        provision_type: "amendment".to_string(),
        local_path: vec![format!("Amendment {roman}")],
        texts: Vec::new(),
    }
}

fn constitution_amendment_section_draft(amendment: u32, section: u32) -> ConstitutionDraft {
    let roman = to_roman(amendment);
    ConstitutionDraft {
        canonical_id: format!("us:constitution:amendment-{amendment}:section-{section}"),
        citation: format!("U.S. Const. amend. {roman}, § {section}"),
        title: format!("Amendment {roman}, Section {section}"),
        chapter: format!("amendment-{amendment}"),
        provision_type: "section".to_string(),
        local_path: vec![format!("Amendment {roman}"), format!("Section {section}")],
        texts: Vec::new(),
    }
}

fn extract_conan_links(html: &str, base_url: &str) -> Vec<(String, String, String)> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a").expect("valid selector");
    let mut out = Vec::new();
    for anchor in document.select(&selector) {
        let text = normalize_ws(&anchor.text().collect::<Vec<_>>().join(" "));
        let href = anchor.value().attr("href").unwrap_or_default();
        let serial = SERIAL_RE
            .captures(&text)
            .and_then(|caps| caps.get(1).map(|m| normalize_conan_serial(m.as_str())))
            .or_else(|| serial_from_href(href));
        let Some(serial) = serial else {
            continue;
        };
        let title = text
            .strip_prefix(&serial)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&text);
        out.push((serial, absolute_url(base_url, href), title.to_string()));
    }
    out
}

fn target_for_conan_serial(serial: &str) -> Option<String> {
    let parts = serial.split('.').collect::<Vec<_>>();
    let head = parts.first()?.to_ascii_lowercase();
    if head.starts_with("pre") {
        return Some("us:constitution:preamble".to_string());
    }
    if let Some(amendment) = head.strip_prefix("amdt") {
        let mut target = format!("us:constitution:amendment-{amendment}");
        if let Some(section) = parts.iter().find_map(|part| {
            part.strip_prefix('S')
                .or_else(|| part.strip_prefix('s'))
                .and_then(|value| value.parse::<u32>().ok())
        }) {
            target.push_str(&format!(":section-{section}"));
        }
        return Some(target);
    }
    if let Some(article) = head.strip_prefix("art") {
        let article = roman_or_decimal_to_u32(article).unwrap_or(0);
        if article == 0 {
            return None;
        }
        let mut target = format!("us:constitution:article-{article}");
        if let Some(section) = parts.iter().find_map(|part| {
            part.strip_prefix('S')
                .or_else(|| part.strip_prefix('s'))
                .and_then(|value| value.parse::<u32>().ok())
        }) {
            target.push_str(&format!(":section-{section}"));
        }
        if let Some(clause) = parts.iter().find_map(|part| {
            part.strip_prefix('C')
                .or_else(|| part.strip_prefix('c'))
                .and_then(|value| value.parse::<u32>().ok())
        }) {
            target.push_str(&format!(":clause-{clause}"));
        }
        return Some(target);
    }
    None
}

fn conan_chapter(serial: &str) -> String {
    target_for_conan_serial(serial)
        .and_then(|target| {
            target
                .split(':')
                .find(|part| part.starts_with("article-") || part.starts_with("amendment-"))
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "conan".to_string())
}

fn serial_from_essay_page(url: &str, text: &str) -> Option<String> {
    SERIAL_RE
        .captures(text)
        .and_then(|caps| caps.get(1).map(|m| normalize_conan_serial(m.as_str())))
        .or_else(|| serial_from_href(url))
}

fn serial_from_href(href: &str) -> Option<String> {
    let lower = href.to_ascii_lowercase();
    let marker = "/browse/essay/";
    let start = lower.find(marker)? + marker.len();
    let slug = href.get(start..)?.split('/').next()?.trim();
    serial_from_slug(slug)
}

fn serial_from_slug(slug: &str) -> Option<String> {
    let mut parts = slug.split('-').filter(|part| !part.is_empty());
    let head = parts.next()?;
    let mut out = normalize_conan_serial(head);
    for part in parts {
        out.push('.');
        out.push_str(&part.to_ascii_uppercase());
    }
    Some(out)
}

fn normalize_conan_serial(value: &str) -> String {
    let mut out = String::new();
    for (index, part) in value.split('.').filter(|part| !part.is_empty()).enumerate() {
        if index > 0 {
            out.push('.');
        }
        let lower = part.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("amdt") {
            out.push_str("Amdt");
            out.push_str(rest);
        } else if let Some(rest) = lower.strip_prefix("art") {
            out.push_str("Art");
            out.push_str(&rest.to_ascii_uppercase());
        } else if let Some(rest) = lower.strip_prefix("intro") {
            out.push_str("Intro");
            out.push_str(rest);
        } else if let Some(rest) = lower.strip_prefix("appx") {
            out.push_str("Appx");
            out.push_str(rest);
        } else if let Some(rest) = lower.strip_prefix("pre") {
            out.push_str("Pre");
            out.push_str(rest);
        } else {
            out.push_str(&part.to_ascii_uppercase());
        }
    }
    out
}

fn title_for_serial(serial: &str, fallback: &str) -> String {
    if fallback
        .to_ascii_lowercase()
        .contains(&serial.to_ascii_lowercase())
    {
        fallback.to_string()
    } else {
        format!("{serial} {fallback}")
    }
}

fn absolute_url(base_url: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with('/') {
        format!("https://constitution.congress.gov{href}")
    } else {
        let base = base_url.trim_end_matches('/');
        format!("{base}/{href}")
    }
}

fn first_heading(document: &Html) -> Option<String> {
    let selector = Selector::parse("h1").expect("valid selector");
    document
        .select(&selector)
        .next()
        .map(|heading| normalize_ws(&heading.text().collect::<Vec<_>>().join(" ")))
        .filter(|value| !value.is_empty())
}

fn normalized_body_text(document: &Html) -> String {
    let selector = Selector::parse("h1, h2, h3, p, li").expect("valid selector");
    document
        .select(&selector)
        .map(|node| normalize_ws(&node.text().collect::<Vec<_>>().join(" ")))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn html_to_text(html: &str) -> String {
    let document = Html::parse_document(html);
    normalized_body_text(&document)
}

fn decode_html(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn normalize_ws(value: &str) -> String {
    html_escape::decode_html_entities(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn safe_id_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn parse_numbered_heading(text: &str, prefix: &str) -> Option<u32> {
    let lower = text.to_ascii_lowercase();
    if !lower.starts_with(&prefix.to_ascii_lowercase()) {
        return None;
    }
    let tail = text
        .get(prefix.len()..)?
        .trim_matches(|ch: char| ch.is_whitespace() || ch == '.' || ch == ':' || ch == '-');
    let number = tail.split_whitespace().next().unwrap_or(tail);
    roman_or_decimal_to_u32(number)
}

fn roman_or_decimal_to_u32(value: &str) -> Option<u32> {
    value.parse::<u32>().ok().or_else(|| roman_to_u32(value))
}

fn roman_to_u32(value: &str) -> Option<u32> {
    let mut total = 0;
    let mut previous = 0;
    for ch in value.trim().to_ascii_uppercase().chars().rev() {
        let current = match ch {
            'I' => 1,
            'V' => 5,
            'X' => 10,
            'L' => 50,
            'C' => 100,
            'D' => 500,
            'M' => 1000,
            _ => return None,
        };
        if current < previous {
            total -= current;
        } else {
            total += current;
            previous = current;
        }
    }
    (total > 0).then_some(total as u32)
}

fn to_roman(mut value: u32) -> String {
    let pairs = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (number, roman) in pairs {
        while value >= number {
            out.push_str(roman);
            value -= number;
        }
    }
    out
}

trait DateParts {
    fn year_ce(&self) -> (bool, u32);
}

impl DateParts for chrono::DateTime<Utc> {
    fn year_ce(&self) -> (bool, u32) {
        use chrono::Datelike;
        self.date_naive().year_ce()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn artifact(url: &str, html: &str) -> RawArtifact {
        let bytes = html.as_bytes().to_vec();
        RawArtifact {
            metadata: ArtifactMetadata {
                artifact_id: "artifact:test".to_string(),
                source_id: CONSTITUTION_SOURCE_ID.to_string(),
                item_id: "test".to_string(),
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
    fn constitution_parser_emits_stable_ids() {
        let html = r#"
          <h1>Constitution of the United States</h1>
          <h2>The Preamble</h2>
          <p>We the People of the United States...</p>
          <h2>Article I</h2>
          <h3>Section 8</h3>
          <p>The Congress shall have Power To lay and collect Taxes.</p>
          <p>To borrow Money on the credit of the United States;</p>
          <p>To regulate Commerce with foreign Nations, and among the several States, and with the Indian Tribes;</p>
          <h2>Amendment XIV</h2>
          <h3>Section 1</h3>
          <p>No State shall deprive any person of life, liberty, or property, without due process of law.</p>
        "#;
        let batch = parse_constitution_html(&artifact(CONSTITUTION_URL, html), 2026).unwrap();
        let provisions = batch.files.get("provisions.jsonl").unwrap();
        assert!(provisions.iter().any(|row| {
            row.get("provision_id").and_then(|value| value.as_str())
                == Some("us:constitution:preamble")
        }));
        assert!(provisions.iter().any(|row| {
            row.get("provision_id").and_then(|value| value.as_str())
                == Some("us:constitution:article-1:section-8:clause-3")
        }));
        assert!(provisions.iter().any(|row| {
            row.get("provision_id").and_then(|value| value.as_str())
                == Some("us:constitution:amendment-14:section-1")
        }));
    }

    #[test]
    fn conan_parser_links_commentary_to_constitution_target() {
        let html = r#"
          <h1>Browse the Constitution Annotated</h1>
          <a href="/browse/essay/amdt14-S1-5-1/ALDE_00000001/">Amdt14.S1.5.1 Overview of Procedural Due Process</a>
        "#;
        let batch = parse_conan_html(
            &artifact(
                "https://constitution.congress.gov/browse/amendment-14/",
                html,
            ),
            2026,
        )
        .unwrap();
        let commentaries = batch.files.get("commentaries.jsonl").unwrap();
        assert!(commentaries.iter().any(|row| {
            row.get("commentary_id").and_then(|value| value.as_str())
                == Some("us:conan:Amdt14.S1.5.1")
                && row
                    .get("target_canonical_id")
                    .and_then(|value| value.as_str())
                    == Some("us:constitution:amendment-14:section-1")
        }));
    }
}
