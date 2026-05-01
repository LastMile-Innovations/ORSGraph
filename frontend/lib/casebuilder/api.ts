import type {
  CaseAiActionResponse,
  AuthorityAttachmentResponse,
  AuthorityTargetType,
  CaseAuthoritySearchResponse,
  CaseCitationCheckFinding,
  CaseDocument,
  CaseDefense,
  CaseEvidence,
  CaseFactCheckFinding,
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
} from "./types"
import { getMatterById as getDemoMatterById, matters as demoMatters } from "./mock-matters"
import { decodeMatterRouteId } from "./routes"

const API_BASE_URL = process.env.NEXT_PUBLIC_ORS_API_BASE_URL || "http://localhost:8080/api/v1"
const API_KEY = process.env.NEXT_PUBLIC_ORS_API_KEY

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

export async function getMatterSummariesState(): Promise<LoadState<MatterSummary[]>> {
  try {
    const live = await fetchCaseBuilder<MatterSummary[]>("/matters")
    return { source: "live", data: live.map(normalizeMatterSummary) }
  } catch (error) {
    return { source: "demo", data: demoMatters, error: errorMessage(error) }
  }
}

export async function getMatterState(id: string): Promise<LoadState<Matter | null>> {
  const matterId = decodeMatterRouteId(id)
  try {
    const live = await fetchCaseBuilder<unknown>(`/matters/${encodeURIComponent(matterId)}`)
    return { source: "live", data: normalizeMatter(live) }
  } catch (error) {
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
    tasks: array(raw.tasks),
    drafts: array(raw.drafts).map(normalizeDraft),
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
