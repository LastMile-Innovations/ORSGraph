import { afterEach, describe, expect, it, vi } from "vitest"
import { fetchRuntimeStatus, INITIAL_RUNTIME_STATUS } from "./runtime-status"

describe("runtime-status", () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it("starts in a checking state before health data loads", () => {
    expect(INITIAL_RUNTIME_STATUS).toEqual({
      state: "checking",
      api: "unknown",
      neo4j: "unknown",
    })
  })

  it("returns connected when the API and graph are healthy", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ ok: true, neo4j: "connected", version: "test" }),
      }),
    )

    await expect(fetchRuntimeStatus()).resolves.toMatchObject({
      state: "connected",
      api: "connected",
      neo4j: "connected",
      version: "test",
    })
  })

  it("marks reachable APIs with graph issues as degraded", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ ok: true, neo4j: "offline" }),
      }),
    )

    await expect(fetchRuntimeStatus()).resolves.toMatchObject({
      state: "degraded",
      api: "connected",
      neo4j: "offline",
      message: "API reachable; graph storage needs attention.",
    })
  })

  it("normalizes disconnected and unknown graph health states", async () => {
    const fetch = vi
      .fn()
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ ok: true, neo4j: "disconnected" }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ ok: true, neo4j: null }),
      })
    vi.stubGlobal("fetch", fetch)

    await expect(fetchRuntimeStatus()).resolves.toMatchObject({
      state: "degraded",
      api: "connected",
      neo4j: "offline",
    })
    await expect(fetchRuntimeStatus()).resolves.toMatchObject({
      state: "degraded",
      api: "connected",
      neo4j: "unknown",
    })
  })

  it("reports offline when the health request fails", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        status: 503,
        json: async () => ({}),
      }),
    )

    await expect(fetchRuntimeStatus()).resolves.toMatchObject({
      state: "offline",
      api: "offline",
      neo4j: "unknown",
      message: "Health check returned 503",
    })
  })

  it("preserves non-Error health failure messages", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValueOnce("plain failure").mockRejectedValueOnce({}))

    await expect(fetchRuntimeStatus()).resolves.toMatchObject({
      state: "offline",
      message: "plain failure",
    })
    await expect(fetchRuntimeStatus()).resolves.toMatchObject({
      state: "offline",
      message: "Health check failed",
    })
  })
})
