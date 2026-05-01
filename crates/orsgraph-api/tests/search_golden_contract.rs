use std::time::Instant;

#[derive(Debug, Default)]
struct GoldenCase {
    name: String,
    query: String,
    expected_top_citation: Option<String>,
    expected_any_citations: Vec<String>,
    expected_semantic_types: Vec<String>,
    expected_source_backed: bool,
}

#[test]
fn golden_search_suite_has_required_metrics_and_queries() {
    let suite = include_str!("../../../golden/search_queries.yaml");

    for expected in [
        "top_1_exact_hit",
        "top_5_contains_expected_statute",
        "top_10_contains_expected_provision",
        "source_backed_coverage",
        "rerank_lift",
        "latency_ms",
    ] {
        assert!(suite.contains(expected), "missing golden metric {expected}");
    }

    let cases = parse_golden_cases(suite);
    assert!(
        cases.len() >= 8,
        "expected a representative golden query set"
    );
    assert!(
        cases
            .iter()
            .any(|case| case.expected_top_citation.is_some()),
        "at least one golden case must assert top-1 citation behavior"
    );
    assert!(
        cases
            .iter()
            .any(|case| !case.expected_semantic_types.is_empty()),
        "at least one golden case must assert semantic-type recall"
    );
}

#[tokio::test]
async fn live_golden_search_suite() {
    if std::env::var("ORSGRAPH_RUN_LIVE_GOLDEN").ok().as_deref() != Some("1") {
        return;
    }

    let suite = include_str!("../../../golden/search_queries.yaml");
    let cases = parse_golden_cases(suite);
    let base_url = std::env::var("ORSGRAPH_API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080/api/v1".to_string());
    let latency_budget_ms = std::env::var("ORSGRAPH_GOLDEN_LATENCY_MS")
        .ok()
        .and_then(|value| value.parse::<u128>().ok())
        .unwrap_or(3000);
    let client = reqwest::Client::new();

    for case in cases {
        let started = Instant::now();
        let response = client
            .get(format!("{base_url}/search"))
            .query(&[
                ("q", case.query.as_str()),
                ("mode", "hybrid"),
                ("limit", "10"),
            ])
            .send()
            .await
            .expect("golden search request succeeds");

        assert!(
            response.status().is_success(),
            "{} returned {}",
            case.name,
            response.status()
        );
        let elapsed = started.elapsed().as_millis();
        assert!(
            elapsed <= latency_budget_ms,
            "{} exceeded latency budget: {elapsed}ms > {latency_budget_ms}ms",
            case.name
        );

        let body: serde_json::Value = response.json().await.expect("search JSON response");
        assert!(
            body["analysis"]["normalized_query"].is_string(),
            "{} response should include required analysis.normalized_query",
            case.name
        );
        assert!(
            body["analysis"]["timings"]["total_ms"].is_number(),
            "{} response should include analysis timings",
            case.name
        );
        let results = body["results"]
            .as_array()
            .unwrap_or_else(|| panic!("{} response should include results", case.name));

        if let Some(expected) = case.expected_top_citation.as_deref() {
            let top = results
                .first()
                .and_then(|result| result["citation"].as_str())
                .unwrap_or_default();
            assert_eq!(top, expected, "{} top citation mismatch", case.name);
        }

        for expected in &case.expected_any_citations {
            assert!(
                results
                    .iter()
                    .any(|result| result["citation"].as_str() == Some(expected.as_str())),
                "{} missing expected citation {expected}",
                case.name
            );
        }

        for expected in &case.expected_semantic_types {
            assert!(
                results.iter().any(|result| {
                    result["semantic_types"]
                        .as_array()
                        .map(|types| {
                            types
                                .iter()
                                .any(|value| value.as_str() == Some(expected.as_str()))
                        })
                        .unwrap_or(false)
                }),
                "{} missing expected semantic type {expected}",
                case.name
            );
        }

        if case.expected_source_backed {
            assert!(
                results
                    .iter()
                    .any(|result| result["source_backed"].as_bool() == Some(true)),
                "{} expected at least one source-backed result",
                case.name
            );
        }
    }

    let direct: serde_json::Value = client
        .get(format!("{base_url}/search/open"))
        .query(&[("q", "ORS 90.300")])
        .send()
        .await
        .expect("direct-open request succeeds")
        .json()
        .await
        .expect("direct-open JSON response");
    assert_eq!(direct["matched"].as_bool(), Some(true));
    assert!(
        matches!(
            direct["match_type"].as_str(),
            Some("exact_statute" | "exact_provision")
        ),
        "direct-open should return a structured exact match"
    );

    let suggestions: serde_json::Value = client
        .get(format!("{base_url}/search/suggest"))
        .query(&[("q", "90.3"), ("limit", "5")])
        .send()
        .await
        .expect("suggest request succeeds")
        .json()
        .await
        .expect("suggest JSON response");
    let suggestions = suggestions
        .as_array()
        .expect("suggest response should be an array");
    assert!(
        suggestions.iter().any(|suggestion| {
            suggestion["href"]
                .as_str()
                .unwrap_or_default()
                .starts_with("/statutes/")
                && suggestion["match_type"].as_str().is_some()
        }),
        "suggest should include direct-open-ready authority suggestions"
    );
}

fn parse_golden_cases(suite: &str) -> Vec<GoldenCase> {
    let mut cases = Vec::new();
    let mut current: Option<GoldenCase> = None;
    let mut section = "";
    let mut list_key = "";

    for line in suite.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("- name:") {
            if let Some(case) = current.take() {
                cases.push(case);
            }
            current = Some(GoldenCase {
                name: strip_yaml_string(name),
                ..GoldenCase::default()
            });
            section = "";
            list_key = "";
            continue;
        }

        let Some(case) = current.as_mut() else {
            continue;
        };

        if let Some(query) = trimmed.strip_prefix("query:") {
            case.query = strip_yaml_string(query);
        } else if trimmed == "expected_top_result:" {
            section = "expected_top_result";
            list_key = "";
        } else if trimmed == "expected_any:" {
            section = "expected_any";
            list_key = "";
        } else if let Some(citation) = trimmed.strip_prefix("citation:") {
            if section == "expected_top_result" {
                case.expected_top_citation = Some(strip_yaml_string(citation));
            }
        } else if trimmed == "citations:" {
            list_key = "citations";
        } else if trimmed == "semantic_types:" {
            list_key = "semantic_types";
        } else if let Some(value) = trimmed.strip_prefix("- ") {
            match list_key {
                "citations" => case.expected_any_citations.push(strip_yaml_string(value)),
                "semantic_types" => case.expected_semantic_types.push(strip_yaml_string(value)),
                _ => {}
            }
        } else if let Some(value) = trimmed.strip_prefix("source_backed:") {
            case.expected_source_backed = strip_yaml_string(value) == "true";
        }
    }

    if let Some(case) = current {
        cases.push(case);
    }
    cases
}

fn strip_yaml_string(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}
