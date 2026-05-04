import { describe, expect, it } from "vitest"
import { safeCallbackHref, toSafeInternalHref } from "./navigation-safety"

describe("navigation safety", () => {
  it("keeps root-relative app hrefs", () => {
    expect(toSafeInternalHref("/dashboard")).toBe("/dashboard")
    expect(toSafeInternalHref("/search?q=ORS+90.320#result")).toBe("/search?q=ORS+90.320#result")
  })

  it("rejects external, protocol-relative, script, blank, and control-character hrefs", () => {
    expect(toSafeInternalHref("https://example.com/dashboard")).toBeNull()
    expect(toSafeInternalHref("//example.com/dashboard")).toBeNull()
    expect(toSafeInternalHref("javascript:alert(1)")).toBeNull()
    expect(toSafeInternalHref("")).toBeNull()
    expect(toSafeInternalHref("/search\n?q=x")).toBeNull()
  })

  it("falls back callback URLs to onboarding", () => {
    expect(safeCallbackHref("/matters")).toBe("/matters")
    expect(safeCallbackHref("https://example.com")).toBe("/onboarding")
    expect(safeCallbackHref(null, "/dashboard")).toBe("/dashboard")
  })
})
