use crate::auth::AuthContext;
use crate::error::{ApiError, ApiResult};
use crate::models::casebuilder::*;
use crate::models::search::{SearchMode, SearchQuery};
use crate::services::casebuilder::BinaryUploadRequest;
use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::DefaultBodyLimit,
    extract::{Extension, Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CompareWorkProductParams {
    from: String,
    to: Option<String>,
    layers: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ListWorkProductsParams {
    include: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/matters", get(list_matters).post(create_matter))
        .route("/admin/matters/claim-ownerless", post(claim_ownerless_matters))
        .route(
            "/matters/:matter_id",
            get(get_matter).patch(patch_matter).delete(delete_matter),
        )
        .route("/matters/:matter_id/graph", get(get_matter_graph))
        .route("/matters/:matter_id/audit", get(list_matter_audit_events))
        .route("/matters/:matter_id/index", get(get_matter_index))
        .route("/matters/:matter_id/index/run", post(run_matter_index))
        .route("/matters/:matter_id/qc/run", post(run_matter_qc))
        .route("/matters/:matter_id/issues/spot", post(spot_issues))
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
            "/matters/:matter_id/documents/:document_id/workspace",
            get(get_document_workspace),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/content",
            get(get_document_content),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/annotations",
            get(list_document_annotations).post(create_document_annotation),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/text",
            patch(save_document_text),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/promote-work-product",
            post(promote_document_work_product),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/download-url",
            post(create_download_url),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/transcriptions",
            get(list_transcriptions).post(create_transcription),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/transcriptions/:transcription_job_id",
            get(get_transcription),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/transcriptions/:transcription_job_id/sync",
            post(sync_transcription),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/transcriptions/:transcription_job_id/segments/:segment_id",
            patch(patch_transcript_segment),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/transcriptions/:transcription_job_id/speakers/:speaker_id",
            patch(patch_transcript_speaker),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/transcriptions/:transcription_job_id/review",
            post(review_transcription),
        )
        .route(
            "/matters/:matter_id/documents/:document_id/extract",
            post(extract_document),
        )
        .route(
            "/casebuilder/webhooks/assemblyai",
            post(assemblyai_webhook),
        )
        .route(
            "/casebuilder/providers/assemblyai/transcripts",
            get(list_assemblyai_transcripts),
        )
        .route(
            "/casebuilder/providers/assemblyai/transcripts/:transcript_id",
            delete(delete_assemblyai_transcript),
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
        .route("/matters/:matter_id/ask", post(ask_matter))
        .route(
            "/matters/:matter_id/deadlines",
            get(list_deadlines).post(create_deadline),
        )
        .route(
            "/matters/:matter_id/deadlines/compute",
            post(compute_deadlines),
        )
        .route(
            "/matters/:matter_id/deadlines/:deadline_id",
            patch(patch_deadline),
        )
        .route(
            "/matters/:matter_id/tasks",
            get(list_tasks).post(create_task),
        )
        .route("/matters/:matter_id/tasks/:task_id", patch(patch_task))
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
            "/matters/:matter_id/work-products/:work_product_id/links/:anchor_id",
            patch(patch_work_product_support).delete(delete_work_product_support),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/text-ranges",
            post(link_work_product_text_range),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/ast",
            get(get_work_product_ast).patch(patch_work_product_ast),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/ast/patch",
            post(apply_work_product_ast_patch),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/ast/validate",
            post(validate_work_product_ast),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/ast/to-markdown",
            post(work_product_ast_to_markdown),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/ast/from-markdown",
            post(work_product_ast_from_markdown),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/ast/to-html",
            post(work_product_ast_to_html),
        )
        .route(
            "/matters/:matter_id/work-products/:work_product_id/ast/to-plain-text",
            post(work_product_ast_to_plain_text),
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
        .route("/matters/:matter_id/export/docx", post(export_matter_docx))
        .route("/matters/:matter_id/export/pdf", post(export_matter_pdf))
        .route(
            "/matters/:matter_id/export/filing-packet",
            post(export_matter_filing_packet),
        )
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
}

async fn list_matters(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> ApiResult<Json<Vec<MatterSummary>>> {
    Ok(Json(
        state
            .casebuilder_service
            .list_matters_for_auth(&auth)
            .await?,
    ))
}

async fn create_matter(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<CreateMatterRequest>,
) -> ApiResult<Json<MatterBundle>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_matter(request, &auth)
            .await?,
    ))
}

async fn claim_ownerless_matters(
    State(state): State<AppState>,
    Json(request): Json<ClaimOwnerlessMattersRequest>,
) -> ApiResult<Json<Vec<MatterSummary>>> {
    Ok(Json(
        state
            .casebuilder_service
            .claim_ownerless_matters(request)
            .await?,
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

async fn get_matter_graph(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<CaseGraphResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_matter_graph(&matter_id)
            .await?,
    ))
}

async fn list_matter_audit_events(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<Vec<AuditEvent>>> {
    Ok(Json(
        state
            .casebuilder_service
            .list_matter_audit_events(&matter_id)
            .await?,
    ))
}

async fn run_matter_qc(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<QcRun>> {
    Ok(Json(
        state.casebuilder_service.run_matter_qc(&matter_id).await?,
    ))
}

async fn spot_issues(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<IssueSpotRequest>,
) -> ApiResult<Json<IssueSpotResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .spot_issues(&matter_id, request)
            .await?,
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

async fn get_matter_index(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<MatterIndexSummary>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_matter_index_summary(&matter_id)
            .await?,
    ))
}

async fn run_matter_index(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<RunMatterIndexRequest>,
) -> ApiResult<Json<MatterIndexRunResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .run_matter_index(&matter_id, request)
            .await?,
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

async fn get_document_workspace(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
) -> ApiResult<Json<DocumentWorkspace>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_document_workspace(&matter_id, &document_id)
            .await?,
    ))
}

async fn get_document_content(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    let (document, bytes) = state
        .casebuilder_service
        .get_document_content_bytes(&matter_id, &document_id)
        .await?;
    let mut headers = HeaderMap::new();
    if let Some(mime_type) = document.mime_type.as_deref() {
        if let Ok(value) = HeaderValue::from_str(mime_type) {
            headers.insert(header::CONTENT_TYPE, value);
        }
    }
    let disposition = format!(
        "inline; filename=\"{}\"",
        document.filename.replace(['"', '\r', '\n'], "_")
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    Ok((headers, bytes).into_response())
}

async fn list_document_annotations(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<DocumentAnnotation>>> {
    Ok(Json(
        state
            .casebuilder_service
            .list_document_annotations(&matter_id, &document_id)
            .await?,
    ))
}

async fn create_document_annotation(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
    Json(request): Json<UpsertDocumentAnnotationRequest>,
) -> ApiResult<Json<DocumentAnnotation>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_document_annotation(&matter_id, &document_id, request)
            .await?,
    ))
}

async fn save_document_text(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
    Json(request): Json<SaveDocumentTextRequest>,
) -> ApiResult<Json<SaveDocumentTextResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .save_document_text(&matter_id, &document_id, request)
            .await?,
    ))
}

async fn promote_document_work_product(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
    Json(request): Json<PromoteDocumentWorkProductRequest>,
) -> ApiResult<Json<PromoteDocumentWorkProductResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .promote_document_work_product(&matter_id, &document_id, request)
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

async fn create_transcription(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
    Json(request): Json<CreateTranscriptionRequest>,
) -> ApiResult<Json<TranscriptionJobResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_transcription(&matter_id, &document_id, request)
            .await?,
    ))
}

async fn list_transcriptions(
    State(state): State<AppState>,
    Path((matter_id, document_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<TranscriptionJobResponse>>> {
    Ok(Json(
        state
            .casebuilder_service
            .list_transcriptions(&matter_id, &document_id)
            .await?,
    ))
}

async fn list_assemblyai_transcripts(
    State(state): State<AppState>,
    Query(query): Query<AssemblyAiTranscriptListQuery>,
) -> ApiResult<Json<AssemblyAiTranscriptListResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .list_assemblyai_transcripts(query)
            .await?,
    ))
}

async fn delete_assemblyai_transcript(
    State(state): State<AppState>,
    Path(transcript_id): Path<String>,
) -> ApiResult<Json<AssemblyAiTranscriptDeleteResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .delete_assemblyai_transcript(&transcript_id)
            .await?,
    ))
}

async fn get_transcription(
    State(state): State<AppState>,
    Path((matter_id, document_id, transcription_job_id)): Path<(String, String, String)>,
) -> ApiResult<Json<TranscriptionJobResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_transcription(&matter_id, &document_id, &transcription_job_id)
            .await?,
    ))
}

async fn sync_transcription(
    State(state): State<AppState>,
    Path((matter_id, document_id, transcription_job_id)): Path<(String, String, String)>,
) -> ApiResult<Json<TranscriptionJobResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .sync_transcription(&matter_id, &document_id, &transcription_job_id)
            .await?,
    ))
}

async fn patch_transcript_segment(
    State(state): State<AppState>,
    Path((matter_id, document_id, transcription_job_id, segment_id)): Path<(
        String,
        String,
        String,
        String,
    )>,
    Json(request): Json<PatchTranscriptSegmentRequest>,
) -> ApiResult<Json<TranscriptionJobResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_transcript_segment(
                &matter_id,
                &document_id,
                &transcription_job_id,
                &segment_id,
                request,
            )
            .await?,
    ))
}

async fn patch_transcript_speaker(
    State(state): State<AppState>,
    Path((matter_id, document_id, transcription_job_id, speaker_id)): Path<(
        String,
        String,
        String,
        String,
    )>,
    Json(request): Json<PatchTranscriptSpeakerRequest>,
) -> ApiResult<Json<TranscriptionJobResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_transcript_speaker(
                &matter_id,
                &document_id,
                &transcription_job_id,
                &speaker_id,
                request,
            )
            .await?,
    ))
}

async fn review_transcription(
    State(state): State<AppState>,
    Path((matter_id, document_id, transcription_job_id)): Path<(String, String, String)>,
    Json(request): Json<ReviewTranscriptionRequest>,
) -> ApiResult<Json<TranscriptionJobResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .review_transcription(&matter_id, &document_id, &transcription_job_id, request)
            .await?,
    ))
}

async fn assemblyai_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AssemblyAiWebhookPayload>,
) -> ApiResult<Json<TranscriptionWebhookResponse>> {
    let header_value = headers
        .get("x-casebuilder-assemblyai-secret")
        .and_then(|value| value.to_str().ok());
    Ok(Json(
        state
            .casebuilder_service
            .handle_assemblyai_webhook(header_value, payload)
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

async fn create_deadline(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateDeadlineRequest>,
) -> ApiResult<Json<CaseDeadline>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_deadline(&matter_id, request)
            .await?,
    ))
}

async fn patch_deadline(
    State(state): State<AppState>,
    Path((matter_id, deadline_id)): Path<(String, String)>,
    Json(request): Json<PatchDeadlineRequest>,
) -> ApiResult<Json<CaseDeadline>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_deadline(&matter_id, &deadline_id, request)
            .await?,
    ))
}

async fn compute_deadlines(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<ComputeDeadlinesResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .compute_deadlines(&matter_id)
            .await?,
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

async fn create_task(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<CreateTaskRequest>,
) -> ApiResult<Json<CaseTask>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_task(&matter_id, request)
            .await?,
    ))
}

async fn patch_task(
    State(state): State<AppState>,
    Path((matter_id, task_id)): Path<(String, String)>,
    Json(request): Json<PatchTaskRequest>,
) -> ApiResult<Json<CaseTask>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_task(&matter_id, &task_id, request)
            .await?,
    ))
}

async fn ask_matter(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
    Json(request): Json<MatterAskRequest>,
) -> ApiResult<Json<MatterAskResponse>> {
    let matter = state.casebuilder_service.get_matter(&matter_id).await?;
    let scope = request.scope.as_deref().unwrap_or("all");
    let include_documents = matches!(scope, "all" | "documents");
    let include_facts = matches!(scope, "all" | "facts" | "claims");
    let include_authority = matches!(scope, "all" | "claims" | "authority" | "authorities");
    let needle = request.question.to_lowercase();
    let terms: Vec<&str> = needle
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|term| term.len() > 3)
        .take(8)
        .collect();

    let mut related_facts: Vec<CaseFact> = if include_facts {
        matter
            .facts
            .iter()
            .filter(|fact| {
                let text = fact.statement.to_lowercase();
                terms.iter().any(|term| text.contains(term))
            })
            .take(5)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    if scope == "claims" {
        let matched_fact_ids = matter
            .claims
            .iter()
            .filter(|claim| {
                let text = format!(
                    "{} {} {}",
                    claim.title, claim.claim_type, claim.legal_theory
                )
                .to_lowercase();
                terms.iter().any(|term| text.contains(term))
            })
            .flat_map(|claim| claim.fact_ids.clone())
            .collect::<std::collections::HashSet<_>>();
        for fact in &matter.facts {
            if matched_fact_ids.contains(&fact.fact_id)
                && !related_facts
                    .iter()
                    .any(|existing| existing.fact_id == fact.fact_id)
            {
                related_facts.push(fact.clone());
            }
        }
        related_facts.truncate(5);
    }
    let related_documents: Vec<CaseDocument> = if include_documents {
        matter
            .documents
            .iter()
            .filter(|document| {
                let text = format!(
                    "{} {} {}",
                    document.title, document.filename, document.summary
                )
                .to_lowercase();
                terms.iter().any(|term| text.contains(term))
            })
            .take(5)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    let mut citations = Vec::new();
    for (index, document) in related_documents.iter().enumerate() {
        citations.push(MatterAskCitation {
            citation_id: format!("matter-doc-{}", index + 1),
            kind: "document".to_string(),
            source_id: document.document_id.clone(),
            title: document.title.clone(),
            snippet: Some(document.summary.clone()),
        });
    }
    for (index, fact) in related_facts.iter().enumerate() {
        citations.push(MatterAskCitation {
            citation_id: format!("matter-fact-{}", index + 1),
            kind: "fact".to_string(),
            source_id: fact.fact_id.clone(),
            title: fact.statement.clone(),
            snippet: fact.notes.clone(),
        });
    }
    let mut authority_count = 0usize;
    let mut top_authority: Option<(String, String)> = None;
    let mut warnings = Vec::new();
    if include_authority {
        let search = state
            .search_service
            .search(SearchQuery {
                q: request.question.clone(),
                r#type: Some("all".to_string()),
                authority_family: None,
                authority_tier: None,
                jurisdiction: None,
                source_role: None,
                chapter: None,
                status: None,
                mode: Some(SearchMode::Auto),
                limit: Some(5),
                offset: Some(0),
                include: None,
                semantic_type: None,
                current_only: Some(true),
                source_backed: Some(true),
                has_citations: None,
                has_deadlines: None,
                has_penalties: None,
                needs_review: None,
                primary_law: None,
                official_commentary: None,
            })
            .await?;
        authority_count = search.results.len();
        if let Some(top) = search.results.first() {
            top_authority = Some((
                top.citation.clone().unwrap_or_else(|| top.id.clone()),
                top.snippet.clone(),
            ));
        }
        for (index, result) in search.results.iter().take(5).enumerate() {
            citations.push(MatterAskCitation {
                citation_id: format!("authority-{}", index + 1),
                kind: "statute".to_string(),
                source_id: result
                    .graph
                    .as_ref()
                    .and_then(|graph| graph.canonical_id.clone())
                    .unwrap_or_else(|| result.id.clone()),
                title: result
                    .citation
                    .clone()
                    .or_else(|| result.title.clone())
                    .unwrap_or_else(|| result.id.clone()),
                snippet: Some(result.snippet.clone()),
            });
        }
        warnings = search.warnings;
    }

    let source_spans = related_facts
        .iter()
        .flat_map(|fact| fact.source_spans.clone())
        .chain(
            related_documents
                .iter()
                .flat_map(|document| document.source_spans.clone()),
        )
        .take(8)
        .collect();

    let answer = if related_facts.is_empty() && related_documents.is_empty() && authority_count == 0
    {
        "I could not find matter records or source-backed authorities that match that question."
            .to_string()
    } else {
        let mut parts = Vec::new();
        if !related_facts.is_empty() {
            parts.push(format!(
                "Found {} matching matter fact{}.",
                related_facts.len(),
                if related_facts.len() == 1 { "" } else { "s" }
            ));
        }
        if !related_documents.is_empty() {
            parts.push(format!(
                "Found {} matching document{}.",
                related_documents.len(),
                if related_documents.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }
        if let Some((citation, snippet)) = top_authority {
            parts.push(format!(
                "The strongest source-backed authority match is {}: {}",
                citation, snippet
            ));
        }
        parts.join(" ")
    };

    warnings.push(
        format!("Matter ask used '{}' scope in provider-free retrieval mode; verify legal conclusions before filing.", scope),
    );

    Ok(Json(MatterAskResponse {
        answer,
        citations,
        source_spans,
        related_facts,
        related_documents,
        warnings,
        mode: "retrieval".to_string(),
        thread_id: request.thread_id,
    }))
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
    Query(params): Query<ListWorkProductsParams>,
) -> ApiResult<Json<Vec<WorkProduct>>> {
    let include_ast = params
        .include
        .as_deref()
        .map(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("document_ast"))
        })
        .unwrap_or(false);
    Ok(Json(
        state
            .casebuilder_service
            .list_work_products_for_api(&matter_id, include_ast)
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

async fn patch_work_product_support(
    State(state): State<AppState>,
    Path((matter_id, work_product_id, anchor_id)): Path<(String, String, String)>,
    Json(request): Json<PatchWorkProductSupportRequest>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_work_product_support(&matter_id, &work_product_id, &anchor_id, request)
            .await?,
    ))
}

async fn delete_work_product_support(
    State(state): State<AppState>,
    Path((matter_id, work_product_id, anchor_id)): Path<(String, String, String)>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .delete_work_product_support(&matter_id, &work_product_id, &anchor_id)
            .await?,
    ))
}

async fn link_work_product_text_range(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<WorkProductTextRangeLinkRequest>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .link_work_product_text_range(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn apply_work_product_ast_patch(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<AstPatch>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .apply_work_product_ast_patch(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn get_work_product_ast(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<WorkProductDocument>> {
    Ok(Json(
        state
            .casebuilder_service
            .get_work_product_ast(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn patch_work_product_ast(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<WorkProductDocument>,
) -> ApiResult<Json<WorkProduct>> {
    Ok(Json(
        state
            .casebuilder_service
            .patch_work_product_ast(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn validate_work_product_ast(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<AstValidationResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .validate_work_product_ast(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn work_product_ast_to_markdown(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<AstMarkdownResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .work_product_ast_to_markdown(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn work_product_ast_from_markdown(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
    Json(request): Json<MarkdownToAstRequest>,
) -> ApiResult<Json<AstDocumentResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .work_product_ast_from_markdown(&matter_id, &work_product_id, request)
            .await?,
    ))
}

async fn work_product_ast_to_html(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<AstRenderedResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .work_product_ast_to_html(&matter_id, &work_product_id)
            .await?,
    ))
}

async fn work_product_ast_to_plain_text(
    State(state): State<AppState>,
    Path((matter_id, work_product_id)): Path<(String, String)>,
) -> ApiResult<Json<AstRenderedResponse>> {
    Ok(Json(
        state
            .casebuilder_service
            .work_product_ast_to_plain_text(&matter_id, &work_product_id)
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
            .list_work_product_snapshots_for_api(&matter_id, &work_product_id)
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
        .unwrap_or("all")
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
            authority_family: params.authority_family.clone(),
            authority_tier: None,
            jurisdiction: None,
            source_role: None,
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
            primary_law: None,
            official_commentary: None,
        })
        .await?;

    Ok(Json(AuthoritySearchResponse {
        matter_id,
        query: params.q,
        source: "orsgraph".to_string(),
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
                authority_family: result.authority_family,
                authority_level: result.authority_level,
                authority_tier: result.authority_tier,
                source_role: result.source_role,
                primary_law: result.primary_law,
                official_commentary: result.official_commentary,
                controlling_weight: result.controlling_weight,
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
            authority_family: None,
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

async fn export_matter_docx(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<AiActionResponse<ExportPackage>>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_matter_export_package(&matter_id, "docx")
            .await?,
    ))
}

async fn export_matter_pdf(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<AiActionResponse<ExportPackage>>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_matter_export_package(&matter_id, "pdf")
            .await?,
    ))
}

async fn export_matter_filing_packet(
    State(state): State<AppState>,
    Path(matter_id): Path<String>,
) -> ApiResult<Json<AiActionResponse<ExportPackage>>> {
    Ok(Json(
        state
            .casebuilder_service
            .create_matter_export_package(&matter_id, "filing_packet")
            .await?,
    ))
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
    let mut relative_path = None;
    let mut upload_batch_id = None;

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
            &mut relative_path,
            &mut upload_batch_id,
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
        relative_path,
        upload_batch_id,
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
    relative_path: &mut Option<String>,
    upload_batch_id: &mut Option<String>,
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
        Some("relative_path") => *relative_path = multipart_text(body),
        Some("upload_batch_id") => *upload_batch_id = multipart_text(body),
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
