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
static US_CONST_CITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\bU\.?\s*S\.?\s+Const\.?\s+(?:(?:art\.?)\s+(?:[IVXLCDM]+|\d+)(?:,\s*§+\s*\d+)?(?:,\s*cl\.?\s*\d+)?|(?:amend\.?)\s+(?:[IVXLCDM]+|\d+)(?:,\s*§+\s*\d+)?)",
    )
    .unwrap()
});
static CONAN_CITATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:Amdt\d+|Art[IVXLCDM]+|Art\d+)[A-Za-z0-9.]+\b").unwrap());
static NAMED_AMENDMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(First|Second|Third|Fourth|Fifth|Sixth|Seventh|Eighth|Ninth|Tenth|Eleventh|Twelfth|Thirteenth|Fourteenth|Fifteenth|Sixteenth|Seventeenth|Eighteenth|Nineteenth|Twentieth|Twenty-First|Twenty-Second|Twenty-Third|Twenty-Fourth|Twenty-Fifth|Twenty-Sixth|Twenty-Seventh)\s+Amendment\b",
    )
    .unwrap()
});
static DUE_PROCESS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bDue Process Clause\b").unwrap());
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
    if upper.starts_with("U.S. CONST") || upper.starts_with("US CONST") {
        return canonical_us_constitution_id(&normalized);
    }
    if upper.starts_with("AMDT") || upper.starts_with("ART") {
        return Some(format!("us:conan:{}", normalize_conan_serial(&normalized)));
    }
    if upper == "DUE PROCESS CLAUSE" {
        return Some("us:constitution:amendment-14:section-1".to_string());
    }
    if let Some(captures) = NAMED_AMENDMENT_RE.captures(&normalized) {
        let number = ordinal_amendment_number(captures.get(1)?.as_str())?;
        return Some(format!("us:constitution:amendment-{number}"));
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
        .chain(US_CONST_CITATION_RE.find_iter(text))
        .chain(CONAN_CITATION_RE.find_iter(text))
        .chain(NAMED_AMENDMENT_RE.find_iter(text))
        .chain(DUE_PROCESS_RE.find_iter(text))
        .chain(SESSION_LAW_CITATION_RE.find_iter(text))
        .enumerate()
    {
        let citation = citation_match
            .as_str()
            .trim()
            .trim_end_matches('.')
            .to_string();
        let canonical_id = canonical_id_for_citation(&citation);
        let target_type = canonical_id
            .as_deref()
            .map(|target| {
                if target.starts_with("us:conan:") {
                    "commentary"
                } else {
                    "provision"
                }
            })
            .unwrap_or("provision");
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
            target_type: target_type.to_string(),
            target_id: canonical_id.clone(),
            pinpoint: None,
            status: if canonical_id.is_some() {
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

fn canonical_us_constitution_id(citation: &str) -> Option<String> {
    let article_re = Regex::new(
        r"(?i)\bU\.?\s*S\.?\s+Const\.?\s+art\.?\s+([IVXLCDM]+|\d+)(?:,\s*§+\s*(\d+))?(?:,\s*cl\.?\s*(\d+))?",
    )
    .unwrap();
    if let Some(captures) = article_re.captures(citation) {
        let article = roman_or_decimal_to_u32(captures.get(1)?.as_str())?;
        let mut id = format!("us:constitution:article-{article}");
        if let Some(section) = captures.get(2).map(|value| value.as_str()) {
            id.push_str(&format!(":section-{section}"));
        }
        if let Some(clause) = captures.get(3).map(|value| value.as_str()) {
            id.push_str(&format!(":clause-{clause}"));
        }
        return Some(id);
    }

    let amendment_re = Regex::new(
        r"(?i)\bU\.?\s*S\.?\s+Const\.?\s+amend\.?\s+([IVXLCDM]+|\d+)(?:,\s*§+\s*(\d+))?",
    )
    .unwrap();
    let captures = amendment_re.captures(citation)?;
    let amendment = roman_or_decimal_to_u32(captures.get(1)?.as_str())?;
    let mut id = format!("us:constitution:amendment-{amendment}");
    if let Some(section) = captures.get(2).map(|value| value.as_str()) {
        id.push_str(&format!(":section-{section}"));
    }
    Some(id)
}

fn normalize_conan_serial(value: &str) -> String {
    value
        .split('.')
        .filter(|part| !part.is_empty())
        .enumerate()
        .map(|(index, part)| {
            let lower = part.to_ascii_lowercase();
            if index == 0 {
                if let Some(rest) = lower.strip_prefix("amdt") {
                    format!("Amdt{rest}")
                } else if let Some(rest) = lower.strip_prefix("art") {
                    format!("Art{}", rest.to_ascii_uppercase())
                } else {
                    part.to_string()
                }
            } else {
                part.to_ascii_uppercase()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
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

fn ordinal_amendment_number(value: &str) -> Option<u32> {
    match value.trim().to_ascii_lowercase().replace(' ', "-").as_str() {
        "first" => Some(1),
        "second" => Some(2),
        "third" => Some(3),
        "fourth" => Some(4),
        "fifth" => Some(5),
        "sixth" => Some(6),
        "seventh" => Some(7),
        "eighth" => Some(8),
        "ninth" => Some(9),
        "tenth" => Some(10),
        "eleventh" => Some(11),
        "twelfth" => Some(12),
        "thirteenth" => Some(13),
        "fourteenth" => Some(14),
        "fifteenth" => Some(15),
        "sixteenth" => Some(16),
        "seventeenth" => Some(17),
        "eighteenth" => Some(18),
        "nineteenth" => Some(19),
        "twentieth" => Some(20),
        "twenty-first" => Some(21),
        "twenty-second" => Some(22),
        "twenty-third" => Some(23),
        "twenty-fourth" => Some(24),
        "twenty-fifth" => Some(25),
        "twenty-sixth" => Some(26),
        "twenty-seventh" => Some(27),
        _ => None,
    }
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
        assert_eq!(
            canonical_id_for_citation("U.S. Const. art. I, § 8, cl. 3"),
            Some("us:constitution:article-1:section-8:clause-3".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("U.S. Const. amend. XIV, § 1"),
            Some("us:constitution:amendment-14:section-1".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("Fourteenth Amendment"),
            Some("us:constitution:amendment-14".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("Due Process Clause"),
            Some("us:constitution:amendment-14:section-1".to_string())
        );
        assert_eq!(
            canonical_id_for_citation("Amdt14.S1.5.1"),
            Some("us:conan:Amdt14.S1.5.1".to_string())
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
