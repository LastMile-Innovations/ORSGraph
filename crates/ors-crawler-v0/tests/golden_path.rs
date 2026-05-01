use ors_crawler_v0::ors_dom_parser::parse_ors_chapter_html;

#[test]
fn golden_path_ors002_parsing() {
    let html = r#"
        <html>
          <body>
            <p class="MsoNormal"><b>2.010 Supreme Court.</b></p>
            <p class="MsoNormal">The Supreme Court exercises judicial power and cites ORS 2.020 for this fixture.</p>
            <p class="MsoNormal">(1) The court may sit in departments for assigned matters.</p>
            <p class="MsoNormal"><b>2.020 Terms of court.</b></p>
            <p class="MsoNormal">A term of the Supreme Court may be held at Salem.</p>
          </body>
        </html>
    "#;

    let parsed = parse_ors_chapter_html(
        html,
        "https://www.oregonlegislature.gov/bills_laws/ors/ors002.html",
        "2",
        2024,
    )
    .expect("parse synthetic Chapter 2 excerpt");

    assert!(!parsed.provisions.is_empty(), "should have provisions");
    assert!(!parsed.versions.is_empty(), "should have versions");
    assert!(!parsed.chunks.is_empty(), "should have chunks");

    let sec_2_010 = parsed
        .versions
        .iter()
        .find(|version| version.canonical_id == "or:ors:2.010")
        .expect("find ORS 2.010");
    assert!(sec_2_010.text.contains("Supreme Court"));

    assert!(
        parsed
            .citations
            .iter()
            .any(|citation| citation.raw_text == "ORS 2.020"),
        "should extract fixture citation"
    );
}
