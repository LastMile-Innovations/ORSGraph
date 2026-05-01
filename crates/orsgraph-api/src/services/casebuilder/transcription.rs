use super::*;

impl CaseBuilderService {
    pub async fn create_transcription(
        &self,
        matter_id: &str,
        document_id: &str,
        request: CreateTranscriptionRequest,
    ) -> ApiResult<TranscriptionJobResponse> {
        let mut document = self.get_document(matter_id, document_id).await?;
        if !document_is_media(&document) {
            return Err(ApiError::BadRequest(
                "Only uploaded audio/video documents can be transcribed.".to_string(),
            ));
        }
        let force = request.force.unwrap_or(false);
        if !force {
            if let Some(existing) = self
                .list_transcription_jobs(matter_id, document_id)
                .await?
                .into_iter()
                .rev()
                .find(|job| {
                    !matches!(
                        job.status.as_str(),
                        "failed" | "provider_disabled" | "cancelled"
                    )
                })
            {
                return self.transcription_response(matter_id, &existing).await;
            }
        }

        let provenance = self
            .ensure_document_original_provenance(matter_id, &mut document)
            .await?;
        let now = now_string();
        let speech_models = assemblyai_speech_models();
        let new_job_id = transcription_job_id(document_id, now_secs());
        let mut job = TranscriptionJob {
            transcription_job_id: new_job_id.clone(),
            id: new_job_id,
            matter_id: matter_id.to_string(),
            document_id: document_id.to_string(),
            document_version_id: provenance
                .as_ref()
                .map(|provenance| provenance.document_version.document_version_id.clone())
                .or_else(|| document.current_version_id.clone()),
            object_blob_id: provenance
                .as_ref()
                .map(|provenance| provenance.object_blob.object_blob_id.clone())
                .or_else(|| document.object_blob_id.clone()),
            provider: "assemblyai".to_string(),
            provider_mode: if self.assemblyai.enabled && self.assemblyai.api_key.is_some() {
                "live".to_string()
            } else {
                "disabled".to_string()
            },
            provider_transcript_id: None,
            provider_status: None,
            status: "queued".to_string(),
            review_status: "not_started".to_string(),
            raw_artifact_version_id: None,
            normalized_artifact_version_id: None,
            redacted_artifact_version_id: None,
            reviewed_document_version_id: None,
            caption_vtt_version_id: None,
            caption_srt_version_id: None,
            language_code: request.language_code.clone(),
            duration_ms: None,
            speaker_count: 0,
            segment_count: 0,
            word_count: 0,
            redact_pii: request.redact_pii.unwrap_or(true),
            speech_models,
            retryable: false,
            error_code: None,
            error_message: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            completed_at: None,
            reviewed_at: None,
        };
        job.id = job.transcription_job_id.clone();

        if job.provider_mode == "disabled" {
            job.status = "provider_disabled".to_string();
            job.error_code = Some("assemblyai_disabled".to_string());
            job.error_message =
                Some("AssemblyAI transcription is disabled or missing an API key.".to_string());
            let job = self.merge_transcription_job(matter_id, &job).await?;
            return self.transcription_response(matter_id, &job).await;
        }

        if document.bytes > self.assemblyai.max_media_bytes {
            job.status = "failed".to_string();
            job.error_code = Some("media_too_large".to_string());
            job.error_message = Some("Media exceeds configured AssemblyAI size limit.".to_string());
            let job = self.merge_transcription_job(matter_id, &job).await?;
            return self.transcription_response(matter_id, &job).await;
        }

        let media_bytes = self.document_bytes(&document).await?;
        document.processing_status = "processing".to_string();
        document.summary = "Transcription submitted to AssemblyAI; review is required before transcript-derived facts or evidence are created.".to_string();
        self.merge_node(matter_id, document_spec(), document_id, &document)
            .await?;
        job.status = "processing".to_string();
        job.provider_status = Some("submitted".to_string());
        job.retryable = true;
        let job = self.merge_transcription_job(matter_id, &job).await?;

        let submit_result = async {
            let upload_url = self
                .assemblyai_upload_bytes(document.mime_type.clone(), media_bytes)
                .await?;
            self.assemblyai_submit_transcript(&upload_url, &request, &job)
                .await
        }
        .await;

        match submit_result {
            Ok(provider) => {
                let mut job = job;
                job.provider_transcript_id = Some(provider.id.clone());
                job.provider_status = Some(provider.status.clone());
                job.updated_at = now_string();
                if provider.status == "completed" {
                    self.import_completed_transcript(matter_id, document, job, provider)
                        .await
                } else if provider.status == "error" {
                    job.status = "failed".to_string();
                    job.retryable = false;
                    job.error_code = Some("assemblyai_error".to_string());
                    job.error_message = Some(assemblyai_transcript_error_message(&provider));
                    let job = self.merge_transcription_job(matter_id, &job).await?;
                    self.transcription_response(matter_id, &job).await
                } else {
                    let job = self.merge_transcription_job(matter_id, &job).await?;
                    self.transcription_response(matter_id, &job).await
                }
            }
            Err(error) => {
                let mut job = job;
                job.status = "failed".to_string();
                job.provider_status = Some("request_failed".to_string());
                job.error_code = Some("assemblyai_request_failed".to_string());
                job.error_message = Some(sanitized_external_error(&error));
                job.updated_at = now_string();
                let job = self.merge_transcription_job(matter_id, &job).await?;
                self.transcription_response(matter_id, &job).await
            }
        }
    }

    pub async fn list_transcriptions(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<Vec<TranscriptionJobResponse>> {
        self.get_document(matter_id, document_id).await?;
        let mut responses = Vec::new();
        for job in self.list_transcription_jobs(matter_id, document_id).await? {
            responses.push(self.transcription_response(matter_id, &job).await?);
        }
        Ok(responses)
    }

    pub async fn get_transcription(
        &self,
        matter_id: &str,
        document_id: &str,
        transcription_job_id: &str,
    ) -> ApiResult<TranscriptionJobResponse> {
        let job = self
            .get_node::<TranscriptionJob>(matter_id, transcription_job_spec(), transcription_job_id)
            .await?;
        if job.document_id != document_id {
            return Err(ApiError::NotFound(
                "Transcription job not found".to_string(),
            ));
        }
        self.transcription_response(matter_id, &job).await
    }

    pub async fn sync_transcription(
        &self,
        matter_id: &str,
        document_id: &str,
        transcription_job_id: &str,
    ) -> ApiResult<TranscriptionJobResponse> {
        let document = self.get_document(matter_id, document_id).await?;
        let mut job = self
            .get_node::<TranscriptionJob>(matter_id, transcription_job_spec(), transcription_job_id)
            .await?;
        if job.document_id != document_id {
            return Err(ApiError::NotFound(
                "Transcription job not found".to_string(),
            ));
        }
        if matches!(job.status.as_str(), "review_ready" | "processed")
            && job.normalized_artifact_version_id.is_some()
        {
            return self.transcription_response(matter_id, &job).await;
        }
        let Some(provider_transcript_id) = job.provider_transcript_id.clone() else {
            return self.transcription_response(matter_id, &job).await;
        };
        if !self.assemblyai.enabled || self.assemblyai.api_key.is_none() {
            job.status = "provider_disabled".to_string();
            job.error_code = Some("assemblyai_disabled".to_string());
            job.error_message =
                Some("AssemblyAI transcription is disabled or missing an API key.".to_string());
            job.updated_at = now_string();
            let job = self.merge_transcription_job(matter_id, &job).await?;
            return self.transcription_response(matter_id, &job).await;
        }
        match self
            .assemblyai_fetch_transcript(&provider_transcript_id)
            .await
        {
            Ok(provider) if provider.status == "completed" => {
                self.import_completed_transcript(matter_id, document, job, provider)
                    .await
            }
            Ok(provider) if provider.status == "error" => {
                job.status = "failed".to_string();
                job.provider_status = Some(provider.status.clone());
                job.retryable = false;
                job.error_code = Some("assemblyai_error".to_string());
                job.error_message = Some(assemblyai_transcript_error_message(&provider));
                job.updated_at = now_string();
                let job = self.merge_transcription_job(matter_id, &job).await?;
                self.transcription_response(matter_id, &job).await
            }
            Ok(provider) => {
                job.status = "processing".to_string();
                job.provider_status = Some(provider.status);
                job.retryable = true;
                job.updated_at = now_string();
                let job = self.merge_transcription_job(matter_id, &job).await?;
                self.transcription_response(matter_id, &job).await
            }
            Err(error) => {
                job.status = "failed".to_string();
                job.provider_status = Some("sync_failed".to_string());
                job.retryable = true;
                job.error_code = Some("assemblyai_sync_failed".to_string());
                job.error_message = Some(sanitized_external_error(&error));
                job.updated_at = now_string();
                let job = self.merge_transcription_job(matter_id, &job).await?;
                self.transcription_response(matter_id, &job).await
            }
        }
    }

    pub async fn patch_transcript_segment(
        &self,
        matter_id: &str,
        document_id: &str,
        transcription_job_id: &str,
        segment_id: &str,
        request: PatchTranscriptSegmentRequest,
    ) -> ApiResult<TranscriptionJobResponse> {
        let job = self
            .get_node::<TranscriptionJob>(matter_id, transcription_job_spec(), transcription_job_id)
            .await?;
        if job.document_id != document_id {
            return Err(ApiError::NotFound(
                "Transcription job not found".to_string(),
            ));
        }
        let mut segment = self
            .get_node::<TranscriptSegment>(matter_id, transcript_segment_spec(), segment_id)
            .await?;
        if segment.transcription_job_id != transcription_job_id
            || segment.document_id != document_id
        {
            return Err(ApiError::NotFound(
                "Transcript segment not found".to_string(),
            ));
        }
        let now = now_string();
        if let Some(text) = request.text {
            if text != segment.text {
                self.record_transcript_review_change(
                    matter_id,
                    document_id,
                    transcription_job_id,
                    "segment",
                    segment_id,
                    "text",
                    Some(segment.text.clone()),
                    Some(text.clone()),
                )
                .await?;
                segment.text = text;
                segment.edited = true;
            }
        }
        if let Some(redacted_text) = request.redacted_text {
            segment.redacted_text = Some(redacted_text);
            segment.edited = true;
        }
        if let Some(speaker_label) = request.speaker_label {
            segment.speaker_label = Some(speaker_label);
            segment.edited = true;
        }
        if let Some(review_status) = request.review_status {
            segment.review_status = review_status;
        }
        segment.updated_at = now;
        self.merge_transcript_segment(matter_id, &segment).await?;
        self.transcription_response(matter_id, &job).await
    }

    pub async fn patch_transcript_speaker(
        &self,
        matter_id: &str,
        document_id: &str,
        transcription_job_id: &str,
        speaker_id: &str,
        request: PatchTranscriptSpeakerRequest,
    ) -> ApiResult<TranscriptionJobResponse> {
        let job = self
            .get_node::<TranscriptionJob>(matter_id, transcription_job_spec(), transcription_job_id)
            .await?;
        if job.document_id != document_id {
            return Err(ApiError::NotFound(
                "Transcription job not found".to_string(),
            ));
        }
        let mut speaker = self
            .get_node::<TranscriptSpeaker>(matter_id, transcript_speaker_spec(), speaker_id)
            .await?;
        if speaker.transcription_job_id != transcription_job_id
            || speaker.document_id != document_id
        {
            return Err(ApiError::NotFound(
                "Transcript speaker not found".to_string(),
            ));
        }
        if request.display_name != speaker.display_name {
            self.record_transcript_review_change(
                matter_id,
                document_id,
                transcription_job_id,
                "speaker",
                speaker_id,
                "display_name",
                speaker.display_name.clone(),
                request.display_name.clone(),
            )
            .await?;
            speaker.display_name = request.display_name;
        }
        if request.role != speaker.role {
            speaker.role = request.role;
        }
        speaker.updated_at = now_string();
        self.merge_transcript_speaker(matter_id, &speaker).await?;
        self.transcription_response(matter_id, &job).await
    }

    pub async fn review_transcription(
        &self,
        matter_id: &str,
        document_id: &str,
        transcription_job_id: &str,
        request: ReviewTranscriptionRequest,
    ) -> ApiResult<TranscriptionJobResponse> {
        let mut document = self.get_document(matter_id, document_id).await?;
        let mut job = self
            .get_node::<TranscriptionJob>(matter_id, transcription_job_spec(), transcription_job_id)
            .await?;
        if job.document_id != document_id {
            return Err(ApiError::NotFound(
                "Transcription job not found".to_string(),
            ));
        }
        let mut segments = self
            .list_transcript_segments(matter_id, transcription_job_id)
            .await?;
        if segments.is_empty() {
            return Err(ApiError::BadRequest(
                "No transcript segments are available to review.".to_string(),
            ));
        }
        let reviewed_text = request
            .reviewed_text
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| transcript_segments_to_text(&segments, false));
        let reviewed_version = self
            .store_document_artifact_version(
                matter_id,
                &document,
                Bytes::from(reviewed_text.clone().into_bytes()),
                Some("text/plain".to_string()),
                "transcript_reviewed_text",
                "transcript_reviewed_text",
                "txt",
                true,
            )
            .await?;
        document.processing_status = "processed".to_string();
        document.summary = summarize_text(&reviewed_text);
        document.extracted_text = Some(reviewed_text);
        document.current_version_id = Some(reviewed_version.document_version_id.clone());
        document.object_blob_id = Some(reviewed_version.object_blob_id.clone());
        document.source_spans = transcript_source_spans(
            matter_id,
            document_id,
            transcription_job_id,
            &segments,
            job.document_version_id.clone(),
            job.object_blob_id.clone(),
            "approved",
        );
        let document = self
            .merge_node(matter_id, document_spec(), document_id, &document)
            .await?;
        for span in &document.source_spans {
            self.merge_source_span(matter_id, span).await?;
            self.link_transcription_job_to_source_span(&job, span)
                .await?;
        }
        for segment in &mut segments {
            segment.review_status = request
                .status
                .clone()
                .unwrap_or_else(|| "approved".to_string());
            segment.updated_at = now_string();
            self.merge_transcript_segment(matter_id, segment).await?;
        }
        job.status = "processed".to_string();
        job.review_status = "approved".to_string();
        job.reviewed_document_version_id = Some(reviewed_version.document_version_id);
        job.updated_at = now_string();
        job.reviewed_at = Some(job.updated_at.clone());
        job.retryable = false;
        job.error_code = None;
        job.error_message = None;
        let job = self.merge_transcription_job(matter_id, &job).await?;
        self.record_transcript_review_change(
            matter_id,
            document_id,
            transcription_job_id,
            "job",
            transcription_job_id,
            "review_status",
            Some("review_ready".to_string()),
            Some("approved".to_string()),
        )
        .await?;
        let _ = document;
        self.transcription_response(matter_id, &job).await
    }

    pub async fn handle_assemblyai_webhook(
        &self,
        header_value: Option<&str>,
        payload: AssemblyAiWebhookPayload,
    ) -> ApiResult<TranscriptionWebhookResponse> {
        let Some(secret) = self.assemblyai.webhook_secret.as_deref() else {
            return Err(ApiError::BadRequest(
                "AssemblyAI webhook secret is not configured.".to_string(),
            ));
        };
        if header_value != Some(secret) {
            return Err(ApiError::BadRequest(
                "Invalid AssemblyAI webhook authentication.".to_string(),
            ));
        }
        let Some(mut job) = self
            .find_transcription_job_by_provider_id(&payload.transcript_id)
            .await?
        else {
            return Ok(TranscriptionWebhookResponse {
                handled: false,
                message: "No matching transcription job.".to_string(),
                transcription: None,
            });
        };
        job.provider_status = payload.status.clone();
        job.updated_at = now_string();
        let job = self
            .merge_transcription_job(&job.matter_id.clone(), &job)
            .await?;
        let transcription = if payload.status.as_deref() == Some("completed") {
            Some(
                self.sync_transcription(
                    &job.matter_id,
                    &job.document_id,
                    &job.transcription_job_id,
                )
                .await?,
            )
        } else {
            Some(self.transcription_response(&job.matter_id, &job).await?)
        };
        Ok(TranscriptionWebhookResponse {
            handled: true,
            message: "AssemblyAI webhook recorded.".to_string(),
            transcription,
        })
    }

    pub(super) async fn list_transcription_jobs(
        &self,
        matter_id: &str,
        document_id: &str,
    ) -> ApiResult<Vec<TranscriptionJob>> {
        let mut jobs = self
            .list_nodes::<TranscriptionJob>(matter_id, transcription_job_spec())
            .await?
            .into_iter()
            .filter(|job| job.document_id == document_id)
            .collect::<Vec<_>>();
        jobs.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(jobs)
    }

    pub(super) async fn list_transcript_segments(
        &self,
        matter_id: &str,
        transcription_job_id: &str,
    ) -> ApiResult<Vec<TranscriptSegment>> {
        let mut segments = self
            .list_nodes::<TranscriptSegment>(matter_id, transcript_segment_spec())
            .await?
            .into_iter()
            .filter(|segment| segment.transcription_job_id == transcription_job_id)
            .collect::<Vec<_>>();
        segments.sort_by_key(|segment| segment.ordinal);
        Ok(segments)
    }

    pub(super) async fn list_transcript_speakers(
        &self,
        matter_id: &str,
        transcription_job_id: &str,
    ) -> ApiResult<Vec<TranscriptSpeaker>> {
        let mut speakers = self
            .list_nodes::<TranscriptSpeaker>(matter_id, transcript_speaker_spec())
            .await?
            .into_iter()
            .filter(|speaker| speaker.transcription_job_id == transcription_job_id)
            .collect::<Vec<_>>();
        speakers.sort_by(|left, right| left.speaker_label.cmp(&right.speaker_label));
        Ok(speakers)
    }

    pub(super) async fn list_transcript_review_changes(
        &self,
        matter_id: &str,
        transcription_job_id: &str,
    ) -> ApiResult<Vec<TranscriptReviewChange>> {
        let mut changes = self
            .list_nodes::<TranscriptReviewChange>(matter_id, transcript_review_change_spec())
            .await?
            .into_iter()
            .filter(|change| change.transcription_job_id == transcription_job_id)
            .collect::<Vec<_>>();
        changes.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(changes)
    }

    pub(super) async fn transcription_response(
        &self,
        matter_id: &str,
        job: &TranscriptionJob,
    ) -> ApiResult<TranscriptionJobResponse> {
        let raw_artifact_version = self
            .optional_document_version(matter_id, &job.raw_artifact_version_id)
            .await?;
        let normalized_artifact_version = self
            .optional_document_version(matter_id, &job.normalized_artifact_version_id)
            .await?;
        let redacted_artifact_version = self
            .optional_document_version(matter_id, &job.redacted_artifact_version_id)
            .await?;
        let reviewed_document_version = self
            .optional_document_version(matter_id, &job.reviewed_document_version_id)
            .await?;
        let caption_vtt_version = self
            .optional_document_version(matter_id, &job.caption_vtt_version_id)
            .await?;
        let caption_srt_version = self
            .optional_document_version(matter_id, &job.caption_srt_version_id)
            .await?;
        let caption_vtt = self
            .version_text(caption_vtt_version.as_ref())
            .await
            .ok()
            .flatten();
        let caption_srt = self
            .version_text(caption_srt_version.as_ref())
            .await
            .ok()
            .flatten();
        Ok(TranscriptionJobResponse {
            job: job.clone(),
            segments: self
                .list_transcript_segments(matter_id, &job.transcription_job_id)
                .await?,
            speakers: self
                .list_transcript_speakers(matter_id, &job.transcription_job_id)
                .await?,
            review_changes: self
                .list_transcript_review_changes(matter_id, &job.transcription_job_id)
                .await?,
            raw_artifact_version,
            normalized_artifact_version,
            redacted_artifact_version,
            reviewed_document_version,
            caption_vtt_version,
            caption_srt_version,
            caption_vtt,
            caption_srt,
            warnings: transcription_warnings(job),
        })
    }

    pub(super) async fn version_text(
        &self,
        version: Option<&DocumentVersion>,
    ) -> ApiResult<Option<String>> {
        let Some(version) = version else {
            return Ok(None);
        };
        let bytes = self.object_store.get_bytes(&version.storage_key).await?;
        Ok(String::from_utf8(bytes.to_vec()).ok())
    }

    pub(super) async fn assemblyai_upload_bytes(
        &self,
        mime_type: Option<String>,
        bytes: Bytes,
    ) -> ApiResult<String> {
        let api_key = self.assemblyai_api_key()?;
        let url = format!("{}/v2/upload", self.assemblyai.base_url);
        let mut request = self
            .http_client
            .post(url)
            .header("Authorization", api_key)
            .timeout(Duration::from_millis(self.assemblyai.timeout_ms))
            .body(bytes);
        if let Some(mime_type) = mime_type {
            request = request.header("content-type", mime_type);
        }
        let response = request.send().await?;
        if !response.status().is_success() {
            return Err(assemblyai_http_error("upload", response.status()));
        }
        let response = response.json::<AssemblyAiUploadResponse>().await?;
        Ok(response.upload_url)
    }

    pub(super) async fn assemblyai_submit_transcript(
        &self,
        upload_url: &str,
        request: &CreateTranscriptionRequest,
        job: &TranscriptionJob,
    ) -> ApiResult<AssemblyAiTranscriptResponse> {
        let api_key = self.assemblyai_api_key()?;
        let payload = assemblyai_transcript_create_request(
            upload_url,
            request,
            job,
            self.assemblyai.webhook_url.clone(),
            self.assemblyai.webhook_secret.clone(),
        );
        let response = self
            .http_client
            .post(format!("{}/v2/transcript", self.assemblyai.base_url))
            .header("Authorization", api_key)
            .json(&payload)
            .timeout(Duration::from_millis(self.assemblyai.timeout_ms))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(assemblyai_http_error(
                "transcript submission",
                response.status(),
            ));
        }
        Ok(response.json::<AssemblyAiTranscriptResponse>().await?)
    }

    pub(super) async fn assemblyai_fetch_transcript(
        &self,
        transcript_id: &str,
    ) -> ApiResult<AssemblyAiTranscriptResponse> {
        let api_key = self.assemblyai_api_key()?;
        let response = self
            .http_client
            .get(format!(
                "{}/v2/transcript/{}",
                self.assemblyai.base_url,
                sanitize_path_segment(transcript_id)
            ))
            .header("Authorization", api_key)
            .timeout(Duration::from_millis(self.assemblyai.timeout_ms))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(assemblyai_http_error("transcript fetch", response.status()));
        }
        Ok(response.json::<AssemblyAiTranscriptResponse>().await?)
    }

    pub(super) fn assemblyai_api_key(&self) -> ApiResult<&str> {
        self.assemblyai
            .api_key
            .as_deref()
            .filter(|key| !key.trim().is_empty())
            .ok_or_else(|| ApiError::BadRequest("AssemblyAI provider is disabled.".to_string()))
    }

    pub(super) async fn import_completed_transcript(
        &self,
        matter_id: &str,
        mut document: CaseDocument,
        mut job: TranscriptionJob,
        provider: AssemblyAiTranscriptResponse,
    ) -> ApiResult<TranscriptionJobResponse> {
        let now = now_string();
        let (mut segments, speakers) =
            transcript_segments_from_provider(matter_id, &document, &job, &provider, &now);
        let source_spans = transcript_source_spans(
            matter_id,
            &document.document_id,
            &job.transcription_job_id,
            &segments,
            job.document_version_id.clone(),
            job.object_blob_id.clone(),
            "unreviewed",
        );
        for (segment, span) in segments.iter_mut().zip(source_spans.iter()) {
            segment.source_span_id = Some(span.source_span_id.clone());
        }
        let raw_bytes =
            serde_json::to_vec(&provider).map_err(|error| ApiError::Internal(error.to_string()))?;
        let raw_artifact = self
            .store_document_artifact_version(
                matter_id,
                &document,
                Bytes::from(raw_bytes),
                Some("application/json".to_string()),
                "transcript_provider_raw_json",
                "transcript_provider_raw_json",
                "json",
                false,
            )
            .await?;
        let normalized_payload = serde_json::json!({
            "job": job.clone(),
            "segments": segments.clone(),
            "speakers": speakers.clone(),
            "provider_status": provider.status.clone(),
            "language_code": provider.language_code.clone(),
            "audio_duration": provider.audio_duration,
        });
        let normalized_bytes = serde_json::to_vec(&normalized_payload)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        let normalized_artifact = self
            .store_document_artifact_version(
                matter_id,
                &document,
                Bytes::from(normalized_bytes),
                Some("application/json".to_string()),
                "transcript_normalized_json",
                "transcript_normalized_json",
                "json",
                false,
            )
            .await?;
        let redaction_method = if job.redact_pii {
            if assemblyai_provider_has_redaction(&provider) {
                "assemblyai-pii-plus-casebuilder-fallback-v1"
            } else {
                "casebuilder-local-pii-v1"
            }
        } else {
            "disabled"
        };
        let redacted_payload = serde_json::json!({
            "segments": segments.iter().map(|segment| serde_json::json!({
                "segment_id": segment.segment_id,
                "ordinal": segment.ordinal,
                "speaker_label": segment.speaker_label,
                "speaker_name": segment.speaker_name,
                "text": segment.redacted_text.clone().unwrap_or_else(|| redact_transcript_text(&segment.text)),
                "time_start_ms": segment.time_start_ms,
                "time_end_ms": segment.time_end_ms,
                "confidence": segment.confidence,
            })).collect::<Vec<_>>(),
            "redaction": redaction_method
        });
        let redacted_bytes = serde_json::to_vec(&redacted_payload)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        let redacted_artifact = self
            .store_document_artifact_version(
                matter_id,
                &document,
                Bytes::from(redacted_bytes),
                Some("application/json".to_string()),
                "transcript_redacted_json",
                "transcript_redacted_json",
                "json",
                false,
            )
            .await?;
        let vtt = transcript_segments_to_vtt(&segments, true);
        let vtt_artifact = self
            .store_document_artifact_version(
                matter_id,
                &document,
                Bytes::from(vtt.clone().into_bytes()),
                Some("text/vtt".to_string()),
                "caption_vtt",
                "caption_vtt",
                "vtt",
                false,
            )
            .await?;
        let srt = transcript_segments_to_srt(&segments, true);
        let srt_artifact = self
            .store_document_artifact_version(
                matter_id,
                &document,
                Bytes::from(srt.clone().into_bytes()),
                Some("application/x-subrip".to_string()),
                "caption_srt",
                "caption_srt",
                "srt",
                false,
            )
            .await?;

        job.status = "review_ready".to_string();
        job.review_status = "needs_review".to_string();
        job.provider_status = Some("completed".to_string());
        job.raw_artifact_version_id = Some(raw_artifact.document_version_id);
        job.normalized_artifact_version_id = Some(normalized_artifact.document_version_id);
        job.redacted_artifact_version_id = Some(redacted_artifact.document_version_id);
        job.caption_vtt_version_id = Some(vtt_artifact.document_version_id);
        job.caption_srt_version_id = Some(srt_artifact.document_version_id);
        job.language_code = provider.language_code.clone();
        job.duration_ms = provider
            .audio_duration
            .map(|seconds| (seconds * 1000.0) as u64);
        job.segment_count = segments.len() as u64;
        job.speaker_count = speakers.len() as u64;
        job.word_count = assemblyai_raw_words(&provider).len() as u64;
        job.retryable = false;
        job.error_code = None;
        job.error_message = None;
        job.completed_at = Some(now.clone());
        job.updated_at = now;

        document.processing_status = "review_ready".to_string();
        document.summary = format!(
            "Transcript is ready for human review: {} segment(s), {} speaker(s).",
            job.segment_count, job.speaker_count
        );
        document.source_spans = source_spans.clone();
        self.merge_node(matter_id, document_spec(), &document.document_id, &document)
            .await?;
        let job = self.merge_transcription_job(matter_id, &job).await?;
        for speaker in &speakers {
            self.merge_transcript_speaker(matter_id, speaker).await?;
        }
        for segment in &segments {
            self.merge_transcript_segment(matter_id, segment).await?;
        }
        for span in &source_spans {
            self.merge_source_span(matter_id, span).await?;
            self.link_transcription_job_to_source_span(&job, span)
                .await?;
        }
        self.transcription_response(matter_id, &job).await
    }

    pub(super) async fn merge_transcription_job(
        &self,
        matter_id: &str,
        job: &TranscriptionJob,
    ) -> ApiResult<TranscriptionJob> {
        let job = self
            .merge_node(
                matter_id,
                transcription_job_spec(),
                &job.transcription_job_id,
                job,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (d:CaseDocument {document_id: $document_id})
                     MATCH (j:TranscriptionJob {transcription_job_id: $transcription_job_id})
                     SET j.document_id = $document_id,
                         j.status = $status,
                         j.provider_transcript_id = $provider_transcript_id
                     MERGE (d)-[:HAS_TRANSCRIPTION_JOB]->(j)
                     WITH d, j
                     OPTIONAL MATCH (v:DocumentVersion {document_version_id: $document_version_id})
                     OPTIONAL MATCH (b:ObjectBlob {object_blob_id: $object_blob_id})
                     FOREACH (_ IN CASE WHEN v IS NULL THEN [] ELSE [1] END |
                       MERGE (j)-[:DERIVED_FROM]->(v)
                     )
                     FOREACH (_ IN CASE WHEN b IS NULL THEN [] ELSE [1] END |
                       MERGE (j)-[:DERIVED_FROM]->(b)
                     )",
                )
                .param("document_id", job.document_id.clone())
                .param("transcription_job_id", job.transcription_job_id.clone())
                .param("status", job.status.clone())
                .param(
                    "provider_transcript_id",
                    job.provider_transcript_id.clone().unwrap_or_default(),
                )
                .param(
                    "document_version_id",
                    job.document_version_id.clone().unwrap_or_default(),
                )
                .param(
                    "object_blob_id",
                    job.object_blob_id.clone().unwrap_or_default(),
                ),
            )
            .await?;
        Ok(job)
    }

    pub(super) async fn merge_transcript_segment(
        &self,
        matter_id: &str,
        segment: &TranscriptSegment,
    ) -> ApiResult<TranscriptSegment> {
        let segment = self
            .merge_node(
                matter_id,
                transcript_segment_spec(),
                &segment.segment_id,
                segment,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (j:TranscriptionJob {transcription_job_id: $transcription_job_id})
                     MATCH (s:TranscriptSegment {segment_id: $segment_id})
                     SET s.transcription_job_id = $transcription_job_id,
                         s.document_id = $document_id,
                         s.ordinal = $ordinal
                     MERGE (j)-[:PRODUCED]->(s)
                     WITH j, s
                     OPTIONAL MATCH (span:SourceSpan {source_span_id: $source_span_id})
                     FOREACH (_ IN CASE WHEN span IS NULL THEN [] ELSE [1] END |
                       MERGE (s)-[:HAS_SOURCE_SPAN]->(span)
                     )",
                )
                .param("transcription_job_id", segment.transcription_job_id.clone())
                .param("segment_id", segment.segment_id.clone())
                .param("document_id", segment.document_id.clone())
                .param("ordinal", segment.ordinal as i64)
                .param(
                    "source_span_id",
                    segment.source_span_id.clone().unwrap_or_default(),
                ),
            )
            .await?;
        Ok(segment)
    }

    pub(super) async fn merge_transcript_speaker(
        &self,
        matter_id: &str,
        speaker: &TranscriptSpeaker,
    ) -> ApiResult<TranscriptSpeaker> {
        let speaker = self
            .merge_node(
                matter_id,
                transcript_speaker_spec(),
                &speaker.speaker_id,
                speaker,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (j:TranscriptionJob {transcription_job_id: $transcription_job_id})
                     MATCH (s:TranscriptSpeaker {speaker_id: $speaker_id})
                     SET s.transcription_job_id = $transcription_job_id,
                         s.speaker_label = $speaker_label
                     MERGE (j)-[:HAS_SPEAKER]->(s)",
                )
                .param("transcription_job_id", speaker.transcription_job_id.clone())
                .param("speaker_id", speaker.speaker_id.clone())
                .param("speaker_label", speaker.speaker_label.clone()),
            )
            .await?;
        Ok(speaker)
    }

    pub(super) async fn merge_transcript_review_change(
        &self,
        matter_id: &str,
        change: &TranscriptReviewChange,
    ) -> ApiResult<TranscriptReviewChange> {
        let change = self
            .merge_node(
                matter_id,
                transcript_review_change_spec(),
                &change.review_change_id,
                change,
            )
            .await?;
        self.neo4j
            .run_rows(
                query(
                    "MATCH (j:TranscriptionJob {transcription_job_id: $transcription_job_id})
                     MATCH (c:TranscriptReviewChange {review_change_id: $review_change_id})
                     MERGE (j)-[:HAS_REVIEW_CHANGE]->(c)",
                )
                .param("transcription_job_id", change.transcription_job_id.clone())
                .param("review_change_id", change.review_change_id.clone()),
            )
            .await?;
        Ok(change)
    }

    pub(super) async fn record_transcript_review_change(
        &self,
        matter_id: &str,
        document_id: &str,
        transcription_job_id: &str,
        target_type: &str,
        target_id: &str,
        field: &str,
        before: Option<String>,
        after: Option<String>,
    ) -> ApiResult<TranscriptReviewChange> {
        let now = now_string();
        let change_id =
            transcript_review_change_id(transcription_job_id, target_id, field, now_secs());
        let change = TranscriptReviewChange {
            review_change_id: change_id.clone(),
            id: change_id,
            matter_id: matter_id.to_string(),
            document_id: document_id.to_string(),
            transcription_job_id: transcription_job_id.to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            field: field.to_string(),
            before,
            after,
            created_by: "user".to_string(),
            created_at: now,
        };
        self.merge_transcript_review_change(matter_id, &change)
            .await
    }

    pub(super) async fn link_transcription_job_to_source_span(
        &self,
        job: &TranscriptionJob,
        span: &SourceSpan,
    ) -> ApiResult<()> {
        self.neo4j
            .run_rows(
                query(
                    "MATCH (j:TranscriptionJob {transcription_job_id: $transcription_job_id})
                     MATCH (s:SourceSpan {source_span_id: $source_span_id})
                     MERGE (j)-[:PRODUCED]->(s)",
                )
                .param("transcription_job_id", job.transcription_job_id.clone())
                .param("source_span_id", span.source_span_id.clone()),
            )
            .await?;
        Ok(())
    }

    pub(super) async fn find_transcription_job_by_provider_id(
        &self,
        provider_transcript_id: &str,
    ) -> ApiResult<Option<TranscriptionJob>> {
        let rows = self
            .neo4j
            .run_rows(
                query(
                    "MATCH (j:TranscriptionJob {provider_transcript_id: $provider_transcript_id})
                     RETURN j.payload AS payload
                     LIMIT 1",
                )
                .param("provider_transcript_id", provider_transcript_id),
            )
            .await?;
        let Some(payload) = rows
            .first()
            .and_then(|row| row.get::<String>("payload").ok())
        else {
            return Ok(None);
        };
        Ok(Some(from_payload(&payload)?))
    }
}
