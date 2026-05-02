import { describe, expect, it } from "vitest"
import {
  authorityCacheControl,
  authorityHotsetObjectPath,
  authorityReadPolicy,
  normalizeAuthoritySearchParams,
} from "./authority-hotset.mjs"

describe("authority hotset helpers", () => {
  it("normalizes query params into a stable cache key order", () => {
    const params = new URLSearchParams("limit=10&q=tenant+repair&utm_source=ad&mode=keyword&q=habitability&_=123")

    expect(normalizeAuthoritySearchParams(params)).toBe("limit=10&mode=keyword&q=habitability&q=tenant%20repair")
  })

  it("builds release-scoped hotset object paths for query-backed reads", () => {
    const params = new URLSearchParams("q=ORS 90.320&authority_family=ORS")

    expect(authorityHotsetObjectPath(["search", "open"], params, "release:2026-05-02")).toMatch(
      /^release%3A2026-05-02\/search\/open\/__query-[a-f0-9]{16}\.json$/,
    )
  })

  it("keeps statute paths inspectable without a query hash", () => {
    expect(authorityHotsetObjectPath(["statutes", "or:ors:90.320", "page"], new URLSearchParams(), "release:x")).toBe(
      "release%3Ax/statutes/or%3Aors%3A90.320/page.json",
    )
  })

  it("allows only public authority reads through the cacheable gateway", () => {
    expect(authorityReadPolicy(["statutes", "or:ors:90.320", "page"]).hotsetEligible).toBe(true)
    expect(authorityReadPolicy(["graph", "neighborhood"], new URLSearchParams("citation=ORS 90.320")).allowed).toBe(true)
    expect(authorityReadPolicy(["search"], new URLSearchParams("q=tenant")).hotsetEligible).toBe(true)
    expect(authorityReadPolicy(["search", "suggest"], new URLSearchParams("q=ten")).cacheable).toBe(false)
    expect(authorityReadPolicy(["casebuilder", "matters"]).allowed).toBe(false)
    expect(authorityReadPolicy(["auth", "me"]).allowed).toBe(false)
  })

  it("uses no-store for non-cacheable authority interactions", () => {
    expect(authorityCacheControl(3600, 86400, false)).toBe("no-store")
    expect(authorityCacheControl(120, 240, true)).toBe("public, max-age=0, s-maxage=120, stale-while-revalidate=240")
  })
})
