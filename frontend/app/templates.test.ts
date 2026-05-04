import { existsSync, readFileSync } from "node:fs"
import { join } from "node:path"
import { describe, expect, it } from "vitest"

const appDir = join(process.cwd(), "app")

const statefulDynamicTemplates = [
  "matters/[id]/documents/[docId]/template.tsx",
  "matters/[id]/drafts/[draftId]/template.tsx",
  "matters/[id]/work-products/[workProductId]/template.tsx",
  "casebuilder/matters/[id]/documents/[docId]/template.tsx",
  "casebuilder/matters/[id]/drafts/[draftId]/template.tsx",
  "casebuilder/matters/[id]/work-products/[workProductId]/template.tsx",
]

describe("template convention policy", () => {
  it("keeps reset templates on stateful dynamic editor routes", () => {
    expect(statefulDynamicTemplates.filter((file) => !existsSync(join(appDir, file)))).toEqual([])
  })

  it("keeps templates as server wrappers instead of hidden client state", () => {
    const clientTemplates = statefulDynamicTemplates.filter((file) =>
      readFileSync(join(appDir, file), "utf8").includes('"use client"'),
    )

    expect(clientTemplates).toEqual([])
  })
})
