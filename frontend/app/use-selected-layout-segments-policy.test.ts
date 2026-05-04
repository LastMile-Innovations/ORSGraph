import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const selectedLayoutSegmentsFiles = sourceFiles.filter((file) => /\buseSelectedLayoutSegments\s*\(/.test(readFileSync(file, "utf8")))
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

describe("useSelectedLayoutSegments() policy", () => {
  it("imports useSelectedLayoutSegments only from next/navigation", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\buseSelectedLayoutSegments\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\buseSelectedLayoutSegments\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\buseSelectedLayoutSegments\b[^}]*\}\s+from\s+["']next\/navigation["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("uses useSelectedLayoutSegments only in Client Components", () => {
    const serverHookCalls = selectedLayoutSegmentsFiles
      .filter((file) => !clientFiles.includes(file))
      .map((file) => relative(frontendDir, file))

    expect(serverHookCalls).toEqual([])
  })

  it("keeps route convention files from calling the Client Component hook directly", () => {
    const routeConventionHookCalls = selectedLayoutSegmentsFiles
      .filter((file) => file.startsWith(appDir))
      .filter((file) => routeConventionFiles.has(basename(file)))
      .map((file) => relative(frontendDir, file))

    expect(routeConventionHookCalls).toEqual([])
  })

  it("does not use useSelectedLayoutSegments in Pages Router files", () => {
    const pagesRouterCalls = selectedLayoutSegmentsFiles
      .filter((file) => relative(frontendDir, file).startsWith("pages/"))
      .map((file) => relative(frontendDir, file))

    expect(pagesRouterCalls).toEqual([])
  })

  it("calls useSelectedLayoutSegments with no args or one literal parallel route key", () => {
    const invalidCalls = selectedLayoutSegmentsCalls()
      .filter(({ args }) => args.length > 1 || (args.length === 1 && !stringLiteral(args[0] ?? "")))
      .map(({ file, line, args }) => `${relative(frontendDir, file)}:${line}:${args.join(",")}`)

    expect(invalidCalls).toEqual([])
  })

  it("keeps parallel route keys literal and slot-backed when introduced", () => {
    const parallelRouteKeys = selectedLayoutSegmentsCalls()
      .map(({ args }) => stringLiteral(args[0] ?? ""))
      .filter((key): key is string => Boolean(key))
    const missingSlots = parallelRouteKeys
      .filter((key) => !sourceFiles.some((file) => relative(appDir, file).split("/").includes(`@${key}`)))

    expect(missingSlots).toEqual([])
  })

  it("filters route group segments before rendering breadcrumbs or labels", () => {
    const unfilteredRenderers = selectedLayoutSegmentsFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\.map\s*\(/.test(source) && !/\.filter\s*\([\s\S]*?startsWith\s*\(\s*["']\(["']\s*\)/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(unfilteredRenderers).toEqual([])
  })

  it("does not split catch-all segment strings into fake hierarchy", () => {
    const catchAllSplits = selectedLayoutSegmentsFiles
      .filter((file) => /\.split\s*\(\s*["']\/["']\s*\)/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(catchAllSplits).toEqual([])
  })
})

function selectedLayoutSegmentsCalls() {
  return selectedLayoutSegmentsFiles.flatMap((file) => {
    const source = readFileSync(file, "utf8")
    return Array.from(source.matchAll(/\buseSelectedLayoutSegments\s*\(([\s\S]*?)\)/g)).map((match) => ({
      file,
      line: source.slice(0, match.index).split("\n").length,
      args: splitTopLevelArgs(match[1] ?? ""),
    }))
  })
}

function splitTopLevelArgs(args: string) {
  const parts: string[] = []
  let depth = 0
  let quote = ""
  let start = 0
  for (let index = 0; index < args.length; index += 1) {
    const char = args[index]
    const previous = args[index - 1]
    if (quote) {
      if (char === quote && previous !== "\\") quote = ""
      continue
    }
    if (char === '"' || char === "'" || char === "`") {
      quote = char
      continue
    }
    if (char === "{" || char === "[" || char === "(") depth += 1
    if (char === "}" || char === "]" || char === ")") depth -= 1
    if (char === "," && depth === 0) {
      parts.push(args.slice(start, index).trim())
      start = index + 1
    }
  }
  const tail = args.slice(start).trim()
  if (tail) parts.push(tail)
  return parts
}

function stringLiteral(value: string) {
  const match = value.match(/^["']([^"']*)["']$/)
  return match?.[1]
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
