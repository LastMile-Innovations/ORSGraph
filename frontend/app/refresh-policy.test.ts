import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const refreshImportFiles = sourceFiles.filter((file) => hasRefreshImport(readFileSync(file, "utf8")))
const refreshCallFiles = sourceFiles.filter((file) => hasServerRefreshCall(readFileSync(file, "utf8")))

describe("refresh() policy", () => {
  it("imports server refresh only from next/cache", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\brefresh\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\brefresh\b[^}]*\}\s+from/.test(source) && !hasRefreshImport(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("calls server refresh only from Server Action modules", () => {
    const invalidRefreshCalls = refreshCallFiles
      .filter((file) => !hasUseServerDirective(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(invalidRefreshCalls).toEqual([])
  })

  it("does not import server refresh into Client Components", () => {
    const clientRefreshImports = clientFiles
      .filter((file) => hasRefreshImport(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(clientRefreshImports).toEqual([])
  })

  it("does not use server refresh from Route Handlers", () => {
    const routeHandlerRefresh = refreshCallFiles
      .filter((file) => basename(file) === "route.ts")
      .map((file) => relative(frontendDir, file))

    expect(routeHandlerRefresh).toEqual([])
  })

  it("calls refresh() without arguments", () => {
    const refreshWithArguments = refreshCallFiles
      .filter((file) => /\brefresh\s*\(\s*[^)\s]/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(refreshWithArguments).toEqual([])
  })

  it("does not use return refresh()", () => {
    const returnedRefresh = refreshCallFiles
      .filter((file) => /\breturn\s+refresh\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(returnedRefresh).toEqual([])
  })

  it("keeps client router.refresh() separate from next/cache refresh()", () => {
    const missingServerRefreshImports = refreshImportFiles
      .filter((file) => !hasServerRefreshCall(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(missingServerRefreshImports).toEqual([])
  })
})

function hasRefreshImport(source: string) {
  return /import\s+\{[^}]*\brefresh\b[^}]*\}\s+from\s+["']next\/cache["']/.test(source)
}

function hasServerRefreshCall(source: string) {
  return /(?<!\.)\brefresh\s*\(/.test(source)
}

function hasUseClientDirective(source: string) {
  return /^\s*["']use client["']/.test(source)
}

function hasUseServerDirective(source: string) {
  return /^\s*["']use server["']/.test(source)
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
