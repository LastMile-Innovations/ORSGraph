import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const proxyFiles = sourceFiles.filter((file) => basename(file) === "proxy.ts")
const updateTagFiles = sourceFiles.filter((file) => /\bupdateTag\s*\(/.test(readFileSync(file, "utf8")))

describe("updateTag() policy", () => {
  it("imports updateTag only from next/cache", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\bupdateTag\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\bupdateTag\b[^}]*\}\s+from/.test(source) && !hasUpdateTagImport(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("does not import updateTag into Client Components or Proxy", () => {
    const invalidFiles = [...clientFiles, ...proxyFiles]
      .filter((file) => hasUpdateTagImport(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("calls updateTag only from Server Action modules", () => {
    const invalidCallsites = updateTagFiles
      .filter((file) => !hasUseServerDirective(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(invalidCallsites).toEqual([])
  })

  it("does not use updateTag from Route Handlers", () => {
    const routeHandlerCalls = updateTagFiles
      .filter((file) => basename(file) === "route.ts")
      .map((file) => relative(frontendDir, file))

    expect(routeHandlerCalls).toEqual([])
  })

  it("calls updateTag with exactly one argument", () => {
    const invalidCalls = updateTagCalls()
      .filter(({ args }) => args.length !== 1)
      .map(({ file, args }) => `${relative(frontendDir, file)}:${args.join(",") || "missing-args"}`)

    expect(invalidCalls).toEqual([])
  })

  it("keeps literal tag values non-empty and within Next.js limits", () => {
    const invalidLiteralTags = updateTagCalls()
      .map(({ file, args }) => ({ file, tag: stringLiteral(args[0] ?? "") }))
      .filter(({ tag }) => tag !== undefined && (tag.length === 0 || tag.length > 256))
      .map(({ file, tag }) => `${relative(frontendDir, file)}:${tag}`)

    expect(invalidLiteralTags).toEqual([])
  })

  it("does not update unbounded user-provided tags directly", () => {
    const unboundedTags = updateTagCalls()
      .filter(({ args }) => /\b(?:formData|request|req|searchParams)\.?(?:get|nextUrl\.searchParams\.get)\s*\(/.test(args[0] ?? ""))
      .map(({ file }) => relative(frontendDir, file))

    expect(unboundedTags).toEqual([])
  })

  it("does not use return updateTag()", () => {
    const returnedUpdates = updateTagFiles
      .filter((file) => /\breturn\s+updateTag\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(returnedUpdates).toEqual([])
  })

  it("keeps authority Route Handler invalidation on revalidateTag stale-while-revalidate", () => {
    const route = readFileSync(join(frontendDir, "app/api/ors/[...path]/route.ts"), "utf8")

    expect(route).toContain("authorityCacheTags(AUTHORITY_RELEASE_ID)")
    expect(route).toContain('revalidateTag(tag, "max")')
    expect(route).not.toContain("updateTag(")
  })
})

function updateTagCalls() {
  return updateTagFiles.flatMap((file) => {
    const source = readFileSync(file, "utf8")
    return Array.from(source.matchAll(/\bupdateTag\s*\(([\s\S]*?)\)/g)).map((match) => ({
      file,
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

function hasUpdateTagImport(source: string) {
  return /import\s+\{[^}]*\bupdateTag\b[^}]*\}\s+from\s+["']next\/cache["']/.test(source)
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
    return /\.(ts|tsx|mjs)$/.test(entry) && !/(\.d|\.test)\.tsx?$/.test(entry) ? [path] : []
  })
}
