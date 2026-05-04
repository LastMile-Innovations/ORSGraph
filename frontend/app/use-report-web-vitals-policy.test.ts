import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const webVitalsFiles = sourceFiles.filter((file) => /\buseReportWebVitals\s*\(/.test(readFileSync(file, "utf8")))
const webVitalsComponentFile = join(frontendDir, "components/web-vitals.tsx")

const routeConventionFiles = new Set([
  "page.tsx",
  "layout.tsx",
  "template.tsx",
  "default.tsx",
  "loading.tsx",
  "error.tsx",
  "global-error.tsx",
  "not-found.tsx",
])

describe("useReportWebVitals() policy", () => {
  it("imports useReportWebVitals only from next/web-vitals", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\buseReportWebVitals\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\buseReportWebVitals\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\buseReportWebVitals\b[^}]*\}\s+from\s+["']next\/web-vitals["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("uses useReportWebVitals only in Client Components", () => {
    const serverHookCalls = webVitalsFiles
      .filter((file) => !clientFiles.includes(file))
      .map((file) => relative(frontendDir, file))

    expect(serverHookCalls).toEqual([])
  })

  it("keeps the web vitals client boundary out of route convention files", () => {
    const routeConventionHookCalls = webVitalsFiles
      .filter((file) => file.startsWith(appDir))
      .filter((file) => routeConventionFiles.has(basename(file)))
      .map((file) => relative(frontendDir, file))

    expect(routeConventionHookCalls).toEqual([])
  })

  it("centralizes web vitals reporting in the dedicated component", () => {
    const decentralizedHookCalls = webVitalsFiles
      .filter((file) => file !== webVitalsComponentFile)
      .map((file) => relative(frontendDir, file))

    expect(decentralizedHookCalls).toEqual([])
  })

  it("uses a stable module-scope callback reference", () => {
    const source = readFileSync(webVitalsComponentFile, "utf8")

    expect(source).toContain("const reportWebVitals: ReportWebVitalsCallback =")
    expect(source.indexOf("const reportWebVitals")).toBeLessThan(source.indexOf("export function WebVitals"))
    expect(source).toContain("useReportWebVitals(reportWebVitals)")
    expect(source).not.toMatch(/useReportWebVitals\s*\(\s*(?:async\s*)?\(?\s*metric\s*=>/)
    expect(source).not.toMatch(/useReportWebVitals\s*\(\s*function/)
  })

  it("keeps reporting optional and Railway-neutral", () => {
    const layout = readFileSync(join(frontendDir, "app/layout.tsx"), "utf8")
    const reporter = readFileSync(webVitalsComponentFile, "utf8")

    expect(layout).toContain("const enableWebVitals = Boolean(process.env.NEXT_PUBLIC_WEB_VITALS_ENDPOINT)")
    expect(layout).toContain("{enableWebVitals && <WebVitals />}")
    expect(reporter).toContain("process.env.NEXT_PUBLIC_WEB_VITALS_ENDPOINT")
    expect(reporter).not.toContain("VERCEL")
  })

  it("uses sendBeacon with a keepalive fetch fallback", () => {
    const source = readFileSync(webVitalsComponentFile, "utf8")

    expect(source).toContain("navigator.sendBeacon")
    expect(source).toContain("fetch(endpoint")
    expect(source).toContain("keepalive: true")
    expect(source).toContain('"content-type": "application/json"')
  })

  it("sends a bounded metric payload instead of raw PerformanceEntry arrays", () => {
    const source = readFileSync(webVitalsComponentFile, "utf8")

    expect(source).not.toContain("JSON.stringify(metric)")
    expect(source).toContain("id: metric.id")
    expect(source).toContain("name: metric.name")
    expect(source).toContain("delta: metric.delta")
    expect(source).toContain("navigationType: metric.navigationType")
    expect(source).toContain("rating: metric.rating")
    expect(source).toContain("value: metric.value")
    expect(source).toContain("path: window.location.pathname")
  })

  it("does not log vitals to the console from production code", () => {
    const consoleVitals = webVitalsFiles
      .filter((file) => /\bconsole\.(?:log|info|warn|error)\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(consoleVitals).toEqual([])
  })
})

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
