import { NextRequest } from "next/server"
import {
  getRedirectUrl,
  unstable_doesMiddlewareMatch as doesProxyMatch,
} from "next/experimental/testing/server"
import { getToken } from "next-auth/jwt"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { config, proxy } from "./proxy"

vi.mock("next-auth/jwt", () => ({
  getToken: vi.fn(),
}))

const getTokenMock = vi.mocked(getToken)

function request(path: string) {
  return new NextRequest(new URL(path, "https://orsgraph.test"))
}

describe("proxy matcher", () => {
  it("runs for app routes and _next/data, but skips API, image, static, and metadata routes", () => {
    expect(doesProxyMatch({ config, url: "/search" })).toBe(true)
    expect(doesProxyMatch({ config, url: "/_next/data/build-id/search.json" })).toBe(true)

    expect(doesProxyMatch({ config, url: "/api" })).toBe(false)
    expect(doesProxyMatch({ config, url: "/api/health" })).toBe(false)
    expect(doesProxyMatch({ config, url: "/_next/static/chunks/app.js" })).toBe(false)
    expect(doesProxyMatch({ config, url: "/_next/image?url=%2Fmarketing%2Fhero.png&w=828&q=75" })).toBe(false)
    expect(doesProxyMatch({ config, url: "/robots.txt" })).toBe(false)
    expect(doesProxyMatch({ config, url: "/sitemap.xml" })).toBe(false)
  })
})

describe("proxy", () => {
  beforeEach(() => {
    getTokenMock.mockReset()
  })

  it("lets exact public auth and invite routes through without checking a token", async () => {
    const response = await proxy(request("/auth/signin?callbackUrl=%2Fsearch"))

    expect(getRedirectUrl(response)).toBeNull()
    expect(getTokenMock).not.toHaveBeenCalled()
  })

  it("does not treat auth lookalike paths as public", async () => {
    getTokenMock.mockResolvedValue(null)

    const response = await proxy(request("/auth/signin-extra"))

    expect(getRedirectUrl(response)).toBe("https://orsgraph.test/auth/signin?callbackUrl=%2Fauth%2Fsignin-extra")
  })

  it("canonicalizes legacy matter paths before auth checks and preserves the query string", async () => {
    const response = await proxy(request("/matters/example?tab=facts"))

    expect(getRedirectUrl(response)).toBe("https://orsgraph.test/casebuilder/matters/example?tab=facts")
    expect(getTokenMock).not.toHaveBeenCalled()
  })

  it("redirects anonymous protected requests to sign-in with the original callback path", async () => {
    getTokenMock.mockResolvedValue(null)

    const response = await proxy(request("/search?q=ors"))

    expect(getRedirectUrl(response)).toBe("https://orsgraph.test/auth/signin?callbackUrl=%2Fsearch%3Fq%3Dors")
  })

  it("routes non-active non-admin users to the pending page", async () => {
    getTokenMock.mockResolvedValue({ accessStatus: "pending", isAdmin: false })

    const response = await proxy(request("/search?q=ors"))

    expect(getRedirectUrl(response)).toBe("https://orsgraph.test/auth/pending?callbackUrl=%2Fsearch%3Fq%3Dors")
  })

  it("allows bootstrap admins through even before regular access activation", async () => {
    getTokenMock.mockResolvedValue({ accessStatus: "pending", isAdmin: true })

    const response = await proxy(request("/admin/auth"))

    expect(getRedirectUrl(response)).toBeNull()
  })
})
