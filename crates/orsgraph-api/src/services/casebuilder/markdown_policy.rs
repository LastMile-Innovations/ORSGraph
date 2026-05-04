pub(crate) fn is_generated_review_notice_line(line: &str) -> bool {
    let trimmed = line.trim();
    let notice = trimmed.strip_prefix('>').map(str::trim).unwrap_or(trimmed);
    matches!(
        notice,
        "Review needed; not legal advice or filing-ready."
            | "Review needed; not legal advice or filing-ready status."
    )
}

#[cfg(test)]
mod tests {
    use super::is_generated_review_notice_line;

    #[test]
    fn generated_review_notice_matches_plain_and_quoted_lines() {
        assert!(is_generated_review_notice_line(
            "Review needed; not legal advice or filing-ready."
        ));
        assert!(is_generated_review_notice_line(
            "> Review needed; not legal advice or filing-ready status."
        ));
        assert!(!is_generated_review_notice_line(
            "> Review needed; confirm the exhibit number."
        ));
    }
}
