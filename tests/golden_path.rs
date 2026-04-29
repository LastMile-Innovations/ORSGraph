use ors_crawler_v0::ors_dom_parser::parse_ors_chapter_html;
use std::fs;

#[test]
fn golden_path_ors002_parsing() {
    let path = "/Users/grey/ORSGraph/data/raw/official/ors002.html";
    let bytes = fs::read(path).expect("Read ors002.html");
    let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
    let html = cow.to_string();

    let parsed = parse_ors_chapter_html(
        &html,
        "https://www.oregonlegislature.gov/bills_laws/ors/ors002.html",
        "2",
        2024,
    )
    .expect("Parse Chapter 2");

    // Basic structural assertions
    assert!(!parsed.provisions.is_empty(), "Should have provisions");
    assert!(!parsed.versions.is_empty(), "Should have versions");
    assert!(!parsed.chunks.is_empty(), "Should have chunks");

    // Spot check a specific section
    let sec_2_010 = parsed
        .versions
        .iter()
        .find(|v| v.canonical_id == "or:ors:2.010")
        .expect("Find ORS 2.010");
    assert!(sec_2_010.text.contains("Supreme Court"));

    // Check citation extraction
    assert!(
        !parsed.citations.is_empty(),
        "Should have extracted citations"
    );
}
