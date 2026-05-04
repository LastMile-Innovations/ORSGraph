import { afterEach, describe, expect, it, vi } from "vitest"
import {
  formatClientErrorEvent,
  initializeClientInstrumentation,
  onRouterTransitionStart,
  sanitizeInstrumentationUrl,
} from "./instrumentation-client"

describe("client instrumentation", () => {
  afterEach(() => {
    window.__ORSGraphClientInstrumentationInitialized = false
    vi.restoreAllMocks()
  })

  it("sanitizes navigation URLs before recording them", () => {
    expect(sanitizeInstrumentationUrl("/search?q=private#result")).toBe("/search")
    expect(sanitizeInstrumentationUrl("https://example.test/matters/123?token=secret")).toBe("/matters/123")
  })

  it("formats client errors without throwing on unknown values", () => {
    expect(formatClientErrorEvent("window_error", new Error("hydration failed"))).toMatchObject({
      category: "error",
      name: "window_error",
      detail: {
        name: "Error",
        message: "hydration failed",
      },
    })

    expect(formatClientErrorEvent("unhandled_rejection", { unexpected: true })).toMatchObject({
      category: "error",
      detail: {
        name: "UnknownError",
        message: "Client instrumentation captured a non-Error value.",
      },
    })
  })

  it("marks initialization once and emits sanitized navigation events", () => {
    const mark = vi.spyOn(performance, "mark").mockImplementation(() => undefined as unknown as PerformanceMark)
    const events: CustomEvent[] = []
    const listener = (event: Event) => events.push(event as CustomEvent)

    window.addEventListener("orsgraph:client-instrumentation", listener)
    initializeClientInstrumentation()
    initializeClientInstrumentation()
    onRouterTransitionStart("/sources?id=secret#top", "push")
    window.removeEventListener("orsgraph:client-instrumentation", listener)

    expect(mark).toHaveBeenCalledWith("orsgraph-client-init")
    expect(mark.mock.calls.filter(([name]) => name === "orsgraph-client-init")).toHaveLength(1)
    expect(events).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          detail: expect.objectContaining({
            category: "navigation",
            name: "router_transition_start",
            detail: {
              path: "/sources",
              navigationType: "push",
            },
          }),
        }),
      ]),
    )
  })
})
