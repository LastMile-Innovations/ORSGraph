import { existsSync, readdirSync, readFileSync, statSync } from "node:fs"
import { basename, dirname, join, relative, sep } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const appFiles = sourceFiles.filter((file) => file.startsWith(appDir))
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const notFoundCallFiles = appFiles.filter((file) => /\bnotFound\s*\(/.test(readFileSync(file, "utf8")))

describe("notFound() policy", () => {
  it("imports notFound only from next/navigation", () => {
    const invalidImports = appFiles
      .filter((file) => /\bnotFound\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\bnotFound\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\bnotFound\b[^}]*\}\s+from\s+["']next\/navigation["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("keeps notFound() out of Client Components", () => {
    const clientNotFoundCalls = clientFiles
      .filter((file) => /\bnotFound\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(clientNotFoundCalls).toEqual([])
  })

  it("does not use return notFound()", () => {
    const returnedNotFound = notFoundCallFiles
      .filter((file) => /\breturn\s+notFound\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(returnedNotFound).toEqual([])
  })

  it("does not hand-roll NEXT_HTTP_ERROR_FALLBACK 404 errors", () => {
    const manualNotFoundErrors = sourceFiles
      .filter((file) => /\bNEXT_HTTP_ERROR_FALLBACK;404\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(manualNotFoundErrors).toEqual([])
  })

  it("uses JSON 404 responses in Route Handlers instead of route UI notFound()", () => {
    const routeHandlerNotFound = notFoundCallFiles
      .filter((file) => basename(file) === "route.ts")
      .map((file) => relative(frontendDir, file))

    expect(routeHandlerNotFound).toEqual([])
  })

  it("has a not-found convention file available for segments that call notFound()", () => {
    const missingConvention = notFoundCallFiles
      .filter((file) => !nearestNotFoundFile(file))
      .map((file) => relative(frontendDir, file))

    expect(missingConvention).toEqual([])
  })

  it("does not wrap notFound route misses in component-level error boundaries", () => {
    const wrappedNotFound = notFoundCallFiles
      .filter((file) => /unstable_catchError|from\s+["']next\/error["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(wrappedNotFound).toEqual([])
  })
})

function nearestNotFoundFile(file: string) {
  let dir = dirname(file)
  while (dir.startsWith(appDir)) {
    const candidate = join(dir, "not-found.tsx")
    if (existsSync(candidate)) return candidate
    if (dir === appDir) break
    dir = dirname(dir)
  }
  return null
}

function hasUseClientDirective(source: string) {
  return /^\s*["']use client["']/.test(source)
}

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx)$/.test(entry) && !/(\.d|\.test)\.tsx?$/.test(entry) ? [path] : []
  }).sort((a, b) => a.split(sep).join("/").localeCompare(b.split(sep).join("/")))
}
