import { fireEvent, render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type {
  CaseDocument,
  DocumentWorkspace as DocumentWorkspaceState,
  Matter,
  TranscriptionJobResponse,
} from "@/lib/casebuilder/types"
import { DocumentWorkspace } from "./document-workspace"

const router = {
  push: vi.fn(),
  refresh: vi.fn(),
}

vi.mock("next/navigation", () => ({
  useRouter: () => router,
}))

const createEvidence = vi.fn()
const createFact = vi.fn()
const createTimelineEvent = vi.fn()
const createDocumentAnnotation = vi.fn()
const createTranscription = vi.fn()
const createDocumentDownloadUrl = vi.fn()
const extractDocument = vi.fn()
const patchTranscriptSegment = vi.fn()
const patchTranscriptSpeaker = vi.fn()
const promoteDocumentWorkProduct = vi.fn()
const reviewTranscription = vi.fn()
const saveDocumentText = vi.fn()
const suggestTimeline = vi.fn()
const syncTranscription = vi.fn()

vi.mock("@/lib/casebuilder/api", () => ({
  createEvidence: (...args: unknown[]) => createEvidence(...args),
  createFact: (...args: unknown[]) => createFact(...args),
  createTimelineEvent: (...args: unknown[]) => createTimelineEvent(...args),
  createDocumentAnnotation: (...args: unknown[]) => createDocumentAnnotation(...args),
  createTranscription: (...args: unknown[]) => createTranscription(...args),
  createDocumentDownloadUrl: (...args: unknown[]) => createDocumentDownloadUrl(...args),
  extractDocument: (...args: unknown[]) => extractDocument(...args),
  patchTranscriptSegment: (...args: unknown[]) => patchTranscriptSegment(...args),
  patchTranscriptSpeaker: (...args: unknown[]) => patchTranscriptSpeaker(...args),
  promoteDocumentWorkProduct: (...args: unknown[]) => promoteDocumentWorkProduct(...args),
  reviewTranscription: (...args: unknown[]) => reviewTranscription(...args),
  saveDocumentText: (...args: unknown[]) => saveDocumentText(...args),
  suggestTimeline: (...args: unknown[]) => suggestTimeline(...args),
  syncTranscription: (...args: unknown[]) => syncTranscription(...args),
}))

const transcription = makeTranscription()
const workspace = makeWorkspace(transcription)
const matter = {
  id: "matter:smith-abc",
  matter_id: "matter:smith-abc",
  name: "Smith v. ABC",
  title: "Smith v. ABC",
} as unknown as Matter

describe("DocumentWorkspace transcript review", () => {
  beforeEach(() => {
    router.push.mockReset()
    router.refresh.mockReset()
    createEvidence.mockReset()
    createFact.mockReset()
    createTimelineEvent.mockReset()
    createDocumentAnnotation.mockReset()
    createTranscription.mockReset()
    createDocumentDownloadUrl.mockReset()
    extractDocument.mockReset()
    patchTranscriptSegment.mockReset()
    patchTranscriptSpeaker.mockReset()
    promoteDocumentWorkProduct.mockReset()
    reviewTranscription.mockReset()
    saveDocumentText.mockReset()
    suggestTimeline.mockReset()
    syncTranscription.mockReset()

    createFact.mockResolvedValue({ data: { fact_id: "fact:segment" } })
    createEvidence.mockResolvedValue({ data: { evidence_id: "evidence:segment" } })
    createTimelineEvent.mockResolvedValue({ data: { event_id: "event:segment" } })
    patchTranscriptSegment.mockImplementation(
      async (
        _matterId: string,
        _documentId: string,
        _transcriptionId: string,
        segmentId: string,
        patch: { text?: string; redacted_text?: string; review_status?: string },
      ) => ({
        data: {
          ...transcription,
          segments: transcription.segments.map((segment) =>
            segment.segment_id === segmentId
              ? {
                  ...segment,
                  text: patch.text ?? segment.text,
                  redacted_text: patch.redacted_text ?? segment.redacted_text,
                  review_status: patch.review_status ?? segment.review_status,
                  edited: true,
                }
              : segment,
          ),
        },
      }),
    )
    reviewTranscription.mockResolvedValue({ data: { ...transcription, job: { ...transcription.job, status: "processed" } } })
  })

  it("commits the redacted transcript as the default review surface", async () => {
    const user = userEvent.setup()
    render(<DocumentWorkspace matter={matter} workspace={workspace} />)

    await user.click(screen.getByRole("button", { name: "Review redacted" }))

    await waitFor(() => {
      expect(reviewTranscription).toHaveBeenCalledWith(
        "matter:smith-abc",
        "doc:audio",
        "transcription:doc-audio",
        expect.objectContaining({
          reviewed_text: expect.stringContaining("[redacted phone]"),
          review_surface: "redacted",
          status: "approved",
        }),
      )
    })
    expect(reviewTranscription.mock.calls[0][3].reviewed_text).not.toContain("503-555-0199")
  })

  it("patches redacted and raw text into their separate segment fields", async () => {
    const user = userEvent.setup()
    render(<DocumentWorkspace matter={matter} workspace={workspace} />)

    const redactedArea = screen.getByLabelText("Transcript segment 1")
    fireEvent.change(redactedArea, { target: { value: "Tenant called from [redacted]." } })
    fireEvent.blur(redactedArea)

    await waitFor(() => {
      expect(patchTranscriptSegment).toHaveBeenCalledWith(
        "matter:smith-abc",
        "doc:audio",
        "transcription:doc-audio",
        "segment:1",
        { redacted_text: "Tenant called from [redacted].", review_status: "edited" },
      )
    })

    patchTranscriptSegment.mockClear()
    await user.click(screen.getByRole("tab", { name: "Privacy" }))
    await user.click(screen.getByRole("button", { name: "Raw" }))
    const rawArea = screen.getByLabelText("Transcript segment 1")
    fireEvent.change(rawArea, { target: { value: "Tenant called from 503-555-0000." } })
    fireEvent.blur(rawArea)

    await waitFor(() => {
      expect(patchTranscriptSegment).toHaveBeenCalledWith(
        "matter:smith-abc",
        "doc:audio",
        "transcription:doc-audio",
        "segment:1",
        { text: "Tenant called from 503-555-0000.", review_status: "edited" },
      )
    })
  })

  it("flushes displayed redacted edits before committing review", async () => {
    const user = userEvent.setup()
    render(<DocumentWorkspace matter={matter} workspace={workspace} />)

    const redactedArea = screen.getByLabelText("Transcript segment 1")
    fireEvent.change(redactedArea, { target: { value: "Tenant called from [redacted phone] and reported flooding." } })

    await user.click(screen.getByRole("button", { name: "Review redacted" }))

    await waitFor(() => {
      expect(patchTranscriptSegment).toHaveBeenCalledWith(
        "matter:smith-abc",
        "doc:audio",
        "transcription:doc-audio",
        "segment:1",
        { redacted_text: "Tenant called from [redacted phone] and reported flooding.", review_status: "edited" },
      )
      expect(reviewTranscription).toHaveBeenCalledWith(
        "matter:smith-abc",
        "doc:audio",
        "transcription:doc-audio",
        expect.objectContaining({
          reviewed_text: expect.stringContaining("Tenant called from [redacted phone] and reported flooding."),
        }),
      )
    })
  })

  it("creates transcript-derived case items with document and source-span provenance", async () => {
    const user = userEvent.setup()
    render(<DocumentWorkspace matter={matter} workspace={workspace} />)

    const segmentCard = screen.getByText(/Speaker A/).closest("section")
    expect(segmentCard).not.toBeNull()
    const card = within(segmentCard as HTMLElement)

    await user.click(card.getByRole("button", { name: "Create fact" }))
    await user.click(card.getByRole("button", { name: "Create evidence" }))
    await user.click(card.getByRole("button", { name: "Create timeline event" }))

    await waitFor(() => {
      expect(createFact).toHaveBeenCalledWith(
        "matter:smith-abc",
        expect.objectContaining({
          source_document_ids: ["doc:audio"],
          source_span_ids: ["span:audio:1"],
        }),
      )
      expect(createEvidence).toHaveBeenCalledWith(
        "matter:smith-abc",
        expect.objectContaining({
          document_id: "doc:audio",
          source_span: "span:audio:1",
        }),
      )
      expect(createTimelineEvent).toHaveBeenCalledWith(
        "matter:smith-abc",
        expect.objectContaining({
          date: "2026-04-01",
          source_document_id: "doc:audio",
          source_span_ids: ["span:audio:1"],
        }),
      )
    })
  })

  it("uses a user-supplied transcript timeline date when the document has no event date", async () => {
    const user = userEvent.setup()
    const workspaceWithoutDate = makeWorkspace(transcription)
    workspaceWithoutDate.document.date_observed = null
    render(<DocumentWorkspace matter={matter} workspace={workspaceWithoutDate} />)

    const segmentCard = screen.getByText(/Speaker A/).closest("section")
    expect(segmentCard).not.toBeNull()
    const card = within(segmentCard as HTMLElement)

    fireEvent.change(card.getByLabelText("Timeline date for segment 1"), { target: { value: "2026-04-03" } })
    await user.click(card.getByRole("button", { name: "Create timeline event" }))

    await waitFor(() => {
      expect(createTimelineEvent).toHaveBeenCalledWith(
        "matter:smith-abc",
        expect.objectContaining({
          date: "2026-04-03",
          source_document_id: "doc:audio",
          source_span_ids: ["span:audio:1"],
        }),
      )
    })
  })
})

describe("DocumentWorkspace Markdown-only processing", () => {
  it("shows stored non-Markdown files as view-only and disables processing actions", () => {
    render(<DocumentWorkspace matter={matter} workspace={viewOnlyWorkspace(docxDocument())} />)

    expect(screen.getByText("Stored source")).toBeInTheDocument()
    expect(screen.getByRole("link", { name: /open source/i })).toHaveAttribute(
      "href",
      "/api/casebuilder/document-content?matterId=matter%3Asmith-abc&documentId=doc%3Alease",
    )
    expect(screen.getByRole("button", { name: /extract text/i })).toBeDisabled()
    expect(screen.getByRole("button", { name: /suggest timeline/i })).toBeDisabled()
    expect(screen.getByRole("button", { name: /promote to work product/i })).toBeDisabled()
    expect(screen.queryByRole("button", { name: /save text/i })).not.toBeInTheDocument()
  })
})

function makeTranscription(): TranscriptionJobResponse {
  return {
    job: {
      transcription_job_id: "transcription:doc-audio",
      id: "transcription:doc-audio",
      matter_id: "matter:smith-abc",
      document_id: "doc:audio",
      provider: "assemblyai",
      provider_mode: "disabled",
      provider_status: "completed",
      status: "processed",
      review_status: "needs_review",
      speaker_count: 1,
      segment_count: 1,
      word_count: 6,
      word_search_terms: [],
      keyterms_prompt: [],
      redact_pii: true,
      speech_models: [],
      retryable: false,
      created_at: "2026-05-02T00:00:00Z",
      updated_at: "2026-05-02T00:00:00Z",
    },
    segments: [
      {
        segment_id: "segment:1",
        id: "segment:1",
        matter_id: "matter:smith-abc",
        document_id: "doc:audio",
        transcription_job_id: "transcription:doc-audio",
        source_span_id: "span:audio:1",
        ordinal: 1,
        paragraph_ordinal: 1,
        speaker_label: "A",
        speaker_name: "Speaker A",
        text: "Tenant called from 503-555-0199.",
        redacted_text: "Tenant called from [redacted phone].",
        time_start_ms: 1000,
        time_end_ms: 4000,
        confidence: 0.92,
        review_status: "approved",
        edited: false,
        created_at: "2026-05-02T00:00:00Z",
        updated_at: "2026-05-02T00:00:00Z",
      },
    ],
    speakers: [],
    review_changes: [],
    raw_artifact_version: null,
    normalized_artifact_version: null,
    redacted_artifact_version: null,
    redacted_audio_version: null,
    reviewed_document_version: null,
    caption_vtt_version: null,
    caption_srt_version: null,
    caption_vtt: null,
    caption_srt: null,
    warnings: [],
  }
}

function makeWorkspace(activeTranscription: TranscriptionJobResponse): DocumentWorkspaceState {
  return {
    matter_id: "matter:smith-abc",
    document: {
      id: "doc:audio",
      document_id: "doc:audio",
      matter_id: "matter:smith-abc",
      title: "Call recording",
      filename: "call-recording.mp3",
      kind: "evidence",
      document_type: "evidence",
      mime_type: "audio/mpeg",
      pages: 1,
      pageCount: 1,
      bytes: 12345,
      fileSize: "12 KB",
      dateUploaded: "2026-05-02",
      uploaded_at: "2026-05-02T00:00:00Z",
      summary: "Audio call.",
      status: "processed",
      processing_status: "processed",
      is_exhibit: false,
      facts_extracted: 0,
      citations_found: 0,
      contradictions_flagged: 0,
      entities: [],
      chunks: [],
      clauses: [],
      linkedFacts: [],
      issues: [],
      parties_mentioned: [],
      entities_mentioned: [],
      linked_claim_ids: [],
      folder: "Evidence",
      storage_status: "stored",
      date_observed: "2026-04-01",
      source_spans: [],
      extracted_text: null,
    },
    current_version: null,
    capabilities: [
      { capability: "view", enabled: true, mode: "media" },
      { capability: "annotate", enabled: true, mode: "graph_sidecar" },
      { capability: "extract", enabled: true, mode: "transcript_review" },
      { capability: "edit", enabled: false, mode: "transcript_review" },
      { capability: "promote", enabled: false, mode: "disabled" },
    ],
    annotations: [],
    source_spans: [
      {
        source_span_id: "span:audio:1",
        id: "span:audio:1",
        matter_id: "matter:smith-abc",
        document_id: "doc:audio",
        time_start_ms: 1000,
        time_end_ms: 4000,
        speaker_label: "A",
        quote: "Tenant called from [redacted phone].",
        extraction_method: "assemblyai_transcript_reviewed_segment",
        confidence: 0.92,
        review_status: "approved",
      },
    ],
    transcriptions: [activeTranscription],
    docx_manifest: null,
    text_content: null,
    content_url: "/media/call-recording.mp3",
    warnings: [],
  }
}

function viewOnlyWorkspace(document: CaseDocument): DocumentWorkspaceState {
  return {
    matter_id: "matter:smith-abc",
    document,
    current_version: null,
    capabilities: [
      { capability: "view", enabled: true, mode: "stored_file" },
      { capability: "edit", enabled: false, mode: "markdown_only_disabled" },
      { capability: "annotate", enabled: true, mode: "graph_sidecar" },
      { capability: "extract", enabled: false, mode: "markdown_only_disabled" },
      { capability: "promote", enabled: false, mode: "markdown_only_disabled" },
    ],
    annotations: [],
    source_spans: [],
    transcriptions: [],
    docx_manifest: null,
    text_content: null,
    content_url: "/api/casebuilder/document-content?matterId=matter%3Asmith-abc&documentId=doc%3Alease",
    warnings: [],
  }
}

function docxDocument(): CaseDocument {
  return {
    id: "doc:lease",
    document_id: "doc:lease",
    title: "Lease ledger",
    filename: "Lease ledger.docx",
    kind: "lease",
    document_type: "lease",
    pages: 1,
    pageCount: 1,
    bytes: 1024,
    fileSize: "1 KB",
    dateUploaded: "2026-05-03",
    uploaded_at: "2026-05-03T00:00:00Z",
    summary: "Stored privately.",
    status: "view_only",
    processing_status: "view_only",
    is_exhibit: false,
    facts_extracted: 0,
    citations_found: 0,
    contradictions_flagged: 0,
    entities: [],
    chunks: [],
    clauses: [],
    linkedFacts: [],
    issues: [],
    parties_mentioned: [],
    entities_mentioned: [],
    folder: "Uploads",
    storage_status: "stored",
    mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  }
}
