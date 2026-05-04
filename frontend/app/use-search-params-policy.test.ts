import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const useSearchParamsFiles = sourceFiles.filter((file) => /\buseSearchParams\s*\(/.test(readFileSync(file, "utf8")))
const expectedUseSearchParamsFiles = new Set([
  "components/casebuilder/timeline-view.tsx",
  "components/orsg/left-rail.tsx",
  "components/orsg/statute/statute-tabs.tsx",
])

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

describe("useSearchParams() policy", () => {
  it("imports useSearchParams only from next/navigation", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\buseSearchParams\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\buseSearchParams\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\buseSearchParams\b[^}]*\}\s+from\s+["']next\/navigation["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("uses useSearchParams only in Client Components", () => {
    const serverHookCalls = useSearchParamsFiles
      .filter((file) => !clientFiles.includes(file))
      .map((file) => relative(frontendDir, file))

    expect(serverHookCalls).toEqual([])
  })

  it("keeps route convention files on searchParams props instead of useSearchParams", () => {
    const routeConventionHookCalls = useSearchParamsFiles
      .filter((file) => file.startsWith(appDir))
      .filter((file) => routeConventionFiles.has(basename(file)))
      .map((file) => relative(frontendDir, file))

    expect(routeConventionHookCalls).toEqual([])
  })

  it("calls useSearchParams without arguments", () => {
    const callsWithArguments = useSearchParamsFiles
      .filter((file) => /\buseSearchParams\s*\(\s*[^)\s]/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(callsWithArguments).toEqual([])
  })

  it("does not use useSearchParams in Pages Router files", () => {
    const pagesRouterCalls = useSearchParamsFiles
      .filter((file) => relative(frontendDir, file).startsWith("pages/"))
      .map((file) => relative(frontendDir, file))

    expect(pagesRouterCalls).toEqual([])
  })

  it("keeps useSearchParams usage limited to known client navigation islands", () => {
    const unexpectedConsumers = useSearchParamsFiles
      .map((file) => relative(frontendDir, file))
      .filter((file) => !expectedUseSearchParamsFiles.has(file))

    expect(unexpectedConsumers).toEqual([])
  })

  it("treats hook return values as read-only", () => {
    const mutations = useSearchParamsFiles.flatMap((file) => {
      const source = readFileSync(file, "utf8")
      return searchParamsBindings(source)
        .filter((binding) => new RegExp(`\\b${binding}\\.(?:append|delete|set|sort)\\s*\\(`).test(source))
        .map((binding) => `${relative(frontendDir, file)}:${binding}`)
    })

    expect(mutations).toEqual([])
  })

  it("uses URLSearchParams copies when updating query strings", () => {
    const unsafeStringAssembly = useSearchParamsFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\?\$\{?\s*searchParams\b/.test(source) || /\+\s*searchParams\s*(?:\+|$)/m.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(unsafeStringAssembly).toEqual([])
  })

  it("keeps current search-param consumers under stable Suspense boundaries", () => {
    const shell = readFileSync(join(frontendDir, "components/orsg/shell.tsx"), "utf8")
    const statuteWorkspace = readFileSync(join(frontendDir, "components/orsg/statute/statute-detail-workspace.tsx"), "utf8")
    const timelinePage = readFileSync(join(frontendDir, "app/matters/[id]/timeline/page.tsx"), "utf8")

    expect(shell).toContain("<Suspense fallback={<div className=\"h-8 w-8 lg:hidden\"")
    expect(shell).toContain("<MobileLeftRailSlot />")
    expect(shell).toContain("<Suspense fallback={<div className=\"w-72 border-r border-border bg-sidebar\" />}>")
    expect(shell).toContain("<LeftRailSlot />")
    expect(statuteWorkspace).toContain("<Suspense fallback={<StatuteTabsFallback />}>")
    expect(statuteWorkspace).toContain("<StatuteTabs")
    expect(timelinePage).toContain("<Suspense fallback={<TimelineFallback />}>")
    expect(timelinePage).toContain("<TimelineView matter={matter} />")
  })

  it("keeps Server Component pages typed with Promise searchParams props", () => {
    const untypedSearchParamPages = sourceFiles
      .filter((file) => file.startsWith(appDir))
      .filter((file) => basename(file) === "page.tsx")
      .filter((file) => !hasUseClientDirective(readFileSync(file, "utf8")))
      .filter((file) => /\bsearchParams\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return !/\bsearchParams\s*:\s*Promise</.test(source) && !/\bPageProps<["']\//.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(untypedSearchParamPages).toEqual([])
  })
})

function searchParamsBindings(source: string) {
  return Array.from(source.matchAll(/\b(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=\s*useSearchParams\s*\(/g), (match) => match[1] ?? "")
    .filter(Boolean)
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
