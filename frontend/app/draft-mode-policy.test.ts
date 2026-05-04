import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))

describe("draftMode() policy", () => {
  it("keeps draftMode() out of Client Components", () => {
    const clientDraftModeUsage = clientFiles
      .filter((file) => /\bdraftMode\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(clientDraftModeUsage).toEqual([])
  })

  it("awaits draftMode() instead of using the deprecated synchronous form", () => {
    const syncDraftModeCalls = sourceFiles
      .filter((file) => hasSyncDraftModeCall(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(syncDraftModeCalls).toEqual([])
  })

  it("enables or disables Draft Mode only from Route Handlers", () => {
    const invalidDraftModeMutations = sourceFiles
      .filter((file) => basename(file) !== "route.ts")
      .filter((file) => hasDraftModeMutation(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(invalidDraftModeMutations).toEqual([])
  })

  it("does not write the Draft Mode bypass cookie directly", () => {
    const directBypassCookieUsage = sourceFiles
      .filter((file) => /__prerender_bypass/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(directBypassCookieUsage).toEqual([])
  })
})

function hasSyncDraftModeCall(source: string) {
  return Array.from(source.matchAll(/\bdraftMode\s*\(\s*\)/g)).some((match) => {
    const index = match.index ?? 0
    const prefix = source.slice(Math.max(0, index - 16), index)
    return !/\bawait\s*$/.test(prefix)
  })
}

function hasDraftModeMutation(source: string) {
  if (/\(\s*await\s+draftMode\s*\(\s*\)\s*\)\s*\.\s*(?:enable|disable)\s*\(/.test(source)) return true

  const draftStores = Array.from(source.matchAll(/\b(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=\s*await\s+draftMode\s*\(\s*\)/g))
    .map((match) => match[1])
    .filter(Boolean)

  return draftStores.some((store) => new RegExp(`\\b${escapeRegExp(store)}\\s*\\.\\s*(?:enable|disable)\\s*\\(`).test(source))
}

function hasUseClientDirective(source: string) {
  return /^\s*["']use client["']/.test(source)
}

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx)$/.test(entry) && !/(\.d|\.test)\.tsx?$/.test(entry) ? [path] : []
  })
}
