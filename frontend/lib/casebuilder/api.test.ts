import { afterEach, describe, expect, it, vi } from "vitest"
import {
  DEFAULT_CASEBUILDER_API_TIMEOUT_MS,
  archiveDocument,
  createFileUpload,
  createMatterIndexJob,
  deleteMatter,
  getCaseBuilderSettingsState,
  getMatterSettingsState,
  getMatterState,
  getMatterSummariesState,
  patchMatterConfig,
  patchDocument,
  putSignedUploadFile,
  resolveCaseBuilderApiTimeoutMs,
  restoreDocument,
} from "./api"

describe("CaseBuilder API timeout", () => {
  it("uses a CaseBuilder-sized timeout instead of inheriting the short generic API default", () => {
    expect(resolveCaseBuilderApiTimeoutMs(undefined, undefined)).toBe(DEFAULT_CASEBUILDER_API_TIMEOUT_MS)
    expect(resolveCaseBuilderApiTimeoutMs(undefined, "5000")).toBe(DEFAULT_CASEBUILDER_API_TIMEOUT_MS)
    expect(resolveCaseBuilderApiTimeoutMs(undefined, "180000")).toBe(180_000)
    expect(resolveCaseBuilderApiTimeoutMs("30000", "5000")).toBe(30_000)
  })
})

describe("CaseBuilder API request loading", () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it("forwards request headers when loading a matter bundle", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(matterBundle()))
    vi.stubGlobal("fetch", fetchMock)

    const state = await getMatterState("intake-test", {
      headers: { cookie: "next-auth.session-token=abc" },
    })

    expect(state.source).toBe("live")
    expect(state.data?.matter_id).toBe("matter:intake-test")
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(fetchMock.mock.calls[0][0]).toBe("/api/ors/matters/matter%3Aintake-test")
    expect((fetchMock.mock.calls[0][1]?.headers as Headers).get("cookie")).toBe("next-auth.session-token=abc")
  })

  it("forwards request headers when loading matter summaries", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse([matterSummary()]))
    vi.stubGlobal("fetch", fetchMock)

    const state = await getMatterSummariesState({
      headers: { cookie: "next-auth.session-token=abc" },
    })

    expect(state.source).toBe("live")
    expect(state.data).toHaveLength(1)
    expect((fetchMock.mock.calls[0][1]?.headers as Headers).get("cookie")).toBe("next-auth.session-token=abc")
  })

  it("loads and normalizes CaseBuilder workspace settings", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      principal: { subject: "user:test", email: "test@example.com", roles: ["casebuilder"] },
      settings: workspaceSettingsResponse({
        default_confidentiality: "sealed",
        auto_index_uploads: false,
      }),
    }))
    vi.stubGlobal("fetch", fetchMock)

    const state = await getCaseBuilderSettingsState({
      headers: { cookie: "next-auth.session-token=abc" },
    })

    expect(fetchMock.mock.calls[0][0]).toBe("/api/ors/casebuilder/settings")
    expect((fetchMock.mock.calls[0][1]?.headers as Headers).get("cookie")).toBe("next-auth.session-token=abc")
    expect(state.data?.principal.email).toBe("test@example.com")
    expect(state.data?.settings.default_confidentiality).toBe("sealed")
    expect(state.data?.settings.auto_index_uploads).toBe(false)
  })

  it("loads matter settings with nullable overrides and effective defaults", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      matter: matterSummary(),
      settings: matterSettingsResponse({
        default_confidentiality: null,
        auto_index_uploads: false,
      }),
      effective: {
        default_confidentiality: "private",
        default_document_type: "exhibit",
        auto_index_uploads: false,
      },
    }))
    vi.stubGlobal("fetch", fetchMock)

    const state = await getMatterSettingsState("matter:intake-test")

    expect(fetchMock.mock.calls[0][0]).toBe("/api/ors/matters/matter%3Aintake-test/settings")
    expect(state.data?.settings.default_confidentiality).toBeNull()
    expect(state.data?.settings.auto_index_uploads).toBe(false)
    expect(state.data?.effective.default_document_type).toBe("exhibit")
  })

  it("patches matter details and settings through the config endpoint", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      matter: matterSummary({ name: "Updated" }),
      settings: matterSettingsResponse({ default_confidentiality: "sealed" }),
      effective: { default_confidentiality: "sealed" },
    }))
    vi.stubGlobal("fetch", fetchMock)

    const state = await patchMatterConfig("matter:intake-test", {
      matter: { name: "Updated" },
      settings: { default_confidentiality: "sealed", auto_index_uploads: null },
    })

    expect(state.data?.matter.name).toBe("Updated")
    expect(fetchMock.mock.calls[0][0]).toBe("/api/ors/matters/matter%3Aintake-test/settings")
    expect(fetchMock.mock.calls[0][1]?.method).toBe("PATCH")
    expect(fetchMock.mock.calls[0][1]?.body).toBe(JSON.stringify({
      matter: { name: "Updated" },
      settings: { default_confidentiality: "sealed", auto_index_uploads: null },
    }))
  })

  it("patches document metadata through the non-destructive document endpoint", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(documentResponse({ library_path: "Evidence/notice.txt" })))
    vi.stubGlobal("fetch", fetchMock)

    const state = await patchDocument("matter:intake-test", "doc:notice", {
      title: "Notice",
      library_path: "Evidence/notice.txt",
    })

    expect(state.data?.library_path).toBe("Evidence/notice.txt")
    expect(fetchMock.mock.calls[0][0]).toBe("/api/ors/matters/matter%3Aintake-test/documents/doc%3Anotice")
    expect(fetchMock.mock.calls[0][1]?.method).toBe("PATCH")
    expect(fetchMock.mock.calls[0][1]?.body).toBe(JSON.stringify({ title: "Notice", library_path: "Evidence/notice.txt" }))
  })

  it("archives and restores documents without calling the destructive delete endpoint", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(documentResponse({ archived_at: "2026-05-04T00:00:00Z" })))
      .mockResolvedValueOnce(jsonResponse(documentResponse({ archived_at: null })))
    vi.stubGlobal("fetch", fetchMock)

    await archiveDocument("matter:intake-test", "doc:notice", { reason: "superseded" })
    await restoreDocument("matter:intake-test", "doc:notice")

    expect(fetchMock.mock.calls[0][0]).toBe("/api/ors/matters/matter%3Aintake-test/documents/doc%3Anotice/archive")
    expect(fetchMock.mock.calls[0][1]?.method).toBe("POST")
    expect(fetchMock.mock.calls[1][0]).toBe("/api/ors/matters/matter%3Aintake-test/documents/doc%3Anotice/restore")
    expect(fetchMock.mock.calls[1][1]?.method).toBe("POST")
  })

  it("hard-deletes a matter through the destructive matter endpoint", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ deleted: true }))
    vi.stubGlobal("fetch", fetchMock)

    const state = await deleteMatter("matter:intake-test")

    expect(state.data?.deleted).toBe(true)
    expect(fetchMock.mock.calls[0][0]).toBe("/api/ors/matters/matter%3Aintake-test")
    expect(fetchMock.mock.calls[0][1]?.method).toBe("DELETE")
  })

  it("creates signed upload intents without the app request timeout", async () => {
    const timeoutSpy = vi.spyOn(globalThis, "setTimeout")
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      upload_id: "upload:doc-upload",
      document_id: "doc:upload",
      method: "PUT",
      url: "https://r2.example/upload",
      expires_at: "999999",
      headers: { "content-type": "text/markdown" },
      document: documentResponse({ document_id: "doc:upload", id: "doc:upload" }),
    }))
    vi.stubGlobal("fetch", fetchMock)

    const state = await createFileUpload("matter:intake-test", {
      filename: "facts.md",
      mime_type: "text/markdown",
      bytes: 12,
      relative_path: "Evidence/facts.md",
      upload_batch_id: "batch:test",
    })

    expect(state.data?.url).toBe("https://r2.example/upload")
    expect(fetchMock.mock.calls[0][0]).toBe("/api/ors/matters/matter%3Aintake-test/files/uploads")
    expect(timeoutSpy).not.toHaveBeenCalled()
  })

  it("uploads file bytes directly to the signed URL, not the CaseBuilder API", async () => {
    const timeoutSpy = vi.spyOn(globalThis, "setTimeout")
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 200, headers: { etag: "abc" } }))
    vi.stubGlobal("fetch", fetchMock)

    const state = await putSignedUploadFile(
      { method: "PUT", url: "https://r2.example/upload", headers: { "content-type": "text/markdown" } },
      new File(["hello"], "facts.md", { type: "text/markdown" }),
    )

    expect(state.data?.etag).toBe("abc")
    expect(fetchMock.mock.calls[0][0]).toBe("https://r2.example/upload")
    expect(fetchMock.mock.calls[0][1]?.body).toBeInstanceOf(File)
    expect(timeoutSpy).not.toHaveBeenCalled()
  })

  it("reports signed upload progress with XMLHttpRequest when requested", async () => {
    const progress = vi.fn()
    const XMLHttpRequestMock = vi.fn(function XMLHttpRequestMock() {
      return new FakeUploadRequest()
    })
    vi.stubGlobal("XMLHttpRequest", XMLHttpRequestMock)

    const state = await putSignedUploadFile(
      { method: "PUT", url: "https://r2.example/upload", headers: { "content-type": "text/markdown" } },
      new File(["0123456789"], "facts.md", { type: "text/markdown" }),
      { onProgress: progress },
    )

    expect(state.data?.etag).toBe("etag-progress")
    expect(XMLHttpRequestMock).toHaveBeenCalledTimes(1)
    const xhr = XMLHttpRequestMock.mock.results[0]?.value as FakeUploadRequest
    expect(xhr.open).toHaveBeenCalledWith("PUT", "https://r2.example/upload", true)
    expect(xhr.setRequestHeader).toHaveBeenCalledWith("content-type", "text/markdown")
    expect(xhr.send).toHaveBeenCalledWith(expect.any(File))
    expect(progress).toHaveBeenCalledWith(expect.objectContaining({ loaded: 5, total: 10 }))
    expect(progress).toHaveBeenLastCalledWith(expect.objectContaining({ loaded: 10, total: 10 }))
  })

  it("starts indexing through the background job endpoint", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(indexJobResponse()))
    vi.stubGlobal("fetch", fetchMock)

    const state = await createMatterIndexJob("matter:intake-test", {
      document_ids: ["doc:upload"],
      upload_batch_id: "batch:test",
    })

    expect(state.data?.index_job_id).toBe("index-job:test")
    expect(fetchMock.mock.calls[0][0]).toBe("/api/ors/matters/matter%3Aintake-test/index/jobs")
    expect(fetchMock.mock.calls[0][1]?.method).toBe("POST")
  })
})

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  })
}

class FakeUploadRequest {
  upload = {
    onprogress: null as ((event: ProgressEvent) => void) | null,
  }
  status = 0
  onload: (() => void) | null = null
  onerror: (() => void) | null = null
  onabort: (() => void) | null = null
  open = vi.fn()
  setRequestHeader = vi.fn()
  send = vi.fn((body: BodyInit | null) => {
    void body
    this.upload.onprogress?.({ loaded: 5, total: 10, lengthComputable: true } as ProgressEvent)
    this.status = 200
    this.onload?.()
  })
  abort = vi.fn(() => {
    this.onabort?.()
  })
  getResponseHeader(name: string) {
    return name.toLowerCase() === "etag" ? "etag-progress" : null
  }
}

function matterSummary(overrides: Record<string, unknown> = {}) {
  return {
    matter_id: "matter:intake-test",
    name: "Intake Test",
    matter_type: "civil",
    status: "intake",
    user_role: "neutral",
    jurisdiction: "Oregon",
    court: "Unassigned",
    case_number: null,
    created_at: "2026-05-03T00:00:00Z",
    updated_at: "2026-05-03T00:00:00Z",
    document_count: 0,
    fact_count: 0,
    evidence_count: 0,
    claim_count: 0,
    draft_count: 0,
    open_task_count: 0,
    next_deadline: null,
    ...overrides,
  }
}

function workspaceSettingsResponse(overrides: Record<string, unknown> = {}) {
  return {
    settings_id: "casebuilder-user-settings:user:test",
    subject: "user:test",
    workspace_label: null,
    display_name: null,
    default_matter_type: "civil",
    default_user_role: "neutral",
    default_jurisdiction: "Oregon",
    default_court: "Unassigned",
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
    created_at: "2026-05-04T00:00:00Z",
    updated_at: "2026-05-04T00:00:00Z",
    ...overrides,
  }
}

function matterSettingsResponse(overrides: Record<string, unknown> = {}) {
  return {
    settings_id: "casebuilder-matter-settings:matter:intake-test",
    matter_id: "matter:intake-test",
    owner_subject: "user:test",
    default_confidentiality: null,
    default_document_type: null,
    auto_index_uploads: null,
    auto_import_complaints: null,
    preserve_folder_paths: null,
    timeline_suggestions_enabled: null,
    ai_timeline_enrichment_enabled: null,
    transcript_redact_pii: null,
    transcript_speaker_labels: null,
    transcript_default_view: null,
    transcript_prompt_preset: null,
    transcript_remove_audio_tags: null,
    export_default_format: null,
    export_include_exhibits: null,
    export_include_qc_report: null,
    created_at: "2026-05-04T00:00:00Z",
    updated_at: "2026-05-04T00:00:00Z",
    ...overrides,
  }
}

function matterBundle() {
  return {
    ...matterSummary(),
    id: "matter:intake-test",
    documents: [],
    parties: [],
    facts: [],
    timeline: [],
    timeline_suggestions: [],
    timeline_agent_runs: [],
    claims: [],
    evidence: [],
    defenses: [],
    deadlines: [],
    tasks: [],
    drafts: [],
    work_products: [],
    fact_check_findings: [],
    citation_check_findings: [],
    chatHistory: [],
    recentThreads: [],
    milestones: [],
  }
}

function documentResponse(overrides: Record<string, unknown> = {}) {
  return {
    document_id: "doc:notice",
    id: "doc:notice",
    matter_id: "matter:intake-test",
    filename: "notice.txt",
    title: "Notice",
    document_type: "notice",
    mime_type: "text/plain",
    pages: 1,
    bytes: 10,
    uploaded_at: "2026-05-04T00:00:00Z",
    source: "user_upload",
    confidentiality: "private",
    processing_status: "queued",
    is_exhibit: false,
    summary: "Uploaded.",
    parties_mentioned: [],
    entities_mentioned: [],
    facts_extracted: 0,
    citations_found: 0,
    contradictions_flagged: 0,
    linked_claim_ids: [],
    folder: "Evidence",
    storage_status: "stored",
    library_path: "Evidence/notice.txt",
    ...overrides,
  }
}

function indexJobResponse() {
  return {
    index_job_id: "index-job:test",
    id: "index-job:test",
    matter_id: "matter:intake-test",
    upload_batch_id: "batch:test",
    document_ids: ["doc:upload"],
    limit: 250,
    status: "queued",
    stage: "queued",
    requested: 1,
    processed: 0,
    skipped: 0,
    failed: 0,
    produced_timeline_suggestions: 0,
    results: [],
    summary: null,
    warnings: [],
    error_code: null,
    error_message: null,
    retryable: true,
    created_at: "2026-05-04T00:00:00Z",
    started_at: null,
    completed_at: null,
  }
}
