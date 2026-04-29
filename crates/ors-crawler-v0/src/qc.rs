use crate::models::{CitationMention, LegalTextVersion, Provision, RetrievalChunk};
use crate::text::count_rule_line_artifacts;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct QcReport {
    pub duplicate_provision_ids: usize,
    pub duplicate_version_ids: usize,
    pub duplicate_provision_paths: usize,
    pub orphan_chunks: usize,
    pub orphan_citations: usize,
    pub active_sections_missing_titles: usize,
    pub heading_leaks: usize,
    pub artifact_leaks: usize,
    pub reserved_tail_leaks: usize,
    pub chunk_year_mismatches: usize,
    pub contextual_chunks: usize,
    pub valid_provisions: usize,
    // Citation resolution fields
    pub unresolved_citations: usize,
    pub resolved_section_unresolved_subpath: usize,
    pub citation_integrity_errors: usize,
    pub citation_warnings: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl QcReport {
    pub fn is_blocking_failure(&self) -> bool {
        self.duplicate_provision_ids > 0
            || self.duplicate_version_ids > 0
            || self.duplicate_provision_paths > 0
            || self.orphan_chunks > 0
            || self.orphan_citations > 0
            || self.active_sections_missing_titles > 0
            || self.heading_leaks > 0
            || self.artifact_leaks > 0
            || self.reserved_tail_leaks > 0
            || self.chunk_year_mismatches > 0
            || self.contextual_chunks < self.valid_provisions
            || self.citation_integrity_errors > 0
    }
}

pub fn validate_outputs(
    versions: &[LegalTextVersion],
    provisions: &[Provision],
    citations: &[CitationMention],
    chunks: &[RetrievalChunk],
) -> QcReport {
    let mut report = QcReport::default();

    report.duplicate_version_ids = count_dupes(versions.iter().map(|v| v.version_id.as_str()));
    report.duplicate_provision_ids =
        count_dupes(provisions.iter().map(|p| p.provision_id.as_str()));

    let path_keys = provisions
        .iter()
        .map(|p| format!("{}::{}", p.version_id, p.local_path.join(".")));
    report.duplicate_provision_paths = count_dupes(path_keys);

    let provision_ids: HashSet<String> =
        provisions.iter().map(|p| p.provision_id.clone()).collect();

    report.orphan_chunks = chunks
        .iter()
        .filter(|c| {
            c.chunk_type != "full_statute"
                && c.source_provision_id
                    .as_ref()
                    .map(|id| !provision_ids.contains(id))
                    .unwrap_or(true)
        })
        .count();

    report.orphan_citations = citations
        .iter()
        .filter(|c| !provision_ids.contains(&c.source_provision_id))
        .count();

    report.active_sections_missing_titles = versions
        .iter()
        .filter(|v| v.status == "active")
        .filter(|v| {
            v.title
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
        })
        .count();

    report.heading_leaks = versions
        .iter()
        .filter(|v| contains_heading_leak(&v.text))
        .count();

    report.artifact_leaks = versions
        .iter()
        .map(|v| artifact_leak_count(&v.text))
        .sum::<usize>()
        + provisions
            .iter()
            .map(|p| artifact_leak_count(&p.text))
            .sum::<usize>()
        + chunks
            .iter()
            .map(|c| artifact_leak_count(&c.text))
            .sum::<usize>();

    report.reserved_tail_leaks = versions
        .iter()
        .map(|v| reserved_tail_leak_count(&v.text))
        .sum::<usize>()
        + provisions
            .iter()
            .map(|p| reserved_tail_leak_count(&p.text))
            .sum::<usize>()
        + chunks
            .iter()
            .map(|c| reserved_tail_leak_count(&c.text))
            .sum::<usize>();

    report.chunk_year_mismatches = chunks
        .iter()
        .filter(|c| {
            c.chunk_type == "contextual_provision"
                && !c.text.starts_with(&format!(
                    "Oregon Revised Statutes. {} Edition.",
                    c.edition_year
                ))
        })
        .count();

    report.contextual_chunks = chunks
        .iter()
        .filter(|c| c.chunk_type == "contextual_provision")
        .count();

    report.valid_provisions = provisions
        .iter()
        .filter(|p| !p.is_implied && !p.text.trim().is_empty())
        .count();

    if report.duplicate_provision_ids > 0 {
        report.errors.push(format!(
            "duplicate provision_id count: {}",
            report.duplicate_provision_ids
        ));
    }

    if report.duplicate_provision_paths > 0 {
        report.errors.push(format!(
            "duplicate (version_id, local_path) count: {}",
            report.duplicate_provision_paths
        ));
    }

    if report.contextual_chunks < report.valid_provisions {
        report.errors.push(format!(
            "contextual chunks too low: {} chunks for {} provisions",
            report.contextual_chunks, report.valid_provisions
        ));
    }

    if report.active_sections_missing_titles > 0 {
        report.errors.push(format!(
            "active sections missing titles: {}",
            report.active_sections_missing_titles
        ));
    }

    if report.artifact_leaks > 0 {
        report.errors.push(format!(
            "layout artifact leaks detected: {}",
            report.artifact_leaks
        ));
    }

    if report.reserved_tail_leaks > 0 {
        report.errors.push(format!(
            "reserved tail leaks detected: {}",
            report.reserved_tail_leaks
        ));
    }

    if report.chunk_year_mismatches > 0 {
        report.errors.push(format!(
            "contextual chunk year mismatches: {}",
            report.chunk_year_mismatches
        ));
    }

    report
}

fn count_dupes<I, S>(items: I) -> usize
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut counts: HashMap<String, usize> = HashMap::new();

    for item in items {
        *counts.entry(item.as_ref().to_string()).or_insert(0) += 1;
    }

    counts.values().filter(|&&n| n > 1).map(|n| n - 1).sum()
}

fn contains_heading_leak(text: &str) -> bool {
    const BAD_HEADINGS: &[&str] = &[
        "COURTHOUSE CAPITAL CONSTRUCTION AND IMPROVEMENT",
        "OPERATION OF COURTHOUSES",
        "COLLECTION OF COURT ACCOUNTS",
        "PUBLICATION OF COURT DECISIONS",
        "ALTERNATIVE DISPUTE RESOLUTION",
        "COURT FACILITIES",
        "COURT OF APPEALS",
        "SUPREME COURT",
    ];

    BAD_HEADINGS.iter().any(|h| text.contains(h))
}

fn artifact_leak_count(text: &str) -> usize {
    count_rule_line_artifacts(text)
}

fn reserved_tail_leak_count(text: &str) -> usize {
    let patterns = [
        "[Reserved for expansion]",
        "CHAPTERS 831 TO 834",
        "TITLES 63 et seq.",
        "CHAPTERS 839 et seq.",
    ];

    patterns.iter().filter(|p| text.contains(**p)).count()
}

/// Validate citation resolution for coverage (warnings)
pub fn validate_citation_coverage(citations: &[CitationMention]) -> (usize, usize) {
    let mut warnings = 0;
    let mut unresolved = 0;

    for citation in citations {
        match citation.resolver_status.as_str() {
            "unresolved_target_not_in_corpus"
            | "unresolved_malformed_citation"
            | "unsupported_citation_type" => {
                warnings += 1;
                unresolved += 1;
            }
            "resolved_section_unresolved_subpath" => {
                warnings += 1;
            }
            _ => {}
        }
    }

    (warnings, unresolved)
}

/// Validate citation resolution for integrity (errors)
pub fn validate_citation_integrity(
    citations: &[CitationMention],
    provision_ids: &HashSet<String>,
    identity_ids: &HashSet<String>,
) -> usize {
    let mut errors = 0;

    for citation in citations {
        // Check source provision exists
        if !provision_ids.contains(&citation.source_provision_id) {
            errors += 1;
            continue;
        }

        // If resolved, check target identity exists
        if citation.resolver_status.starts_with("resolved") {
            if let Some(ref target_id) = citation.target_canonical_id {
                if !identity_ids.contains(target_id) {
                    errors += 1;
                }
            }
        }
    }

    errors
}
