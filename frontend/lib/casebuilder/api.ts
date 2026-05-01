import type {
  CaseAiActionResponse,
  AIEditAudit,
  AstDocumentResponse,
  AstMarkdownResponse,
  AstPatch,
  AstRenderedResponse,
  AstValidationResponse,
  AuthorityAttachmentResponse,
  AuthorityTargetType,
  CaseAuthoritySearchResponse,
  CaseCitationCheckFinding,
  CaseDocument,
  CaseDefense,
  CaseEvidence,
  CaseTask,
  CaseFactCheckFinding,
  ChangeSet,
  ComplaintCaption,
  ComplaintDraft,
  ComplaintImportResponse,
  ComplaintPreviewResponse,
  ComplaintSection,
  ComplaintCount,
  CompareVersionsResponse,
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
  Claim,
  ClaimElement,
  Deadline,
  Draft,
  DraftParagraph,
  DraftSection,
  ExtractedFact,
  IngestionRun,
  Matter,
  MatterParty,
  MatterSummary,
  SourceSpan,
  TimelineEvent,
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
import { decodeMatterRouteId } from "./routes"

const API_BASE_URL = process.env.NEXT_PUBLIC_ORS_API_BASE_URL || "http://localhost:8080/api/v1"
const API_KEY = process.env.NEXT_PUBLIC_ORS_API_KEY
const DEMO_MODE = process.env.NEXT_PUBLIC_ORS_DEMO_MODE === "true"

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

export interface CreateMatterInput {
  name: string
  matter_type?: MatterSummary["matter_type"]
  user_role?: MatterSummary["user_role"]
  jurisdiction?: string
  court?: string
  case_number?: string | null
}

export interface PatchMatterInput extends Partial<CreateMatterInput> {
  status?: MatterSummary["status"]
}

export interface UploadTextFileInput {
  filename: string
  text?: string
  mime_type?: string
  bytes?: number
  document_type?: string
  folder?: string
  confidentiality?: string
}

export interface CreateFileUploadInput {
  filename: string
  mime_type?: string
  bytes: number
  document_type?: string
  folder?: string
  confidentiality?: string
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
  }>
  proposed_facts: ExtractedFact[]
  ingestion_run?: IngestionRun | null
  document_version?: DocumentVersion | null
  source_spans: SourceSpan[]
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

export async function getMatterSummariesState(): Promise<LoadState<MatterSummary[]>> {
  try {
    const live = await fetchCaseBuilder<MatterSummary[]>("/matters")
    return { source: "live", data: live.map(normalizeMatterSummary) }
  } catch (error) {
    if (DEMO_MODE) return { source: "demo", data: demoMatters, error: errorMessage(error) }
    return { source: "error", data: [], error: errorMessage(error) }
  }
}

export async function getMatterState(id: string): Promise<LoadState<Matter | null>> {
  const matterId = decodeMatterRouteId(id)
  try {
    const live = await fetchCaseBuilder<unknown>(`/matters/${encodeURIComponent(matterId)}`)
    return { source: "live", data: normalizeMatter(live) }
  } catch (error) {
    if (!DEMO_MODE) {
      return { source: "error", data: null, error: errorMessage(error) }
    }
    const demo = getDemoMatterById(matterId) ?? null
    return { source: demo ? "demo" : "error", data: demo, error: errorMessage(error) }
  }
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
      normalize: normalizeDocument,
    },
  )
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
          document_version: response.document_version ? normalizeDocumentVersion(response.document_version) : null,
          source_spans: array(response.source_spans).map(normalizeSourceSpan),
        }
      },
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
    )
    return { source: "live", data: live.map(normalizeWorkProduct) }
  } catch (error) {
    if (!DEMO_MODE) {
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
      )
      return { source: "live", data: normalizeWorkProduct(live) }
    }
    const query = workProductListQuery(options)
    const live = await fetchCaseBuilder<unknown[]>(
      `/matters/${encodeURIComponent(decodedMatterId)}/work-products${query}`,
    )
    const products = live.map(normalizeWorkProduct).filter((product) => product.product_type !== "complaint")
    return { source: "live", data: products[0] ?? null }
  } catch (error) {
    if (!DEMO_MODE) {
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
): Promise<LoadState<ComplaintDraft | null>> {
  const decodedMatterId = decodeMatterRouteId(matterId)
  try {
    if (complaintId) {
      const live = await fetchCaseBuilder<unknown>(
        `/matters/${encodeURIComponent(decodedMatterId)}/complaints/${encodeURIComponent(complaintId)}`,
      )
      return { source: "live", data: normalizeComplaint(live) }
    }
    const live = await fetchCaseBuilder<unknown[]>(
      `/matters/${encodeURIComponent(decodedMatterId)}/complaints`,
    )
    const complaints = live.map(normalizeComplaint)
    return { source: "live", data: complaints[0] ?? null }
  } catch (error) {
    if (!DEMO_MODE) {
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

async function runCaseBuilderAction<T>(
  endpoint: string,
  options: RequestInit & { normalize: (raw: unknown) => T },
): Promise<ActionState<T>> {
  const { normalize, ...requestOptions } = options
  try {
    const raw = await fetchCaseBuilder<unknown>(endpoint, requestOptions)
    return { source: "live", data: normalize(raw) }
  } catch (error) {
    return { source: "error", data: null, error: errorMessage(error) }
  }
}

async function fetchCaseBuilder<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
  const headers = new Headers(options.headers)
  if (!headers.has("Content-Type") && typeof options.body === "string") {
    headers.set("Content-Type", "application/json")
  }
  if (API_KEY && !headers.has("x-api-key")) {
    headers.set("x-api-key", API_KEY)
  }
  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    cache: "no-store",
    ...options,
    headers,
  })
  if (!response.ok) {
    const body = await response.json().catch(() => ({}))
    throw new Error(body.error || `CaseBuilder API error: ${response.status}`)
  }
  return response.json()
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
    object_blob_id: input.object_blob_id ?? input.objectBlobId ?? null,
    current_version_id: input.current_version_id ?? input.currentVersionId ?? null,
    ingestion_run_ids: array(input.ingestion_run_ids, input.ingestionRunIds),
    source_spans: array(input.source_spans, input.sourceSpans).map(normalizeSourceSpan),
  }
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

function normalizeWorkProduct(input: any): WorkProduct {
  const workProductId = string(input.work_product_id, input.id, "work-product:demo")
  const documentAst = normalizeWorkProductDocument(input.document_ast ?? input.documentAst ?? {}, {
    workProductId,
    matterId: string(input.matter_id, "matter:demo"),
    productType: string(input.product_type, "motion"),
    title: string(input.title, "Work product"),
    fallbackBlocks: array(input.blocks).map((block) => normalizeWorkProductBlock(block, workProductId)),
    fallbackFindings: array(input.findings).map(normalizeWorkProductFinding),
  })
  return {
    ...input,
    work_product_id: workProductId,
    id: string(input.id, workProductId),
    matter_id: string(input.matter_id, "matter:demo"),
    title: string(input.title, "Work product"),
    product_type: string(input.product_type, "motion"),
    status: string(input.status, "draft"),
    review_status: string(input.review_status, "needs_human_review"),
    setup_stage: string(input.setup_stage, "guided_setup"),
    source_draft_id: input.source_draft_id ?? null,
    source_complaint_id: input.source_complaint_id ?? null,
    created_at: string(input.created_at, ""),
    updated_at: string(input.updated_at, ""),
    profile: normalizeWorkProductProfile(input.profile ?? {}, string(input.product_type, "motion")),
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
  return {
    schema_version: string(input.schema_version, input.schemaVersion, "work-product-ast-v1"),
    document_id: string(input.document_id, input.documentId, `${context.workProductId}:document`),
    work_product_id: string(input.work_product_id, input.workProductId, context.workProductId),
    matter_id: string(input.matter_id, input.matterId, context.matterId),
    product_type: string(input.product_type, input.type, context.productType),
    title: string(input.title, context.title),
    metadata: normalizeWorkProductMetadata(input.metadata ?? {}),
    blocks,
    links: array(input.links).map(normalizeWorkProductLink),
    citations: array(input.citations).map(normalizeWorkProductCitationUse),
    exhibits: array(input.exhibits).map(normalizeWorkProductExhibitReference),
    rule_findings: array(input.rule_findings, input.ruleFindings).length
      ? array(input.rule_findings, input.ruleFindings).map(normalizeWorkProductFinding)
      : context.fallbackFindings,
    created_at: string(input.created_at, input.createdAt),
    updated_at: string(input.updated_at, input.updatedAt),
  }
}

function normalizeWorkProductMetadata(input: any) {
  return {
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
    category: string(input.category, "rules"),
    severity: string(input.severity, "warning"),
    target_type: string(input.target_type, "work_product"),
    target_id: string(input.target_id),
    message: string(input.message),
    explanation: string(input.explanation),
    suggested_fix: string(input.suggested_fix),
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
      productType: string(raw.product_type, raw.type, "custom"),
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

function decodeRouteSegment(value: string) {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
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
