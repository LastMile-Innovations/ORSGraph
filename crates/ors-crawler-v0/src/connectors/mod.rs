use crate::artifact_store::RawArtifact;
use crate::graph_batch::GraphBatch;
use crate::hash::{sha256_hex, stable_id};
use crate::models::{ParserDiagnostic, SourceDocument, SourcePage};
use crate::ors_dom_parser::parse_ors_chapter_html;
use crate::source_qc::{QcReport, qc_source_batch};
use crate::source_registry::{SourceKind, SourceRegistryEntry};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

const DEFAULT_ORS_CHAPTER_COUNT: u32 = 524;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    pub item_id: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub content_type: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[async_trait]
pub trait DataConnector: Send + Sync {
    fn source_id(&self) -> &'static str;
    fn source_kind(&self) -> SourceKind;
    async fn discover(&self) -> Result<Vec<SourceItem>>;
    async fn parse(&self, artifact: &RawArtifact) -> Result<GraphBatch>;
    async fn qc(
        &self,
        artifacts: &[crate::artifact_store::ArtifactMetadata],
        batch: &GraphBatch,
    ) -> Result<QcReport>;
}

#[derive(Debug, Clone)]
pub struct ConnectorOptions {
    pub edition_year: i32,
    pub chapters: Option<String>,
    pub session_key: Option<String>,
    pub max_items: usize,
}

pub fn connector_for(
    entry: SourceRegistryEntry,
    options: ConnectorOptions,
) -> Box<dyn DataConnector> {
    if entry.source_id == "or_leg_ors_html" {
        Box::new(OrsHtmlConnector { entry, options })
    } else if entry.source_id == "or_leg_odata" {
        Box::new(crate::oregon_leg_odata::OregonLegODataConnector::new(
            entry, options,
        ))
    } else if entry.source_id == "or_leg_constitution" {
        Box::new(
            crate::oregon_constitution::OregonConstitutionConnector::new(entry, options),
        )
    } else if matches!(
        entry.source_id.as_str(),
        "congress_gov_us_constitution" | "congress_gov_constitution_annotated"
    ) {
        Box::new(crate::congress_constitution::CongressConstitutionConnector::new(entry, options))
    } else {
        Box::new(RegistryBackedConnector { entry, options })
    }
}

struct OrsHtmlConnector {
    entry: SourceRegistryEntry,
    options: ConnectorOptions,
}

#[async_trait]
impl DataConnector for OrsHtmlConnector {
    fn source_id(&self) -> &'static str {
        "or_leg_ors_html"
    }

    fn source_kind(&self) -> SourceKind {
        SourceKind::StaticHtml
    }

    async fn discover(&self) -> Result<Vec<SourceItem>> {
        let chapters = if let Some(chapters) = &self.options.chapters {
            parse_chapter_list(chapters)?
        } else {
            default_ors_chapters(self.options.max_items)
        };
        Ok(chapters
            .into_iter()
            .map(|chapter| {
                let padded = chapter_pad(&chapter);
                let url =
                    format!("https://www.oregonlegislature.gov/bills_laws/ors/ors{padded}.html");
                let mut metadata = BTreeMap::new();
                metadata.insert("chapter".to_string(), chapter.clone());
                SourceItem {
                    item_id: format!("ors{padded}"),
                    url: Some(url),
                    title: Some(format!("ORS Chapter {chapter}")),
                    content_type: Some("text/html".to_string()),
                    metadata,
                }
            })
            .collect())
    }

    async fn parse(&self, artifact: &RawArtifact) -> Result<GraphBatch> {
        let chapter = artifact
            .metadata
            .item_id
            .strip_prefix("ors")
            .unwrap_or(&artifact.metadata.item_id)
            .trim_start_matches('0')
            .to_string();
        let chapter = if chapter.is_empty() {
            "0".to_string()
        } else {
            chapter
        };
        let html = decode_html(&artifact.bytes);
        let parsed = parse_ors_chapter_html(
            &html,
            &artifact.metadata.url,
            &chapter,
            self.options.edition_year,
        )?;
        let mut batch = GraphBatch::default();
        batch.extend_parsed_chapter(&parsed)?;
        Ok(batch)
    }

    async fn qc(
        &self,
        artifacts: &[crate::artifact_store::ArtifactMetadata],
        batch: &GraphBatch,
    ) -> Result<QcReport> {
        Ok(qc_source_batch(&self.entry, artifacts, batch))
    }
}

struct RegistryBackedConnector {
    entry: SourceRegistryEntry,
    options: ConnectorOptions,
}

#[async_trait]
impl DataConnector for RegistryBackedConnector {
    fn source_id(&self) -> &'static str {
        "registry_backed"
    }

    fn source_kind(&self) -> SourceKind {
        self.entry.source_type
    }

    async fn discover(&self) -> Result<Vec<SourceItem>> {
        if self.entry.source_url == "varies"
            || self.entry.source_url == "varies by county"
            || (self.entry.source_type == SourceKind::SearchPage
                && self.entry.robots_acceptable_use == "needs_review")
        {
            return Ok(vec![SourceItem {
                item_id: "registry".to_string(),
                url: None,
                title: Some(self.entry.name.clone()),
                content_type: Some("application/json".to_string()),
                metadata: BTreeMap::new(),
            }]);
        }
        Ok(vec![SourceItem {
            item_id: stable_id(&self.entry.source_url),
            url: Some(self.entry.source_url.clone()),
            title: Some(self.entry.name.clone()),
            content_type: None,
            metadata: BTreeMap::new(),
        }])
    }

    async fn parse(&self, artifact: &RawArtifact) -> Result<GraphBatch> {
        let text = String::from_utf8_lossy(&artifact.bytes).to_string();
        let normalized = normalize_text(&text);
        let source_document_id = format!(
            "src:{}:{}",
            self.entry.source_id,
            stable_id(&artifact.metadata.url)
        );
        let source_document = SourceDocument {
            source_document_id: source_document_id.clone(),
            source_provider: self.entry.owner.clone(),
            source_kind: self.entry.source_type.as_str().to_string(),
            url: artifact.metadata.url.clone(),
            chapter: self.entry.source_id.clone(),
            corpus_id: None,
            edition_id: None,
            authority_family: Some(authority_family(&self.entry)),
            authority_type: Some(self.entry.source_type.as_str().to_string()),
            title: Some(self.entry.name.clone()),
            source_type: Some(self.entry.source_type.as_str().to_string()),
            file_name: artifact
                .metadata
                .path
                .rsplit('/')
                .next()
                .map(ToOwned::to_owned),
            page_count: Some(1),
            effective_date: None,
            copyright_status: Some(self.entry.access.as_str().to_string()),
            chapter_title: Some(self.entry.name.clone()),
            edition_year: self.options.edition_year,
            html_encoding: Some("utf-8".to_string()),
            source_path: Some(artifact.metadata.path.clone()),
            paragraph_count: Some(
                normalized
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .count(),
            ),
            first_body_paragraph_index: Some(0),
            parser_profile: Some("registry_backed_connector_v1".to_string()),
            official_status: self.entry.official_status.as_str().to_string(),
            disclaimer_required: self.entry.official_status.as_str() != "official",
            raw_hash: artifact.metadata.raw_hash.clone(),
            normalized_hash: sha256_hex(normalized.as_bytes()),
        };
        let source_page = SourcePage {
            source_page_id: format!("{source_document_id}:page:1"),
            source_document_id: source_document_id.clone(),
            page_number: 1,
            printed_label: Some("1".to_string()),
            text: text.chars().take(80_000).collect(),
            normalized_text: normalized.chars().take(80_000).collect(),
            text_hash: sha256_hex(normalized.as_bytes()),
        };
        let mut batch = GraphBatch::default();
        batch.push("source_documents.jsonl", &source_document)?;
        batch.push("source_pages.jsonl", &source_page)?;

        for label in &self.entry.graph_nodes_created {
            let row = json!({
                "id": format!("{}:{}:{}", self.entry.source_id, label, stable_id(&artifact.metadata.url)),
                "source_id": self.entry.source_id,
                "source_document_id": source_document_id,
                "label": label,
                "name": self.entry.name,
                "jurisdiction_id": self.entry.jurisdiction,
                "official_status": self.entry.official_status.as_str(),
                "parser_profile": "registry_backed_connector_v1",
                "raw_hash": artifact.metadata.raw_hash,
                "source_url": artifact.metadata.url,
                "parsed_at": Utc::now().to_rfc3339(),
            });
            batch.push("source_registry_nodes.jsonl", &row)?;
        }
        for edge_type in &self.entry.graph_edges_created {
            let row = json!({
                "id": format!("{}:{}:{}", self.entry.source_id, edge_type, stable_id(&artifact.metadata.url)),
                "source_id": self.entry.source_id,
                "source_document_id": source_document_id,
                "relationship_type": edge_type,
                "official_status": self.entry.official_status.as_str(),
                "parser_profile": "registry_backed_connector_v1",
                "raw_hash": artifact.metadata.raw_hash,
                "source_url": artifact.metadata.url,
                "parsed_at": Utc::now().to_rfc3339(),
            });
            batch.push("source_registry_edges.jsonl", &row)?;
        }

        let diagnostic = ParserDiagnostic {
            parser_diagnostic_id: format!(
                "diag:{}:{}",
                self.entry.source_id,
                stable_id(&format!("{}:registry_backed", artifact.metadata.url))
            ),
            source_document_id,
            chapter: self.entry.source_id.clone(),
            edition_year: self.options.edition_year,
            severity: "warning".to_string(),
            diagnostic_type: "registry_backed_parse".to_string(),
            message: "Connector emitted source-backed graph rows from the registry contract; source-specific deep parsing should refine these rows.".to_string(),
            source_paragraph_order: None,
            related_id: None,
            parser_profile: "registry_backed_connector_v1".to_string(),
        };
        batch.push("parser_diagnostics.jsonl", &diagnostic)?;
        Ok(batch)
    }

    async fn qc(
        &self,
        artifacts: &[crate::artifact_store::ArtifactMetadata],
        batch: &GraphBatch,
    ) -> Result<QcReport> {
        Ok(qc_source_batch(&self.entry, artifacts, batch))
    }
}

fn parse_chapter_list(list: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for item in list.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if let Some((start, end)) = item.split_once('-') {
            let start: u32 = start.trim().parse()?;
            let end: u32 = end.trim().parse()?;
            if start > end {
                return Err(anyhow!("invalid chapter range: {item}"));
            }
            for chapter in start..=end {
                out.push(chapter.to_string());
            }
        } else {
            out.push(item.parse::<u32>()?.to_string());
        }
    }
    Ok(out)
}

fn default_ors_chapters(max_items: usize) -> Vec<String> {
    let max = if max_items > 0 {
        max_items as u32
    } else {
        DEFAULT_ORS_CHAPTER_COUNT
    };
    (1..=max).map(|chapter| chapter.to_string()).collect()
}

fn chapter_pad(chapter: &str) -> String {
    if chapter.chars().all(|ch| ch.is_ascii_digit()) {
        format!("{:03}", chapter.parse::<u32>().unwrap_or(0))
    } else {
        chapter.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_ORS_CHAPTER_COUNT, default_ors_chapters};

    #[test]
    fn default_ors_discovery_uses_full_corpus_when_unbounded() {
        let chapters = default_ors_chapters(0);

        assert_eq!(chapters.len(), DEFAULT_ORS_CHAPTER_COUNT as usize);
        assert_eq!(chapters.first().map(String::as_str), Some("1"));
        assert_eq!(
            chapters.last().cloned(),
            Some(DEFAULT_ORS_CHAPTER_COUNT.to_string())
        );
    }

    #[test]
    fn default_ors_discovery_respects_positive_max_items() {
        assert_eq!(
            default_ors_chapters(2),
            vec!["1".to_string(), "2".to_string()]
        );
    }
}

fn decode_html(bytes: &[u8]) -> String {
    let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    cow.to_string()
}

fn normalize_text(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn authority_family(entry: &SourceRegistryEntry) -> String {
    match entry.source_id.as_str() {
        "congress_gov_us_constitution" => "USCONST",
        "congress_gov_constitution_annotated" => "CONAN",
        "or_leg_constitution" => "ORCONST",
        "or_leg_orcp" => "ORCP",
        "or_sos_oar" => "OAR",
        "ojd_utcr" => "UTCR",
        "ojd_slr_registry" => "RuleAuthorityDocument",
        "or_leg_oregon_laws" => "SessionLaw",
        _ => entry.source_id.as_str(),
    }
    .to_string()
}
