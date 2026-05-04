import { afterEach, describe, expect, it, vi } from "vitest"
import {
  DEFAULT_CASEBUILDER_API_TIMEOUT_MS,
  archiveDocument,
  getMatterState,
  getMatterSummariesState,
  patchDocument,
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
})

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  })
}

function matterSummary() {
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
