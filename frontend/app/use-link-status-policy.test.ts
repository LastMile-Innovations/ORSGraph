import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const useLinkStatusFiles = sourceFiles.filter((file) => /\buseLinkStatus\s*\(/.test(readFileSync(file, "utf8")))
const hoverPrefetchLinkFile = join(frontendDir, "components/navigation/hover-prefetch-link.tsx")

describe("useLinkStatus() policy", () => {
  it("imports useLinkStatus only from next/link", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\buseLinkStatus\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+[\s\S]*?\buseLinkStatus\b[\s\S]*?\s+from/.test(source) && !/from\s+["']next\/link["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("uses useLinkStatus only in Client Components", () => {
    const serverHookCalls = useLinkStatusFiles
      .filter((file) => !clientFiles.includes(file))
      .map((file) => relative(frontendDir, file))

    expect(serverHookCalls).toEqual([])
  })

  it("centralizes inline link pending state in HoverPrefetchLink", () => {
    const decentralizedHookCalls = useLinkStatusFiles
      .filter((file) => file !== hoverPrefetchLinkFile)
      .map((file) => relative(frontendDir, file))

    expect(decentralizedHookCalls).toEqual([])
  })

  it("calls useLinkStatus without arguments", () => {
    const callsWithArguments = useLinkStatusFiles
      .filter((file) => /\buseLinkStatus\s*\(\s*[^)\s]/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(callsWithArguments).toEqual([])
  })

  it("keeps the hook below a Link descendant instead of standalone route UI", () => {
    const source = readFileSync(hoverPrefetchLinkFile, "utf8")

    expect(source).toContain("<Link")
    expect(source).toContain("<LinkPendingIndicator")
    expect(source.indexOf("<Link")).toBeLessThan(source.indexOf("<LinkPendingIndicator"))
    expect(source.indexOf("function LinkPendingIndicator")).toBeLessThan(source.indexOf("useLinkStatus()"))
  })

  it("uses hover/focus prefetch activation instead of permanently disabling prefetch", () => {
    const source = readFileSync(hoverPrefetchLinkFile, "utf8")

    expect(source).toContain("prefetch={active ? null : false}")
    expect(source).toContain("onFocus={(event)")
    expect(source).toContain("onMouseEnter={(event)")
  })

  it("keeps the pending hint layout-stable and non-announced", () => {
    const source = readFileSync(hoverPrefetchLinkFile, "utf8")

    expect(source).toContain('aria-hidden="true"')
    expect(source).toMatch(/\bh-[\w./[\]-]+\b/)
    expect(source).toMatch(/\bw-[\w./[\]-]+\b/)
    expect(source).toContain("shrink-0")
    expect(source).toContain("opacity-0")
    expect(source).toContain("delay-100")
    expect(source).toContain("pending &&")
  })

  it("uses HoverPrefetchLink pending indicators only from positioned links", () => {
    const invalidUsages = sourceFiles
      .filter((file) => /<HoverPrefetchLink[\s\S]*?pendingIndicatorClassName=/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return !/<HoverPrefetchLink[\s\S]*?pendingIndicatorClassName=[\s\S]*?className=["'][^"']*\brelative\b/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidUsages).toEqual([])
  })

  it("does not use useLinkStatus in Pages Router files", () => {
    const pagesRouterCalls = useLinkStatusFiles
      .filter((file) => relative(frontendDir, file).startsWith(`pages${pathSeparator()}`))
      .map((file) => relative(frontendDir, file))

    expect(pagesRouterCalls).toEqual([])
  })
})

function pathSeparator() {
  return "/"
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
  })
}
