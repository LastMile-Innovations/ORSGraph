use crate::artifact_store::{ArtifactMetadata, RawArtifact};
use crate::connectors::{ConnectorOptions, DataConnector, SourceItem};
use crate::graph_batch::GraphBatch;
use crate::hash::{sha256_hex, stable_id};
use crate::models::{
    LegalActor, LineageEvent, ParserDiagnostic, SessionLaw, SourceDocument, StatusEvent,
};
use crate::source_qc::{qc_source_batch, QcReport, QcReportStatus};
use crate::source_registry::{SourceKind, SourceRegistryEntry};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

const SOURCE_ID: &str = "or_leg_odata";
const PARSER_PROFILE: &str = "oregon_leg_odata_connector_v1";
static ENTITY_SET_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<EntitySet\s+Name="([^"]+)"\s+EntityType="([^"]+)""#)
        .expect("valid OData EntitySet regex")
});
static PROPERTY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<Property\s+Name="([^"]+)"\s+Type="([^"]+)""#)
        .expect("valid OData Property regex")
});

const SESSION_ENTITY_SETS: &[(&str, Option<&str>, Option<&str>)] = &[
    (
        "Measures",
        Some("SessionKey,MeasurePrefix,MeasureNumber,CatchLine,MeasureSummary,ChapterNumber,CurrentLocation,CurrentCommitteeCode,EffectiveDate,EmergencyClause,Vetoed,CreatedDate,ModifiedDate"),
        None,
    ),
    ("MeasureDocuments", None, None),
    ("MeasureAnalysisDocuments", None, None),
    ("MeasureHistoryActions", None, Some("ActionDate")),
    ("MeasureSponsors", None, None),
    ("Committees", None, None),
    ("Legislators", None, None),
    ("CommitteeMeetings", None, Some("MeetingDate")),
    ("MeasureVotes", None, None),
    ("CommitteeVotes", None, None),
];

pub struct OregonLegODataConnector {
    entry: SourceRegistryEntry,
    options: ConnectorOptions,
}

impl OregonLegODataConnector {
    pub fn new(entry: SourceRegistryEntry, options: ConnectorOptions) -> Self {
        Self { entry, options }
    }

    fn base_url(&self) -> String {
        self.entry.source_url.trim_end_matches('/').to_string()
    }

    fn session_key(&self) -> String {
        if let Some(value) = self
            .options
            .session_key
            .as_deref()
            .or(self.options.chapters.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            value.to_string()
        } else {
            format!("{}R1", self.options.edition_year)
        }
    }

    fn source_document_for_artifact(&self, artifact: &RawArtifact) -> SourceDocument {
        SourceDocument {
            source_document_id: raw_response_document_id(&artifact.metadata),
            source_provider: self.entry.owner.clone(),
            source_kind: self.entry.source_type.as_str().to_string(),
            url: artifact.metadata.url.clone(),
            chapter: artifact.metadata.item_id.clone(),
            corpus_id: None,
            edition_id: None,
            authority_family: Some("Oregon Legislature".to_string()),
            authority_type: Some("legislative_api".to_string()),
            title: Some(format!("{} {}", self.entry.name, artifact.metadata.item_id)),
            source_type: Some("odata_response".to_string()),
            file_name: artifact
                .metadata
                .path
                .rsplit('/')
                .next()
                .map(ToOwned::to_owned),
            page_count: Some(1),
            effective_date: None,
            copyright_status: Some(self.entry.access.as_str().to_string()),
            chapter_title: Some(artifact.metadata.item_id.clone()),
            edition_year: self.options.edition_year,
            html_encoding: Some("utf-8".to_string()),
            source_path: Some(artifact.metadata.path.clone()),
            paragraph_count: Some(count_nonempty_lines(&artifact.bytes)),
            first_body_paragraph_index: Some(0),
            parser_profile: Some(PARSER_PROFILE.to_string()),
            official_status: self.entry.official_status.as_str().to_string(),
            disclaimer_required: false,
            raw_hash: artifact.metadata.raw_hash.clone(),
            normalized_hash: artifact.metadata.raw_hash.clone(),
        }
    }

    fn diagnostic(
        &self,
        source_document_id: &str,
        severity: &str,
        diagnostic_type: &str,
        message: impl Into<String>,
        related_id: Option<String>,
    ) -> ParserDiagnostic {
        let message = message.into();
        ParserDiagnostic {
            parser_diagnostic_id: format!(
                "diag:{}:{}",
                SOURCE_ID,
                stable_id(&format!(
                    "{source_document_id}:{severity}:{diagnostic_type}:{}",
                    message
                ))
            ),
            source_document_id: source_document_id.to_string(),
            chapter: SOURCE_ID.to_string(),
            edition_year: self.options.edition_year,
            severity: severity.to_string(),
            diagnostic_type: diagnostic_type.to_string(),
            message,
            source_paragraph_order: None,
            related_id,
            parser_profile: PARSER_PROFILE.to_string(),
        }
    }

    fn push_diagnostic(
        &self,
        batch: &mut GraphBatch,
        source_document_id: &str,
        severity: &str,
        diagnostic_type: &str,
        message: impl Into<String>,
        related_id: Option<String>,
    ) -> Result<()> {
        let diagnostic = self.diagnostic(
            source_document_id,
            severity,
            diagnostic_type,
            message.into(),
            related_id,
        );
        batch.push("parser_diagnostics.jsonl", &diagnostic)
    }

    fn parse_metadata(&self, artifact: &RawArtifact, batch: &mut GraphBatch) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        let text = String::from_utf8_lossy(&artifact.bytes);

        let mut entity_sets = Vec::new();
        for captures in ENTITY_SET_RE.captures_iter(&text) {
            let name = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
            let entity_type = captures.get(2).map(|m| m.as_str()).unwrap_or_default();
            entity_sets.push(name.to_string());
            let row = json!({
                "odata_entity_set_id": format!("orleg:odata-entity-set:{}", clean_id_part(name)),
                "name": name,
                "entity_type": entity_type,
                "source_id": SOURCE_ID,
                "source_document_id": source_document_id,
                "parser_profile": PARSER_PROFILE,
                "raw_hash": artifact.metadata.raw_hash,
            });
            batch.push("odata_entity_sets.jsonl", &row)?;
        }

        let properties = PROPERTY_RE
            .captures_iter(&text)
            .filter_map(|captures| {
                Some(json!({
                    "name": captures.get(1)?.as_str(),
                    "type": captures.get(2)?.as_str(),
                }))
            })
            .collect::<Vec<_>>();
        batch.push(
            "odata_metadata_summary.jsonl",
            &json!({
                "metadata_summary_id": format!("orleg:odata-metadata:{}", artifact.metadata.raw_hash),
                "source_id": SOURCE_ID,
                "source_document_id": source_document_id,
                "entity_set_count": entity_sets.len(),
                "property_count": properties.len(),
                "entity_sets": entity_sets,
                "properties": properties,
                "parser_profile": PARSER_PROFILE,
                "raw_hash": artifact.metadata.raw_hash,
            }),
        )?;

        if batch
            .files
            .get("odata_entity_sets.jsonl")
            .map(Vec::is_empty)
            .unwrap_or(true)
        {
            self.push_diagnostic(
                batch,
                &source_document_id,
                "warning",
                "metadata_entity_sets_missing",
                "No EntitySet declarations were found in the OData metadata artifact.",
                None,
            )?;
        }
        Ok(())
    }

    fn parse_rows_artifact(
        &self,
        artifact: &RawArtifact,
        entity_set: &str,
        batch: &mut GraphBatch,
    ) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        let value: Value = serde_json::from_slice(&artifact.bytes).with_context(|| {
            format!(
                "failed to parse OData JSON artifact {}",
                artifact.metadata.item_id
            )
        })?;
        let rows = extract_odata_rows(&value, entity_set);
        let next_link_present = has_next_link(&value);
        batch.push(
            "odata_entity_set_stats.jsonl",
            &json!({
                "odata_entity_set_stats_id": format!("orleg:odata-entity-set-stats:{}:{}", entity_set, artifact.metadata.raw_hash),
                "source_id": SOURCE_ID,
                "entity_set": entity_set,
                "item_id": artifact.metadata.item_id,
                "source_document_id": source_document_id,
                "row_count": rows.len(),
                "next_link_present": next_link_present,
                "byte_len": artifact.metadata.byte_len,
                "raw_hash": artifact.metadata.raw_hash,
                "parser_profile": PARSER_PROFILE,
            }),
        )?;
        if rows.is_empty() {
            self.push_diagnostic(
                batch,
                &source_document_id,
                "warning",
                "odata_empty_entity_set",
                format!("{entity_set} artifact contained no rows"),
                Some(artifact.metadata.item_id.clone()),
            )?;
        }
        if next_link_present {
            self.push_diagnostic(
                batch,
                &source_document_id,
                "warning",
                "odata_paging_next_link_present",
                format!(
                    "{entity_set} response contains an OData next link; this run preserved the first page and should be resumed with paging support if row counts look truncated."
                ),
                Some(artifact.metadata.item_id.clone()),
            )?;
        }

        match entity_set {
            "LegislativeSessions" => self.parse_legislative_sessions(artifact, &rows, batch),
            "Measures" => self.parse_measures(artifact, &rows, batch),
            "MeasureDocuments" | "MeasureAnalysisDocuments" => {
                self.parse_measure_documents(artifact, entity_set, &rows, batch)
            }
            "MeasureHistoryActions" => self.parse_measure_history_actions(artifact, &rows, batch),
            "MeasureSponsors" => self.parse_measure_sponsors(artifact, &rows, batch),
            "Committees" => self.parse_committees(artifact, &rows, batch),
            "Legislators" => self.parse_legislators(artifact, &rows, batch),
            "CommitteeMeetings" => self.parse_committee_meetings(artifact, &rows, batch),
            "MeasureVotes" | "CommitteeVotes" => {
                self.parse_votes(artifact, entity_set, &rows, batch)
            }
            other => {
                self.push_diagnostic(
                    batch,
                    &source_document_id,
                    "warning",
                    "odata_entity_set_not_mapped",
                    format!("{other} has no source-specific parser yet"),
                    Some(artifact.metadata.item_id.clone()),
                )?;
                Ok(())
            }
        }
    }

    fn parse_legislative_sessions(
        &self,
        artifact: &RawArtifact,
        rows: &[&Value],
        batch: &mut GraphBatch,
    ) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        for row in rows.iter().copied() {
            let Some(session_key) = value_string(row, &["SessionKey", "session_key"]) else {
                self.push_diagnostic(
                    batch,
                    &source_document_id,
                    "error",
                    "session_missing_key",
                    "LegislativeSession row is missing SessionKey.",
                    None,
                )?;
                continue;
            };
            let session_id = session_id(&session_key);
            batch.push(
                "legislative_sessions.jsonl",
                &json!({
                    "legislative_session_id": session_id,
                    "session_key": session_key,
                    "name": value_string(row, &["SessionName", "Name", "session_name"]),
                    "begin_date": value_date(row, &["BeginDate", "begin_date"]),
                    "end_date": value_date(row, &["EndDate", "end_date"]),
                    "default_session": value_bool(row, &["DefaultSession", "default_session"]),
                    "source_id": SOURCE_ID,
                    "source_document_id": source_document_id,
                    "jurisdiction_id": self.entry.jurisdiction,
                    "official_status": self.entry.official_status.as_str(),
                    "parser_profile": PARSER_PROFILE,
                    "raw_hash": artifact.metadata.raw_hash,
                }),
            )?;
        }
        Ok(())
    }

    fn parse_measures(
        &self,
        artifact: &RawArtifact,
        rows: &[&Value],
        batch: &mut GraphBatch,
    ) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        for row in rows.iter().copied() {
            let measure_key = match measure_key(row) {
                Ok(key) => key,
                Err(error) => {
                    self.push_diagnostic(
                        batch,
                        &source_document_id,
                        "error",
                        "measure_missing_composite_key",
                        error.to_string(),
                        None,
                    )?;
                    continue;
                }
            };
            let measure_id = measure_key.measure_id();
            let bill_number = measure_key.bill_number();
            let effective_date = value_date(row, &["EffectiveDate", "effective_date"]);
            let created_date = value_date(row, &["CreatedDate", "created_date"]);
            let modified_date = value_date(row, &["ModifiedDate", "modified_date"]);
            let chapter_number = value_string(row, &["ChapterNumber", "chapter_number"]);
            let catch_line = value_string(row, &["CatchLine", "catch_line"]);
            let summary = value_string(row, &["MeasureSummary", "Summary", "measure_summary"]);

            batch.push(
                "legislative_measures.jsonl",
                &json!({
                    "legislative_measure_id": measure_id,
                    "measure_id": measure_id,
                    "session_key": measure_key.session_key,
                    "measure_prefix": measure_key.measure_prefix,
                    "measure_number": measure_key.measure_number,
                    "bill_number": bill_number,
                    "catch_line": catch_line,
                    "measure_summary": summary,
                    "chapter_number": chapter_number,
                    "current_location": value_string(row, &["CurrentLocation", "current_location"]),
                    "current_committee_code": value_string(row, &["CurrentCommitteeCode", "current_committee_code"]),
                    "effective_date": effective_date,
                    "emergency_clause": value_bool(row, &["EmergencyClause", "emergency_clause"]),
                    "vetoed": value_bool(row, &["Vetoed", "vetoed"]),
                    "created_date": created_date,
                    "modified_date": modified_date,
                    "source_id": SOURCE_ID,
                    "source_document_id": source_document_id,
                    "jurisdiction_id": self.entry.jurisdiction,
                    "official_status": self.entry.official_status.as_str(),
                    "parser_profile": PARSER_PROFILE,
                    "raw_hash": artifact.metadata.raw_hash,
                }),
            )?;
            batch.push(
                "legislative_edges.jsonl",
                &edge_row(
                    &measure_id,
                    &session_id(&measure_key.session_key),
                    "IN_SESSION",
                    &source_document_id,
                ),
            )?;

            if let Some(chapter) = chapter_number.filter(|value| !value.trim().is_empty()) {
                let year = effective_date
                    .as_deref()
                    .or(created_date.as_deref())
                    .and_then(date_year)
                    .unwrap_or(self.options.edition_year);
                let session_law_id = format!("or:laws:{year}:c:{}", clean_number(&chapter));
                let law = SessionLaw {
                    session_law_id: session_law_id.clone(),
                    jurisdiction_id: Some(self.entry.jurisdiction.clone()),
                    citation: format!("{year} c.{}", clean_number(&chapter)),
                    year,
                    chapter: Some(clean_number(&chapter)),
                    section: None,
                    bill_number: Some(bill_number.clone()),
                    effective_date: effective_date.clone(),
                    text: summary.clone().or(catch_line.clone()),
                    raw_text: summary.clone().or(catch_line.clone()),
                    source_document_id: Some(source_document_id.clone()),
                    source_note_id: None,
                    confidence: 0.82,
                };
                batch.push("session_laws.jsonl", &law)?;
                batch.push(
                    "legislative_edges.jsonl",
                    &edge_row(
                        &session_law_id,
                        &measure_id,
                        "ENACTED_BY",
                        &source_document_id,
                    ),
                )?;
            }
        }
        Ok(())
    }

    fn parse_measure_documents(
        &self,
        artifact: &RawArtifact,
        entity_set: &str,
        rows: &[&Value],
        batch: &mut GraphBatch,
    ) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        let base_url = self.base_url();
        for row in rows.iter().copied() {
            let measure_key = match measure_key(row) {
                Ok(key) => Some(key),
                Err(error) => {
                    self.push_diagnostic(
                        batch,
                        &source_document_id,
                        "warning",
                        "measure_document_missing_measure_key",
                        error.to_string(),
                        None,
                    )?;
                    None
                }
            };
            let version = value_string(
                row,
                &[
                    "VersionDescription",
                    "DocumentType",
                    "MeasureDocumentType",
                    "Version",
                    "version_description",
                ],
            )
            .unwrap_or_else(|| entity_set.to_string());
            let url = value_string(
                row,
                &[
                    "Url",
                    "URL",
                    "DocumentUrl",
                    "DocumentURL",
                    "PdfUrl",
                    "PDFUrl",
                    "PDFURL",
                    "FileUrl",
                    "FileURL",
                    "Link",
                ],
            );
            let absolute_url = url.as_deref().and_then(|url| absolute_url(url, &base_url));
            let document_id = if let Some(key) = &measure_key {
                format!(
                    "orleg:measure-document:{}:{}:{}:{}",
                    key.session_key,
                    key.measure_prefix,
                    key.measure_number,
                    clean_id_part(&version)
                )
            } else {
                format!("orleg:measure-document:{}", row_stable_id(row))
            };
            let measure_id = measure_key.as_ref().map(MeasureKey::measure_id);
            let title =
                value_string(row, &["Title", "DocumentTitle", "Description"]).or_else(|| {
                    measure_key
                        .as_ref()
                        .map(|key| format!("{} {}", key.bill_number(), version))
                });
            let document_date = value_date(row, &["DocumentDate", "CreatedDate", "ModifiedDate"]);
            let source_document_node_id = absolute_url
                .as_ref()
                .map(|url| format!("src:{}:{}", SOURCE_ID, stable_id(url)));

            batch.push(
                "legislative_measure_documents.jsonl",
                &json!({
                    "measure_document_id": document_id,
                    "measure_id": measure_id,
                    "session_key": measure_key.as_ref().map(|key| key.session_key.clone()),
                    "measure_prefix": measure_key.as_ref().map(|key| key.measure_prefix.clone()),
                    "measure_number": measure_key.as_ref().map(|key| key.measure_number.clone()),
                    "version_description": version,
                    "document_kind": entity_set,
                    "title": title,
                    "url": absolute_url,
                    "source_document_node_id": source_document_node_id,
                    "document_date": document_date,
                    "source_id": SOURCE_ID,
                    "source_document_id": source_document_id,
                    "official_status": self.entry.official_status.as_str(),
                    "parser_profile": PARSER_PROFILE,
                    "raw_hash": artifact.metadata.raw_hash,
                }),
            )?;

            if let Some(key) = &measure_key {
                batch.push(
                    "legislative_measure_versions.jsonl",
                    &json!({
                        "measure_version_id": format!("orleg:measure-version:{}:{}:{}:{}", key.session_key, key.measure_prefix, key.measure_number, clean_id_part(&version)),
                        "measure_id": key.measure_id(),
                        "session_key": key.session_key,
                        "measure_prefix": key.measure_prefix,
                        "measure_number": key.measure_number,
                        "version_description": version,
                        "source_document_id": source_document_id,
                        "parser_profile": PARSER_PROFILE,
                    }),
                )?;
                batch.push(
                    "legislative_edges.jsonl",
                    &edge_row(
                        &key.measure_id(),
                        &document_id,
                        "HAS_DOCUMENT",
                        &source_document_id,
                    ),
                )?;
            }

            match absolute_url {
                Some(url) => {
                    let source_document = SourceDocument {
                        source_document_id: source_document_node_id
                            .unwrap_or_else(|| format!("src:{}:{}", SOURCE_ID, stable_id(&url))),
                        source_provider: self.entry.owner.clone(),
                        source_kind: "legislative_document".to_string(),
                        url: url.clone(),
                        chapter: measure_key
                            .as_ref()
                            .map(MeasureKey::bill_number)
                            .unwrap_or_else(|| entity_set.to_string()),
                        corpus_id: None,
                        edition_id: None,
                        authority_family: Some("Oregon Legislature".to_string()),
                        authority_type: Some(entity_set.to_string()),
                        title,
                        source_type: Some(entity_set.to_string()),
                        file_name: file_name_from_url(&url),
                        page_count: None,
                        effective_date: document_date,
                        copyright_status: Some(self.entry.access.as_str().to_string()),
                        chapter_title: measure_key.as_ref().map(MeasureKey::bill_number),
                        edition_year: self.options.edition_year,
                        html_encoding: None,
                        source_path: None,
                        paragraph_count: None,
                        first_body_paragraph_index: None,
                        parser_profile: Some(PARSER_PROFILE.to_string()),
                        official_status: self.entry.official_status.as_str().to_string(),
                        disclaimer_required: false,
                        raw_hash: artifact.metadata.raw_hash.clone(),
                        normalized_hash: sha256_hex(url.as_bytes()),
                    };
                    batch.push("source_documents.jsonl", &source_document)?;
                }
                None => {
                    if let Some(raw_url) = url {
                        self.push_diagnostic(
                            batch,
                            &source_document_id,
                            "warning",
                            "measure_document_invalid_url",
                            format!("Measure document URL is not absolute or joinable: {raw_url}"),
                            Some(document_id),
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_measure_history_actions(
        &self,
        artifact: &RawArtifact,
        rows: &[&Value],
        batch: &mut GraphBatch,
    ) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        for row in rows.iter().copied() {
            let action_key = value_string(
                row,
                &["MeasureHistoryId", "MeasureHistoryActionId", "HistoryId"],
            )
            .unwrap_or_else(|| row_stable_id(row));
            let measure_key = match measure_key(row) {
                Ok(key) => Some(key),
                Err(error) => {
                    self.push_diagnostic(
                        batch,
                        &source_document_id,
                        "warning",
                        "history_action_missing_measure_key",
                        error.to_string(),
                        Some(action_key.clone()),
                    )?;
                    None
                }
            };
            let action_id = if let Some(key) = &measure_key {
                format!(
                    "orleg:history-action:{}:{}",
                    key.session_key,
                    clean_id_part(&action_key)
                )
            } else {
                format!("orleg:history-action:{}", clean_id_part(&action_key))
            };
            let action_text = value_string(
                row,
                &[
                    "ActionText",
                    "Action",
                    "Description",
                    "MeasureHistoryAction",
                    "Text",
                ],
            );
            let action_date = value_date(row, &["ActionDate", "CreatedDate", "ModifiedDate"]);
            let measure_id = measure_key.as_ref().map(MeasureKey::measure_id);
            batch.push(
                "legislative_measure_history_actions.jsonl",
                &json!({
                    "measure_history_action_id": action_id,
                    "measure_id": measure_id,
                    "session_key": measure_key.as_ref().map(|key| key.session_key.clone()),
                    "measure_prefix": measure_key.as_ref().map(|key| key.measure_prefix.clone()),
                    "measure_number": measure_key.as_ref().map(|key| key.measure_number.clone()),
                    "action_date": action_date,
                    "action_text": action_text,
                    "location": value_string(row, &["Location", "CurrentLocation", "Chamber"]),
                    "source_id": SOURCE_ID,
                    "source_document_id": source_document_id,
                    "official_status": self.entry.official_status.as_str(),
                    "parser_profile": PARSER_PROFILE,
                    "raw_hash": artifact.metadata.raw_hash,
                }),
            )?;
            if let Some(measure_id) = measure_id {
                batch.push(
                    "status_events.jsonl",
                    &StatusEvent {
                        status_event_id: format!("status:{}", stable_id(&action_id)),
                        status_type: "legislative_history_action".to_string(),
                        status_text: action_text.clone(),
                        source_document_id: Some(source_document_id.clone()),
                        canonical_id: measure_id.clone(),
                        version_id: None,
                        event_year: action_date.as_deref().and_then(date_year),
                        effective_date: action_date.clone(),
                        source_note_id: None,
                        effect_type: Some("history_action".to_string()),
                        trigger_text: action_text.clone(),
                        operative_date: None,
                        repeal_date: None,
                        session_law_ref: None,
                        confidence: 0.86,
                        extraction_method: PARSER_PROFILE.to_string(),
                    },
                )?;
                if let Some(text) = action_text {
                    batch.push(
                        "lineage_events.jsonl",
                        &LineageEvent {
                            lineage_event_id: format!("lineage:{}", stable_id(&action_id)),
                            source_note_id: None,
                            from_canonical_id: None,
                            to_canonical_id: None,
                            current_canonical_id: measure_id.clone(),
                            lineage_type: "legislative_history_action".to_string(),
                            raw_text: text,
                            year: action_date.as_deref().and_then(date_year),
                            confidence: 0.72,
                        },
                    )?;
                }
                batch.push(
                    "legislative_edges.jsonl",
                    &edge_row(
                        &measure_id,
                        &action_id,
                        "HAS_HISTORY_ACTION",
                        &source_document_id,
                    ),
                )?;
            }
        }
        Ok(())
    }

    fn parse_measure_sponsors(
        &self,
        artifact: &RawArtifact,
        rows: &[&Value],
        batch: &mut GraphBatch,
    ) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        for row in rows.iter().copied() {
            let sponsor_key = value_string(row, &["MeasureSponsorId", "SponsorId"])
                .unwrap_or_else(|| row_stable_id(row));
            let measure_key = match measure_key(row) {
                Ok(key) => Some(key),
                Err(error) => {
                    self.push_diagnostic(
                        batch,
                        &source_document_id,
                        "warning",
                        "sponsor_missing_measure_key",
                        error.to_string(),
                        Some(sponsor_key.clone()),
                    )?;
                    None
                }
            };
            let session_key = measure_key
                .as_ref()
                .map(|key| key.session_key.clone())
                .or_else(|| value_string(row, &["SessionKey", "session_key"]))
                .unwrap_or_else(|| self.session_key());
            let legislator_code = value_string(
                row,
                &[
                    "LegislatorCode",
                    "LegislatoreCode",
                    "SponsorCode",
                    "MemberCode",
                ],
            );
            let committee_code = value_string(row, &["CommitteeCode", "CommitteCode"]);
            let actor_name = value_string(row, &["SponsorName", "Name", "FullName"])
                .or_else(|| legislator_code.clone())
                .or_else(|| committee_code.clone());
            let actor_id = match (
                legislator_code.as_deref(),
                committee_code.as_deref(),
                actor_name.as_deref(),
            ) {
                (Some(code), _, _) => Some(legislator_id(&session_key, code)),
                (_, Some(code), _) => Some(committee_id(&session_key, code)),
                (_, _, Some(name)) => Some(format!("orleg:actor:{}", stable_id(name))),
                _ => None,
            };
            let sponsor_id = format!(
                "orleg:measure-sponsor:{}:{}",
                session_key,
                clean_id_part(&sponsor_key)
            );
            let measure_id = measure_key.as_ref().map(MeasureKey::measure_id);
            batch.push(
                "legislative_measure_sponsors.jsonl",
                &json!({
                    "measure_sponsor_id": sponsor_id,
                    "measure_id": measure_id,
                    "actor_id": actor_id,
                    "session_key": session_key,
                    "measure_prefix": measure_key.as_ref().map(|key| key.measure_prefix.clone()),
                    "measure_number": measure_key.as_ref().map(|key| key.measure_number.clone()),
                    "legislator_code": legislator_code,
                    "committee_code": committee_code,
                    "sponsor_name": actor_name,
                    "sponsor_type": value_string(row, &["SponsorType", "Type", "Title"]),
                    "chief_sponsor": value_bool(row, &["ChiefSponsor", "IsChiefSponsor"]),
                    "source_id": SOURCE_ID,
                    "source_document_id": source_document_id,
                    "official_status": self.entry.official_status.as_str(),
                    "parser_profile": PARSER_PROFILE,
                    "raw_hash": artifact.metadata.raw_hash,
                }),
            )?;
            if let Some(actor_id) = actor_id {
                if let Some(actor_name) = actor_name {
                    batch.push(
                        "legal_actors.jsonl",
                        &LegalActor {
                            actor_id: actor_id.clone(),
                            name: actor_name.clone(),
                            normalized_name: normalize_actor_name(&actor_name),
                            actor_type: Some("legislative_sponsor".to_string()),
                            jurisdiction_id: Some(self.entry.jurisdiction.clone()),
                        },
                    )?;
                }
                if let Some(measure_id) = measure_id {
                    batch.push(
                        "legislative_edges.jsonl",
                        &edge_row(&measure_id, &actor_id, "SPONSORED_BY", &source_document_id),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn parse_committees(
        &self,
        artifact: &RawArtifact,
        rows: &[&Value],
        batch: &mut GraphBatch,
    ) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        for row in rows.iter().copied() {
            let Some(session_key) = value_string(row, &["SessionKey", "session_key"]) else {
                self.push_diagnostic(
                    batch,
                    &source_document_id,
                    "error",
                    "committee_missing_session_key",
                    "Committee row is missing SessionKey.",
                    None,
                )?;
                continue;
            };
            let Some(code) = value_string(row, &["CommitteeCode", "CommitteCode"]) else {
                self.push_diagnostic(
                    batch,
                    &source_document_id,
                    "error",
                    "committee_missing_code",
                    "Committee row is missing CommitteeCode.",
                    Some(session_key),
                )?;
                continue;
            };
            let id = committee_id(&session_key, &code);
            let name = value_string(row, &["CommitteeName", "Name", "FullName"])
                .unwrap_or_else(|| code.clone());
            batch.push(
                "legislative_committees.jsonl",
                &json!({
                    "committee_id": id,
                    "session_key": session_key,
                    "committee_code": code,
                    "name": name,
                    "chamber": value_string(row, &["Chamber"]),
                    "committee_type": value_string(row, &["CommitteeType", "Type"]),
                    "active": value_bool(row, &["Active", "IsActive"]),
                    "source_id": SOURCE_ID,
                    "source_document_id": source_document_id,
                    "jurisdiction_id": self.entry.jurisdiction,
                    "official_status": self.entry.official_status.as_str(),
                    "parser_profile": PARSER_PROFILE,
                    "raw_hash": artifact.metadata.raw_hash,
                }),
            )?;
            batch.push(
                "legal_actors.jsonl",
                &LegalActor {
                    actor_id: id.clone(),
                    name: name.clone(),
                    normalized_name: normalize_actor_name(&name),
                    actor_type: Some("legislative_committee".to_string()),
                    jurisdiction_id: Some(self.entry.jurisdiction.clone()),
                },
            )?;
        }
        Ok(())
    }

    fn parse_legislators(
        &self,
        artifact: &RawArtifact,
        rows: &[&Value],
        batch: &mut GraphBatch,
    ) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        for row in rows.iter().copied() {
            let Some(session_key) = value_string(row, &["SessionKey", "session_key"]) else {
                self.push_diagnostic(
                    batch,
                    &source_document_id,
                    "error",
                    "legislator_missing_session_key",
                    "Legislator row is missing SessionKey.",
                    None,
                )?;
                continue;
            };
            let Some(code) = value_string(row, &["LegislatorCode", "LegislatoreCode"]) else {
                self.push_diagnostic(
                    batch,
                    &source_document_id,
                    "error",
                    "legislator_missing_code",
                    "Legislator row is missing LegislatorCode.",
                    Some(session_key),
                )?;
                continue;
            };
            let id = legislator_id(&session_key, &code);
            let name = display_name(row, &code);
            batch.push(
                "legislative_legislators.jsonl",
                &json!({
                    "legislator_id": id,
                    "session_key": session_key,
                    "legislator_code": code,
                    "name": name,
                    "first_name": value_string(row, &["FirstName", "first_name"]),
                    "last_name": value_string(row, &["LastName", "last_name"]),
                    "party": value_string(row, &["Party", "PartyCode"]),
                    "district": value_string(row, &["District", "DistrictNumber"]),
                    "chamber": value_string(row, &["Chamber"]),
                    "source_id": SOURCE_ID,
                    "source_document_id": source_document_id,
                    "jurisdiction_id": self.entry.jurisdiction,
                    "official_status": self.entry.official_status.as_str(),
                    "parser_profile": PARSER_PROFILE,
                    "raw_hash": artifact.metadata.raw_hash,
                }),
            )?;
            batch.push(
                "legal_actors.jsonl",
                &LegalActor {
                    actor_id: id,
                    name: name.clone(),
                    normalized_name: normalize_actor_name(&name),
                    actor_type: Some("legislator".to_string()),
                    jurisdiction_id: Some(self.entry.jurisdiction.clone()),
                },
            )?;
        }
        Ok(())
    }

    fn parse_committee_meetings(
        &self,
        artifact: &RawArtifact,
        rows: &[&Value],
        batch: &mut GraphBatch,
    ) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        let base_url = self.base_url();
        for row in rows.iter().copied() {
            let session_key = value_string(row, &["SessionKey", "session_key"])
                .unwrap_or_else(|| self.session_key());
            let Some(code) = value_string(row, &["CommitteeCode", "CommitteCode"]) else {
                self.push_diagnostic(
                    batch,
                    &source_document_id,
                    "warning",
                    "committee_meeting_missing_code",
                    "CommitteeMeeting row is missing CommitteeCode.",
                    Some(session_key),
                )?;
                continue;
            };
            let meeting_date = value_date(row, &["MeetingDate", "Date", "StartDate"]);
            let meeting_id = format!(
                "orleg:committee-meeting:{}:{}:{}",
                session_key,
                clean_id_part(&code),
                clean_id_part(meeting_date.as_deref().unwrap_or("unknown"))
            );
            batch.push(
                "legislative_committee_meetings.jsonl",
                &json!({
                    "committee_meeting_id": meeting_id,
                    "committee_id": committee_id(&session_key, &code),
                    "session_key": session_key,
                    "committee_code": code,
                    "meeting_date": meeting_date,
                    "location": value_string(row, &["Location", "Room"]),
                    "agenda_url": value_string(row, &["AgendaUrl", "AgendaURL", "Url"]).and_then(|url| absolute_url(&url, &base_url)),
                    "source_id": SOURCE_ID,
                    "source_document_id": source_document_id,
                    "official_status": self.entry.official_status.as_str(),
                    "parser_profile": PARSER_PROFILE,
                    "raw_hash": artifact.metadata.raw_hash,
                }),
            )?;
            batch.push(
                "legislative_edges.jsonl",
                &edge_row(
                    &committee_id(&session_key, &code),
                    &meeting_id,
                    "HELD_MEETING",
                    &source_document_id,
                ),
            )?;
        }
        Ok(())
    }

    fn parse_votes(
        &self,
        artifact: &RawArtifact,
        entity_set: &str,
        rows: &[&Value],
        batch: &mut GraphBatch,
    ) -> Result<()> {
        let source_document_id = raw_response_document_id(&artifact.metadata);
        for row in rows.iter().copied() {
            let session_key = value_string(row, &["SessionKey", "session_key"])
                .unwrap_or_else(|| self.session_key());
            let vote_key = value_string(row, &["MeasureVoteId", "CommitteeVoteId", "VoteId"])
                .unwrap_or_else(|| row_stable_id(row));
            let vote_scope = if entity_set == "MeasureVotes" {
                "measure"
            } else {
                "committee"
            };
            let vote_id = format!(
                "orleg:vote:{vote_scope}:{}:{}",
                session_key,
                clean_id_part(&vote_key)
            );
            let measure_key = measure_key(row).ok();
            let target_measure_id = measure_key.as_ref().map(MeasureKey::measure_id);
            let committee_code = value_string(row, &["CommitteeCode", "CommitteCode"]);
            let committee_id = committee_code
                .as_deref()
                .map(|code| committee_id(&session_key, code));
            let vote_date = value_date(row, &["VoteDate", "ActionDate", "CreatedDate"]);
            let vote_row = json!({
                "legislative_vote_id": vote_id,
                "vote_event_id": vote_id,
                "vote_scope": vote_scope,
                "session_key": session_key,
                "measure_id": target_measure_id,
                "committee_id": committee_id,
                "committee_code": committee_code,
                "vote_date": vote_date,
                "motion": value_string(row, &["Motion", "Description", "VoteDescription"]),
                "result": value_string(row, &["Result", "VoteResult", "Outcome"]),
                "ayes": value_i64(row, &["Ayes", "YesVotes", "YeaVotes"]),
                "nays": value_i64(row, &["Nays", "NoVotes", "NayVotes"]),
                "excused": value_i64(row, &["Excused"]),
                "absent": value_i64(row, &["Absent"]),
                "source_id": SOURCE_ID,
                "source_document_id": source_document_id,
                "official_status": self.entry.official_status.as_str(),
                "parser_profile": PARSER_PROFILE,
                "raw_hash": artifact.metadata.raw_hash,
            });
            batch.push("legislative_votes.jsonl", &vote_row)?;
            batch.push("vote_events.jsonl", &vote_row)?;

            let legislator_code =
                value_string(row, &["LegislatorCode", "LegislatoreCode", "MemberCode"]);
            let member_vote = value_string(row, &["Vote", "MemberVote", "VoteValue"]);
            if legislator_code.is_some() || member_vote.is_some() {
                let row_id = row_stable_id(row);
                let actor_id = legislator_code
                    .as_deref()
                    .map(|code| legislator_id(&session_key, code));
                batch.push(
                    "vote_records.jsonl",
                    &json!({
                        "vote_record_id": format!("orleg:vote-record:{}:{}", stable_id(&vote_id), row_id),
                        "vote_event_id": vote_id,
                        "actor_id": actor_id,
                        "legislator_code": legislator_code,
                        "member_name": value_string(row, &["MemberName", "Name", "FullName"]),
                        "vote": member_vote,
                        "source_document_id": source_document_id,
                        "parser_profile": PARSER_PROFILE,
                    }),
                )?;
            }
            if let Some(measure_id) = target_measure_id {
                batch.push(
                    "legislative_edges.jsonl",
                    &edge_row(&measure_id, &vote_id, "CAST_VOTE", &source_document_id),
                )?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl DataConnector for OregonLegODataConnector {
    fn source_id(&self) -> &'static str {
        SOURCE_ID
    }

    fn source_kind(&self) -> SourceKind {
        self.entry.source_type
    }

    async fn discover(&self) -> Result<Vec<SourceItem>> {
        let base = self.base_url();
        let session_key = self.session_key();
        let mut items = vec![
            SourceItem {
                item_id: "metadata".to_string(),
                url: Some(format!("{base}/$metadata")),
                title: Some("Oregon Legislature OData metadata".to_string()),
                content_type: Some("application/xml".to_string()),
                metadata: BTreeMap::new(),
            },
            SourceItem {
                item_id: "LegislativeSessions".to_string(),
                url: Some(format!(
                    "{base}/LegislativeSessions?$orderby=BeginDate%20desc&$format=json"
                )),
                title: Some("Oregon legislative sessions".to_string()),
                content_type: Some("application/json".to_string()),
                metadata: BTreeMap::new(),
            },
        ];
        for (entity_set, select, order_by) in SESSION_ENTITY_SETS {
            let mut metadata = BTreeMap::new();
            metadata.insert("session_key".to_string(), session_key.clone());
            metadata.insert("entity_set".to_string(), (*entity_set).to_string());
            items.push(SourceItem {
                item_id: format!("{entity_set}_{session_key}"),
                url: Some(session_entity_url(
                    &base,
                    entity_set,
                    &session_key,
                    *select,
                    *order_by,
                )),
                title: Some(format!("{entity_set} for {session_key}")),
                content_type: Some("application/json".to_string()),
                metadata,
            });
        }
        Ok(items)
    }

    async fn parse(&self, artifact: &RawArtifact) -> Result<GraphBatch> {
        let mut batch = GraphBatch::default();
        let source_document = self.source_document_for_artifact(artifact);
        batch.push("source_documents.jsonl", &source_document)?;
        let entity_set = entity_set_from_item_id(&artifact.metadata.item_id);
        if entity_set == "$metadata" {
            self.parse_metadata(artifact, &mut batch)?;
        } else {
            self.parse_rows_artifact(artifact, entity_set, &mut batch)?;
        }
        dedupe_batch(&mut batch);
        Ok(batch)
    }

    async fn qc(&self, artifacts: &[ArtifactMetadata], batch: &GraphBatch) -> Result<QcReport> {
        let mut report = qc_source_batch(&self.entry, artifacts, batch);
        for diagnostic in batch
            .files
            .get("parser_diagnostics.jsonl")
            .into_iter()
            .flat_map(|rows| rows.iter())
        {
            if diagnostic
                .get("severity")
                .and_then(Value::as_str)
                .is_some_and(|severity| severity == "error")
            {
                report.errors.push(format!(
                    "parser error: {}",
                    diagnostic
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown parser error")
                ));
            }
        }

        if artifacts
            .iter()
            .any(|artifact| artifact.item_id.starts_with("Measures_"))
            && !batch.files.contains_key("legislative_measures.jsonl")
        {
            report
                .errors
                .push("Measures artifact emitted no legislative measure rows".to_string());
        }
        if let Some(measures) = batch.files.get("legislative_measures.jsonl") {
            for measure in measures {
                for field in ["session_key", "measure_prefix", "measure_number"] {
                    if measure
                        .get(field)
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .is_none()
                    {
                        report.errors.push(format!(
                            "measure {} is missing {field}",
                            measure
                                .get("legislative_measure_id")
                                .and_then(Value::as_str)
                                .unwrap_or("<unknown>")
                        ));
                    }
                }
            }
        }

        report.status = if !report.errors.is_empty() {
            QcReportStatus::Fail
        } else if !report.warnings.is_empty() {
            QcReportStatus::Warning
        } else {
            QcReportStatus::Pass
        };
        Ok(report)
    }
}

fn session_entity_url(
    base: &str,
    entity_set: &str,
    session_key: &str,
    select: Option<&str>,
    order_by: Option<&str>,
) -> String {
    let mut parts = vec![format!(
        "$filter=SessionKey%20eq%20'{}'",
        session_key.replace('\'', "''")
    )];
    if let Some(select) = select {
        parts.push(format!("$select={select}"));
    }
    if let Some(order_by) = order_by {
        parts.push(format!("$orderby={order_by}"));
    }
    parts.push("$format=json".to_string());
    format!("{base}/{entity_set}?{}", parts.join("&"))
}

fn raw_response_document_id(metadata: &ArtifactMetadata) -> String {
    format!("src:{}:{}", SOURCE_ID, stable_id(&metadata.url))
}

fn entity_set_from_item_id(item_id: &str) -> &str {
    if item_id == "metadata" {
        return "$metadata";
    }
    item_id.split('_').next().unwrap_or(item_id)
}

fn extract_odata_rows<'a>(value: &'a Value, entity_set: &str) -> Vec<&'a Value> {
    for candidate in [
        value.pointer("/d/results"),
        value.pointer("/d"),
        value.pointer("/value"),
        value.pointer("/results"),
        value.get(entity_set),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(rows) = candidate.as_array() {
            return rows.iter().collect();
        }
    }
    if let Some(rows) = value.as_array() {
        return rows.iter().collect();
    }
    Vec::new()
}

fn has_next_link(value: &Value) -> bool {
    value.pointer("/d/__next").is_some()
        || value.pointer("/odata.nextLink").is_some()
        || value.pointer("/@odata.nextLink").is_some()
}

fn row_stable_id(row: &Value) -> String {
    stable_id(&serde_json::to_string(row).unwrap_or_default())
}

fn dedupe_batch(batch: &mut GraphBatch) {
    for (file_name, rows) in &mut batch.files {
        let mut seen = BTreeSet::new();
        rows.retain(|row| seen.insert(row_identity(file_name, row)));
    }
}

fn row_identity(file_name: &str, row: &Value) -> String {
    for field in identity_fields(file_name) {
        let Some(value) = row.get(field) else {
            continue;
        };
        match value {
            Value::Null => {}
            Value::String(value) if value.trim().is_empty() => {}
            Value::String(value) => return format!("{field}:{value}"),
            other => return format!("{field}:{other}"),
        }
    }
    row_stable_id(row)
}

fn identity_fields(file_name: &str) -> &'static [&'static str] {
    match file_name {
        "source_documents.jsonl" => &["source_document_id"],
        "odata_entity_sets.jsonl" => &["odata_entity_set_id"],
        "odata_metadata_summary.jsonl" => &["metadata_summary_id"],
        "odata_entity_set_stats.jsonl" => &["odata_entity_set_stats_id"],
        "legislative_sessions.jsonl" => &["legislative_session_id"],
        "legislative_measures.jsonl" => &["legislative_measure_id", "measure_id"],
        "legislative_measure_documents.jsonl" => &["measure_document_id"],
        "legislative_measure_versions.jsonl" => &["measure_version_id"],
        "legislative_measure_history_actions.jsonl" => &["measure_history_action_id"],
        "legislative_measure_sponsors.jsonl" => &["measure_sponsor_id"],
        "legislative_committees.jsonl" => &["committee_id"],
        "legislative_legislators.jsonl" => &["legislator_id"],
        "legislative_committee_meetings.jsonl" => &["committee_meeting_id"],
        "legislative_votes.jsonl" | "vote_events.jsonl" => {
            &["legislative_vote_id", "vote_event_id"]
        }
        "vote_records.jsonl" => &["vote_record_id"],
        "session_laws.jsonl" => &["session_law_id"],
        "status_events.jsonl" => &["status_event_id"],
        "lineage_events.jsonl" => &["lineage_event_id"],
        "legal_actors.jsonl" => &["actor_id"],
        "legislative_edges.jsonl" => &["edge_id"],
        "parser_diagnostics.jsonl" => &["parser_diagnostic_id"],
        _ => &[],
    }
}

#[derive(Debug, Clone)]
struct MeasureKey {
    session_key: String,
    measure_prefix: String,
    measure_number: String,
}

impl MeasureKey {
    fn measure_id(&self) -> String {
        format!(
            "orleg:measure:{}:{}:{}",
            self.session_key, self.measure_prefix, self.measure_number
        )
    }

    fn bill_number(&self) -> String {
        format!("{} {}", self.measure_prefix, self.measure_number)
    }
}

fn measure_key(row: &Value) -> Result<MeasureKey> {
    let session_key = value_string(row, &["SessionKey", "session_key"])
        .ok_or_else(|| anyhow!("measure-linked row is missing SessionKey"))?;
    let measure_prefix = value_string(row, &["MeasurePrefix", "measure_prefix", "Prefix"])
        .ok_or_else(|| anyhow!("measure-linked row is missing MeasurePrefix"))?;
    let measure_number = value_string(row, &["MeasureNumber", "measure_number", "Number"])
        .ok_or_else(|| anyhow!("measure-linked row is missing MeasureNumber"))?;
    Ok(MeasureKey {
        session_key: clean_id_part(&session_key),
        measure_prefix: clean_id_part(&measure_prefix).to_ascii_uppercase(),
        measure_number: clean_number(&measure_number),
    })
}

fn session_id(session_key: &str) -> String {
    format!("orleg:session:{}", clean_id_part(session_key))
}

fn committee_id(session_key: &str, committee_code: &str) -> String {
    format!(
        "orleg:committee:{}:{}",
        clean_id_part(session_key),
        clean_id_part(committee_code)
    )
}

fn legislator_id(session_key: &str, legislator_code: &str) -> String {
    format!(
        "orleg:legislator:{}:{}",
        clean_id_part(session_key),
        clean_id_part(legislator_code)
    )
}

fn edge_row(
    from_id: &str,
    to_id: &str,
    relationship_type: &str,
    source_document_id: &str,
) -> Value {
    json!({
        "edge_id": format!("edge:{}:{}:{}", relationship_type, stable_id(from_id), stable_id(to_id)),
        "from_id": from_id,
        "to_id": to_id,
        "relationship_type": relationship_type,
        "source_id": SOURCE_ID,
        "source_document_id": source_document_id,
        "parser_profile": PARSER_PROFILE,
    })
}

fn value_string(row: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        let Some(value) = row.get(*key) else {
            continue;
        };
        let text = match value {
            Value::Null => None,
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        };
        if let Some(text) = text.map(|value| normalize_ws(&value)) {
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

fn value_i64(row: &Value, keys: &[&str]) -> Option<i64> {
    for key in keys {
        let Some(value) = row.get(*key) else {
            continue;
        };
        if let Some(value) = value.as_i64() {
            return Some(value);
        }
        if let Some(value) = value.as_str().and_then(|value| value.trim().parse().ok()) {
            return Some(value);
        }
    }
    None
}

fn value_bool(row: &Value, keys: &[&str]) -> Option<bool> {
    for key in keys {
        let Some(value) = row.get(*key) else {
            continue;
        };
        match value {
            Value::Bool(value) => return Some(*value),
            Value::Number(value) => return value.as_i64().map(|value| value != 0),
            Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
                "true" | "t" | "yes" | "y" | "1" => return Some(true),
                "false" | "f" | "no" | "n" | "0" => return Some(false),
                _ => {}
            },
            _ => {}
        }
    }
    None
}

fn value_date(row: &Value, keys: &[&str]) -> Option<String> {
    value_string(row, keys).map(|value| normalize_odata_date(&value))
}

fn normalize_odata_date(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(inner) = trimmed
        .strip_prefix("/Date(")
        .and_then(|value| value.strip_suffix(")/"))
    {
        let millis = inner
            .split(|ch| ch == '+' || ch == '-')
            .next()
            .unwrap_or(inner)
            .parse::<i64>()
            .ok();
        if let Some(datetime) = millis.and_then(|millis| Utc.timestamp_millis_opt(millis).single())
        {
            return datetime.to_rfc3339();
        }
    }
    trimmed.to_string()
}

fn date_year(value: &str) -> Option<i32> {
    value.get(0..4)?.parse().ok()
}

fn clean_id_part(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_was_separator = false;
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_separator = false;
        } else if matches!(ch, '-' | '_' | '.') || ch.is_whitespace() {
            if !last_was_separator && !out.is_empty() {
                out.push('-');
                last_was_separator = true;
            }
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

fn clean_number(value: &str) -> String {
    let cleaned = value
        .trim()
        .trim_end_matches(".0")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    if cleaned.is_empty() {
        clean_id_part(value)
    } else {
        cleaned
    }
}

fn normalize_ws(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut pending_space = false;
    for part in value.split_whitespace() {
        if pending_space {
            out.push(' ');
        }
        out.push_str(part);
        pending_space = true;
    }
    out
}

fn count_nonempty_lines(bytes: &[u8]) -> usize {
    let mut count = 0usize;
    let mut line_has_content = false;
    for byte in bytes {
        match byte {
            b'\n' | b'\r' => {
                if line_has_content {
                    count += 1;
                    line_has_content = false;
                }
            }
            b' ' | b'\t' => {}
            _ => line_has_content = true,
        }
    }
    if line_has_content {
        count += 1;
    }
    count.max(1)
}

fn normalize_actor_name(value: &str) -> String {
    normalize_ws(value).to_ascii_lowercase()
}

fn display_name(row: &Value, fallback: &str) -> String {
    if let Some(name) = value_string(row, &["FullName", "Name", "DisplayName"]) {
        return name;
    }
    let first = value_string(row, &["FirstName", "first_name"]);
    let last = value_string(row, &["LastName", "last_name"]);
    match (first, last) {
        (Some(first), Some(last)) => normalize_ws(&format!("{first} {last}")),
        (Some(first), None) => first,
        (None, Some(last)) => last,
        (None, None) => fallback.to_string(),
    }
}

fn absolute_url(value: &str, base: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("//") {
        return Some(format!("https:{trimmed}"));
    }
    if Url::parse(trimmed).is_ok() {
        return Some(trimmed.to_string());
    }
    Url::parse(base)
        .ok()
        .and_then(|base| base.join(trimmed).ok())
        .map(|url| url.to_string())
}

fn file_name_from_url(value: &str) -> Option<String> {
    Url::parse(value).ok().and_then(|url| {
        url.path_segments()
            .and_then(|mut segments| segments.next_back())
            .filter(|segment| !segment.is_empty())
            .map(ToOwned::to_owned)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_registry::{AccessModel, ConnectorStatus, OfficialStatus, SourcePriority};

    fn test_entry() -> SourceRegistryEntry {
        SourceRegistryEntry {
            source_id: SOURCE_ID.to_string(),
            name: "Oregon Legislature OData API".to_string(),
            owner: "Oregon Legislature".to_string(),
            jurisdiction: "or:state".to_string(),
            source_type: SourceKind::Api,
            access: AccessModel::Free,
            official_status: OfficialStatus::Official,
            data_types: vec![],
            update_frequency: "session".to_string(),
            rate_limits_terms: "cache".to_string(),
            robots_acceptable_use: "allowed".to_string(),
            preferred_ingestion_method: "odata".to_string(),
            fallback_ingestion_method: "static".to_string(),
            graph_nodes_created: vec![],
            graph_edges_created: vec![],
            connector_status: ConnectorStatus::Implemented,
            priority: SourcePriority::P0,
            risks: vec![],
            source_url: "https://api.oregonlegislature.gov/odata/ODataService.svc/".to_string(),
            docs_url: "https://www.oregonlegislature.gov/citizen_engagement/Pages/data.aspx"
                .to_string(),
        }
    }

    fn connector() -> OregonLegODataConnector {
        OregonLegODataConnector::new(
            test_entry(),
            ConnectorOptions {
                edition_year: 2025,
                chapters: None,
                session_key: Some("2025R1".to_string()),
                max_items: 0,
            },
        )
    }

    fn artifact(item_id: &str, bytes: Vec<u8>) -> RawArtifact {
        let raw_hash = sha256_hex(&bytes);
        RawArtifact {
            metadata: ArtifactMetadata {
                artifact_id: format!("artifact:{}", stable_id(item_id)),
                source_id: SOURCE_ID.to_string(),
                item_id: item_id.to_string(),
                url: format!("https://example.test/{item_id}"),
                path: format!("/tmp/{item_id}.json"),
                content_type: Some("application/json".to_string()),
                etag: None,
                last_modified: None,
                retrieved_at: Utc::now(),
                raw_hash,
                byte_len: bytes.len(),
                status: "fixture".to_string(),
                skipped: false,
            },
            bytes,
        }
    }

    #[tokio::test]
    async fn discovers_metadata_sessions_and_session_scoped_entity_sets() {
        let items = connector().discover().await.unwrap();
        let item_ids = items
            .into_iter()
            .map(|item| item.item_id)
            .collect::<Vec<_>>();
        assert!(item_ids.contains(&"metadata".to_string()));
        assert!(item_ids.contains(&"LegislativeSessions".to_string()));
        assert!(item_ids.contains(&"Measures_2025R1".to_string()));
        assert!(item_ids.contains(&"MeasureHistoryActions_2025R1".to_string()));
    }

    #[tokio::test]
    async fn parses_measure_rows_with_stable_measure_and_session_law_ids() {
        let payload = json!({
            "d": {
                "results": [{
                    "SessionKey": "2025R1",
                    "MeasurePrefix": "HB",
                    "MeasureNumber": 2001,
                    "CatchLine": "Relating to housing",
                    "MeasureSummary": "Creates a housing program.",
                    "ChapterNumber": 88,
                    "EffectiveDate": "2025-06-01T00:00:00"
                }]
            }
        });
        let bytes = serde_json::to_vec(&payload).unwrap();
        let batch = connector()
            .parse(&artifact("Measures_2025R1", bytes))
            .await
            .unwrap();
        let measures = batch.files.get("legislative_measures.jsonl").unwrap();
        assert_eq!(
            measures[0]["legislative_measure_id"],
            "orleg:measure:2025R1:HB:2001"
        );
        let session_laws = batch.files.get("session_laws.jsonl").unwrap();
        assert_eq!(session_laws[0]["session_law_id"], "or:laws:2025:c:88");
        assert_eq!(session_laws[0]["bill_number"], "HB 2001");
    }

    #[tokio::test]
    async fn parses_legacy_and_v4_json_shapes() {
        let legacy = json!({"d": {"results": [{"SessionKey": "2025R1"}]}});
        let v4 = json!({"value": [{"SessionKey": "2024R1"}]});
        assert_eq!(extract_odata_rows(&legacy, "LegislativeSessions").len(), 1);
        assert_eq!(extract_odata_rows(&v4, "LegislativeSessions").len(), 1);
    }

    #[tokio::test]
    async fn qc_fails_on_parser_error_diagnostics() {
        let payload = json!({"d": {"results": [{"SessionKey": "2025R1"}]}});
        let raw = artifact("Measures_2025R1", serde_json::to_vec(&payload).unwrap());
        let connector = connector();
        let batch = connector.parse(&raw).await.unwrap();
        let report = connector.qc(&[raw.metadata], &batch).await.unwrap();
        assert_eq!(report.status, QcReportStatus::Fail);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("MeasurePrefix")));
    }

    #[tokio::test]
    async fn dedupes_repeated_vote_events_but_keeps_member_records() {
        let payload = json!({
            "d": {
                "results": [
                    {
                        "SessionKey": "2025R1",
                        "MeasurePrefix": "HB",
                        "MeasureNumber": 2001,
                        "MeasureVoteId": 77,
                        "LegislatorCode": "SMITH",
                        "Vote": "Aye"
                    },
                    {
                        "SessionKey": "2025R1",
                        "MeasurePrefix": "HB",
                        "MeasureNumber": 2001,
                        "MeasureVoteId": 77,
                        "LegislatorCode": "JONES",
                        "Vote": "Nay"
                    }
                ]
            }
        });
        let raw = artifact("MeasureVotes_2025R1", serde_json::to_vec(&payload).unwrap());
        let batch = connector().parse(&raw).await.unwrap();
        assert_eq!(batch.files.get("vote_events.jsonl").unwrap().len(), 1);
        assert_eq!(batch.files.get("legislative_votes.jsonl").unwrap().len(), 1);
        assert_eq!(batch.files.get("vote_records.jsonl").unwrap().len(), 2);
    }
}
