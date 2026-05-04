import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const appSourceFiles = sourceFiles.filter((file) => !file.includes(`${join("frontend", "scripts")}/`))
const forceCacheAllowlist = new Set([
  "app/api/authority/[...path]/route.ts",
  "lib/api.ts",
])

describe("fetch cache policy", () => {
  it("sets explicit cache intent on application fetch calls", () => {
    const implicitFetches = appSourceFiles
      .flatMap((file) => fetchCalls(file))
      .filter((call) => !hasExplicitCacheIntent(call.source, call.fileSource))
      .map((call) => `${relative(frontendDir, call.file)}:${call.line}`)

    expect(implicitFetches).toEqual([])
  })

  it("uses force-cache only for public authority reads", () => {
    const unexpectedForceCache = appSourceFiles
      .filter((file) => /cache\s*:\s*["']force-cache["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))
      .filter((file) => !forceCacheAllowlist.has(file))

    expect(unexpectedForceCache).toEqual([])
  })

  it("does not combine no-store with next.revalidate", () => {
    const conflicts = appSourceFiles
      .flatMap((file) => fetchCalls(file))
      .filter((call) => /cache\s*:\s*["']no-store["']/.test(call.source) && /revalidate\s*:/.test(call.source))
      .map((call) => `${relative(frontendDir, call.file)}:${call.line}`)

    expect(conflicts).toEqual([])
  })

  it("does not attach literal fetch cache tags outside the authority tag helper path", () => {
    const directFetchTags = appSourceFiles
      .flatMap((file) => fetchCalls(file))
      .filter((call) => /tags\s*:\s*\[/.test(call.source))
      .map((call) => `${relative(frontendDir, call.file)}:${call.line}`)

    expect(directFetchTags).toEqual([])
  })
})

function hasExplicitCacheIntent(source: string, fileSource: string) {
  if (/cache\s*:/.test(source) || /signal\s*:/.test(source)) return true

  const optionsIdentifier = source.match(/,\s*([A-Za-z_$][\w$]*)\s*\)?\s*$/)?.[1]
  return Boolean(
    optionsIdentifier &&
      new RegExp(`\\b(?:const|let)\\s+${escapeRegExp(optionsIdentifier)}\\b[\\s\\S]*?=\\s*{[\\s\\S]*?(?:cache|signal)\\s*:`).test(fileSource),
  )
}

function fetchCalls(file: string) {
  const source = readFileSync(file, "utf8")
  return Array.from(source.matchAll(/\bfetch\s*\(/g)).map((match) => {
    const index = match.index ?? 0
    return {
      file,
      fileSource: source,
      line: source.slice(0, index).split("\n").length,
      source: source.slice(index, callExpressionEnd(source, index)),
    }
  })
}

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}

function callExpressionEnd(source: string, start: number) {
  let depth = 0
  for (let index = start; index < source.length; index += 1) {
    const char = source[index]
    if (char === "(") depth += 1
    if (char === ")") {
      depth -= 1
      if (depth === 0) return index + 1
    }
  }
  return source.length
}

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx|mjs)$/.test(entry) && !/(\.d|\.test)\.tsx?$/.test(entry) ? [path] : []
  })
}
