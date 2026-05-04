import { readFileSync } from "node:fs"
import { join } from "node:path"

import { afterEach, describe, expect, it, vi } from "vitest"

vi.mock("server-only", () => ({}))
vi.mock("next/cache", () => ({
  cacheLife: vi.fn(),
  cacheTag: vi.fn(),
}))

import {
  authorityServerCacheMode,
  getCachedSearchWithParamsState,
  getCachedStatuteIndexState,
  getCachedStatutePageDataState,
} from "./authority-server-cache"

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as unknown as Response
}

const statutePageResponse = {
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
  citation_counts: { outbound: 0, inbound: 0 },
  semantic_counts: { obligations: 0, exceptions: 0, deadlines: 0, penalties: 0, definitions: 0 },
  source_notes: [],
  provisions: [],
}

describe("authority server cache wrappers", () => {
  afterEach(() => {
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
  })

  it("does not mask statute index recovery with a cached offline state", async () => {
    vi.spyOn(console, "info").mockImplementation(() => undefined)
    const fetchMock = vi
      .fn()
      .mockRejectedValueOnce(new Error("fetch failed"))
      .mockResolvedValueOnce(jsonResponse({
        items: [{
          canonical_id: "or:ors:3.130",
          citation: "ORS 3.130",
          title: "Powers of court",
          chapter: "3",
          status: "active",
          edition_year: 2025,
        }],
        total: 1,
        limit: 60,
        offset: 0,
      }))
    vi.stubGlobal("fetch", fetchMock)

    const failed = await getCachedStatuteIndexState()
    const recovered = await getCachedStatuteIndexState()

    expect(failed.source).toBe("offline")
    expect(recovered.source).toBe("live")
    expect(recovered.data.items[0].canonical_id).toBe("or:ors:3.130")
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it("does not cache empty statute detail returned during a transient miss", async () => {
    vi.spyOn(console, "info").mockImplementation(() => undefined)
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({ error: "Statute not found: or:ors:3.130" }, 404))
      .mockResolvedValueOnce(jsonResponse(statutePageResponse))
    vi.stubGlobal("fetch", fetchMock)

    const empty = await getCachedStatutePageDataState("or:ors:3.130")
    const recovered = await getCachedStatutePageDataState("or:ors:3.130")

    expect(empty.source).toBe("empty")
    expect(empty.data).toBeNull()
    expect(recovered.source).toBe("live")
    expect(recovered.data?.identity.canonical_id).toBe("or:ors:3.130")
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it("does not mask search recovery with a cached error state", async () => {
    vi.spyOn(console, "info").mockImplementation(() => undefined)
    const fetchMock = vi
      .fn()
      .mockRejectedValueOnce(new Error("network split"))
      .mockResolvedValueOnce(jsonResponse({
        query: "tenant",
        results: [],
        total: 0,
        limit: 20,
        offset: 0,
        facets: {},
        analysis: { timings: { total_ms: 1 } },
        warnings: [],
      }))
    vi.stubGlobal("fetch", fetchMock)

    const failed = await getCachedSearchWithParamsState({ q: "tenant" })
    const recovered = await getCachedSearchWithParamsState({ q: "tenant" })

    expect(failed.source).toBe("offline")
    expect(recovered.source).toBe("live")
    expect(recovered.data?.query).toBe("tenant")
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it("documents that authority reads use the local cache plus hotset layer, not remote cache handlers", () => {
    const source = readFileSync(join(process.cwd(), "lib/authority-server-cache.ts"), "utf8")

    expect(authorityServerCacheMode()).toBe("memory-with-authority-hotset")
    expect(source).not.toContain("use cache: remote")
  })
})
