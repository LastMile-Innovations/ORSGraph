import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const usePathnameFiles = sourceFiles.filter((file) => /\busePathname\s*\(/.test(readFileSync(file, "utf8")))

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

describe("usePathname() policy", () => {
  it("imports usePathname only from next/navigation", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\busePathname\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\busePathname\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\busePathname\b[^}]*\}\s+from\s+["']next\/navigation["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("uses usePathname only in Client Components", () => {
    const serverHookCalls = usePathnameFiles
      .filter((file) => !clientFiles.includes(file))
      .map((file) => relative(frontendDir, file))

    expect(serverHookCalls).toEqual([])
  })

  it("keeps route convention files from reading pathname directly", () => {
    const routeConventionHookCalls = usePathnameFiles
      .filter((file) => file.startsWith(appDir))
      .filter((file) => routeConventionFiles.has(basename(file)))
      .map((file) => relative(frontendDir, file))

    expect(routeConventionHookCalls).toEqual([])
  })

  it("calls usePathname without arguments", () => {
    const callsWithArguments = usePathnameFiles
      .filter((file) => /\busePathname\s*\(\s*[^)\s]/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(callsWithArguments).toEqual([])
  })

  it("does not use usePathname in Pages Router files", () => {
    const pagesRouterCalls = usePathnameFiles
      .filter((file) => relative(frontendDir, file).startsWith("pages/"))
      .map((file) => relative(frontendDir, file))

    expect(pagesRouterCalls).toEqual([])
  })

  it("centralizes TopNav pathname reads behind TopNavBoundary", () => {
    const directTopNavImports = sourceFiles
      .filter((file) => !file.endsWith("components/orsg/top-nav-boundary.tsx"))
      .filter((file) => /from\s+["'](?:@\/components\/orsg\/top-nav|\.\/top-nav)["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))
    const boundary = readFileSync(join(frontendDir, "components/orsg/top-nav-boundary.tsx"), "utf8")

    expect(directTopNavImports).toEqual([])
    expect(boundary).toContain("<Suspense")
    expect(boundary).toContain("fallback={<TopNavFallback />}")
    expect(boundary).toContain("<TopNav")
  })

  it("keeps matter pathname consumers under Suspense on dynamic matter routes", () => {
    const matterShell = readFileSync(join(frontendDir, "components/casebuilder/matter-shell.tsx"), "utf8")

    expect(matterShell).toContain("<TopNavBoundary />")
    expect(matterShell).toContain("<Suspense fallback={<MatterSidebarFallback />}>")
    expect(matterShell).toContain("<MatterSidebar matter={matter} counts={counts} />")
    expect(matterShell).toMatch(/<Suspense\s+fallback=\{<div[^>]*md:hidden/)
    expect(matterShell).toContain("<MatterSidebarSheet matter={matter} counts={counts} />")
  })

  it("keeps statute and timeline pathname consumers under route-level Suspense", () => {
    const statuteWorkspace = readFileSync(join(frontendDir, "components/orsg/statute/statute-detail-workspace.tsx"), "utf8")
    const timelinePage = readFileSync(join(frontendDir, "app/matters/[id]/timeline/page.tsx"), "utf8")

    expect(statuteWorkspace).toContain("<Suspense fallback={<StatuteTabsFallback />}>")
    expect(statuteWorkspace).toContain("<StatuteTabs")
    expect(timelinePage).toContain("<Suspense fallback={<TimelineFallback />}>")
    expect(timelinePage).toContain("<TimelineView matter={matter} />")
  })

  it("keeps shared ORS shell pathname consumers under stable Suspense fallbacks", () => {
    const shell = readFileSync(join(frontendDir, "components/orsg/shell.tsx"), "utf8")

    expect(shell).toContain("<TopNavBoundary")
    expect(shell).toContain("<Suspense fallback={<div className=\"h-8 w-8 lg:hidden\"")
    expect(shell).toContain("<MobileLeftRailSlot />")
    expect(shell).toContain("<Suspense fallback={<div className=\"w-72 border-r border-border bg-sidebar\" />}>")
    expect(shell).toContain("<LeftRailSlot />")
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
