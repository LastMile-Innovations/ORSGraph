import { fireEvent, render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { CaseDocument, MatterSummary, TranscriptionJobResponse } from "@/lib/casebuilder/types"
import { DocumentLibrary } from "./document-library"

const router = {
  refresh: vi.fn(),
}

vi.mock("next/navigation", () => ({
  useRouter: () => router,
}))

const createTranscription = vi.fn()
const createMatterIndexJob = vi.fn()
const enqueueMatterUploads = vi.fn()
const getMatterSettingsState = vi.fn()
const getMatterIndexSummary = vi.fn()
const listTranscriptions = vi.fn()
const syncTranscription = vi.fn()
const uploadOptionsFromEffectiveSettings = vi.fn()

vi.mock("@/lib/casebuilder/api", () => ({
  createMatterIndexJob: (...args: unknown[]) => createMatterIndexJob(...args),
  createTranscription: (...args: unknown[]) => createTranscription(...args),
  getMatterSettingsState: (...args: unknown[]) => getMatterSettingsState(...args),
  getMatterIndexSummary: (...args: unknown[]) => getMatterIndexSummary(...args),
  listTranscriptions: (...args: unknown[]) => listTranscriptions(...args),
  syncTranscription: (...args: unknown[]) => syncTranscription(...args),
}))

vi.mock("./upload-provider", () => ({
  uploadOptionsFromEffectiveSettings: (...args: unknown[]) => uploadOptionsFromEffectiveSettings(...args),
  useCaseBuilderUploads: () => ({
    activeCount: 0,
    batches: [],
    cancelRow: vi.fn(),
    dismissBatch: vi.fn(),
    enqueueMatterIntake: vi.fn(),
    enqueueMatterUploads,
    retryRow: vi.fn(),
    rows: [],
  }),
}))

const matter = {
  matter_id: "matter:smith-abc",
  name: "Smith v. ABC",
} as MatterSummary

describe("DocumentLibrary media queue", () => {
  beforeEach(() => {
    router.refresh.mockReset()
    createTranscription.mockReset()
    createMatterIndexJob.mockReset()
    enqueueMatterUploads.mockReset()
    getMatterSettingsState.mockReset()
    getMatterIndexSummary.mockReset()
    listTranscriptions.mockReset()
    syncTranscription.mockReset()
    uploadOptionsFromEffectiveSettings.mockReset()

    enqueueMatterUploads.mockReturnValue("folder:test")
    getMatterSettingsState.mockResolvedValue({ data: { effective: null } })
    getMatterIndexSummary.mockResolvedValue({ data: indexSummary() })
    listTranscriptions.mockResolvedValue({ data: [transcription()] })
    uploadOptionsFromEffectiveSettings.mockReturnValue({})
  })

  it("turns the media queue into an operations table with transcript status and actions", async () => {
    const user = userEvent.setup()
    render(<DocumentLibrary matter={matter} documents={[mediaDocument(), pdfDocument()]} />)

    await user.click(screen.getByRole("button", { name: /media queue/i }))

    expect(screen.getByText("media operations queue")).toBeInTheDocument()
    expect(screen.getByText("Interview_audio.mp3")).toBeInTheDocument()
    expect(screen.queryByText("Lease.pdf")).not.toBeInTheDocument()

    await waitFor(() => {
      expect(listTranscriptions).toHaveBeenCalledWith("matter:smith-abc", "doc:audio")
    })

    const row = screen.getByText("Interview_audio.mp3").closest("tr")
    expect(row).not.toBeNull()
    const mediaRow = within(row as HTMLElement)

    expect(mediaRow.getByText("review_ready")).toBeInTheDocument()
    expect(mediaRow.getByRole("button", { name: /retry/i })).toBeDisabled()
    expect(mediaRow.getByRole("button", { name: /sync/i })).toBeDisabled()
    expect(mediaRow.getByRole("link", { name: /review/i })).toHaveAttribute(
      "href",
      "/casebuilder/matters/smith-abc/documents/doc%3Aaudio",
    )
    expect(mediaRow.getByRole("link", { name: /review/i })).toHaveAttribute("aria-disabled", "true")
    expect(mediaRow.getByRole("link", { name: /open/i })).toHaveAttribute(
      "href",
      "/casebuilder/matters/smith-abc/documents/doc%3Aaudio",
    )
    expect(screen.getByRole("button", { name: /bulk sync pending/i })).toBeDisabled()
  })

  it("keeps media transcription retry disabled while Markdown-only processing is enabled", async () => {
    const user = userEvent.setup()
    listTranscriptions.mockResolvedValue({ data: [transcription({ status: "failed", retryable: true })] })
    createTranscription.mockResolvedValue({ data: transcription({ status: "queued", retryable: false }) })
    render(<DocumentLibrary matter={matter} documents={[mediaDocument()]} />)

    await user.click(screen.getByRole("button", { name: /media queue/i }))

    const row = await screen.findByText("Interview_audio.mp3")
    const mediaRow = within(row.closest("tr") as HTMLElement)
    expect(mediaRow.getByRole("button", { name: /retry/i })).toBeDisabled()
    expect(createTranscription).not.toHaveBeenCalled()
  })

  it("queues mixed folder uploads through the shared provider and preserves relative paths", async () => {
    const user = userEvent.setup()
    const { container } = render(<DocumentLibrary matter={matter} documents={[]} />)
    const markdown = new File(["# Facts"], "facts.md", { type: "text/markdown" })
    const pdf = new File(["pdf"], "lease.pdf", { type: "application/pdf" })
    const image = new File(["img"], "photo.png", { type: "image/png" })
    const text = new File(["plain"], "notes.txt", { type: "text/plain" })
    defineRelativePath(markdown, "Case File/Facts/facts.md")
    defineRelativePath(pdf, "Case File/Contracts/lease.pdf")
    defineRelativePath(image, "Case File/Photos/photo.png")
    defineRelativePath(text, "Case File/Notes/notes.txt")

    fireEvent.change(container.querySelector<HTMLInputElement>('input[type="file"]') as HTMLInputElement, {
      target: { files: [markdown, pdf, image, text] },
    })
    await user.click(await screen.findByRole("button", { name: /upload batch/i }))

    await waitFor(() => {
      expect(enqueueMatterUploads).toHaveBeenCalledTimes(1)
      expect(screen.getByText(/upload started/i)).toBeInTheDocument()
    })
    const [matterId, candidates, options] = enqueueMatterUploads.mock.calls[0] as [
      string,
      { file: File; relativePath: string; folder: string }[],
      { label: string },
    ]
    expect(matterId).toBe("matter:smith-abc")
    expect(candidates.map((candidate) => candidate.relativePath)).toEqual([
      "Case File/Facts/facts.md",
      "Case File/Contracts/lease.pdf",
      "Case File/Photos/photo.png",
      "Case File/Notes/notes.txt",
    ])
    expect(candidates.map((candidate) => candidate.file)).toEqual([markdown, pdf, image, text])
    expect(options).toEqual({ label: "Folder upload" })
  })
})

function defineRelativePath(file: File, relativePath: string) {
  Object.defineProperty(file, "webkitRelativePath", {
    configurable: true,
    value: relativePath,
  })
}

function mediaDocument(): CaseDocument {
  return {
    ...baseDocument(),
    id: "doc:audio",
    document_id: "doc:audio",
    title: "Interview audio",
    filename: "Interview_audio.mp3",
    kind: "evidence",
    document_type: "evidence",
    mime_type: "audio/mpeg",
    processing_status: "transcription_deferred",
    status: "transcription_deferred",
    folder: "Evidence",
    summary: "Audio interview.",
  }
}

function pdfDocument(): CaseDocument {
  return {
    ...baseDocument(),
    id: "doc:lease",
    document_id: "doc:lease",
    title: "Lease",
    filename: "Lease.pdf",
    kind: "lease",
    document_type: "lease",
    mime_type: "application/pdf",
    processing_status: "processed",
    status: "processed",
    folder: "Contracts",
    summary: "Lease document.",
  }
}

function baseDocument(): CaseDocument {
  return {
    id: "doc",
    document_id: "doc",
    title: "Document",
    filename: "Document.pdf",
    kind: "evidence",
    document_type: "evidence",
    pages: 1,
    pageCount: 1,
    bytes: 1024,
    fileSize: "1 KB",
    dateUploaded: "2026-05-02",
    uploaded_at: "2026-05-02T00:00:00Z",
    summary: "Document.",
    status: "queued",
    processing_status: "queued",
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
    folder: "Inbox",
    storage_status: "stored",
  }
}

function transcription(overrides: Partial<TranscriptionJobResponse["job"]> = {}): TranscriptionJobResponse {
  return {
    job: {
      transcription_job_id: "transcription:doc-audio",
      id: "transcription:doc-audio",
      matter_id: "matter:smith-abc",
      document_id: "doc:audio",
      provider: "assemblyai",
      provider_mode: "disabled",
      provider_status: "queued",
      status: "review_ready",
      review_status: "needs_review",
      speaker_count: 1,
      segment_count: 1,
      word_count: 5,
      word_search_terms: [],
      keyterms_prompt: [],
      redact_pii: true,
      speech_models: [],
      retryable: false,
      created_at: "2026-05-02T00:00:00Z",
      updated_at: "2026-05-02T00:00:00Z",
      ...overrides,
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
        text: "Tenant reported the issue.",
        redacted_text: "Tenant reported the issue.",
        time_start_ms: 0,
        time_end_ms: 2000,
        confidence: 0.9,
        review_status: "unreviewed",
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

function indexSummary() {
  return {
    matter_id: "matter:smith-abc",
    document_count: 2,
    indexed_documents: 0,
    pending_documents: 1,
    extractable_pending_documents: 0,
    ocr_required_documents: 0,
    transcription_deferred_documents: 1,
    failed_documents: 0,
    duplicate_groups: [],
    recent_ingestion_runs: [],
    recent_index_runs: [],
  }
}
