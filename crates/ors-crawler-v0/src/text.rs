use regex::Regex;
use std::sync::LazyLock;

static REPLACEMENT_SECTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\u{FFFD}\s*(\d)").unwrap());
static REPLACEMENT_CHAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\u{FFFD}+").unwrap());
static RULE_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^_{5,}\s*").unwrap());
static RULE_SUFFIX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*_{5,}$").unwrap());
static RULE_LINE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^_{5,}$").unwrap());
static RESERVED_TAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?:titles?\s+\d+[A-Z]?(?:(?:\s+to|\s+and)\s+\d+[A-Z]?)?(?:\s+et\s+seq\.)?|chapters?\s+\d+[A-Z]?(?:(?:\s+to|\s+and)\s+\d+[A-Z]?)?(?:\s+et\s+seq\.)?)$",
    )
    .unwrap()
});
static RESERVED_TAIL_LINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?:titles?\s+\d+[A-Z]?(?:(?:\s+to|\s+and)\s+\d+[A-Z]?)?(?:\s+et\s+seq\.)?|chapters?\s+\d+[A-Z]?(?:(?:\s+to|\s+and)\s+\d+[A-Z]?)?(?:\s+et\s+seq\.)?)\s+\[reserved for expansion\]_*$",
    )
    .unwrap()
});
static RESERVED_TAIL_SUFFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\s+(?:titles?\s+\d+[A-Z]?(?:(?:\s+to|\s+and)\s+\d+[A-Z]?)?(?:\s+et\s+seq\.)?|chapters?\s+\d+[A-Z]?(?:(?:\s+to|\s+and)\s+\d+[A-Z]?)?(?:\s+et\s+seq\.)?)\s+\[reserved for expansion\]_*\s*$",
    )
    .unwrap()
});
static WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t\r\n]+").unwrap());
static NBSP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\u{00A0}\u{2007}\u{202F}]").unwrap());

pub fn normalize_ws(s: &str) -> String {
    let decoded = html_escape::decode_html_entities(s).to_string();
    let decoded = trim_leading_marker_garbage(&decoded);
    let decoded = REPLACEMENT_SECTION_RE
        .replace_all(&decoded, "§$1")
        .to_string();
    let decoded = REPLACEMENT_CHAR_RE.replace_all(&decoded, "").to_string();
    let s = NBSP_RE.replace_all(&decoded, " ");
    WS_RE.replace_all(s.trim(), " ").trim().to_string()
}

fn trim_leading_marker_garbage(s: &str) -> String {
    let trimmed = s.trim_start_matches(|c| c == '\u{FFFD}' || c == '\u{00A7}' || c == ' ');
    if matches!(trimmed.chars().next(), Some(c) if c.is_ascii_digit() || c == '(') {
        trimmed.to_string()
    } else {
        s.to_string()
    }
}

pub fn normalize_for_hash(s: &str) -> String {
    normalize_ws(s).to_lowercase()
}

pub fn strip_trailing_period(s: &str) -> String {
    s.trim().trim_end_matches('.').trim().to_string()
}

pub fn is_blank(s: &str) -> bool {
    let t = normalize_ws(s);
    t.is_empty() || t == "&nbsp;" || t == "\u{00A0}"
}

pub fn is_rule_line(s: &str) -> bool {
    RULE_LINE_RE.is_match(normalize_ws(s).as_str())
}

pub fn is_reserved_tail_heading(s: &str) -> bool {
    let normalized = normalize_ws(s);
    RESERVED_TAIL_RE.is_match(normalized.as_str())
        || RESERVED_TAIL_LINE_RE.is_match(normalized.as_str())
}

pub fn is_reserved_expansion_text(s: &str) -> bool {
    normalize_ws(s).eq_ignore_ascii_case("[Reserved for expansion]")
}

pub fn clean_parser_text(s: &str) -> String {
    let normalized = normalize_ws(s);
    if RESERVED_TAIL_LINE_RE.is_match(&normalized) {
        return String::new();
    }
    let normalized = RULE_PREFIX_RE.replace(&normalized, "").to_string();
    let normalized = RULE_SUFFIX_RE.replace(&normalized, "").to_string();
    let normalized = RESERVED_TAIL_SUFFIX_RE.replace(&normalized, "").to_string();
    normalized.trim().to_string()
}

pub fn count_rule_line_artifacts(text: &str) -> usize {
    text.lines().filter(|line| is_rule_line(line)).count()
}

pub fn is_all_caps_heading(s: &str) -> bool {
    let t = normalize_ws(s);
    if t.len() < 4 {
        return false;
    }

    if t.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }

    let letters: Vec<char> = t.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() < 3 {
        return false;
    }

    letters.iter().all(|c| !c.is_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{
        clean_parser_text, count_rule_line_artifacts, is_reserved_expansion_text,
        is_reserved_tail_heading, is_rule_line, normalize_ws,
    };

    #[test]
    fn normalizes_word_export_garbage() {
        assert_eq!(normalize_ws("����� 830.080"), "830.080");
        assert_eq!(normalize_ws("[1991 c.590 �5]"), "[1991 c.590 §5]");
        assert_eq!(
            normalize_ws("As used in this compact, �state� means a state."),
            "As used in this compact, state means a state."
        );
    }

    #[test]
    fn detects_layout_artifacts() {
        assert!(is_rule_line("_______________"));
        assert!(is_reserved_tail_heading("CHAPTERS 831 TO 834"));
        assert!(is_reserved_tail_heading("CHAPTERS 256 AND 257"));
        assert!(is_reserved_tail_heading("TITLES 63 et seq."));
        assert!(is_reserved_tail_heading(
            "CHAPTERS 102 TO 104 [Reserved for expansion]"
        ));
        assert!(is_reserved_tail_heading(
            "CHAPTER 531 [Reserved for expansion]_"
        ));
        assert!(is_reserved_expansion_text("[Reserved for expansion]"));
        assert_eq!(clean_parser_text("  _______________  "), "");
        assert_eq!(
            clean_parser_text("_______________ [1991 c.590 §5]"),
            "[1991 c.590 §5]"
        );
        assert_eq!(
            clean_parser_text("CHAPTERS 102 TO 104 [Reserved for expansion]"),
            ""
        );
        assert_eq!(
            clean_parser_text("CHAPTER 531 [Reserved for expansion]_"),
            ""
        );
        assert_eq!(
            clean_parser_text(
                "This chapter may be cited as the Act. CHAPTERS 102 TO 104 [Reserved for expansion]"
            ),
            "This chapter may be cited as the Act."
        );
    }

    #[test]
    fn counts_only_standalone_rule_lines_as_artifacts() {
        assert_eq!(
            count_rule_line_artifacts("TO: _______________ (Garnishee)."),
            0
        );
        assert_eq!(count_rule_line_artifacts("foo\n_______________\nbar"), 1);
    }

    #[test]
    fn test_more_text_utils() {
        use super::{is_all_caps_heading, is_blank, normalize_for_hash, strip_trailing_period};
        assert_eq!(normalize_for_hash("  Test  "), "test");
        assert_eq!(strip_trailing_period("foo."), "foo");
        assert!(is_blank("  "));
        assert!(is_all_caps_heading("THIS IS A HEADING"));
        assert!(!is_all_caps_heading("This is not"));
        assert!(!is_all_caps_heading("A")); // Too short
    }
}
