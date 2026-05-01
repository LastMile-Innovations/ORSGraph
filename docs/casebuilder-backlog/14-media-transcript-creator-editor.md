# 14 - Media Transcript Creator And Editor

CaseBuilder media transcription turns uploaded audio and video evidence into private, reviewable, time-coded transcript material. The first provider implementation uses AssemblyAI pre-recorded transcription behind an explicit feature flag.

## Current Code Baseline

- AssemblyAI configuration is feature-gated with explicit enablement, API key, base URL, webhook secret, timeout, and conservative media-size controls.
- CaseBuilder models now include `TranscriptionJob`, `TranscriptSegment`, `TranscriptSpeaker`, `TranscriptReviewChange`, time-coded `SourceSpan` fields, and workspace-level transcription responses.
- Media documents expose transcription routes for create, list, get, sync, segment edit, speaker edit, review commit, and AssemblyAI webhook receipt.
- The backend uploads private object-store bytes to AssemblyAI instead of requiring public media URLs.
- When PII redaction is requested, the AssemblyAI request asks for both redacted and unredacted transcript fields so raw private artifacts are not lost.
- Completed transcripts are imported into graph nodes and heavy artifact versions for provider raw JSON, normalized JSON, redacted JSON, reviewed text, VTT, and SRT.
- Webhooks store only provider ID/status from the delivery and fetch full transcript results server-side.
- Review commit marks the transcription processed, writes reviewed text, and keeps transcript-derived fact/evidence/timeline creation behind explicit user action.
- The document workspace renders a media player plus transcript editor for media documents, with segment seeking, speaker edits, raw/redacted toggle, review save, provenance/privacy panels, and selected-span case-link actions.
- The document library exposes a dense media queue filter and transcript-oriented status counts.

## Architecture Decisions

- Raw provider payloads are private chain-of-custody artifacts.
- Redacted transcript content is the default review/display/export surface when available.
- Transcript-derived facts, evidence items, and timeline events are never created automatically.
- The document extraction lifecycle remains aligned with existing CaseBuilder states: queued/deferred, processing, review-ready, and processed only after transcript review.
- Heavy transcript artifacts stay in object/document-version storage; Neo4j carries queryable job, segment, speaker, review, and source-span metadata.

## Data Model

- `(:CaseDocument)-[:HAS_TRANSCRIPTION_JOB]->(:TranscriptionJob)`
- `(:TranscriptionJob)-[:DERIVED_FROM]->(:DocumentVersion|ObjectBlob)`
- `(:TranscriptionJob)-[:PRODUCED]->(:TranscriptSegment|SourceSpan|DocumentVersion)`
- Reviewed transcript spans connect to facts, evidence, and timeline items through existing CaseBuilder support-link/mutation paths.

## API Surface

- `POST /matters/:matter_id/documents/:document_id/transcriptions`
- `GET /matters/:matter_id/documents/:document_id/transcriptions`
- `GET /matters/:matter_id/documents/:document_id/transcriptions/:job_id`
- `POST /matters/:matter_id/documents/:document_id/transcriptions/:job_id/sync`
- `PATCH /matters/:matter_id/documents/:document_id/transcriptions/:job_id/segments/:segment_id`
- `PATCH /matters/:matter_id/documents/:document_id/transcriptions/:job_id/speakers/:speaker_id`
- `POST /matters/:matter_id/documents/:document_id/transcriptions/:job_id/review`
- `POST /casebuilder/webhooks/assemblyai`

## Backlog

## CB-TR-001 - Provider config and AssemblyAI client
- Priority: P0
- Area: Provider/config
- Problem: Media transcription needs an explicitly enabled external provider without leaking secrets.
- Expected behavior: Disabled/provider-free states are visible; API keys never appear in logs or user-facing errors.
- Status: Done

## CB-TR-002 - Backend DTOs, graph constraints, frontend types, and contracts
- Priority: P0
- Area: Data model/API/frontend
- Problem: Transcript jobs, speakers, segments, review changes, and time-coded source spans need a shared contract.
- Expected behavior: Rust DTOs, Neo4j constraints/indexes, frontend types, API normalizers, and route contract tests stay aligned.
- Status: Done

## CB-TR-003 - Submit and sync jobs from private object bytes
- Priority: P0
- Area: Provider/storage
- Problem: Evidence media should not require a public object-store URL before transcription.
- Expected behavior: The backend reads private object bytes, uploads them to AssemblyAI, submits a transcript request, and can sync provider status/results.
- Status: Done

## CB-TR-004 - Import transcript artifacts and time-coded spans
- Priority: P0
- Area: Ingestion/provenance
- Problem: Provider output needs durable private artifacts plus queryable normalized transcript nodes.
- Expected behavior: Store raw provider JSON, normalized transcript, redacted copy, segment/speaker records, captions, and time-coded source spans.
- Status: Partial
- Progress: Segment, speaker, provider/local redacted artifact, raw artifact, caption, and source-span imports are implemented. Separate word-level persistence remains future work.

## CB-TR-005 - Media transcript editor
- Priority: P0
- Area: Frontend/editor
- Problem: Users need to review and correct media transcripts before evidence use.
- Expected behavior: Show player, transcript timeline, segment edits, speaker rename, redacted/raw toggle, review save, provenance, and privacy controls.
- Status: Done

## CB-TR-006 - Review commit and no-auto-facts guard
- Priority: P0
- Area: Review/safety
- Problem: Transcript text should not become processed evidence or create facts before review.
- Expected behavior: Review writes reviewed transcript text, marks the job processed, and keeps fact/evidence/timeline creation explicit.
- Status: Done

## CB-TR-007 - Webhook receiver and manual polling fallback
- Priority: P1
- Area: Provider/reliability
- Problem: Provider webhooks can fail or arrive before users open the document.
- Expected behavior: Authenticated webhook updates provider status and manual sync can fetch full results.
- Status: Done

## CB-TR-008 - Create case links from selected transcript spans
- Priority: P1
- Area: Evidence workflow
- Problem: Reviewed transcript passages should feed existing fact, evidence, timeline, and annotation workflows.
- Expected behavior: Selected reviewed segments can create annotations and case records through current CaseBuilder mutation patterns.
- Status: Done

## CB-TR-009 - Caption artifacts and preview
- Priority: P1
- Area: Export/media
- Problem: Reviewed transcript material needs portable caption output.
- Expected behavior: Produce VTT/SRT artifacts and allow the editor to preview/download captions.
- Status: Done

## CB-TR-010 - Matter media queue and bulk retry/sync
- Priority: P1
- Area: Document library
- Problem: Matters with many recordings need a dense review queue.
- Expected behavior: Surface media transcript states and support bulk retry/sync actions.
- Status: Partial
- Progress: The document library has a media queue filter and status surfacing. Bulk actions remain future work.

## CB-TR-011 - Advanced transcript search and proposals
- Priority: P2
- Area: Search/AI
- Problem: Longer recordings need higher-level review aids.
- Expected behavior: Add transcript search, summaries, speaker identification, and batched issue/fact proposals after the reviewed-span foundation is stable.
- Status: Todo

## Verification

- `cargo check -p orsgraph-api`
- `cargo test -p orsgraph-api casebuilder_routes_cover_v0_contracts`
- `cargo test -p orsgraph-api casebuilder_provenance_dtos_exist_in_backend_and_frontend`
- `cargo test -p orsgraph-api document_workspace_contract_is_oss_only_and_casebuilder_native`
- `cargo test -p orsgraph-api assemblyai_request`
- `cargo test -p orsgraph-api transcript_redaction`
- `pnpm run typecheck` from `frontend/`
- `pnpm run lint` from `frontend/`

Live AssemblyAI verification is opt-in and should require explicit environment configuration.
