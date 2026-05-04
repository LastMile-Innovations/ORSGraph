import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const proxyFiles = sourceFiles.filter((file) => basename(file) === "proxy.ts")
const revalidateTagFiles = sourceFiles.filter((file) => /\brevalidateTag\s*\(/.test(readFileSync(file, "utf8")))
const cacheLifeProfiles = configuredCacheLifeProfiles()

describe("revalidateTag() policy", () => {
  it("imports revalidateTag only from next/cache", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\brevalidateTag\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\brevalidateTag\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\brevalidateTag\b[^}]*\}\s+from\s+["']next\/cache["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("does not import revalidateTag into Client Components or Proxy", () => {
    const invalidFiles = [...clientFiles, ...proxyFiles]
      .filter((file) => /import\s+\{[^}]*\brevalidateTag\b[^}]*\}\s+from\s+["']next\/cache["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("calls revalidateTag only from Server Actions or Route Handlers", () => {
    const invalidCallsites = revalidateTagFiles
      .filter((file) => !hasUseServerDirective(readFileSync(file, "utf8")) && basename(file) !== "route.ts")
      .map((file) => relative(frontendDir, file))

    expect(invalidCallsites).toEqual([])
  })

  it("uses the non-deprecated two-argument signature", () => {
    const singleArgumentCalls = revalidateTagCalls()
      .filter(({ args }) => args.length < 2)
      .map(({ file, args }) => `${relative(frontendDir, file)}:${args.join(",") || "missing-args"}`)

    expect(singleArgumentCalls).toEqual([])
  })

  it("uses recommended or configured revalidation profiles", () => {
    const invalidProfiles = revalidateTagCalls()
      .filter(({ args }) => {
        const profile = args[1] ?? ""
        const literalProfile = stringLiteral(profile)
        if (literalProfile) return literalProfile !== "max" && !cacheLifeProfiles.has(literalProfile)
        return !/^\{\s*expire\s*:\s*\d+\s*\}$/.test(profile)
      })
      .map(({ file, args }) => `${relative(frontendDir, file)}:${args[1] ?? "missing-profile"}`)

    expect(invalidProfiles).toEqual([])
  })

  it("keeps literal tag values non-empty and within Next.js limits", () => {
    const invalidLiteralTags = revalidateTagCalls()
      .map(({ file, args }) => ({ file, tag: stringLiteral(args[0] ?? "") }))
      .filter(({ tag }) => tag && tag.length > 256)
      .map(({ file, tag }) => `${relative(frontendDir, file)}:${tag}`)

    expect(invalidLiteralTags).toEqual([])
  })

  it("does not revalidate unbounded user-provided tag query params directly", () => {
    const directQueryTags = revalidateTagCalls()
      .filter(({ args }) => /\b(?:request|req)\.nextUrl\.searchParams\.get\s*\(/.test(args[0] ?? ""))
      .map(({ file }) => relative(frontendDir, file))

    expect(directQueryTags).toEqual([])
  })

  it("does not use return revalidateTag()", () => {
    const returnedRevalidation = revalidateTagFiles
      .filter((file) => /\breturn\s+revalidateTag\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(returnedRevalidation).toEqual([])
  })

  it("uses immediate expire objects only from Route Handlers", () => {
    const invalidImmediateExpiry = revalidateTagCalls()
      .filter(({ file, args }) => /\{\s*expire\s*:\s*0\s*\}/.test(args[1] ?? "") && basename(file) !== "route.ts")
      .map(({ file }) => relative(frontendDir, file))

    expect(invalidImmediateExpiry).toEqual([])
  })

  it("invalidates authority caches through release-scoped tags with stale-while-revalidate semantics", () => {
    const route = readFileSync(join(frontendDir, "app/api/ors/[...path]/route.ts"), "utf8")

    expect(route).toContain("authorityCacheTags(AUTHORITY_RELEASE_ID)")
    expect(route).toContain('revalidateTag(tag, "max")')
  })
})

function revalidateTagCalls() {
  return revalidateTagFiles.flatMap((file) => {
    const source = readFileSync(file, "utf8")
    return Array.from(source.matchAll(/\brevalidateTag\s*\(([\s\S]*?)\)/g)).map((match) => ({
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
  return match?.[1] ?? ""
}

function configuredCacheLifeProfiles() {
  const config = readFileSync(join(frontendDir, "next.config.mjs"), "utf8")
  const profileBlock = config.match(/\bcacheLife\s*:\s*\{([\s\S]*?)\n\s*\},\n\s*experimental:/)?.[1] ?? ""
  return new Set(Array.from(profileBlock.matchAll(/\b([A-Za-z][\w-]*)\s*:\s*\{/g), (match) => match[1]))
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
