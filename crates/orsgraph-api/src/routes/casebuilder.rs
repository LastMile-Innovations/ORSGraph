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
            mode: Some(SearchMode::Hybrid),
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
