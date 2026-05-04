import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const appFiles = sourceFiles.filter((file) => file.startsWith(appDir))
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))

describe("headers() policy", () => {
  it("does not import request-time header APIs into Client Components", () => {
    const clientHeadersImports = clientFiles
      .filter((file) => /from\s+["']next\/headers["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(clientHeadersImports).toEqual([])
  })

  it("keeps non-app modules that read request headers marked server-only", () => {
    const unmarkedServerModules = sourceFiles
      .filter((file) => !file.startsWith(appDir))
      .filter((file) => /from\s+["']next\/headers["']/.test(readFileSync(file, "utf8")))
      .filter((file) => !/import\s+["']server-only["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(unmarkedServerModules).toEqual([])
  })

  it("awaits headers() instead of using the deprecated synchronous form", () => {
    const syncHeaderReads = sourceFiles
      .filter((file) => hasSyncHeadersCall(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(syncHeaderReads).toEqual([])
  })

  it("calls headers() without arguments", () => {
    const headersWithArguments = sourceFiles
      .filter((file) => /\bheaders\s*\(\s*[^)\s]/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(headersWithArguments).toEqual([])
  })

  it("treats headers() results as read-only", () => {
    const mutatedRequestHeaders = sourceFiles
      .filter((file) => hasRequestHeadersMutation(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(mutatedRequestHeaders).toEqual([])
  })

  it("does not read headers inside Server Component after() callbacks", () => {
    const unsafeAfterHeaderReads = serverComponentFiles()
      .filter((file) => /\bafter\s*\(\s*(?:async\s*)?\([^)]*\)\s*=>\s*{[\s\S]*?\bheaders\s*\(/m.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(unsafeAfterHeaderReads).toEqual([])
  })

  it("keeps request headers out of metadata and viewport generation", () => {
    const requestHeaderHeadExports = appFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bheaders\s*\(/.test(generateMetadataBody(source)) || /\bheaders\s*\(/.test(generateViewportBody(source))
      })
      .map((file) => relative(frontendDir, file))

    expect(requestHeaderHeadExports).toEqual([])
  })
})

function hasSyncHeadersCall(source: string) {
  return Array.from(source.matchAll(/\bheaders\s*\(\s*\)/g)).some((match) => {
    const index = match.index ?? 0
    const prefix = source.slice(Math.max(0, index - 16), index)
    return !/\bawait\s*$/.test(prefix)
  })
}

function hasRequestHeadersMutation(source: string) {
  if (/\(\s*await\s+headers\s*\(\s*\)\s*\)\s*\.\s*(?:append|delete|set)\s*\(/.test(source)) return true

  const headersStores = Array.from(source.matchAll(/\b(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=\s*await\s+headers\s*\(\s*\)/g))
    .map((match) => match[1])
    .filter(Boolean)

  return headersStores.some((store) =>
    new RegExp(`\\b${escapeRegExp(store)}\\s*\\.\\s*(?:append|delete|set)\\s*\\(`).test(source),
  )
}

function generateMetadataBody(source: string) {
  const start = source.search(/export\s+(?:async\s+)?function\s+generateMetadata\b/)
  if (start < 0) return ""
  return source.slice(start, blockEnd(source, source.indexOf("{", start)))
}

function generateViewportBody(source: string) {
  const start = source.search(/export\s+(?:async\s+)?function\s+generateViewport\b/)
  if (start < 0) return ""
  return source.slice(start, blockEnd(source, source.indexOf("{", start)))
}

function serverComponentFiles() {
  return appFiles.filter((file) => {
    const source = readFileSync(file, "utf8")
    if (hasUseClientDirective(source) || hasUseServerDirective(source)) return false
    if (basename(file) === "route.ts") return false
    return /\.(ts|tsx)$/.test(file)
  })
}

function blockEnd(source: string, start: number) {
  if (start < 0) return source.length

  let depth = 0
  for (let index = start; index < source.length; index += 1) {
    const char = source[index]
    if (char === "{") depth += 1
    if (char === "}") {
      depth -= 1
      if (depth === 0) return index + 1
    }
  }
  return source.length
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
