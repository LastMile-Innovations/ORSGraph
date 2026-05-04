import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))

describe("cookies() policy", () => {
  it("does not import request-time APIs into Client Components", () => {
    const clientRequestApiImports = clientFiles
      .filter((file) => /from\s+["']next\/headers["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(clientRequestApiImports).toEqual([])
  })

  it("awaits cookies() instead of using the deprecated synchronous form", () => {
    const syncCookieReads = sourceFiles
      .filter((file) => hasSyncCookiesCall(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(syncCookieReads).toEqual([])
  })

  it("writes cookies only from Server Functions or Route Handlers", () => {
    const invalidCookieWrites = sourceFiles
      .filter((file) => !canWriteCookies(file, readFileSync(file, "utf8")))
      .filter((file) => hasCookieWrite(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(invalidCookieWrites).toEqual([])
  })

  it("does not read cookies inside Server Component after() callbacks", () => {
    const unsafeAfterCookieReads = serverComponentFiles()
      .filter((file) => /\bafter\s*\(\s*(?:async\s*)?\([^)]*\)\s*=>\s*{[\s\S]*?\bcookies\s*\(/m.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(unsafeAfterCookieReads).toEqual([])
  })
})

function hasSyncCookiesCall(source: string) {
  return Array.from(source.matchAll(/\bcookies\s*\(\s*\)/g)).some((match) => {
    const index = match.index ?? 0
    const prefix = source.slice(Math.max(0, index - 16), index)
    return !/\bawait\s*$/.test(prefix)
  })
}

function hasCookieWrite(source: string) {
  if (/\(\s*await\s+cookies\s*\(\s*\)\s*\)\s*\.\s*(?:set|delete)\s*\(/.test(source)) return true

  const cookieStores = Array.from(source.matchAll(/\b(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=\s*await\s+cookies\s*\(\s*\)/g))
    .map((match) => match[1])
    .filter(Boolean)

  return cookieStores.some((store) => new RegExp(`\\b${escapeRegExp(store)}\\s*\\.\\s*(?:set|delete)\\s*\\(`).test(source))
}

function canWriteCookies(file: string, source: string) {
  return basename(file) === "route.ts" || hasUseServerDirective(source)
}

function serverComponentFiles() {
  const appDir = join(frontendDir, "app")
  return sourceFiles.filter((file) => {
    if (!file.startsWith(appDir)) return false
    const source = readFileSync(file, "utf8")
    if (hasUseClientDirective(source) || hasUseServerDirective(source)) return false
    if (basename(file) === "route.ts") return false
    return /\.(ts|tsx)$/.test(file)
  })
}

function hasUseClientDirective(source: string) {
  return /^\s*["']use client["']/.test(source)
}

function hasUseServerDirective(source: string) {
  return /^\s*["']use server["']/.test(source)
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
