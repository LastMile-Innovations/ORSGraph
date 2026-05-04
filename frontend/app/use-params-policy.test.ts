import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const useParamsFiles = sourceFiles.filter((file) => /\buseParams\s*\(/.test(readFileSync(file, "utf8")))

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

describe("useParams() policy", () => {
  it("imports useParams only from next/navigation", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\buseParams\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\buseParams\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\buseParams\b[^}]*\}\s+from\s+["']next\/navigation["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("uses useParams only in Client Components", () => {
    const serverHookCalls = useParamsFiles
      .filter((file) => !clientFiles.includes(file))
      .map((file) => relative(frontendDir, file))

    expect(serverHookCalls).toEqual([])
  })

  it("keeps route convention files on params props instead of useParams", () => {
    const routeConventionHookCalls = useParamsFiles
      .filter((file) => file.startsWith(appDir))
      .filter((file) => routeConventionFiles.has(basename(file)))
      .map((file) => relative(frontendDir, file))

    expect(routeConventionHookCalls).toEqual([])
  })

  it("calls useParams without runtime arguments", () => {
    const callsWithArguments = useParamsFiles
      .filter((file) => /\buseParams(?:\s*<[^>]+>)?\s*\(\s*[^)\s]/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(callsWithArguments).toEqual([])
  })

  it("uses explicit TypeScript generics for client-side params", () => {
    const untypedCalls = useParamsFiles
      .filter((file) => !/\buseParams\s*</.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(untypedCalls).toEqual([])
  })

  it("does not assume Pages Router null behavior", () => {
    const pagesRouterCalls = useParamsFiles
      .filter((file) => relative(frontendDir, file).startsWith("pages/"))
      .map((file) => relative(frontendDir, file))

    expect(pagesRouterCalls).toEqual([])
  })

  it("keeps Server Components typed with PageProps or LayoutProps for dynamic params", () => {
    const untypedServerDynamicRoutes = sourceFiles
      .filter((file) => file.startsWith(appDir))
      .filter((file) => routeConventionFiles.has(basename(file)))
      .filter((file) => relative(appDir, file).includes("["))
      .filter((file) => !hasUseClientDirective(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bparams\b/.test(source) && !/\b(?:PageProps|LayoutProps)<["']\//.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(untypedServerDynamicRoutes).toEqual([])
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
