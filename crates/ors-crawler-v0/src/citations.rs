use crate::hash::stable_id;
use crate::models::{CitationMention, Provision};
use regex::Regex;
use std::sync::LazyLock;

static ORS_RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\bORS\s+([0-9]{1,3}[A-Z]?\.[0-9]{3,4})(?:\s*\([^)]+\))*\s+(?:to|through)\s+([0-9]{1,3}[A-Z]?\.[0-9]{3,4})(?:\s*\([^)]+\))*",
    )
    .unwrap()
});

static ORS_CHAPTER_RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\bORS\s+chapters?\s+([0-9]{1,3}[A-Z]?)\s+(?:to|through)\s+([0-9]{1,3}[A-Z]?)\b",
    )
    .unwrap()
});

static ORS_CHAPTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bORS\s+chapters?\s+([0-9]{1,3}[A-Z]?)\b").unwrap());

static ORS_GROUP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\bORS\s+([0-9]{1,3}[A-Z]?\.[0-9]{3,4}(?:\s*\([^)]+\))*(?:\s*(?:,|or|and)\s*[0-9]{1,3}[A-Z]?\.[0-9]{3,4}(?:\s*\([^)]+\))*)*)",
    )
    .unwrap()
});

static SINGLE_SECTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([0-9]{1,3}[A-Z]?\.[0-9]{3,4})((?:\s*\([^)]+\))*)").unwrap());

pub fn extract_citation_mentions(provision: &Provision) -> Vec<CitationMention> {
    let mut out = Vec::new();
    let text = &provision.text;
    let mut occupied_spans = Vec::<(usize, usize)>::new();

    for caps in ORS_RANGE_RE.captures_iter(text) {
        let whole = caps.get(0).unwrap();
        occupied_spans.push((whole.start(), whole.end()));
        let raw = whole.as_str().to_string();
        let start = caps.get(1).unwrap().as_str();
        let end = caps.get(2).unwrap().as_str();

        out.push(CitationMention {
            citation_mention_id: citation_id(&provision.provision_id, &raw),
            source_provision_id: provision.provision_id.clone(),
            raw_text: raw,
            normalized_citation: format!("ORS {} to {}", start, end),
            citation_type: "statute_range".to_string(),
            target_canonical_id: None,
            target_start_canonical_id: Some(format!("or:ors:{}", start)),
            target_end_canonical_id: Some(format!("or:ors:{}", end)),
            target_provision_id: None,
            unresolved_subpath: None,
            resolver_status: "parsed_unverified".to_string(),
            confidence: 0.94,
            qc_severity: None,
        });
    }

    for caps in ORS_CHAPTER_RANGE_RE.captures_iter(text) {
        let whole = caps.get(0).unwrap();
        occupied_spans.push((whole.start(), whole.end()));
        let raw = whole.as_str().to_string();
        let start = caps.get(1).unwrap().as_str();
        let end = caps.get(2).unwrap().as_str();

        out.push(CitationMention {
            citation_mention_id: citation_id(&provision.provision_id, &raw),
            source_provision_id: provision.provision_id.clone(),
            raw_text: raw,
            normalized_citation: format!("ORS chapters {} to {}", start, end),
            citation_type: "statute_chapter_range".to_string(),
            target_canonical_id: None,
            target_start_canonical_id: Some(format!("or:ors:chapter:{}", start)),
            target_end_canonical_id: Some(format!("or:ors:chapter:{}", end)),
            target_provision_id: None,
            unresolved_subpath: None,
            resolver_status: "parsed_unverified".to_string(),
            confidence: 0.9,
            qc_severity: None,
        });
    }

    for caps in ORS_CHAPTER_RE.captures_iter(text) {
        let whole = caps.get(0).unwrap();
        if overlaps_any((whole.start(), whole.end()), &occupied_spans) {
            continue;
        }
        let raw = whole.as_str().to_string();
        let chapter = caps.get(1).unwrap().as_str();

        out.push(CitationMention {
            citation_mention_id: citation_id(&provision.provision_id, &raw),
            source_provision_id: provision.provision_id.clone(),
            raw_text: raw,
            normalized_citation: format!("ORS chapter {}", chapter),
            citation_type: "statute_chapter".to_string(),
            target_canonical_id: Some(format!("or:ors:chapter:{}", chapter)),
            target_start_canonical_id: None,
            target_end_canonical_id: None,
            target_provision_id: None,
            unresolved_subpath: None,
            resolver_status: "parsed_unverified".to_string(),
            confidence: 0.9,
            qc_severity: None,
        });
    }

    for caps in ORS_GROUP_RE.captures_iter(text) {
        let whole = caps.get(0).unwrap();
        if overlaps_any((whole.start(), whole.end()), &occupied_spans) {
            continue;
        }
        let raw_group = whole.as_str().to_string();
        let inner = caps.get(1).unwrap().as_str();

        for sec_caps in SINGLE_SECTION_RE.captures_iter(inner) {
            let section = sec_caps.get(1).unwrap().as_str();
            let pin = sec_caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
            let raw = if pin.is_empty() {
                format!("ORS {}", section)
            } else {
                format!("ORS {} {}", section, pin)
            };

            let normalized = if pin.is_empty() {
                format!("ORS {}", section)
            } else {
                format!("ORS {}{}", section, pin.replace(' ', ""))
            };

            out.push(CitationMention {
                citation_mention_id: citation_id(
                    &provision.provision_id,
                    &format!("{}::{}", raw_group, raw),
                ),
                source_provision_id: provision.provision_id.clone(),
                raw_text: raw,
                normalized_citation: normalized,
                citation_type: if pin.is_empty() {
                    "statute_section".to_string()
                } else {
                    "statute_subsection".to_string()
                },
                target_canonical_id: Some(format!("or:ors:{}", section)),
                target_start_canonical_id: None,
                target_end_canonical_id: None,
                target_provision_id: None,
                unresolved_subpath: None,
                resolver_status: "parsed_unverified".to_string(),
                confidence: 0.92,
                qc_severity: None,
            });
        }
    }

    dedupe(out)
}

fn citation_id(source_provision_id: &str, raw: &str) -> String {
    format!(
        "cite:{}",
        stable_id(&format!("{}::{}", source_provision_id, raw))
    )
}

fn dedupe(rows: Vec<CitationMention>) -> Vec<CitationMention> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();

    for row in rows {
        let key = (
            row.source_provision_id.clone(),
            row.normalized_citation.clone(),
            row.citation_type.clone(),
        );

        if seen.insert(key) {
            out.push(row);
        }
    }

    out
}

fn overlaps_any(span: (usize, usize), occupied: &[(usize, usize)]) -> bool {
    occupied
        .iter()
        .any(|existing| span.0 < existing.1 && existing.0 < span.1)
}

#[cfg(test)]
mod tests {
    use super::extract_citation_mentions;
    use crate::models::Provision;

    fn provision(text: &str) -> Provision {
        Provision {
            provision_id: "or:ors:1.001@2025::p:1".to_string(),
            text: text.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn range_mentions_do_not_duplicate_section_mentions() {
        let mentions = extract_citation_mentions(&provision("See ORS 1.001 to 1.003."));
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].citation_type, "statute_range");
        assert_eq!(mentions[0].normalized_citation, "ORS 1.001 to 1.003");
    }

    #[test]
    fn extracts_through_grouped_subsections_and_chapter_ranges() {
        let mentions = extract_citation_mentions(&provision(
            "See ORS 1.001 through 1.003, ORS 2.010(1)(a) and 2.020, and ORS chapters 3 to 5.",
        ));
        let normalized = mentions
            .iter()
            .map(|m| (m.citation_type.as_str(), m.normalized_citation.as_str()))
            .collect::<Vec<_>>();

        assert!(normalized.contains(&("statute_range", "ORS 1.001 to 1.003")));
        assert!(normalized.contains(&("statute_subsection", "ORS 2.010(1)(a)")));
        assert!(normalized.contains(&("statute_section", "ORS 2.020")));
        assert!(normalized.contains(&("statute_chapter_range", "ORS chapters 3 to 5")));
    }
}
