import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)

describe("cacheTag policy", () => {
  it("uses authorityCacheTags as the only cacheTag source", () => {
    const cacheTagCallers = sourceFiles
      .filter((file) => /\bcacheTag\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(cacheTagCallers).toEqual(["lib/authority-server-cache.ts"])
  })

  it("does not apply literal cacheTag values directly", () => {
    const literalCacheTags = sourceFiles
      .filter((file) => /\bcacheTag\s*\(\s*["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(literalCacheTags).toEqual([])
  })

  it("invalidates authority caches through release-scoped tags after mutating admin jobs", () => {
    const route = readFileSync(join(frontendDir, "app/api/ors/[...path]/route.ts"), "utf8")

    expect(route).toContain("authorityCacheTags(AUTHORITY_RELEASE_ID)")
    expect(route).toContain('revalidateTag(tag, "max")')
  })
})

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx|mjs)$/.test(entry) && !/\.test\.(ts|tsx)$/.test(entry) ? [path] : []
  })
}
