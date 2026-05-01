import { describe, expect, it } from "vitest"
import {
  casebuilderHomeHref,
  decodeMatterRouteId,
  encodeMatterId,
  encodeMatterSlug,
  matterClaimsHref,
  matterComplaintHref,
  matterDocumentHref,
  matterDraftHref,
  matterFactsHref,
  matterHref,
  matterWorkProductHref,
  newMatterHref,
  newWorkProductHref,
} from "./routes"

describe("casebuilder route helpers", () => {
  it("keeps canonical matter routes short while decoding back to matter ids", () => {
    expect(casebuilderHomeHref()).toBe("/casebuilder")
    expect(newMatterHref()).toBe("/casebuilder/new")
    expect(matterHref("matter:smith-abc")).toBe("/casebuilder/matters/smith-abc")
    expect(matterHref("matter:smith-abc", "/facts")).toBe("/casebuilder/matters/smith-abc/facts")
    expect(encodeMatterSlug("matter%3Asmith-abc")).toBe("smith-abc")
    expect(encodeMatterId("doc:complaint")).toBe("doc%3Acomplaint")
    expect(decodeMatterRouteId("smith-abc")).toBe("matter:smith-abc")
    expect(decodeMatterRouteId("matter%3Asmith-abc")).toBe("matter:smith-abc")
    expect(decodeMatterRouteId("%E0%A4%A")).toBe("matter:%E0%A4%A")
  })

  it("encodes ids, query params, and hashes for nested workspaces", () => {
    expect(newMatterHref("fight now")).toBe("/casebuilder/new?intent=fight%20now")
    expect(newWorkProductHref("matter:smith-abc")).toBe("/casebuilder/matters/smith-abc/work-products/new")
    expect(newWorkProductHref("matter:smith-abc", "legal memo")).toBe(
      "/casebuilder/matters/smith-abc/work-products/new?type=legal%20memo",
    )
    expect(matterDocumentHref("matter:smith-abc", "doc:complaint", "page 2")).toBe(
      "/casebuilder/matters/smith-abc/documents/doc%3Acomplaint#page%202",
    )
    expect(
      matterWorkProductHref("matter:smith-abc", "work-product:matter:smith-abc:answer", "editor", {
        type: "citation",
        id: "ORS 90.320",
      }),
    ).toBe(
      "/casebuilder/matters/smith-abc/work-products/work-product%3Amatter%3Asmith-abc%3Aanswer/editor?targetType=citation#ORS%2090.320",
    )
    expect(matterWorkProductHref("matter:smith-abc", "memo 1")).toBe(
      "/casebuilder/matters/smith-abc/work-products/memo%201",
    )
  })

  it("builds draft, fact, and claim anchors", () => {
    expect(matterDraftHref("matter:smith-abc")).toBe("/casebuilder/matters/smith-abc/drafts")
    expect(matterDraftHref("matter:smith-abc", "draft:1")).toBe("/casebuilder/matters/smith-abc/drafts/draft%3A1")
    expect(matterFactsHref("matter:smith-abc", "fact:rent ledger")).toBe(
      "/casebuilder/matters/smith-abc/facts#fact%3Arent%20ledger",
    )
    expect(matterClaimsHref("matter:smith-abc")).toBe("/casebuilder/matters/smith-abc/claims")
  })

  it("preserves complaint return targets safely", () => {
    expect(matterComplaintHref("matter:smith-abc")).toBe("/casebuilder/matters/smith-abc/complaint")
    expect(
      matterComplaintHref("matter:smith-abc", "qc", {
        type: "finding",
        id: "warn:1",
        returnTo: "/casebuilder/matters/smith-abc",
      }),
    ).toBe(
      "/casebuilder/matters/smith-abc/complaint/qc?targetType=finding&returnTo=%2Fcasebuilder%2Fmatters%2Fsmith-abc#warn%3A1",
    )
  })
})
