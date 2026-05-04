use super::*;
use crate::auth::AuthContext;
use neo4rs::Row;
use serde::{Serialize, de::DeserializeOwned};

const DEFAULT_SETTINGS_SUBJECT: &str = "service:api-key";
const SETTINGS_TEXT_LIMIT: usize = 160;

const ALLOWED_MATTER_TYPES: &[&str] = &[
    "civil",
    "family",
    "small_claims",
    "admin",
    "criminal",
    "appeal",
    "landlord_tenant",
    "employment",
    "fact_check",
    "complaint_analysis",
    "other",
];

const ALLOWED_USER_ROLES: &[&str] = &[
    "plaintiff",
    "defendant",
    "petitioner",
    "respondent",
    "neutral",
    "researcher",
];

const ALLOWED_CONFIDENTIALITY: &[&str] = &["private", "filed", "public", "sealed"];
const ALLOWED_DOCUMENT_TYPES: &[&str] = &[
    "complaint",
    "answer",
    "motion",
    "order",
    "contract",
    "lease",
    "email",
    "letter",
    "notice",
    "medical",
    "police",
    "agency_record",
    "public_record",
    "spreadsheet",
    "photo",
    "screenshot",
    "audio_transcript",
    "receipt",
    "invoice",
    "evidence",
    "exhibit",
    "other",
];
const ALLOWED_TRANSCRIPT_VIEWS: &[&str] = &["redacted", "raw"];
const ALLOWED_TRANSCRIPT_PRESETS: &[&str] = &[
    "verbatim_multilingual",
    "unclear_masked",
    "unclear",
    "legal",
    "medical",
    "financial",
    "technical",
    "code_switching",
    "customer_support",
];
const ALLOWED_EXPORT_FORMATS: &[&str] = &["pdf", "docx", "html", "markdown", "text", "json"];

impl CaseBuilderService {
    pub async fn get_user_settings(
        &self,
        auth: &AuthContext,
    ) -> ApiResult<CaseBuilderUserSettingsResponse> {
        let subject = auth.subject()?.to_string();
        let settings = self
            .ensure_user_settings(&subject, auth.email.clone(), auth.name.clone())
            .await?;
        Ok(CaseBuilderUserSettingsResponse {
            principal: settings_principal(auth),
            settings,
        })
    }

    pub async fn patch_user_settings(
        &self,
        auth: &AuthContext,
        request: PatchCaseBuilderUserSettingsRequest,
    ) -> ApiResult<CaseBuilderUserSettingsResponse> {
        let subject = auth.subject()?.to_string();
        let mut settings = self
            .ensure_user_settings(&subject, auth.email.clone(), auth.name.clone())
            .await?;
        apply_user_settings_patch(&mut settings, request)?;
        settings.updated_at = now_string();
        let settings = self.merge_user_settings(&settings).await?;
        Ok(CaseBuilderUserSettingsResponse {
            principal: settings_principal(auth),
            settings,
        })
    }

    pub async fn get_matter_settings_response(
        &self,
        matter_id: &str,
        auth: &AuthContext,
    ) -> ApiResult<CaseBuilderMatterSettingsResponse> {
        self.matter_settings_response(matter_id, auth).await
    }

    pub async fn patch_matter_config(
        &self,
        matter_id: &str,
        request: PatchCaseBuilderMatterConfigRequest,
        auth: &AuthContext,
    ) -> ApiResult<CaseBuilderMatterSettingsResponse> {
        if let Some(matter_request) = request.matter {
            self.patch_matter(matter_id, matter_request).await?;
        }
        if let Some(settings_request) = request.settings {
            self.patch_matter_settings(matter_id, settings_request, auth)
                .await?;
        }
        self.matter_settings_response(matter_id, auth).await
    }

    pub(super) async fn create_initial_matter_settings(
        &self,
        matter_id: &str,
        request: PatchCaseBuilderMatterSettingsRequest,
        auth: &AuthContext,
    ) -> ApiResult<CaseBuilderMatterSettings> {
        self.patch_matter_settings(matter_id, request, auth).await
    }

    pub(super) async fn effective_settings_for_matter_id(
        &self,
        matter_id: &str,
    ) -> ApiResult<CaseBuilderEffectiveSettings> {
        let matter = self.get_matter_summary(matter_id).await?;
        let subject = settings_subject_for_matter(&matter);
        let user_settings = self.ensure_user_settings(&subject, None, None).await?;
        let matter_settings = self
            .get_matter_settings(matter_id)
            .await?
            .unwrap_or_else(|| default_matter_settings(&matter, now_string()));
        Ok(effective_settings(&user_settings, &matter_settings))
    }

    pub(super) async fn timeline_suggestions_enabled_for_matter(
        &self,
        matter_id: &str,
    ) -> ApiResult<bool> {
        Ok(self
            .effective_settings_for_matter_id(matter_id)
            .await?
            .timeline_suggestions_enabled)
    }

    pub(super) async fn ai_timeline_enrichment_enabled_for_matter(
        &self,
        matter_id: &str,
    ) -> ApiResult<bool> {
        Ok(self
            .effective_settings_for_matter_id(matter_id)
            .await?
            .ai_timeline_enrichment_enabled)
    }

    async fn matter_settings_response(
        &self,
        matter_id: &str,
        auth: &AuthContext,
    ) -> ApiResult<CaseBuilderMatterSettingsResponse> {
        let matter = self.get_matter_summary(matter_id).await?;
        let subject = settings_subject_for_matter_or_auth(&matter, auth);
        let user_settings = self.ensure_user_settings(&subject, None, None).await?;
        let settings = self.ensure_matter_settings(&matter).await?;
        let effective = effective_settings(&user_settings, &settings);
        Ok(CaseBuilderMatterSettingsResponse {
            matter,
            settings,
            effective,
        })
    }

    async fn patch_matter_settings(
        &self,
        matter_id: &str,
        request: PatchCaseBuilderMatterSettingsRequest,
        _auth: &AuthContext,
    ) -> ApiResult<CaseBuilderMatterSettings> {
        let matter = self.get_matter_summary(matter_id).await?;
        let mut settings = self.ensure_matter_settings(&matter).await?;
        apply_matter_settings_patch(&mut settings, request)?;
        settings.owner_subject = matter.owner_subject.clone();
        settings.updated_at = now_string();
        self.merge_matter_settings(&settings).await
    }

    async fn ensure_user_settings(
        &self,
        subject: &str,
        email: Option<String>,
        name: Option<String>,
    ) -> ApiResult<CaseBuilderUserSettings> {
        if let Some(settings) = self.get_user_settings_by_subject(subject).await? {
            return Ok(settings);
        }
        let mut settings = default_user_settings(subject, now_string());
        settings.display_name = name.or(email);
        self.merge_user_settings(&settings).await
    }

    async fn ensure_matter_settings(
        &self,
        matter: &MatterSummary,
    ) -> ApiResult<CaseBuilderMatterSettings> {
        if let Some(mut settings) = self.get_matter_settings(&matter.matter_id).await? {
            if settings.owner_subject != matter.owner_subject {
                settings.owner_subject = matter.owner_subject.clone();
                settings.updated_at = now_string();
                return self.merge_matter_settings(&settings).await;
            }
            return Ok(settings);
        }
        let settings = default_matter_settings(matter, now_string());
        self.merge_matter_settings(&settings).await
    }

    async fn get_user_settings_by_subject(
        &self,
        subject: &str,
    ) -> ApiResult<Option<CaseBuilderUserSettings>> {
        let rows = self
            .neo4j
            .run_rows(
                query("MATCH (s:CaseBuilderUserSettings {subject: $subject}) RETURN s.payload AS payload")
                    .param("subject", subject.to_string()),
            )
            .await?;
        rows.into_iter().next().map(payload_from_row).transpose()
    }

    async fn get_matter_settings(
        &self,
        matter_id: &str,
    ) -> ApiResult<Option<CaseBuilderMatterSettings>> {
        let rows = self
            .neo4j
            .run_rows(
                query("MATCH (s:CaseBuilderMatterSettings {matter_id: $matter_id}) RETURN s.payload AS payload")
                    .param("matter_id", matter_id.to_string()),
            )
            .await?;
        rows.into_iter().next().map(payload_from_row).transpose()
    }

    async fn merge_user_settings(
        &self,
        settings: &CaseBuilderUserSettings,
    ) -> ApiResult<CaseBuilderUserSettings> {
        let payload = to_payload(settings)?;
        self.neo4j
            .run_rows(
                query(
                    "MERGE (s:CaseBuilderUserSettings {settings_id: $settings_id})
                     SET s.payload = $payload,
                         s.subject = $subject,
                         s.updated_at = $updated_at
                     RETURN s.payload AS payload",
                )
                .param("settings_id", settings.settings_id.clone())
                .param("payload", payload)
                .param("subject", settings.subject.clone())
                .param("updated_at", settings.updated_at.clone()),
            )
            .await?;
        Ok(settings.clone())
    }

    async fn merge_matter_settings(
        &self,
        settings: &CaseBuilderMatterSettings,
    ) -> ApiResult<CaseBuilderMatterSettings> {
        let payload = to_payload(settings)?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (m:Matter {matter_id: $matter_id})
                     MERGE (s:CaseBuilderMatterSettings {matter_id: $matter_id})
                     SET s.payload = $payload,
                         s.settings_id = $settings_id,
                         s.owner_subject = $owner_subject,
                         s.updated_at = $updated_at
                     MERGE (m)-[:HAS_SETTINGS]->(s)
                     RETURN s.payload AS payload",
                )
                .param("matter_id", settings.matter_id.clone())
                .param("payload", payload)
                .param("settings_id", settings.settings_id.clone())
                .param("owner_subject", settings.owner_subject.clone())
                .param("updated_at", settings.updated_at.clone()),
            )
            .await?;
        Ok(settings.clone())
    }
}

fn settings_principal(auth: &AuthContext) -> CaseBuilderSettingsPrincipal {
    CaseBuilderSettingsPrincipal {
        subject: auth
            .subject
            .clone()
            .unwrap_or_else(|| DEFAULT_SETTINGS_SUBJECT.to_string()),
        email: auth.email.clone(),
        name: auth.name.clone(),
        roles: auth.roles.iter().cloned().collect(),
        is_service: auth.is_service(),
    }
}

fn default_user_settings(subject: &str, now: String) -> CaseBuilderUserSettings {
    CaseBuilderUserSettings {
        settings_id: format!("casebuilder-user-settings:{subject}"),
        subject: subject.to_string(),
        workspace_label: None,
        display_name: None,
        default_matter_type: "civil".to_string(),
        default_user_role: "neutral".to_string(),
        default_jurisdiction: "Oregon".to_string(),
        default_court: "Unassigned".to_string(),
        default_confidentiality: "private".to_string(),
        default_document_type: "other".to_string(),
        auto_index_uploads: true,
        auto_import_complaints: true,
        preserve_folder_paths: true,
        timeline_suggestions_enabled: true,
        ai_timeline_enrichment_enabled: true,
        transcript_redact_pii: true,
        transcript_speaker_labels: true,
        transcript_default_view: "redacted".to_string(),
        transcript_prompt_preset: "unclear".to_string(),
        transcript_remove_audio_tags: true,
        export_default_format: "pdf".to_string(),
        export_include_exhibits: true,
        export_include_qc_report: true,
        created_at: now.clone(),
        updated_at: now,
    }
}

fn default_matter_settings(matter: &MatterSummary, now: String) -> CaseBuilderMatterSettings {
    CaseBuilderMatterSettings {
        settings_id: format!("casebuilder-matter-settings:{}", matter.matter_id),
        matter_id: matter.matter_id.clone(),
        owner_subject: matter.owner_subject.clone(),
        default_confidentiality: None,
        default_document_type: None,
        auto_index_uploads: None,
        auto_import_complaints: None,
        preserve_folder_paths: None,
        timeline_suggestions_enabled: None,
        ai_timeline_enrichment_enabled: None,
        transcript_redact_pii: None,
        transcript_speaker_labels: None,
        transcript_default_view: None,
        transcript_prompt_preset: None,
        transcript_remove_audio_tags: None,
        export_default_format: None,
        export_include_exhibits: None,
        export_include_qc_report: None,
        created_at: now.clone(),
        updated_at: now,
    }
}

fn effective_settings(
    user: &CaseBuilderUserSettings,
    matter: &CaseBuilderMatterSettings,
) -> CaseBuilderEffectiveSettings {
    CaseBuilderEffectiveSettings {
        default_confidentiality: matter
            .default_confidentiality
            .clone()
            .unwrap_or_else(|| user.default_confidentiality.clone()),
        default_document_type: matter
            .default_document_type
            .clone()
            .unwrap_or_else(|| user.default_document_type.clone()),
        auto_index_uploads: matter.auto_index_uploads.unwrap_or(user.auto_index_uploads),
        auto_import_complaints: matter
            .auto_import_complaints
            .unwrap_or(user.auto_import_complaints),
        preserve_folder_paths: matter
            .preserve_folder_paths
            .unwrap_or(user.preserve_folder_paths),
        timeline_suggestions_enabled: matter
            .timeline_suggestions_enabled
            .unwrap_or(user.timeline_suggestions_enabled),
        ai_timeline_enrichment_enabled: matter
            .ai_timeline_enrichment_enabled
            .unwrap_or(user.ai_timeline_enrichment_enabled),
        transcript_redact_pii: matter
            .transcript_redact_pii
            .unwrap_or(user.transcript_redact_pii),
        transcript_speaker_labels: matter
            .transcript_speaker_labels
            .unwrap_or(user.transcript_speaker_labels),
        transcript_default_view: matter
            .transcript_default_view
            .clone()
            .unwrap_or_else(|| user.transcript_default_view.clone()),
        transcript_prompt_preset: matter
            .transcript_prompt_preset
            .clone()
            .unwrap_or_else(|| user.transcript_prompt_preset.clone()),
        transcript_remove_audio_tags: matter
            .transcript_remove_audio_tags
            .unwrap_or(user.transcript_remove_audio_tags),
        export_default_format: matter
            .export_default_format
            .clone()
            .unwrap_or_else(|| user.export_default_format.clone()),
        export_include_exhibits: matter
            .export_include_exhibits
            .unwrap_or(user.export_include_exhibits),
        export_include_qc_report: matter
            .export_include_qc_report
            .unwrap_or(user.export_include_qc_report),
    }
}

fn apply_user_settings_patch(
    settings: &mut CaseBuilderUserSettings,
    request: PatchCaseBuilderUserSettingsRequest,
) -> ApiResult<()> {
    if let Some(value) = request.workspace_label {
        settings.workspace_label = clean_nullable_text(value, "workspace_label", 80)?;
    }
    if let Some(value) = request.display_name {
        settings.display_name = clean_nullable_text(value, "display_name", 80)?;
    }
    if let Some(value) = request.default_matter_type {
        settings.default_matter_type = choice(value, "default_matter_type", ALLOWED_MATTER_TYPES)?;
    }
    if let Some(value) = request.default_user_role {
        settings.default_user_role = choice(value, "default_user_role", ALLOWED_USER_ROLES)?;
    }
    if let Some(value) = request.default_jurisdiction {
        settings.default_jurisdiction =
            text_or_default(value, "default_jurisdiction", "Oregon", SETTINGS_TEXT_LIMIT)?;
    }
    if let Some(value) = request.default_court {
        settings.default_court =
            text_or_default(value, "default_court", "Unassigned", SETTINGS_TEXT_LIMIT)?;
    }
    if let Some(value) = request.default_confidentiality {
        settings.default_confidentiality =
            choice(value, "default_confidentiality", ALLOWED_CONFIDENTIALITY)?;
    }
    if let Some(value) = request.default_document_type {
        settings.default_document_type =
            choice(value, "default_document_type", ALLOWED_DOCUMENT_TYPES)?;
    }
    patch_bool(request.auto_index_uploads, &mut settings.auto_index_uploads);
    patch_bool(
        request.auto_import_complaints,
        &mut settings.auto_import_complaints,
    );
    patch_bool(
        request.preserve_folder_paths,
        &mut settings.preserve_folder_paths,
    );
    patch_bool(
        request.timeline_suggestions_enabled,
        &mut settings.timeline_suggestions_enabled,
    );
    patch_bool(
        request.ai_timeline_enrichment_enabled,
        &mut settings.ai_timeline_enrichment_enabled,
    );
    patch_bool(
        request.transcript_redact_pii,
        &mut settings.transcript_redact_pii,
    );
    patch_bool(
        request.transcript_speaker_labels,
        &mut settings.transcript_speaker_labels,
    );
    if let Some(value) = request.transcript_default_view {
        settings.transcript_default_view =
            choice(value, "transcript_default_view", ALLOWED_TRANSCRIPT_VIEWS)?;
    }
    if let Some(value) = request.transcript_prompt_preset {
        settings.transcript_prompt_preset = choice(
            value,
            "transcript_prompt_preset",
            ALLOWED_TRANSCRIPT_PRESETS,
        )?;
    }
    patch_bool(
        request.transcript_remove_audio_tags,
        &mut settings.transcript_remove_audio_tags,
    );
    if let Some(value) = request.export_default_format {
        settings.export_default_format =
            choice(value, "export_default_format", ALLOWED_EXPORT_FORMATS)?;
    }
    patch_bool(
        request.export_include_exhibits,
        &mut settings.export_include_exhibits,
    );
    patch_bool(
        request.export_include_qc_report,
        &mut settings.export_include_qc_report,
    );
    Ok(())
}

fn apply_matter_settings_patch(
    settings: &mut CaseBuilderMatterSettings,
    request: PatchCaseBuilderMatterSettingsRequest,
) -> ApiResult<()> {
    patch_optional_choice(
        request.default_confidentiality,
        &mut settings.default_confidentiality,
        "default_confidentiality",
        ALLOWED_CONFIDENTIALITY,
    )?;
    patch_optional_choice(
        request.default_document_type,
        &mut settings.default_document_type,
        "default_document_type",
        ALLOWED_DOCUMENT_TYPES,
    )?;
    patch_optional_bool(request.auto_index_uploads, &mut settings.auto_index_uploads);
    patch_optional_bool(
        request.auto_import_complaints,
        &mut settings.auto_import_complaints,
    );
    patch_optional_bool(
        request.preserve_folder_paths,
        &mut settings.preserve_folder_paths,
    );
    patch_optional_bool(
        request.timeline_suggestions_enabled,
        &mut settings.timeline_suggestions_enabled,
    );
    patch_optional_bool(
        request.ai_timeline_enrichment_enabled,
        &mut settings.ai_timeline_enrichment_enabled,
    );
    patch_optional_bool(
        request.transcript_redact_pii,
        &mut settings.transcript_redact_pii,
    );
    patch_optional_bool(
        request.transcript_speaker_labels,
        &mut settings.transcript_speaker_labels,
    );
    patch_optional_choice(
        request.transcript_default_view,
        &mut settings.transcript_default_view,
        "transcript_default_view",
        ALLOWED_TRANSCRIPT_VIEWS,
    )?;
    patch_optional_choice(
        request.transcript_prompt_preset,
        &mut settings.transcript_prompt_preset,
        "transcript_prompt_preset",
        ALLOWED_TRANSCRIPT_PRESETS,
    )?;
    patch_optional_bool(
        request.transcript_remove_audio_tags,
        &mut settings.transcript_remove_audio_tags,
    );
    patch_optional_choice(
        request.export_default_format,
        &mut settings.export_default_format,
        "export_default_format",
        ALLOWED_EXPORT_FORMATS,
    )?;
    patch_optional_bool(
        request.export_include_exhibits,
        &mut settings.export_include_exhibits,
    );
    patch_optional_bool(
        request.export_include_qc_report,
        &mut settings.export_include_qc_report,
    );
    Ok(())
}

fn settings_subject_for_matter(matter: &MatterSummary) -> String {
    matter
        .owner_subject
        .clone()
        .unwrap_or_else(|| DEFAULT_SETTINGS_SUBJECT.to_string())
}

fn settings_subject_for_matter_or_auth(matter: &MatterSummary, auth: &AuthContext) -> String {
    matter
        .owner_subject
        .clone()
        .or_else(|| auth.subject.clone())
        .unwrap_or_else(|| DEFAULT_SETTINGS_SUBJECT.to_string())
}

fn clean_nullable_text(
    value: Option<String>,
    field: &str,
    max_chars: usize,
) -> ApiResult<Option<String>> {
    match value {
        Some(value) => clean_text(value, field, max_chars),
        None => Ok(None),
    }
}

fn text_or_default(
    value: String,
    field: &str,
    default: &str,
    max_chars: usize,
) -> ApiResult<String> {
    Ok(clean_text(value, field, max_chars)?.unwrap_or_else(|| default.to_string()))
}

fn clean_text(value: String, field: &str, max_chars: usize) -> ApiResult<Option<String>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_chars {
        return Err(ApiError::BadRequest(format!(
            "CaseBuilder setting {field} must be at most {max_chars} characters"
        )));
    }
    Ok(Some(value.to_string()))
}

fn choice(value: String, field: &str, allowed: &[&str]) -> ApiResult<String> {
    let value = value.trim();
    if allowed.contains(&value) {
        Ok(value.to_string())
    } else {
        Err(ApiError::BadRequest(format!(
            "Unsupported CaseBuilder setting {field} {value}"
        )))
    }
}

fn patch_optional_choice(
    patch: Option<Option<String>>,
    target: &mut Option<String>,
    field: &str,
    allowed: &[&str],
) -> ApiResult<()> {
    if let Some(value) = patch {
        *target = value
            .map(|value| choice(value, field, allowed))
            .transpose()?;
    }
    Ok(())
}

fn patch_bool(patch: Option<bool>, target: &mut bool) {
    if let Some(value) = patch {
        *target = value;
    }
}

fn patch_optional_bool(patch: Option<Option<bool>>, target: &mut Option<bool>) {
    if let Some(value) = patch {
        *target = value;
    }
}

fn to_payload<T: Serialize>(value: &T) -> ApiResult<String> {
    serde_json::to_string(value).map_err(|error| ApiError::Internal(error.to_string()))
}

fn from_payload<T: DeserializeOwned>(payload: &str) -> ApiResult<T> {
    serde_json::from_str(payload).map_err(|error| ApiError::Internal(error.to_string()))
}

fn payload_from_row<T: DeserializeOwned>(row: Row) -> ApiResult<T> {
    let payload = row
        .get::<String>("payload")
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    from_payload(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matter() -> MatterSummary {
        MatterSummary {
            matter_id: "matter:test".to_string(),
            name: "Test Matter".to_string(),
            short_name: Some("Test Matter".to_string()),
            matter_type: "civil".to_string(),
            status: "intake".to_string(),
            user_role: "neutral".to_string(),
            jurisdiction: "Oregon".to_string(),
            court: "Unassigned".to_string(),
            case_number: None,
            owner_subject: Some("user:test".to_string()),
            owner_email: None,
            owner_name: None,
            created_by_subject: Some("user:test".to_string()),
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
            document_count: 0,
            fact_count: 0,
            evidence_count: 0,
            claim_count: 0,
            draft_count: 0,
            open_task_count: 0,
            next_deadline: None,
        }
    }

    #[test]
    fn default_user_settings_are_operationally_safe() {
        let settings = default_user_settings("user:test", "1".to_string());
        assert_eq!(settings.default_jurisdiction, "Oregon");
        assert_eq!(settings.default_matter_type, "civil");
        assert_eq!(settings.default_user_role, "neutral");
        assert_eq!(settings.default_confidentiality, "private");
        assert!(settings.auto_index_uploads);
        assert!(settings.transcript_redact_pii);
        assert_eq!(settings.transcript_default_view, "redacted");
    }

    #[test]
    fn matter_settings_inherit_until_overridden() {
        let user = default_user_settings("user:test", "1".to_string());
        let mut matter_settings = default_matter_settings(&matter(), "1".to_string());
        let effective = effective_settings(&user, &matter_settings);
        assert_eq!(effective.default_confidentiality, "private");
        assert!(effective.auto_index_uploads);

        matter_settings.default_confidentiality = Some("sealed".to_string());
        matter_settings.auto_index_uploads = Some(false);
        let effective = effective_settings(&user, &matter_settings);
        assert_eq!(effective.default_confidentiality, "sealed");
        assert!(!effective.auto_index_uploads);
    }

    #[test]
    fn patches_validate_choices_and_allow_matter_clear() {
        let mut user = default_user_settings("user:test", "1".to_string());
        apply_user_settings_patch(
            &mut user,
            PatchCaseBuilderUserSettingsRequest {
                default_matter_type: Some("landlord_tenant".to_string()),
                default_confidentiality: Some("sealed".to_string()),
                transcript_default_view: Some("redacted".to_string()),
                ..Default::default()
            },
        )
        .expect("valid patch");
        assert_eq!(user.default_matter_type, "landlord_tenant");
        assert_eq!(user.default_confidentiality, "sealed");

        assert!(
            apply_user_settings_patch(
                &mut user,
                PatchCaseBuilderUserSettingsRequest {
                    default_document_type: Some("surprise".to_string()),
                    ..Default::default()
                },
            )
            .is_err()
        );

        let mut matter_settings = default_matter_settings(&matter(), "1".to_string());
        apply_matter_settings_patch(
            &mut matter_settings,
            PatchCaseBuilderMatterSettingsRequest {
                default_document_type: Some(Some("evidence".to_string())),
                auto_import_complaints: Some(Some(false)),
                ..Default::default()
            },
        )
        .expect("valid matter patch");
        assert_eq!(
            matter_settings.default_document_type.as_deref(),
            Some("evidence")
        );
        assert_eq!(matter_settings.auto_import_complaints, Some(false));

        apply_matter_settings_patch(
            &mut matter_settings,
            PatchCaseBuilderMatterSettingsRequest {
                default_document_type: Some(None),
                auto_import_complaints: Some(None),
                ..Default::default()
            },
        )
        .expect("clear matter override");
        assert!(matter_settings.default_document_type.is_none());
        assert!(matter_settings.auto_import_complaints.is_none());
    }

    #[test]
    fn patch_deserialization_preserves_null_as_clear() {
        let matter_patch: PatchCaseBuilderMatterSettingsRequest =
            serde_json::from_str(r#"{"default_document_type":null,"auto_index_uploads":null}"#)
                .expect("matter patch json");
        assert!(matches!(matter_patch.default_document_type, Some(None)));
        assert!(matches!(matter_patch.auto_index_uploads, Some(None)));

        let user_patch: PatchCaseBuilderUserSettingsRequest =
            serde_json::from_str(r#"{"workspace_label":null}"#).expect("user patch json");
        assert!(matches!(user_patch.workspace_label, Some(None)));
    }
}
