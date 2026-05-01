use crate::models::{self, TimeInterval};
use crate::semantic::{
    derive_historical_nodes, derive_note_semantics, derive_provision_temporal_effects,
    derive_semantic_nodes, derive_session_laws_from_amendments, derive_source_note_status_events,
};
use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct GraphBatch {
    pub files: BTreeMap<String, Vec<Value>>,
}

impl GraphBatch {
    pub fn is_empty(&self) -> bool {
        self.files.values().all(Vec::is_empty)
    }

    pub fn push<T: Serialize>(&mut self, file_name: impl Into<String>, row: &T) -> Result<()> {
        self.files
            .entry(file_name.into())
            .or_default()
            .push(serde_json::to_value(row)?);
        Ok(())
    }

    pub fn extend<T: Serialize>(&mut self, file_name: impl Into<String>, rows: &[T]) -> Result<()> {
        let file_name = file_name.into();
        for row in rows {
            self.push(file_name.clone(), row)?;
        }
        Ok(())
    }

    pub fn extend_parsed_chapter(&mut self, parsed: &models::ParsedChapter) -> Result<()> {
        self.push("source_documents.jsonl", &parsed.source_document)?;
        self.extend("legal_text_identities.jsonl", &parsed.identities)?;
        self.extend("legal_text_versions.jsonl", &parsed.versions)?;
        self.extend("provisions.jsonl", &parsed.provisions)?;
        self.extend("citation_mentions.jsonl", &parsed.citations)?;
        self.extend("retrieval_chunks.jsonl", &parsed.chunks)?;
        self.extend("chapter_headings.jsonl", &parsed.headings)?;
        self.extend("html_paragraphs.debug.jsonl", &parsed.html_paragraphs_debug)?;
        self.extend("chapter_front_matter.jsonl", &parsed.chapter_front_matter)?;
        self.extend("title_chapter_entries.jsonl", &parsed.title_chapter_entries)?;
        self.extend("source_notes.jsonl", &parsed.source_notes)?;
        self.extend("chapter_toc_entries.jsonl", &parsed.chapter_toc_entries)?;
        self.extend("reserved_ranges.jsonl", &parsed.reserved_ranges)?;
        self.extend("parser_diagnostics.jsonl", &parsed.parser_diagnostic_rows)?;
        self.extend_derived_nodes(parsed)?;
        Ok(())
    }

    pub fn write_to_dir(&self, dir: impl AsRef<Path>) -> Result<()> {
        fs::create_dir_all(dir.as_ref())?;
        for (file_name, rows) in &self.files {
            let path = dir.as_ref().join(file_name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let file = fs::File::create(path)?;
            let mut writer = std::io::BufWriter::new(file);
            for row in rows {
                writer.write_all(serde_json::to_string(row)?.as_bytes())?;
                writer.write_all(b"\n")?;
            }
            writer.flush()?;
        }
        Ok(())
    }

    pub fn row_count(&self) -> usize {
        self.files.values().map(Vec::len).sum()
    }

    fn extend_derived_nodes(&mut self, parsed: &models::ParsedChapter) -> Result<()> {
        let historical = derive_historical_nodes(&parsed.versions, &parsed.source_document);
        let note_semantics = derive_note_semantics(
            &parsed.source_notes,
            &parsed.source_document,
            parsed.edition_year,
        );
        let mut temporal_effects = note_semantics.temporal_effects;
        temporal_effects.extend(derive_provision_temporal_effects(&parsed.provisions));
        dedupe_by(&mut temporal_effects, |row| row.temporal_effect_id.clone());
        let mut status_events = historical.status_events;
        status_events.extend(derive_source_note_status_events(
            &parsed.source_notes,
            &parsed.source_document,
            parsed.edition_year,
        ));
        let semantic = derive_semantic_nodes(&parsed.provisions);
        let mut session_laws = note_semantics.session_laws;
        session_laws.extend(derive_session_laws_from_amendments(
            &parsed.amendments,
            &parsed.source_document,
        ));
        dedupe_by(&mut session_laws, |row| row.session_law_id.clone());

        self.extend("status_events.jsonl", &status_events)?;
        self.extend("temporal_effects.jsonl", &temporal_effects)?;
        self.extend("lineage_events.jsonl", &note_semantics.lineage_events)?;
        self.extend("amendments.jsonl", &parsed.amendments)?;
        self.extend("session_laws.jsonl", &session_laws)?;
        self.extend("time_intervals.jsonl", &note_semantics.time_intervals)?;
        self.extend("defined_terms.jsonl", &semantic.defined_terms)?;
        self.extend("definition_scopes.jsonl", &semantic.definition_scopes)?;
        self.extend("definitions.jsonl", &semantic.definitions)?;
        self.extend("legal_semantic_nodes.jsonl", &semantic.legal_semantic_nodes)?;
        self.extend("legal_actors.jsonl", &semantic.legal_actors)?;
        self.extend("legal_actions.jsonl", &semantic.legal_actions)?;
        self.extend("obligations.jsonl", &semantic.obligations)?;
        self.extend("exceptions.jsonl", &semantic.exceptions)?;
        self.extend("deadlines.jsonl", &semantic.deadlines)?;
        self.extend("penalties.jsonl", &semantic.penalties)?;
        self.extend("remedies.jsonl", &semantic.remedies)?;
        self.extend("money_amounts.jsonl", &semantic.money_amounts)?;
        self.extend("tax_rules.jsonl", &semantic.tax_rules)?;
        self.extend("rate_limits.jsonl", &semantic.rate_limits)?;
        self.extend("required_notices.jsonl", &semantic.required_notices)?;
        self.extend("form_texts.jsonl", &semantic.form_texts)?;
        self.extend("time_intervals.jsonl", &Vec::<TimeInterval>::new())?;
        Ok(())
    }
}

pub fn label_file_name(label: &str) -> String {
    match label {
        "LegalCorpus" => return "legal_corpora.jsonl".to_string(),
        "LegalTextIdentity" => return "legal_text_identities.jsonl".to_string(),
        "LegalTextVersion" => return "legal_text_versions.jsonl".to_string(),
        "SourceDocument" => return "source_documents.jsonl".to_string(),
        "SourcePage" => return "source_pages.jsonl".to_string(),
        "CitationMention" => return "citation_mentions.jsonl".to_string(),
        "RetrievalChunk" => return "retrieval_chunks.jsonl".to_string(),
        "BusinessEntity" => return "business_entities.jsonl".to_string(),
        "RegisteredAgent" => return "registered_agents.jsonl".to_string(),
        "Agency" => return "agencies.jsonl".to_string(),
        "Opinion" => return "opinions.jsonl".to_string(),
        "CourtCase" => return "court_cases.jsonl".to_string(),
        "VoteEvent" => return "vote_events.jsonl".to_string(),
        "VoteRecord" => return "vote_records.jsonl".to_string(),
        "DatasetRecord" => return "dataset_records.jsonl".to_string(),
        _ => {}
    }
    let mut out = String::new();
    for (index, ch) in label.chars().enumerate() {
        if ch.is_ascii_uppercase() && index > 0 {
            out.push('_');
        }
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '_' || ch == '-' || ch.is_whitespace() {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let singular = out.trim_matches('_');
    let plural = if singular.ends_with('y') {
        format!("{}ies", singular.trim_end_matches('y'))
    } else if singular.ends_with('s') {
        format!("{singular}es")
    } else {
        format!("{singular}s")
    };
    format!("{plural}.jsonl")
}

fn dedupe_by<T, F>(rows: &mut Vec<T>, mut key: F)
where
    F: FnMut(&T) -> String,
{
    let mut seen = BTreeSet::new();
    rows.retain(|row| seen.insert(key(row)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_file_names_are_snake_case_plural() {
        assert_eq!(
            label_file_name("LegislativeSession"),
            "legislative_sessions.jsonl"
        );
        assert_eq!(label_file_name("CourtCase"), "court_cases.jsonl");
        assert_eq!(label_file_name("Agency"), "agencies.jsonl");
        assert_eq!(label_file_name("LegalCorpus"), "legal_corpora.jsonl");
    }
}
