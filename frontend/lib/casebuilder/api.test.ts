import { afterEach, describe, expect, it, vi } from "vitest"
import { getMatterState, getMatterSummariesState } from "./api"

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
