use crate::models::{CitationMention, CitesEdge, LegalTextIdentity, LegalTextVersion, Provision};
use crate::text::normalize_ws;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

static PIN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(([0-9A-Za-z]+)\)").unwrap());
static SUBSECTION_PIN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^ORS\s+([0-9]{1,3}[A-Z]?\.[0-9]{3,4})\s*((?:\([0-9A-Za-z]+\))+)$").unwrap()
});
static CHAPTER_CITATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^ORS\s+chapter\s+([0-9]{1,3}[A-Z]?)$").unwrap());

#[derive(Debug, Default)]
pub struct GlobalSymbolTable {
    pub identities: HashMap<String, LegalTextIdentity>,
    pub versions: HashMap<String, LegalTextVersion>,
    pub provisions: HashMap<String, Provision>,
    pub section_by_citation: HashMap<String, String>,
    pub chapter_versions: HashMap<String, String>,
    pub provision_by_path: HashMap<String, String>,
}

impl GlobalSymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_identity(&mut self, identity: LegalTextIdentity) {
        self.section_by_citation.insert(
            identity.citation.to_lowercase(),
            identity.canonical_id.clone(),
        );
        self.identities
            .insert(identity.canonical_id.clone(), identity);
    }

    pub fn add_version(&mut self, version: LegalTextVersion) {
        let chapter_version_id = format!(
            "or:ors:chapter:{}@{}",
            version.chapter, version.edition_year
        );
        self.chapter_versions.insert(
            format!(
                "ors chapter {}@{}",
                version.chapter.to_lowercase(),
                version.edition_year
            ),
            chapter_version_id,
        );
        self.versions.insert(version.version_id.clone(), version);
    }

    pub fn add_provision(&mut self, provision: Provision) {
        let path_key = format!(
            "{}::{}",
            provision.canonical_id,
            provision.local_path.join(".")
        );
        self.provision_by_path
            .insert(path_key, provision.provision_id.clone());
        self.provisions
            .insert(provision.provision_id.clone(), provision);
    }

    pub fn resolve_canonical_id(&self, citation: &str) -> Option<&String> {
        self.section_by_citation.get(&citation.to_lowercase())
    }

    pub fn resolve_version_id(&self, canonical_id: &str, edition_year: i32) -> Option<String> {
        let version_id = format!("{}@{}", canonical_id, edition_year);
        if self.versions.contains_key(&version_id) {
            Some(version_id)
        } else {
            None
        }
    }

    pub fn resolve_provision_path(&self, canonical_id: &str, path: &[String]) -> Option<&String> {
        let path_key = format!("{}::{}", canonical_id, path.join("."));
        self.provision_by_path.get(&path_key)
    }

    pub fn resolve_chapter_version(&self, chapter: &str, edition_year: i32) -> Option<String> {
        let lookup_key = format!("ors chapter {}@{}", chapter.to_lowercase(), edition_year);
        self.chapter_versions.get(&lookup_key).cloned()
    }
}

pub fn build_global_symbol_table(
    graph_dir: &Path,
    _edition_year: i32,
) -> Result<GlobalSymbolTable> {
    let mut table = GlobalSymbolTable::new();
    let identities_path = graph_dir.join("legal_text_identities.jsonl");
    if identities_path.exists() {
        for line in fs::read_to_string(&identities_path)?.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let identity: LegalTextIdentity = serde_json::from_str(line)?;
            table.add_identity(identity);
        }
    }
    let versions_path = graph_dir.join("legal_text_versions.jsonl");
    if versions_path.exists() {
        for line in fs::read_to_string(&versions_path)?.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let version: LegalTextVersion = serde_json::from_str(line)?;
            table.add_version(version);
        }
    }
    let provisions_path = graph_dir.join("provisions.jsonl");
    if provisions_path.exists() {
        for line in fs::read_to_string(&provisions_path)?.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let provision: Provision = serde_json::from_str(line)?;
            table.add_provision(provision);
        }
    }
    Ok(table)
}

#[derive(Debug, Default)]
pub struct ResolutionStats {
    pub total: usize,
    pub resolved_section: usize,
    pub resolved_section_and_provision: usize,
    pub resolved_range: usize,
    pub resolved_chapter: usize,
    pub resolved_external_placeholder: usize,
    pub resolved_section_unresolved_subpath: usize,
    pub unresolved_target_not_in_corpus: usize,
    pub unresolved_malformed_citation: usize,
    pub unsupported_citation_type: usize,
    pub warnings: usize,
    pub errors: usize,
}

pub fn resolve_all_citations(
    table: &GlobalSymbolTable,
    citations: &mut [CitationMention],
    edition_year: i32,
) -> (Vec<CitesEdge>, ResolutionStats) {
    let mut edges = Vec::new();
    let mut stats = ResolutionStats::default();
    stats.total = citations.len();

    for citation in citations.iter_mut() {
        let result = resolve_single_citation(table, citation, edition_year);
        if let Some(edge) = result.edge {
            edges.push(edge);
        }
        citation.resolver_status = result.status.clone();
        citation.target_provision_id = result.target_provision_id;
        citation.unresolved_subpath = result.unresolved_subpath;
        citation.qc_severity = result.qc_severity;

        match result.status.as_str() {
            "resolved_section" => stats.resolved_section += 1,
            "resolved_section_and_provision" => stats.resolved_section_and_provision += 1,
            "resolved_range" => stats.resolved_range += 1,
            "resolved_chapter" => stats.resolved_chapter += 1,
            "resolved_external_placeholder" => stats.resolved_external_placeholder += 1,
            "resolved_section_unresolved_subpath" => {
                stats.resolved_section_unresolved_subpath += 1;
                stats.warnings += 1;
            }
            "unresolved_target_not_in_corpus" => {
                stats.unresolved_target_not_in_corpus += 1;
                stats.warnings += 1;
            }
            "unresolved_malformed_citation" => {
                stats.unresolved_malformed_citation += 1;
                stats.warnings += 1;
            }
            "unsupported_citation_type" => {
                stats.unsupported_citation_type += 1;
                stats.warnings += 1;
            }
            _ => {}
        }
    }
    (edges, stats)
}

#[derive(Debug)]
struct ResolutionResult {
    status: String,
    edge: Option<CitesEdge>,
    target_provision_id: Option<String>,
    unresolved_subpath: Option<Vec<String>>,
    qc_severity: Option<String>,
}

fn resolve_single_citation(
    table: &GlobalSymbolTable,
    citation: &CitationMention,
    edition_year: i32,
) -> ResolutionResult {
    match citation.citation_type.as_str() {
        "statute_section" => resolve_section_citation(table, citation, edition_year),
        "statute_subsection" => resolve_subsection_citation(table, citation, edition_year),
        "statute_chapter" => resolve_chapter_citation(table, citation, edition_year),
        "statute_range" => resolve_range_citation(table, citation, edition_year),
        "statute_chapter_range" => resolve_chapter_range_citation(table, citation, edition_year),
        _ => ResolutionResult {
            status: "unsupported_citation_type".to_string(),
            edge: None,
            target_provision_id: None,
            unresolved_subpath: None,
            qc_severity: Some("warning".to_string()),
        },
    }
}

fn resolve_chapter_range_citation(
    table: &GlobalSymbolTable,
    citation: &CitationMention,
    edition_year: i32,
) -> ResolutionResult {
    let Some(start_chapter) = citation
        .target_start_canonical_id
        .as_deref()
        .and_then(|id| id.strip_prefix("or:ors:chapter:"))
    else {
        return ResolutionResult {
            status: "unresolved_malformed_citation".to_string(),
            edge: None,
            target_provision_id: None,
            unresolved_subpath: None,
            qc_severity: Some("warning".to_string()),
        };
    };
    let Some(end_chapter) = citation
        .target_end_canonical_id
        .as_deref()
        .and_then(|id| id.strip_prefix("or:ors:chapter:"))
    else {
        return ResolutionResult {
            status: "unresolved_malformed_citation".to_string(),
            edge: None,
            target_provision_id: None,
            unresolved_subpath: None,
            qc_severity: Some("warning".to_string()),
        };
    };

    let Some(start_version_id) = table.resolve_chapter_version(start_chapter, edition_year) else {
        return ResolutionResult {
            status: "unresolved_target_not_in_corpus".to_string(),
            edge: None,
            target_provision_id: None,
            unresolved_subpath: None,
            qc_severity: Some("warning".to_string()),
        };
    };
    if table
        .resolve_chapter_version(end_chapter, edition_year)
        .is_none()
    {
        return ResolutionResult {
            status: "unresolved_target_not_in_corpus".to_string(),
            edge: None,
            target_provision_id: None,
            unresolved_subpath: None,
            qc_severity: Some("warning".to_string()),
        };
    }

    let start_canonical = format!("or:ors:chapter:{start_chapter}");
    let edge = CitesEdge {
        edge_id: format!(
            "edge:{}:CITES_CHAPTER_RANGE:{}",
            citation.citation_mention_id, start_canonical
        ),
        edge_type: "CITES_CHAPTER_RANGE".to_string(),
        source_provision_id: citation.source_provision_id.clone(),
        target_canonical_id: Some(start_canonical),
        target_version_id: Some(start_version_id.clone()),
        target_provision_id: None,
        target_chapter_id: Some(start_version_id),
        citation_kind: Some("chapter_range".to_string()),
        citation_mention_id: citation.citation_mention_id.clone(),
    };

    ResolutionResult {
        status: "resolved_range".to_string(),
        edge: Some(edge),
        target_provision_id: None,
        unresolved_subpath: None,
        qc_severity: None,
    }
}

fn resolve_section_citation(
    table: &GlobalSymbolTable,
    citation: &CitationMention,
    edition_year: i32,
) -> ResolutionResult {
    let normalized = normalize_ws(&citation.normalized_citation);
    let canonical_id = match table.resolve_canonical_id(&normalized) {
        Some(id) => id.clone(),
        None => {
            return ResolutionResult {
                status: "unresolved_target_not_in_corpus".to_string(),
                edge: None,
                target_provision_id: None,
                unresolved_subpath: None,
                qc_severity: Some("warning".to_string()),
            }
        }
    };
    let version_id = table.resolve_version_id(&canonical_id, edition_year);
    let edge = CitesEdge {
        edge_id: format!(
            "edge:{}:CITES:{}",
            citation.citation_mention_id, canonical_id
        ),
        edge_type: "CITES".to_string(),
        source_provision_id: citation.source_provision_id.clone(),
        target_canonical_id: Some(canonical_id.clone()),
        target_version_id: version_id.clone(),
        target_provision_id: None,
        target_chapter_id: None,
        citation_kind: None,
        citation_mention_id: citation.citation_mention_id.clone(),
    };
    ResolutionResult {
        status: "resolved_section".to_string(),
        edge: Some(edge),
        target_provision_id: None,
        unresolved_subpath: None,
        qc_severity: None,
    }
}

fn resolve_subsection_citation(
    table: &GlobalSymbolTable,
    citation: &CitationMention,
    edition_year: i32,
) -> ResolutionResult {
    let normalized = normalize_ws(&citation.normalized_citation);
    let caps = match SUBSECTION_PIN_RE.captures(&normalized) {
        Some(c) => c,
        None => {
            return ResolutionResult {
                status: "unresolved_malformed_citation".to_string(),
                edge: None,
                target_provision_id: None,
                unresolved_subpath: None,
                qc_severity: Some("warning".to_string()),
            }
        }
    };
    let section_citation = format!("ORS {}", caps.get(1).unwrap().as_str());
    let pin_chain = caps.get(2).unwrap().as_str();
    let path: Vec<String> = PIN_RE
        .captures_iter(pin_chain)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect();
    let canonical_id = match table.resolve_canonical_id(&section_citation) {
        Some(id) => id.clone(),
        None => {
            return ResolutionResult {
                status: "unresolved_target_not_in_corpus".to_string(),
                edge: None,
                target_provision_id: None,
                unresolved_subpath: Some(path),
                qc_severity: Some("warning".to_string()),
            }
        }
    };
    let version_id = table.resolve_version_id(&canonical_id, edition_year);
    let provision_id = table.resolve_provision_path(&canonical_id, &path);
    let edge = CitesEdge {
        edge_id: format!(
            "edge:{}:CITES:{}",
            citation.citation_mention_id, canonical_id
        ),
        edge_type: "CITES".to_string(),
        source_provision_id: citation.source_provision_id.clone(),
        target_canonical_id: Some(canonical_id.clone()),
        target_version_id: version_id.clone(),
        target_provision_id: provision_id.cloned(),
        target_chapter_id: None,
        citation_kind: None,
        citation_mention_id: citation.citation_mention_id.clone(),
    };
    if provision_id.is_some() {
        ResolutionResult {
            status: "resolved_section_and_provision".to_string(),
            edge: Some(edge),
            target_provision_id: provision_id.cloned(),
            unresolved_subpath: None,
            qc_severity: None,
        }
    } else {
        ResolutionResult {
            status: "resolved_section_unresolved_subpath".to_string(),
            edge: Some(edge),
            target_provision_id: None,
            unresolved_subpath: Some(path),
            qc_severity: Some("warning".to_string()),
        }
    }
}

fn resolve_chapter_citation(
    table: &GlobalSymbolTable,
    citation: &CitationMention,
    edition_year: i32,
) -> ResolutionResult {
    let normalized = normalize_ws(&citation.normalized_citation);
    let caps = match CHAPTER_CITATION_RE.captures(&normalized) {
        Some(c) => c,
        None => {
            return ResolutionResult {
                status: "unresolved_malformed_citation".to_string(),
                edge: None,
                target_provision_id: None,
                unresolved_subpath: None,
                qc_severity: Some("warning".to_string()),
            }
        }
    };
    let chapter = caps.get(1).unwrap().as_str();
    let chapter_version_id = match table.resolve_chapter_version(chapter, edition_year) {
        Some(id) => id,
        None => {
            return ResolutionResult {
                status: "unresolved_target_not_in_corpus".to_string(),
                edge: None,
                target_provision_id: None,
                unresolved_subpath: None,
                qc_severity: Some("warning".to_string()),
            }
        }
    };
    let edge = CitesEdge {
        edge_id: format!(
            "edge:{}:CITES_CHAPTER:{}",
            citation.citation_mention_id, chapter_version_id
        ),
        edge_type: "CITES_CHAPTER".to_string(),
        source_provision_id: citation.source_provision_id.clone(),
        target_canonical_id: Some(format!("or:ors:chapter:{}", chapter)),
        target_version_id: Some(chapter_version_id.clone()),
        target_provision_id: None,
        target_chapter_id: Some(chapter_version_id.clone()),
        citation_kind: None,
        citation_mention_id: citation.citation_mention_id.clone(),
    };
    ResolutionResult {
        status: "resolved_chapter".to_string(),
        edge: Some(edge),
        target_provision_id: None,
        unresolved_subpath: None,
        qc_severity: None,
    }
}

fn resolve_range_citation(
    table: &GlobalSymbolTable,
    citation: &CitationMention,
    edition_year: i32,
) -> ResolutionResult {
    let start_citation = citation
        .target_start_canonical_id
        .clone()
        .map(|s| format!("ORS {}", s.strip_prefix("or:ors:").unwrap_or(&s)))
        .unwrap_or_default();
    let end_citation = citation
        .target_end_canonical_id
        .clone()
        .map(|s| format!("ORS {}", s.strip_prefix("or:ors:").unwrap_or(&s)))
        .unwrap_or_default();
    let start_canonical = table.resolve_canonical_id(&start_citation);
    let end_canonical = table.resolve_canonical_id(&end_citation);
    if start_canonical.is_none() || end_canonical.is_none() {
        return ResolutionResult {
            status: "unresolved_target_not_in_corpus".to_string(),
            edge: None,
            target_provision_id: None,
            unresolved_subpath: None,
            qc_severity: Some("warning".to_string()),
        };
    }
    let start_id = start_canonical.unwrap().clone();
    let end_id = end_canonical.unwrap().clone();
    let start_version = table.resolve_version_id(&start_id, edition_year);
    let _end_version = table.resolve_version_id(&end_id, edition_year);
    let edge = CitesEdge {
        edge_id: format!(
            "edge:{}:CITES_RANGE:{}",
            citation.citation_mention_id, start_id
        ),
        edge_type: "CITES_RANGE".to_string(),
        source_provision_id: citation.source_provision_id.clone(),
        target_canonical_id: Some(start_id),
        target_version_id: start_version,
        target_provision_id: None,
        target_chapter_id: None,
        citation_kind: Some("range".to_string()),
        citation_mention_id: citation.citation_mention_id.clone(),
    };
    ResolutionResult {
        status: "resolved_range".to_string(),
        edge: Some(edge),
        target_provision_id: None,
        unresolved_subpath: None,
        qc_severity: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CitationMention, LegalTextIdentity, LegalTextVersion, Provision};

    #[test]
    fn test_global_symbol_table_resolution() {
        let mut table = GlobalSymbolTable::new();

        let identity = LegalTextIdentity {
            canonical_id: "or:ors:1.001".to_string(),
            citation: "ORS 1.001".to_string(),
            chapter: "1".to_string(),
            ..Default::default()
        };
        table.add_identity(identity);

        assert_eq!(
            table.resolve_canonical_id("ORS 1.001"),
            Some(&"or:ors:1.001".to_string())
        );
        assert_eq!(
            table.resolve_canonical_id("ors 1.001"),
            Some(&"or:ors:1.001".to_string())
        ); // Case insensitive
    }

    #[test]
    fn test_resolve_subsection() {
        let mut table = GlobalSymbolTable::new();
        table.add_identity(LegalTextIdentity {
            canonical_id: "or:ors:1.001".to_string(),
            citation: "ORS 1.001".to_string(),
            ..Default::default()
        });
        table.add_provision(Provision {
            provision_id: "p1".to_string(),
            canonical_id: "or:ors:1.001".to_string(),
            local_path: vec!["1".to_string(), "a".to_string()],
            ..Default::default()
        });

        let mention = CitationMention {
            citation_mention_id: "m1".to_string(),
            source_provision_id: "src1".to_string(),
            raw_text: "ORS 1.001 (1)(a)".to_string(),
            normalized_citation: "ORS 1.001 (1)(a)".to_string(),
            citation_type: "statute_subsection".to_string(),
            ..Default::default()
        };

        let (edges, stats) = resolve_all_citations(&table, &mut [mention], 2025);
        assert_eq!(stats.resolved_section_and_provision, 1);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target_provision_id, Some("p1".to_string()));
    }

    #[test]
    fn test_resolve_chapter() {
        let mut table = GlobalSymbolTable::new();
        table.add_version(LegalTextVersion {
            version_id: "or:ors:chapter:1@2025".to_string(),
            chapter: "1".to_string(),
            edition_year: 2025,
            ..Default::default()
        });

        let mention = CitationMention {
            citation_mention_id: "m1".to_string(),
            normalized_citation: "ORS chapter 1".to_string(),
            citation_type: "statute_chapter".to_string(),
            ..Default::default()
        };

        let (edges, stats) = resolve_all_citations(&table, &mut [mention], 2025);
        assert_eq!(stats.resolved_chapter, 1);
        assert_eq!(edges.len(), 1);
        assert_eq!(
            edges[0].target_chapter_id,
            Some("or:ors:chapter:1@2025".to_string())
        );
    }

    #[test]
    fn test_resolve_chapter_range() {
        let mut table = GlobalSymbolTable::new();
        for chapter in ["3", "5"] {
            table.add_version(LegalTextVersion {
                version_id: format!("or:ors:chapter:{chapter}@2025"),
                chapter: chapter.to_string(),
                edition_year: 2025,
                ..Default::default()
            });
        }

        let mut mention = CitationMention {
            citation_mention_id: "m1".to_string(),
            source_provision_id: "src1".to_string(),
            normalized_citation: "ORS chapters 3 to 5".to_string(),
            citation_type: "statute_chapter_range".to_string(),
            target_start_canonical_id: Some("or:ors:chapter:3".to_string()),
            target_end_canonical_id: Some("or:ors:chapter:5".to_string()),
            ..Default::default()
        };

        let (edges, stats) =
            resolve_all_citations(&table, std::slice::from_mut(&mut mention), 2025);
        assert_eq!(stats.resolved_range, 1);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, "CITES_CHAPTER_RANGE");
        assert_eq!(
            edges[0].target_chapter_id,
            Some("or:ors:chapter:3@2025".to_string())
        );
    }

    #[test]
    fn test_resolve_unresolved_subpath() {
        let mut table = GlobalSymbolTable::new();
        table.add_identity(LegalTextIdentity {
            canonical_id: "or:ors:1.001".to_string(),
            citation: "ORS 1.001".to_string(),
            ..Default::default()
        });

        let mut mention = CitationMention {
            citation_mention_id: "m1".to_string(),
            source_provision_id: "src1".to_string(),
            normalized_citation: "ORS 1.001 (99)".to_string(),
            citation_type: "statute_subsection".to_string(),
            ..Default::default()
        };

        let (edges, stats) =
            resolve_all_citations(&table, std::slice::from_mut(&mut mention), 2025);
        assert_eq!(stats.resolved_section_unresolved_subpath, 1);
        assert_eq!(edges.len(), 1);
        assert_eq!(mention.unresolved_subpath, Some(vec!["99".to_string()]));
    }
}
