use crate::error::{ApiError, ApiResult};
use crate::models::casebuilder::*;
use crate::models::search::{SearchMode, SearchQuery};
use crate::services::casebuilder::BinaryUploadRequest;
use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::DefaultBodyLimit,
    extract::{Path, Query, State},
    http::{header, HeaderMap},
    routing::{get, patch, post},
    Json, Router,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CompareWorkProductParams {
    from: String,
    to: Option<String>,
    layers: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/matters", get(list_matters).post(create_matter))
        .route(
            "/matters/:matter_id",
            get(get_matter).patch(patch_matter).delete(delete_matter),
        )
        .route(
            "/matters/:matter_id/parties",
            get(list_parties).post(create_party),
        )
        .route("/matters/:matter_id/files", post(upload_file))
        .route("/matters/:matter_id/files/binary", post(upload_binary_file))
        .route(
            "/matters/:matter_id/files/uploads",
            post(create_file_upload),
        )
        .route(
            "/matters/:matter_id/files/uploads/:upload_id/complete",
            post(complete_file_upload),
        )
        .route("/matters/:matter_id/documents", get(list_documents))
        .route(
            "/matters/:matter_id/documents/:document_id",
            get(get_document).delete(delete_document),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/download-url",
            post(create_download_url),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/extract",
            post(extract_document),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/import-complaint",
            post(import_document_complaint),
        )
        .route(
            "/matters/:matter_id/facts",
            get(list_facts).post(create_fact),
        )
        .route("/matters/:matter_id/facts/:fact_id", patch(patch_fact))
        .route(
            "/matters/:matter_id/facts/:fact_id/approve",
            post(approve_fact),
        )
        .route(
            "/matters/:matter_id/timeline",
            get(list_timeline).post(create_timeline_event),
        )
        .route(
            "/matters/:matter_id/claims",
            get(list_claims).post(create_claim),
        )
        .route(
            "/matters/:matter_id/claims/:claim_id/map-elements",
            post(map_claim_elements),
        )
        .route(
            "/matters/:matter_id/defenses",
            get(list_defenses).post(create_defense),
        )
        .route(
            "/matters/:matter_id/evidence",
            get(list_evidence).post(create_evidence),
        )
        .route(
            "/matters/:matter_id/evidence/:evidence_id/link-fact",
            post(link_evidence_fact),
        )
        .route("/matters/:matter_id/deadlines", get(list_deadlines))
        .route("/matters/:matter_id/tasks", get(list_tasks))
        .route(
            "/matters/:matter_id/work-products",
            get(list_work_products).post(create_work_product),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id",
            get(get_work_product).patch(patch_work_product),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/blocks",
            post(create_work_product_block),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/blocks/:block_id",
            patch(patch_work_product_block),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/links",
            post(link_work_product_support),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/qc/run",
            post(run_work_product_qc),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/qc/findings",
            get(list_work_product_findings),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/qc/findings/:finding_id",
            patch(patch_work_product_finding),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/preview",
            get(preview_work_product),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/export",
            post(export_work_product),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/artifacts/:artifact_id",
            get(get_work_product_artifact),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/artifacts/:artifact_id/download",
            get(download_work_product_artifact),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/ai/commands",
            post(run_work_product_ai_command),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/history",
            get(work_product_history),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/change-sets/:change_set_id",
            get(get_work_product_change_set),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/snapshots",
            get(list_work_product_snapshots).post(create_work_product_snapshot),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/snapshots/:snapshot_id",
            get(get_work_product_snapshot),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/compare",
            get(compare_work_product_snapshots),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/restore",
            post(restore_work_product_version),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/export-history",
            get(work_product_export_history),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/ai-audit",
            get(work_product_ai_audit),
        )
        .route(
            "/matters/:matter_id/complaints",
            get(list_complaints).post(create_complaint),
        )
        .route(
            "/matters/:matter_id/complaints/import",
            post(import_complaints),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id",
            get(get_complaint).patch(patch_complaint),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/setup",
            patch(update_complaint_setup),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/sections",
            post(create_complaint_section),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/counts",
            post(create_complaint_count),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/paragraphs",
            post(create_complaint_paragraph),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/paragraphs/renumber",
            post(renumber_complaint_paragraphs),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/paragraphs/:paragraph_id",
            patch(patch_complaint_paragraph),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/links",
            post(link_complaint_support),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/qc/run",
            post(run_complaint_qc),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/qc/findings",
            get(list_complaint_findings),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/qc/findings/:finding_id",
            patch(patch_complaint_finding),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/preview",
            get(preview_complaint),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/export",
            post(export_complaint),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/artifacts/:artifact_id",
            get(get_complaint_artifact),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/artifacts/:artifact_id/download",
            get(download_complaint_artifact),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/ai/commands",
            post(run_complaint_ai_command),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/history",
            get(work_product_history),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/change-sets/:change_set_id",
            get(get_work_product_change_set),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/snapshots",
            get(list_work_product_snapshots).post(create_work_product_snapshot),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/snapshots/:snapshot_id",
            get(get_work_product_snapshot),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/compare",
            get(compare_work_product_snapshots),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/restore",
            post(restore_work_product_version),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/export-history",
            get(work_product_export_history),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/ai-audit",
            get(work_product_ai_audit),
        )
        .route(
            "/matters/:matter_id/complaints/:complaint_id/filing-packet",
            get(filing_packet),
        )
        .route(
            "/matters/:matter_id/drafts",
            get(list_drafts).post(create_draft),
        )
        .route(
            "/matters/:matter_id/drafts/:draft_id",
            get(get_draft).patch(patch_draft),
        )
        .route(
            "/matters/:matter_id/drafts/:draft_id/generate",
            post(generate_draft),
        )
        .route(
            "/matters/:matter_id/drafts/:draft_id/fact-check",
            post(fact_check_draft),
        )
        .route(
            "/matters/:matter_id/drafts/:draft_id/citation-check",
            post(citation_check_draft),
        )
        .route(
            "/matters/:matter_id/authority/search",
            get(authority_search),
        )
        .route(
            "/matters/:matter_id/authority/recommend",
            post(authority_recommend),
        )
        .route(
            "/matters/:matter_id/authority/attach",
            post(authority_attach),
        )
        .route(
            "/matters/:matter_id/authority/detach",
            post(authority_detach),
        )
        .route("/matters/:matter_id/export/docx", post(export_not_ready))
        .route("/matters/:matter_id/export/pdf", post(export_not_ready))
        .route(
            "/matters/:matter_id/export/filing-packet",
            post(export_not_ready),
        )
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
}

async fn list_matters(State(state): State<AppState>) -> ApiResult<Json<Vec<MatterSummary>>> {
    Ok(Json(state.casebuilder_service.list_matters().await?))
}

async fn create_matter(
    State(state): State<AppState>,
    Json(request): Json<CreateMatterRequest>,
) -> ApiResult<Json<MatterBundle>> {
    Ok(Json(
        state.casebuilder_service.create_matter(request).await?,
    ))
}

async fn get_matter(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<MatterBundle>> {
    Ok(Json(
        state.casebuilder_service.get_matter(&matter_id).await?,
    ))
}

async fn patch_matter(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<PatchMatterRequest>,
) -> ApiResult<Json<MatterBundle>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_matter(&matter_id, request)
            .await?,
    ))
}

async fn delete_matter(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    state.casebuilder_service.delete_matter(&matter_id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn list_parties(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<CaseParty>>> {
    Ok(Json(
        state.casebuilder_service.list_parties(&matter_id).await?,
    ))
}

async fn create_party(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreatePartyRequest>,
) -> ApiResult<Json<CaseParty>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_party(&matter_id, request)
            .await?,
    ))
}

async fn upload_file(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<UploadFileRequest>,
) -> ApiResult<Json<CaseDocument>> {
    Ok(Json(
        state
            .casebuilder_service
            .upload_file(&matter_id, request)
            .await?,
    ))
}

async fn upload_binary_file(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<CaseDocument>> {
    let request = parse_binary_upload(&headers, body)?;
    Ok(Json(
        state
            .casebuilder_service
            .upload_binary_file(&matter_id, request)
            .await?,
    ))
}

async fn create_file_upload(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateFileUploadRequest>,
) -> ApiResult<Json<FileUploadResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_file_upload(&matter_id, request)
            .await?,
    ))
}

async fn complete_file_upload(
    State(state): State<AppState>,
    Path((matter_id, upload_id)): Path<(String, String)>,
    Json(request): Json<CompleteFileUploadRequest>,
) -> ApiResult<Json<CaseDocument>> {
    Ok(Json(
        state
            .casebuilder_service
            .complete_file_upload(&matter_id, &upload_id, request)
            .await?,
    ))
}

async fn list_documents(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<CaseDocument>>> {
    Ok(Json(
        state.casebuilder_service.list_documents(&matter_id).await?,
    ))
}

async fn get_document(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
) -> ApiResult<Json<CaseDocument>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_document(&matter_id, &document_id)
            .await?,
    ))
}

async fn extract_document(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
) -> ApiResult<Json<DocumentExtractionResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .extract_document(&matter_id, &document_id)
            .await?,
    ))
}

async fn import_document_complaint(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
    Json(mut request): Json<ComplaintImportRequest>,
) -> ApiResult<Json<ComplaintImportResponse>> {
    request.document_id = Some(document_id.clone());
    Ok(Json(
        state
            .casebuilder_service
            .import_complaint_from_document(&matter_id, &document_id, request)
            .await?,
    ))
}

async fn create_download_url(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
) -> ApiResult<Json<DownloadUrlResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_download_url(&matter_id, &document_id)
            .await?,
    ))
}

async fn delete_document(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
) -> ApiResult<Json<DeleteDocumentResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .delete_document(&matter_id, &document_id)
            .await?,
    ))
}

async fn list_facts(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<CaseFact>>> {
    Ok(Json(
        state.casebuilder_service.list_facts(&matter_id).await?,
    ))
}

async fn create_fact(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateFactRequest>,
) -> ApiResult<Json<CaseFact>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_fact(&matter_id, request)
            .await?,
    ))
}

async fn patch_fact(
    State(state): State<AppState>,
    Path((matter_id, fact_id)): Path<(String, String)>,
    Json(request): Json<PatchFactRequest>,
) -> ApiResult<Json<CaseFact>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_fact(&matter_id, &fact_id, request)
            .await?,
    ))
}

async fn approve_fact(
    State(state): State<AppState>,
    Path((matter_id, fact_id)): Path<(String, String)>,
) -> ApiResult<Json<CaseFact>> {
    Ok(Json(
        state
            .casebuilder_service
            .approve_fact(&matter_id, &fact_id)
            .await?,
    ))
}

async fn list_timeline(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<CaseTimelineEvent>>> {
    Ok(Json(
        state.casebuilder_service.list_timeline(&matter_id).await?,
    ))
}

async fn create_timeline_event(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateTimelineEventRequest>,
) -> ApiResult<Json<CaseTimelineEvent>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_timeline_event(&matter_id, request)
            .await?,
    ))
}

async fn list_claims(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<CaseClaim>>> {
    Ok(Json(
        state.casebuilder_service.list_claims(&matter_id).await?,
    ))
}

async fn create_claim(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateClaimRequest>,
) -> ApiResult<Json<CaseClaim>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_claim(&matter_id, request)
            .await?,
    ))
}

async fn map_claim_elements(
    State(state): State<AppState>,
    Path((matter_id, claim_id)): Path<(String, String)>,
) -> ApiResult<Json<CaseClaim>> {
    Ok(Json(
        state
            .casebuilder_service
            .map_claim_elements(&matter_id, &claim_id)
            .await?,
    ))
}

async fn list_defenses(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<CaseDefense>>> {
    Ok(Json(
        state.casebuilder_service.list_defenses(&matter_id).await?,
    ))
}

async fn create_defense(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateDefenseRequest>,
) -> ApiResult<Json<CaseDefense>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_defense(&matter_id, request)
            .await?,
    ))
}

async fn list_evidence(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<CaseEvidence>>> {
    Ok(Json(
        state.casebuilder_service.list_evidence(&matter_id).await?,
    ))
}

async fn create_evidence(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateEvidenceRequest>,
) -> ApiResult<Json<CaseEvidence>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_evidence(&matter_id, request)
            .await?,
    ))
}

async fn link_evidence_fact(
    State(state): State<AppState>,
    Path((matter_id, evidence_id)): Path<(String, String)>,
    Json(request): Json<LinkEvidenceFactRequest>,
) -> ApiResult<Json<CaseEvidence>> {
    Ok(Json(
        state
            .casebuilder_service
            .link_evidence_fact(&matter_id, &evidence_id, request)
            .await?,
    ))
}

async fn list_deadlines(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<CaseDeadline>>> {
    Ok(Json(
        state.casebuilder_service.list_deadlines(&matter_id).await?,
    ))
}

async fn list_tasks(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<CaseTask>>> {
    Ok(Json(
        state.casebuilder_service.list_tasks(&matter_id).await?,
    ))
}

async fn list_complaints(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<ComplaintDraft>>> {
    Ok(Json(
        state
            .casebuilder_service
            .list_complaints(&matter_id)
            .await?,
    ))
}

async fn create_complaint(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateComplaintRequest>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_complaint(&matter_id, request)
            .await?,
    ))
}

async fn import_complaints(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<ComplaintImportRequest>,
) -> ApiResult<Json<ComplaintImportResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .import_complaints(&matter_id, request)
            .await?,
    ))
}

async fn get_complaint(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_complaint(&matter_id, &complaint_id)
            .await?,
    ))
}

async fn patch_complaint(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
    Json(request): Json<PatchComplaintRequest>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_complaint(&matter_id, &complaint_id, request)
            .await?,
    ))
}

async fn update_complaint_setup(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
    Json(request): Json<PatchComplaintRequest>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .update_complaint_setup(&matter_id, &complaint_id, request)
            .await?,
    ))
}

async fn create_complaint_section(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
    Json(request): Json<CreateComplaintSectionRequest>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_complaint_section(&matter_id, &complaint_id, request)
            .await?,
    ))
}

async fn create_complaint_count(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
    Json(request): Json<CreateComplaintCountRequest>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_complaint_count(&matter_id, &complaint_id, request)
            .await?,
    ))
}

async fn create_complaint_paragraph(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
    Json(request): Json<CreateComplaintParagraphRequest>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_complaint_paragraph(&matter_id, &complaint_id, request)
            .await?,
    ))
}

async fn patch_complaint_paragraph(
    State(state): State<AppState>,
    Path((matter_id, complaint_id, paragraph_id)): Path<(String, String, String)>,
    Json(request): Json<PatchComplaintParagraphRequest>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_complaint_paragraph(&matter_id, &complaint_id, &paragraph_id, request)
            .await?,
    ))
}

async fn renumber_complaint_paragraphs(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .renumber_complaint_paragraphs(&matter_id, &complaint_id)
            .await?,
    ))
}

async fn link_complaint_support(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
    Json(request): Json<ComplaintLinkRequest>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .link_complaint_support(&matter_id, &complaint_id, request)
            .await?,
    ))
}

async fn run_complaint_qc(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
) -> ApiResult<Json<AiActionResponse<Vec<RuleCheckFinding>>>> {
    Ok(Json(
        state
            .casebuilder_service
            .run_complaint_qc(&matter_id, &complaint_id)
            .await?,
    ))
}

async fn list_complaint_findings(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<RuleCheckFinding>>> {
    Ok(Json(
        state
            .casebuilder_service
            .list_complaint_findings(&matter_id, &complaint_id)
            .await?,
    ))
}

async fn patch_complaint_finding(
    State(state): State<AppState>,
    Path((matter_id, complaint_id, finding_id)): Path<(String, String, String)>,
    Json(request): Json<PatchRuleFindingRequest>,
) -> ApiResult<Json<ComplaintDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_complaint_finding(&matter_id, &complaint_id, &finding_id, request)
            .await?,
    ))
}

async fn preview_complaint(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
) -> ApiResult<Json<ComplaintPreviewResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .preview_complaint(&matter_id, &complaint_id)
            .await?,
    ))
}

async fn export_complaint(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
    Json(request): Json<ExportComplaintRequest>,
) -> ApiResult<Json<ExportArtifact>> {
    Ok(Json(
        state
            .casebuilder_service
            .export_complaint(&matter_id, &complaint_id, request)
            .await?,
    ))
}

async fn get_complaint_artifact(
    State(state): State<AppState>,
    Path((matter_id, complaint_id, artifact_id)): Path<(String, String, String)>,
) -> ApiResult<Json<ExportArtifact>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_complaint_artifact(&matter_id, &complaint_id, &artifact_id)
            .await?,
    ))
}

async fn download_complaint_artifact(
    State(state): State<AppState>,
    Path((matter_id, complaint_id, artifact_id)): Path<(String, String, String)>,
) -> ApiResult<Json<ComplaintDownloadResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .download_complaint_artifact(&matter_id, &complaint_id, &artifact_id)
            .await?,
    ))
}

async fn run_complaint_ai_command(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
    Json(request): Json<ComplaintAiCommandRequest>,
) -> ApiResult<Json<AiActionResponse<ComplaintDraft>>> {
    Ok(Json(
        state
            .casebuilder_service
            .run_complaint_ai_command(&matter_id, &complaint_id, request)
            .await?,
    ))
}

async fn filing_packet(
    State(state): State<AppState>,
    Path((matter_id, complaint_id)): Path<(String, String)>,
) -> ApiResult<Json<FilingPacket>> {
    Ok(Json(
        state
            .casebuilder_service
            .filing_packet(&matter_id, &complaint_id)
            .await?,
    ))
}

async fn list_work_products(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<WorkProduct>>> {
    Ok(Json(
        state
            .casebuilder_service
            .list_work_products(&matter_id)
            .await?,
    ))
}

async fn create_work_product(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateWorkProductRequest>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_work_product(&matter_id, request)
            .await?,
    ))
}

async fn get_work_product(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_work_product(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn patch_work_product(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<PatchWorkProductRequest>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_work_product(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn create_work_product_block(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<CreateWorkProductBlockRequest>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_work_product_block(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn patch_work_product_block(
    State(state): State<AppState>,
    Path((matter_id, work_product_id, block_id)): Path<(String, String, String)>,
    Json(request): Json<PatchWorkProductBlockRequest>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_work_product_block(&matter_id, &work_product_id, &block_id, request)
            .await?,
    ))
}

async fn link_work_product_support(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<WorkProductLinkRequest>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .link_work_product_support(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn run_work_product_qc(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<AiActionResponse<Vec<WorkProductFinding>>>> {
    Ok(Json(
        state
            .casebuilder_service
            .run_work_product_qc(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn list_work_product_findings(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<WorkProductFinding>>> {
    Ok(Json(
        state
            .casebuilder_service
            .list_work_product_findings(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn patch_work_product_finding(
    State(state): State<AppState>,
    Path((matter_id, work_product_id, finding_id)): Path<(String, String, String)>,
    Json(request): Json<PatchWorkProductFindingRequest>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_work_product_finding(&matter_id, &work_product_id, &finding_id, request)
            .await?,
    ))
}

async fn preview_work_product(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<WorkProductPreviewResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .preview_work_product(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn export_work_product(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<ExportWorkProductRequest>,
) -> ApiResult<Json<WorkProductArtifact>> {
    Ok(Json(
        state
            .casebuilder_service
            .export_work_product(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn get_work_product_artifact(
    State(state): State<AppState>,
    Path((matter_id, work_product_id, artifact_id)): Path<(String, String, String)>,
) -> ApiResult<Json<WorkProductArtifact>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_work_product_artifact(&matter_id, &work_product_id, &artifact_id)
            .await?,
    ))
}

async fn download_work_product_artifact(
    State(state): State<AppState>,
    Path((matter_id, work_product_id, artifact_id)): Path<(String, String, String)>,
) -> ApiResult<Json<WorkProductDownloadResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .download_work_product_artifact(&matter_id, &work_product_id, &artifact_id)
            .await?,
    ))
}

async fn run_work_product_ai_command(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<WorkProductAiCommandRequest>,
) -> ApiResult<Json<AiActionResponse<WorkProduct>>> {
    Ok(Json(
        state
            .casebuilder_service
            .run_work_product_ai_command(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn work_product_history(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<ChangeSet>>> {
    Ok(Json(
        state
            .casebuilder_service
            .work_product_history(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn get_work_product_change_set(
    State(state): State<AppState>,
    Path((matter_id, work_product_id, change_set_id)): Path<(String, String, String)>,
) -> ApiResult<Json<ChangeSet>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_work_product_change_set(&matter_id, &work_product_id, &change_set_id)
            .await?,
    ))
}

async fn list_work_product_snapshots(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<VersionSnapshot>>> {
    Ok(Json(
        state
            .casebuilder_service
            .list_work_product_snapshots(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn get_work_product_snapshot(
    State(state): State<AppState>,
    Path((matter_id, work_product_id, snapshot_id)): Path<(String, String, String)>,
) -> ApiResult<Json<VersionSnapshot>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_work_product_snapshot(&matter_id, &work_product_id, &snapshot_id)
            .await?,
    ))
}

async fn create_work_product_snapshot(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<CreateVersionSnapshotRequest>,
) -> ApiResult<Json<VersionSnapshot>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_work_product_snapshot(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn compare_work_product_snapshots(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Query(params): Query<CompareWorkProductParams>,
) -> ApiResult<Json<CompareVersionsResponse>> {
    let layers = params
        .layers
        .as_deref()
        .unwrap_or("text")
        .split(',')
        .map(str::trim)
        .filter(|layer| !layer.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    Ok(Json(
        state
            .casebuilder_service
            .compare_work_product_snapshots(
                &matter_id,
                &work_product_id,
                &params.from,
                params.to.as_deref(),
                layers,
            )
            .await?,
    ))
}

async fn restore_work_product_version(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<RestoreVersionRequest>,
) -> ApiResult<Json<RestoreVersionResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .restore_work_product_version(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn work_product_export_history(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<WorkProductArtifact>>> {
    Ok(Json(
        state
            .casebuilder_service
            .work_product_export_history(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn work_product_ai_audit(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<AIEditAudit>>> {
    Ok(Json(
        state
            .casebuilder_service
            .work_product_ai_audit(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn list_drafts(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<CaseDraft>>> {
    Ok(Json(
        state.casebuilder_service.list_drafts(&matter_id).await?,
    ))
}

async fn create_draft(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateDraftRequest>,
) -> ApiResult<Json<CaseDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_draft(&matter_id, request)
            .await?,
    ))
}

async fn get_draft(
    State(state): State<AppState>,
    Path((matter_id, draft_id)): Path<(String, String)>,
) -> ApiResult<Json<CaseDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_draft(&matter_id, &draft_id)
            .await?,
    ))
}

async fn patch_draft(
    State(state): State<AppState>,
    Path((matter_id, draft_id)): Path<(String, String)>,
    Json(request): Json<PatchDraftRequest>,
) -> ApiResult<Json<CaseDraft>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_draft(&matter_id, &draft_id, request)
            .await?,
    ))
}

async fn generate_draft(
    State(state): State<AppState>,
    Path((matter_id, draft_id)): Path<(String, String)>,
) -> ApiResult<Json<AiActionResponse<CaseDraft>>> {
    Ok(Json(
        state
            .casebuilder_service
            .generate_draft(&matter_id, &draft_id)
            .await?,
    ))
}

async fn fact_check_draft(
    State(state): State<AppState>,
    Path((matter_id, draft_id)): Path<(String, String)>,
) -> ApiResult<Json<AiActionResponse<Vec<FactCheckFinding>>>> {
    Ok(Json(
        state
            .casebuilder_service
            .fact_check_draft(&matter_id, &draft_id)
            .await?,
    ))
}

async fn citation_check_draft(
    State(state): State<AppState>,
    Path((matter_id, draft_id)): Path<(String, String)>,
) -> ApiResult<Json<AiActionResponse<Vec<CitationCheckFinding>>>> {
    Ok(Json(
        state
            .casebuilder_service
            .citation_check_draft(&matter_id, &draft_id)
            .await?,
    ))
}

async fn authority_search(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Query(params): Query<AuthoritySearchQuery>,
) -> ApiResult<Json<AuthoritySearchResponse>> {
    let search = state
        .search_service
        .search(SearchQuery {
            q: params.q.clone(),
            r#type: Some("all".to_string()),
            chapter: None,
            status: None,
            mode: Some(SearchMode::Auto),
            limit: params.limit.or(Some(10)),
            offset: Some(0),
            include: None,
            semantic_type: None,
            current_only: Some(true),
            source_backed: Some(true),
            has_citations: None,
            has_deadlines: None,
            has_penalties: None,
            needs_review: None,
        })
        .await?;

    Ok(Json(AuthoritySearchResponse {
        matter_id,
        query: params.q,
        source: "live_orsgraph".to_string(),
        warnings: search.warnings,
        results: search
            .results
            .into_iter()
            .map(|result| AuthoritySearchItem {
                canonical_id: result
                    .graph
                    .as_ref()
                    .and_then(|graph| graph.canonical_id.clone())
                    .or_else(|| result.citation.clone()),
                id: result.id,
                kind: result.kind,
                citation: result.citation,
                title: result.title,
                snippet: result.snippet,
                score: result.score,
                href: result.href,
            })
            .collect(),
    }))
}

async fn authority_recommend(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<AuthorityRecommendRequest>,
) -> ApiResult<Json<AuthoritySearchResponse>> {
    authority_search(
        State(state),
        Path(matter_id),
        Query(AuthoritySearchQuery {
            q: request.text,
            limit: request.limit,
        }),
    )
    .await
}

async fn authority_attach(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<AuthorityAttachmentRequest>,
) -> ApiResult<Json<AuthorityAttachmentResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .attach_authority(&matter_id, request)
            .await?,
    ))
}

async fn authority_detach(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<AuthorityAttachmentRequest>,
) -> ApiResult<Json<AuthorityAttachmentResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .detach_authority(&matter_id, request)
            .await?,
    ))
}

async fn export_not_ready(
    Path(matter_id): Path<String>,
) -> ApiResult<Json<AiActionResponse<serde_json::Value>>> {
    Err(ApiError::BadRequest(format!(
        "Export is deferred for CaseBuilder V0 matter {matter_id}; DOCX/PDF/filing packets are V0.2+."
    )))
}

fn parse_binary_upload(headers: &HeaderMap, body: Bytes) -> ApiResult<BinaryUploadRequest> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("Missing multipart Content-Type".to_string()))?;
    let boundary = multipart_boundary(content_type)
        .ok_or_else(|| ApiError::BadRequest("Missing multipart boundary".to_string()))?;
    let marker = format!("--{boundary}").into_bytes();
    let bytes = body.as_ref();
    let mut cursor = find_bytes(bytes, &marker, 0)
        .ok_or_else(|| ApiError::BadRequest("Multipart body missing boundary".to_string()))?;

    let mut filename = None;
    let mut file_mime_type = None;
    let mut file_bytes: Option<Bytes> = None;
    let mut document_type = None;
    let mut folder = None;
    let mut confidentiality = None;

    loop {
        cursor += marker.len();
        if bytes.get(cursor..cursor + 2) == Some(b"--") {
            break;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"\r\n") {
            cursor += 2;
        }
        let Some(next_boundary) = find_bytes(bytes, &marker, cursor) else {
            break;
        };
        let mut part = &bytes[cursor..next_boundary];
        if part.ends_with(b"\r\n") {
            part = &part[..part.len().saturating_sub(2)];
        }
        parse_multipart_part(
            part,
            &mut filename,
            &mut file_mime_type,
            &mut file_bytes,
            &mut document_type,
            &mut folder,
            &mut confidentiality,
        )?;
        cursor = next_boundary;
    }

    let filename = filename
        .ok_or_else(|| ApiError::BadRequest("Multipart upload missing file".to_string()))?;
    let bytes = file_bytes
        .ok_or_else(|| ApiError::BadRequest("Multipart upload missing bytes".to_string()))?;

    Ok(BinaryUploadRequest {
        filename,
        mime_type: file_mime_type,
        bytes,
        document_type,
        folder,
        confidentiality,
    })
}

fn parse_multipart_part(
    part: &[u8],
    filename: &mut Option<String>,
    file_mime_type: &mut Option<String>,
    file_bytes: &mut Option<Bytes>,
    document_type: &mut Option<String>,
    folder: &mut Option<String>,
    confidentiality: &mut Option<String>,
) -> ApiResult<()> {
    let (header_end, delimiter_len) = find_bytes(part, b"\r\n\r\n", 0)
        .map(|index| (index, 4))
        .or_else(|| find_bytes(part, b"\n\n", 0).map(|index| (index, 2)))
        .ok_or_else(|| ApiError::BadRequest("Malformed multipart part".to_string()))?;
    let header_text = String::from_utf8_lossy(&part[..header_end]);
    let body = &part[header_end + delimiter_len..];
    let mut name = None;
    let mut part_filename = None;
    let mut content_type = None;

    for line in header_text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("content-disposition") {
            name = disposition_param(value, "name");
            part_filename = disposition_param(value, "filename");
        } else if key.trim().eq_ignore_ascii_case("content-type") {
            content_type = Some(value.trim().to_string());
        }
    }

    match name.as_deref() {
        Some("file") => {
            *filename = Some(
                part_filename
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "upload.bin".to_string()),
            );
            *file_mime_type = content_type;
            *file_bytes = Some(Bytes::copy_from_slice(body));
        }
        Some("document_type") => *document_type = multipart_text(body),
        Some("folder") => *folder = multipart_text(body),
        Some("confidentiality") => *confidentiality = multipart_text(body),
        _ => {}
    }
    Ok(())
}

fn multipart_boundary(content_type: &str) -> Option<String> {
    content_type.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        if key.trim().eq_ignore_ascii_case("boundary") {
            Some(value.trim().trim_matches('"').to_string())
        } else {
            None
        }
    })
}

fn disposition_param(value: &str, name: &str) -> Option<String> {
    value.split(';').find_map(|part| {
        let (key, raw_value) = part.trim().split_once('=')?;
        if key.trim().eq_ignore_ascii_case(name) {
            Some(raw_value.trim().trim_matches('"').to_string())
        } else {
            None
        }
    })
}

fn multipart_text(bytes: &[u8]) -> Option<String> {
    String::from_utf8(bytes.to_vec())
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if needle.is_empty() || start > haystack.len() {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|position| position + start)
}
