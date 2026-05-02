use crate::embedding_profiles::*;
use crate::hash::sha256_hex;
use crate::neo4j_loader::{EmbeddingCandidate, EmbeddingUpdate, Neo4jLoader};
use crate::voyage::{VOYAGE_4_LARGE, VoyageClient, estimate_tokens, model_config};
use anyhow::{Result, anyhow};
use neo4rs::query;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EmbeddingPhase {
    Chunks = 1,
    Authority = 2,
    Semantic = 3,
    DefinitionsHistory = 4,
    Specialized = 5,
}

impl EmbeddingPhase {
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::Chunks),
            2 => Ok(Self::Authority),
            3 => Ok(Self::Semantic),
            4 => Ok(Self::DefinitionsHistory),
            5 => Ok(Self::Specialized),
            _ => Err(anyhow!("unsupported embedding phase {value}; expected 1-5")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EmbeddingTargetSpec {
    pub label: &'static str,
    pub id_property: &'static str,
    pub profile: &'static EmbeddingProfile,
    pub phase: EmbeddingPhase,
    pub smoke_limit: usize,
    pub input_expr: &'static str,
    pub where_clause: &'static str,
}

#[derive(Debug, Clone)]
pub struct EmbeddingRunConfig {
    pub edition_year: i32,
    pub smoke: bool,
    pub resume: bool,
    pub max_label_nodes: Option<usize>,
    pub phases: BTreeSet<EmbeddingPhase>,
    pub embedding_batch_size: usize,
    pub scan_batch_size: usize,
    pub max_batch_chars: usize,
    pub max_batch_estimated_tokens: usize,
    pub create_vector_indexes: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct EmbeddingRunReport {
    pub labels: BTreeMap<String, EmbeddingLabelReport>,
    pub request_count: usize,
    pub voyage_total_tokens: usize,
    pub vector_dimension_mismatches: usize,
    pub metadata_mismatches: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct EmbeddingLabelReport {
    pub embedded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub over_context: usize,
    pub scanned: usize,
    pub request_count: usize,
    pub voyage_total_tokens: usize,
}

pub const EMBEDDING_TARGETS: &[EmbeddingTargetSpec] = &[
    EmbeddingTargetSpec {
        label: "RetrievalChunk",
        id_property: "chunk_id",
        profile: &LEGAL_CHUNK_PRIMARY,
        phase: EmbeddingPhase::Chunks,
        smoke_limit: 25,
        where_clause: "n.embedding_policy IN ['embed_primary', 'embed_special'] AND n.text IS NOT NULL AND n.text <> ''",
        input_expr: "CASE coalesce(n.authority_family, 'ORS') WHEN 'ORCONST' THEN 'Oregon Constitution. ' WHEN 'UTCR' THEN 'Oregon Uniform Trial Court Rules. ' WHEN 'SLR' THEN 'Oregon Supplementary Local Court Rules. ' ELSE 'Oregon Revised Statutes. ' END + toString(coalesce(n.edition_year, $edition_year)) + ' Edition.\\nChunk type: ' + coalesce(n.chunk_type, '') + '\\nCitation: ' + coalesce(n.citation, '') + '\\nBreadcrumb: ' + coalesce(n.breadcrumb, '') + '\\nSource kind: ' + coalesce(n.source_kind, '') + '\\n\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "Provision",
        id_property: "provision_id",
        profile: &LEGAL_PROVISION_PRIMARY,
        phase: EmbeddingPhase::Authority,
        smoke_limit: 25,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "CASE coalesce(n.authority_family, 'ORS') WHEN 'ORCONST' THEN 'Oregon Constitution. ' WHEN 'UTCR' THEN 'Oregon Uniform Trial Court Rules. ' WHEN 'SLR' THEN 'Oregon Supplementary Local Court Rules. ' ELSE 'Oregon Revised Statutes. ' END + toString($edition_year) + ' Edition.\\nCitation: ' + coalesce(n.display_citation, n.citation, '') + '\\nParent authority: ' + coalesce(n.citation, '') + '\\nProvision type: ' + coalesce(n.provision_type, '') + '\\nStatus: active\\n\\nProvision text:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "LegalTextVersion",
        id_property: "version_id",
        profile: &LEGAL_VERSION_PRIMARY,
        phase: EmbeddingPhase::Authority,
        smoke_limit: 25,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "CASE coalesce(n.authority_family, 'ORS') WHEN 'ORCONST' THEN 'Oregon Constitution. ' WHEN 'UTCR' THEN 'Oregon Uniform Trial Court Rules. ' WHEN 'SLR' THEN 'Oregon Supplementary Local Court Rules. ' ELSE 'Oregon Revised Statutes. ' END + toString(coalesce(n.edition_year, $edition_year)) + ' Edition.\\nCitation: ' + coalesce(n.citation, '') + '\\nTitle: ' + coalesce(n.title, '') + '\\nStatus: ' + coalesce(n.status, '') + '\\nChapter: ' + coalesce(n.chapter, '') + '\\n\\nAuthority text:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "LegalSemanticNode",
        id_property: "semantic_id",
        profile: &LEGAL_SEMANTIC_NODE_PRIMARY,
        phase: EmbeddingPhase::Semantic,
        smoke_limit: 0,
        where_clause: "n.text IS NOT NULL AND n.text <> '' AND NOT n:Obligation AND NOT n:Exception AND NOT n:Deadline AND NOT n:Penalty AND NOT n:Remedy AND NOT n:RequiredNotice AND NOT n:FormText AND NOT n:MoneyAmount AND NOT n:TaxRule AND NOT n:RateLimit",
        input_expr: "'Oregon legal semantic node.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nType: ' + coalesce(n.semantic_type, '') + '\\n\\nText:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "Obligation",
        id_property: "obligation_id",
        profile: &LEGAL_OBLIGATION_PRIMARY,
        phase: EmbeddingPhase::Semantic,
        smoke_limit: 25,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "'Oregon legal obligation.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nActor: ' + coalesce(n.actor_text, '') + '\\nAction: ' + coalesce(n.action_text, '') + '\\nCondition: ' + coalesce(n.condition_text, '') + '\\n\\nText:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "Exception",
        id_property: "exception_id",
        profile: &LEGAL_EXCEPTION_PRIMARY,
        phase: EmbeddingPhase::Semantic,
        smoke_limit: 0,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "'Oregon legal exception.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nTrigger: ' + coalesce(n.trigger_phrase, '') + '\\nType: ' + coalesce(n.exception_type, '') + '\\n\\nText:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "Deadline",
        id_property: "deadline_id",
        profile: &LEGAL_DEADLINE_PRIMARY,
        phase: EmbeddingPhase::Semantic,
        smoke_limit: 25,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "'Oregon legal deadline.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nDeadline: ' + coalesce(n.duration, n.date_text, '') + '\\nTrigger: ' + coalesce(n.trigger_event, '') + '\\nActor: ' + coalesce(n.actor, '') + '\\nRequired action: ' + coalesce(n.action_required, '') + '\\n\\nText:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "Penalty",
        id_property: "penalty_id",
        profile: &LEGAL_PENALTY_PRIMARY,
        phase: EmbeddingPhase::Semantic,
        smoke_limit: 25,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "'Oregon legal penalty.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nPenalty type: ' + coalesce(n.penalty_type, '') + '\\nAmount/class: ' + coalesce(n.amount, n.civil_penalty_amount, n.criminal_class, '') + '\\nTarget conduct: ' + coalesce(n.target_conduct, n.condition, '') + '\\n\\nText:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "Remedy",
        id_property: "remedy_id",
        profile: &LEGAL_REMEDY_PRIMARY,
        phase: EmbeddingPhase::Semantic,
        smoke_limit: 0,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "'Oregon legal remedy.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nRemedy type: ' + coalesce(n.remedy_type, '') + '\\nAvailable to: ' + coalesce(n.available_to, '') + '\\nAvailable against: ' + coalesce(n.available_against, '') + '\\n\\nText:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "Definition",
        id_property: "definition_id",
        profile: &LEGAL_DEFINITION_PRIMARY,
        phase: EmbeddingPhase::DefinitionsHistory,
        smoke_limit: 25,
        where_clause: "n.definition_text IS NOT NULL AND n.definition_text <> ''",
        input_expr: "'Oregon legal definition.\\nTerm: ' + coalesce(n.term, '') + '\\nScope: ' + coalesce(n.scope_type, '') + ' ' + coalesce(n.scope_citation, '') + '\\nDefined in: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\n\\nDefinition:\\n' + coalesce(n.definition_text, '')",
    },
    EmbeddingTargetSpec {
        label: "DefinedTerm",
        id_property: "defined_term_id",
        profile: &LEGAL_DEFINED_TERM_PRIMARY,
        phase: EmbeddingPhase::DefinitionsHistory,
        smoke_limit: 0,
        where_clause: "n.term IS NOT NULL AND n.term <> ''",
        input_expr: "'Oregon defined legal term.\\nTerm: ' + coalesce(n.term, '') + '\\nNormalized term: ' + coalesce(n.normalized_term, '') + '\\nAuthority family: ' + coalesce(n.authority_family, '')",
    },
    EmbeddingTargetSpec {
        label: "DefinitionScope",
        id_property: "definition_scope_id",
        profile: &LEGAL_DEFINITION_SCOPE_PRIMARY,
        phase: EmbeddingPhase::DefinitionsHistory,
        smoke_limit: 0,
        where_clause: "n.scope_type IS NOT NULL AND n.scope_type <> ''",
        input_expr: "'Oregon legal definition scope.\\nScope type: ' + coalesce(n.scope_type, '') + '\\nScope citation: ' + coalesce(n.scope_citation, '') + '\\nTarget chapter: ' + coalesce(n.target_chapter_id, '') + '\\nTarget range: ' + coalesce(n.target_range_start, '') + ' ' + coalesce(n.target_range_end, '')",
    },
    EmbeddingTargetSpec {
        label: "SourceNote",
        id_property: "source_note_id",
        profile: &LEGAL_SOURCE_NOTE_PRIMARY,
        phase: EmbeddingPhase::DefinitionsHistory,
        smoke_limit: 25,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "'Oregon legal history/currentness note.\\nCitation: ' + coalesce(n.citation, '') + '\\nType: ' + coalesce(n.note_type, '') + '\\nSession law: \\n\\nText:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "StatusEvent",
        id_property: "status_event_id",
        profile: &LEGAL_STATUS_EVENT_PRIMARY,
        phase: EmbeddingPhase::DefinitionsHistory,
        smoke_limit: 0,
        where_clause: "coalesce(n.status_text, n.trigger_text, n.status_type) IS NOT NULL",
        input_expr: "'Oregon legal status/currentness event.\\nCitation: ' + coalesce(n.canonical_id, '') + '\\nType: ' + coalesce(n.status_type, n.effect_type, '') + '\\nSession law: ' + coalesce(n.session_law_ref, '') + '\\n\\nText:\\n' + coalesce(n.status_text, n.trigger_text, '')",
    },
    EmbeddingTargetSpec {
        label: "TemporalEffect",
        id_property: "temporal_effect_id",
        profile: &LEGAL_TEMPORAL_EFFECT_PRIMARY,
        phase: EmbeddingPhase::DefinitionsHistory,
        smoke_limit: 10,
        where_clause: "coalesce(n.trigger_text, n.effect_type) IS NOT NULL",
        input_expr: "'Oregon legal history/currentness note.\\nCitation: ' + coalesce(n.canonical_id, '') + '\\nType: ' + coalesce(n.effect_type, '') + '\\nSession law: ' + coalesce(n.session_law_ref, '') + '\\n\\nText:\\n' + coalesce(n.trigger_text, '')",
    },
    EmbeddingTargetSpec {
        label: "LineageEvent",
        id_property: "lineage_event_id",
        profile: &LEGAL_LINEAGE_EVENT_PRIMARY,
        phase: EmbeddingPhase::DefinitionsHistory,
        smoke_limit: 0,
        where_clause: "n.raw_text IS NOT NULL AND n.raw_text <> ''",
        input_expr: "'Oregon legal lineage event.\\nCurrent citation: ' + coalesce(n.current_canonical_id, '') + '\\nType: ' + coalesce(n.lineage_type, '') + '\\nYear: ' + toString(coalesce(n.year, '')) + '\\n\\nText:\\n' + coalesce(n.raw_text, '')",
    },
    EmbeddingTargetSpec {
        label: "Amendment",
        id_property: "amendment_id",
        profile: &LEGAL_AMENDMENT_PRIMARY,
        phase: EmbeddingPhase::DefinitionsHistory,
        smoke_limit: 0,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "'Oregon legal amendment.\\nCitation: ' + coalesce(n.canonical_id, n.affected_canonical_id, '') + '\\nType: ' + coalesce(n.amendment_type, '') + '\\nSession law: ' + coalesce(n.session_law_citation, '') + '\\nEffective date: ' + coalesce(n.effective_date, '') + '\\n\\nText:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "SessionLaw",
        id_property: "session_law_id",
        profile: &LEGAL_SESSION_LAW_PRIMARY,
        phase: EmbeddingPhase::DefinitionsHistory,
        smoke_limit: 10,
        where_clause: "coalesce(n.text, n.raw_text, n.citation) IS NOT NULL",
        input_expr: "'Oregon session law.\\nCitation: ' + coalesce(n.citation, '') + '\\nYear: ' + toString(coalesce(n.year, '')) + '\\nChapter: ' + coalesce(n.chapter, '') + '\\nSection: ' + coalesce(n.section, '') + '\\nBill: ' + coalesce(n.bill_number, '') + '\\nEffective date: ' + coalesce(n.effective_date, '') + '\\n\\nText:\\n' + coalesce(n.text, n.raw_text, '')",
    },
    EmbeddingTargetSpec {
        label: "RequiredNotice",
        id_property: "required_notice_id",
        profile: &LEGAL_REQUIRED_NOTICE_PRIMARY,
        phase: EmbeddingPhase::Specialized,
        smoke_limit: 10,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "'Oregon required legal notice.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nNotice type: ' + coalesce(n.notice_type, '') + '\\nSender: ' + coalesce(n.required_sender, '') + '\\nRecipient: ' + coalesce(n.required_recipient, '') + '\\nTrigger: ' + coalesce(n.trigger_event, '') + '\\n\\nText:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "FormText",
        id_property: "form_text_id",
        profile: &LEGAL_FORM_TEXT_PRIMARY,
        phase: EmbeddingPhase::Specialized,
        smoke_limit: 10,
        where_clause: "n.text IS NOT NULL AND n.text <> ''",
        input_expr: "'Oregon statutory form text.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nForm type: ' + coalesce(n.form_type, '') + '\\n\\nText:\\n' + coalesce(n.text, '')",
    },
    EmbeddingTargetSpec {
        label: "MoneyAmount",
        id_property: "money_amount_id",
        profile: &LEGAL_MONEY_AMOUNT_PRIMARY,
        phase: EmbeddingPhase::Specialized,
        smoke_limit: 0,
        where_clause: "n.amount_text IS NOT NULL AND n.amount_text <> ''",
        input_expr: "'Oregon legal money amount.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nAmount: ' + coalesce(n.amount_text, '') + '\\nType: ' + coalesce(n.amount_type, '')",
    },
    EmbeddingTargetSpec {
        label: "TaxRule",
        id_property: "tax_rule_id",
        profile: &LEGAL_TAX_RULE_PRIMARY,
        phase: EmbeddingPhase::Specialized,
        smoke_limit: 0,
        where_clause: "coalesce(n.tax_type, n.rate_text, n.base, n.cap) IS NOT NULL",
        input_expr: "'Oregon tax rule.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nTax type: ' + coalesce(n.tax_type, '') + '\\nRate: ' + coalesce(n.rate_text, '') + '\\nBase: ' + coalesce(n.base, '') + '\\nCap: ' + coalesce(n.cap, '') + '\\nRecipient: ' + coalesce(n.recipient, '') + '\\nFund: ' + coalesce(n.fund_name, '')",
    },
    EmbeddingTargetSpec {
        label: "RateLimit",
        id_property: "rate_limit_id",
        profile: &LEGAL_RATE_LIMIT_PRIMARY,
        phase: EmbeddingPhase::Specialized,
        smoke_limit: 0,
        where_clause: "coalesce(n.rate_type, n.amount_text, n.cap_text) IS NOT NULL",
        input_expr: "'Oregon legal rate limit.\\nSource: ' + coalesce(p.display_citation, n.source_provision_id, '') + '\\nRate type: ' + coalesce(n.rate_type, '') + '\\nPercent: ' + toString(coalesce(n.percent_value, '')) + '\\nAmount: ' + coalesce(n.amount_text, '') + '\\nCap: ' + coalesce(n.cap_text, '')",
    },
    EmbeddingTargetSpec {
        label: "LegalActor",
        id_property: "actor_id",
        profile: &LEGAL_ACTOR_PRIMARY,
        phase: EmbeddingPhase::Specialized,
        smoke_limit: 0,
        where_clause: "n.name IS NOT NULL AND n.name <> ''",
        input_expr: "'Oregon legal actor.\\nName: ' + coalesce(n.name, '') + '\\nNormalized name: ' + coalesce(n.normalized_name, '') + '\\nActor type: ' + coalesce(n.actor_type, '')",
    },
    EmbeddingTargetSpec {
        label: "LegalAction",
        id_property: "action_id",
        profile: &LEGAL_ACTION_PRIMARY,
        phase: EmbeddingPhase::Specialized,
        smoke_limit: 0,
        where_clause: "coalesce(n.normalized_action, n.verb) IS NOT NULL",
        input_expr: "'Oregon legal action.\\nVerb: ' + coalesce(n.verb, '') + '\\nObject: ' + coalesce(n.object, '') + '\\nAction: ' + coalesce(n.normalized_action, '')",
    },
];

pub fn selected_targets(
    phases: &BTreeSet<EmbeddingPhase>,
    smoke: bool,
) -> Vec<&'static EmbeddingTargetSpec> {
    EMBEDDING_TARGETS
        .iter()
        .filter(|spec| phases.contains(&spec.phase))
        .filter(|spec| !smoke || spec.smoke_limit > 0)
        .collect()
}

pub async fn run_neo4j_embeddings(
    loader: &Neo4jLoader,
    voyage: &VoyageClient,
    config: EmbeddingRunConfig,
) -> Result<EmbeddingRunReport> {
    if config.create_vector_indexes {
        for spec in selected_targets(&config.phases, config.smoke) {
            loader.create_vector_index_for_profile(spec.profile).await?;
        }
    }

    let voyage_config = model_config("voyage-4-large").unwrap_or(&VOYAGE_4_LARGE);
    let context_token_limit = voyage_config.context_tokens;
    let batch_token_limit = config
        .max_batch_estimated_tokens
        .min(voyage_config.batch_token_safety_limit);
    let mut report = EmbeddingRunReport::default();

    for spec in selected_targets(&config.phases, config.smoke) {
        let label_limit = if config.smoke {
            Some(spec.smoke_limit)
        } else {
            config.max_label_nodes
        };
        info!(
            "Embedding target {} with profile {}",
            spec.label, spec.profile.name
        );
        let label_report = embed_label(
            loader,
            voyage,
            spec,
            &config,
            label_limit,
            context_token_limit,
            batch_token_limit,
        )
        .await?;
        report.request_count += label_report.request_count;
        report.voyage_total_tokens += label_report.voyage_total_tokens;
        report.labels.insert(spec.label.to_string(), label_report);
    }

    Ok(report)
}

async fn embed_label(
    loader: &Neo4jLoader,
    voyage: &VoyageClient,
    spec: &EmbeddingTargetSpec,
    config: &EmbeddingRunConfig,
    label_limit: Option<usize>,
    context_token_limit: usize,
    batch_token_limit: usize,
) -> Result<EmbeddingLabelReport> {
    let mut out = EmbeddingLabelReport::default();
    let mut offset = 0usize;
    let target_total = label_limit.unwrap_or(usize::MAX);

    while processed_for_limit(&out) < target_total {
        let candidates = loader
            .fetch_embedding_candidates(spec, config.edition_year, offset, config.scan_batch_size)
            .await?;
        if candidates.is_empty() {
            break;
        }
        offset += candidates.len();
        out.scanned += candidates.len();

        let mut safe_batch: Vec<(EmbeddingCandidate, String)> = Vec::new();
        let mut batch_chars = 0usize;
        let mut batch_tokens = 0usize;

        for candidate in candidates {
            if processed_for_limit(&out) + safe_batch.len() >= target_total {
                break;
            }

            let input_hash = calculate_embedding_input_hash(&candidate.input_text);
            if config.resume && candidate_is_current(&candidate, spec.profile, &input_hash) {
                out.skipped += 1;
                continue;
            }

            let tokens = estimate_tokens(&candidate.input_text, spec.profile.model);
            if tokens > context_token_limit {
                out.over_context += 1;
                if spec.label == "LegalTextVersion" {
                    loader
                        .run_query(
                            query("MATCH (n:LegalTextVersion {version_id: $id}) SET n.embedding_strategy = 'split_chunks_only'")
                                .param("id", candidate.id.clone()),
                        )
                        .await?;
                }
                continue;
            }

            if !safe_batch.is_empty()
                && (safe_batch.len() >= config.embedding_batch_size
                    || batch_chars + candidate.input_text.chars().count() > config.max_batch_chars
                    || batch_tokens + tokens > batch_token_limit
                    || processed_for_limit(&out) + safe_batch.len() >= target_total)
            {
                flush_batch(loader, voyage, spec, &mut safe_batch, &mut out).await?;
                batch_chars = 0;
                batch_tokens = 0;
            }

            batch_chars += candidate.input_text.chars().count();
            batch_tokens += tokens;
            safe_batch.push((candidate, input_hash));
        }

        flush_batch(loader, voyage, spec, &mut safe_batch, &mut out).await?;
    }

    Ok(out)
}

fn processed_for_limit(out: &EmbeddingLabelReport) -> usize {
    out.embedded + out.skipped + out.failed + out.over_context
}

async fn flush_batch(
    loader: &Neo4jLoader,
    voyage: &VoyageClient,
    spec: &EmbeddingTargetSpec,
    safe_batch: &mut Vec<(EmbeddingCandidate, String)>,
    out: &mut EmbeddingLabelReport,
) -> Result<()> {
    if safe_batch.is_empty() {
        return Ok(());
    }

    let texts: Vec<String> = safe_batch
        .iter()
        .map(|(candidate, _)| candidate.input_text.clone())
        .collect();
    let response = voyage
        .embed(
            texts,
            spec.profile.model,
            Some(spec.profile.output_dimension),
            Some("document"),
            Some(spec.profile.output_dtype),
        )
        .await?;

    out.request_count += 1;
    out.voyage_total_tokens += response.usage.total_tokens;

    let mut updates = Vec::new();
    for (idx, (candidate, input_hash)) in safe_batch.drain(..).enumerate() {
        let embedding = response
            .data
            .get(idx)
            .ok_or_else(|| anyhow!("Voyage response missing embedding index {idx}"))?
            .embedding
            .clone();
        if embedding.len() != spec.profile.output_dimension as usize {
            out.failed += 1;
            warn!(
                "{} {} returned {} dimensions, expected {}",
                spec.label,
                candidate.id,
                embedding.len(),
                spec.profile.output_dimension
            );
            continue;
        }
        updates.push(EmbeddingUpdate {
            chunk_id: candidate.id,
            embedding,
            embedding_model: spec.profile.model.to_string(),
            embedding_dim: spec.profile.output_dimension,
            embedding_input_hash: input_hash,
            embedding_profile: Some(spec.profile.name.to_string()),
            embedding_output_dtype: Some(spec.profile.output_dtype.to_string()),
            embedding_source_dimension: Some(spec.profile.output_dimension),
        });
    }

    let embedded_ids: Vec<String> = updates.iter().map(|u| u.chunk_id.clone()).collect();
    let update_count = updates.len();
    loader
        .update_node_embeddings(spec.label, spec.id_property, updates)
        .await?;
    out.embedded += update_count;

    if spec.label == "LegalTextVersion" && !embedded_ids.is_empty() {
        loader
            .run_query(
                query("UNWIND $ids AS id MATCH (n:LegalTextVersion {version_id: id}) SET n.embedding_strategy = 'full_text'")
                    .param("ids", embedded_ids),
            )
            .await?;
    }

    Ok(())
}

pub fn calculate_embedding_input_hash(text: &str) -> String {
    sha256_hex(text.as_bytes())
}

pub fn candidate_is_current(
    candidate: &EmbeddingCandidate,
    profile: &EmbeddingProfile,
    current_hash: &str,
) -> bool {
    candidate.has_embedding
        && candidate.embedding_profile.as_deref() == Some(profile.name)
        && candidate.embedding_model.as_deref() == Some(profile.model)
        && candidate.embedding_dim == Some(profile.output_dimension)
        && candidate.embedding_output_dtype.as_deref() == Some(profile.output_dtype)
        && candidate.embedding_input_hash.as_deref() == Some(current_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_primary_profiles_are_registered() {
        for profile in PRIMARY_PROFILES {
            assert_eq!(
                get_embedding_profile(profile.name).unwrap().name,
                profile.name
            );
            assert_eq!(profile.output_dimension, 1024);
            assert_eq!(profile.output_dtype, "float");
        }
    }

    #[test]
    fn resume_predicate_requires_every_metadata_field() {
        let hash = calculate_embedding_input_hash("abc");
        let mut candidate = EmbeddingCandidate {
            id: "x".to_string(),
            input_text: "abc".to_string(),
            has_embedding: true,
            embedding_profile: Some("legal_chunk_primary_v1".to_string()),
            embedding_model: Some("voyage-4-large".to_string()),
            embedding_dim: Some(1024),
            embedding_output_dtype: Some("float".to_string()),
            embedding_input_hash: Some(hash.clone()),
        };
        assert!(candidate_is_current(
            &candidate,
            &LEGAL_CHUNK_PRIMARY,
            &hash
        ));
        candidate.embedding_profile = Some("other".to_string());
        assert!(!candidate_is_current(
            &candidate,
            &LEGAL_CHUNK_PRIMARY,
            &hash
        ));
    }

    #[test]
    fn generic_semantic_spec_excludes_specialized_labels() {
        let spec = EMBEDDING_TARGETS
            .iter()
            .find(|spec| spec.label == "LegalSemanticNode")
            .unwrap();
        assert!(spec.where_clause.contains("NOT n:Obligation"));
        assert!(spec.where_clause.contains("NOT n:RequiredNotice"));
    }

    #[test]
    fn smoke_targets_match_requested_counts() {
        let phases = BTreeSet::from([
            EmbeddingPhase::Chunks,
            EmbeddingPhase::Authority,
            EmbeddingPhase::Semantic,
            EmbeddingPhase::DefinitionsHistory,
            EmbeddingPhase::Specialized,
        ]);
        let counts: BTreeMap<_, _> = selected_targets(&phases, true)
            .into_iter()
            .map(|spec| (spec.label, spec.smoke_limit))
            .collect();
        assert_eq!(counts["RetrievalChunk"], 25);
        assert_eq!(counts["Provision"], 25);
        assert_eq!(counts["LegalTextVersion"], 25);
        assert_eq!(counts["Definition"], 25);
        assert_eq!(counts["Obligation"], 25);
        assert_eq!(counts["Deadline"], 25);
        assert_eq!(counts["Penalty"], 25);
        assert_eq!(counts["SourceNote"], 25);
        assert_eq!(counts["TemporalEffect"], 10);
        assert_eq!(counts["SessionLaw"], 10);
        assert_eq!(counts["RequiredNotice"], 10);
        assert_eq!(counts["FormText"], 10);
    }
}
