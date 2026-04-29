use crate::hash::stable_id;
use crate::models::{
    Deadline, DefinedTerm, Definition, DefinitionScope, Exception, FormText, LegalAction,
    LegalActor, LegalSemanticNode, LegalTextVersion, LineageEvent, MoneyAmount, Obligation,
    Penalty, Provision, RateLimit, Remedy, RequiredNotice, SessionLaw, SourceDocument, SourceNote,
    StatusEvent, TaxRule, TemporalEffect,
};
use crate::text::normalize_ws;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

static QUOTED_TERM_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#""([^"]{2,120})""#).expect("quoted term regex"));
static MEANS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)^\s*([A-Za-z][A-Za-z0-9 ,'-]{1,120})\s+means\s+"#).expect("means regex")
});
static OBLIGATION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(shall|must|is required to|required to)\b").expect("obligation regex")
});
static DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2},\s+[12]\d{3}\b").expect("date regex")
});
static SESSION_LAW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)([12]\d{3})\s+c\.?\s*([0-9A-Za-z]+)(?:\s*(?:§|sec\.?|section)\s*([0-9A-Za-z.\-]+))?",
    )
    .expect("session law regex")
});
static OREGON_LAWS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)Oregon Laws\s+([12]\d{3}),\s+chapter\s+([0-9A-Za-z]+)(?:,\s+section\s+([0-9A-Za-z.\-]+))?").expect("oregon laws regex")
});
static OREGON_LAWS_ALT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:section\s+([0-9A-Za-z.\-]+),\s+)?chapter\s+([0-9A-Za-z]+),\s+Oregon Laws\s+([12]\d{3})").expect("oregon laws alt regex")
});
static FORMERLY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bFormerly\s+(\d{1,3}\.\d{3,4})\b").expect("formerly regex"));
static RENUMBERED_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\brenumbered(?:\s+(?:to|from))?\s+(\d{1,3}\.\d{3,4})(?:\s+in\s+([12]\d{3}))?\b",
    )
    .expect("renumbered regex")
});
static REPEALED_BY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\brepealed\s+by\s+([12]\d{3}\s+c\.?\s*[0-9A-Za-z]+(?:\s*§\s*[0-9A-Za-z.\-]+)?)",
    )
    .expect("repealed by regex")
});
static PENALTY_CLASS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bClass\s+([A-Z])\s+(misdemeanor|felony|violation)\b")
        .expect("penalty class regex")
});
static MONEY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\$[0-9][0-9,]*(?:\.\d{2})?").expect("money regex"));
static PERCENT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b\d+(?:\.\d+)?\s*percent\b").expect("percent regex"));
static DURATION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(?:within|before|after|on or before|not later than|no later than)\s+[^.;]{1,80}",
    )
    .expect("duration regex")
});
static JAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(?:imprisonment|jail|county jail)[^.]{0,80}\b(?:\d+\s+(?:days?|months?|years?))",
    )
    .expect("jail regex")
});
static SCOPE_RANGE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bAs used in ORS\s+(\d{1,3}\.\d{3,4})\s+to\s+(\d{1,3}\.\d{3,4})\b")
        .expect("definition range regex")
});

#[derive(Debug, Default)]
pub struct DerivedHistoricalNodes {
    pub status_events: Vec<StatusEvent>,
    pub temporal_effects: Vec<TemporalEffect>,
    pub lineage_events: Vec<LineageEvent>,
    pub session_laws: Vec<SessionLaw>,
}

#[derive(Debug, Default)]
pub struct DerivedSemanticNodes {
    pub defined_terms: Vec<DefinedTerm>,
    pub definitions: Vec<Definition>,
    pub definition_scopes: Vec<DefinitionScope>,
    pub legal_semantic_nodes: Vec<LegalSemanticNode>,
    pub obligations: Vec<Obligation>,
    pub exceptions: Vec<Exception>,
    pub deadlines: Vec<Deadline>,
    pub penalties: Vec<Penalty>,
    pub remedies: Vec<Remedy>,
    pub legal_actors: Vec<LegalActor>,
    pub legal_actions: Vec<LegalAction>,
    pub money_amounts: Vec<MoneyAmount>,
    pub tax_rules: Vec<TaxRule>,
    pub rate_limits: Vec<RateLimit>,
    pub required_notices: Vec<RequiredNotice>,
    pub form_texts: Vec<FormText>,
}

pub fn derive_historical_nodes(
    versions: &[LegalTextVersion],
    source_document: &SourceDocument,
) -> DerivedHistoricalNodes {
    let status_events = versions
        .iter()
        .filter(|version| version.status != "active" || version.status_text.is_some())
        .map(|version| {
            let status_text = version
                .status_text
                .clone()
                .unwrap_or_else(|| version.status.clone());
            StatusEvent {
                status_event_id: format!(
                    "status_event:{}",
                    stable_id(&format!("{}::{}", version.version_id, status_text))
                ),
                status_type: version.status.clone(),
                status_text: Some(status_text),
                source_document_id: Some(source_document.source_document_id.clone()),
                canonical_id: version.canonical_id.clone(),
                version_id: Some(version.version_id.clone()),
                event_year: Some(version.edition_year),
                effective_date: None,
                source_note_id: None,
                effect_type: None,
                trigger_text: None,
                operative_date: None,
                repeal_date: None,
                session_law_ref: None,
                confidence: 0.8,
                extraction_method: "status_text_parser_v1".to_string(),
            }
        })
        .collect();

    DerivedHistoricalNodes {
        status_events,
        ..Default::default()
    }
}

pub fn derive_source_note_status_events(
    notes: &[SourceNote],
    source_document: &SourceDocument,
    edition_year: i32,
) -> Vec<StatusEvent> {
    notes
        .iter()
        .filter(|note| note.note_type != "official_note")
        .map(|note| StatusEvent {
            status_event_id: format!(
                "status_event:{}",
                stable_id(&format!(
                    "{}::{}::{}",
                    note.source_note_id, note.note_type, note.text
                ))
            ),
            status_type: note.note_type.clone(),
            status_text: Some(note.text.clone()),
            source_document_id: Some(source_document.source_document_id.clone()),
            canonical_id: note.canonical_id.clone(),
            version_id: note.version_id.clone(),
            event_year: Some(edition_year),
            effective_date: None,
            source_note_id: Some(note.source_note_id.clone()),
            effect_type: Some(note.note_type.clone()),
            trigger_text: Some(note.text.clone()),
            operative_date: None,
            repeal_date: None,
            session_law_ref: extract_first_session_law_citation(&note.text),
            confidence: 0.75,
            extraction_method: "source_note_status_parser_v1".to_string(),
        })
        .collect()
}

pub fn derive_note_semantics(
    notes: &[SourceNote],
    source_document: &SourceDocument,
    edition_year: i32,
) -> DerivedHistoricalNodes {
    let mut out = DerivedHistoricalNodes::default();

    for note in notes {
        out.temporal_effects
            .extend(extract_temporal_effects(note, edition_year));
        out.lineage_events.extend(extract_lineage_events(note));
        out.session_laws.extend(extract_session_laws_from_text(
            &note.text,
            source_document,
            Some(note),
        ));
    }

    dedupe_historical_nodes(&mut out);
    out
}

pub fn derive_session_laws_from_amendments(
    amendments: &[crate::models::Amendment],
    source_document: &SourceDocument,
) -> Vec<SessionLaw> {
    let mut laws = Vec::new();
    for amendment in amendments {
        if let Some(citation) = amendment
            .session_law_citation
            .as_deref()
            .or(amendment.raw_text.as_deref())
        {
            laws.extend(extract_session_laws_from_text(
                citation,
                source_document,
                None,
            ));
        }
    }
    laws
}

pub fn derive_semantic_nodes(provisions: &[Provision]) -> DerivedSemanticNodes {
    let mut out = DerivedSemanticNodes::default();

    for provision in provisions {
        if provision.is_implied || provision.text.trim().is_empty() {
            continue;
        }

        let text = normalize_ws(&provision.text);
        let normalized_text = text.to_lowercase();
        let actor = extract_actor_text(&text);
        let action = extract_action_text(&text);
        let actor_id = actor.as_ref().map(|actor_text| {
            format!(
                "legal_actor:{}",
                stable_id(&format!(
                    "{}::{}",
                    provision.provision_id,
                    actor_text.to_lowercase()
                ))
            )
        });
        let action_id = action.as_ref().map(|(verb, object)| {
            format!(
                "legal_action:{}",
                stable_id(&format!(
                    "{}::{}::{}",
                    provision.provision_id,
                    verb.to_lowercase(),
                    object.as_deref().unwrap_or("")
                ))
            )
        });

        if let Some(actor_text) = &actor {
            out.legal_actors.push(LegalActor {
                actor_id: actor_id.clone().unwrap(),
                name: actor_text.clone(),
                normalized_name: actor_text.to_lowercase(),
                actor_type: classify_actor_type(actor_text).map(ToString::to_string),
                jurisdiction_id: Some("or:state".to_string()),
            });
        }

        if let Some((verb, object)) = &action {
            out.legal_actions.push(LegalAction {
                action_id: action_id.clone().unwrap(),
                verb: verb.clone(),
                object: object.clone(),
                normalized_action: normalize_ws(&format!(
                    "{} {}",
                    verb.to_lowercase(),
                    object.clone().unwrap_or_default().to_lowercase()
                )),
                source_provision_id: Some(provision.provision_id.clone()),
                confidence: Some(0.58),
            });
        }

        if provision.is_definition_candidate {
            let term =
                extract_defined_term(&text).unwrap_or_else(|| provision.display_citation.clone());
            let normalized_term = normalize_ws(&term).to_lowercase();
            let defined_term_id = format!(
                "defined_term:{}",
                stable_id(&format!("{}::{}", provision.canonical_id, normalized_term))
            );
            let definition_scope_id = format!(
                "definition_scope:{}",
                stable_id(&format!(
                    "{}::{}",
                    provision.provision_id, provision.canonical_id
                ))
            );
            let definition_id = format!(
                "definition:{}",
                stable_id(&format!("{}::{}", provision.provision_id, normalized_term))
            );

            out.defined_terms.push(DefinedTerm {
                defined_term_id: defined_term_id.clone(),
                term: term.clone(),
                normalized_term: normalized_term.clone(),
                jurisdiction_id: "or:state".to_string(),
                authority_family: "statute".to_string(),
            });
            out.definition_scopes.push(DefinitionScope {
                definition_scope_id: definition_scope_id.clone(),
                scope_type: definition_scope_type(provision, &text).to_string(),
                scope_citation: definition_scope_citation(provision, &text),
                target_canonical_id: definition_scope_target_canonical(provision, &text),
                target_chapter_id: definition_scope_target_chapter(provision, &text),
                target_range_start: definition_scope_range(&text).map(|(start, _)| start),
                target_range_end: definition_scope_range(&text).map(|(_, end)| end),
            });
            out.definitions.push(Definition {
                definition_id,
                term,
                normalized_term,
                definition_text: text.clone(),
                scope_type: Some(definition_scope_type(provision, &text).to_string()),
                scope_citation: definition_scope_citation(provision, &text),
                source_provision_id: provision.provision_id.clone(),
                confidence: 0.7,
                review_status: Some("needs_review".to_string()),
                extraction_method: "definition_candidate_flag_v1".to_string(),
                defined_term_id: Some(defined_term_id),
                definition_scope_id: Some(definition_scope_id),
            });
        }

        if OBLIGATION_RE.is_match(&text) {
            let obligation_id = format!(
                "obligation:{}",
                stable_id(&format!("{}::{}", provision.provision_id, text))
            );
            out.obligations.push(Obligation {
                obligation_id,
                text: text.clone(),
                actor_text: actor.clone(),
                action_text: action.as_ref().map(|(verb, object)| {
                    normalize_ws(&format!("{} {}", verb, object.clone().unwrap_or_default()))
                }),
                object_text: None,
                condition_text: extract_condition_text(&text),
                source_provision_id: provision.provision_id.clone(),
                confidence: if actor.is_some() || action.is_some() {
                    0.68
                } else {
                    0.55
                },
                actor_id: actor_id.clone(),
                action_id: action_id.clone(),
                deadline_id: None,
                exception_id: None,
                penalty_id: None,
            });
        }

        if provision.is_exception_candidate {
            out.exceptions.push(Exception {
                exception_id: format!(
                    "exception:{}",
                    stable_id(&format!("{}::{}", provision.provision_id, text))
                ),
                text: text.clone(),
                trigger_phrase: extract_trigger_phrase(
                    &normalized_text,
                    &["except", "unless", "notwithstanding"],
                ),
                exception_type: None,
                source_provision_id: provision.provision_id.clone(),
                confidence: 0.65,
                target_provision_id: None,
                target_canonical_id: Some(provision.canonical_id.clone()),
                target_obligation_id: None,
            });
        }

        if provision.is_deadline_candidate {
            out.deadlines.push(Deadline {
                deadline_id: format!(
                    "deadline:{}",
                    stable_id(&format!("{}::{}", provision.provision_id, text))
                ),
                text: text.clone(),
                duration: extract_duration_text(&text),
                date_text: extract_first_date(&text),
                trigger_event: extract_deadline_trigger(&text),
                actor: actor.clone(),
                action_required: action.as_ref().map(|(verb, object)| {
                    normalize_ws(&format!("{} {}", verb, object.clone().unwrap_or_default()))
                }),
                source_provision_id: provision.provision_id.clone(),
                confidence: 0.6,
                obligation_id: None,
            });
        }

        if provision.is_penalty_candidate {
            let penalty_details = extract_penalty_details(&text);
            out.penalties.push(Penalty {
                penalty_id: format!(
                    "penalty:{}",
                    stable_id(&format!("{}::{}", provision.provision_id, text))
                ),
                text: text.clone(),
                penalty_type: penalty_details.penalty_type,
                amount: penalty_details.amount.clone(),
                minimum: penalty_details.minimum.clone(),
                maximum: penalty_details.maximum.clone(),
                condition: None,
                source_provision_id: provision.provision_id.clone(),
                confidence: if penalty_details.has_detail {
                    0.78
                } else {
                    0.6
                },
                obligation_id: None,
                criminal_class: penalty_details.criminal_class,
                civil_penalty_amount: penalty_details.civil_penalty_amount,
                min_amount: penalty_details.minimum,
                max_amount: penalty_details.maximum,
                jail_term: penalty_details.jail_term,
                license_suspension: penalty_details.license_suspension,
                revocation: penalty_details.revocation,
                target_conduct: penalty_details.target_conduct,
                target_citation: penalty_details.target_citation,
            });
        }

        if let Some(remedy) = extract_remedy(provision, &text) {
            out.remedies.push(remedy);
        }

        out.money_amounts
            .extend(extract_money_amounts(provision, &text));

        if let Some(tax_rule) = extract_tax_rule(provision, &text) {
            out.tax_rules.push(tax_rule);
        }

        out.rate_limits
            .extend(extract_rate_limits(provision, &text));

        if let Some(required_notice) = extract_required_notice(provision, &text) {
            out.required_notices.push(required_notice);
        }

        if let Some(form_text) = extract_form_text(provision, &text) {
            out.form_texts.push(form_text);
        }

        for semantic_type in
            semantic_types_for_provision(provision, &text, OBLIGATION_RE.is_match(&text))
        {
            out.legal_semantic_nodes.push(LegalSemanticNode {
                semantic_id: format!(
                    "semantic:{}",
                    stable_id(&format!(
                        "{}::{}::{}",
                        provision.provision_id, semantic_type, text
                    ))
                ),
                semantic_type: semantic_type.to_string(),
                text: text.clone(),
                normalized_text: normalized_text.clone(),
                source_provision_id: provision.provision_id.clone(),
                confidence: 0.55,
                review_status: Some("needs_review".to_string()),
                extraction_method: "provision_signal_parser_v1".to_string(),
            });
        }
    }

    dedupe_semantic_nodes(&mut out);
    out
}

fn dedupe_historical_nodes(out: &mut DerivedHistoricalNodes) {
    retain_unique_by(&mut out.status_events, |row| row.status_event_id.clone());
    retain_unique_by(&mut out.temporal_effects, |row| {
        row.temporal_effect_id.clone()
    });
    retain_unique_by(&mut out.lineage_events, |row| row.lineage_event_id.clone());
    retain_unique_by(&mut out.session_laws, |row| row.session_law_id.clone());
}

fn dedupe_semantic_nodes(out: &mut DerivedSemanticNodes) {
    retain_unique_by(&mut out.defined_terms, |row| row.defined_term_id.clone());
    retain_unique_by(&mut out.definitions, |row| row.definition_id.clone());
    retain_unique_by(&mut out.definition_scopes, |row| {
        row.definition_scope_id.clone()
    });
    retain_unique_by(&mut out.legal_semantic_nodes, |row| row.semantic_id.clone());
    retain_unique_by(&mut out.obligations, |row| row.obligation_id.clone());
    retain_unique_by(&mut out.exceptions, |row| row.exception_id.clone());
    retain_unique_by(&mut out.deadlines, |row| row.deadline_id.clone());
    retain_unique_by(&mut out.penalties, |row| row.penalty_id.clone());
    retain_unique_by(&mut out.remedies, |row| row.remedy_id.clone());
    retain_unique_by(&mut out.legal_actors, |row| row.actor_id.clone());
    retain_unique_by(&mut out.legal_actions, |row| row.action_id.clone());
    retain_unique_by(&mut out.money_amounts, |row| row.money_amount_id.clone());
    retain_unique_by(&mut out.tax_rules, |row| row.tax_rule_id.clone());
    retain_unique_by(&mut out.rate_limits, |row| row.rate_limit_id.clone());
    retain_unique_by(&mut out.required_notices, |row| {
        row.required_notice_id.clone()
    });
    retain_unique_by(&mut out.form_texts, |row| row.form_text_id.clone());
}

pub fn derive_provision_temporal_effects(provisions: &[Provision]) -> Vec<TemporalEffect> {
    let mut rows = Vec::new();

    for provision in provisions {
        if provision.is_implied || provision.text.trim().is_empty() {
            continue;
        }
        let text = normalize_ws(&provision.text);
        let lowered = text.to_lowercase();
        let mut effect_types = Vec::new();
        if lowered.contains("becomes operative") || lowered.contains("is operative") {
            effect_types.push("operative");
        }
        if lowered.contains("takes effect") || lowered.contains("effective date") {
            effect_types.push("effective");
        }
        if lowered.contains("is repealed") || lowered.contains("repealed") {
            effect_types.push("repeal");
        }
        if lowered.contains("expires") || lowered.contains("expiration") {
            effect_types.push("expiration");
        }
        if lowered.contains("before ") || lowered.contains("on or before") {
            effect_types.push("deadline");
        }

        for effect_type in effect_types {
            let date = extract_first_date(&text);
            rows.push(TemporalEffect {
                temporal_effect_id: format!(
                    "temporal_effect:{}",
                    stable_id(&format!(
                        "{}::{effect_type}::{}",
                        provision.provision_id, text
                    ))
                ),
                source_note_id: None,
                source_provision_id: Some(provision.provision_id.clone()),
                version_id: Some(provision.version_id.clone()),
                canonical_id: Some(provision.canonical_id.clone()),
                effect_type: effect_type.to_string(),
                trigger_text: text.clone(),
                effective_date: if effect_type == "effective" {
                    date.clone()
                } else {
                    None
                },
                operative_date: if effect_type == "operative" {
                    date.clone()
                } else {
                    None
                },
                repeal_date: if effect_type == "repeal" {
                    date.clone()
                } else {
                    None
                },
                expiration_date: if effect_type == "expiration" {
                    date.clone()
                } else {
                    None
                },
                session_law_ref: extract_first_session_law_citation(&text),
                confidence: 0.62,
            });
        }
    }

    retain_unique_by(&mut rows, |row| row.temporal_effect_id.clone());
    rows
}

fn retain_unique_by<T, F>(rows: &mut Vec<T>, mut key_fn: F)
where
    F: FnMut(&T) -> String,
{
    let mut seen = HashSet::new();
    rows.retain(|row| seen.insert(key_fn(row)));
}

fn extract_temporal_effects(note: &SourceNote, edition_year: i32) -> Vec<TemporalEffect> {
    let text = normalize_ws(&note.text);
    let lowered = text.to_lowercase();
    let mut effects = Vec::new();
    let patterns = [
        (
            "operative",
            ["becomes operative", "is operative", "become operative"].as_slice(),
        ),
        (
            "effective",
            ["becomes effective", "takes effect", "effective date"].as_slice(),
        ),
        (
            "repeal",
            ["is repealed", "repealed on", "repealed"].as_slice(),
        ),
        ("sunset", ["sunsets", "sunset"].as_slice()),
        ("applies_to", ["applies to"].as_slice()),
        ("expires", ["expires", "expiration"].as_slice()),
        (
            "retroactive",
            ["retroactive", "applies retroactively"].as_slice(),
        ),
        (
            "delayed_effective",
            ["notwithstanding the effective date"].as_slice(),
        ),
    ];

    for (effect_type, triggers) in patterns {
        if !triggers.iter().any(|trigger| lowered.contains(*trigger)) {
            continue;
        }
        let date = extract_first_date(&text);
        let temporal_effect_id = format!(
            "temporal_effect:{}",
            stable_id(&format!("{}::{effect_type}::{text}", note.source_note_id))
        );
        effects.push(TemporalEffect {
            temporal_effect_id,
            source_note_id: Some(note.source_note_id.clone()),
            source_provision_id: note.provision_id.clone(),
            version_id: note.version_id.clone(),
            canonical_id: Some(note.canonical_id.clone()),
            effect_type: effect_type.to_string(),
            trigger_text: text.clone(),
            effective_date: if effect_type == "effective" {
                date.clone()
            } else {
                None
            },
            operative_date: if effect_type == "operative" {
                date.clone()
            } else {
                None
            },
            repeal_date: if effect_type == "repeal" {
                date.clone()
            } else {
                None
            },
            expiration_date: if matches!(effect_type, "sunset" | "expires") {
                date
            } else {
                None
            },
            session_law_ref: extract_first_session_law_citation(&text),
            confidence: if edition_year > 0 { 0.78 } else { 0.7 },
        });
    }

    effects
}

fn extract_lineage_events(note: &SourceNote) -> Vec<LineageEvent> {
    let text = normalize_ws(&note.text);
    let mut events = Vec::new();

    for caps in FORMERLY_RE.captures_iter(&text) {
        let from = caps.get(1).unwrap().as_str();
        events.push(LineageEvent {
            lineage_event_id: format!(
                "lineage_event:{}",
                stable_id(&format!("{}::formerly::{from}", note.source_note_id))
            ),
            source_note_id: Some(note.source_note_id.clone()),
            from_canonical_id: Some(format!("or:ors:{from}")),
            to_canonical_id: Some(note.canonical_id.clone()),
            current_canonical_id: note.canonical_id.clone(),
            lineage_type: "formerly".to_string(),
            raw_text: text.clone(),
            year: None,
            confidence: 0.85,
        });
    }

    for caps in RENUMBERED_RE.captures_iter(&text) {
        let target = caps.get(1).unwrap().as_str();
        let year = caps.get(2).and_then(|m| m.as_str().parse::<i32>().ok());
        events.push(LineageEvent {
            lineage_event_id: format!(
                "lineage_event:{}",
                stable_id(&format!("{}::renumbered_to::{target}", note.source_note_id))
            ),
            source_note_id: Some(note.source_note_id.clone()),
            from_canonical_id: Some(note.canonical_id.clone()),
            to_canonical_id: Some(format!("or:ors:{target}")),
            current_canonical_id: note.canonical_id.clone(),
            lineage_type: "renumbered_to".to_string(),
            raw_text: text.clone(),
            year,
            confidence: 0.85,
        });
    }

    for caps in REPEALED_BY_RE.captures_iter(&text) {
        let raw = caps.get(1).unwrap().as_str();
        let year = raw
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<i32>().ok());
        events.push(LineageEvent {
            lineage_event_id: format!(
                "lineage_event:{}",
                stable_id(&format!("{}::repealed_by::{raw}", note.source_note_id))
            ),
            source_note_id: Some(note.source_note_id.clone()),
            from_canonical_id: Some(note.canonical_id.clone()),
            to_canonical_id: None,
            current_canonical_id: note.canonical_id.clone(),
            lineage_type: "repealed_by".to_string(),
            raw_text: text.clone(),
            year,
            confidence: 0.82,
        });
    }

    events
}

fn extract_session_laws_from_text(
    text: &str,
    source_document: &SourceDocument,
    source_note: Option<&SourceNote>,
) -> Vec<SessionLaw> {
    let mut rows = Vec::new();
    for caps in SESSION_LAW_RE.captures_iter(text) {
        rows.push(session_law_from_parts(
            caps.get(1).unwrap().as_str(),
            caps.get(2).unwrap().as_str(),
            caps.get(3).map(|m| m.as_str()),
            text,
            source_document,
            source_note,
        ));
    }
    for caps in OREGON_LAWS_RE.captures_iter(text) {
        rows.push(session_law_from_parts(
            caps.get(1).unwrap().as_str(),
            caps.get(2).unwrap().as_str(),
            caps.get(3).map(|m| m.as_str()),
            text,
            source_document,
            source_note,
        ));
    }
    for caps in OREGON_LAWS_ALT_RE.captures_iter(text) {
        rows.push(session_law_from_parts(
            caps.get(3).unwrap().as_str(),
            caps.get(2).unwrap().as_str(),
            caps.get(1).map(|m| m.as_str()),
            text,
            source_document,
            source_note,
        ));
    }
    retain_unique_by(&mut rows, |row| row.session_law_id.clone());
    rows
}

fn session_law_from_parts(
    year: &str,
    chapter: &str,
    section: Option<&str>,
    raw_text: &str,
    source_document: &SourceDocument,
    source_note: Option<&SourceNote>,
) -> SessionLaw {
    let session_law_id = match section {
        Some(sec) => format!("or:laws:{year}:c:{chapter}:s:{sec}"),
        None => format!("or:laws:{year}:c:{chapter}"),
    };
    let citation = match section {
        Some(sec) => format!("{year} c.{chapter} §{sec}"),
        None => format!("{year} c.{chapter}"),
    };
    SessionLaw {
        session_law_id,
        jurisdiction_id: Some("or:state".to_string()),
        citation,
        year: year.parse::<i32>().unwrap_or(source_document.edition_year),
        chapter: Some(chapter.to_string()),
        section: section.map(ToString::to_string),
        bill_number: None,
        effective_date: None,
        text: Some(normalize_ws(raw_text)),
        raw_text: Some(normalize_ws(raw_text)),
        source_document_id: Some(source_document.source_document_id.clone()),
        source_note_id: source_note.map(|n| n.source_note_id.clone()),
        confidence: 0.85,
    }
}

fn extract_first_date(text: &str) -> Option<String> {
    DATE_RE.find(text).map(|m| m.as_str().to_string())
}

fn extract_first_session_law_citation(text: &str) -> Option<String> {
    SESSION_LAW_RE
        .captures(text)
        .map(|caps| {
            let year = caps.get(1).unwrap().as_str();
            let chapter = caps.get(2).unwrap().as_str();
            match caps.get(3).map(|m| m.as_str()) {
                Some(sec) => format!("{year} c.{chapter} §{sec}"),
                None => format!("{year} c.{chapter}"),
            }
        })
        .or_else(|| {
            OREGON_LAWS_RE.captures(text).map(|caps| {
                let year = caps.get(1).unwrap().as_str();
                let chapter = caps.get(2).unwrap().as_str();
                match caps.get(3).map(|m| m.as_str()) {
                    Some(sec) => format!("{year} c.{chapter} §{sec}"),
                    None => format!("{year} c.{chapter}"),
                }
            })
        })
        .or_else(|| {
            OREGON_LAWS_ALT_RE.captures(text).map(|caps| {
                let year = caps.get(3).unwrap().as_str();
                let chapter = caps.get(2).unwrap().as_str();
                match caps.get(1).map(|m| m.as_str()) {
                    Some(sec) => format!("{year} c.{chapter} §{sec}"),
                    None => format!("{year} c.{chapter}"),
                }
            })
        })
}

struct PenaltyDetails {
    penalty_type: Option<String>,
    amount: Option<String>,
    minimum: Option<String>,
    maximum: Option<String>,
    criminal_class: Option<String>,
    civil_penalty_amount: Option<String>,
    jail_term: Option<String>,
    license_suspension: Option<bool>,
    revocation: Option<bool>,
    target_conduct: Option<String>,
    target_citation: Option<String>,
    has_detail: bool,
}

fn extract_penalty_details(text: &str) -> PenaltyDetails {
    let lowered = text.to_lowercase();
    let class = PENALTY_CLASS_RE.captures(text).map(|caps| {
        format!(
            "Class {} {}",
            caps.get(1).unwrap().as_str(),
            caps.get(2).unwrap().as_str().to_lowercase()
        )
    });
    let money = MONEY_RE.find(text).map(|m| m.as_str().to_string());
    let not_to_exceed = lowered.contains("not to exceed") || lowered.contains("may not exceed");
    let penalty_type = if lowered.contains("civil penalty") {
        Some("civil".to_string())
    } else if class.is_some() || lowered.contains("misdemeanor") || lowered.contains("felony") {
        Some("criminal".to_string())
    } else if lowered.contains("suspend") || lowered.contains("revoke") {
        Some("administrative".to_string())
    } else {
        None
    };
    let target_citation = Regex::new(r"ORS\s+\d{1,3}\.\d{3,4}")
        .ok()
        .and_then(|re| re.find(text).map(|m| m.as_str().to_string()));
    let jail_term = JAIL_RE.find(text).map(|m| normalize_ws(m.as_str()));
    let license_suspension = lowered.contains("license") && lowered.contains("suspend");
    let revocation = lowered.contains("revocation") || lowered.contains("revoke");
    let has_detail = class.is_some()
        || money.is_some()
        || jail_term.is_some()
        || license_suspension
        || revocation
        || penalty_type.is_some();

    PenaltyDetails {
        penalty_type,
        amount: money.clone(),
        minimum: None,
        maximum: if not_to_exceed { money.clone() } else { None },
        criminal_class: class,
        civil_penalty_amount: if lowered.contains("civil penalty") {
            money.clone()
        } else {
            None
        },
        jail_term,
        license_suspension: if license_suspension { Some(true) } else { None },
        revocation: if revocation { Some(true) } else { None },
        target_conduct: text.split(". ").next().map(normalize_ws),
        target_citation,
        has_detail,
    }
}

fn extract_money_amounts(provision: &Provision, text: &str) -> Vec<MoneyAmount> {
    let mut rows = Vec::new();

    for m in MONEY_RE.find_iter(text) {
        let amount_text = m.as_str().to_string();
        rows.push(MoneyAmount {
            money_amount_id: format!(
                "money_amount:{}",
                stable_id(&format!("{}::{}", provision.provision_id, amount_text))
            ),
            amount_value: parse_money_value(&amount_text),
            percent_value: None,
            amount_type: Some("currency".to_string()),
            amount_text,
            source_provision_id: provision.provision_id.clone(),
            confidence: 0.82,
        });
    }

    for m in PERCENT_RE.find_iter(text) {
        let amount_text = m.as_str().to_string();
        rows.push(MoneyAmount {
            money_amount_id: format!(
                "money_amount:{}",
                stable_id(&format!("{}::{}", provision.provision_id, amount_text))
            ),
            amount_value: None,
            percent_value: parse_percent_value(&amount_text),
            amount_type: Some("percent".to_string()),
            amount_text,
            source_provision_id: provision.provision_id.clone(),
            confidence: 0.78,
        });
    }

    rows
}

fn extract_tax_rule(provision: &Provision, text: &str) -> Option<TaxRule> {
    let lowered = text.to_lowercase();
    if !lowered.contains("tax")
        && !lowered.contains("assessment")
        && !lowered.contains("rate limit")
        && !lowered.contains("levy")
    {
        return None;
    }

    let rate_text = PERCENT_RE.find(text).map(|m| m.as_str().to_string());
    let tax_type = if lowered.contains("ad valorem") {
        Some("ad_valorem".to_string())
    } else if lowered.contains("construction tax") {
        Some("construction".to_string())
    } else if lowered.contains("amusement device") {
        Some("amusement_device".to_string())
    } else if lowered.contains("income tax") {
        Some("income".to_string())
    } else if lowered.contains("special tax") {
        Some("special".to_string())
    } else {
        Some("tax".to_string())
    };

    Some(TaxRule {
        tax_rule_id: format!(
            "tax_rule:{}",
            stable_id(&format!("{}::{}", provision.provision_id, text))
        ),
        tax_type,
        rate_text,
        base: extract_after_phrase(text, "of ", 100),
        cap: if lowered.contains("may not exceed")
            || lowered.contains("not to exceed")
            || lowered.contains("limit")
        {
            Some(first_sentence(text))
        } else {
            None
        },
        recipient: extract_recipient(text),
        fund_name: extract_fund_name(text),
        source_provision_id: provision.provision_id.clone(),
        confidence: 0.64,
    })
}

fn extract_rate_limits(provision: &Provision, text: &str) -> Vec<RateLimit> {
    let lowered = text.to_lowercase();
    if !lowered.contains("limit")
        && !lowered.contains("may not exceed")
        && !lowered.contains("not to exceed")
        && PERCENT_RE.find(text).is_none()
    {
        return Vec::new();
    }

    let mut rows = Vec::new();
    if let Some(percent) = PERCENT_RE.find(text) {
        let amount_text = percent.as_str().to_string();
        rows.push(RateLimit {
            rate_limit_id: format!(
                "rate_limit:{}",
                stable_id(&format!(
                    "{}::percent::{}",
                    provision.provision_id, amount_text
                ))
            ),
            rate_type: Some(if lowered.contains("bond") {
                "bond_cap".to_string()
            } else if lowered.contains("tax") {
                "tax_rate_limit".to_string()
            } else {
                "percentage_limit".to_string()
            }),
            percent_value: parse_percent_value(&amount_text),
            amount_text: Some(amount_text),
            cap_text: Some(first_sentence(text)),
            source_provision_id: provision.provision_id.clone(),
            confidence: 0.7,
        });
    } else if lowered.contains("limit")
        || lowered.contains("may not exceed")
        || lowered.contains("not to exceed")
    {
        rows.push(RateLimit {
            rate_limit_id: format!(
                "rate_limit:{}",
                stable_id(&format!("{}::limit::{}", provision.provision_id, text))
            ),
            rate_type: Some("textual_limit".to_string()),
            percent_value: None,
            amount_text: MONEY_RE.find(text).map(|m| m.as_str().to_string()),
            cap_text: Some(first_sentence(text)),
            source_provision_id: provision.provision_id.clone(),
            confidence: 0.58,
        });
    }

    rows
}

fn extract_required_notice(provision: &Provision, text: &str) -> Option<RequiredNotice> {
    let lowered = text.to_lowercase();
    if !(lowered.contains("notice")
        || lowered.contains("notify")
        || lowered.contains("service or delivery"))
    {
        return None;
    }
    if !(lowered.contains("shall")
        || lowered.contains("must")
        || lowered.contains("required")
        || lowered.contains("may"))
    {
        return None;
    }

    Some(RequiredNotice {
        required_notice_id: format!(
            "required_notice:{}",
            stable_id(&format!("{}::{}", provision.provision_id, text))
        ),
        notice_type: Some(if lowered.contains("actual notice") {
            "actual_notice".to_string()
        } else {
            "notice".to_string()
        }),
        text: text.to_string(),
        required_recipient: extract_notice_party(text, "to"),
        required_sender: extract_actor_text(text),
        trigger_event: extract_condition_text(text),
        source_provision_id: provision.provision_id.clone(),
        confidence: 0.66,
    })
}

fn extract_form_text(provision: &Provision, text: &str) -> Option<FormText> {
    let lowered = text.to_lowercase();
    if !(lowered.contains("form")
        || lowered.contains("substantially the following")
        || lowered.contains("notice must contain")
        || text.contains("_____")
        || text.contains("____"))
    {
        return None;
    }

    Some(FormText {
        form_text_id: format!(
            "form_text:{}",
            stable_id(&format!("{}::{}", provision.provision_id, text))
        ),
        form_type: Some(if lowered.contains("notice") {
            "notice_template".to_string()
        } else {
            "form_text".to_string()
        }),
        text: text.to_string(),
        source_provision_id: provision.provision_id.clone(),
        source_paragraph_ids: provision.source_paragraph_ids.clone(),
        confidence: 0.62,
    })
}

fn extract_remedy(provision: &Provision, text: &str) -> Option<Remedy> {
    let lowered = text.to_lowercase();
    if !(lowered.contains("remedy")
        || lowered.contains("recover damages")
        || lowered.contains("injunctive relief")
        || lowered.contains("may recover")
        || lowered.contains("terminate the rental agreement")
        || lowered.contains("bring an action"))
    {
        return None;
    }

    Some(Remedy {
        remedy_id: format!(
            "remedy:{}",
            stable_id(&format!("{}::{}", provision.provision_id, text))
        ),
        text: text.to_string(),
        remedy_type: Some(
            if lowered.contains("damages") || lowered.contains("recover") {
                "damages".to_string()
            } else if lowered.contains("injunctive") {
                "injunctive_relief".to_string()
            } else if lowered.contains("terminate") {
                "termination".to_string()
            } else {
                "remedy".to_string()
            },
        ),
        available_to: extract_actor_text(text),
        available_against: None,
        source_provision_id: provision.provision_id.clone(),
        confidence: 0.62,
    })
}

fn parse_money_value(amount: &str) -> Option<f64> {
    amount
        .trim_start_matches('$')
        .replace(',', "")
        .parse::<f64>()
        .ok()
}

fn parse_percent_value(amount: &str) -> Option<f64> {
    amount
        .to_lowercase()
        .replace("percent", "")
        .trim()
        .parse::<f64>()
        .ok()
}

fn extract_actor_text(text: &str) -> Option<String> {
    let lowered = text.to_lowercase();
    let actors = [
        "landlord",
        "tenant",
        "applicant",
        "department",
        "Department of Revenue",
        "Housing and Community Services Department",
        "Oregon State Lottery",
        "district board",
        "district",
        "county governing body",
        "city",
        "electors",
        "local government",
        "school district",
    ];
    actors
        .iter()
        .find(|actor| lowered.contains(&actor.to_lowercase()))
        .map(|actor| (*actor).to_string())
}

fn classify_actor_type(actor: &str) -> Option<&'static str> {
    let lowered = actor.to_lowercase();
    if lowered.contains("department")
        || lowered.contains("lottery")
        || lowered.contains("district")
        || lowered.contains("county")
        || lowered.contains("city")
        || lowered.contains("government")
    {
        Some("public_body")
    } else if matches!(
        lowered.as_str(),
        "landlord" | "tenant" | "applicant" | "electors"
    ) {
        Some("role")
    } else {
        None
    }
}

fn extract_action_text(text: &str) -> Option<(String, Option<String>)> {
    let re = Regex::new(
        r"(?i)\b(shall|must|may|is required to|required to|may not)\s+([A-Za-z][^.;]{0,120})",
    )
    .ok()?;
    let caps = re.captures(text)?;
    let modality = caps.get(1)?.as_str().to_lowercase();
    let object = caps.get(2).map(|m| first_clause(m.as_str()));
    Some((modality, object))
}

fn extract_condition_text(text: &str) -> Option<String> {
    let lowered = text.to_lowercase();
    for phrase in [
        "if ",
        "when ",
        "unless ",
        "except as provided",
        "provided that",
    ] {
        if let Some(idx) = lowered.find(phrase) {
            return Some(first_clause(&text[idx..]));
        }
    }
    None
}

fn extract_duration_text(text: &str) -> Option<String> {
    DURATION_RE.find(text).map(|m| first_clause(m.as_str()))
}

fn extract_deadline_trigger(text: &str) -> Option<String> {
    let lowered = text.to_lowercase();
    lowered
        .find(" after ")
        .map(|idx| first_clause(&text[idx + 1..]))
}

fn extract_after_phrase(text: &str, phrase: &str, max_len: usize) -> Option<String> {
    let idx = text.to_lowercase().find(&phrase.to_lowercase())?;
    let start = idx + phrase.len();
    Some(first_clause(&text[start..]).chars().take(max_len).collect())
}

fn extract_recipient(text: &str) -> Option<String> {
    extract_after_phrase(text, "to the ", 80).map(|s| format!("the {s}"))
}

fn extract_notice_party(text: &str, phrase: &str) -> Option<String> {
    extract_after_phrase(text, &format!("{phrase} "), 80)
}

fn extract_fund_name(text: &str) -> Option<String> {
    let re = Regex::new(r"(?i)\b([A-Z][A-Za-z ]{2,80}\s+Fund)\b").ok()?;
    re.captures(text)
        .and_then(|caps| caps.get(1).map(|m| normalize_ws(m.as_str())))
}

fn first_sentence(text: &str) -> String {
    text.split_once(". ")
        .map(|(head, _)| normalize_ws(head))
        .unwrap_or_else(|| normalize_ws(text))
}

fn first_clause(text: &str) -> String {
    text.split([';', '.'])
        .next()
        .map(normalize_ws)
        .unwrap_or_else(|| normalize_ws(text))
}

fn definition_scope_type(provision: &Provision, text: &str) -> &'static str {
    let lowered = text.to_lowercase();
    if SCOPE_RANGE_RE.is_match(text) {
        "range"
    } else if lowered.contains("as used in this chapter") {
        "chapter"
    } else if lowered.contains("as used in this section") {
        "section"
    } else if provision
        .heading_path
        .iter()
        .any(|h| h.to_lowercase().contains("article"))
    {
        "article"
    } else {
        "section"
    }
}

fn definition_scope_citation(provision: &Provision, text: &str) -> Option<String> {
    if let Some((start, end)) = definition_scope_range(text) {
        Some(format!("ORS {start} to {end}"))
    } else if text.to_lowercase().contains("as used in this chapter") {
        Some(format!(
            "ORS chapter {}",
            chapter_from_citation(&provision.citation)
        ))
    } else {
        Some(provision.citation.clone())
    }
}

fn definition_scope_target_canonical(provision: &Provision, text: &str) -> Option<String> {
    if definition_scope_type(provision, text) == "section" {
        Some(provision.canonical_id.clone())
    } else {
        None
    }
}

fn definition_scope_target_chapter(provision: &Provision, text: &str) -> Option<String> {
    if definition_scope_type(provision, text) == "chapter" {
        Some(format!(
            "or:ors:chapter:{}",
            chapter_from_citation(&provision.citation)
        ))
    } else {
        None
    }
}

fn definition_scope_range(text: &str) -> Option<(String, String)> {
    let caps = SCOPE_RANGE_RE.captures(text)?;
    Some((
        format!("or:ors:{}", caps.get(1)?.as_str()),
        format!("or:ors:{}", caps.get(2)?.as_str()),
    ))
}

fn chapter_from_citation(citation: &str) -> String {
    citation
        .trim_start_matches("ORS ")
        .split('.')
        .next()
        .unwrap_or("")
        .to_string()
}

fn extract_defined_term(text: &str) -> Option<String> {
    QUOTED_TERM_RE
        .captures(text)
        .and_then(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        .or_else(|| {
            MEANS_RE
                .captures(text)
                .and_then(|captures| captures.get(1).map(|m| normalize_ws(m.as_str())))
        })
}

fn extract_trigger_phrase(text: &str, phrases: &[&str]) -> Option<String> {
    phrases
        .iter()
        .find(|phrase| text.contains(**phrase))
        .map(|phrase| (*phrase).to_string())
}

fn semantic_types_for_provision(
    provision: &Provision,
    text: &str,
    is_obligation: bool,
) -> Vec<&'static str> {
    let mut types = Vec::new();
    let lowered = text.to_lowercase();
    if is_obligation {
        types.push("Obligation");
    }
    if lowered.contains(" may ") || lowered.starts_with("may ") {
        types.push("Permission");
    }
    if lowered.contains("shall have the power")
        || lowered.contains("has the power")
        || lowered.contains("power to")
        || lowered.contains("authority to")
    {
        types.push("Power");
        types.push("AuthorityGrant");
    }
    if lowered.contains("may not")
        || lowered.contains("shall not")
        || lowered.contains("must not")
        || lowered.contains("prohibited")
    {
        types.push("Prohibition");
    }
    if provision.is_definition_candidate {
        types.push("Definition");
    }
    if provision.is_exception_candidate {
        types.push("Exception");
    }
    if provision.is_deadline_candidate {
        types.push("Deadline");
    }
    if provision.is_penalty_candidate {
        types.push("Penalty");
    }
    if lowered.contains("remedy")
        || lowered.contains("recover damages")
        || lowered.contains("injunctive relief")
        || lowered.contains("may recover")
    {
        types.push("Remedy");
    }
    if MONEY_RE.is_match(text) {
        types.push("MoneyAmount");
    }
    if lowered.contains("tax")
        || lowered.contains("assessment")
        || lowered.contains("levy")
        || lowered.contains("rate limit")
    {
        types.push("TaxRule");
    }
    if lowered.contains("bond") || lowered.contains("indebtedness") || lowered.contains("debt") {
        types.push("BondAuthority");
    }
    if lowered.contains("distribute")
        || lowered.contains("distribution")
        || lowered.contains("fund")
        || lowered.contains("account")
        || lowered.contains("pay over")
    {
        types.push("DistributionRule");
    }
    if lowered.contains("notice") || lowered.contains("notify") {
        types.push("RequiredNotice");
    }
    if lowered.contains("form")
        || lowered.contains("oath")
        || lowered.contains("disclosure")
        || text.contains("____")
    {
        types.push("FormText");
    }
    if DURATION_RE.is_match(text) || DATE_RE.is_match(text) {
        types.push("TimePeriod");
    }
    types.sort_unstable();
    types.dedup();
    types
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LegalTextVersion, Provision, SourceDocument};

    #[test]
    fn test_extract_defined_term() {
        assert_eq!(
            extract_defined_term(r#" "Vehicle" means a thing. "#),
            Some("Vehicle".to_string())
        );
        assert_eq!(
            extract_defined_term("Vehicle means a thing."),
            Some("Vehicle".to_string())
        );
    }

    #[test]
    fn test_derive_semantic_nodes() {
        let provisions = vec![
            Provision {
                provision_id: "p1".to_string(),
                text: r#" "Vehicle" means a thing. "#.to_string(),
                is_definition_candidate: true,
                ..Default::default()
            },
            Provision {
                provision_id: "p2".to_string(),
                text: "The person shall pay a fine.".to_string(),
                is_penalty_candidate: true,
                ..Default::default()
            },
        ];

        let derived = derive_semantic_nodes(&provisions);
        assert_eq!(derived.definitions.len(), 1);
        assert_eq!(derived.definitions[0].term, "Vehicle");
        assert_eq!(derived.obligations.len(), 1);
        assert_eq!(derived.penalties.len(), 1);
    }

    #[test]
    fn test_derive_semantic_nodes_dedupes_stable_ids() {
        let provisions = vec![
            Provision {
                provision_id: "p1".to_string(),
                canonical_id: "or:ors:1.001".to_string(),
                citation: "ORS 1.001".to_string(),
                text: r#""Vehicle" means a thing."#.to_string(),
                is_definition_candidate: true,
                ..Default::default()
            },
            Provision {
                provision_id: "p1".to_string(),
                canonical_id: "or:ors:1.001".to_string(),
                citation: "ORS 1.001".to_string(),
                text: r#""Vehicle" means a thing."#.to_string(),
                is_definition_candidate: true,
                ..Default::default()
            },
        ];

        let derived = derive_semantic_nodes(&provisions);
        assert_eq!(derived.defined_terms.len(), 1);
        assert_eq!(derived.definitions.len(), 1);
        assert_eq!(derived.definition_scopes.len(), 1);
        assert_eq!(derived.legal_semantic_nodes.len(), 1);
    }

    #[test]
    fn test_derive_historical_nodes() {
        let versions = vec![LegalTextVersion {
            version_id: "v1".to_string(),
            status: "repealed".to_string(),
            edition_year: 2025,
            ..Default::default()
        }];
        let source_doc = SourceDocument {
            source_document_id: "doc1".to_string(),
            ..Default::default()
        };
        let derived = derive_historical_nodes(&versions, &source_doc);
        assert_eq!(derived.status_events.len(), 1);
        assert_eq!(derived.status_events[0].status_type, "repealed");
    }
}
