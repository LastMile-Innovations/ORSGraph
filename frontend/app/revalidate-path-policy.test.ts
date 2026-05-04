import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const proxyFiles = sourceFiles.filter((file) => basename(file) === "proxy.ts")
const revalidatePathFiles = sourceFiles.filter((file) => /\brevalidatePath\s*\(/.test(readFileSync(file, "utf8")))

describe("revalidatePath() policy", () => {
  it("imports revalidatePath only from next/cache", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\brevalidatePath\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\brevalidatePath\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\brevalidatePath\b[^}]*\}\s+from\s+["']next\/cache["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("does not import revalidatePath into Client Components or Proxy", () => {
    const invalidFiles = [...clientFiles, ...proxyFiles]
      .filter((file) => /import\s+\{[^}]*\brevalidatePath\b[^}]*\}\s+from\s+["']next\/cache["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("calls revalidatePath only from Server Actions or Route Handlers", () => {
    const invalidCallsites = revalidatePathFiles
      .filter((file) => !hasUseServerDirective(readFileSync(file, "utf8")) && basename(file) !== "route.ts")
      .map((file) => relative(frontendDir, file))

    expect(invalidCallsites).toEqual([])
  })

  it("does not use return revalidatePath()", () => {
    const returnedRevalidation = revalidatePathFiles
      .filter((file) => /\breturn\s+revalidatePath\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(returnedRevalidation).toEqual([])
  })

  it("uses literal, bounded paths for revalidatePath()", () => {
    const invalidPaths = revalidatePathCalls()
      .filter(({ path }) => !path || path.length > 1024 || !path.startsWith("/") || path.endsWith("/"))
      .map(({ file, path }) => `${relative(frontendDir, file)}:${path || "dynamic"}`)

    expect(invalidPaths).toEqual([])
  })

  it("does not append /page or /layout to revalidatePath paths", () => {
    const conventionSuffixes = revalidatePathCalls()
      .filter(({ path }) => /\/(?:page|layout)$/.test(path))
      .map(({ file, path }) => `${relative(frontendDir, file)}:${path}`)

    expect(conventionSuffixes).toEqual([])
  })

  it("passes type when revalidating dynamic route patterns", () => {
    const missingTypes = revalidatePathCalls()
      .filter(({ path, type }) => /\[[^\]]+\]/.test(path) && type !== "page" && type !== "layout")
      .map(({ file, path }) => `${relative(frontendDir, file)}:${path}`)

    expect(missingTypes).toEqual([])
  })

  it("omits type when revalidating concrete literal paths", () => {
    const unnecessaryTypes = revalidatePathCalls()
      .filter(({ path, type }) => !/\[[^\]]+\]/.test(path) && Boolean(type))
      .map(({ file, path }) => `${relative(frontendDir, file)}:${path}`)

    expect(unnecessaryTypes).toEqual([])
  })

  it("keeps broad layout revalidation out of routine mutations", () => {
    const broadLayoutRevalidation = revalidatePathCalls()
      .filter(({ path, type }) => path === "/" && type === "layout")
      .map(({ file }) => relative(frontendDir, file))

    expect(broadLayoutRevalidation).toEqual([])
  })

  it("uses tag revalidation for release-scoped authority data instead of page-only invalidation", () => {
    const authorityPathRevalidation = revalidatePathCalls()
      .filter(({ path }) => path.startsWith("/statutes") || path.startsWith("/sources") || path.startsWith("/provisions"))
      .map(({ file, path }) => `${relative(frontendDir, file)}:${path}`)

    expect(authorityPathRevalidation).toEqual([])
  })
})

function revalidatePathCalls() {
  return revalidatePathFiles.flatMap((file) => {
    const source = readFileSync(file, "utf8")
    return Array.from(source.matchAll(/\brevalidatePath\s*\(\s*([^,\n)]+)(?:,\s*([^)]+))?\)/g)).map((match) => {
      const rawPath = match[1]?.trim() ?? ""
      const rawType = match[2]?.trim() ?? ""
      return {
        file,
        path: stringLiteral(rawPath),
        type: stringLiteral(rawType),
      }
    })
  })
}

function stringLiteral(value: string) {
  const match = value.match(/^["']([^"']*)["']$/)
  return match?.[1] ?? ""
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
    return /\.(ts|tsx)$/.test(entry) && !/(\.d|\.test|\.test)\.tsx?$/.test(entry) ? [path] : []
  })
}
