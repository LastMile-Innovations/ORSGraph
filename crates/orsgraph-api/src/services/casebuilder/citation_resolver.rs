use super::work_product_ast::sanitize_path_segment;
use crate::models::casebuilder::*;
use regex::Regex;
use std::sync::LazyLock;

static ORS_CITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\bORS\s+(?:chapters?\s+)?[0-9]{1,3}[A-Z]?(?:\.[0-9]{3,4})?(?:\s*(?:to|through)\s*[0-9]{1,3}[A-Z]?(?:\.[0-9]{3,4})?)?(?:\([^)]+\))*",
    )
    .unwrap()
});
static ORCP_CITATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bORCP\s+[0-9]+[A-Z]?(?:\s*[A-Z])?(?:\([^)]+\))*").unwrap());
static UTCR_CITATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bUTCR\s+[0-9]+(?:\.[0-9]+)?(?:\([^)]+\))*").unwrap());
static SESSION_LAW_CITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:Or(?:egon)?\.?\s+Laws?|Oregon Laws)\s+([0-9]{4}),?\s+ch(?:apter)?\.?\s*([0-9]+)(?:,?\s*(?:§|sec(?:tion)?\.?)\s*([0-9A-Za-z.-]+))?",
    )
    .unwrap()
});

pub(crate) fn canonical_id_for_citation(citation: &str) -> Option<String> {
    let normalized = citation.split_whitespace().collect::<Vec<_>>().join(" ");
    let upper = normalized.to_ascii_uppercase();
    if upper.starts_with("ORS CHAPTER") || upper.starts_with("ORS CHAPTERS") {
        let chapter = normalized
            .split_whitespace()
            .find(|part| part.chars().any(|ch| ch.is_ascii_digit()))?
            .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        return Some(format!("or:ors:chapter:{chapter}"));
    }
    if upper.starts_with("ORS ") {
        let section = normalized
            .split_whitespace()
            .nth(1)?
            .split('(')
            .next()
            .unwrap_or_default()
            .trim_end_matches(',');
        if section.contains('.') {
            return Some(format!("or:ors:{section}"));
        }
    }
    if upper.starts_with("ORCP ") {
        return Some(format!(
            "or:orcp:{}",
            sanitize_path_segment(&normalized[5..].trim().to_ascii_lowercase())
        ));
    }
    if upper.starts_with("UTCR ") {
        let rule = normalized[5..]
            .trim()
            .split('(')
            .next()
            .unwrap_or_default()
            .trim_end_matches(',');
        return Some(format!(
            "or:utcr:{}",
            sanitize_path_segment(&rule.to_ascii_lowercase())
        ));
    }
    if upper.starts_with("OR LAWS") || upper.starts_with("OREGON LAWS") {
        if let Some(captures) = SESSION_LAW_CITATION_RE.captures(&normalized) {
            let year = captures.get(1).map(|value| value.as_str())?;
            let chapter = captures.get(2).map(|value| value.as_str())?;
            let section = captures.get(3).map(|value| {
                format!(
                    ":sec:{}",
                    sanitize_path_segment(&value.as_str().to_ascii_lowercase())
                )
            });
            return Some(format!(
                "or:session-law:{year}:ch:{chapter}{}",
                section.unwrap_or_default()
            ));
        }
    }
    None
}

pub(crate) fn work_product_citations_for_text(
    work_product_id: &str,
    block_id: &str,
    text: &str,
    created_at: &str,
) -> Vec<WorkProductCitationUse> {
    let mut citations = Vec::new();
    for (index, citation_match) in ORS_CITATION_RE
        .find_iter(text)
        .chain(ORCP_CITATION_RE.find_iter(text))
        .chain(UTCR_CITATION_RE.find_iter(text))
        .chain(SESSION_LAW_CITATION_RE.find_iter(text))
        .enumerate()
    {
        let citation = citation_match
            .as_str()
            .trim()
            .trim_end_matches('.')
            .to_string();
        let canonical_id = canonical_id_for_citation(&citation);
        let (start_offset, end_offset) =
            char_offsets_for_byte_range(text, citation_match.start(), citation_match.end());
        citations.push(WorkProductCitationUse {
            citation_use_id: format!(
                "{}:citation:{}:{}:{}",
                block_id,
                index + 1,
                start_offset,
                sanitize_path_segment(&citation)
            ),
            source_block_id: block_id.to_string(),
            source_text_range: Some(TextRange {
                start_offset,
                end_offset,
                quote: Some(citation.clone()),
            }),
            raw_text: citation.clone(),
            normalized_citation: Some(citation.clone()),
            target_type: "provision".to_string(),
            target_id: canonical_id,
            pinpoint: None,
            status: if canonical_id_for_citation(&citation).is_some() {
                "resolved".to_string()
            } else {
                "unresolved".to_string()
            },
            resolver_message: Some(format!("Resolved by {work_product_id} citation resolver.")),
            created_at: created_at.to_string(),
        });
    }
    citations
}

fn char_offsets_for_byte_range(text: &str, start: usize, end: usize) -> (u64, u64) {
    let start_offset = text[..start].chars().count() as u64;
    let end_offset = start_offset + text[start..end].chars().count() as u64;
    (start_offset, end_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_ids_cover_oregon_authorities() {
        assert_eq!(
            canonical_id_for_citation("ORS 90.320"),
            Some("or:ors:90.320".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("ORCP 16 D"),
            Some("or:orcp:16-d".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("UTCR 2.010(4)"),
            Some("or:utcr:2.010".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("Or Laws 2023, ch 13, § 4"),
            Some("or:session-law:2023:ch:13:sec:4".to_string())
        );
    }

    #[test]
    fn work_product_citations_include_stable_text_ranges_for_each_use() {
        let citations = work_product_citations_for_text(
            "wp:test",
            "block:1",
            "ORS 90.320 applies. ORS 90.320 also appears.",
            "1",
        );
        assert_eq!(citations.len(), 2);
        assert_ne!(citations[0].citation_use_id, citations[1].citation_use_id);
        assert_eq!(
            citations[0]
                .source_text_range
                .as_ref()
                .unwrap()
                .quote
                .as_deref(),
            Some("ORS 90.320")
        );
        assert_eq!(citations[0].status, "resolved");
    }
}
