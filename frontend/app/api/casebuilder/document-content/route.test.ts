import { NextRequest } from "next/server"
import { getServerSession } from "next-auth"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { GET, HEAD } from "./route"

vi.mock("next-auth", () => ({
  getServerSession: vi.fn(),
}))

vi.mock("@/lib/auth", () => ({
  authOptions: {},
}))

vi.mock("@/lib/ors-backend-api-url", () => ({
  orsBackendApiBaseUrl: () => "https://api.test/api/v1",
}))

const getServerSessionMock = vi.mocked(getServerSession)
const fetchMock = vi.fn()

function request(method: "GET" | "HEAD", path: string) {
  return new NextRequest(new URL(path, "https://orsgraph.test"), { method })
}

describe("document content route handler", () => {
  beforeEach(() => {
    getServerSessionMock.mockReset()
    getServerSessionMock.mockResolvedValue({ accessToken: "session-token" })
    fetchMock.mockReset()
    globalThis.fetch = fetchMock as unknown as typeof fetch
  })

  it("requires both matterId and documentId", async () => {
    const response = await GET(request("GET", "/api/casebuilder/document-content?matterId=matter-1"))

    expect(response.status).toBe(400)
    expect(await response.json()).toEqual({ error: "matterId and documentId are required" })
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("forwards GET requests with auth and streams the upstream body", async () => {
    fetchMock.mockResolvedValue(
      new Response("document body", {
        status: 200,
        headers: {
          "content-type": "text/plain",
          "content-length": "13",
          "x-upstream-debug": "hidden",
        },
      }),
    )

    const response = await GET(request("GET", "/api/casebuilder/document-content?matterId=matter:1&documentId=doc/2"))

    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.test/api/v1/matters/matter%3A1/documents/doc%2F2/content",
      expect.objectContaining({
        cache: "no-store",
        method: "GET",
      }),
    )
    const headers = fetchMock.mock.calls[0][1].headers as Headers
    expect(headers.get("Authorization")).toBe("Bearer session-token")
    expect(response.status).toBe(200)
    expect(response.headers.get("content-type")).toBe("text/plain")
    expect(response.headers.get("cache-control")).toBe("no-store")
    expect(response.headers.get("x-upstream-debug")).toBeNull()
    expect(await response.text()).toBe("document body")
  })

  it("forwards HEAD requests and returns metadata without a body", async () => {
    fetchMock.mockResolvedValue(
      new Response(null, {
        status: 200,
        headers: {
          "content-type": "application/pdf",
          "content-length": "12345",
        },
      }),
    )

    const response = await HEAD(request("HEAD", "/api/casebuilder/document-content?matterId=matter-1&documentId=doc-2"))

    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.test/api/v1/matters/matter-1/documents/doc-2/content",
      expect.objectContaining({
        cache: "no-store",
        method: "HEAD",
      }),
    )
    expect(response.status).toBe(200)
    expect(response.headers.get("content-type")).toBe("application/pdf")
    expect(response.headers.get("content-length")).toBe("12345")
    expect(await response.text()).toBe("")
  })
})
