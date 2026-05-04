import type {
  AuditEvent,
  CaseAiActionResponse,
  AIEditAudit,
  AssemblyAiSpeakerOptions,
  AssemblyAiTranscriptDeleteResponse,
  AssemblyAiTranscriptListQuery,
  AssemblyAiTranscriptListResponse,
  AstDocumentResponse,
  AstMarkdownResponse,
  AstPatch,
  AstRenderedResponse,
  AstValidationResponse,
  AuthorityAttachmentResponse,
  AuthorityTargetType,
  CaseAuthoritySearchResponse,
  CaseBuilderEmbeddingCoverage,
  CaseBuilderEmbeddingRecord,
  CaseBuilderEmbeddingRun,
  CaseBuilderEmbeddingSearchInput,
  CaseBuilderEmbeddingSearchResponse,
  CaseBuilderEffectiveSettings,
  CaseBuilderMatterSettings,
  CaseBuilderMatterSettingsResponse,
  CaseBuilderSettingsPrincipal,
  CaseBuilderUserSettings,
  CaseBuilderUserSettingsResponse,
  CaseGraphResponse,
  CaseCitationCheckFinding,
  CaseDocument,
  CaseDefense,
  CaseEntity,
  CaseEvidence,
  CaseTask,
  CreateMatterIndexJobInput,
  DocxPackageManifest,
  DocumentAnnotation,
  DocumentCapability,
  CaseFactCheckFinding,
  ChangeSet,
  ComplaintCaption,
  ComplaintDraft,
  ComplaintImportResponse,
  ComplaintPreviewResponse,
  ComplaintSection,
  ComplaintCount,
  CompareVersionsResponse,
  ExportPackage,
  ExportArtifact,
  FormattingProfile,
  LegalImpactSummary,
  PleadingParagraph,
  ReliefRequest,
  RulePack,
  RuleCheckFinding,
  RestoreVersionResponse,
  SignatureBlock,
  DocumentVersion,
  DocumentWorkspace,
  Claim,
  ClaimElement,
  Deadline,
  Draft,
  DraftParagraph,
  DraftSection,
  EntityMention,
  EvidenceSpan,
  ExtractionArtifactManifest,
  ExtractedFact,
  IngestionRun,
  IndexRun,
  IssueSpotResponse,
  Matter,
  MatterIndexJob,
  MatterIndexRunDocumentResult,
  MatterIndexRunResponse,
  MatterIndexSummary,
  MatterParty,
  MatterSummary,
  MarkdownAstDocument,
  MarkdownAstNode,
  MarkdownSemanticUnit,
  Page,
  PatchCaseBuilderMatterSettingsInput,
  PatchCaseBuilderUserSettingsInput,
  QcRun,
  RunCaseBuilderEmbeddingsInput,
  RunCaseBuilderEmbeddingsResponse,
  SearchIndexRecord,
  SourceSpan,
  TextChunk,
  TimelineAgentRun,
  TimelineEvent,
  TimelineSuggestResponse,
  TimelineSuggestion,
  TimelineSuggestionApprovalResponse,
  TranscriptionJob,
  TranscriptionJobResponse,
  TranscriptReviewChange,
  TranscriptSegment,
  TranscriptSpeaker,
  WorkProduct,
  WorkProductAnchor,
  WorkProductArtifact,
  WorkProductBlock,
  WorkProductCitationUse,
  WorkProductDocument,
  WorkProductExhibitReference,
  WorkProductFinding,
  WorkProductLink,
  WorkProductMark,
  WorkProductPreviewResponse,
  VersionChangeSummary,
  VersionLayerDiff,
  VersionSnapshot,
  VersionTextDiff,
} from "./types"
import { getMatterById as getDemoMatterById, matters as demoMatters } from "./mock-matters"
import { orsApiBaseUrl } from "../ors-api-url"
import { decodeMatterRouteId, decodeRouteSegment } from "./routes"

const API_BASE_URL = orsApiBaseUrl()
const DEMO_MODE = process.env.NEXT_PUBLIC_ORS_DEMO_MODE === "true"
export const DEFAULT_CASEBUILDER_API_TIMEOUT_MS = 120_000
const API_TIMEOUT_MS = resolveCaseBuilderApiTimeoutMs()

export function resolveCaseBuilderApiTimeoutMs(
  casebuilderTimeout = process.env.NEXT_PUBLIC_ORS_CASEBUILDER_API_TIMEOUT_MS,
  genericTimeout = process.env.NEXT_PUBLIC_ORS_API_TIMEOUT_MS,
) {
  const explicitCasebuilderTimeout = positiveTimeoutMs(casebuilderTimeout)
  if (explicitCasebuilderTimeout != null) return explicitCasebuilderTimeout

  const inheritedTimeout = positiveTimeoutMs(genericTimeout)
  return inheritedTimeout == null
    ? DEFAULT_CASEBUILDER_API_TIMEOUT_MS
    : Math.max(inheritedTimeout, DEFAULT_CASEBUILDER_API_TIMEOUT_MS)
}

function positiveTimeoutMs(value: string | undefined) {
  if (!value?.trim()) return null
  const parsed = Number(value)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null
}

function documentContentProxyUrl(matterId: string, documentId: string, contentUrl?: string | null): string | null {
  if (!contentUrl) return null
  const params = new URLSearchParams({ matterId, documentId })
  return `/api/casebuilder/document-content?${params.toString()}`
}

export type LoadSource = "live" | "demo" | "offline" | "error"

export interface LoadState<T> {
  source: LoadSource
  data: T
  error?: string
}

export interface ActionState<T> {
  source: "live" | "error"
  data: T | null
  error?: string
}

export interface CaseBuilderRequestOptions extends Pick<RequestInit, "headers" | "signal"> {
  timeoutMs?: number | null
}

export interface SignedUploadProgress {
  loaded: number
  total: number | null
  speedBps: number | null
  elapsedMs: number
}

export interface CreateMatterInput {
  name: string
  matter_type?: MatterSummary["matter_type"]
  user_role?: MatterSummary["user_role"]
  jurisdiction?: string
  court?: string
  case_number?: string | null
  settings?: PatchCaseBuilderMatterSettingsInput
}

export interface PatchMatterInput extends Partial<CreateMatterInput> {
  status?: MatterSummary["status"]
}

export interface PatchMatterConfigInput {
  matter?: PatchMatterInput
  settings?: PatchCaseBuilderMatterSettingsInput
}

export interface UploadTextFileInput {
  filename: string
  text?: string
  mime_type?: string
  bytes?: number
  document_type?: string
  folder?: string
  confidentiality?: string
  relative_path?: string
  upload_batch_id?: string
}

export interface CreateFileUploadInput {
  filename: string
  mime_type?: string
  bytes: number
  document_type?: string
  folder?: string
  confidentiality?: string
  relative_path?: string
  upload_batch_id?: string
  sha256?: string
}

export interface FileUploadIntent {
  upload_id: string
  document_id: string
  method: "PUT" | string
  url: string
  expires_at: string
  headers: Record<string, string>
  document: CaseDocument
}

export interface CompleteFileUploadInput {
  document_id: string
  etag?: string | null
  bytes?: number
  sha256?: string
}

export interface DownloadUrlResponse {
  method: "GET" | string
  url: string
  expires_at: string
  headers: Record<string, string>
  filename: string
  mime_type?: string | null
  bytes: number
}

export interface SignedUploadPutResponse {
  etag?: string | null
}

export interface ExtractDocumentResponse {
  enabled: boolean
  mode: string
  status: string
  message: string
  document: CaseDocument
  chunks: Array<{
    chunk_id: string
    document_id: string
    page: number
    text: string
    document_version_id?: string | null
    object_blob_id?: string | null
    source_span_id?: string | null
    byte_start?: number | null
    byte_end?: number | null
    char_start?: number | null
    char_end?: number | null
    markdown_ast_node_ids?: string[]
  }>
  proposed_facts: ExtractedFact[]
  ingestion_run?: IngestionRun | null
  index_run?: IndexRun | null
  document_version?: DocumentVersion | null
  index_artifacts: DocumentVersion[]
  artifact_manifest?: ExtractionArtifactManifest | null
  pages: Page[]
  text_chunks: TextChunk[]
  evidence_spans: EvidenceSpan[]
  entity_mentions: EntityMention[]
  markdown_ast_document?: MarkdownAstDocument | null
  markdown_ast_nodes: MarkdownAstNode[]
  markdown_semantic_units: MarkdownSemanticUnit[]
  entities: CaseEntity[]
  search_index_records: SearchIndexRecord[]
  embedding_run?: CaseBuilderEmbeddingRun | null
  source_spans: SourceSpan[]
  timeline_suggestions: TimelineSuggestion[]
}

export interface RunMatterIndexInput {
  document_ids?: string[]
  limit?: number
}

export interface SaveDocumentTextInput {
  text: string
}

export interface PatchDocumentInput {
  title?: string
  library_path?: string
  document_type?: string
  confidentiality?: string
  is_exhibit?: boolean
  exhibit_label?: string | null
  date_observed?: string | null
}

export interface ArchiveDocumentInput {
  reason?: string
}

export interface SaveDocumentTextResponse {
  document: CaseDocument
  document_version: DocumentVersion
  ingestion_run: IngestionRun
  warnings: string[]
}

export interface CreateDocumentAnnotationInput {
  annotation_type: string
  label?: string
  note?: string
  color?: string
  page_range?: DocumentAnnotation["page_range"]
  text_range?: DocumentAnnotation["text_range"]
  target_type?: string
  target_id?: string
  status?: string
}

export interface PromoteDocumentWorkProductInput {
  product_type?: string
  title?: string
}

export interface PromoteDocumentWorkProductResponse {
  work_product: WorkProduct
  warnings: string[]
}

export interface CreateTranscriptionInput {
  force?: boolean
  language_code?: string | null
  redact_pii?: boolean
  speaker_labels?: boolean
  speakers_expected?: number | null
  speaker_options?: AssemblyAiSpeakerOptions | null
  word_search_terms?: string[]
  prompt_preset?: string | null
  prompt?: string | null
  keyterms_prompt?: string[]
  remove_audio_tags?: "all" | string | null
}

export interface PatchTranscriptSegmentInput {
  text?: string
  redacted_text?: string | null
  speaker_label?: string | null
  review_status?: string
}

export interface PatchTranscriptSpeakerInput {
  display_name?: string | null
  role?: string | null
}

export interface ReviewTranscriptionInput {
  reviewed_text?: string
  status?: string
  review_surface?: "redacted" | "raw"
}

export interface CreatePartyInput {
  name: string
  role?: string
  party_type?: string
  represented_by?: string | null
  contact_email?: string
  contact_phone?: string
  notes?: string
}

export interface CreateFactInput {
  statement: string
  status?: string
  confidence?: number
  date?: string | null
  party_id?: string | null
  source_document_ids?: string[]
  source_evidence_ids?: string[]
  source_span_ids?: string[]
  markdown_ast_node_ids?: string[]
  notes?: string | null
}

export type PatchFactInput = Partial<CreateFactInput>

export interface CreateTimelineEventInput {
  date: string
  title: string
  description?: string | null
  kind?: string
  source_document_id?: string | null
  party_ids?: string[]
  linked_fact_ids?: string[]
  linked_claim_ids?: string[]
  source_span_ids?: string[]
  text_chunk_ids?: string[]
  markdown_ast_node_ids?: string[]
  suggestion_id?: string | null
  agent_run_id?: string | null
}

export interface TimelineSuggestInput {
  document_ids?: string[]
  source_span_ids?: string[]
  work_product_id?: string
  block_id?: string
  limit?: number
  mode?: string
}

export interface PatchTimelineSuggestionInput {
  date?: string
  date_text?: string
  date_confidence?: number
  title?: string
  description?: string | null
  kind?: string
  source_document_id?: string | null
  source_span_ids?: string[]
  text_chunk_ids?: string[]
  linked_fact_ids?: string[]
  linked_claim_ids?: string[]
  status?: string
  warnings?: string[]
}

export interface CreateClaimInput {
  kind?: string
  title: string
  claim_type?: string
  legal_theory?: string
  status?: string
  risk_level?: string
  fact_ids?: string[]
  evidence_ids?: string[]
  authorities?: Array<{ citation: string; canonical_id: string; reason?: string; pinpoint?: string }>
  elements?: Array<{ text: string; authority?: string; fact_ids?: string[]; evidence_ids?: string[] }>
}

export interface CreateDefenseInput {
  name: string
  basis?: string
  status?: string
  applies_to_claim_ids?: string[]
  required_facts?: string[]
  fact_ids?: string[]
  evidence_ids?: string[]
  authorities?: Array<{ citation: string; canonical_id: string; reason?: string; pinpoint?: string }>
  viability?: string
}

export interface CreateEvidenceInput {
  document_id: string
  source_span?: string
  quote: string
  evidence_type?: string
  strength?: string
  confidence?: number
  exhibit_label?: string | null
  supports_fact_ids?: string[]
  contradicts_fact_ids?: string[]
}

export interface LinkEvidenceFactInput {
  fact_id: string
  relation?: "supports" | "contradicts"
}

export interface CreateDeadlineInput {
  title: string
  due_date: string
  description?: string
  category?: string
  kind?: string
  severity?: string
  source?: string
  source_citation?: string | null
  source_canonical_id?: string | null
  triggered_by_event_id?: string | null
  status?: string
  notes?: string | null
}

export type PatchDeadlineInput = Partial<CreateDeadlineInput>

export interface ComputeDeadlinesResponse {
  generated: Deadline[]
  warnings: string[]
}

export interface CreateTaskInput {
  title: string
  status?: string
  priority?: string
  due_date?: string | null
  assigned_to?: string | null
  related_claim_ids?: string[]
  related_document_ids?: string[]
  related_deadline_id?: string | null
  source?: string
  description?: string | null
}

export type PatchTaskInput = Partial<CreateTaskInput>

export interface AuthorityAttachmentInput {
  target_type: AuthorityTargetType
  target_id: string
  citation: string
  canonical_id: string
  reason?: string
  pinpoint?: string
}

export interface CreateDraftInput {
  title: string
  draft_type?: string
  description?: string
  status?: string
}

export interface PatchDraftInput {
  title?: string
  description?: string
  status?: string
  sections?: Draft["sections"]
  paragraphs?: DraftParagraph[]
}

export interface CreateComplaintInput {
  title?: string
  template?: string
  source_draft_id?: string
}

export interface ComplaintImportInput {
  document_id?: string
  document_ids?: string[]
  title?: string
  force?: boolean
  mode?: string
}

export interface PatchComplaintInput {
  title?: string
  status?: string
  review_status?: string
  setup_stage?: string
  caption?: ComplaintCaption
  sections?: ComplaintSection[]
  counts?: ComplaintCount[]
  paragraphs?: PleadingParagraph[]
  relief?: ReliefRequest[]
  signature?: SignatureBlock
  formatting_profile?: FormattingProfile
}

export interface CreateWorkProductInput {
  title?: string
  product_type: string
  template?: string
  source_draft_id?: string
  source_complaint_id?: string
}

export interface PatchWorkProductInput {
  title?: string
  status?: string
  review_status?: string
  setup_stage?: string
  document_ast?: WorkProductDocument
  blocks?: WorkProductBlock[]
  marks?: WorkProductMark[]
  anchors?: WorkProductAnchor[]
  formatting_profile?: FormattingProfile
}

export interface GetWorkProductsOptions {
  includeDocumentAst?: boolean
  request?: CaseBuilderRequestOptions
}

export type AstPatchConcurrency =
  | { base_document_hash: string; base_snapshot_id?: string | null }
  | { base_document_hash?: string | null; base_snapshot_id: string }

export interface CreateWorkProductBlockInput {
  block_type?: string
  role?: string
  title?: string
  text: string
  parent_block_id?: string | null
  fact_ids?: string[]
  evidence_ids?: string[]
  authorities?: WorkProductBlock["authorities"]
}

export interface PatchWorkProductBlockInput {
  block_type?: string
  role?: string
  title?: string
  text?: string
  parent_block_id?: string | null
  fact_ids?: string[]
  evidence_ids?: string[]
  authorities?: WorkProductBlock["authorities"]
  locked?: boolean
  review_status?: string
  prosemirror_json?: Record<string, unknown> | null
}

export interface WorkProductLinkInput {
  block_id: string
  anchor_type?: string
  relation?: string
  target_type: string
  target_id: string
  citation?: string
  canonical_id?: string
  pinpoint?: string
  quote?: string
}

export interface PatchWorkProductSupportInput {
  relation?: string
  status?: string
  citation?: string | null
  canonical_id?: string | null
  pinpoint?: string | null
  quote?: string | null
}

export interface WorkProductTextRangeLinkInput {
  block_id: string
  start_offset: number
  end_offset: number
  quote: string
  target_type: string
  target_id: string
  relation?: string
  citation?: string
  canonical_id?: string
  pinpoint?: string
  exhibit_label?: string
  document_id?: string
  page_range?: string
}

export interface ExportWorkProductInput {
  format: string
  profile?: string
  mode?: string
  include_exhibits?: boolean
  include_qc_report?: boolean
}

export interface WorkProductAiCommandInput {
  command: string
  target_id?: string
  prompt?: string
}

export interface MarkdownToAstInput {
  markdown: string
}

export interface CreateVersionSnapshotInput {
  title?: string
  message?: string
}

export interface CompareWorkProductVersionsInput {
  from: string
  to?: string
  layers?: string[]
}

export interface RestoreVersionInput {
  snapshot_id: string
  scope: "work_product" | "complaint" | "block" | "paragraph" | string
  target_ids?: string[]
  mode?: string
  branch_id?: string
  dry_run?: boolean
}

export interface CreateComplaintSectionInput {
  title: string
  section_type?: string
}

export interface CreateComplaintCountInput {
  title: string
  claim_id?: string
  legal_theory?: string
  against_party_ids?: string[]
  element_ids?: string[]
  relief_ids?: string[]
}

export interface CreateComplaintParagraphInput {
  section_id?: string
  count_id?: string
  role?: string
  text: string
  fact_ids?: string[]
  evidence_ids?: string[]
}

export interface PatchComplaintParagraphInput {
  section_id?: string
  count_id?: string
  role?: string
  text?: string
  fact_ids?: string[]
  locked?: boolean
  review_status?: string
}

export interface ComplaintLinkInput {
  target_type: "paragraph" | "sentence" | "count" | string
  target_id: string
  relation?: string
  fact_id?: string
  evidence_id?: string
  document_id?: string
  source_span_id?: string
  citation?: string
  canonical_id?: string
  pinpoint?: string
  quote?: string
  exhibit_label?: string
}

export interface ExportComplaintInput {
  format: "pdf" | "docx" | "html" | "markdown" | "text" | "plain_text" | "json" | string
  profile?: string
  mode?: string
  include_exhibits?: boolean
  include_qc_report?: boolean
}

export interface MatterAskInput {
  question: string
  scope?: "all" | "documents" | "facts" | "claims" | string
  thread_id?: string
}

export interface MatterAskResponse {
  answer: string
  citations: Array<{
    citation_id: string
    kind: string
    source_id: string
    title: string
    snippet?: string | null
  }>
  source_spans: SourceSpan[]
  related_facts: ExtractedFact[]
  related_documents: CaseDocument[]
  warnings: string[]
  mode: string
  thread_id?: string | null
}

export async function getMatterSummariesState(
  options: CaseBuilderRequestOptions = {},
): Promise<LoadState<MatterSummary[]>> {
  try {
    const live = await fetchCaseBuilder<MatterSummary[]>("/matters", options)
    return { source: "live", data: live.map(normalizeMatterSummary) }
  } catch (error) {
    if (shouldUseDemoMatterFallback(error)) {
      return { source: "demo", data: demoMatters, error: errorMessage(error) }
    }
    return { source: "error", data: [], error: errorMessage(error) }
  }
}

export async function getMatterState(
  id: string,
  options: CaseBuilderRequestOptions = {},
): Promise<LoadState<Matter | null>> {
  const matterId = decodeMatterRouteId(id)
  try {
    const live = await fetchCaseBuilder<unknown>(`/matters/${encodeURIComponent(matterId)}`, options)
    return { source: "live", data: normalizeMatter(live) }
  } catch (error) {
    const demo = getDemoMatterById(matterId) ?? null
    if (demo && shouldUseDemoMatterFallback(error, { allowNotFound: true })) {
      return { source: "demo", data: demo, error: errorMessage(error) }
    }
    return { source: "error", data: null, error: errorMessage(error) }
  }
}

export async function getCaseBuilderSettingsState(
  options: CaseBuilderRequestOptions = {},
): Promise<LoadState<CaseBuilderUserSettingsResponse | null>> {
  try {
    const live = await fetchCaseBuilder<unknown>("/casebuilder/settings", options)
    return { source: "live", data: normalizeCaseBuilderUserSettingsResponse(live) }
  } catch (error) {
    return { source: "error", data: null, error: errorMessage(error) }
  }
}

export function patchCaseBuilderSettings(
  input: PatchCaseBuilderUserSettingsInput,
): Promise<ActionState<CaseBuilderUserSettingsResponse>> {
  return runCaseBuilderAction("/casebuilder/settings", {
    method: "PATCH",
    body: JSON.stringify(input),
    normalize: normalizeCaseBuilderUserSettingsResponse,
  })
}

export async function getMatterSettingsState(
  matterId: string,
  options: CaseBuilderRequestOptions = {},
): Promise<LoadState<CaseBuilderMatterSettingsResponse | null>> {
  try {
    const live = await fetchCaseBuilder<unknown>(
      `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/settings`,
      options,
    )
    return { source: "live", data: normalizeCaseBuilderMatterSettingsResponse(live) }
  } catch (error) {
    return { source: "error", data: null, error: errorMessage(error) }
  }
}

export function patchMatterConfig(
  matterId: string,
  input: PatchMatterConfigInput,
): Promise<ActionState<CaseBuilderMatterSettingsResponse>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/settings`, {
    method: "PATCH",
    body: JSON.stringify(input),
    normalize: normalizeCaseBuilderMatterSettingsResponse,
  })
}

export function createMatter(input: CreateMatterInput): Promise<ActionState<Matter>> {
  return runCaseBuilderAction("/matters", {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeMatter,
  })
}

export function patchMatter(matterId: string, input: PatchMatterInput): Promise<ActionState<Matter>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}`, {
    method: "PATCH",
    body: JSON.stringify(input),
    normalize: normalizeMatter,
  })
}

export function deleteMatter(matterId: string): Promise<ActionState<{ deleted: boolean }>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}`, {
    method: "DELETE",
    normalize: (raw) => ({ deleted: Boolean((raw as any)?.deleted) }),
  })
}

export function uploadTextFile(
  matterId: string,
  input: UploadTextFileInput,
): Promise<ActionState<CaseDocument>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/files`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeDocument,
  })
}

export function createFileUpload(
  matterId: string,
  input: CreateFileUploadInput,
): Promise<ActionState<FileUploadIntent>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/files/uploads`, {
    method: "POST",
    body: JSON.stringify(input),
    timeoutMs: null,
    normalize: (raw) => {
      const response = raw as any
      return {
        ...response,
        headers: response.headers ?? {},
        document: normalizeDocument(response.document),
      } as FileUploadIntent
    },
  })
}

export function completeFileUpload(
  matterId: string,
  uploadId: string,
  input: CompleteFileUploadInput,
): Promise<ActionState<CaseDocument>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/files/uploads/${encodeURIComponent(uploadId)}/complete`,
    {
      method: "POST",
      body: JSON.stringify(input),
      timeoutMs: null,
      normalize: normalizeDocument,
    },
  )
}

export async function putSignedUploadFile(
  intent: Pick<FileUploadIntent, "method" | "url" | "headers">,
  file: File,
  options: Pick<RequestInit, "signal"> & {
    onProgress?: (progress: SignedUploadProgress) => void
  } = {},
): Promise<ActionState<SignedUploadPutResponse>> {
  if (options.onProgress && typeof XMLHttpRequest !== "undefined") {
    return putSignedUploadFileWithProgress(intent, file, options)
  }

  try {
    const response = await fetch(intent.url, {
      method: intent.method || "PUT",
      headers: new Headers(intent.headers),
      body: file,
      signal: options.signal,
    })
    if (!response.ok) {
      throw new Error(`Signed upload failed: ${response.status}`)
    }
    return { source: "live", data: { etag: response.headers.get("etag") } }
  } catch (error) {
    return { source: "error", data: null, error: errorMessage(error) }
  }
}

function putSignedUploadFileWithProgress(
  intent: Pick<FileUploadIntent, "method" | "url" | "headers">,
  file: File,
  options: Pick<RequestInit, "signal"> & {
    onProgress?: (progress: SignedUploadProgress) => void
  },
): Promise<ActionState<SignedUploadPutResponse>> {
  return new Promise((resolve) => {
    const xhr = new XMLHttpRequest()
    const startedAt = performance.now()
    let settled = false

    function finish(state: ActionState<SignedUploadPutResponse>) {
      if (settled) return
      settled = true
      options.signal?.removeEventListener("abort", abortUpload)
      resolve(state)
    }

    function progress(loaded: number, total: number | null) {
      const elapsedMs = Math.max(performance.now() - startedAt, 1)
      const speedBps = loaded > 0 ? loaded / (elapsedMs / 1000) : 0
      options.onProgress?.({ loaded, total, speedBps, elapsedMs })
    }

    function abortUpload() {
      xhr.abort()
      finish({ source: "error", data: null, error: "Upload canceled" })
    }

    xhr.upload.onprogress = (event) => {
      progress(event.loaded, event.lengthComputable ? event.total : file.size)
    }
    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        progress(file.size, file.size)
        finish({ source: "live", data: { etag: xhr.getResponseHeader("etag") } })
      } else {
        finish({ source: "error", data: null, error: `Signed upload failed: ${xhr.status}` })
      }
    }
    xhr.onerror = () => finish({ source: "error", data: null, error: "Signed upload failed." })
    xhr.onabort = () => finish({ source: "error", data: null, error: "Upload canceled" })

    if (options.signal?.aborted) {
      abortUpload()
      return
    }
    options.signal?.addEventListener("abort", abortUpload, { once: true })

    try {
      xhr.open(intent.method || "PUT", intent.url, true)
      new Headers(intent.headers).forEach((value, key) => xhr.setRequestHeader(key, value))
      progress(0, file.size)
      xhr.send(file)
    } catch (error) {
      finish({ source: "error", data: null, error: errorMessage(error) })
    }
  })
}

export async function uploadBinaryFile(
  matterId: string,
  file: File,
  input: Omit<CreateFileUploadInput, "filename" | "mime_type" | "bytes" | "sha256"> = {},
): Promise<ActionState<CaseDocument>> {
  try {
    const form = new FormData()
    form.append("file", file, file.name)
    if (input.document_type) form.append("document_type", input.document_type)
    if (input.folder) form.append("folder", input.folder)
    if (input.confidentiality) form.append("confidentiality", input.confidentiality)
    if (input.relative_path) form.append("relative_path", input.relative_path)
    if (input.upload_batch_id) form.append("upload_batch_id", input.upload_batch_id)
    return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/files/binary`, {
      method: "POST",
      body: form,
      normalize: normalizeDocument,
    })
  } catch (error) {
    return { source: "error", data: null, error: errorMessage(error) }
  }
}

export function createDocumentDownloadUrl(
  matterId: string,
  documentId: string,
): Promise<ActionState<DownloadUrlResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/download-url`,
    {
      method: "POST",
      normalize: (raw) => {
        const response = raw as any
        return {
          ...response,
          headers: response.headers ?? {},
        } as DownloadUrlResponse
      },
    },
  )
}

export function getMatterIndexSummary(matterId: string): Promise<ActionState<MatterIndexSummary>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/index`, {
    normalize: normalizeMatterIndexSummary,
  })
}

export function runMatterIndex(
  matterId: string,
  input: RunMatterIndexInput = {},
): Promise<ActionState<MatterIndexRunResponse>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/index/run`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeMatterIndexRunResponse,
  })
}

export function createMatterIndexJob(
  matterId: string,
  input: CreateMatterIndexJobInput = {},
): Promise<ActionState<MatterIndexJob>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/index/jobs`, {
    method: "POST",
    body: JSON.stringify(input),
    timeoutMs: null,
    normalize: normalizeMatterIndexJob,
  })
}

export function listMatterIndexJobs(
  matterId: string,
  input: { active?: boolean } = {},
): Promise<ActionState<MatterIndexJob[]>> {
  const params = new URLSearchParams()
  if (typeof input.active === "boolean") params.set("active", String(input.active))
  const suffix = params.toString() ? `?${params.toString()}` : ""
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/index/jobs${suffix}`, {
    normalize: (raw) => array(raw).map(normalizeMatterIndexJob),
  })
}

export function getMatterIndexJob(
  matterId: string,
  jobId: string,
): Promise<ActionState<MatterIndexJob>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/index/jobs/${encodeURIComponent(jobId)}`,
    {
      normalize: normalizeMatterIndexJob,
    },
  )
}

export async function getDocumentWorkspace(
  matterId: string,
  documentId: string,
  options: CaseBuilderRequestOptions = {},
): Promise<LoadState<DocumentWorkspace | null>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  const decodedDocumentId = decodeRouteSegment(documentId)
  try {
    const live = await fetchCaseBuilder<unknown>(
      `/matters/${encodeURIComponent(decodedMatterId)}/documents/${encodeURIComponent(decodedDocumentId)}/workspace`,
      options,
    )
    return { source: "live", data: normalizeDocumentWorkspace(live) }
  } catch (error) {
    if (!shouldUseDemoMatterFallback(error, { allowNotFound: true })) {
      return { source: "error", data: null, error: errorMessage(error) }
    }
    const matter = getDemoMatterById(decodedMatterId)
    const document = matter?.documents.find(
      (candidate) => candidate.id === decodedDocumentId || candidate.document_id === decodedDocumentId,
    )
    if (!matter || !document) {
      return { source: "error", data: null, error: errorMessage(error) }
    }
    return {
      source: "demo",
      data: normalizeDocumentWorkspace({
        matter_id: matter.id,
        document,
        current_version: null,
        capabilities: demoCapabilitiesForDocument(document),
        annotations: [],
        source_spans: document.source_spans ?? [],
        markdown_ast_document: null,
        markdown_ast_nodes: [],
        markdown_semantic_units: [],
        text_chunks: [],
        evidence_spans: [],
        entity_mentions: [],
        entities: [],
        search_index_records: [],
        embedding_runs: [],
        embedding_records: [],
        embedding_coverage: {},
        proposed_facts: [],
        timeline_suggestions: [],
        docx_manifest: null,
        text_content: document.extracted_text ?? document.chunks?.map((chunk) => chunk.text).join("\n\n") ?? null,
        content_url: null,
        warnings: ["Demo mode is showing an offline document workspace."],
      }),
      error: errorMessage(error),
    }
  }
}

export function createDocumentAnnotation(
  matterId: string,
  documentId: string,
  input: CreateDocumentAnnotationInput,
): Promise<ActionState<DocumentAnnotation>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/annotations`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeDocumentAnnotation,
    },
  )
}

export function saveDocumentText(
  matterId: string,
  documentId: string,
  input: SaveDocumentTextInput,
): Promise<ActionState<SaveDocumentTextResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/text`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: (raw) => {
        const response = raw as any
        return {
          document: normalizeDocument(response.document),
          document_version: normalizeDocumentVersion(response.document_version),
          ingestion_run: normalizeIngestionRun(response.ingestion_run),
          warnings: array(response.warnings),
        }
      },
    },
  )
}

export function promoteDocumentWorkProduct(
  matterId: string,
  documentId: string,
  input: PromoteDocumentWorkProductInput,
): Promise<ActionState<PromoteDocumentWorkProductResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/promote-work-product`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: (raw) => {
        const response = raw as any
        return {
          work_product: normalizeWorkProduct(response.work_product),
          warnings: array(response.warnings),
        }
      },
    },
  )
}

export function patchDocument(
  matterId: string,
  documentId: string,
  input: PatchDocumentInput,
): Promise<ActionState<CaseDocument>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeDocument,
    },
  )
}

export function archiveDocument(
  matterId: string,
  documentId: string,
  input: ArchiveDocumentInput = {},
): Promise<ActionState<CaseDocument>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/archive`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeDocument,
    },
  )
}

export function restoreDocument(
  matterId: string,
  documentId: string,
): Promise<ActionState<CaseDocument>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/restore`,
    {
      method: "POST",
      normalize: normalizeDocument,
    },
  )
}

export function deleteDocument(
  matterId: string,
  documentId: string,
): Promise<ActionState<{ deleted: boolean; document: CaseDocument }>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}`,
    {
      method: "DELETE",
      normalize: (raw) => {
        const response = raw as any
        return {
          deleted: Boolean(response.deleted),
          document: normalizeDocument(response.document),
        }
      },
    },
  )
}

export function extractDocument(
  matterId: string,
  documentId: string,
): Promise<ActionState<ExtractDocumentResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/extract`,
    {
      method: "POST",
      normalize: (raw) => {
        const response = raw as any
        return {
          ...response,
          document: normalizeDocument(response.document),
          chunks: array(response.chunks).map(normalizeExtractionChunk),
          proposed_facts: array(response.proposed_facts).map(normalizeFact),
          ingestion_run: response.ingestion_run ? normalizeIngestionRun(response.ingestion_run) : null,
          index_run: response.index_run ? normalizeIndexRun(response.index_run) : null,
          document_version: response.document_version ? normalizeDocumentVersion(response.document_version) : null,
          index_artifacts: array(response.index_artifacts, response.indexArtifacts).map(normalizeDocumentVersion),
          artifact_manifest: response.artifact_manifest || response.artifactManifest ? normalizeExtractionArtifactManifest(response.artifact_manifest ?? response.artifactManifest) : null,
          pages: array(response.pages).map(normalizeIndexPage),
          text_chunks: array(response.text_chunks, response.textChunks).map(normalizeTextChunk),
          evidence_spans: array(response.evidence_spans, response.evidenceSpans).map(normalizeEvidenceSpan),
          entity_mentions: array(response.entity_mentions, response.entityMentions).map(normalizeEntityMention),
          markdown_ast_document:
            response.markdown_ast_document || response.markdownAstDocument
              ? normalizeMarkdownAstDocument(response.markdown_ast_document ?? response.markdownAstDocument)
              : null,
          markdown_ast_nodes: array(response.markdown_ast_nodes, response.markdownAstNodes).map(normalizeMarkdownAstNode),
          markdown_semantic_units: array(response.markdown_semantic_units, response.markdownSemanticUnits).map(normalizeMarkdownSemanticUnit),
          entities: array(response.entities).map(normalizeCaseEntity),
          search_index_records: array(response.search_index_records, response.searchIndexRecords).map(normalizeSearchIndexRecord),
          embedding_run:
            response.embedding_run || response.embeddingRun
              ? normalizeCaseBuilderEmbeddingRun(response.embedding_run ?? response.embeddingRun)
              : null,
          source_spans: array(response.source_spans).map(normalizeSourceSpan),
          timeline_suggestions: array(response.timeline_suggestions, response.timelineSuggestions).map(normalizeTimelineSuggestion),
        }
      },
    },
  )
}

export function runMatterEmbeddings(
  matterId: string,
  input: RunCaseBuilderEmbeddingsInput = {},
): Promise<ActionState<RunCaseBuilderEmbeddingsResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/embeddings/run`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeRunCaseBuilderEmbeddingsResponse,
    },
  )
}

export function runDocumentEmbeddings(
  matterId: string,
  documentId: string,
): Promise<ActionState<CaseBuilderEmbeddingRun>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/embeddings/run`,
    {
      method: "POST",
      normalize: normalizeCaseBuilderEmbeddingRun,
    },
  )
}

export function searchMatterEmbeddings(
  matterId: string,
  input: CaseBuilderEmbeddingSearchInput,
): Promise<ActionState<CaseBuilderEmbeddingSearchResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/embeddings/search`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeCaseBuilderEmbeddingSearchResponse,
    },
  )
}

export function createTranscription(
  matterId: string,
  documentId: string,
  input: CreateTranscriptionInput = {},
): Promise<ActionState<TranscriptionJobResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/transcriptions`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeTranscriptionJobResponse,
    },
  )
}

export function listTranscriptions(
  matterId: string,
  documentId: string,
): Promise<ActionState<TranscriptionJobResponse[]>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/transcriptions`,
    {
      method: "GET",
      normalize: (raw) => array(raw).map(normalizeTranscriptionJobResponse),
    },
  )
}

export function listAssemblyAiTranscripts(
  input: AssemblyAiTranscriptListQuery = {},
): Promise<ActionState<AssemblyAiTranscriptListResponse>> {
  const params = new URLSearchParams()
  if (input.limit !== undefined) params.set("limit", String(input.limit))
  if (input.status) params.set("status", input.status)
  if (input.created_on) params.set("created_on", input.created_on)
  if (input.before_id) params.set("before_id", input.before_id)
  if (input.after_id) params.set("after_id", input.after_id)
  if (input.throttled_only !== undefined) params.set("throttled_only", String(input.throttled_only))
  const query = params.toString()
  return runCaseBuilderAction(
    `/casebuilder/providers/assemblyai/transcripts${query ? `?${query}` : ""}`,
    {
      method: "GET",
      normalize: normalizeAssemblyAiTranscriptListResponse,
    },
  )
}

export function deleteAssemblyAiTranscript(
  transcriptId: string,
): Promise<ActionState<AssemblyAiTranscriptDeleteResponse>> {
  return runCaseBuilderAction(
    `/casebuilder/providers/assemblyai/transcripts/${encodeURIComponent(transcriptId)}`,
    {
      method: "DELETE",
      normalize: normalizeAssemblyAiTranscriptDeleteResponse,
    },
  )
}

export function syncTranscription(
  matterId: string,
  documentId: string,
  transcriptionJobId: string,
): Promise<ActionState<TranscriptionJobResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/transcriptions/${encodeURIComponent(transcriptionJobId)}/sync`,
    {
      method: "POST",
      normalize: normalizeTranscriptionJobResponse,
    },
  )
}

export function patchTranscriptSegment(
  matterId: string,
  documentId: string,
  transcriptionJobId: string,
  segmentId: string,
  input: PatchTranscriptSegmentInput,
): Promise<ActionState<TranscriptionJobResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/transcriptions/${encodeURIComponent(transcriptionJobId)}/segments/${encodeURIComponent(segmentId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeTranscriptionJobResponse,
    },
  )
}

export function patchTranscriptSpeaker(
  matterId: string,
  documentId: string,
  transcriptionJobId: string,
  speakerId: string,
  input: PatchTranscriptSpeakerInput,
): Promise<ActionState<TranscriptionJobResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/transcriptions/${encodeURIComponent(transcriptionJobId)}/speakers/${encodeURIComponent(speakerId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeTranscriptionJobResponse,
    },
  )
}

export function reviewTranscription(
  matterId: string,
  documentId: string,
  transcriptionJobId: string,
  input: ReviewTranscriptionInput,
): Promise<ActionState<TranscriptionJobResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/transcriptions/${encodeURIComponent(transcriptionJobId)}/review`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeTranscriptionJobResponse,
    },
  )
}

export function createParty(matterId: string, input: CreatePartyInput): Promise<ActionState<MatterParty>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/parties`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeParty,
  })
}

export function createFact(matterId: string, input: CreateFactInput): Promise<ActionState<ExtractedFact>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/facts`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeFact,
  })
}

export function patchFact(
  matterId: string,
  factId: string,
  input: PatchFactInput,
): Promise<ActionState<ExtractedFact>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/facts/${encodeURIComponent(factId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeFact,
    },
  )
}

export function approveFact(matterId: string, factId: string): Promise<ActionState<ExtractedFact>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/facts/${encodeURIComponent(factId)}/approve`,
    {
      method: "POST",
      normalize: normalizeFact,
    },
  )
}

export function createTimelineEvent(
  matterId: string,
  input: CreateTimelineEventInput,
): Promise<ActionState<TimelineEvent>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/timeline`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeTimelineEvent,
  })
}

export async function listTimelineSuggestions(matterId: string): Promise<TimelineSuggestion[]> {
  const live = await fetchCaseBuilder<unknown[]>(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/timeline/suggestions`,
  )
  return array(live).map(normalizeTimelineSuggestion)
}

export async function listTimelineAgentRuns(matterId: string): Promise<TimelineAgentRun[]> {
  const live = await fetchCaseBuilder<unknown[]>(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/timeline/agent-runs`,
  )
  return array(live).map(normalizeTimelineAgentRun)
}

export async function getTimelineAgentRun(matterId: string, agentRunId: string): Promise<TimelineAgentRun> {
  const live = await fetchCaseBuilder<unknown>(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/timeline/agent-runs/${encodeURIComponent(agentRunId)}`,
  )
  return normalizeTimelineAgentRun(live)
}

export function suggestTimeline(
  matterId: string,
  input: TimelineSuggestInput,
): Promise<ActionState<TimelineSuggestResponse>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/timeline/suggest`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeTimelineSuggestResponse,
  })
}

export function patchTimelineSuggestion(
  matterId: string,
  suggestionId: string,
  input: PatchTimelineSuggestionInput,
): Promise<ActionState<TimelineSuggestion>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/timeline/suggestions/${encodeURIComponent(suggestionId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeTimelineSuggestion,
    },
  )
}

export function approveTimelineSuggestion(
  matterId: string,
  suggestionId: string,
): Promise<ActionState<TimelineSuggestionApprovalResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/timeline/suggestions/${encodeURIComponent(suggestionId)}/approve`,
    {
      method: "POST",
      normalize: normalizeTimelineSuggestionApprovalResponse,
    },
  )
}

export function createClaim(matterId: string, input: CreateClaimInput): Promise<ActionState<Claim>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/claims`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeClaim,
  })
}

export function mapClaimElements(matterId: string, claimId: string): Promise<ActionState<Claim>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/claims/${encodeURIComponent(claimId)}/map-elements`,
    {
      method: "POST",
      normalize: normalizeClaim,
    },
  )
}

export function createDefense(matterId: string, input: CreateDefenseInput): Promise<ActionState<CaseDefense>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/defenses`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: (raw) => raw as CaseDefense,
  })
}

export function createEvidence(
  matterId: string,
  input: CreateEvidenceInput,
): Promise<ActionState<CaseEvidence>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/evidence`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeEvidence,
  })
}

export function linkEvidenceFact(
  matterId: string,
  evidenceId: string,
  input: LinkEvidenceFactInput,
): Promise<ActionState<CaseEvidence>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/evidence/${encodeURIComponent(evidenceId)}/link-fact`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeEvidence,
    },
  )
}

export function createDeadline(
  matterId: string,
  input: CreateDeadlineInput,
): Promise<ActionState<Deadline>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/deadlines`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeDeadline,
  })
}

export function patchDeadline(
  matterId: string,
  deadlineId: string,
  input: PatchDeadlineInput,
): Promise<ActionState<Deadline>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/deadlines/${encodeURIComponent(deadlineId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeDeadline,
    },
  )
}

export function computeDeadlines(
  matterId: string,
): Promise<ActionState<ComputeDeadlinesResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/deadlines/compute`,
    {
      method: "POST",
      normalize: (raw) => {
        const response = raw as any
        return {
          generated: array(response.generated).map(normalizeDeadline),
          warnings: array(response.warnings),
        }
      },
    },
  )
}

export function createTask(matterId: string, input: CreateTaskInput): Promise<ActionState<CaseTask>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/tasks`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeTask,
  })
}

export function patchTask(
  matterId: string,
  taskId: string,
  input: PatchTaskInput,
): Promise<ActionState<CaseTask>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/tasks/${encodeURIComponent(taskId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeTask,
    },
  )
}

export async function getMatterGraphState(
  matterId: string,
  options: CaseBuilderRequestOptions = {},
): Promise<LoadState<CaseGraphResponse | null>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    const live = await fetchCaseBuilder<unknown>(`/matters/${encodeURIComponent(decodedMatterId)}/graph`, options)
    return { source: "live", data: normalizeCaseGraphResponse(live) }
  } catch (error) {
    return { source: "error", data: null, error: errorMessage(error) }
  }
}

export async function getMatterAuditEventsState(
  matterId: string,
  options: CaseBuilderRequestOptions = {},
): Promise<LoadState<AuditEvent[]>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    const live = await fetchCaseBuilder<unknown[]>(`/matters/${encodeURIComponent(decodedMatterId)}/audit`, options)
    return { source: "live", data: live.map(normalizeAuditEvent) }
  } catch (error) {
    return { source: "error", data: [], error: errorMessage(error) }
  }
}

export function runMatterQc(matterId: string): Promise<ActionState<QcRun>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/qc/run`, {
    method: "POST",
    normalize: normalizeQcRun,
  })
}

export function spotIssues(
  matterId: string,
  input: { mode?: string; limit?: number } = {},
): Promise<ActionState<IssueSpotResponse>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/issues/spot`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeIssueSpotResponse,
  })
}

export function exportMatterPackage(
  matterId: string,
  format: "docx" | "pdf" | "filing_packet" | string,
): Promise<ActionState<CaseAiActionResponse<ExportPackage>>> {
  const pathFormat = format === "filing_packet" ? "filing-packet" : format
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/export/${pathFormat}`, {
    method: "POST",
    normalize: (raw) => normalizeAiAction(raw, normalizeExportPackage),
  })
}

export function createDraft(matterId: string, input: CreateDraftInput): Promise<ActionState<Draft>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/drafts`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeDraft,
  })
}

export function patchDraft(
  matterId: string,
  draftId: string,
  input: PatchDraftInput,
): Promise<ActionState<Draft>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/drafts/${encodeURIComponent(draftId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(serializeDraftPatch(input)),
      normalize: normalizeDraft,
    },
  )
}

export function askMatter(
  matterId: string,
  input: MatterAskInput,
): Promise<ActionState<MatterAskResponse>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/ask`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeMatterAskResponse,
  })
}

export function generateDraft(
  matterId: string,
  draftId: string,
): Promise<ActionState<CaseAiActionResponse<Draft>>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/drafts/${encodeURIComponent(draftId)}/generate`,
    {
      method: "POST",
      normalize: (raw) => normalizeAiAction(raw, normalizeDraft),
    },
  )
}

export function factCheckDraft(
  matterId: string,
  draftId: string,
): Promise<ActionState<CaseAiActionResponse<CaseFactCheckFinding[]>>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/drafts/${encodeURIComponent(draftId)}/fact-check`,
    {
      method: "POST",
      normalize: (raw) =>
        normalizeAiAction(raw, (items) => array(items).map((item) => item as CaseFactCheckFinding)),
    },
  )
}

export function citationCheckDraft(
  matterId: string,
  draftId: string,
): Promise<ActionState<CaseAiActionResponse<CaseCitationCheckFinding[]>>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/drafts/${encodeURIComponent(draftId)}/citation-check`,
    {
      method: "POST",
      normalize: (raw) =>
        normalizeAiAction(raw, (items) => array(items).map((item) => item as CaseCitationCheckFinding)),
    },
  )
}

function workProductListQuery(options: GetWorkProductsOptions = {}) {
  if (!options.includeDocumentAst) return ""
  return "?include=document_ast"
}

export async function getWorkProductsState(
  matterId: string,
  options: GetWorkProductsOptions = {},
): Promise<LoadState<WorkProduct[]>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    const query = workProductListQuery(options)
    const live = await fetchCaseBuilder<unknown[]>(
      `/matters/${encodeURIComponent(decodedMatterId)}/work-products${query}`,
      options.request,
    )
    return { source: "live", data: live.map(normalizeWorkProduct) }
  } catch (error) {
    if (!shouldUseDemoMatterFallback(error, { allowNotFound: true })) {
      return { source: "error", data: [], error: errorMessage(error) }
    }
    const demo = getDemoMatterById(decodedMatterId)
    return {
      source: demo ? "demo" : "error",
      data: demo ? buildDemoWorkProducts(demo) : [],
      error: errorMessage(error),
    }
  }
}

export async function getWorkProductState(
  matterId: string,
  workProductId?: string,
  options: GetWorkProductsOptions = {},
): Promise<LoadState<WorkProduct | null>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  const decodedWorkProductId = workProductId ? decodeRouteSegment(workProductId) : undefined
  try {
    if (decodedWorkProductId) {
      const live = await fetchCaseBuilder<unknown>(
        `/matters/${encodeURIComponent(decodedMatterId)}/work-products/${encodeURIComponent(decodedWorkProductId)}`,
        options.request,
      )
      return { source: "live", data: normalizeWorkProduct(live) }
    }
    const query = workProductListQuery(options)
    const live = await fetchCaseBuilder<unknown[]>(
      `/matters/${encodeURIComponent(decodedMatterId)}/work-products${query}`,
      options.request,
    )
    const products = live.map(normalizeWorkProduct).filter((product) => product.product_type !== "complaint")
    return { source: "live", data: products[0] ?? null }
  } catch (error) {
    if (!shouldUseDemoMatterFallback(error, { allowNotFound: true })) {
      return { source: "error", data: null, error: errorMessage(error) }
    }
    const demo = getDemoMatterById(decodedMatterId)
    const products = demo ? buildDemoWorkProducts(demo) : []
    return {
      source: demo ? "demo" : "error",
      data: decodedWorkProductId
        ? products.find((product) => product.id === decodedWorkProductId || product.work_product_id === decodedWorkProductId) ?? null
        : products[0] ?? null,
      error: errorMessage(error),
    }
  }
}

export function createWorkProduct(
  matterId: string,
  input: CreateWorkProductInput,
): Promise<ActionState<WorkProduct>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeWorkProduct,
  })
}

export function patchWorkProduct(
  matterId: string,
  workProductId: string,
  input: PatchWorkProductInput,
): Promise<ActionState<WorkProduct>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeWorkProduct,
    },
  )
}

export function createWorkProductBlock(
  matterId: string,
  workProductId: string,
  input: CreateWorkProductBlockInput,
): Promise<ActionState<WorkProduct>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/blocks`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeWorkProduct,
    },
  )
}

export function patchWorkProductBlock(
  matterId: string,
  workProductId: string,
  blockId: string,
  input: PatchWorkProductBlockInput,
): Promise<ActionState<WorkProduct>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/blocks/${encodeURIComponent(blockId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeWorkProduct,
    },
  )
}

export function linkWorkProductSupport(
  matterId: string,
  workProductId: string,
  input: WorkProductLinkInput,
): Promise<ActionState<WorkProduct>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/links`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeWorkProduct,
    },
  )
}

export function patchWorkProductSupport(
  matterId: string,
  workProductId: string,
  anchorId: string,
  input: PatchWorkProductSupportInput,
): Promise<ActionState<WorkProduct>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/links/${encodeURIComponent(anchorId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeWorkProduct,
    },
  )
}

export function deleteWorkProductSupport(
  matterId: string,
  workProductId: string,
  anchorId: string,
): Promise<ActionState<WorkProduct>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/links/${encodeURIComponent(anchorId)}`,
    {
      method: "DELETE",
      normalize: normalizeWorkProduct,
    },
  )
}

export function linkWorkProductTextRange(
  matterId: string,
  workProductId: string,
  input: WorkProductTextRangeLinkInput,
): Promise<ActionState<WorkProduct>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/text-ranges`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeWorkProduct,
    },
  )
}

export function applyWorkProductAstPatch(
  matterId: string,
  workProductId: string,
  input: AstPatch & AstPatchConcurrency,
): Promise<ActionState<WorkProduct>> {
  if (!input.base_document_hash && !input.base_snapshot_id) {
    return Promise.resolve({
      source: "error",
      data: null,
      error: "AST patch requires base_document_hash or base_snapshot_id.",
    })
  }
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/ast/patch`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeWorkProduct,
    },
  )
}

export function getWorkProductAst(
  matterId: string,
  workProductId: string,
): Promise<ActionState<WorkProductDocument>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/ast`,
    {
      method: "GET",
      normalize: (raw) =>
        normalizeWorkProductDocument(raw, {
          workProductId,
          matterId: decodeMatterRouteId(matterId),
          productType: "custom",
          title: "Work product",
          fallbackBlocks: [],
          fallbackFindings: [],
        }),
    },
  )
}

export function patchWorkProductAst(
  matterId: string,
  workProductId: string,
  input: WorkProductDocument,
): Promise<ActionState<WorkProduct>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/ast`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeWorkProduct,
    },
  )
}

export function validateWorkProductAst(
  matterId: string,
  workProductId: string,
): Promise<ActionState<AstValidationResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/ast/validate`,
    {
      method: "POST",
      normalize: normalizeAstValidationResponse,
    },
  )
}

export function workProductAstToMarkdown(
  matterId: string,
  workProductId: string,
): Promise<ActionState<AstMarkdownResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/ast/to-markdown`,
    {
      method: "POST",
      normalize: normalizeAstMarkdownResponse,
    },
  )
}

export function workProductAstFromMarkdown(
  matterId: string,
  workProductId: string,
  input: MarkdownToAstInput,
): Promise<ActionState<AstDocumentResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/ast/from-markdown`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeAstDocumentResponse,
    },
  )
}

export function workProductAstToHtml(
  matterId: string,
  workProductId: string,
): Promise<ActionState<AstRenderedResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/ast/to-html`,
    {
      method: "POST",
      normalize: normalizeAstRenderedResponse,
    },
  )
}

export function workProductAstToPlainText(
  matterId: string,
  workProductId: string,
): Promise<ActionState<AstRenderedResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/ast/to-plain-text`,
    {
      method: "POST",
      normalize: normalizeAstRenderedResponse,
    },
  )
}

export function runWorkProductQc(
  matterId: string,
  workProductId: string,
): Promise<ActionState<CaseAiActionResponse<WorkProductFinding[]>>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/qc/run`,
    {
      method: "POST",
      normalize: (raw) => normalizeAiAction(raw, (items) => array(items).map(normalizeWorkProductFinding)),
    },
  )
}

export function patchWorkProductFinding(
  matterId: string,
  workProductId: string,
  findingId: string,
  status: string,
): Promise<ActionState<WorkProduct>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/qc/findings/${encodeURIComponent(findingId)}`,
    {
      method: "PATCH",
      body: JSON.stringify({ status }),
      normalize: normalizeWorkProduct,
    },
  )
}

export function previewWorkProduct(
  matterId: string,
  workProductId: string,
): Promise<ActionState<WorkProductPreviewResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/preview`,
    {
      method: "GET",
      normalize: normalizeWorkProductPreview,
    },
  )
}

export function exportWorkProduct(
  matterId: string,
  workProductId: string,
  input: ExportWorkProductInput,
): Promise<ActionState<WorkProductArtifact>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/export`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeWorkProductArtifact,
    },
  )
}

export function runWorkProductAiCommand(
  matterId: string,
  workProductId: string,
  input: WorkProductAiCommandInput,
): Promise<ActionState<CaseAiActionResponse<WorkProduct>>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/ai/commands`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: (raw) => normalizeAiAction(raw, normalizeWorkProduct),
    },
  )
}

export async function getWorkProductHistory(
  matterId: string,
  workProductId: string,
): Promise<LoadState<ChangeSet[]>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    const live = await fetchCaseBuilder<unknown[]>(
      `/matters/${encodeURIComponent(decodedMatterId)}/work-products/${encodeURIComponent(workProductId)}/history`,
    )
    return { source: "live", data: live.map(normalizeChangeSet) }
  } catch (error) {
    return { source: "error", data: [], error: errorMessage(error) }
  }
}

export async function getWorkProductChangeSet(
  matterId: string,
  workProductId: string,
  changeSetId: string,
): Promise<LoadState<ChangeSet | null>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    const live = await fetchCaseBuilder<unknown>(
      `/matters/${encodeURIComponent(decodedMatterId)}/work-products/${encodeURIComponent(workProductId)}/change-sets/${encodeURIComponent(changeSetId)}`,
    )
    return { source: "live", data: normalizeChangeSet(live) }
  } catch (error) {
    return { source: "error", data: null, error: errorMessage(error) }
  }
}

export async function getWorkProductSnapshots(
  matterId: string,
  workProductId: string,
): Promise<LoadState<VersionSnapshot[]>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    const live = await fetchCaseBuilder<unknown[]>(
      `/matters/${encodeURIComponent(decodedMatterId)}/work-products/${encodeURIComponent(workProductId)}/snapshots`,
    )
    return { source: "live", data: live.map(normalizeVersionSnapshot) }
  } catch (error) {
    return { source: "error", data: [], error: errorMessage(error) }
  }
}

export async function getWorkProductSnapshot(
  matterId: string,
  workProductId: string,
  snapshotId: string,
): Promise<LoadState<VersionSnapshot | null>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    const live = await fetchCaseBuilder<unknown>(
      `/matters/${encodeURIComponent(decodedMatterId)}/work-products/${encodeURIComponent(workProductId)}/snapshots/${encodeURIComponent(snapshotId)}`,
    )
    return { source: "live", data: normalizeVersionSnapshot(live) }
  } catch (error) {
    return { source: "error", data: null, error: errorMessage(error) }
  }
}

export function createWorkProductSnapshot(
  matterId: string,
  workProductId: string,
  input: CreateVersionSnapshotInput = {},
): Promise<ActionState<VersionSnapshot>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/snapshots`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeVersionSnapshot,
    },
  )
}

export function compareWorkProductVersions(
  matterId: string,
  workProductId: string,
  input: CompareWorkProductVersionsInput,
): Promise<ActionState<CompareVersionsResponse>> {
  const params = new URLSearchParams({ from: input.from })
  if (input.to) params.set("to", input.to)
  if (input.layers?.length) params.set("layers", input.layers.join(","))
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/compare?${params.toString()}`,
    {
      method: "GET",
      normalize: normalizeCompareVersionsResponse,
    },
  )
}

export function restoreWorkProductVersion(
  matterId: string,
  workProductId: string,
  input: RestoreVersionInput,
): Promise<ActionState<RestoreVersionResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/work-products/${encodeURIComponent(workProductId)}/restore`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeRestoreVersionResponse,
    },
  )
}

export async function getWorkProductExportHistory(
  matterId: string,
  workProductId: string,
): Promise<LoadState<WorkProductArtifact[]>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    const live = await fetchCaseBuilder<unknown[]>(
      `/matters/${encodeURIComponent(decodedMatterId)}/work-products/${encodeURIComponent(workProductId)}/export-history`,
    )
    return { source: "live", data: live.map(normalizeWorkProductArtifact) }
  } catch (error) {
    return { source: "error", data: [], error: errorMessage(error) }
  }
}

export async function getWorkProductAiAudit(
  matterId: string,
  workProductId: string,
): Promise<LoadState<AIEditAudit[]>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    const live = await fetchCaseBuilder<unknown[]>(
      `/matters/${encodeURIComponent(decodedMatterId)}/work-products/${encodeURIComponent(workProductId)}/ai-audit`,
    )
    return { source: "live", data: live.map(normalizeAIEditAudit) }
  } catch (error) {
    return { source: "error", data: [], error: errorMessage(error) }
  }
}

export async function getComplaintState(
  matterId: string,
  complaintId?: string,
  options: CaseBuilderRequestOptions = {},
): Promise<LoadState<ComplaintDraft | null>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    if (complaintId) {
      const live = await fetchCaseBuilder<unknown>(
        `/matters/${encodeURIComponent(decodedMatterId)}/complaints/${encodeURIComponent(complaintId)}`,
        options,
      )
      return { source: "live", data: normalizeComplaint(live) }
    }
    const live = await fetchCaseBuilder<unknown[]>(
      `/matters/${encodeURIComponent(decodedMatterId)}/complaints`,
      options,
    )
    const complaints = live.map(normalizeComplaint)
    return { source: "live", data: complaints[0] ?? null }
  } catch (error) {
    if (!shouldUseDemoMatterFallback(error, { allowNotFound: true })) {
      return { source: "error", data: null, error: errorMessage(error) }
    }
    const demo = getDemoMatterById(decodedMatterId)
    return {
      source: demo ? "demo" : "error",
      data: demo ? buildDemoComplaint(demo) : null,
      error: errorMessage(error),
    }
  }
}

export function createComplaint(
  matterId: string,
  input: CreateComplaintInput = {},
): Promise<ActionState<ComplaintDraft>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeComplaint,
  })
}

export function importComplaints(
  matterId: string,
  input: ComplaintImportInput,
): Promise<ActionState<ComplaintImportResponse>> {
  return runCaseBuilderAction(`/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/import`, {
    method: "POST",
    body: JSON.stringify(input),
    normalize: normalizeComplaintImportResponse,
  })
}

export function importDocumentComplaint(
  matterId: string,
  documentId: string,
  input: ComplaintImportInput = {},
): Promise<ActionState<ComplaintImportResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/documents/${encodeURIComponent(documentId)}/import-complaint`,
    {
      method: "POST",
      body: JSON.stringify({ ...input, document_id: documentId }),
      normalize: normalizeComplaintImportResponse,
    },
  )
}

export function patchComplaint(
  matterId: string,
  complaintId: string,
  input: PatchComplaintInput,
): Promise<ActionState<ComplaintDraft>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeComplaint,
    },
  )
}

export function createComplaintSection(
  matterId: string,
  complaintId: string,
  input: CreateComplaintSectionInput,
): Promise<ActionState<ComplaintDraft>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/sections`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeComplaint,
    },
  )
}

export function createComplaintCount(
  matterId: string,
  complaintId: string,
  input: CreateComplaintCountInput,
): Promise<ActionState<ComplaintDraft>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/counts`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeComplaint,
    },
  )
}

export function createComplaintParagraph(
  matterId: string,
  complaintId: string,
  input: CreateComplaintParagraphInput,
): Promise<ActionState<ComplaintDraft>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/paragraphs`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeComplaint,
    },
  )
}

export function patchComplaintParagraph(
  matterId: string,
  complaintId: string,
  paragraphId: string,
  input: PatchComplaintParagraphInput,
): Promise<ActionState<ComplaintDraft>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/paragraphs/${encodeURIComponent(paragraphId)}`,
    {
      method: "PATCH",
      body: JSON.stringify(input),
      normalize: normalizeComplaint,
    },
  )
}

export function renumberComplaintParagraphs(
  matterId: string,
  complaintId: string,
): Promise<ActionState<ComplaintDraft>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/paragraphs/renumber`,
    {
      method: "POST",
      normalize: normalizeComplaint,
    },
  )
}

export function linkComplaintSupport(
  matterId: string,
  complaintId: string,
  input: ComplaintLinkInput,
): Promise<ActionState<ComplaintDraft>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/links`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeComplaint,
    },
  )
}

export function runComplaintQc(
  matterId: string,
  complaintId: string,
): Promise<ActionState<CaseAiActionResponse<RuleCheckFinding[]>>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/qc/run`,
    {
      method: "POST",
      normalize: (raw) => normalizeAiAction(raw, (items) => array(items).map((item) => item as RuleCheckFinding)),
    },
  )
}

export function patchComplaintFinding(
  matterId: string,
  complaintId: string,
  findingId: string,
  status: string,
): Promise<ActionState<ComplaintDraft>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/qc/findings/${encodeURIComponent(findingId)}`,
    {
      method: "PATCH",
      body: JSON.stringify({ status }),
      normalize: normalizeComplaint,
    },
  )
}

export function previewComplaint(
  matterId: string,
  complaintId: string,
): Promise<ActionState<ComplaintPreviewResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/preview`,
    {
      method: "GET",
      normalize: (raw) => raw as ComplaintPreviewResponse,
    },
  )
}

export function exportComplaint(
  matterId: string,
  complaintId: string,
  input: ExportComplaintInput,
): Promise<ActionState<ExportArtifact>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/export`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: (raw) => raw as ExportArtifact,
    },
  )
}

export function runComplaintAiCommand(
  matterId: string,
  complaintId: string,
  input: { command: string; target_id?: string; prompt?: string },
): Promise<ActionState<CaseAiActionResponse<ComplaintDraft>>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/complaints/${encodeURIComponent(complaintId)}/ai/commands`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: (raw) => normalizeAiAction(raw, normalizeComplaint),
    },
  )
}

export function searchAuthority(
  matterId: string,
  query: string,
  limit = 10,
): Promise<ActionState<CaseAuthoritySearchResponse>> {
  const params = new URLSearchParams({ q: query, limit: String(limit) })
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/authority/search?${params}`,
    {
      method: "GET",
      normalize: (raw) => raw as CaseAuthoritySearchResponse,
    },
  )
}

export function recommendAuthority(
  matterId: string,
  input: { text: string; claim_id?: string; limit?: number },
): Promise<ActionState<CaseAuthoritySearchResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/authority/recommend`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: (raw) => raw as CaseAuthoritySearchResponse,
    },
  )
}

export function attachAuthority(
  matterId: string,
  input: AuthorityAttachmentInput,
): Promise<ActionState<AuthorityAttachmentResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/authority/attach`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeAuthorityAttachment,
    },
  )
}

export function detachAuthority(
  matterId: string,
  input: AuthorityAttachmentInput,
): Promise<ActionState<AuthorityAttachmentResponse>> {
  return runCaseBuilderAction(
    `/matters/${encodeURIComponent(decodeMatterRouteId(matterId))}/authority/detach`,
    {
      method: "POST",
      body: JSON.stringify(input),
      normalize: normalizeAuthorityAttachment,
    },
  )
}

type CaseBuilderActionOptions<T> = RequestInit & {
  normalize: (raw: unknown) => T
  timeoutMs?: number | null
}

async function runCaseBuilderAction<T>(
  endpoint: string,
  options: CaseBuilderActionOptions<T>,
): Promise<ActionState<T>> {
  const { normalize, ...requestOptions } = options
  try {
    const raw = await fetchCaseBuilder<unknown>(endpoint, requestOptions)
    return { source: "live", data: normalize(raw) }
  } catch (error) {
    return { source: "error", data: null, error: errorMessage(error) }
  }
}

async function fetchCaseBuilder<T>(
  endpoint: string,
  options: RequestInit & { timeoutMs?: number | null } = {},
): Promise<T> {
  const { timeoutMs = API_TIMEOUT_MS, ...fetchOptions } = options
  const headers = new Headers(options.headers)
  if (!headers.has("Content-Type") && typeof options.body === "string") {
    headers.set("Content-Type", "application/json")
  }
  const controller = new AbortController()
  let timedOut = false
  const timeout =
    timeoutMs == null
      ? null
      : setTimeout(() => {
          timedOut = true
          controller.abort()
        }, timeoutMs)
  const parentSignal = options.signal
  const abortFromParent = () => controller.abort()

  if (parentSignal?.aborted) {
    controller.abort()
  } else {
    parentSignal?.addEventListener("abort", abortFromParent, { once: true })
  }

  try {
    const response = await fetch(`${API_BASE_URL}${endpoint}`, {
      cache: "no-store",
      ...fetchOptions,
      signal: controller.signal,
      headers,
    })
    if (!response.ok) {
      const body = await response.json().catch(() => ({}))
      throw new Error(body.error || `CaseBuilder API error: ${response.status}`)
    }
    return response.json()
  } catch (error) {
    if (timedOut && isAbortError(error)) {
      throw new Error(`CaseBuilder API request timed out after ${Math.round((timeoutMs ?? API_TIMEOUT_MS) / 1000)}s`)
    }
    throw error
  } finally {
    if (timeout) clearTimeout(timeout)
    parentSignal?.removeEventListener("abort", abortFromParent)
  }
}

function normalizeAiAction<T>(input: unknown, normalizeResult: (raw: unknown) => T): CaseAiActionResponse<T> {
  const raw = input as any
  return {
    enabled: Boolean(raw.enabled),
    mode: string(raw.mode, "disabled"),
    message: string(raw.message),
    result: raw.result == null ? null : normalizeResult(raw.result),
  }
}

function serializeDraftPatch(input: PatchDraftInput) {
  return {
    ...input,
    sections: input.sections?.map((section) => ({
      section_id: section.id,
      heading: section.heading,
      body: section.body,
      citations: section.citations.map((citation) => ({
        citation: citation.shortLabel,
        canonical_id: citation.sourceId,
        reason: citation.snippet,
        pinpoint: citation.page ? `p.${citation.page}` : undefined,
      })),
    })),
    paragraphs: input.paragraphs?.map((paragraph) => ({
      ...paragraph,
      authorities: paragraph.authorities.map((authority) => ({
        citation: authority.citation,
        canonical_id: authority.canonical_id,
        pinpoint: authority.pinpoint,
      })),
    })),
  }
}

function normalizeMatter(input: unknown): Matter {
  const raw = input as Record<string, any>
  const summary = normalizeMatterSummary(raw)
  const documents = array(raw.documents).map((document) => normalizeDocument(document))
  const facts = array(raw.facts).map((fact) => normalizeFact(fact))
  return {
    ...summary,
    id: string(raw.id, summary.matter_id),
    title: string(raw.title, summary.name),
    shortName: string(raw.shortName, raw.short_name, summary.shortName, summary.name),
    documents,
    parties: array(raw.parties).map(normalizeParty),
    facts,
    timeline: array(raw.timeline).map(normalizeTimelineEvent),
    timeline_suggestions: array(raw.timeline_suggestions, raw.timelineSuggestions).map(normalizeTimelineSuggestion),
    timeline_agent_runs: array(raw.timeline_agent_runs, raw.timelineAgentRuns).map(normalizeTimelineAgentRun),
    claims: array(raw.claims).map(normalizeClaim),
    evidence: array(raw.evidence).map(normalizeEvidence),
    defenses: array(raw.defenses),
    deadlines: array(raw.deadlines).map(normalizeDeadline),
    tasks: array(raw.tasks).map(normalizeTask),
    drafts: array(raw.drafts).map(normalizeDraft),
    work_products: array(raw.work_products, raw.workProducts).map(normalizeWorkProduct),
    fact_check_findings: array(raw.fact_check_findings, raw.factCheckFindings).map(
      normalizeFactCheckFinding,
    ),
    citation_check_findings: array(raw.citation_check_findings, raw.citationCheckFindings).map(
      normalizeCitationCheckFinding,
    ),
    chatHistory: array(raw.chatHistory, raw.chat_history),
    recentThreads: array(raw.recentThreads, raw.recent_threads),
    milestones: array(raw.milestones),
  }
}

function normalizeComplaint(input: unknown): ComplaintDraft {
  const raw = input as Record<string, any>
  const complaintId = string(raw.complaint_id, raw.id, "complaint:demo")
  const matterId = string(raw.matter_id, "matter:demo")
  const caption = raw.caption ?? {}
  const formatting = raw.formatting_profile ?? {}
  return {
    complaint_id: complaintId,
    id: string(raw.id, complaintId),
    matter_id: matterId,
    title: string(raw.title, "Complaint"),
    status: string(raw.status, "draft"),
    review_status: string(raw.review_status, "needs_human_review"),
    setup_stage: string(raw.setup_stage, "guided_setup"),
    active_profile_id: string(raw.active_profile_id, "oregon-circuit-civil-complaint"),
    created_at: string(raw.created_at, ""),
    updated_at: string(raw.updated_at, ""),
    caption: {
      court_name: string(caption.court_name, "Unassigned"),
      county: string(caption.county, ""),
      case_number: caption.case_number ?? null,
      document_title: string(caption.document_title, "Complaint"),
      plaintiff_names: array(caption.plaintiff_names),
      defendant_names: array(caption.defendant_names),
      jury_demand: Boolean(caption.jury_demand),
      jurisdiction: string(caption.jurisdiction, "Oregon"),
      venue: string(caption.venue, ""),
    },
    parties: array(raw.parties),
    sections: array(raw.sections),
    counts: array(raw.counts),
    paragraphs: array(raw.paragraphs),
    relief: array(raw.relief),
    signature: {
      name: string(raw.signature?.name),
      bar_number: raw.signature?.bar_number ?? null,
      firm: raw.signature?.firm ?? null,
      address: string(raw.signature?.address),
      phone: string(raw.signature?.phone),
      email: string(raw.signature?.email),
      signature_date: raw.signature?.signature_date ?? null,
    },
    certificate_of_service: raw.certificate_of_service ?? null,
    formatting_profile: {
      profile_id: string(formatting.profile_id, "oregon-circuit-civil-complaint"),
      name: string(formatting.name, "Oregon Circuit Civil Complaint"),
      jurisdiction: string(formatting.jurisdiction, "Oregon"),
      line_numbers: formatting.line_numbers ?? true,
      double_spaced: formatting.double_spaced ?? true,
      first_page_top_blank_inches: number(formatting.first_page_top_blank_inches, 2),
      margin_top_inches: number(formatting.margin_top_inches, 1),
      margin_bottom_inches: number(formatting.margin_bottom_inches, 1),
      margin_left_inches: number(formatting.margin_left_inches, 1),
      margin_right_inches: number(formatting.margin_right_inches, 1),
      font_family: string(formatting.font_family, "Times New Roman"),
      font_size_pt: number(formatting.font_size_pt, 12),
    },
    rule_pack: raw.rule_pack ?? {
      rule_pack_id: "oregon-circuit-civil-complaint-orcp-utcr",
      name: "Oregon Circuit Civil Complaint - ORCP + UTCR",
      jurisdiction: "Oregon",
      version: "provider-free",
      effective_date: "",
      rules: [],
    },
    findings: array(raw.findings),
    export_artifacts: array(raw.export_artifacts),
    history: array(raw.history),
    next_actions: array(raw.next_actions),
    ai_commands: array(raw.ai_commands),
    filing_packet: raw.filing_packet ?? {
      packet_id: `${complaintId}:packet:filing`,
      matter_id: matterId,
      complaint_id: complaintId,
      status: "review_needed",
      items: [],
      warnings: [],
    },
    import_provenance: raw.import_provenance ?? null,
  }
}

function normalizeComplaintImportResponse(input: unknown): ComplaintImportResponse {
  const raw = input as Record<string, any>
  return {
    matter_id: string(raw.matter_id),
    mode: string(raw.mode, "structured_import"),
    imported: array(raw.imported).map(normalizeComplaintImportResult),
    skipped: array(raw.skipped).map(normalizeComplaintImportResult),
    warnings: array(raw.warnings),
  }
}

function normalizeComplaintImportResult(input: unknown): ComplaintImportResponse["imported"][number] {
  const raw = input as Record<string, any>
  return {
    document_id: string(raw.document_id),
    complaint_id: raw.complaint_id ?? null,
    status: string(raw.status),
    message: string(raw.message),
    parser_id: string(raw.parser_id),
    likely_complaint: Boolean(raw.likely_complaint),
    complaint: raw.complaint ? normalizeComplaint(raw.complaint) : null,
  }
}

function buildDemoComplaint(matter: Matter): ComplaintDraft {
  const complaintId = `complaint:${matter.id}:demo`
  const plaintiffs = matter.parties.filter((party) => ["plaintiff", "petitioner"].includes(party.role))
  const defendants = matter.parties.filter((party) => ["defendant", "respondent"].includes(party.role))
  const factSectionId = `${complaintId}:section:facts`
  const countsSectionId = `${complaintId}:section:counts`
  const paragraphs: PleadingParagraph[] = [
    {
      paragraph_id: `${complaintId}:paragraph:1`,
      id: `${complaintId}:paragraph:1`,
      matter_id: matter.id,
      complaint_id: complaintId,
      section_id: factSectionId,
      count_id: null,
      number: 1,
      ordinal: 1,
      role: "factual_allegation",
      text: matter.facts[0]?.statement || "Plaintiff alleges facts after support is linked and reviewed.",
      sentences: [],
      fact_ids: matter.facts[0] ? [matter.facts[0].id] : [],
      evidence_uses: [],
      citation_uses: [],
      exhibit_references: [],
      rule_finding_ids: [],
      locked: false,
      review_status: "needs_review",
    },
  ]
  const counts: ComplaintCount[] = matter.claims
    .filter((claim) => claim.kind !== "defense")
    .slice(0, 3)
    .map((claim, index) => ({
      count_id: `${complaintId}:count:${index + 1}`,
      id: `${complaintId}:count:${index + 1}`,
      matter_id: matter.id,
      complaint_id: complaintId,
      ordinal: index + 1,
      title: claim.title,
      claim_id: claim.id,
      legal_theory: claim.theory || claim.legal_theory || "",
      against_party_ids: [],
      element_ids: claim.elements.map((element) => element.id),
      fact_ids: claim.fact_ids || claim.supportingFactIds || [],
      evidence_ids: claim.evidence_ids || [],
      authorities: claim.authorities || [],
      relief_ids: [],
      paragraph_ids: [],
      incorporation_range: "1 through preceding paragraph",
      health: "needs_review",
      weaknesses: [],
    }))
  return normalizeComplaint({
    complaint_id: complaintId,
    id: complaintId,
    matter_id: matter.id,
    title: `${matter.shortName || matter.name} complaint`,
    status: "draft",
    review_status: "needs_human_review",
    setup_stage: "guided_setup",
    active_profile_id: "oregon-circuit-civil-complaint",
    created_at: matter.created_at,
    updated_at: matter.updated_at,
    caption: {
      court_name: matter.court,
      county: "",
      case_number: matter.case_number,
      document_title: "Complaint",
      plaintiff_names: plaintiffs.length ? plaintiffs.map((party) => party.name) : ["Plaintiff"],
      defendant_names: defendants.length ? defendants.map((party) => party.name) : ["Defendant"],
      jury_demand: false,
      jurisdiction: matter.jurisdiction,
      venue: matter.court,
    },
    parties: matter.parties.map((party) => ({
      party_id: `${complaintId}:party:${party.id}`,
      matter_party_id: party.id,
      name: party.name,
      role: party.role,
      party_type: party.party_type || party.partyType,
      represented_by: party.represented_by || party.representedBy || null,
    })),
    sections: [
      {
        section_id: factSectionId,
        id: factSectionId,
        matter_id: matter.id,
        complaint_id: complaintId,
        title: "Factual Allegations",
        section_type: "facts",
        ordinal: 1,
        paragraph_ids: paragraphs.map((paragraph) => paragraph.paragraph_id),
        count_ids: [],
        review_status: "needs_review",
      },
      {
        section_id: countsSectionId,
        id: countsSectionId,
        matter_id: matter.id,
        complaint_id: complaintId,
        title: "Claims for Relief",
        section_type: "counts",
        ordinal: 2,
        paragraph_ids: [],
        count_ids: counts.map((count) => count.count_id),
        review_status: "needs_review",
      },
    ],
    counts,
    paragraphs,
    relief: [
      {
        relief_id: `${complaintId}:relief:general`,
        id: `${complaintId}:relief:general`,
        matter_id: matter.id,
        complaint_id: complaintId,
        category: "general",
        text: "Relief requires human review before filing or service.",
        amount: null,
        authority_ids: [],
        supported: false,
      },
    ],
    signature: { name: "", bar_number: null, firm: null, address: "", phone: "", email: "", signature_date: null },
    findings: [],
    next_actions: [
      {
        action_id: "demo:run-qc",
        priority: "info",
        label: "Run QC",
        detail: "Demo complaint loaded because live complaint data is unavailable.",
        action_type: "run_qc",
        target_type: "complaint",
        target_id: complaintId,
        href: null,
      },
    ],
  })
}

function buildDemoWorkProducts(matter: Matter): WorkProduct[] {
  const workProductId = `work-product:${matter.id}:answer-demo`
  const now = matter.updated_at || matter.created_at || ""
  const title = matter.drafts[0]?.title || `${matter.shortName || matter.name} answer`
  const blocks = [
    {
      block_id: `${workProductId}:block:intro`,
      id: `${workProductId}:block:intro`,
      matter_id: matter.id,
      work_product_id: workProductId,
      block_type: "section",
      role: "introduction",
      title: "Introduction",
      text: matter.facts[0]?.statement || "Draft work product is ready for matter review.",
      ordinal: 1,
      parent_block_id: null,
      fact_ids: matter.facts[0] ? [matter.facts[0].id] : [],
      evidence_ids: [],
      authorities: [],
      mark_ids: [],
      locked: false,
      review_status: "needs_review",
      prosemirror_json: null,
    },
    {
      block_id: `${workProductId}:block:claims`,
      id: `${workProductId}:block:claims`,
      matter_id: matter.id,
      work_product_id: workProductId,
      block_type: "section",
      role: "claims_or_defenses",
      title: "Claims and defenses",
      text: matter.claims[0]?.title || "Claims and defenses will populate as the matter graph develops.",
      ordinal: 2,
      parent_block_id: null,
      fact_ids: matter.claims[0]?.fact_ids || matter.claims[0]?.supportingFactIds || [],
      evidence_ids: matter.claims[0]?.evidence_ids || [],
      authorities: matter.claims[0]?.authorities || [],
      mark_ids: [],
      locked: false,
      review_status: "needs_review",
      prosemirror_json: null,
    },
  ]

  return [
    normalizeWorkProduct({
      work_product_id: workProductId,
      id: workProductId,
      matter_id: matter.id,
      title,
      product_type: "answer",
      status: "draft",
      review_status: "needs_human_review",
      setup_stage: "guided_setup",
      source_draft_id: matter.drafts[0]?.id ?? null,
      source_complaint_id: null,
      created_at: now,
      updated_at: now,
      profile: {
        profile_id: "work-product-answer-v1",
        product_type: "answer",
        name: "Answer",
        jurisdiction: matter.jurisdiction,
        version: "demo",
        route_slug: "answer",
        required_block_roles: ["introduction", "claims_or_defenses"],
        optional_block_roles: [],
        supports_rich_text: true,
      },
      blocks,
      marks: [],
      anchors: [],
      findings: [],
      artifacts: [],
      history: [],
      ai_commands: [],
      formatting_profile: {},
      rule_pack: {},
    }),
  ]
}

function normalizeMatterSummary(input: any): MatterSummary {
  return {
    matter_id: string(input.matter_id, input.id),
    name: string(input.name, input.title, "Untitled matter"),
    shortName: optionalString(input.shortName, input.short_name),
    matter_type: string(input.matter_type, "civil") as MatterSummary["matter_type"],
    status: string(input.status, "intake") as MatterSummary["status"],
    user_role: string(input.user_role, "neutral") as MatterSummary["user_role"],
    jurisdiction: string(input.jurisdiction, "Oregon"),
    court: string(input.court, "Unassigned"),
    case_number: optionalString(input.case_number) ?? null,
    owner_subject: optionalString(input.owner_subject) ?? null,
    owner_email: optionalString(input.owner_email) ?? null,
    owner_name: optionalString(input.owner_name) ?? null,
    created_by_subject: optionalString(input.created_by_subject) ?? null,
    created_at: string(input.created_at, input.createdAt, ""),
    updated_at: string(input.updated_at, input.updatedAt, ""),
    document_count: number(input.document_count),
    fact_count: number(input.fact_count),
    evidence_count: number(input.evidence_count),
    claim_count: number(input.claim_count),
    draft_count: number(input.draft_count),
    open_task_count: number(input.open_task_count),
    next_deadline: input.next_deadline ?? null,
  }
}

function normalizeCaseBuilderSettingsPrincipal(input: any): CaseBuilderSettingsPrincipal {
  return {
    subject: string(input?.subject),
    email: input?.email ?? null,
    name: input?.name ?? null,
    roles: array(input?.roles),
    is_service: Boolean(input?.is_service ?? input?.isService),
  }
}

function normalizeCaseBuilderUserSettings(input: any): CaseBuilderUserSettings {
  return {
    settings_id: string(input?.settings_id, input?.settingsId),
    subject: string(input?.subject),
    workspace_label: input?.workspace_label ?? input?.workspaceLabel ?? null,
    display_name: input?.display_name ?? input?.displayName ?? null,
    default_matter_type: string(input?.default_matter_type, input?.defaultMatterType, "civil") as MatterSummary["matter_type"],
    default_user_role: string(input?.default_user_role, input?.defaultUserRole, "neutral") as MatterSummary["user_role"],
    default_jurisdiction: string(input?.default_jurisdiction, input?.defaultJurisdiction, "Oregon"),
    default_court: string(input?.default_court, input?.defaultCourt, "Unassigned"),
    default_confidentiality: string(input?.default_confidentiality, input?.defaultConfidentiality, "private"),
    default_document_type: string(input?.default_document_type, input?.defaultDocumentType, "other") as CaseBuilderUserSettings["default_document_type"],
    auto_index_uploads: booleanWithDefault(input?.auto_index_uploads ?? input?.autoIndexUploads, true),
    auto_import_complaints: booleanWithDefault(input?.auto_import_complaints ?? input?.autoImportComplaints, true),
    preserve_folder_paths: booleanWithDefault(input?.preserve_folder_paths ?? input?.preserveFolderPaths, true),
    timeline_suggestions_enabled: booleanWithDefault(
      input?.timeline_suggestions_enabled ?? input?.timelineSuggestionsEnabled,
      true,
    ),
    ai_timeline_enrichment_enabled: booleanWithDefault(
      input?.ai_timeline_enrichment_enabled ?? input?.aiTimelineEnrichmentEnabled,
      true,
    ),
    transcript_redact_pii: booleanWithDefault(input?.transcript_redact_pii ?? input?.transcriptRedactPii, true),
    transcript_speaker_labels: booleanWithDefault(input?.transcript_speaker_labels ?? input?.transcriptSpeakerLabels, true),
    transcript_default_view: string(input?.transcript_default_view, input?.transcriptDefaultView, "redacted"),
    transcript_prompt_preset: string(input?.transcript_prompt_preset, input?.transcriptPromptPreset, "unclear"),
    transcript_remove_audio_tags: booleanWithDefault(
      input?.transcript_remove_audio_tags ?? input?.transcriptRemoveAudioTags,
      true,
    ),
    export_default_format: string(input?.export_default_format, input?.exportDefaultFormat, "pdf"),
    export_include_exhibits: booleanWithDefault(input?.export_include_exhibits ?? input?.exportIncludeExhibits, true),
    export_include_qc_report: booleanWithDefault(input?.export_include_qc_report ?? input?.exportIncludeQcReport, true),
    created_at: string(input?.created_at, input?.createdAt),
    updated_at: string(input?.updated_at, input?.updatedAt),
  }
}

function normalizeCaseBuilderMatterSettings(input: any): CaseBuilderMatterSettings {
  return {
    settings_id: string(input?.settings_id, input?.settingsId),
    matter_id: string(input?.matter_id, input?.matterId),
    owner_subject: input?.owner_subject ?? input?.ownerSubject ?? null,
    default_confidentiality: input?.default_confidentiality ?? input?.defaultConfidentiality ?? null,
    default_document_type: input?.default_document_type ?? input?.defaultDocumentType ?? null,
    auto_index_uploads: nullableBoolean(input?.auto_index_uploads ?? input?.autoIndexUploads),
    auto_import_complaints: nullableBoolean(input?.auto_import_complaints ?? input?.autoImportComplaints),
    preserve_folder_paths: nullableBoolean(input?.preserve_folder_paths ?? input?.preserveFolderPaths),
    timeline_suggestions_enabled: nullableBoolean(input?.timeline_suggestions_enabled ?? input?.timelineSuggestionsEnabled),
    ai_timeline_enrichment_enabled: nullableBoolean(input?.ai_timeline_enrichment_enabled ?? input?.aiTimelineEnrichmentEnabled),
    transcript_redact_pii: nullableBoolean(input?.transcript_redact_pii ?? input?.transcriptRedactPii),
    transcript_speaker_labels: nullableBoolean(input?.transcript_speaker_labels ?? input?.transcriptSpeakerLabels),
    transcript_default_view: input?.transcript_default_view ?? input?.transcriptDefaultView ?? null,
    transcript_prompt_preset: input?.transcript_prompt_preset ?? input?.transcriptPromptPreset ?? null,
    transcript_remove_audio_tags: nullableBoolean(input?.transcript_remove_audio_tags ?? input?.transcriptRemoveAudioTags),
    export_default_format: input?.export_default_format ?? input?.exportDefaultFormat ?? null,
    export_include_exhibits: nullableBoolean(input?.export_include_exhibits ?? input?.exportIncludeExhibits),
    export_include_qc_report: nullableBoolean(input?.export_include_qc_report ?? input?.exportIncludeQcReport),
    created_at: string(input?.created_at, input?.createdAt),
    updated_at: string(input?.updated_at, input?.updatedAt),
  }
}

function normalizeCaseBuilderEffectiveSettings(input: any): CaseBuilderEffectiveSettings {
  return {
    default_confidentiality: string(input?.default_confidentiality, input?.defaultConfidentiality, "private"),
    default_document_type: string(input?.default_document_type, input?.defaultDocumentType, "other") as CaseBuilderEffectiveSettings["default_document_type"],
    auto_index_uploads: booleanWithDefault(input?.auto_index_uploads ?? input?.autoIndexUploads, true),
    auto_import_complaints: booleanWithDefault(input?.auto_import_complaints ?? input?.autoImportComplaints, true),
    preserve_folder_paths: booleanWithDefault(input?.preserve_folder_paths ?? input?.preserveFolderPaths, true),
    timeline_suggestions_enabled: booleanWithDefault(
      input?.timeline_suggestions_enabled ?? input?.timelineSuggestionsEnabled,
      true,
    ),
    ai_timeline_enrichment_enabled: booleanWithDefault(
      input?.ai_timeline_enrichment_enabled ?? input?.aiTimelineEnrichmentEnabled,
      true,
    ),
    transcript_redact_pii: booleanWithDefault(input?.transcript_redact_pii ?? input?.transcriptRedactPii, true),
    transcript_speaker_labels: booleanWithDefault(input?.transcript_speaker_labels ?? input?.transcriptSpeakerLabels, true),
    transcript_default_view: string(input?.transcript_default_view, input?.transcriptDefaultView, "redacted"),
    transcript_prompt_preset: string(input?.transcript_prompt_preset, input?.transcriptPromptPreset, "unclear"),
    transcript_remove_audio_tags: booleanWithDefault(
      input?.transcript_remove_audio_tags ?? input?.transcriptRemoveAudioTags,
      true,
    ),
    export_default_format: string(input?.export_default_format, input?.exportDefaultFormat, "pdf"),
    export_include_exhibits: booleanWithDefault(input?.export_include_exhibits ?? input?.exportIncludeExhibits, true),
    export_include_qc_report: booleanWithDefault(input?.export_include_qc_report ?? input?.exportIncludeQcReport, true),
  }
}

function normalizeCaseBuilderUserSettingsResponse(input: any): CaseBuilderUserSettingsResponse {
  return {
    principal: normalizeCaseBuilderSettingsPrincipal(input?.principal ?? {}),
    settings: normalizeCaseBuilderUserSettings(input?.settings ?? {}),
  }
}

function normalizeCaseBuilderMatterSettingsResponse(input: any): CaseBuilderMatterSettingsResponse {
  return {
    matter: normalizeMatterSummary(input?.matter ?? {}),
    settings: normalizeCaseBuilderMatterSettings(input?.settings ?? {}),
    effective: normalizeCaseBuilderEffectiveSettings(input?.effective ?? {}),
  }
}

function normalizeDocument(input: any): CaseDocument {
  const documentId = string(input.document_id, input.id)
  const bytes = number(input.bytes)
  const extractedText = optionalString(input.extracted_text)
  return {
    ...input,
    id: documentId,
    document_id: documentId,
    title: string(input.title, titleFromFilename(input.filename), documentId),
    filename: string(input.filename, documentId),
    kind: string(input.kind, input.document_type, "other") as CaseDocument["kind"],
    document_type: string(input.document_type, input.kind, "other") as CaseDocument["document_type"],
    pages: number(input.pages, input.pageCount, 1),
    pageCount: number(input.pageCount, input.pages, 1),
    bytes,
    fileSize: string(input.fileSize, formatBytes(bytes)),
    dateUploaded: string(input.dateUploaded, input.uploaded_at, input.created_at, ""),
    dateFiled: optionalString(input.dateFiled, input.date_observed),
    summary: string(input.summary, "No summary available."),
    status: string(input.status, input.processing_status, "queued") as CaseDocument["status"],
    processing_status: string(input.processing_status, input.status, "queued") as CaseDocument["processing_status"],
    is_exhibit: Boolean(input.is_exhibit),
    facts_extracted: number(input.facts_extracted),
    citations_found: number(input.citations_found),
    contradictions_flagged: number(input.contradictions_flagged),
    entities: array(input.entities),
    chunks: array(input.chunks).length
      ? array(input.chunks).map(normalizeDocumentChunk)
      : extractedText
        ? [
            {
              id: `chunk:${documentId}:1`,
              chunk_id: `chunk:${documentId}:1`,
              document_id: documentId,
              page: 1,
              text: extractedText,
              tokens: Math.ceil(extractedText.length / 4),
            },
          ]
        : [],
    clauses: array(input.clauses),
    linkedFacts: array(input.linkedFacts, input.linked_facts),
    issues: array(input.issues),
    uploaded_at: string(input.uploaded_at, input.dateUploaded, ""),
    parties_mentioned: array(input.parties_mentioned),
    entities_mentioned: array(input.entities_mentioned),
    folder: string(input.folder, "Uploads"),
    storage_provider: optionalString(input.storage_provider) ?? "local",
    storage_status: string(input.storage_status, input.storageStatus, "stored") as CaseDocument["storage_status"],
    storage_bucket: input.storage_bucket ?? input.storageBucket ?? null,
    storage_key: input.storage_key ?? input.storageKey ?? null,
    content_etag: input.content_etag ?? input.contentEtag ?? null,
    upload_expires_at: input.upload_expires_at ?? input.uploadExpiresAt ?? null,
    deleted_at: input.deleted_at ?? input.deletedAt ?? null,
    library_path: input.library_path ?? input.libraryPath ?? input.original_relative_path ?? input.originalRelativePath ?? null,
    archived_at: input.archived_at ?? input.archivedAt ?? null,
    archived_reason: input.archived_reason ?? input.archivedReason ?? null,
    original_relative_path: input.original_relative_path ?? input.originalRelativePath ?? null,
    upload_batch_id: input.upload_batch_id ?? input.uploadBatchId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    current_version_id: input.current_version_id ?? input.currentVersionId ?? null,
    ingestion_run_ids: array(input.ingestion_run_ids, input.ingestionRunIds),
    source_spans: array(input.source_spans, input.sourceSpans).map(normalizeSourceSpan),
    extracted_text: extractedText,
  }
}

function normalizeDocumentWorkspace(input: any): DocumentWorkspace {
  const matterId = string(input.matter_id)
  const document = normalizeDocument(input.document)
  return {
    matter_id: matterId,
    document,
    current_version: input.current_version ? normalizeDocumentVersion(input.current_version) : null,
    capabilities: array(input.capabilities).map(normalizeDocumentCapability),
    annotations: array(input.annotations).map(normalizeDocumentAnnotation),
    source_spans: array(input.source_spans, input.sourceSpans).map(normalizeSourceSpan),
    markdown_ast_document:
      input.markdown_ast_document || input.markdownAstDocument
        ? normalizeMarkdownAstDocument(input.markdown_ast_document ?? input.markdownAstDocument)
        : null,
    markdown_ast_nodes: array(input.markdown_ast_nodes, input.markdownAstNodes).map(normalizeMarkdownAstNode),
    markdown_semantic_units: array(input.markdown_semantic_units, input.markdownSemanticUnits).map(normalizeMarkdownSemanticUnit),
    text_chunks: array(input.text_chunks, input.textChunks).map(normalizeTextChunk),
    evidence_spans: array(input.evidence_spans, input.evidenceSpans).map(normalizeEvidenceSpan),
    entity_mentions: array(input.entity_mentions, input.entityMentions).map(normalizeEntityMention),
    entities: array(input.entities).map(normalizeCaseEntity),
    search_index_records: array(input.search_index_records, input.searchIndexRecords).map(normalizeSearchIndexRecord),
    embedding_runs: array(input.embedding_runs, input.embeddingRuns).map(normalizeCaseBuilderEmbeddingRun),
    embedding_records: array(input.embedding_records, input.embeddingRecords).map(normalizeCaseBuilderEmbeddingRecord),
    embedding_coverage: normalizeCaseBuilderEmbeddingCoverage(input.embedding_coverage ?? input.embeddingCoverage ?? {}),
    proposed_facts: array(input.proposed_facts, input.proposedFacts).map(normalizeFact),
    timeline_suggestions: array(input.timeline_suggestions, input.timelineSuggestions).map(normalizeTimelineSuggestion),
    transcriptions: array(input.transcriptions).map(normalizeTranscriptionJobResponse),
    docx_manifest: input.docx_manifest ? normalizeDocxPackageManifest(input.docx_manifest) : null,
    text_content: input.text_content ?? input.textContent ?? null,
    content_url: documentContentProxyUrl(matterId, document.document_id, input.content_url ?? input.contentUrl ?? null),
    warnings: array(input.warnings),
  }
}

function normalizeDocumentCapability(input: any): DocumentCapability {
  return {
    capability: string(input.capability),
    enabled: Boolean(input.enabled),
    mode: string(input.mode),
    reason: input.reason ?? null,
  }
}

function normalizeDocumentAnnotation(input: any): DocumentAnnotation {
  return {
    ...input,
    id: string(input.id, input.annotation_id),
    annotation_id: string(input.annotation_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    annotation_type: string(input.annotation_type, input.annotationType, "note"),
    status: string(input.status, "active"),
    label: string(input.label, "Note"),
    note: input.note ?? null,
    color: input.color ?? null,
    page_range: input.page_range ?? input.pageRange ?? null,
    text_range: input.text_range ?? input.textRange ?? null,
    target_type: input.target_type ?? input.targetType ?? null,
    target_id: input.target_id ?? input.targetId ?? null,
    created_by: string(input.created_by, input.createdBy, "user"),
    created_at: string(input.created_at, input.createdAt),
    updated_at: string(input.updated_at, input.updatedAt),
  }
}

function normalizeDocxPackageManifest(input: any): DocxPackageManifest {
  return {
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    entry_count: number(input.entry_count, input.entryCount),
    text_part_count: number(input.text_part_count, input.textPartCount),
    editable: Boolean(input.editable),
    unsupported_features: array(input.unsupported_features, input.unsupportedFeatures),
    entries: array(input.entries).map((entry: any) => ({
      name: string(entry.name),
      size_bytes: number(entry.size_bytes, entry.sizeBytes),
      compressed_size_bytes: number(entry.compressed_size_bytes, entry.compressedSizeBytes),
      compression: string(entry.compression),
      supported_text_part: Boolean(entry.supported_text_part ?? entry.supportedTextPart),
    })),
    text_preview: input.text_preview ?? input.textPreview ?? null,
  }
}

function demoCapabilitiesForDocument(document: CaseDocument): DocumentCapability[] {
  const filename = document.filename.toLowerCase()
  const mime = (document.mime_type ?? "").toLowerCase()
  if (filename.endsWith(".pdf") || mime === "application/pdf") {
    return [
      { capability: "view", enabled: true, mode: "pdfjs" },
      { capability: "edit", enabled: false, mode: "immutable_pdf_bytes" },
      { capability: "annotate", enabled: true, mode: "graph_sidecar" },
    ]
  }
  if (filename.endsWith(".docx")) {
    return [
      { capability: "view", enabled: true, mode: "custom_docx" },
      { capability: "edit", enabled: true, mode: "ooxml_round_trip_text" },
      { capability: "annotate", enabled: true, mode: "graph_sidecar" },
    ]
  }
  return [
    { capability: "view", enabled: true, mode: "text_or_preview" },
    { capability: "edit", enabled: filename.endsWith(".md") || mime.startsWith("text/"), mode: "source_text" },
    { capability: "annotate", enabled: true, mode: "graph_sidecar" },
  ]
}

function normalizeDocumentChunk(input: any) {
  const id = string(input.id, input.chunk_id)
  const text = string(input.text)
  return {
    ...input,
    id,
    chunk_id: string(input.chunk_id, id),
    document_id: input.document_id ?? input.documentId,
    page: number(input.page, 1),
    text,
    tokens: number(input.tokens, Math.ceil(text.length / 4)),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    source_span_id: input.source_span_id ?? input.sourceSpanId ?? null,
    byte_start: nullableNumber(input.byte_start, input.byteStart),
    byte_end: nullableNumber(input.byte_end, input.byteEnd),
    char_start: nullableNumber(input.char_start, input.charStart),
    char_end: nullableNumber(input.char_end, input.charEnd),
  }
}

function normalizeParty(input: any): MatterParty {
  return {
    ...input,
    id: string(input.id, input.party_id),
    party_id: string(input.party_id, input.id),
    partyType: string(input.partyType, input.party_type, "individual") as MatterParty["partyType"],
    party_type: string(input.party_type, input.partyType, "individual") as MatterParty["party_type"],
    representedBy: input.representedBy ?? input.represented_by ?? null,
    represented_by: input.represented_by ?? input.representedBy ?? null,
    contactEmail: input.contactEmail ?? input.contact_email,
    contact_email: input.contact_email ?? input.contactEmail,
  }
}

function normalizeFact(input: any): ExtractedFact {
  return {
    ...input,
    id: string(input.id, input.fact_id),
    fact_id: string(input.fact_id, input.id),
    statement: string(input.statement, input.text),
    text: string(input.text, input.statement),
    status: string(input.status, "alleged") as ExtractedFact["status"],
    confidence: number(input.confidence, 0.7),
    disputed: Boolean(input.disputed) || string(input.status) === "disputed",
    tags: array(input.tags),
    sourceDocumentIds: array(input.sourceDocumentIds, input.source_document_ids),
    source_evidence_ids: array(input.source_evidence_ids),
    markdown_ast_node_ids: array(input.markdown_ast_node_ids, input.markdownAstNodeIds),
    citations: array(input.citations),
    source_spans: array(input.source_spans, input.sourceSpans).map(normalizeSourceSpan),
  }
}

function normalizeEvidence(input: any): CaseEvidence {
  return {
    ...input,
    evidence_id: string(input.evidence_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id, input.documentId),
    source_span: string(input.source_span, input.sourceSpan, "document"),
    quote: string(input.quote),
    evidence_type: string(input.evidence_type, input.evidenceType, "document_text"),
    strength: string(input.strength, "moderate") as CaseEvidence["strength"],
    confidence: number(input.confidence, 0.75),
    supports_fact_ids: array(input.supports_fact_ids, input.supportsFactIds),
    contradicts_fact_ids: array(input.contradicts_fact_ids, input.contradictsFactIds),
    source_spans: array(input.source_spans, input.sourceSpans).map(normalizeSourceSpan),
  }
}

function normalizeExtractionChunk(input: any) {
  const chunk = normalizeDocumentChunk(input)
  return {
    chunk_id: chunk.chunk_id,
    document_id: string(chunk.document_id),
    page: chunk.page,
    text: chunk.text,
    document_version_id: chunk.document_version_id,
    object_blob_id: chunk.object_blob_id,
    source_span_id: chunk.source_span_id,
    byte_start: chunk.byte_start,
    byte_end: chunk.byte_end,
    char_start: chunk.char_start,
    char_end: chunk.char_end,
    markdown_ast_node_ids: array(input.markdown_ast_node_ids, input.markdownAstNodeIds),
  }
}

function normalizeDocumentVersion(input: any): DocumentVersion {
  return {
    ...input,
    id: string(input.id, input.document_version_id),
    document_version_id: string(input.document_version_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    object_blob_id: string(input.object_blob_id),
    role: string(input.role, "original"),
    artifact_kind: string(input.artifact_kind, "original_upload"),
    source_version_id: input.source_version_id ?? input.sourceVersionId ?? null,
    created_by: string(input.created_by, input.createdBy, "casebuilder"),
    current: Boolean(input.current),
    created_at: string(input.created_at, input.createdAt),
    storage_provider: string(input.storage_provider, input.storageProvider, "local"),
    storage_bucket: input.storage_bucket ?? input.storageBucket ?? null,
    storage_key: string(input.storage_key, input.storageKey),
    sha256: input.sha256 ?? null,
    size_bytes: number(input.size_bytes, input.sizeBytes),
    mime_type: input.mime_type ?? input.mimeType ?? null,
  }
}

function normalizeIngestionRun(input: any): IngestionRun {
  return {
    ...input,
    id: string(input.id, input.ingestion_run_id),
    ingestion_run_id: string(input.ingestion_run_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    input_sha256: input.input_sha256 ?? input.inputSha256 ?? null,
    status: string(input.status, "queued"),
    stage: string(input.stage, "queued"),
    mode: string(input.mode, "deterministic"),
    started_at: string(input.started_at, input.startedAt),
    completed_at: input.completed_at ?? input.completedAt ?? null,
    error_code: input.error_code ?? input.errorCode ?? null,
    error_message: input.error_message ?? input.errorMessage ?? null,
    retryable: Boolean(input.retryable),
    produced_node_ids: array(input.produced_node_ids, input.producedNodeIds),
    produced_object_keys: array(input.produced_object_keys, input.producedObjectKeys),
    parser_id: input.parser_id ?? input.parserId ?? null,
    parser_version: input.parser_version ?? input.parserVersion ?? null,
    chunker_version: input.chunker_version ?? input.chunkerVersion ?? null,
    citation_resolver_version: input.citation_resolver_version ?? input.citationResolverVersion ?? null,
    index_version: input.index_version ?? input.indexVersion ?? null,
  }
}

function normalizeIndexRun(input: any): IndexRun {
  return {
    ...input,
    id: string(input.id, input.index_run_id),
    index_run_id: string(input.index_run_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    ingestion_run_id: input.ingestion_run_id ?? input.ingestionRunId ?? null,
    status: string(input.status, "queued"),
    stage: string(input.stage, "queued"),
    mode: string(input.mode, "deterministic"),
    started_at: string(input.started_at, input.startedAt),
    completed_at: input.completed_at ?? input.completedAt ?? null,
    error_code: input.error_code ?? input.errorCode ?? null,
    error_message: input.error_message ?? input.errorMessage ?? null,
    retryable: Boolean(input.retryable),
    parser_id: input.parser_id ?? input.parserId ?? null,
    parser_version: input.parser_version ?? input.parserVersion ?? null,
    chunker_version: input.chunker_version ?? input.chunkerVersion ?? null,
    citation_resolver_version: input.citation_resolver_version ?? input.citationResolverVersion ?? null,
    index_version: input.index_version ?? input.indexVersion ?? null,
    produced_node_ids: array(input.produced_node_ids, input.producedNodeIds),
    produced_object_keys: array(input.produced_object_keys, input.producedObjectKeys),
    stale: Boolean(input.stale),
  }
}

function normalizeIndexPage(input: any): Page {
  return {
    ...input,
    id: string(input.id, input.page_id),
    page_id: string(input.page_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    ingestion_run_id: input.ingestion_run_id ?? input.ingestionRunId ?? null,
    index_run_id: input.index_run_id ?? input.indexRunId ?? null,
    page_number: number(input.page_number, input.pageNumber, input.page),
    unit_type: string(input.unit_type, input.unitType, "logical_text_page"),
    title: input.title ?? null,
    text_hash: input.text_hash ?? input.textHash ?? null,
    byte_start: nullableNumber(input.byte_start, input.byteStart),
    byte_end: nullableNumber(input.byte_end, input.byteEnd),
    char_start: nullableNumber(input.char_start, input.charStart),
    char_end: nullableNumber(input.char_end, input.charEnd),
    status: string(input.status, "indexed"),
  }
}

function normalizeTextChunk(input: any): TextChunk {
  return {
    ...input,
    id: string(input.id, input.text_chunk_id),
    text_chunk_id: string(input.text_chunk_id, input.id, input.chunk_id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    page_id: input.page_id ?? input.pageId ?? null,
    source_span_id: input.source_span_id ?? input.sourceSpanId ?? null,
    ingestion_run_id: input.ingestion_run_id ?? input.ingestionRunId ?? null,
    index_run_id: input.index_run_id ?? input.indexRunId ?? null,
    ordinal: number(input.ordinal),
    page: number(input.page),
    text_hash: string(input.text_hash, input.textHash),
    text_excerpt: string(input.text_excerpt, input.textExcerpt, input.text),
    token_count: number(input.token_count, input.tokenCount),
    unit_type: input.unit_type ?? input.unitType ?? null,
    structure_path: input.structure_path ?? input.structurePath ?? null,
    markdown_ast_node_ids: array(input.markdown_ast_node_ids, input.markdownAstNodeIds),
    byte_start: nullableNumber(input.byte_start, input.byteStart),
    byte_end: nullableNumber(input.byte_end, input.byteEnd),
    char_start: nullableNumber(input.char_start, input.charStart),
    char_end: nullableNumber(input.char_end, input.charEnd),
    status: string(input.status, "indexed"),
  }
}

function normalizeEvidenceSpan(input: any): EvidenceSpan {
  return {
    ...input,
    id: string(input.id, input.evidence_span_id),
    evidence_span_id: string(input.evidence_span_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    text_chunk_id: input.text_chunk_id ?? input.textChunkId ?? null,
    source_span_id: input.source_span_id ?? input.sourceSpanId ?? null,
    markdown_ast_node_ids: array(input.markdown_ast_node_ids, input.markdownAstNodeIds),
    ingestion_run_id: input.ingestion_run_id ?? input.ingestionRunId ?? null,
    index_run_id: input.index_run_id ?? input.indexRunId ?? null,
    quote_hash: string(input.quote_hash, input.quoteHash),
    quote_excerpt: string(input.quote_excerpt, input.quoteExcerpt, input.quote),
    byte_start: nullableNumber(input.byte_start, input.byteStart),
    byte_end: nullableNumber(input.byte_end, input.byteEnd),
    char_start: nullableNumber(input.char_start, input.charStart),
    char_end: nullableNumber(input.char_end, input.charEnd),
    review_status: string(input.review_status, input.reviewStatus, "unreviewed"),
  }
}

function normalizeEntityMention(input: any): EntityMention {
  return {
    ...input,
    id: string(input.id, input.entity_mention_id),
    entity_mention_id: string(input.entity_mention_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    text_chunk_id: input.text_chunk_id ?? input.textChunkId ?? null,
    source_span_id: input.source_span_id ?? input.sourceSpanId ?? null,
    entity_id: input.entity_id ?? input.entityId ?? null,
    markdown_ast_node_ids: array(input.markdown_ast_node_ids, input.markdownAstNodeIds),
    mention_text: string(input.mention_text, input.mentionText),
    entity_type: string(input.entity_type, input.entityType, "unknown"),
    confidence: number(input.confidence),
    byte_start: nullableNumber(input.byte_start, input.byteStart),
    byte_end: nullableNumber(input.byte_end, input.byteEnd),
    char_start: nullableNumber(input.char_start, input.charStart),
    char_end: nullableNumber(input.char_end, input.charEnd),
    review_status: string(input.review_status, input.reviewStatus, "unreviewed"),
  }
}

function normalizeMarkdownAstDocument(input: any): MarkdownAstDocument {
  return {
    ...input,
    id: string(input.id, input.markdown_ast_document_id),
    markdown_ast_document_id: string(input.markdown_ast_document_id, input.markdownAstDocumentId, input.id),
    matter_id: string(input.matter_id, input.matterId),
    document_id: string(input.document_id, input.documentId),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    ingestion_run_id: input.ingestion_run_id ?? input.ingestionRunId ?? null,
    index_run_id: input.index_run_id ?? input.indexRunId ?? null,
    parser_id: string(input.parser_id, input.parserId, "pulldown-cmark"),
    parser_version: string(input.parser_version, input.parserVersion),
    source_sha256: string(input.source_sha256, input.sourceSha256),
    root_node_id: string(input.root_node_id, input.rootNodeId),
    node_count: number(input.node_count, input.nodeCount),
    semantic_unit_count: number(input.semantic_unit_count, input.semanticUnitCount),
    heading_count: number(input.heading_count, input.headingCount),
    block_count: number(input.block_count, input.blockCount),
    inline_count: number(input.inline_count, input.inlineCount),
    reference_count: number(input.reference_count, input.referenceCount),
    max_depth: number(input.max_depth, input.maxDepth),
    entity_mention_count: number(input.entity_mention_count, input.entityMentionCount),
    citation_count: number(input.citation_count, input.citationCount),
    date_count: number(input.date_count, input.dateCount),
    money_count: number(input.money_count, input.moneyCount),
    graph_schema_version: string(input.graph_schema_version, input.graphSchemaVersion),
    status: string(input.status, "indexed"),
    created_at: string(input.created_at, input.createdAt),
  }
}

function normalizeMarkdownAstNode(input: any): MarkdownAstNode {
  return {
    ...input,
    id: string(input.id, input.markdown_ast_node_id),
    markdown_ast_node_id: string(input.markdown_ast_node_id, input.markdownAstNodeId, input.id),
    matter_id: string(input.matter_id, input.matterId),
    document_id: string(input.document_id, input.documentId),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    ingestion_run_id: input.ingestion_run_id ?? input.ingestionRunId ?? null,
    index_run_id: input.index_run_id ?? input.indexRunId ?? null,
    markdown_ast_document_id: string(input.markdown_ast_document_id, input.markdownAstDocumentId),
    parent_ast_node_id: input.parent_ast_node_id ?? input.parentAstNodeId ?? null,
    previous_ast_node_id: input.previous_ast_node_id ?? input.previousAstNodeId ?? null,
    semantic_unit_id: input.semantic_unit_id ?? input.semanticUnitId ?? null,
    node_kind: string(input.node_kind, input.nodeKind, "text"),
    tag: string(input.tag, "text"),
    ordinal: number(input.ordinal),
    depth: number(input.depth),
    ast_path: string(input.ast_path, input.astPath),
    sibling_index: number(input.sibling_index, input.siblingIndex),
    child_count: number(input.child_count, input.childCount),
    structure_path: input.structure_path ?? input.structurePath ?? null,
    section_ast_node_id: input.section_ast_node_id ?? input.sectionAstNodeId ?? null,
    section_path: input.section_path ?? input.sectionPath ?? null,
    heading_level: nullableNumber(input.heading_level, input.headingLevel),
    heading_text: input.heading_text ?? input.headingText ?? null,
    semantic_role: input.semantic_role ?? input.semanticRole ?? null,
    semantic_fingerprint: input.semantic_fingerprint ?? input.semanticFingerprint ?? null,
    text_hash: input.text_hash ?? input.textHash ?? null,
    text_excerpt: input.text_excerpt ?? input.textExcerpt ?? null,
    byte_start: nullableNumber(input.byte_start, input.byteStart),
    byte_end: nullableNumber(input.byte_end, input.byteEnd),
    char_start: nullableNumber(input.char_start, input.charStart),
    char_end: nullableNumber(input.char_end, input.charEnd),
    source_span_ids: array(input.source_span_ids, input.sourceSpanIds),
    text_chunk_ids: array(input.text_chunk_ids, input.textChunkIds),
    evidence_span_ids: array(input.evidence_span_ids, input.evidenceSpanIds),
    search_index_record_ids: array(input.search_index_record_ids, input.searchIndexRecordIds),
    entity_mention_ids: array(input.entity_mention_ids, input.entityMentionIds),
    fact_ids: array(input.fact_ids, input.factIds),
    timeline_suggestion_ids: array(input.timeline_suggestion_ids, input.timelineSuggestionIds),
    citation_texts: array(input.citation_texts, input.citationTexts),
    date_texts: array(input.date_texts, input.dateTexts),
    money_texts: array(input.money_texts, input.moneyTexts),
    contains_entity_mention: Boolean(input.contains_entity_mention ?? input.containsEntityMention ?? false),
    contains_citation: Boolean(input.contains_citation ?? input.containsCitation ?? false),
    contains_date: Boolean(input.contains_date ?? input.containsDate ?? false),
    contains_money: Boolean(input.contains_money ?? input.containsMoney ?? false),
    review_status: string(input.review_status, input.reviewStatus, "unreviewed"),
  }
}

function normalizeMarkdownSemanticUnit(input: any): MarkdownSemanticUnit {
  return {
    ...input,
    id: string(input.id, input.semantic_unit_id),
    semantic_unit_id: string(input.semantic_unit_id, input.semanticUnitId, input.id),
    matter_id: string(input.matter_id, input.matterId),
    document_id: string(input.document_id, input.documentId),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    markdown_ast_document_id: string(input.markdown_ast_document_id, input.markdownAstDocumentId),
    unit_kind: string(input.unit_kind, input.unitKind, "markdown"),
    semantic_role: string(input.semantic_role, input.semanticRole, "markdown_node"),
    canonical_label: string(input.canonical_label, input.canonicalLabel),
    normalized_key: string(input.normalized_key, input.normalizedKey),
    structure_path: input.structure_path ?? input.structurePath ?? null,
    section_path: input.section_path ?? input.sectionPath ?? null,
    section_ast_node_id: input.section_ast_node_id ?? input.sectionAstNodeId ?? null,
    text_hash: input.text_hash ?? input.textHash ?? null,
    semantic_fingerprint: string(input.semantic_fingerprint, input.semanticFingerprint),
    markdown_ast_node_ids: array(input.markdown_ast_node_ids, input.markdownAstNodeIds),
    entity_mention_ids: array(input.entity_mention_ids, input.entityMentionIds),
    citation_texts: array(input.citation_texts, input.citationTexts),
    date_texts: array(input.date_texts, input.dateTexts),
    money_texts: array(input.money_texts, input.moneyTexts),
    occurrence_count: number(input.occurrence_count, input.occurrenceCount),
    evidence_span_count: number(input.evidence_span_count, input.evidenceSpanCount),
    text_chunk_count: number(input.text_chunk_count, input.textChunkCount),
    source_span_count: number(input.source_span_count, input.sourceSpanCount),
    review_status: string(input.review_status, input.reviewStatus, "unreviewed"),
    created_at: string(input.created_at, input.createdAt),
    updated_at: string(input.updated_at, input.updatedAt),
  }
}

function normalizeCaseEntity(input: any): CaseEntity {
  return {
    ...input,
    id: string(input.id, input.entity_id),
    entity_id: string(input.entity_id, input.entityId, input.id),
    matter_id: string(input.matter_id, input.matterId),
    entity_type: string(input.entity_type, input.entityType, "unknown"),
    canonical_name: string(input.canonical_name, input.canonicalName, input.name),
    normalized_key: string(input.normalized_key, input.normalizedKey),
    confidence: number(input.confidence),
    review_status: string(input.review_status, input.reviewStatus, "unreviewed"),
    mention_ids: array(input.mention_ids, input.mentionIds),
    party_match_ids: array(input.party_match_ids, input.partyMatchIds),
    created_at: string(input.created_at, input.createdAt),
    updated_at: string(input.updated_at, input.updatedAt),
  }
}

function normalizeSearchIndexRecord(input: any): SearchIndexRecord {
  return {
    ...input,
    id: string(input.id, input.search_index_record_id),
    search_index_record_id: string(input.search_index_record_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    text_chunk_id: input.text_chunk_id ?? input.textChunkId ?? null,
    index_run_id: input.index_run_id ?? input.indexRunId ?? null,
    index_name: string(input.index_name, input.indexName, "casebuilder_document_text"),
    index_type: string(input.index_type, input.indexType, "fulltext"),
    index_version: string(input.index_version, input.indexVersion),
    status: string(input.status, "indexed"),
    stale: Boolean(input.stale),
    created_at: string(input.created_at, input.createdAt),
    indexed_at: input.indexed_at ?? input.indexedAt ?? null,
  }
}

function normalizeCaseBuilderEmbeddingRun(input: any): CaseBuilderEmbeddingRun {
  return {
    ...input,
    id: string(input.id, input.embedding_run_id, input.embeddingRunId),
    embedding_run_id: string(input.embedding_run_id, input.embeddingRunId, input.id),
    matter_id: string(input.matter_id, input.matterId),
    document_id: input.document_id ?? input.documentId ?? null,
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    index_run_id: input.index_run_id ?? input.indexRunId ?? null,
    model: string(input.model, "voyage-4-large"),
    profile: string(input.profile, "casebuilder_markdown_v1"),
    dimension: number(input.dimension, 1024),
    vector_index_name: string(input.vector_index_name, input.vectorIndexName, "casebuilder_markdown_embedding_1024"),
    status: string(input.status, "queued"),
    stage: string(input.stage, "queued"),
    target_count: number(input.target_count, input.targetCount),
    embedded_count: number(input.embedded_count, input.embeddedCount),
    skipped_count: number(input.skipped_count, input.skippedCount),
    stale_count: number(input.stale_count, input.staleCount),
    produced_embedding_record_ids: array(input.produced_embedding_record_ids, input.producedEmbeddingRecordIds),
    warnings: array(input.warnings),
    error_code: input.error_code ?? input.errorCode ?? null,
    error_message: input.error_message ?? input.errorMessage ?? null,
    retryable: Boolean(input.retryable),
    started_at: string(input.started_at, input.startedAt),
    completed_at: input.completed_at ?? input.completedAt ?? null,
  }
}

function normalizeCaseBuilderEmbeddingRecord(input: any): CaseBuilderEmbeddingRecord {
  return {
    ...input,
    id: string(input.id, input.embedding_record_id, input.embeddingRecordId),
    embedding_record_id: string(input.embedding_record_id, input.embeddingRecordId, input.id),
    matter_id: string(input.matter_id, input.matterId),
    document_id: string(input.document_id, input.documentId),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    index_run_id: input.index_run_id ?? input.indexRunId ?? null,
    embedding_run_id: input.embedding_run_id ?? input.embeddingRunId ?? null,
    target_kind: string(input.target_kind, input.targetKind),
    target_id: string(input.target_id, input.targetId),
    target_label: string(input.target_label, input.targetLabel),
    model: string(input.model, "voyage-4-large"),
    profile: string(input.profile, "casebuilder_markdown_v1"),
    dimension: number(input.dimension, 1024),
    vector_index_name: string(input.vector_index_name, input.vectorIndexName, "casebuilder_markdown_embedding_1024"),
    input_hash: string(input.input_hash, input.inputHash),
    source_text_hash: string(input.source_text_hash, input.sourceTextHash),
    chunker_version: input.chunker_version ?? input.chunkerVersion ?? null,
    graph_schema_version: input.graph_schema_version ?? input.graphSchemaVersion ?? null,
    embedding_strategy: string(input.embedding_strategy, input.embeddingStrategy, "direct"),
    embedding_input_type: string(input.embedding_input_type, input.embeddingInputType, "document"),
    embedding_output_dtype: string(input.embedding_output_dtype, input.embeddingOutputDtype, "float"),
    status: string(input.status, "queued"),
    stale: Boolean(input.stale),
    review_status: string(input.review_status, input.reviewStatus, "system"),
    text_excerpt: input.text_excerpt ?? input.textExcerpt ?? null,
    source_span_ids: array(input.source_span_ids, input.sourceSpanIds),
    text_chunk_ids: array(input.text_chunk_ids, input.textChunkIds),
    markdown_ast_node_ids: array(input.markdown_ast_node_ids, input.markdownAstNodeIds),
    markdown_semantic_unit_ids: array(input.markdown_semantic_unit_ids, input.markdownSemanticUnitIds),
    centroid_source_record_ids: array(input.centroid_source_record_ids, input.centroidSourceRecordIds),
    created_at: string(input.created_at, input.createdAt),
    embedded_at: input.embedded_at ?? input.embeddedAt ?? null,
  }
}

function normalizeCaseBuilderEmbeddingCoverage(input: any): CaseBuilderEmbeddingCoverage {
  return {
    enabled: Boolean(input?.enabled),
    model: input?.model ?? null,
    profile: input?.profile ?? null,
    dimension: input?.dimension ?? null,
    vector_index_name: input?.vector_index_name ?? input?.vectorIndexName ?? null,
    target_count: number(input?.target_count, input?.targetCount),
    embedded_count: number(input?.embedded_count, input?.embeddedCount),
    current_count: number(input?.current_count, input?.currentCount),
    stale_count: number(input?.stale_count, input?.staleCount),
    skipped_count: number(input?.skipped_count, input?.skippedCount),
    failed_count: number(input?.failed_count, input?.failedCount),
    full_file_embedded: Boolean(input?.full_file_embedded ?? input?.fullFileEmbedded),
    chunk_embedded: number(input?.chunk_embedded, input?.chunkEmbedded),
    semantic_unit_embedded: number(input?.semantic_unit_embedded, input?.semanticUnitEmbedded),
  }
}

function normalizeRunCaseBuilderEmbeddingsResponse(input: any): RunCaseBuilderEmbeddingsResponse {
  return {
    matter_id: string(input.matter_id, input.matterId),
    requested: number(input.requested),
    processed: number(input.processed),
    skipped: number(input.skipped),
    failed: number(input.failed),
    runs: array(input.runs).map(normalizeCaseBuilderEmbeddingRun),
    warnings: array(input.warnings),
  }
}

function normalizeCaseBuilderEmbeddingSearchResponse(input: any): CaseBuilderEmbeddingSearchResponse {
  return {
    enabled: Boolean(input.enabled),
    mode: string(input.mode, "markdown_embeddings"),
    query: string(input.query),
    total: number(input.total),
    results: array(input.results).map((result: any) => {
      const record = normalizeCaseBuilderEmbeddingRecord(result.embedding_record ?? result.embeddingRecord ?? {})
      return {
        score: number(result.score),
        embedding_record: record,
        target_kind: string(result.target_kind, result.targetKind, record.target_kind),
        target_id: string(result.target_id, result.targetId, record.target_id),
        document_id: string(result.document_id, result.documentId, record.document_id),
        document_version_id: result.document_version_id ?? result.documentVersionId ?? record.document_version_id ?? null,
        text_excerpt: result.text_excerpt ?? result.textExcerpt ?? record.text_excerpt ?? null,
        source_span_ids: array(result.source_span_ids, result.sourceSpanIds, record.source_span_ids),
        text_chunk_ids: array(result.text_chunk_ids, result.textChunkIds, record.text_chunk_ids),
        markdown_ast_node_ids: array(result.markdown_ast_node_ids, result.markdownAstNodeIds, record.markdown_ast_node_ids),
        markdown_semantic_unit_ids: array(
          result.markdown_semantic_unit_ids,
          result.markdownSemanticUnitIds,
          record.markdown_semantic_unit_ids,
        ),
        stale: Boolean(result.stale ?? record.stale),
      }
    }),
    model: input.model ?? null,
    profile: input.profile ?? null,
    dimension: input.dimension ?? null,
    warnings: array(input.warnings),
  }
}

function normalizeExtractionArtifactManifest(input: any): ExtractionArtifactManifest {
  return {
    ...input,
    id: string(input.id, input.manifest_id),
    manifest_id: string(input.manifest_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    ingestion_run_id: input.ingestion_run_id ?? input.ingestionRunId ?? null,
    index_run_id: input.index_run_id ?? input.indexRunId ?? null,
    normalized_text_version_id: input.normalized_text_version_id ?? input.normalizedTextVersionId ?? null,
    pages_version_id: input.pages_version_id ?? input.pagesVersionId ?? null,
    manifest_version_id: input.manifest_version_id ?? input.manifestVersionId ?? null,
    text_sha256: string(input.text_sha256, input.textSha256),
    pages_sha256: input.pages_sha256 ?? input.pagesSha256 ?? null,
    manifest_sha256: input.manifest_sha256 ?? input.manifestSha256 ?? null,
    page_ids: array(input.page_ids, input.pageIds),
    text_chunk_ids: array(input.text_chunk_ids, input.textChunkIds),
    evidence_span_ids: array(input.evidence_span_ids, input.evidenceSpanIds),
    entity_mention_ids: array(input.entity_mention_ids, input.entityMentionIds),
    search_index_record_ids: array(input.search_index_record_ids, input.searchIndexRecordIds),
    produced_object_keys: array(input.produced_object_keys, input.producedObjectKeys),
    created_at: string(input.created_at, input.createdAt),
  }
}

function normalizeMatterIndexSummary(input: any): MatterIndexSummary {
  return {
    matter_id: string(input.matter_id, input.matterId),
    total_documents: number(input.total_documents, input.totalDocuments),
    active_documents: number(input.active_documents, input.activeDocuments, input.total_documents, input.totalDocuments),
    archived_documents: number(input.archived_documents, input.archivedDocuments),
    indexed_documents: number(input.indexed_documents, input.indexedDocuments),
    pending_documents: number(input.pending_documents, input.pendingDocuments),
    extractable_pending_documents: number(input.extractable_pending_documents, input.extractablePendingDocuments),
    failed_documents: number(input.failed_documents, input.failedDocuments),
    ocr_required_documents: number(input.ocr_required_documents, input.ocrRequiredDocuments),
    transcription_deferred_documents: number(input.transcription_deferred_documents, input.transcriptionDeferredDocuments),
    unsupported_documents: number(input.unsupported_documents, input.unsupportedDocuments),
    processing_status_counts: array(input.processing_status_counts, input.processingStatusCounts).map((item: any) => ({
      status: string(item.status),
      count: number(item.count),
    })),
    storage_status_counts: array(input.storage_status_counts, input.storageStatusCounts).map((item: any) => ({
      status: string(item.status),
      count: number(item.count),
    })),
    duplicate_groups: array(input.duplicate_groups, input.duplicateGroups).map((group: any) => ({
      file_hash: string(group.file_hash, group.fileHash),
      count: number(group.count),
      document_ids: array(group.document_ids, group.documentIds),
      filenames: array(group.filenames),
    })),
    folders: array(input.folders).map((folder: any) => ({
      folder: string(folder.folder, "Uploads"),
      count: number(folder.count),
      indexed: number(folder.indexed),
      pending: number(folder.pending),
      failed: number(folder.failed),
    })),
    upload_batches: array(input.upload_batches, input.uploadBatches).map((batch: any) => ({
      upload_batch_id: string(batch.upload_batch_id, batch.uploadBatchId),
      count: number(batch.count),
      indexed: number(batch.indexed),
      pending: number(batch.pending),
      failed: number(batch.failed),
    })),
    recent_ingestion_runs: array(input.recent_ingestion_runs, input.recentIngestionRuns).map(normalizeIngestionRun),
    extractable_pending_document_ids: array(input.extractable_pending_document_ids, input.extractablePendingDocumentIds),
  }
}

function normalizeMatterIndexRunResponse(input: any): MatterIndexRunResponse {
  return {
    matter_id: string(input.matter_id, input.matterId),
    requested: number(input.requested),
    processed: number(input.processed),
    skipped: number(input.skipped),
    failed: number(input.failed),
    produced_timeline_suggestions: number(input.produced_timeline_suggestions, input.producedTimelineSuggestions),
    results: array(input.results).map((result: any): MatterIndexRunDocumentResult => ({
      document_id: string(result.document_id, result.documentId),
      status: string(result.status, "skipped"),
      extraction_status: result.extraction_status ?? result.extractionStatus ?? null,
      message: string(result.message),
      produced_chunks: number(result.produced_chunks, result.producedChunks),
      produced_facts: number(result.produced_facts, result.producedFacts),
      produced_timeline_suggestions: number(result.produced_timeline_suggestions, result.producedTimelineSuggestions),
      produced_markdown_ast_nodes: number(result.produced_markdown_ast_nodes, result.producedMarkdownAstNodes),
      produced_markdown_semantic_units: number(
        result.produced_markdown_semantic_units,
        result.producedMarkdownSemanticUnits,
      ),
      produced_embedding_records: number(result.produced_embedding_records, result.producedEmbeddingRecords),
      produced_entities: number(result.produced_entities, result.producedEntities),
    })),
    summary: normalizeMatterIndexSummary(input.summary ?? {}),
  }
}

function normalizeMatterIndexJob(input: any): MatterIndexJob {
  return {
    ...input,
    id: string(input.id, input.index_job_id, input.indexJobId),
    index_job_id: string(input.index_job_id, input.indexJobId, input.id),
    matter_id: string(input.matter_id, input.matterId),
    upload_batch_id: input.upload_batch_id ?? input.uploadBatchId ?? null,
    document_ids: array(input.document_ids, input.documentIds),
    limit: number(input.limit, 250),
    status: string(input.status, "queued"),
    stage: string(input.stage, "queued"),
    requested: number(input.requested),
    processed: number(input.processed),
    skipped: number(input.skipped),
    failed: number(input.failed),
    produced_timeline_suggestions: number(input.produced_timeline_suggestions, input.producedTimelineSuggestions),
    results: array(input.results).map((result: any): MatterIndexRunDocumentResult => ({
      document_id: string(result.document_id, result.documentId),
      status: string(result.status, "skipped"),
      extraction_status: result.extraction_status ?? result.extractionStatus ?? null,
      message: string(result.message),
      produced_chunks: number(result.produced_chunks, result.producedChunks),
      produced_facts: number(result.produced_facts, result.producedFacts),
      produced_timeline_suggestions: number(result.produced_timeline_suggestions, result.producedTimelineSuggestions),
      produced_markdown_ast_nodes: number(result.produced_markdown_ast_nodes, result.producedMarkdownAstNodes),
      produced_markdown_semantic_units: number(
        result.produced_markdown_semantic_units,
        result.producedMarkdownSemanticUnits,
      ),
      produced_embedding_records: number(result.produced_embedding_records, result.producedEmbeddingRecords),
      produced_entities: number(result.produced_entities, result.producedEntities),
    })),
    summary: input.summary ? normalizeMatterIndexSummary(input.summary) : null,
    warnings: array(input.warnings),
    error_code: input.error_code ?? input.errorCode ?? null,
    error_message: input.error_message ?? input.errorMessage ?? null,
    retryable: Boolean(input.retryable),
    created_at: string(input.created_at, input.createdAt),
    started_at: input.started_at ?? input.startedAt ?? null,
    completed_at: input.completed_at ?? input.completedAt ?? null,
  }
}

function normalizeTranscriptionJobResponse(input: any): TranscriptionJobResponse {
  return {
    job: normalizeTranscriptionJob(input.job),
    segments: array(input.segments).map(normalizeTranscriptSegment),
    speakers: array(input.speakers).map(normalizeTranscriptSpeaker),
    review_changes: array(input.review_changes, input.reviewChanges).map(normalizeTranscriptReviewChange),
    raw_artifact_version: input.raw_artifact_version ? normalizeDocumentVersion(input.raw_artifact_version) : null,
    normalized_artifact_version: input.normalized_artifact_version ? normalizeDocumentVersion(input.normalized_artifact_version) : null,
    redacted_artifact_version: input.redacted_artifact_version ? normalizeDocumentVersion(input.redacted_artifact_version) : null,
    redacted_audio_version: input.redacted_audio_version || input.redactedAudioVersion ? normalizeDocumentVersion(input.redacted_audio_version ?? input.redactedAudioVersion) : null,
    reviewed_document_version: input.reviewed_document_version ? normalizeDocumentVersion(input.reviewed_document_version) : null,
    caption_vtt_version: input.caption_vtt_version ? normalizeDocumentVersion(input.caption_vtt_version) : null,
    caption_srt_version: input.caption_srt_version ? normalizeDocumentVersion(input.caption_srt_version) : null,
    caption_vtt: input.caption_vtt ?? input.captionVtt ?? null,
    caption_srt: input.caption_srt ?? input.captionSrt ?? null,
    warnings: array(input.warnings),
  }
}

function normalizeAssemblyAiTranscriptListResponse(input: any): AssemblyAiTranscriptListResponse {
  const pageDetails = input.page_details ?? input.pageDetails ?? {}
  return {
    page_details: {
      limit: number(pageDetails.limit),
      result_count: number(pageDetails.result_count, pageDetails.resultCount),
      current_url: string(pageDetails.current_url, pageDetails.currentUrl),
      prev_url: pageDetails.prev_url ?? pageDetails.prevUrl ?? null,
      next_url: pageDetails.next_url ?? pageDetails.nextUrl ?? null,
    },
    transcripts: array(input.transcripts).map((transcript: any) => ({
      id: string(transcript.id),
      resource_url: string(transcript.resource_url, transcript.resourceUrl),
      status: string(transcript.status),
      created: string(transcript.created),
      completed: transcript.completed ?? null,
      audio_url: string(transcript.audio_url, transcript.audioUrl),
      error: transcript.error ?? null,
    })),
  }
}

function normalizeAssemblyAiTranscriptDeleteResponse(input: any): AssemblyAiTranscriptDeleteResponse {
  const providerResponse = input.provider_response ?? input.providerResponse ?? {}
  const deleted =
    input.deleted != null
      ? Boolean(input.deleted)
      : (input.status ?? providerResponse.status) === "deleted"

  return {
    id: string(input.id, providerResponse.id),
    status: string(input.status, providerResponse.status, "deleted"),
    deleted,
    provider_response: providerResponse,
  }
}

function normalizeAssemblyAiSpeakerOptions(input: any): AssemblyAiSpeakerOptions | null {
  if (!input || typeof input !== "object") return null
  const min = nullableNumber(input.min_speakers_expected, input.minSpeakersExpected)
  const max = nullableNumber(input.max_speakers_expected, input.maxSpeakersExpected)
  if (min == null && max == null) return null
  return {
    min_speakers_expected: min,
    max_speakers_expected: max,
  }
}

function normalizeTranscriptionJob(input: any): TranscriptionJob {
  return {
    ...input,
    id: string(input.id, input.transcription_job_id),
    transcription_job_id: string(input.transcription_job_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    provider: string(input.provider, "assemblyai"),
    provider_mode: string(input.provider_mode, input.providerMode, "disabled"),
    provider_transcript_id: input.provider_transcript_id ?? input.providerTranscriptId ?? null,
    provider_status: input.provider_status ?? input.providerStatus ?? null,
    status: string(input.status, "queued"),
    review_status: string(input.review_status, input.reviewStatus, "not_started"),
    raw_artifact_version_id: input.raw_artifact_version_id ?? input.rawArtifactVersionId ?? null,
    normalized_artifact_version_id: input.normalized_artifact_version_id ?? input.normalizedArtifactVersionId ?? null,
    redacted_artifact_version_id: input.redacted_artifact_version_id ?? input.redactedArtifactVersionId ?? null,
    redacted_audio_version_id: input.redacted_audio_version_id ?? input.redactedAudioVersionId ?? null,
    reviewed_document_version_id: input.reviewed_document_version_id ?? input.reviewedDocumentVersionId ?? null,
    caption_vtt_version_id: input.caption_vtt_version_id ?? input.captionVttVersionId ?? null,
    caption_srt_version_id: input.caption_srt_version_id ?? input.captionSrtVersionId ?? null,
    language_code: input.language_code ?? input.languageCode ?? null,
    duration_ms: nullableNumber(input.duration_ms, input.durationMs),
    speaker_count: number(input.speaker_count, input.speakerCount),
    segment_count: number(input.segment_count, input.segmentCount),
    word_count: number(input.word_count, input.wordCount),
    speakers_expected: nullableNumber(input.speakers_expected, input.speakersExpected),
    speaker_options: normalizeAssemblyAiSpeakerOptions(input.speaker_options ?? input.speakerOptions),
    word_search_terms: array(input.word_search_terms, input.wordSearchTerms).map(String),
    prompt_preset: input.prompt_preset ?? input.promptPreset ?? null,
    prompt: input.prompt ?? null,
    keyterms_prompt: array(input.keyterms_prompt, input.keytermsPrompt).map(String),
    remove_audio_tags: input.remove_audio_tags ?? input.removeAudioTags ?? null,
    redact_pii: Boolean(input.redact_pii ?? input.redactPii ?? true),
    speech_models: array(input.speech_models, input.speechModels).map(String),
    retryable: Boolean(input.retryable),
    error_code: input.error_code ?? input.errorCode ?? null,
    error_message: input.error_message ?? input.errorMessage ?? null,
    created_at: string(input.created_at, input.createdAt),
    updated_at: string(input.updated_at, input.updatedAt),
    completed_at: input.completed_at ?? input.completedAt ?? null,
    reviewed_at: input.reviewed_at ?? input.reviewedAt ?? null,
  }
}

function normalizeTranscriptSegment(input: any): TranscriptSegment {
  return {
    ...input,
    id: string(input.id, input.segment_id),
    segment_id: string(input.segment_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    transcription_job_id: string(input.transcription_job_id, input.transcriptionJobId),
    source_span_id: input.source_span_id ?? input.sourceSpanId ?? null,
    ordinal: number(input.ordinal),
    paragraph_ordinal: nullableNumber(input.paragraph_ordinal, input.paragraphOrdinal),
    speaker_label: input.speaker_label ?? input.speakerLabel ?? null,
    speaker_name: input.speaker_name ?? input.speakerName ?? null,
    channel: input.channel ?? null,
    text: string(input.text),
    redacted_text: input.redacted_text ?? input.redactedText ?? null,
    time_start_ms: number(input.time_start_ms, input.timeStartMs),
    time_end_ms: number(input.time_end_ms, input.timeEndMs),
    confidence: number(input.confidence),
    review_status: string(input.review_status, input.reviewStatus, "unreviewed"),
    edited: Boolean(input.edited),
    created_at: string(input.created_at, input.createdAt),
    updated_at: string(input.updated_at, input.updatedAt),
  }
}

function normalizeTranscriptSpeaker(input: any): TranscriptSpeaker {
  return {
    ...input,
    id: string(input.id, input.speaker_id),
    speaker_id: string(input.speaker_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    transcription_job_id: string(input.transcription_job_id, input.transcriptionJobId),
    speaker_label: string(input.speaker_label, input.speakerLabel),
    display_name: input.display_name ?? input.displayName ?? null,
    role: input.role ?? null,
    confidence: nullableNumber(input.confidence),
    segment_count: number(input.segment_count, input.segmentCount),
    created_at: string(input.created_at, input.createdAt),
    updated_at: string(input.updated_at, input.updatedAt),
  }
}

function normalizeTranscriptReviewChange(input: any): TranscriptReviewChange {
  return {
    ...input,
    id: string(input.id, input.review_change_id),
    review_change_id: string(input.review_change_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    transcription_job_id: string(input.transcription_job_id, input.transcriptionJobId),
    target_type: string(input.target_type, input.targetType),
    target_id: string(input.target_id, input.targetId),
    field: string(input.field),
    before: input.before ?? null,
    after: input.after ?? null,
    created_by: string(input.created_by, input.createdBy, "user"),
    created_at: string(input.created_at, input.createdAt),
  }
}

function normalizeSourceSpan(input: any): SourceSpan {
  return {
    ...input,
    id: string(input.id, input.source_span_id),
    source_span_id: string(input.source_span_id, input.id),
    matter_id: string(input.matter_id),
    document_id: string(input.document_id),
    document_version_id: input.document_version_id ?? input.documentVersionId ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    ingestion_run_id: input.ingestion_run_id ?? input.ingestionRunId ?? null,
    page: nullableNumber(input.page),
    chunk_id: input.chunk_id ?? input.chunkId ?? null,
    byte_start: nullableNumber(input.byte_start, input.byteStart),
    byte_end: nullableNumber(input.byte_end, input.byteEnd),
    char_start: nullableNumber(input.char_start, input.charStart),
    char_end: nullableNumber(input.char_end, input.charEnd),
    time_start_ms: nullableNumber(input.time_start_ms, input.timeStartMs),
    time_end_ms: nullableNumber(input.time_end_ms, input.timeEndMs),
    speaker_label: input.speaker_label ?? input.speakerLabel ?? null,
    quote: input.quote ?? null,
    extraction_method: string(input.extraction_method, input.extractionMethod, "unknown"),
    confidence: number(input.confidence),
    review_status: string(input.review_status, input.reviewStatus, "unreviewed"),
    unavailable_reason: input.unavailable_reason ?? input.unavailableReason ?? null,
  }
}

function normalizeTimelineEvent(input: any): TimelineEvent {
  return {
    ...input,
    id: string(input.id, input.event_id),
    event_id: string(input.event_id, input.id),
    title: string(input.title, input.description, "Untitled event"),
    kind: string(input.kind, "other") as TimelineEvent["kind"],
    category: string(input.category, input.kind, "other"),
    sourceDocumentId: input.sourceDocumentId ?? input.source_document_id ?? null,
    source_document_id: input.source_document_id ?? input.sourceDocumentId ?? null,
    source_span_ids: array(input.source_span_ids, input.sourceSpanIds),
    text_chunk_ids: array(input.text_chunk_ids, input.textChunkIds),
    markdown_ast_node_ids: array(input.markdown_ast_node_ids, input.markdownAstNodeIds),
    linked_fact_ids: array(input.linked_fact_ids, input.linkedFactIds),
    linked_claim_ids: array(input.linked_claim_ids, input.linkedClaimIds),
    suggestion_id: input.suggestion_id ?? input.suggestionId ?? null,
    agent_run_id: input.agent_run_id ?? input.agentRunId ?? null,
    date_confidence: number(input.date_confidence, input.dateConfidence, 1),
    disputed: Boolean(input.disputed),
  }
}

function normalizeTimelineAgentRun(input: any): TimelineAgentRun {
  return {
    ...input,
    agent_run_id: string(input.agent_run_id, input.agentRunId, input.id),
    id: string(input.id, input.agent_run_id, input.agentRunId),
    matter_id: string(input.matter_id, input.matterId),
    subject_type: string(input.subject_type, input.subjectType, "matter"),
    subject_id: input.subject_id ?? input.subjectId ?? null,
    agent_type: string(input.agent_type, input.agentType, "timeline_builder"),
    scope_type: string(input.scope_type, input.scopeType, input.subject_type, input.subjectType, "matter"),
    scope_ids: array(input.scope_ids, input.scopeIds),
    input_hash: input.input_hash ?? input.inputHash ?? null,
    pipeline_version: string(input.pipeline_version, input.pipelineVersion),
    extractor_version: string(input.extractor_version, input.extractorVersion),
    prompt_template_id: input.prompt_template_id ?? input.promptTemplateId ?? null,
    provider: string(input.provider, "disabled"),
    model: input.model ?? null,
    mode: string(input.mode, "template"),
    provider_mode: string(input.provider_mode, input.providerMode, "template"),
    status: string(input.status, "recorded"),
    message: string(input.message),
    produced_suggestion_ids: array(input.produced_suggestion_ids, input.producedSuggestionIds),
    warnings: array(input.warnings),
    started_at: input.started_at ?? input.startedAt ?? null,
    completed_at: input.completed_at ?? input.completedAt ?? null,
    duration_ms: input.duration_ms ?? input.durationMs ?? null,
    error_code: input.error_code ?? input.errorCode ?? null,
    error_message: input.error_message ?? input.errorMessage ?? null,
    deterministic_candidate_count: number(input.deterministic_candidate_count, input.deterministicCandidateCount),
    provider_enriched_count: number(input.provider_enriched_count, input.providerEnrichedCount),
    provider_rejected_count: number(input.provider_rejected_count, input.providerRejectedCount),
    duplicate_candidate_count: number(input.duplicate_candidate_count, input.duplicateCandidateCount),
    stored_suggestion_count: number(input.stored_suggestion_count, input.storedSuggestionCount),
    preserved_review_count: number(input.preserved_review_count, input.preservedReviewCount),
    created_at: string(input.created_at, input.createdAt),
  }
}

function normalizeTimelineSuggestion(input: any): TimelineSuggestion {
  return {
    ...input,
    suggestion_id: string(input.suggestion_id, input.suggestionId, input.id),
    id: string(input.id, input.suggestion_id, input.suggestionId),
    matter_id: string(input.matter_id, input.matterId),
    date: string(input.date),
    date_text: string(input.date_text, input.dateText, input.date),
    date_confidence: number(input.date_confidence, input.dateConfidence, 0),
    title: string(input.title, input.description, "Timeline suggestion"),
    description: input.description ?? null,
    kind: string(input.kind, "other"),
    source_type: string(input.source_type, input.sourceType, "matter_graph"),
    source_document_id: input.source_document_id ?? input.sourceDocumentId ?? null,
    source_span_ids: array(input.source_span_ids, input.sourceSpanIds),
    text_chunk_ids: array(input.text_chunk_ids, input.textChunkIds),
    markdown_ast_node_ids: array(input.markdown_ast_node_ids, input.markdownAstNodeIds),
    linked_fact_ids: array(input.linked_fact_ids, input.linkedFactIds),
    linked_claim_ids: array(input.linked_claim_ids, input.linkedClaimIds),
    work_product_id: input.work_product_id ?? input.workProductId ?? null,
    block_id: input.block_id ?? input.blockId ?? null,
    agent_run_id: input.agent_run_id ?? input.agentRunId ?? null,
    index_run_id: input.index_run_id ?? input.indexRunId ?? null,
    dedupe_key: input.dedupe_key ?? input.dedupeKey ?? null,
    cluster_id: input.cluster_id ?? input.clusterId ?? null,
    duplicate_of_suggestion_id: input.duplicate_of_suggestion_id ?? input.duplicateOfSuggestionId ?? null,
    agent_explanation: input.agent_explanation ?? input.agentExplanation ?? null,
    agent_confidence: input.agent_confidence ?? input.agentConfidence ?? null,
    status: string(input.status, "suggested"),
    warnings: array(input.warnings),
    approved_event_id: input.approved_event_id ?? input.approvedEventId ?? null,
    created_at: string(input.created_at, input.createdAt),
    updated_at: string(input.updated_at, input.updatedAt),
  }
}

function normalizeTimelineSuggestResponse(input: any): TimelineSuggestResponse {
  return {
    enabled: Boolean(input.enabled),
    mode: string(input.mode, "template"),
    message: string(input.message),
    suggestions: array(input.suggestions).map(normalizeTimelineSuggestion),
    agent_run: input.agent_run || input.agentRun ? normalizeTimelineAgentRun(input.agent_run ?? input.agentRun) : null,
    warnings: array(input.warnings),
  }
}

function normalizeTimelineSuggestionApprovalResponse(input: any): TimelineSuggestionApprovalResponse {
  return {
    suggestion: normalizeTimelineSuggestion(input.suggestion),
    event: normalizeTimelineEvent(input.event),
  }
}

function normalizeClaim(input: any): Claim {
  const claimId = string(input.id, input.claim_id, input.defense_id)
  const factIds = array(input.supportingFactIds, input.fact_ids)
  return {
    ...input,
    id: claimId,
    claim_id: input.claim_id ?? claimId,
    kind: string(input.kind, "claim") as Claim["kind"],
    title: string(input.title, input.name, "Untitled claim"),
    name: string(input.name, input.title, "Untitled claim"),
    cause: string(input.cause, input.claim_type, "custom"),
    claim_type: string(input.claim_type, input.cause, "custom"),
    theory: string(input.theory, input.legal_theory),
    legal_theory: string(input.legal_theory, input.theory),
    against: string(input.against, "Opposing party"),
    risk: string(input.risk, input.risk_level, "medium") as Claim["risk"],
    risk_level: string(input.risk_level, input.risk, "medium") as Claim["risk_level"],
    status: string(input.status, "candidate") as Claim["status"],
    elements: array(input.elements).map(normalizeElement),
    supportingFactIds: factIds,
    fact_ids: array(input.fact_ids, factIds),
    counterArguments: array(input.counterArguments, input.counter_arguments),
  }
}

function normalizeElement(input: any): ClaimElement {
  const factIds = array(input.supportingFactIds, input.fact_ids)
  return {
    ...input,
    id: string(input.id, input.element_id),
    element_id: string(input.element_id, input.id),
    title: string(input.title, input.text, "Element"),
    description: string(input.description, input.text),
    status: string(input.status, input.satisfied ? "supported" : "missing") as ClaimElement["status"],
    legalAuthority: input.legalAuthority ?? input.authority,
    authority: input.authority ?? input.legalAuthority,
    authorities: array(input.authorities),
    supportingFactIds: factIds,
    fact_ids: array(input.fact_ids, factIds),
    evidence_ids: array(input.evidence_ids),
    missing_facts: array(input.missing_facts),
  }
}

function normalizeDeadline(input: any): Deadline {
  return {
    ...input,
    id: string(input.id, input.deadline_id),
    deadline_id: string(input.deadline_id, input.id),
    dueDate: string(input.dueDate, input.due_date),
    due_date: string(input.due_date, input.dueDate),
    daysRemaining: number(input.daysRemaining, input.days_remaining),
    days_remaining: number(input.days_remaining, input.daysRemaining),
    sourceCitation: input.sourceCitation ?? input.source_citation,
    source_citation: input.source_citation ?? input.sourceCitation,
    sourceCanonicalId: input.sourceCanonicalId ?? input.source_canonical_id,
    source_canonical_id: input.source_canonical_id ?? input.sourceCanonicalId,
    tasks: array(input.tasks),
  }
}

function normalizeTask(input: any): CaseTask {
  return {
    ...input,
    task_id: string(input.task_id, input.id),
    matter_id: string(input.matter_id),
    title: string(input.title),
    status: string(input.status, "todo") as CaseTask["status"],
    priority: string(input.priority, "med") as CaseTask["priority"],
    due_date: input.due_date ?? input.dueDate ?? null,
    assigned_to: input.assigned_to ?? input.assignedTo ?? null,
    related_claim_ids: array(input.related_claim_ids, input.relatedClaimIds),
    related_document_ids: array(input.related_document_ids, input.relatedDocumentIds),
    related_deadline_id: input.related_deadline_id ?? input.relatedDeadlineId ?? null,
    source: string(input.source, "user"),
    description: input.description ?? null,
  }
}

function normalizeCaseGraphResponse(input: unknown): CaseGraphResponse {
  const raw = input as any
  return {
    matter_id: string(raw.matter_id, raw.matterId),
    generated_at: string(raw.generated_at, raw.generatedAt),
    modes: array(raw.modes),
    nodes: array(raw.nodes).map((node: any) => ({
      id: string(node.id),
      kind: string(node.kind, "node"),
      label: string(node.label, node.id),
      subtitle: node.subtitle ?? null,
      status: node.status ?? null,
      risk: node.risk ?? null,
      href: node.href ?? null,
      metadata: node.metadata ?? {},
    })),
    edges: array(raw.edges).map((edge: any) => ({
      id: string(edge.id, `${edge.kind}:${edge.source}:${edge.target}`),
      source: string(edge.source),
      target: string(edge.target),
      kind: string(edge.kind, "related"),
      label: string(edge.label, edge.kind, "related"),
      status: edge.status ?? null,
      metadata: edge.metadata ?? {},
    })),
    warnings: array(raw.warnings),
  }
}

function normalizeIssueSpotResponse(input: unknown): IssueSpotResponse {
  const raw = input as any
  return {
    matter_id: string(raw.matter_id, raw.matterId),
    generated_at: string(raw.generated_at, raw.generatedAt),
    mode: string(raw.mode, "deterministic_review"),
    suggestions: array(raw.suggestions).map((suggestion: any, index: number) => ({
      suggestion_id: string(suggestion.suggestion_id, suggestion.id, `issue-${index + 1}`),
      id: string(suggestion.id, suggestion.suggestion_id, `issue-${index + 1}`),
      matter_id: string(suggestion.matter_id, raw.matter_id),
      issue_type: string(suggestion.issue_type, suggestion.issueType, "issue"),
      title: string(suggestion.title, "Issue suggestion"),
      summary: string(suggestion.summary),
      confidence: number(suggestion.confidence),
      severity: string(suggestion.severity, "warning"),
      status: string(suggestion.status, "open"),
      fact_ids: array(suggestion.fact_ids, suggestion.factIds),
      evidence_ids: array(suggestion.evidence_ids, suggestion.evidenceIds),
      document_ids: array(suggestion.document_ids, suggestion.documentIds),
      authority_refs: array(suggestion.authority_refs, suggestion.authorityRefs),
      recommended_action: string(suggestion.recommended_action, suggestion.recommendedAction),
      mode: string(suggestion.mode, raw.mode, "deterministic_review"),
    })),
    warnings: array(raw.warnings),
  }
}

function normalizeQcRun(input: unknown): QcRun {
  const raw = input as any
  return {
    qc_run_id: string(raw.qc_run_id, raw.id),
    id: string(raw.id, raw.qc_run_id),
    matter_id: string(raw.matter_id, raw.matterId),
    status: string(raw.status, "complete"),
    mode: string(raw.mode, "deterministic"),
    generated_at: string(raw.generated_at, raw.generatedAt),
    evidence_gaps: array(raw.evidence_gaps, raw.evidenceGaps).map((gap: any) => ({
      gap_id: string(gap.gap_id, gap.id),
      id: string(gap.id, gap.gap_id),
      matter_id: string(gap.matter_id, raw.matter_id),
      target_type: string(gap.target_type, gap.targetType),
      target_id: string(gap.target_id, gap.targetId),
      title: string(gap.title, "Evidence gap"),
      message: string(gap.message),
      severity: string(gap.severity, "warning"),
      status: string(gap.status, "open"),
      fact_ids: array(gap.fact_ids, gap.factIds),
      evidence_ids: array(gap.evidence_ids, gap.evidenceIds),
    })),
    authority_gaps: array(raw.authority_gaps, raw.authorityGaps).map((gap: any) => ({
      gap_id: string(gap.gap_id, gap.id),
      id: string(gap.id, gap.gap_id),
      matter_id: string(gap.matter_id, raw.matter_id),
      target_type: string(gap.target_type, gap.targetType),
      target_id: string(gap.target_id, gap.targetId),
      title: string(gap.title, "Authority gap"),
      message: string(gap.message),
      severity: string(gap.severity, "warning"),
      status: string(gap.status, "open"),
      authority_refs: array(gap.authority_refs, gap.authorityRefs),
    })),
    contradictions: array(raw.contradictions).map((item: any) => ({
      contradiction_id: string(item.contradiction_id, item.id),
      id: string(item.id, item.contradiction_id),
      matter_id: string(item.matter_id, raw.matter_id),
      title: string(item.title, "Contradiction"),
      message: string(item.message),
      severity: string(item.severity, "warning"),
      status: string(item.status, "open"),
      fact_ids: array(item.fact_ids, item.factIds),
      evidence_ids: array(item.evidence_ids, item.evidenceIds),
      source_document_ids: array(item.source_document_ids, item.sourceDocumentIds),
    })),
    fact_findings: array(raw.fact_findings, raw.factFindings).map(normalizeFactCheckFinding),
    citation_findings: array(raw.citation_findings, raw.citationFindings).map(normalizeCitationCheckFinding),
    work_product_findings: array(raw.work_product_findings, raw.workProductFindings).map(normalizeWorkProductFinding),
    work_product_sentences: array(raw.work_product_sentences, raw.workProductSentences).map((sentence: any, index: number) => ({
      sentence_id: string(sentence.sentence_id, sentence.id, `sentence-${index + 1}`),
      id: string(sentence.id, sentence.sentence_id, `sentence-${index + 1}`),
      matter_id: string(sentence.matter_id, raw.matter_id),
      work_product_id: string(sentence.work_product_id, sentence.workProductId),
      block_id: string(sentence.block_id, sentence.blockId),
      text: string(sentence.text),
      index: number(sentence.index, index),
      support_status: string(sentence.support_status, sentence.supportStatus, "unsupported"),
      fact_ids: array(sentence.fact_ids, sentence.factIds),
      evidence_ids: array(sentence.evidence_ids, sentence.evidenceIds),
      authority_refs: array(sentence.authority_refs, sentence.authorityRefs),
      finding_ids: array(sentence.finding_ids, sentence.findingIds),
    })),
    suggested_tasks: array(raw.suggested_tasks, raw.suggestedTasks).map((task: any) => ({
      title: string(task.title),
      status: task.status ?? "todo",
      priority: task.priority ?? "med",
      due_date: task.due_date ?? task.dueDate ?? null,
      assigned_to: task.assigned_to ?? task.assignedTo ?? null,
      related_claim_ids: array(task.related_claim_ids, task.relatedClaimIds),
      related_document_ids: array(task.related_document_ids, task.relatedDocumentIds),
      related_deadline_id: task.related_deadline_id ?? task.relatedDeadlineId ?? null,
      source: string(task.source, "qc_run"),
      description: task.description ?? null,
    })),
    warnings: array(raw.warnings),
  }
}

function normalizeExportPackage(input: any): ExportPackage {
  return {
    export_package_id: string(input.export_package_id, input.id),
    id: string(input.id, input.export_package_id),
    matter_id: string(input.matter_id, input.matterId),
    format: string(input.format),
    status: string(input.status, "review_needed"),
    profile: string(input.profile, "matter-package"),
    created_at: string(input.created_at, input.createdAt),
    artifact_count: number(input.artifact_count, input.artifactCount),
    work_product_ids: array(input.work_product_ids, input.workProductIds),
    warnings: array(input.warnings),
    download_url: input.download_url ?? input.downloadUrl ?? null,
  }
}

function normalizeAuditEvent(input: any): AuditEvent {
  return {
    audit_event_id: string(input.audit_event_id, input.id),
    id: string(input.id, input.audit_event_id),
    matter_id: string(input.matter_id, input.matterId),
    event_type: string(input.event_type, input.eventType),
    actor: string(input.actor, "system"),
    target_type: string(input.target_type, input.targetType),
    target_id: string(input.target_id, input.targetId),
    summary: string(input.summary),
    created_at: string(input.created_at, input.createdAt),
    metadata: (input.metadata ?? {}) as Record<string, string>,
  }
}

function normalizeMatterAskResponse(input: unknown): MatterAskResponse {
  const raw = input as any
  return {
    answer: string(raw.answer),
    citations: array(raw.citations).map((citation: any, index: number) => ({
      citation_id: string(citation.citation_id, citation.id, `citation-${index + 1}`),
      kind: string(citation.kind, "source"),
      source_id: string(citation.source_id, citation.sourceId),
      title: string(citation.title, citation.source_id, "Source"),
      snippet: citation.snippet ?? null,
    })),
    source_spans: array(raw.source_spans, raw.sourceSpans).map(normalizeSourceSpan),
    related_facts: array(raw.related_facts, raw.relatedFacts).map(normalizeFact),
    related_documents: array(raw.related_documents, raw.relatedDocuments).map(normalizeDocument),
    warnings: array(raw.warnings),
    mode: string(raw.mode, "retrieval"),
    thread_id: raw.thread_id ?? raw.threadId ?? null,
  }
}

function normalizeDraft(input: any): Draft {
  return {
    ...input,
    id: string(input.id, input.draft_id),
    draft_id: string(input.draft_id, input.id),
    kind: string(input.kind, input.draft_type, "complaint") as Draft["kind"],
    draft_type: string(input.draft_type, input.kind, "complaint") as Draft["draft_type"],
    status: string(input.status, "draft") as Draft["status"],
    lastEdited: string(input.lastEdited, input.updated_at, input.created_at, ""),
    wordCount: number(input.wordCount, input.word_count),
    word_count: number(input.word_count, input.wordCount),
    sections: array(input.sections).map(normalizeDraftSection),
    paragraphs: array(input.paragraphs).map(normalizeDraftParagraph),
    factcheck_summary: input.factcheck_summary ?? {},
    citeCheckIssues: array(input.citeCheckIssues, input.cite_check_issues),
    versions: array(input.versions),
  }
}

function normalizeDraftSection(input: any): DraftSection {
  return {
    id: string(input.id, input.section_id),
    heading: string(input.heading, "Section"),
    body: string(input.body),
    citations: array(input.citations).map((citation: any, index: number) => ({
      id: string(citation.id, `citation:${index}`),
      sourceId: string(citation.sourceId, citation.canonical_id),
      sourceKind: "statute",
      shortLabel: string(citation.shortLabel, citation.citation),
      fullLabel: string(citation.fullLabel, citation.citation),
      verified: Boolean(citation.verified ?? citation.canonical_id),
    })),
    comments: array(input.comments),
    suggestions: array(input.suggestions),
  }
}

function normalizeDraftParagraph(input: any): DraftParagraph {
  return {
    ...input,
    paragraph_id: string(input.paragraph_id, input.id),
    index: number(input.index),
    role: string(input.role, "paragraph"),
    text: string(input.text),
    fact_ids: array(input.fact_ids),
    evidence_ids: array(input.evidence_ids),
    authorities: array(input.authorities),
    factcheck_status: string(input.factcheck_status, "unchecked") as DraftParagraph["factcheck_status"],
  }
}

const CANONICAL_WORK_PRODUCT_TYPES = new Set([
  "complaint",
  "answer",
  "motion",
  "declaration",
  "affidavit",
  "memo",
  "notice",
  "letter",
  "exhibit_list",
  "proposed_order",
  "custom",
])

function normalizeWorkProductType(...values: any[]): string {
  const raw = string(...values)
  const normalized = raw.trim().toLowerCase().replace(/-/g, "_")
  if (normalized === "legal_memo" || normalized === "brief") return "memo"
  if (normalized === "demand_letter") return "letter"
  return CANONICAL_WORK_PRODUCT_TYPES.has(normalized) ? normalized : "custom"
}

function normalizeWorkProduct(input: any): WorkProduct {
  const workProductId = string(input.work_product_id, input.id, "work-product:demo")
  const matterId = string(input.matter_id, input.matterId, "matter:demo")
  const productType = normalizeWorkProductType(input.product_type, input.productType, "motion")
  const documentAst = normalizeWorkProductDocument(input.document_ast ?? input.documentAst ?? {}, {
    workProductId,
    matterId,
    productType,
    title: string(input.title, "Work product"),
    fallbackBlocks: array(input.blocks).map((block) => normalizeWorkProductBlock(block, workProductId)),
    fallbackFindings: array(input.findings).map(normalizeWorkProductFinding),
  })
  return {
    ...input,
    work_product_id: workProductId,
    id: string(input.id, workProductId),
    matter_id: matterId,
    title: string(input.title, "Work product"),
    product_type: productType,
    status: string(input.status, "draft"),
    review_status: string(input.review_status, "needs_human_review"),
    setup_stage: string(input.setup_stage, "guided_setup"),
    source_draft_id: input.source_draft_id ?? input.sourceDraftId ?? null,
    source_complaint_id: input.source_complaint_id ?? input.sourceComplaintId ?? null,
    created_at: string(input.created_at, ""),
    updated_at: string(input.updated_at, ""),
    profile: normalizeWorkProductProfile(input.profile ?? {}, productType),
    document_ast: documentAst,
    blocks: flattenWorkProductBlocks(documentAst.blocks),
    marks: array(input.marks).map(normalizeWorkProductMark),
    anchors: array(input.anchors).map(normalizeWorkProductAnchor),
    findings: documentAst.rule_findings.length
      ? documentAst.rule_findings
      : array(input.findings).map(normalizeWorkProductFinding),
    artifacts: array(input.artifacts).map(normalizeWorkProductArtifact),
    history: array(input.history).map(normalizeChangeSet),
    ai_commands: array(input.ai_commands, input.aiCommands).map((command: any) => ({
      command_id: string(command.command_id, command.id),
      label: string(command.label, command.command_id, "Command"),
      status: string(command.status, "disabled"),
      mode: string(command.mode, "template"),
      description: string(command.description),
      last_message: command.last_message ?? null,
    })),
    formatting_profile: normalizeFormattingProfile(input.formatting_profile ?? {}),
    rule_pack: normalizeRulePack(input.rule_pack ?? {}),
  }
}

function normalizeWorkProductDocument(
  input: any,
  context: {
    workProductId: string
    matterId: string
    productType: string
    title: string
    fallbackBlocks: WorkProductBlock[]
    fallbackFindings: WorkProductFinding[]
  },
): WorkProductDocument {
  const blocks = array(input.blocks).length
    ? array(input.blocks).map((block) => normalizeWorkProductBlock(block, context.workProductId))
    : context.fallbackBlocks
  const documentType = normalizeWorkProductType(
    input.document_type,
    input.documentType,
    input.product_type,
    input.productType,
    input.type,
    context.productType,
    "custom",
  )
  return {
    schema_version: string(input.schema_version, input.schemaVersion, "work-product-ast-v1"),
    document_id: string(input.document_id, input.documentId, `${context.workProductId}:document`),
    work_product_id: string(input.work_product_id, input.workProductId, context.workProductId),
    matter_id: string(input.matter_id, input.matterId, context.matterId),
    draft_id: input.draft_id ?? input.draftId ?? null,
    document_type: documentType,
    product_type: documentType,
    title: string(input.title, context.title),
    metadata: normalizeWorkProductMetadata(input.metadata ?? {}),
    blocks,
    links: array(input.links).map(normalizeWorkProductLink),
    citations: array(input.citations).map(normalizeWorkProductCitationUse),
    exhibits: array(input.exhibits).map(normalizeWorkProductExhibitReference),
    rule_findings: array(input.rule_findings, input.ruleFindings).length
      ? array(input.rule_findings, input.ruleFindings).map(normalizeWorkProductFinding)
      : context.fallbackFindings,
    tombstones: array(input.tombstones).map((block) => normalizeWorkProductBlock(block, context.workProductId)),
    created_at: string(input.created_at, input.createdAt),
    updated_at: string(input.updated_at, input.updatedAt),
  }
}

function normalizeWorkProductMetadata(input: any) {
  return {
    work_product_type: input.work_product_type ?? input.workProductType ?? null,
    document_title: input.document_title ?? input.documentTitle ?? null,
    jurisdiction: input.jurisdiction ?? null,
    court: input.court ?? null,
    county: input.county ?? null,
    case_number: input.case_number ?? input.caseNumber ?? null,
    rule_pack_id: input.rule_pack_id ?? input.rulePackId ?? null,
    template_id: input.template_id ?? input.templateId ?? null,
    formatting_profile_id: input.formatting_profile_id ?? input.formattingProfileId ?? null,
    parties: input.parties
      ? {
          plaintiffs: array(input.parties.plaintiffs),
          defendants: array(input.parties.defendants),
          petitioners: array(input.parties.petitioners),
          respondents: array(input.parties.respondents),
        }
      : null,
    status: string(input.status, "draft"),
    created_at: input.created_at ?? input.createdAt ?? null,
    updated_at: input.updated_at ?? input.updatedAt ?? null,
    created_by: input.created_by ?? input.createdBy ?? null,
    last_modified_by: input.last_modified_by ?? input.lastModifiedBy ?? null,
  }
}

function normalizeWorkProductProfile(input: any, productType: string) {
  return {
    profile_id: string(input.profile_id, `work-product-${productType}-v1`),
    product_type: string(input.product_type, productType),
    name: string(input.name, "Structured Work Product"),
    jurisdiction: string(input.jurisdiction, "Oregon"),
    version: string(input.version, "provider-free"),
    route_slug: string(input.route_slug, productType.replace(/_/g, "-")),
    required_block_roles: array(input.required_block_roles),
    optional_block_roles: array(input.optional_block_roles),
    supports_rich_text: Boolean(input.supports_rich_text ?? true),
  }
}

function normalizeWorkProductBlock(input: any, workProductId: string): WorkProductBlock {
  const blockId = string(input.block_id, input.id)
  const blockType = string(input.type, input.block_type, "section")
  const orderIndex = number(input.order_index, input.orderIndex, input.ordinal)
  return {
    ...input,
    block_id: blockId,
    id: string(input.id, blockId),
    matter_id: string(input.matter_id),
    work_product_id: string(input.work_product_id, workProductId),
    type: blockType,
    block_type: blockType,
    role: string(input.role, "custom"),
    title: string(input.title, input.role, "Section"),
    text: string(input.text),
    order_index: orderIndex,
    ordinal: orderIndex,
    parent_block_id: input.parent_block_id ?? null,
    parent_id: input.parent_id ?? input.parent_block_id ?? null,
    children: array(input.children).map((child) => normalizeWorkProductBlock(child, workProductId)),
    links: array(input.links),
    citations: array(input.citations),
    exhibits: array(input.exhibits),
    rule_finding_ids: array(input.rule_finding_ids, input.ruleFindingIds),
    paragraph_number: input.paragraph_number ?? input.paragraphNumber ?? null,
    sentence_index: input.sentence_index ?? input.sentenceIndex ?? null,
    sentence_id: input.sentence_id ?? input.sentenceId ?? null,
    section_kind: input.section_kind ?? input.sectionKind ?? null,
    count_number: input.count_number ?? input.countNumber ?? null,
    claim_type: input.claim_type ?? input.claimType ?? null,
    defendants: array(input.defendants),
    requested_relief: array(input.requested_relief, input.requestedRelief),
    support_status: input.support_status ?? input.supportStatus ?? null,
    created_at: string(input.created_at, input.createdAt),
    updated_at: string(input.updated_at, input.updatedAt),
    fact_ids: array(input.fact_ids),
    evidence_ids: array(input.evidence_ids),
    authorities: array(input.authorities),
    mark_ids: array(input.mark_ids),
    locked: Boolean(input.locked),
    tombstoned: Boolean(input.tombstoned),
    deleted_at: input.deleted_at ?? input.deletedAt ?? null,
    source_document_id: input.source_document_id ?? input.sourceDocumentId ?? null,
    source_span_id: input.source_span_id ?? input.sourceSpanId ?? null,
    created_by: input.created_by ?? input.createdBy ?? null,
    last_modified_by: input.last_modified_by ?? input.lastModifiedBy ?? null,
    provenance: input.provenance ?? null,
    review_status: string(input.review_status, "needs_review"),
    prosemirror_json: input.prosemirror_json ?? null,
  }
}

function flattenWorkProductBlocks(blocks: WorkProductBlock[]): WorkProductBlock[] {
  return blocks.flatMap((block) => {
    const { children, ...rest } = block
    return [{ ...rest, children }, ...flattenWorkProductBlocks(children)]
  })
}

function normalizeTextRange(input: any) {
  if (!input) return null
  return {
    start_offset: number(input.start_offset, input.startOffset),
    end_offset: number(input.end_offset, input.endOffset),
    quote: input.quote ?? null,
  }
}

function normalizeWorkProductLink(input: any): WorkProductLink {
  return {
    link_id: string(input.link_id, input.linkId, input.id),
    source_block_id: string(input.source_block_id, input.sourceBlockId),
    source_text_range: normalizeTextRange(input.source_text_range ?? input.sourceTextRange),
    target_type: string(input.target_type, input.targetType),
    target_id: string(input.target_id, input.targetId),
    relation: string(input.relation, "supports"),
    confidence: input.confidence ?? null,
    created_by: string(input.created_by, input.createdBy, "system"),
    created_at: string(input.created_at, input.createdAt),
  }
}

function normalizeWorkProductCitationUse(input: any): WorkProductCitationUse {
  return {
    citation_use_id: string(input.citation_use_id, input.citationUseId, input.id),
    source_block_id: string(input.source_block_id, input.sourceBlockId),
    source_text_range: normalizeTextRange(input.source_text_range ?? input.sourceTextRange),
    raw_text: string(input.raw_text, input.rawText, input.citation),
    normalized_citation: input.normalized_citation ?? input.normalizedCitation ?? input.citation ?? null,
    target_type: string(input.target_type, input.targetType, "unknown"),
    target_id: input.target_id ?? input.targetId ?? input.canonical_id ?? null,
    pinpoint: input.pinpoint ?? null,
    status: string(input.status, "needs_review"),
    resolver_message: input.resolver_message ?? input.resolverMessage ?? null,
    created_at: string(input.created_at, input.createdAt),
  }
}

function normalizeWorkProductExhibitReference(input: any): WorkProductExhibitReference {
  return {
    exhibit_reference_id: string(input.exhibit_reference_id, input.exhibitReferenceId, input.id),
    source_block_id: string(input.source_block_id, input.sourceBlockId),
    source_text_range: normalizeTextRange(input.source_text_range ?? input.sourceTextRange),
    label: string(input.label, input.exhibit_label, "Exhibit"),
    exhibit_id: input.exhibit_id ?? input.exhibitId ?? null,
    document_id: input.document_id ?? input.documentId ?? null,
    page_range: input.page_range ?? input.pageRange ?? null,
    status: string(input.status, "needs_review"),
    created_at: string(input.created_at, input.createdAt),
  }
}

function normalizeWorkProductMark(input: any): WorkProductMark {
  return {
    ...input,
    mark_id: string(input.mark_id, input.id),
    id: string(input.id, input.mark_id),
    matter_id: string(input.matter_id),
    work_product_id: string(input.work_product_id),
    block_id: string(input.block_id),
    mark_type: string(input.mark_type, "annotation"),
    from_offset: number(input.from_offset),
    to_offset: number(input.to_offset),
    label: string(input.label),
    target_type: string(input.target_type),
    target_id: string(input.target_id),
    status: string(input.status, "needs_review"),
  }
}

function normalizeWorkProductAnchor(input: any): WorkProductAnchor {
  return {
    ...input,
    anchor_id: string(input.anchor_id, input.id),
    id: string(input.id, input.anchor_id),
    matter_id: string(input.matter_id),
    work_product_id: string(input.work_product_id),
    block_id: string(input.block_id),
    anchor_type: string(input.anchor_type, "support"),
    target_type: string(input.target_type),
    target_id: string(input.target_id),
    relation: string(input.relation, "supports"),
    citation: input.citation ?? null,
    canonical_id: input.canonical_id ?? null,
    pinpoint: input.pinpoint ?? null,
    quote: input.quote ?? null,
    status: string(input.status, "needs_review"),
  }
}

function normalizeWorkProductFinding(input: any): WorkProductFinding {
  return {
    ...input,
    finding_id: string(input.finding_id, input.id),
    id: string(input.id, input.finding_id),
    matter_id: string(input.matter_id),
    work_product_id: string(input.work_product_id),
    rule_id: string(input.rule_id),
    rule_pack_id: input.rule_pack_id ?? input.rulePackId ?? null,
    source_citation: input.source_citation ?? input.sourceCitation ?? null,
    source_url: input.source_url ?? input.sourceUrl ?? null,
    category: string(input.category, "rules"),
    severity: string(input.severity, "warning"),
    target_type: string(input.target_type, "work_product"),
    target_id: string(input.target_id),
    message: string(input.message),
    explanation: string(input.explanation),
    suggested_fix: string(input.suggested_fix),
    auto_fix_available: Boolean(input.auto_fix_available ?? input.autoFixAvailable),
    primary_action: input.primary_action ?? {
      action_id: "action:open",
      label: "Open editor",
      action_type: "open_editor",
      target_type: string(input.target_type, "work_product"),
      target_id: string(input.target_id),
    },
    status: string(input.status, "open"),
    created_at: string(input.created_at, ""),
    updated_at: string(input.updated_at, ""),
  }
}

function normalizeWorkProductArtifact(input: any): WorkProductArtifact {
  return {
    ...input,
    artifact_id: string(input.artifact_id, input.id),
    id: string(input.id, input.artifact_id),
    matter_id: string(input.matter_id),
    work_product_id: string(input.work_product_id),
    format: string(input.format, "html"),
    profile: string(input.profile, "review"),
    mode: string(input.mode, "review_needed"),
    status: string(input.status, "generated_review_needed"),
    download_url: string(input.download_url),
    page_count: number(input.page_count, 1),
    generated_at: string(input.generated_at, ""),
    warnings: array(input.warnings),
    content_preview: string(input.content_preview),
    snapshot_id: input.snapshot_id ?? null,
    artifact_hash: input.artifact_hash ?? null,
    render_profile_hash: input.render_profile_hash ?? null,
    qc_status_at_export: input.qc_status_at_export ?? null,
    changed_since_export:
      typeof input.changed_since_export === "boolean" ? input.changed_since_export : input.changed_since_export ?? null,
    immutable: typeof input.immutable === "boolean" ? input.immutable : input.immutable ?? null,
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    size_bytes: typeof input.size_bytes === "number" ? input.size_bytes : input.sizeBytes ?? null,
    mime_type: input.mime_type ?? input.mimeType ?? null,
    storage_status: input.storage_status ?? input.storageStatus ?? null,
  }
}

function normalizeLegalImpactSummary(input: any): LegalImpactSummary {
  return {
    affected_counts: array(input?.affected_counts),
    affected_elements: array(input?.affected_elements),
    affected_facts: array(input?.affected_facts),
    affected_evidence: array(input?.affected_evidence),
    affected_authorities: array(input?.affected_authorities),
    affected_exhibits: array(input?.affected_exhibits),
    support_status_before: input?.support_status_before ?? null,
    support_status_after: input?.support_status_after ?? null,
    qc_warnings_added: array(input?.qc_warnings_added),
    qc_warnings_resolved: array(input?.qc_warnings_resolved),
    blocking_issues_added: array(input?.blocking_issues_added),
    blocking_issues_resolved: array(input?.blocking_issues_resolved),
  }
}

function normalizeVersionChangeSummary(input: any): VersionChangeSummary {
  return {
    text_changes: number(input?.text_changes),
    support_changes: number(input?.support_changes),
    citation_changes: number(input?.citation_changes),
    authority_changes: number(input?.authority_changes),
    qc_changes: number(input?.qc_changes),
    export_changes: number(input?.export_changes),
    ai_changes: number(input?.ai_changes),
    targets_changed: array(input?.targets_changed).map((target: any) => ({
      target_type: string(target.target_type),
      target_id: string(target.target_id),
      label: target.label ?? null,
    })),
    risk_level: string(input?.risk_level, "low"),
    user_summary: string(input?.user_summary),
  }
}

function normalizeChangeSet(input: any): ChangeSet {
  const changeSetId = string(input.change_set_id, input.event_id, input.id)
  return {
    ...input,
    change_set_id: changeSetId,
    id: string(input.id, changeSetId),
    matter_id: string(input.matter_id),
    subject_id: string(input.subject_id, input.work_product_id),
    branch_id: string(input.branch_id),
    snapshot_id: string(input.snapshot_id),
    parent_snapshot_id: input.parent_snapshot_id ?? null,
    title: string(input.title, input.summary, input.event_type, "History event"),
    summary: string(input.summary),
    reason: input.reason ?? null,
    actor_type: string(input.actor_type, "system"),
    actor_id: input.actor_id ?? null,
    source: string(input.source, input.event_type, "system"),
    created_at: string(input.created_at, input.timestamp),
    change_ids: array(input.change_ids),
    legal_impact: normalizeLegalImpactSummary(input.legal_impact ?? {}),
  }
}

function normalizeVersionSnapshot(input: any): VersionSnapshot {
  const snapshotId = string(input.snapshot_id, input.id)
  return {
    ...input,
    snapshot_id: snapshotId,
    id: string(input.id, snapshotId),
    matter_id: string(input.matter_id),
    subject_type: string(input.subject_type, "work_product"),
    subject_id: string(input.subject_id),
    product_type: string(input.product_type),
    profile_id: string(input.profile_id),
    branch_id: string(input.branch_id),
    sequence_number: number(input.sequence_number),
    title: string(input.title, `Snapshot ${number(input.sequence_number)}`),
    message: input.message ?? null,
    created_at: string(input.created_at),
    created_by: string(input.created_by, "system"),
    actor_id: input.actor_id ?? null,
    snapshot_type: string(input.snapshot_type, "auto"),
    parent_snapshot_ids: array(input.parent_snapshot_ids),
    document_hash: string(input.document_hash),
    support_graph_hash: string(input.support_graph_hash),
    qc_state_hash: string(input.qc_state_hash),
    formatting_hash: string(input.formatting_hash),
    manifest_hash: string(input.manifest_hash),
    manifest_ref: input.manifest_ref ?? null,
    full_state_ref: input.full_state_ref ?? null,
    full_state_inline: input.full_state_inline ?? null,
    summary: normalizeVersionChangeSummary(input.summary ?? {}),
  }
}

function normalizeVersionTextDiff(input: any): VersionTextDiff {
  return {
    target_type: string(input.target_type, "block"),
    target_id: string(input.target_id),
    title: string(input.title, "Block"),
    status: string(input.status, "unchanged"),
    before: input.before ?? null,
    after: input.after ?? null,
  }
}

function normalizeVersionLayerDiff(input: any): VersionLayerDiff {
  return {
    layer: string(input.layer, "support"),
    target_type: string(input.target_type, "work_product"),
    target_id: string(input.target_id),
    title: string(input.title, "Change"),
    status: string(input.status, "modified"),
    before_hash: input.before_hash ?? input.beforeHash ?? null,
    after_hash: input.after_hash ?? input.afterHash ?? null,
    before_summary: input.before_summary ?? input.beforeSummary ?? null,
    after_summary: input.after_summary ?? input.afterSummary ?? null,
  }
}

function normalizeCompareVersionsResponse(input: any): CompareVersionsResponse {
  return {
    matter_id: string(input.matter_id),
    subject_id: string(input.subject_id),
    from_snapshot_id: string(input.from_snapshot_id),
    to_snapshot_id: string(input.to_snapshot_id),
    layers: array(input.layers, ["text"]),
    summary: normalizeVersionChangeSummary(input.summary ?? {}),
    text_diffs: array(input.text_diffs).map(normalizeVersionTextDiff),
    layer_diffs: array(input.layer_diffs, input.layerDiffs).map(normalizeVersionLayerDiff),
  }
}

function normalizeRestoreVersionResponse(input: any): RestoreVersionResponse {
  return {
    restored: Boolean(input.restored),
    dry_run: Boolean(input.dry_run),
    warnings: array(input.warnings),
    snapshot_id: string(input.snapshot_id),
    change_set: input.change_set ? normalizeChangeSet(input.change_set) : null,
    result: input.result ? normalizeWorkProduct(input.result) : null,
  }
}

function normalizeAstValidationResponse(input: any): AstValidationResponse {
  return {
    valid: Boolean(input.valid),
    errors: array(input.errors).map(normalizeAstValidationIssue),
    warnings: array(input.warnings).map(normalizeAstValidationIssue),
  }
}

function normalizeAstValidationIssue(input: any) {
  return {
    code: string(input.code),
    message: string(input.message),
    severity: input.severity ?? null,
    blocking: Boolean(input.blocking),
    target_type: input.target_type ?? input.targetType ?? null,
    target_id: input.target_id ?? input.targetId ?? null,
  }
}

function normalizeAstMarkdownResponse(input: any): AstMarkdownResponse {
  return {
    markdown: string(input.markdown),
    warnings: array(input.warnings),
  }
}

function normalizeAstDocumentResponse(input: any): AstDocumentResponse {
  const raw = input.document_ast ?? input.documentAst ?? {}
  const workProductId = string(raw.work_product_id, raw.workProductId, "work-product:ast")
  return {
    document_ast: normalizeWorkProductDocument(raw, {
      workProductId,
      matterId: string(raw.matter_id, raw.matterId),
      productType: normalizeWorkProductType(raw.document_type, raw.documentType, raw.product_type, raw.type, "custom"),
      title: string(raw.title, "Work product"),
      fallbackBlocks: [],
      fallbackFindings: [],
    }),
    warnings: array(input.warnings),
  }
}

function normalizeAstRenderedResponse(input: any): AstRenderedResponse {
  return {
    html: input.html ?? null,
    plain_text: input.plain_text ?? input.plainText ?? null,
    warnings: array(input.warnings),
  }
}

function normalizeAIEditAudit(input: any): AIEditAudit {
  const auditId = string(input.ai_audit_id, input.id)
  return {
    ...input,
    ai_audit_id: auditId,
    id: string(input.id, auditId),
    matter_id: string(input.matter_id),
    subject_type: string(input.subject_type, "work_product"),
    subject_id: string(input.subject_id),
    target_type: string(input.target_type),
    target_id: string(input.target_id),
    command: string(input.command),
    prompt_template_id: input.prompt_template_id ?? null,
    model: input.model ?? null,
    provider_mode: string(input.provider_mode, "template"),
    input_fact_ids: array(input.input_fact_ids),
    input_evidence_ids: array(input.input_evidence_ids),
    input_authority_ids: array(input.input_authority_ids),
    input_snapshot_id: string(input.input_snapshot_id),
    output_text: input.output_text ?? null,
    inserted_text: input.inserted_text ?? null,
    user_action: string(input.user_action, "recorded"),
    warnings: array(input.warnings),
    created_at: string(input.created_at),
  }
}

function normalizeWorkProductPreview(input: any): WorkProductPreviewResponse {
  return {
    work_product_id: string(input.work_product_id),
    matter_id: string(input.matter_id),
    html: string(input.html),
    plain_text: string(input.plain_text),
    page_count: number(input.page_count, 1),
    warnings: array(input.warnings),
    generated_at: string(input.generated_at),
    review_label: string(input.review_label, "Review needed"),
  }
}

function normalizeFormattingProfile(input: any): FormattingProfile {
  return {
    profile_id: string(input.profile_id, "oregon-circuit-civil"),
    name: string(input.name, "Oregon Circuit Civil"),
    jurisdiction: string(input.jurisdiction, "Oregon"),
    line_numbers: Boolean(input.line_numbers ?? true),
    double_spaced: Boolean(input.double_spaced ?? true),
    first_page_top_blank_inches: number(input.first_page_top_blank_inches, 2),
    margin_top_inches: number(input.margin_top_inches, 1),
    margin_bottom_inches: number(input.margin_bottom_inches, 1),
    margin_left_inches: number(input.margin_left_inches, 1),
    margin_right_inches: number(input.margin_right_inches, 1),
    font_family: string(input.font_family, "Times New Roman"),
    font_size_pt: number(input.font_size_pt, 12),
  }
}

function normalizeRulePack(input: any): RulePack {
  return {
    rule_pack_id: string(input.rule_pack_id, "baseline"),
    name: string(input.name, "Baseline rule pack"),
    jurisdiction: string(input.jurisdiction, "Oregon"),
    version: string(input.version, "provider-free"),
    effective_date: string(input.effective_date, ""),
    rule_profile: normalizeRuleProfileSummary(input.rule_profile ?? {}),
    rules: array(input.rules).map((rule: any) => ({
      rule_id: string(rule.rule_id),
      source_citation: string(rule.source_citation),
      source_url: string(rule.source_url),
      severity: string(rule.severity, "warning"),
      target_type: string(rule.target_type),
      category: string(rule.category),
      message: string(rule.message),
      explanation: string(rule.explanation),
      suggested_fix: string(rule.suggested_fix),
      auto_fix_available: Boolean(rule.auto_fix_available),
    })),
  }
}

function normalizeRuleProfileSummary(input: any) {
  return {
    jurisdiction_id: string(input.jurisdiction_id, "or:state"),
    court_id: input.court_id ?? null,
    court: input.court ?? null,
    filing_date: input.filing_date ?? null,
    utcr_edition_id: input.utcr_edition_id ?? "or:utcr@2025",
    slr_edition_id: input.slr_edition_id ?? null,
    active_statewide_order_ids: array(input.active_statewide_order_ids).map((id: any) => string(id)),
    active_local_order_ids: array(input.active_local_order_ids).map((id: any) => string(id)),
    active_out_of_cycle_amendment_ids: array(input.active_out_of_cycle_amendment_ids).map((id: any) => string(id)),
    currentness_warnings: array(input.currentness_warnings).map((warning: any) => string(warning)),
    resolver_endpoint: string(
      input.resolver_endpoint,
      "/api/v1/rules/applicable?jurisdiction=Linn&date=YYYY-MM-DD&type=complaint",
    ),
  }
}

function normalizeFactCheckFinding(input: any): CaseFactCheckFinding {
  return {
    ...input,
    id: string(input.id, input.finding_id),
    finding_id: string(input.finding_id, input.id),
    matter_id: string(input.matter_id),
    draft_id: string(input.draft_id),
    paragraph_id: input.paragraph_id ?? input.paragraphId ?? null,
    finding_type: string(input.finding_type, input.findingType, "unsupported_fact"),
    severity: string(input.severity, "warning"),
    message: string(input.message),
    source_fact_ids: array(input.source_fact_ids, input.sourceFactIds),
    source_evidence_ids: array(input.source_evidence_ids, input.sourceEvidenceIds),
    status: string(input.status, "open"),
  }
}

function normalizeCitationCheckFinding(input: any): CaseCitationCheckFinding {
  return {
    ...input,
    id: string(input.id, input.finding_id),
    finding_id: string(input.finding_id, input.id),
    matter_id: string(input.matter_id),
    draft_id: string(input.draft_id),
    citation: string(input.citation),
    canonical_id: input.canonical_id ?? input.canonicalId ?? null,
    finding_type: string(input.finding_type, input.findingType, "missing_citation"),
    severity: string(input.severity, "warning"),
    message: string(input.message),
    status: string(input.status, "open"),
  }
}

function normalizeAuthorityAttachment(input: any): AuthorityAttachmentResponse {
  return {
    matter_id: string(input.matter_id),
    target_type: string(input.target_type, "claim") as AuthorityTargetType,
    target_id: string(input.target_id),
    authority: input.authority,
    attached: Boolean(input.attached),
  }
}

function array(...values: any[]): any[] {
  for (const value of values) {
    if (Array.isArray(value)) return value
  }
  return []
}

function string(...values: any[]): string {
  for (const value of values) {
    if (typeof value === "string" && value.length > 0) return value
  }
  return ""
}

function optionalString(...values: any[]): string | undefined {
  const value = string(...values)
  return value || undefined
}

function booleanWithDefault(value: any, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback
}

function nullableBoolean(value: any): boolean | null {
  return typeof value === "boolean" ? value : null
}

function number(...values: any[]): number {
  for (const value of values) {
    if (typeof value === "number" && Number.isFinite(value)) return value
    if (typeof value === "string" && value.trim() && Number.isFinite(Number(value))) return Number(value)
  }
  return 0
}

function nullableNumber(...values: any[]): number | null {
  for (const value of values) {
    if (value == null) continue
    if (typeof value === "number" && Number.isFinite(value)) return value
    if (typeof value === "string" && value.trim() && Number.isFinite(Number(value))) return Number(value)
  }
  return null
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function titleFromFilename(filename?: string) {
  return string(filename)
    .replace(/\.[a-z0-9]+$/i, "")
    .replace(/[_-]+/g, " ")
    .trim()
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}

function shouldUseDemoMatterFallback(error: unknown, options: { allowNotFound?: boolean } = {}) {
  return DEMO_MODE || isOfflineCaseBuilderError(error) || Boolean(options.allowNotFound && isNotFoundError(error))
}

function isOfflineCaseBuilderError(error: unknown) {
  const message = errorMessage(error).toLowerCase()
  return (
    message.includes("fetch failed") ||
    message.includes("failed to fetch") ||
    message.includes("econnrefused") ||
    message.includes("enotfound") ||
    message.includes("network") ||
    message.includes("timed out") ||
    message.includes("timeout") ||
    message.includes("aborted")
  )
}

function isNotFoundError(error: unknown) {
  const message = errorMessage(error).toLowerCase()
  return message.includes("api error: 404") || message.includes("not found")
}

function isAbortError(error: unknown) {
  return error instanceof DOMException
    ? error.name === "AbortError"
    : error instanceof Error && error.name === "AbortError"
}
