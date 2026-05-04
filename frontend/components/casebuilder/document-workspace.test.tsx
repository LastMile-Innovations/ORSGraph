import { fireEvent, render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type {
  CaseBuilderEffectiveSettings,
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

  it("uses effective matter settings for transcript setup", async () => {
    const user = userEvent.setup()
    createTranscription.mockResolvedValue({ data: transcription })
    render(
      <DocumentWorkspace
        matter={matter}
        workspace={workspace}
        settings={effectiveSettings({
          transcript_default_view: "raw",
          transcript_redact_pii: false,
          transcript_speaker_labels: false,
          transcript_prompt_preset: "legal",
          transcript_remove_audio_tags: false,
        })}
      />,
    )

    expect(screen.getByRole("button", { name: "Review raw" })).toBeInTheDocument()
    await user.click(screen.getByRole("button", { name: "Transcribe" }))

    await waitFor(() => {
      expect(createTranscription).toHaveBeenCalledWith(
        "matter:smith-abc",
        "doc:audio",
        expect.objectContaining({
          redact_pii: false,
          speaker_labels: false,
          prompt_preset: "legal",
          remove_audio_tags: null,
        }),
      )
    })
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

  it("shows Markdown graph provenance and jumps to source ranges", async () => {
    const user = userEvent.setup()
    render(<DocumentWorkspace matter={matter} workspace={markdownGraphWorkspace()} />)

    await user.click(screen.getByRole("tab", { name: /markdown graph/i }))

    expect(screen.getByText("Markdown Graph")).toBeInTheDocument()
    expect(screen.getByText("Debra Paynter")).toBeInTheDocument()
    expect(screen.getAllByText("Tenant paid rent").length).toBeGreaterThan(0)

    const outlineButton = screen
      .getAllByRole("button")
      .find((button) => button.textContent?.includes("Facts") && button.textContent?.includes("heading"))
    expect(outlineButton).toBeTruthy()
    await user.click(outlineButton as HTMLButtonElement)

    const editor = screen.getByRole("textbox") as HTMLTextAreaElement
    await waitFor(() => {
      expect(editor.selectionStart).toBe(0)
      expect(editor.selectionEnd).toBe(7)
    })
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
    embedding_coverage: emptyEmbeddingCoverage(),
    proposed_facts: [],
    timeline_suggestions: [],
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
    embedding_coverage: emptyEmbeddingCoverage(),
    proposed_facts: [],
    timeline_suggestions: [],
    transcriptions: [],
    docx_manifest: null,
    text_content: null,
    content_url: "/api/casebuilder/document-content?matterId=matter%3Asmith-abc&documentId=doc%3Alease",
    warnings: [],
  }
}

function markdownGraphWorkspace(): DocumentWorkspaceState {
  const text = "# Facts\n\nDebra Paynter paid $1,250 on April 1, 2026.\n"
  const document: CaseDocument = {
    ...docxDocument(),
    id: "doc:markdown",
    document_id: "doc:markdown",
    title: "Facts",
    filename: "facts.md",
    mime_type: "text/markdown",
    processing_status: "processed",
    status: "processed",
    storage_status: "stored",
    extracted_text: text,
  }
  return {
    matter_id: "matter:smith-abc",
    document,
    current_version: null,
    capabilities: [
      { capability: "view", enabled: true, mode: "markdown_source" },
      { capability: "edit", enabled: true, mode: "markdown_source" },
      { capability: "annotate", enabled: true, mode: "graph_sidecar" },
      { capability: "extract", enabled: true, mode: "markdown_index" },
      { capability: "promote", enabled: true, mode: "work_product_ast" },
    ],
    annotations: [],
    source_spans: [
      {
        source_span_id: "span:markdown:1",
        id: "span:markdown:1",
        matter_id: "matter:smith-abc",
        document_id: "doc:markdown",
        byte_start: 9,
        byte_end: text.length,
        char_start: 9,
        char_end: text.length,
        quote: "Debra Paynter paid $1,250 on April 1, 2026.",
        extraction_method: "markdown_index",
        confidence: 1,
        review_status: "unreviewed",
      },
    ],
    markdown_ast_document: {
      markdown_ast_document_id: "markdown-ast:doc_markdown:a",
      id: "markdown-ast:doc_markdown:a",
      matter_id: "matter:smith-abc",
      document_id: "doc:markdown",
      parser_id: "pulldown-cmark",
      parser_version: "pulldown-cmark-0.13",
      source_sha256: "sha256:text",
      root_node_id: "markdown-node:doc_markdown:root",
      node_count: 2,
      status: "indexed",
      created_at: "2026-05-02T00:00:00Z",
    },
    markdown_ast_nodes: [
      {
        markdown_ast_node_id: "markdown-node:doc_markdown:root",
        id: "markdown-node:doc_markdown:root",
        matter_id: "matter:smith-abc",
        document_id: "doc:markdown",
        markdown_ast_document_id: "markdown-ast:doc_markdown:a",
        node_kind: "document",
        tag: "root",
        ordinal: 0,
        depth: 0,
        byte_start: 0,
        byte_end: text.length,
        char_start: 0,
        char_end: text.length,
        source_span_ids: ["span:markdown:1"],
        text_chunk_ids: ["chunk:markdown:1"],
        evidence_span_ids: [],
        search_index_record_ids: [],
        review_status: "unreviewed",
      },
      {
        markdown_ast_node_id: "markdown-node:doc_markdown:heading",
        id: "markdown-node:doc_markdown:heading",
        matter_id: "matter:smith-abc",
        document_id: "doc:markdown",
        markdown_ast_document_id: "markdown-ast:doc_markdown:a",
        parent_ast_node_id: "markdown-node:doc_markdown:root",
        node_kind: "heading",
        tag: "Heading(H1)",
        ordinal: 1,
        depth: 1,
        structure_path: "Facts",
        text_excerpt: "# Facts",
        byte_start: 0,
        byte_end: 7,
        char_start: 0,
        char_end: 7,
        source_span_ids: [],
        text_chunk_ids: [],
        evidence_span_ids: [],
        search_index_record_ids: [],
        review_status: "unreviewed",
      },
    ],
    markdown_semantic_units: [
      {
        semantic_unit_id: "markdown-unit:doc_markdown:facts",
        id: "markdown-unit:doc_markdown:facts",
        matter_id: "matter:smith-abc",
        document_id: "doc:markdown",
        markdown_ast_document_id: "markdown-ast:doc_markdown:a",
        unit_kind: "heading",
        semantic_role: "section_heading",
        canonical_label: "Facts",
        normalized_key: "facts",
        semantic_fingerprint: "fingerprint:facts",
        markdown_ast_node_ids: ["markdown-node:doc_markdown:heading"],
        entity_mention_ids: [],
        citation_texts: [],
        date_texts: [],
        money_texts: [],
        occurrence_count: 1,
        evidence_span_count: 0,
        text_chunk_count: 0,
        source_span_count: 0,
        review_status: "unreviewed",
        created_at: "2026-05-02T00:00:00Z",
        updated_at: "2026-05-02T00:00:00Z",
      },
    ],
    text_chunks: [
      {
        text_chunk_id: "chunk:markdown:1",
        id: "chunk:markdown:1",
        matter_id: "matter:smith-abc",
        document_id: "doc:markdown",
        ordinal: 1,
        page: 1,
        text_hash: "sha256:chunk",
        text_excerpt: "Debra Paynter paid $1,250 on April 1, 2026.",
        token_count: 12,
        structure_path: "Facts",
        markdown_ast_node_ids: ["markdown-node:doc_markdown:root"],
        byte_start: 9,
        byte_end: text.length,
        char_start: 9,
        char_end: text.length,
        status: "indexed",
      },
    ],
    evidence_spans: [],
    entity_mentions: [
      {
        entity_mention_id: "entity-mention:markdown:1",
        id: "entity-mention:markdown:1",
        matter_id: "matter:smith-abc",
        document_id: "doc:markdown",
        text_chunk_id: "chunk:markdown:1",
        source_span_id: "span:markdown:1",
        entity_id: "case-entity:matter_smith_abc:debra",
        markdown_ast_node_ids: ["markdown-node:doc_markdown:root"],
        mention_text: "Debra Paynter",
        entity_type: "party",
        confidence: 0.8,
        review_status: "unreviewed",
      },
    ],
    entities: [
      {
        entity_id: "case-entity:matter_smith_abc:debra",
        id: "case-entity:matter_smith_abc:debra",
        matter_id: "matter:smith-abc",
        entity_type: "party",
        canonical_name: "Debra Paynter",
        normalized_key: "debra paynter",
        confidence: 0.8,
        review_status: "unreviewed",
        mention_ids: ["entity-mention:markdown:1"],
        party_match_ids: [],
        created_at: "2026-05-02T00:00:00Z",
        updated_at: "2026-05-02T00:00:00Z",
      },
    ],
    search_index_records: [],
    embedding_runs: [
      {
        embedding_run_id: "embedding-run:markdown:1",
        id: "embedding-run:markdown:1",
        matter_id: "matter:smith-abc",
        document_id: "doc:markdown",
        document_version_id: "version:markdown:1",
        index_run_id: "index:markdown:1",
        model: "voyage-4-large",
        profile: "casebuilder_markdown_v1",
        dimension: 1024,
        vector_index_name: "casebuilder_markdown_embedding_1024",
        status: "completed",
        stage: "completed",
        target_count: 2,
        embedded_count: 2,
        skipped_count: 0,
        stale_count: 0,
        produced_embedding_record_ids: ["embedding-record:chunk:1", "embedding-record:unit:1"],
        warnings: [],
        retryable: false,
        started_at: "2026-05-02T00:00:00Z",
        completed_at: "2026-05-02T00:01:00Z",
      },
    ],
    embedding_records: [
      {
        embedding_record_id: "embedding-record:chunk:1",
        id: "embedding-record:chunk:1",
        matter_id: "matter:smith-abc",
        document_id: "doc:markdown",
        document_version_id: "version:markdown:1",
        index_run_id: "index:markdown:1",
        embedding_run_id: "embedding-run:markdown:1",
        target_kind: "text_chunk",
        target_id: "chunk:markdown:1",
        target_label: "Chunk 1",
        model: "voyage-4-large",
        profile: "casebuilder_markdown_v1",
        dimension: 1024,
        vector_index_name: "casebuilder_markdown_embedding_1024",
        input_hash: "sha256:embedding-input",
        source_text_hash: "sha256:text",
        chunker_version: "casebuilder-semantic-chunker-v2",
        graph_schema_version: "casebuilder-markdown-graph-v2",
        embedding_strategy: "direct",
        embedding_input_type: "document",
        embedding_output_dtype: "float",
        status: "embedded",
        stale: false,
        review_status: "system",
        text_excerpt: "Debra Paynter paid $1,250 on April 1, 2026.",
        source_span_ids: ["span:markdown:1"],
        text_chunk_ids: ["chunk:markdown:1"],
        markdown_ast_node_ids: ["markdown-node:doc_markdown:root"],
        markdown_semantic_unit_ids: [],
        centroid_source_record_ids: [],
        created_at: "2026-05-02T00:00:00Z",
        embedded_at: "2026-05-02T00:01:00Z",
      },
      {
        embedding_record_id: "embedding-record:unit:1",
        id: "embedding-record:unit:1",
        matter_id: "matter:smith-abc",
        document_id: "doc:markdown",
        document_version_id: "version:markdown:1",
        index_run_id: "index:markdown:1",
        embedding_run_id: "embedding-run:markdown:1",
        target_kind: "markdown_semantic_unit",
        target_id: "markdown-unit:doc_markdown:facts",
        target_label: "Facts",
        model: "voyage-4-large",
        profile: "casebuilder_markdown_v1",
        dimension: 1024,
        vector_index_name: "casebuilder_markdown_embedding_1024",
        input_hash: "sha256:unit-input",
        source_text_hash: "sha256:text",
        chunker_version: "casebuilder-semantic-chunker-v2",
        graph_schema_version: "casebuilder-markdown-graph-v2",
        embedding_strategy: "direct",
        embedding_input_type: "document",
        embedding_output_dtype: "float",
        status: "embedded",
        stale: false,
        review_status: "system",
        text_excerpt: "Facts",
        source_span_ids: [],
        text_chunk_ids: [],
        markdown_ast_node_ids: ["markdown-node:doc_markdown:heading"],
        markdown_semantic_unit_ids: ["markdown-unit:doc_markdown:facts"],
        centroid_source_record_ids: [],
        created_at: "2026-05-02T00:00:00Z",
        embedded_at: "2026-05-02T00:01:00Z",
      },
    ],
    embedding_coverage: {
      enabled: true,
      model: "voyage-4-large",
      profile: "casebuilder_markdown_v1",
      dimension: 1024,
      vector_index_name: "casebuilder_markdown_embedding_1024",
      target_count: 2,
      embedded_count: 2,
      current_count: 2,
      stale_count: 0,
      skipped_count: 0,
      failed_count: 0,
      full_file_embedded: false,
      chunk_embedded: 1,
      semantic_unit_embedded: 1,
    },
    proposed_facts: [
      {
        id: "fact:markdown:1",
        fact_id: "fact:markdown:1",
        statement: "Tenant paid rent",
        text: "Tenant paid rent",
        status: "proposed",
        confidence: 0.86,
        disputed: false,
        tags: [],
        sourceDocumentIds: ["doc:markdown"],
        citations: [],
        markdown_ast_node_ids: ["markdown-node:doc_markdown:root"],
      },
    ],
    timeline_suggestions: [
      {
        suggestion_id: "timeline-suggestion:markdown:1",
        id: "timeline-suggestion:markdown:1",
        matter_id: "matter:smith-abc",
        date: "2026-04-01",
        date_text: "April 1, 2026",
        date_confidence: 0.9,
        title: "Tenant paid rent",
        kind: "payment",
        source_type: "document_index",
        source_document_id: "doc:markdown",
        source_span_ids: ["span:markdown:1"],
        text_chunk_ids: ["chunk:markdown:1"],
        markdown_ast_node_ids: ["markdown-node:doc_markdown:root"],
        linked_fact_ids: ["fact:markdown:1"],
        linked_claim_ids: [],
        status: "suggested",
        warnings: [],
        created_at: "2026-05-02T00:00:00Z",
        updated_at: "2026-05-02T00:00:00Z",
      },
    ],
    transcriptions: [],
    docx_manifest: null,
    text_content: text,
    content_url: null,
    warnings: [],
  }
}

function emptyEmbeddingCoverage() {
  return {
    enabled: false,
    model: null,
    profile: null,
    dimension: null,
    vector_index_name: null,
    target_count: 0,
    embedded_count: 0,
    current_count: 0,
    stale_count: 0,
    skipped_count: 0,
    failed_count: 0,
    full_file_embedded: false,
    chunk_embedded: 0,
    semantic_unit_embedded: 0,
  }
}

function effectiveSettings(overrides: Partial<CaseBuilderEffectiveSettings> = {}): CaseBuilderEffectiveSettings {
  return {
    default_confidentiality: "private",
    default_document_type: "other",
    auto_index_uploads: true,
    auto_import_complaints: true,
    preserve_folder_paths: true,
    timeline_suggestions_enabled: true,
    ai_timeline_enrichment_enabled: true,
    transcript_redact_pii: true,
    transcript_speaker_labels: true,
    transcript_default_view: "redacted",
    transcript_prompt_preset: "unclear",
    transcript_remove_audio_tags: true,
    export_default_format: "pdf",
    export_include_exhibits: true,
    export_include_qc_report: true,
    ...overrides,
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
