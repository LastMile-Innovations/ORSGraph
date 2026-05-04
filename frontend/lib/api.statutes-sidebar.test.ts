import { afterEach, describe, expect, it, vi } from "vitest"
import { getSidebarState, getStatuteIndexState, getStatutePageDataState } from "./api"

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as unknown as Response
}

describe("statute and sidebar API adapters", () => {
  afterEach(() => {
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
  })

  it("maps the live statute index response without using bundled statute data", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      items: [{
        canonical_id: "or:ors:3.130",
        citation: "ORS 3.130",
        title: "Powers of court",
        chapter: "3",
        status: "active",
        edition_year: 2025,
      }],
      total: 1,
      limit: 5,
      offset: 10,
    }))
    vi.stubGlobal("fetch", fetchMock)

    const state = await getStatuteIndexState({
      q: "court",
      chapter: "3",
      status: "active",
      limit: 5,
      offset: 10,
    })

    expect(state.source).toBe("live")
    expect(state.data).toMatchObject({
      total: 1,
      limit: 5,
      offset: 10,
      items: [{
        canonical_id: "or:ors:3.130",
        citation: "ORS 3.130",
        title: "Powers of court",
        edition: 2025,
      }],
    })
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(String(fetchMock.mock.calls[0][0])).toContain("/statutes?limit=5&offset=10&q=court&chapter=3&status=active")
  })

  it("returns an explicit empty error state instead of mock statutes when the index API fails", async () => {
    vi.spyOn(console, "info").mockImplementation(() => undefined)
    const fetchMock = vi.fn().mockRejectedValue(new Error("fetch failed"))
    vi.stubGlobal("fetch", fetchMock)

    const state = await getStatuteIndexState({ limit: 30, offset: 0 })

    expect(state.source).toBe("offline")
    expect(state.data).toEqual({ items: [], total: 0, limit: 30, offset: 0 })
    expect(state.error).toBe("fetch failed")
  })

  it("loads statute detail from the page endpoint only and preserves live DTO fields", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      identity: {
        canonical_id: "or:ors:3.130",
        citation: "ORS 3.130",
        title: "Powers of court",
        chapter: "3",
        status: "active",
      },
      current_version: {
        version_id: "or:ors:3.130:v2025",
        effective_date: "2025-01-01",
        end_date: null,
        is_current: true,
        text: "The court may exercise powers provided by law.",
      },
      source_document: {
        source_id: "source:ors:2025:3",
        url: "https://example.test/ors/3.130",
        edition_year: 2025,
      },
      provision_count: 1,
      citation_counts: { outbound: 2, inbound: 1 },
      semantic_counts: { obligations: 0, exceptions: 0, deadlines: 0, penalties: 0, definitions: 0 },
      source_notes: [],
      provisions: [{
        provision_id: "or:ors:3.130:p1",
        display_citation: "ORS 3.130(1)",
        local_path: ["1"],
        depth: 1,
        text: "The court may exercise powers provided by law.",
        children: [],
      }],
    }))
    vi.stubGlobal("fetch", fetchMock)

    const state = await getStatutePageDataState("or:ors:3.130")

    expect(state.source).toBe("live")
    expect(state.data?.identity).toMatchObject({
      canonical_id: "or:ors:3.130",
      citation: "ORS 3.130",
      title: "Powers of court",
      edition: 2025,
    })
    expect(state.data?.summary_counts).toMatchObject({
      provision_count: 1,
      citation_counts: { outbound: 2, inbound: 1 },
    })
    expect(state.data?.provisions[0]).toMatchObject({
      provision_id: "or:ors:3.130:p1",
      display_citation: "ORS 3.130(1)",
    })
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(String(fetchMock.mock.calls[0][0])).toContain("/statutes/or%3Aors%3A3.130/page")
  })

  it("does not fall back to legacy or mock statute detail data when the page endpoint fails", async () => {
    vi.spyOn(console, "info").mockImplementation(() => undefined)
    const fetchMock = vi.fn().mockRejectedValue(new Error("fetch failed"))
    vi.stubGlobal("fetch", fetchMock)

    const state = await getStatutePageDataState("or:ors:3.130")

    expect(state.source).toBe("offline")
    expect(state.data).toBeNull()
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it("treats live statute 404s as empty rather than substituting bundled records", async () => {
    vi.spyOn(console, "info").mockImplementation(() => undefined)
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({ error: "Statute not found: missing" }, 404)))

    const state = await getStatutePageDataState("missing")

    expect(state.source).toBe("empty")
    expect(state.data).toBeNull()
    expect(state.error).toBe("Statute not found: missing")
  })

  it("maps the live sidebar response and does not synthesize saved statutes", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      corpus: {
        jurisdiction: "Oregon",
        corpus: "ORS",
        edition_year: 2025,
        total_statutes: 1,
        chapters: [{
          chapter: "3",
          label: "Chapter 3",
          count: 1,
          items: [{
            canonical_id: "or:ors:3.130",
            citation: "ORS 3.130",
            title: null,
            chapter: "3",
            status: "active",
            edition_year: 2025,
          }],
        }],
      },
      saved_searches: [],
      saved_statutes: [],
      recent_statutes: [],
      active_matter: null,
      updated_at: "2026-05-01T00:00:00Z",
    }))
    vi.stubGlobal("fetch", fetchMock)

    const state = await getSidebarState()

    expect(state.source).toBe("live")
    expect(state.data?.corpus.chapters[0].items[0]).toMatchObject({
      canonical_id: "or:ors:3.130",
      title: "ORS 3.130",
    })
    expect(state.data?.saved_statutes).toEqual([])
    expect(state.data?.recent_statutes).toEqual([])
  })

  it("forwards request headers when loading sidebar state", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      corpus: {
        jurisdiction: "Oregon",
        corpus: "ORS",
        edition_year: 2025,
        total_statutes: 0,
        chapters: [],
      },
      saved_searches: [],
      saved_statutes: [],
      recent_statutes: [],
      active_matter: null,
      updated_at: "2026-05-01T00:00:00Z",
    }))
    vi.stubGlobal("fetch", fetchMock)

    const state = await getSidebarState({ headers: { cookie: "next-auth.session-token=abc" } })

    expect(state.source).toBe("live")
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(fetchMock.mock.calls[0][1]?.headers).toMatchObject({
      cookie: "next-auth.session-token=abc",
    })
  })

  it("returns no sidebar data when the live sidebar API is unavailable", async () => {
    vi.spyOn(console, "info").mockImplementation(() => undefined)
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("fetch failed")))

    const state = await getSidebarState()

    expect(state.source).toBe("offline")
    expect(state.data).toBeNull()
    expect(state.error).toBe("fetch failed")
  })
})
