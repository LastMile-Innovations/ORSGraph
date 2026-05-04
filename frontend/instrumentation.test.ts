import { afterEach, describe, expect, it, vi } from "vitest"
import { formatRequestErrorEvent, onRequestError } from "./instrumentation"

const request = {
  path: "/statutes/ors:90.320?tab=text",
  method: "GET",
  headers: {
    authorization: "Bearer secret",
    cookie: "session=secret",
  },
}

const context = {
  routerKind: "App Router" as const,
  routePath: "/app/statutes/[id]",
  routeType: "render" as const,
  renderSource: "server-rendering" as const,
  revalidateReason: undefined,
  renderType: "dynamic-resume" as const,
}

describe("instrumentation request errors", () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it("formats server request errors without logging headers", () => {
    const error = new Error("database unavailable")
    Object.assign(error, { digest: "digest-123" })

    const event = formatRequestErrorEvent(error, request, context)

    expect(event.error).toEqual({
      name: "Error",
      message: "database unavailable",
      digest: "digest-123",
    })
    expect(event.request).toEqual({
      method: "GET",
      path: "/statutes/ors:90.320?tab=text",
    })
    expect(event.context).toEqual(context)
    expect(JSON.stringify(event)).not.toContain("secret")
  })

  it("awaits the request error logger path", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {})

    await onRequestError(new Error("route failed"), request, context)

    expect(consoleError).toHaveBeenCalledWith(
      "[ORSGraph] server request error",
      expect.objectContaining({
        error: expect.objectContaining({ message: "route failed" }),
        request: { method: "GET", path: "/statutes/ors:90.320?tab=text" },
      }),
    )
  })
})
