use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Api,
    Bulk,
    StaticHtml,
    Pdf,
    Socrata,
    Arcgis,
    SearchPage,
}

impl SourceKind {
    pub fn parse(value: &str) -> Result<Self> {
        match normalize_enum(value).as_str() {
            "api" => Ok(Self::Api),
            "bulk" => Ok(Self::Bulk),
            "static_html" => Ok(Self::StaticHtml),
            "pdf" => Ok(Self::Pdf),
            "socrata" => Ok(Self::Socrata),
            "arcgis" => Ok(Self::Arcgis),
            "search_page" => Ok(Self::SearchPage),
            other => Err(anyhow!("unsupported source_type: {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Bulk => "bulk",
            Self::StaticHtml => "static_html",
            Self::Pdf => "pdf",
            Self::Socrata => "socrata",
            Self::Arcgis => "arcgis",
            Self::SearchPage => "search_page",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AccessModel {
    Free,
    FreeKeyRequired,
    PublicSearch,
    Mixed,
}

impl AccessModel {
    pub fn parse(value: &str) -> Result<Self> {
        match normalize_enum(value).as_str() {
            "free" => Ok(Self::Free),
            "free_key_required" => Ok(Self::FreeKeyRequired),
            "public_search" => Ok(Self::PublicSearch),
            "mixed" => Ok(Self::Mixed),
            other => Err(anyhow!("unsupported access: {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::FreeKeyRequired => "free_key_required",
            Self::PublicSearch => "public_search",
            Self::Mixed => "mixed",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum OfficialStatus {
    Official,
    Nonprofit,
    Secondary,
    Unknown,
}

impl OfficialStatus {
    pub fn parse(value: &str) -> Result<Self> {
        match normalize_enum(value).as_str() {
            "official" => Ok(Self::Official),
            "nonprofit" => Ok(Self::Nonprofit),
            "secondary" => Ok(Self::Secondary),
            "unknown" => Ok(Self::Unknown),
            other => Err(anyhow!("unsupported official_status: {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Nonprofit => "nonprofit",
            Self::Secondary => "secondary",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorStatus {
    NotStarted,
    Planned,
    Partial,
    Implemented,
    Blocked,
    Deferred,
}

impl ConnectorStatus {
    pub fn parse(value: &str) -> Result<Self> {
        match normalize_enum(value).as_str() {
            "not_started" => Ok(Self::NotStarted),
            "planned" => Ok(Self::Planned),
            "partial" => Ok(Self::Partial),
            "implemented" => Ok(Self::Implemented),
            "blocked" => Ok(Self::Blocked),
            "deferred" => Ok(Self::Deferred),
            other => Err(anyhow!("unsupported connector_status: {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Planned => "planned",
            Self::Partial => "partial",
            Self::Implemented => "implemented",
            Self::Blocked => "blocked",
            Self::Deferred => "deferred",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourcePriority {
    P0,
    P1,
    P2,
}

impl SourcePriority {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "P0" => Ok(Self::P0),
            "P1" => Ok(Self::P1),
            "P2" => Ok(Self::P2),
            other => Err(anyhow!("unsupported priority: {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRegistry {
    pub sources: Vec<SourceRegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRegistryEntry {
    pub source_id: String,
    pub name: String,
    pub owner: String,
    pub jurisdiction: String,
    pub source_type: SourceKind,
    pub access: AccessModel,
    pub official_status: OfficialStatus,
    pub data_types: Vec<String>,
    pub update_frequency: String,
    pub rate_limits_terms: String,
    pub robots_acceptable_use: String,
    pub preferred_ingestion_method: String,
    pub fallback_ingestion_method: String,
    pub graph_nodes_created: Vec<String>,
    pub graph_edges_created: Vec<String>,
    pub connector_status: ConnectorStatus,
    pub priority: SourcePriority,
    pub risks: Vec<String>,
    pub source_url: String,
    pub docs_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistryValidationReport {
    pub source_count: usize,
    pub p0_sources: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl RegistryValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn load_registry(path: impl AsRef<Path>) -> Result<SourceRegistry> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read source registry {}", path.display()))?;
    match path.extension().and_then(|value| value.to_str()) {
        Some("md") => parse_markdown_registry(&text),
        Some("json") | Some("yaml") | Some("yml") => serde_json::from_str(&text)
            .with_context(|| format!("failed to parse machine registry {}", path.display())),
        _ => Err(anyhow!(
            "unsupported source registry extension for {}",
            path.display()
        )),
    }
}

pub fn load_default_registry() -> Result<SourceRegistry> {
    let yaml = Path::new("docs/data/source-registry.yaml");
    if yaml.exists() {
        return load_registry(yaml);
    }
    load_registry("docs/data/source-registry.md")
}

pub fn validate_registry(registry: &SourceRegistry) -> RegistryValidationReport {
    let mut report = RegistryValidationReport {
        source_count: registry.sources.len(),
        p0_sources: registry
            .sources
            .iter()
            .filter(|source| source.priority == SourcePriority::P0)
            .count(),
        ..Default::default()
    };

    let mut seen = BTreeSet::new();
    for source in &registry.sources {
        if source.source_id.trim().is_empty() {
            report
                .errors
                .push("source_id must not be empty".to_string());
            continue;
        }
        if !seen.insert(source.source_id.clone()) {
            report
                .errors
                .push(format!("duplicate source_id {}", source.source_id));
        }
        if source.name.trim().is_empty()
            || source.owner.trim().is_empty()
            || source.jurisdiction.trim().is_empty()
        {
            report.errors.push(format!(
                "{} has missing required identity fields",
                source.source_id
            ));
        }
        if source.rate_limits_terms.trim().is_empty() {
            report
                .errors
                .push(format!("{} has empty rate_limits_terms", source.source_id));
        }
        if source.robots_acceptable_use.trim().is_empty() {
            report.errors.push(format!(
                "{} has empty robots_acceptable_use",
                source.source_id
            ));
        }
        if source.preferred_ingestion_method.trim().is_empty()
            || source.fallback_ingestion_method.trim().is_empty()
        {
            report.errors.push(format!(
                "{} must define preferred and fallback ingestion methods",
                source.source_id
            ));
        }
        if source.source_url.contains("utm_") || source.docs_url.contains("utm_") {
            report
                .errors
                .push(format!("{} contains tracking parameters", source.source_id));
        }
        if source.priority == SourcePriority::P0
            && (source.graph_nodes_created.is_empty() || source.graph_edges_created.is_empty())
        {
            report.errors.push(format!(
                "{} P0 source has incomplete graph mapping",
                source.source_id
            ));
        }
        if source.source_type == SourceKind::SearchPage
            && !matches!(
                source.access,
                AccessModel::PublicSearch | AccessModel::Mixed | AccessModel::Free
            )
        {
            report.warnings.push(format!(
                "{} is search_page but access is {}",
                source.source_id,
                source.access.as_str()
            ));
        }
        if source.robots_acceptable_use == "needs_review" {
            report.warnings.push(format!(
                "{} needs explicit robots/acceptable-use review",
                source.source_id
            ));
        }
    }

    report
}

pub fn parse_markdown_registry(text: &str) -> Result<SourceRegistry> {
    let mut sources = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        if !line.starts_with("| `") {
            continue;
        }
        let mut cells = split_markdown_row(line);
        if cells.len() < 15 {
            continue;
        }
        if cells.len() == 19 {
            cells.insert(10, "needs_review".to_string());
        }
        if cells.len() != 20 {
            return Err(anyhow!(
                "source-registry.md row {} has {} cells, expected 20",
                line_index + 1,
                cells.len()
            ));
        }
        sources.push(entry_from_cells(&cells).with_context(|| {
            format!(
                "failed to parse source-registry.md row {} ({})",
                line_index + 1,
                cells.first().cloned().unwrap_or_default()
            )
        })?);
    }
    Ok(SourceRegistry { sources })
}

pub fn write_canonical_registry_json(
    path: impl AsRef<Path>,
    registry: &SourceRegistry,
) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(registry)?)?;
    Ok(())
}

pub fn by_id(registry: &SourceRegistry) -> BTreeMap<String, SourceRegistryEntry> {
    registry
        .sources
        .iter()
        .cloned()
        .map(|source| (source.source_id.clone(), source))
        .collect()
}

fn entry_from_cells(cells: &[String]) -> Result<SourceRegistryEntry> {
    Ok(SourceRegistryEntry {
        source_id: strip_code(&cells[0]),
        name: clean_cell(&cells[1]),
        owner: clean_cell(&cells[2]),
        jurisdiction: strip_code(&cells[3]),
        source_type: SourceKind::parse(&cells[4])?,
        access: AccessModel::parse(&cells[5])?,
        official_status: OfficialStatus::parse(&cells[6])?,
        data_types: split_list(&cells[7]),
        update_frequency: clean_cell(&cells[8]),
        rate_limits_terms: clean_cell(&cells[9]),
        robots_acceptable_use: clean_cell(&cells[10]),
        preferred_ingestion_method: clean_cell(&cells[11]),
        fallback_ingestion_method: clean_cell(&cells[12]),
        graph_nodes_created: split_list(&cells[13]),
        graph_edges_created: split_list(&cells[14]),
        connector_status: ConnectorStatus::parse(&cells[15])?,
        priority: SourcePriority::parse(&cells[16])?,
        risks: split_semicolon_or_sentence(&cells[17]),
        source_url: clean_url(&cells[18]),
        docs_url: clean_url(&cells[19]),
    })
}

fn split_markdown_row(line: &str) -> Vec<String> {
    let mut cells = line
        .trim()
        .trim_matches('|')
        .split('|')
        .map(clean_cell)
        .collect::<Vec<_>>();
    while cells.last().is_some_and(|cell| cell.is_empty()) {
        cells.pop();
    }
    cells
}

fn split_list(value: &str) -> Vec<String> {
    clean_cell(value)
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty() && *item != "unknown")
        .map(|item| strip_code(item))
        .collect()
}

fn split_semicolon_or_sentence(value: &str) -> Vec<String> {
    clean_cell(value)
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn clean_url(value: &str) -> String {
    clean_cell(value)
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_string()
}

fn clean_cell(value: &str) -> String {
    html_escape::decode_html_entities(value.trim())
        .trim()
        .trim_matches('`')
        .to_string()
}

fn strip_code(value: &str) -> String {
    clean_cell(value)
        .trim_matches('`')
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_string()
}

fn normalize_enum(value: &str) -> String {
    strip_code(value).trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_row_with_missing_robots_cell() {
        let md = "| `or_leg_orcp` | Oregon Rules of Civil Procedure | Oregon Legislature | `or:state` | static_html | free | official | civil procedure rules, rule text | biennial | Throttle static fetches. | Static crawl. | PDF fallback | LegalCorpus, Provision | CONTAINS, CITES | planned | P0 | ORCP alignment risk. | <https://example.test/source> | <https://example.test/docs> |";
        let registry = parse_markdown_registry(md).expect("parse markdown registry");
        let source = &registry.sources[0];
        assert_eq!(source.source_id, "or_leg_orcp");
        assert_eq!(source.robots_acceptable_use, "needs_review");
        assert_eq!(source.preferred_ingestion_method, "Static crawl.");
        assert_eq!(source.fallback_ingestion_method, "PDF fallback");
    }

    #[test]
    fn validates_duplicate_ids() {
        let mut registry = SourceRegistry {
            sources: Vec::new(),
        };
        let entry = SourceRegistryEntry {
            source_id: "x".to_string(),
            name: "X".to_string(),
            owner: "Owner".to_string(),
            jurisdiction: "us".to_string(),
            source_type: SourceKind::Api,
            access: AccessModel::Free,
            official_status: OfficialStatus::Official,
            data_types: vec!["data".to_string()],
            update_frequency: "unknown".to_string(),
            rate_limits_terms: "needs_review".to_string(),
            robots_acceptable_use: "needs_review".to_string(),
            preferred_ingestion_method: "API".to_string(),
            fallback_ingestion_method: "Bulk".to_string(),
            graph_nodes_created: vec!["SourceDocument".to_string()],
            graph_edges_created: vec!["DERIVED_FROM".to_string()],
            connector_status: ConnectorStatus::Planned,
            priority: SourcePriority::P0,
            risks: vec!["risk".to_string()],
            source_url: "https://example.test".to_string(),
            docs_url: "https://example.test/docs".to_string(),
        };
        registry.sources.push(entry.clone());
        registry.sources.push(entry);
        let report = validate_registry(&registry);
        assert!(!report.is_valid());
        assert!(
            report
                .errors
                .iter()
                .any(|error| error.contains("duplicate"))
        );
    }
}
