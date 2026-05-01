use crate::hash::{sha256_hex, stable_id};
use crate::models::{
    Court, CourtRulesRegistrySnapshot, CourtRulesRegistrySource, EffectiveInterval, Jurisdiction,
    ParserDiagnostic, RuleApplicabilityEdge, RuleAuthorityDocument, RulePublicationEntry,
    RuleSupersessionEdge, RuleTopic, SupplementaryLocalRuleEdition, WorkProductRulePackAuthority,
};
use anyhow::{anyhow, Result};
use chrono::{Datelike, NaiveDate};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::LazyLock;

const PARSER_PROFILE: &str = "court_rules_registry_parser_v1";

static CJO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bCJO\s+([0-9]{2,4}-[0-9]{3})\b").unwrap());
static PJO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bPJO\s*([0-9]{2,4}(?:-[0-9]{3})?)\b").unwrap());
static SLR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bSLR\s+([0-9]+(?:\.[0-9]+)?)\b").unwrap());
static SUPERSEDES_CJO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)supersed(?:e|ing)\s+CJO\s+([0-9]{2,4}-[0-9]{3})").unwrap());
static DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[0-9]{1,2}/[0-9]{1,2}/[0-9]{4}\b").unwrap());

#[derive(Debug, Clone)]
pub struct CourtRulesRegistryParseConfig {
    pub jurisdiction: String,
    pub snapshot_date: String,
    pub source_url: String,
    pub state_id: String,
    pub state_name: String,
    pub base_rule_corpus_id: String,
    pub court_id: Option<String>,
    pub court_name: Option<String>,
    pub judicial_district_id: Option<String>,
    pub judicial_district_name: Option<String>,
}

impl CourtRulesRegistryParseConfig {
    pub fn oregon(jurisdiction: String, snapshot_date: String, source_url: String) -> Self {
        Self {
            jurisdiction,
            snapshot_date,
            source_url,
            state_id: "or:state".to_string(),
            state_name: "Oregon".to_string(),
            base_rule_corpus_id: "or:utcr".to_string(),
            court_id: None,
            court_name: None,
            judicial_district_id: None,
            judicial_district_name: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct ParsedCourtRulesRegistry {
    pub registry_sources: Vec<CourtRulesRegistrySource>,
    pub registry_snapshots: Vec<CourtRulesRegistrySnapshot>,
    pub publication_entries: Vec<RulePublicationEntry>,
    pub jurisdictions: Vec<Jurisdiction>,
    pub courts: Vec<Court>,
    pub authority_documents: Vec<RuleAuthorityDocument>,
    pub chief_justice_orders: Vec<RuleAuthorityDocument>,
    pub presiding_judge_orders: Vec<RuleAuthorityDocument>,
    pub supplementary_local_rule_editions: Vec<SupplementaryLocalRuleEdition>,
    pub out_of_cycle_amendments: Vec<RuleAuthorityDocument>,
    pub effective_intervals: Vec<EffectiveInterval>,
    pub rule_topics: Vec<RuleTopic>,
    pub rule_supersession_edges: Vec<RuleSupersessionEdge>,
    pub rule_applicability_edges: Vec<RuleApplicabilityEdge>,
    pub work_product_rule_pack_authorities: Vec<WorkProductRulePackAuthority>,
    pub parser_diagnostics: Vec<ParserDiagnostic>,
}

#[derive(Debug, Clone)]
struct ParsedRow {
    title: String,
    jurisdiction: String,
    jurisdiction_id: String,
    subcategory: String,
    authority_kind: String,
    publication_bucket: String,
    table_section: String,
    row_index: usize,
    effective_start_date: String,
    effective_end_date: Option<String>,
    authority_identifier: Option<String>,
    date_status: String,
    status_flags: Vec<String>,
}

pub fn parse_court_rules_registry_text(
    input: &str,
    config: CourtRulesRegistryParseConfig,
) -> Result<ParsedCourtRulesRegistry> {
    let snapshot_date = parse_iso_date(&config.snapshot_date)?;
    let jurisdiction_id = normalize_jurisdiction_id(&config.jurisdiction, &config.state_id);
    let court_id = local_court_id(&config, &jurisdiction_id);
    let registry_source_id = format!(
        "{}:courts:slr_registry:{}:snapshot:{}",
        state_prefix(&config.state_id),
        jurisdiction_id
            .trim_start_matches(&format!("{}:", state_prefix(&config.state_id)))
            .replace(':', "_"),
        snapshot_date.year()
    );
    let registry_snapshot_id = format!("{}:{}", registry_source_id, config.snapshot_date);

    let mut parsed = ParsedCourtRulesRegistry {
        registry_sources: vec![CourtRulesRegistrySource {
            registry_source_id: registry_source_id.clone(),
            source_type: "court_rules_registry".to_string(),
            jurisdiction: config.jurisdiction.clone(),
            jurisdiction_id: jurisdiction_id.clone(),
            source_url: config.source_url.clone(),
            snapshot_date: config.snapshot_date.clone(),
            contains_current_future: input.contains("Current and Future Rules"),
            contains_prior: input.contains("Prior Rules"),
        }],
        registry_snapshots: vec![CourtRulesRegistrySnapshot {
            registry_snapshot_id: registry_snapshot_id.clone(),
            registry_source_id: registry_source_id.clone(),
            snapshot_date: config.snapshot_date.clone(),
            jurisdiction_id: jurisdiction_id.clone(),
            source_url: config.source_url.clone(),
            parser_profile: PARSER_PROFILE.to_string(),
            entry_count: 0,
            input_hash: sha256_hex(input),
        }],
        jurisdictions: base_jurisdictions(&config, &jurisdiction_id),
        courts: base_courts(&config, &jurisdiction_id),
        ..ParsedCourtRulesRegistry::default()
    };

    let mut section: Option<(&str, &str)> = None;
    let mut rows = Vec::<ParsedRow>::new();
    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("Current and Future Rules") {
            section = Some(("current_future", "Current and Future Rules"));
            continue;
        }
        if line.eq_ignore_ascii_case("Prior Rules") {
            section = Some(("prior", "Prior Rules"));
            continue;
        }
        if line.starts_with("Description")
            && line.contains("Jurisdiction")
            && line.contains("Effective Start Date")
        {
            continue;
        }
        let Some((publication_bucket, table_section)) = section else {
            continue;
        };
        if !is_table_like(line) {
            continue;
        }
        match parse_row(
            line,
            publication_bucket,
            table_section,
            rows.len() + 1,
            snapshot_date,
            &config.state_id,
        ) {
            Ok(row) => rows.push(row),
            Err(err) => parsed.parser_diagnostics.push(diagnostic(
                &registry_source_id,
                "warning",
                "registry_row_parse_failed",
                &format!("Could not parse registry row {}: {err}", line_index + 1),
                Some(line_index + 1),
                None,
            )),
        }
    }

    if rows.is_empty() {
        parsed.parser_diagnostics.push(diagnostic(
            &registry_source_id,
            "error",
            "registry_rows_missing",
            "No court rules registry table rows were parsed.",
            None,
            Some(registry_snapshot_id.clone()),
        ));
    }

    let mut topic_map = BTreeMap::<String, RuleTopic>::new();
    let mut slr_editions = Vec::<SupplementaryLocalRuleEdition>::new();
    let mut doc_index_by_identifier = HashMap::<String, String>::new();

    for row in rows {
        let authority_document_id = authority_document_id(&row);
        if let Some(identifier) = &row.authority_identifier {
            doc_index_by_identifier.insert(
                identifier.to_ascii_lowercase(),
                authority_document_id.clone(),
            );
        }
        let effective_interval_id =
            format!("effective_interval:{}", stable_id(&authority_document_id));
        let topic_ids = topics_for_title(&row.title)
            .into_iter()
            .map(|name| {
                let normalized = normalize_topic(&name);
                let id = format!("rule_topic:{}", normalized);
                topic_map.entry(id.clone()).or_insert_with(|| RuleTopic {
                    rule_topic_id: id.clone(),
                    name,
                    normalized_name: normalized,
                });
                id
            })
            .collect::<Vec<_>>();
        let amends_authority_document_id = if row.authority_kind == "OutOfCycleAmendment" {
            let start_year = row
                .effective_start_date
                .get(0..4)
                .map(str::to_string)
                .unwrap_or_else(|| snapshot_date.year().to_string());
            Some(format!("{}:slr@{start_year}", row.jurisdiction_id))
        } else {
            None
        };

        parsed.publication_entries.push(RulePublicationEntry {
            publication_entry_id: format!(
                "rule_publication_entry:{}",
                stable_id(&format!(
                    "{}::{}::{}",
                    registry_snapshot_id, row.row_index, authority_document_id
                ))
            ),
            registry_source_id: registry_source_id.clone(),
            registry_snapshot_id: registry_snapshot_id.clone(),
            authority_document_id: authority_document_id.clone(),
            effective_interval_id: effective_interval_id.clone(),
            title: row.title.clone(),
            jurisdiction: row.jurisdiction.clone(),
            jurisdiction_id: row.jurisdiction_id.clone(),
            subcategory: row.subcategory.clone(),
            authority_kind: row.authority_kind.clone(),
            publication_bucket: row.publication_bucket.clone(),
            table_section: row.table_section.clone(),
            row_index: row.row_index,
            effective_start_date: row.effective_start_date.clone(),
            effective_end_date: row.effective_end_date.clone(),
            date_status: row.date_status.clone(),
            status_flags: row.status_flags.clone(),
            authority_identifier: row.authority_identifier.clone(),
        });

        let doc = RuleAuthorityDocument {
            authority_document_id: authority_document_id.clone(),
            title: row.title.clone(),
            jurisdiction_id: row.jurisdiction_id.clone(),
            jurisdiction: row.jurisdiction.clone(),
            subcategory: row.subcategory.clone(),
            authority_kind: row.authority_kind.clone(),
            authority_identifier: row.authority_identifier.clone(),
            effective_start_date: row.effective_start_date.clone(),
            effective_end_date: row.effective_end_date.clone(),
            publication_bucket: row.publication_bucket.clone(),
            date_status: row.date_status.clone(),
            status_flags: row.status_flags.clone(),
            topic_ids,
            amends_authority_document_id,
            source_registry_id: registry_source_id.clone(),
            source_snapshot_id: registry_snapshot_id.clone(),
            source_url: config.source_url.clone(),
        };

        parsed.effective_intervals.push(EffectiveInterval {
            effective_interval_id,
            authority_document_id: authority_document_id.clone(),
            start_date: row.effective_start_date.clone(),
            end_date: row.effective_end_date.clone(),
            label: effective_label(&row.effective_start_date, row.effective_end_date.as_deref()),
            certainty: "official_registry".to_string(),
        });

        parsed.rule_applicability_edges.push(RuleApplicabilityEdge {
            edge_id: format!(
                "rule_applicability:{}",
                stable_id(&format!(
                    "{}::{}",
                    authority_document_id, row.jurisdiction_id
                ))
            ),
            authority_document_id: authority_document_id.clone(),
            jurisdiction_id: row.jurisdiction_id.clone(),
            court_id: if row.jurisdiction_id == config.state_id {
                None
            } else {
                Some(court_id.clone())
            },
            relationship_type: "APPLIES_TO".to_string(),
        });

        if row.authority_kind == "SupplementaryLocalRuleEdition" {
            let edition_year = row
                .effective_start_date
                .get(0..4)
                .and_then(|year| year.parse::<i32>().ok())
                .unwrap_or(snapshot_date.year());
            slr_editions.push(SupplementaryLocalRuleEdition {
                edition_id: authority_document_id.clone(),
                authority_document_id: authority_document_id.clone(),
                corpus_id: format!("{}:slr", row.jurisdiction_id),
                supplements_corpus_id: Some(config.base_rule_corpus_id.clone()),
                jurisdiction_id: row.jurisdiction_id.clone(),
                court_id: court_id.clone(),
                edition_year,
                title: row.title.clone(),
                effective_start_date: row.effective_start_date.clone(),
                effective_end_date: row.effective_end_date.clone(),
                date_status: row.date_status.clone(),
            });
        }

        match row.authority_kind.as_str() {
            "ChiefJusticeOrder" => parsed.chief_justice_orders.push(doc.clone()),
            "PresidingJudgeOrder" => parsed.presiding_judge_orders.push(doc.clone()),
            "OutOfCycleAmendment" => parsed.out_of_cycle_amendments.push(doc.clone()),
            _ => {}
        }
        parsed.authority_documents.push(doc);
    }

    parsed.supplementary_local_rule_editions = slr_editions;
    parsed.rule_topics = topic_map.into_values().collect();
    parsed.rule_supersession_edges = build_supersession_edges(
        &parsed.authority_documents,
        &parsed.supplementary_local_rule_editions,
        &doc_index_by_identifier,
    );
    apply_superseded_flags(
        &mut parsed.authority_documents,
        &mut parsed.publication_entries,
        &parsed.rule_supersession_edges,
    );
    parsed.work_product_rule_pack_authorities =
        build_rule_pack_authorities(&parsed.authority_documents);
    if let Some(snapshot) = parsed.registry_snapshots.first_mut() {
        snapshot.entry_count = parsed.publication_entries.len();
    }

    Ok(parsed)
}

fn parse_row(
    line: &str,
    publication_bucket: &str,
    table_section: &str,
    row_index: usize,
    snapshot_date: NaiveDate,
    state_id: &str,
) -> Result<ParsedRow> {
    let columns = split_columns(line)?;
    let title = columns[0].trim().to_string();
    let jurisdiction = columns[1].trim().to_string();
    let subcategory = columns[2].trim().to_string();
    let effective_start_date = parse_registry_date(columns[3].trim())?;
    let effective_end_date = columns
        .get(4)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(parse_registry_date)
        .transpose()?;
    let jurisdiction_id = normalize_jurisdiction_id(&jurisdiction, state_id);
    let authority_kind = normalize_subcategory(&subcategory);
    let authority_identifier = extract_authority_identifier(&title, &authority_kind);
    let (date_status, mut status_flags) = compute_status_flags(
        &effective_start_date,
        effective_end_date.as_deref(),
        publication_bucket,
        &authority_kind,
        snapshot_date,
    )?;
    Ok(ParsedRow {
        title,
        jurisdiction,
        jurisdiction_id,
        subcategory,
        authority_kind,
        publication_bucket: publication_bucket.to_string(),
        table_section: table_section.to_string(),
        row_index,
        effective_start_date,
        effective_end_date,
        authority_identifier,
        date_status,
        status_flags: {
            status_flags.sort();
            status_flags.dedup();
            status_flags
        },
    })
}

fn split_columns(line: &str) -> Result<Vec<String>> {
    let mut columns = line
        .split('\t')
        .map(|part| part.trim().to_string())
        .collect::<Vec<_>>();
    if columns.len() == 4 {
        columns.push(String::new());
    }
    if columns.len() >= 5 {
        return Ok(columns.into_iter().take(5).collect());
    }

    let dates = DATE_RE.find_iter(line).collect::<Vec<_>>();
    if dates.is_empty() {
        return Err(anyhow!("row does not include an effective start date"));
    }
    let start = dates[0];
    let end = dates.get(1).copied();
    let before_date = line[..start.start()].trim();
    let end_date = end.map(|m| m.as_str()).unwrap_or("");
    let before_parts = before_date
        .rsplitn(3, char::is_whitespace)
        .map(str::trim)
        .collect::<Vec<_>>();
    if before_parts.len() < 3 {
        return Err(anyhow!("row does not include all required text columns"));
    }
    let subcategory = before_parts[0].to_string();
    let jurisdiction = before_parts[1].to_string();
    let description = before_parts[2].to_string();
    Ok(vec![
        description,
        jurisdiction,
        subcategory,
        start.as_str().to_string(),
        end_date.to_string(),
    ])
}

fn is_table_like(line: &str) -> bool {
    line.contains('\t') || DATE_RE.is_match(line)
}

fn parse_registry_date(raw: &str) -> Result<String> {
    let parts = raw.split('/').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(anyhow!("invalid date {raw}"));
    }
    let month = parts[0].parse::<u32>()?;
    let day = parts[1].parse::<u32>()?;
    let year = parts[2].parse::<i32>()?;
    let date = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| anyhow!("invalid calendar date {raw}"))?;
    Ok(date.format("%Y-%m-%d").to_string())
}

fn parse_iso_date(raw: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|err| anyhow!("invalid ISO date {raw}: {err}"))
}

fn normalize_jurisdiction_id(value: &str, state_id: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "statewide" => state_id.to_string(),
        "oregon" | "or:state" => "or:state".to_string(),
        other if other.starts_with("or:") => other.to_string(),
        other if other.ends_with(" county") => {
            format!(
                "{}:{}",
                state_prefix(state_id),
                slug(other.trim_end_matches(" county"))
            )
        }
        other => format!("{}:{}", state_prefix(state_id), slug(other)),
    }
}

fn state_prefix(state_id: &str) -> &str {
    state_id.split(':').next().unwrap_or("or")
}

fn normalize_subcategory(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "cjo" => "ChiefJusticeOrder".to_string(),
        "pjo" => "PresidingJudgeOrder".to_string(),
        "rule" => "SupplementaryLocalRuleEdition".to_string(),
        "out-of-cycle" => "OutOfCycleAmendment".to_string(),
        other => {
            let mut result = String::new();
            for part in other.split(|c: char| !c.is_ascii_alphanumeric()) {
                if part.is_empty() {
                    continue;
                }
                let mut chars = part.chars();
                if let Some(first) = chars.next() {
                    result.push(first.to_ascii_uppercase());
                    result.push_str(chars.as_str());
                }
            }
            result
        }
    }
}

fn extract_authority_identifier(title: &str, authority_kind: &str) -> Option<String> {
    match authority_kind {
        "ChiefJusticeOrder" => CJO_RE
            .captures(title)
            .and_then(|caps| caps.get(1))
            .map(|m| format!("CJO {}", m.as_str())),
        "PresidingJudgeOrder" => PJO_RE
            .captures(title)
            .and_then(|caps| caps.get(1))
            .map(|m| format!("PJO {}", m.as_str())),
        "OutOfCycleAmendment" => {
            let mut parts = Vec::new();
            if let Some(caps) = SLR_RE.captures(title) {
                if let Some(rule) = caps.get(1) {
                    parts.push(format!("SLR {}", rule.as_str()));
                }
            }
            if title.to_ascii_lowercase().contains("appendix b") {
                parts.push("Appendix B".to_string());
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("; "))
            }
        }
        _ => None,
    }
}

fn authority_document_id(row: &ParsedRow) -> String {
    if row.authority_kind == "SupplementaryLocalRuleEdition" {
        if let Some(year) = row.effective_start_date.get(0..4) {
            return format!("{}:slr@{}", row.jurisdiction_id, year);
        }
    }
    if let Some(identifier) = &row.authority_identifier {
        let id = identifier
            .to_ascii_lowercase()
            .replace(' ', ":")
            .replace('-', "-");
        return format!("{}:{}", row.jurisdiction_id, id);
    }
    format!(
        "{}:{}:{}",
        row.jurisdiction_id,
        row.authority_kind.to_ascii_lowercase(),
        slug(&format!("{} {}", row.title, row.effective_start_date))
    )
}

fn compute_status_flags(
    start: &str,
    end: Option<&str>,
    publication_bucket: &str,
    authority_kind: &str,
    snapshot_date: NaiveDate,
) -> Result<(String, Vec<String>)> {
    let start_date = parse_iso_date(start)?;
    let end_date = end.map(parse_iso_date).transpose()?;
    let date_status = if start_date > snapshot_date {
        "future"
    } else if end_date.is_some_and(|date| date < snapshot_date) {
        "expired"
    } else {
        "current"
    };
    let mut flags = vec![date_status.to_string()];
    if publication_bucket == "prior" {
        flags.push("prior".to_string());
    }
    if end_date.is_none() {
        flags.push("open_ended".to_string());
    }
    if end_date.is_some_and(|date| date == start_date) {
        flags.push("one_day_only".to_string());
    }
    if authority_kind == "OutOfCycleAmendment" {
        flags.push("out_of_cycle".to_string());
    }
    Ok((date_status.to_string(), flags))
}

fn build_supersession_edges(
    docs: &[RuleAuthorityDocument],
    slr_editions: &[SupplementaryLocalRuleEdition],
    doc_index_by_identifier: &HashMap<String, String>,
) -> Vec<RuleSupersessionEdge> {
    let mut edges = Vec::<RuleSupersessionEdge>::new();
    let mut sorted_slrs = slr_editions.to_vec();
    sorted_slrs.sort_by(|a, b| b.edition_year.cmp(&a.edition_year));
    for pair in sorted_slrs.windows(2) {
        let newer = &pair[0];
        let older = &pair[1];
        edges.push(RuleSupersessionEdge {
            edge_id: format!(
                "rule_supersession:{}",
                stable_id(&format!(
                    "{}::{}",
                    newer.authority_document_id, older.authority_document_id
                ))
            ),
            from_authority_document_id: newer.authority_document_id.clone(),
            to_authority_document_id: older.authority_document_id.clone(),
            relationship_type: "SUPERSEDES".to_string(),
            reason: format!("Annual {} SLR edition sequence.", newer.jurisdiction_id),
            confidence: 0.95,
        });
    }

    for doc in docs {
        for caps in SUPERSEDES_CJO_RE.captures_iter(&doc.title) {
            if let Some(target) = caps.get(1) {
                let identifier = format!("CJO {}", target.as_str()).to_ascii_lowercase();
                if let Some(target_id) = doc_index_by_identifier.get(&identifier) {
                    edges.push(RuleSupersessionEdge {
                        edge_id: format!(
                            "rule_supersession:{}",
                            stable_id(&format!("{}::{}", doc.authority_document_id, target_id))
                        ),
                        from_authority_document_id: doc.authority_document_id.clone(),
                        to_authority_document_id: target_id.clone(),
                        relationship_type: "SUPERSEDES".to_string(),
                        reason: "Title states supersession.".to_string(),
                        confidence: 0.9,
                    });
                }
            }
        }
    }
    edges.sort_by(|a, b| a.edge_id.cmp(&b.edge_id));
    edges.dedup_by(|a, b| a.edge_id == b.edge_id);
    edges
}

fn apply_superseded_flags(
    docs: &mut [RuleAuthorityDocument],
    entries: &mut [RulePublicationEntry],
    edges: &[RuleSupersessionEdge],
) {
    let superseded_ids = edges
        .iter()
        .map(|edge| edge.to_authority_document_id.clone())
        .collect::<BTreeSet<_>>();
    for doc in docs {
        if superseded_ids.contains(&doc.authority_document_id)
            && !doc.status_flags.iter().any(|flag| flag == "superseded")
        {
            doc.status_flags.push("superseded".to_string());
            doc.status_flags.sort();
        }
    }
    for entry in entries {
        if superseded_ids.contains(&entry.authority_document_id)
            && !entry.status_flags.iter().any(|flag| flag == "superseded")
        {
            entry.status_flags.push("superseded".to_string());
            entry.status_flags.sort();
        }
    }
}

fn build_rule_pack_authorities(
    docs: &[RuleAuthorityDocument],
) -> Vec<WorkProductRulePackAuthority> {
    let packs = [
        ("complaint", "or:utcr:2025:oregon_circuit_civil_complaint"),
        ("motion", "or:utcr:2025:oregon_circuit_civil_motion"),
        ("answer", "or:utcr:2025:oregon_circuit_answer"),
        ("declaration", "or:utcr:2025:oregon_circuit_declaration"),
        ("filing_packet", "or:utcr:2025:oregon_circuit_filing_packet"),
    ];
    let mut rows = Vec::new();
    for doc in docs {
        for (work_product_type, rule_pack_id) in packs {
            rows.push(WorkProductRulePackAuthority {
                rule_pack_authority_id: format!(
                    "work_product_rule_pack_authority:{}",
                    stable_id(&format!("{}::{}", rule_pack_id, doc.authority_document_id))
                ),
                rule_pack_id: rule_pack_id.to_string(),
                authority_document_id: doc.authority_document_id.clone(),
                work_product_type: work_product_type.to_string(),
                jurisdiction_id: doc.jurisdiction_id.clone(),
                inclusion_reason: "Court rules registry overlay authority.".to_string(),
            });
        }
    }
    rows
}

fn topics_for_title(title: &str) -> Vec<String> {
    let lower = title.to_ascii_lowercase();
    let mut topics = Vec::new();
    let topic_rules = [
        ("emergency", "Emergency Closure"),
        ("closure", "Court Closure"),
        ("court operations", "Court Operations"),
        ("covid", "COVID-19"),
        ("remote", "Remote Proceedings"),
        ("pretrial release", "Pretrial Release"),
        ("fees", "Fees"),
        ("certified", "Certified Copies"),
        ("exemplified", "Certified Copies"),
        ("security screening", "Security Screening"),
        ("probate commissioner", "Probate Commissioner"),
        ("weapons", "Weapons"),
        ("landlord tenant", "Landlord Tenant Sealing"),
        ("immigration", "Immigration Enforcement"),
        ("ojcin", "OJCIN Fees"),
        ("face covering", "Face Coverings"),
        ("fed", "FED Extensions"),
        ("duii", "DUII Diversions"),
        ("vaccination", "Vaccination"),
        ("social distancing", "Social Distancing"),
        ("set aside", "Landlord Tenant Sealing"),
        ("seal eligible", "Landlord Tenant Sealing"),
    ];
    for (needle, topic) in topic_rules {
        if lower.contains(needle) && !topics.iter().any(|existing| existing == topic) {
            topics.push(topic.to_string());
        }
    }
    if topics.is_empty() {
        topics.push("Court Rules Registry".to_string());
    }
    topics
}

fn normalize_topic(value: &str) -> String {
    slug(value).replace('-', "_")
}

fn effective_label(start: &str, end: Option<&str>) -> String {
    match end {
        Some(end) => format!("{start} through {end}"),
        None => format!("{start} onward"),
    }
}

fn base_jurisdictions(
    config: &CourtRulesRegistryParseConfig,
    jurisdiction_id: &str,
) -> Vec<Jurisdiction> {
    let mut jurisdictions = vec![
        Jurisdiction {
            jurisdiction_id: "us".to_string(),
            name: "United States".to_string(),
            jurisdiction_type: "federal".to_string(),
            parent_jurisdiction_id: None,
            country: Some("US".to_string()),
        },
        Jurisdiction {
            jurisdiction_id: config.state_id.clone(),
            name: config.state_name.clone(),
            jurisdiction_type: "state".to_string(),
            parent_jurisdiction_id: Some("us".to_string()),
            country: Some("US".to_string()),
        },
    ];
    if jurisdiction_id != config.state_id {
        jurisdictions.push(Jurisdiction {
            jurisdiction_id: jurisdiction_id.to_string(),
            name: jurisdiction_name(&config.jurisdiction),
            jurisdiction_type: "county".to_string(),
            parent_jurisdiction_id: Some(config.state_id.clone()),
            country: Some("US".to_string()),
        });
    }
    if let (Some(district_id), Some(district_name)) = (
        config.judicial_district_id.as_ref(),
        config.judicial_district_name.as_ref(),
    ) {
        jurisdictions.push(Jurisdiction {
            jurisdiction_id: district_id.clone(),
            name: district_name.clone(),
            jurisdiction_type: "judicial_district".to_string(),
            parent_jurisdiction_id: Some(config.state_id.clone()),
            country: Some("US".to_string()),
        });
    }
    jurisdictions
}

fn base_courts(config: &CourtRulesRegistryParseConfig, jurisdiction_id: &str) -> Vec<Court> {
    if jurisdiction_id == config.state_id {
        return Vec::new();
    }
    vec![Court {
        court_id: local_court_id(config, jurisdiction_id),
        name: config.court_name.clone().unwrap_or_else(|| {
            format!("{} Circuit Court", jurisdiction_name(&config.jurisdiction))
        }),
        court_type: "circuit_court".to_string(),
        jurisdiction_id: jurisdiction_id.to_string(),
        county_jurisdiction_id: Some(jurisdiction_id.to_string()),
        judicial_district_id: config.judicial_district_id.clone(),
        judicial_district: config.judicial_district_name.clone(),
    }]
}

fn local_court_id(config: &CourtRulesRegistryParseConfig, jurisdiction_id: &str) -> String {
    config
        .court_id
        .clone()
        .unwrap_or_else(|| format!("{jurisdiction_id}:circuit_court"))
}

fn jurisdiction_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("statewide") {
        "Statewide".to_string()
    } else if trimmed.to_ascii_lowercase().ends_with(" county") {
        title_case(trimmed)
    } else {
        format!("{} County", title_case(trimmed))
    }
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let rest = chars.as_str().to_ascii_lowercase();
                    format!("{}{}", first.to_ascii_uppercase(), rest)
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn diagnostic(
    source_document_id: &str,
    severity: &str,
    diagnostic_type: &str,
    message: &str,
    source_paragraph_order: Option<usize>,
    related_id: Option<String>,
) -> ParserDiagnostic {
    ParserDiagnostic {
        parser_diagnostic_id: format!(
            "parser_diagnostic:{}",
            stable_id(&format!(
                "{source_document_id}::{diagnostic_type}::{message}"
            ))
        ),
        source_document_id: source_document_id.to_string(),
        chapter: "court_rules_registry".to_string(),
        edition_year: 0,
        severity: severity.to_string(),
        diagnostic_type: diagnostic_type.to_string(),
        message: message.to_string(),
        source_paragraph_order,
        related_id,
        parser_profile: PARSER_PROFILE.to_string(),
    }
}

fn slug(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"Supplementary Local Court Rules (SLRs)
Current and Future Rules
Description	Jurisdiction	Subcategory	Effective Start Date	Effective End Date
CJO 25-018 Order Establishing Interim Procedures Addressing Immigration Enforcement Activities in the Oregon Courts (PDF)	Statewide	CJO	7/1/2025	
PJO 25-005 Order to Set Aside Judgments and Seal Eligible Residential Landlord Tenant Cases	Linn	PJO	11/10/2025	
Linn County Supplementary Local Court Rules (SLR) (PDF)	Linn	Rule	2/1/2026	1/31/2027
Prior Rules
Description	Jurisdiction	Subcategory	Effective Start Date	Effective End Date
* Out-of-Cycle Amendment of SLR 6.101 and Appendix B (effective April 19, 2025) (PDF)	Linn	Out-of-Cycle	4/19/2025	12/31/2025
Linn County Supplementary Local Court Rules (SLR) (PDF)	Linn	Rule	2/1/2025	1/31/2026
"#;

    #[test]
    fn parses_current_and_prior_registry_rows() {
        let parsed = parse_court_rules_registry_text(
            SAMPLE,
            CourtRulesRegistryParseConfig::oregon(
                "Linn".to_string(),
                "2026-05-01".to_string(),
                "https://www.courts.oregon.gov/courts/linn/go/pages/rules.aspx".to_string(),
            ),
        )
        .unwrap();
        assert_eq!(parsed.publication_entries.len(), 5);
        assert!(parsed
            .authority_documents
            .iter()
            .any(|doc| doc.authority_document_id == "or:linn:slr@2026"
                && doc.date_status == "current"));
        assert!(parsed
            .authority_documents
            .iter()
            .any(
                |doc| doc.authority_identifier.as_deref() == Some("CJO 25-018")
                    && doc.status_flags.iter().any(|flag| flag == "open_ended")
            ));
        assert!(parsed
            .out_of_cycle_amendments
            .iter()
            .any(|doc| doc.authority_identifier.as_deref() == Some("SLR 6.101; Appendix B")));
    }

    #[test]
    fn builds_slr_supersession_and_amendment_links() {
        let parsed = parse_court_rules_registry_text(
            SAMPLE,
            CourtRulesRegistryParseConfig::oregon(
                "Linn".to_string(),
                "2026-05-01".to_string(),
                "https://example.test/linn".to_string(),
            ),
        )
        .unwrap();
        assert!(parsed.rule_supersession_edges.iter().any(|edge| {
            edge.from_authority_document_id == "or:linn:slr@2026"
                && edge.to_authority_document_id == "or:linn:slr@2025"
        }));
        assert!(parsed.out_of_cycle_amendments.iter().any(|doc| {
            doc.amends_authority_document_id.as_deref() == Some("or:linn:slr@2025")
        }));
    }
}
