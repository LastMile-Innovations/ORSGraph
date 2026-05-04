import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const appFiles = collectSourceFiles(appDir)

describe("generateSitemaps policy", () => {
  it("uses the exact generateSitemaps API name", () => {
    const misnamedGenerators = appFiles
      .filter((file) => /\bgenerateSiteMaps\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(misnamedGenerators).toEqual([])
  })

  it("declares generateSitemaps only from sitemap metadata files", () => {
    const invalidFiles = appFiles
      .filter((file) => /\bgenerateSitemaps\b/.test(readFileSync(file, "utf8")))
      .filter((file) => basename(file) !== "sitemap.ts")
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("returns sitemap descriptors with id fields", () => {
    const missingIds = appFiles
      .filter((file) => /\bgenerateSitemaps\b/.test(readFileSync(file, "utf8")))
      .filter((file) => !/\bid\s*:/.test(generateSitemapsBody(readFileSync(file, "utf8"))))
      .map((file) => relative(frontendDir, file))

    expect(missingIds).toEqual([])
  })

  it("awaits promised sitemap ids in split sitemap functions", () => {
    const syncSitemapIds = appFiles
      .filter((file) => /\bgenerateSitemaps\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const body = defaultSitemapBody(readFileSync(file, "utf8"))
        return body && !/\bawait\s+(?:props\.)?id\b/.test(body)
      })
      .map((file) => relative(frontendDir, file))

    expect(syncSitemapIds).toEqual([])
  })

  it("does not use build-time timestamps in sitemap entries", () => {
    const buildTimeLastModified = appFiles
      .filter((file) => basename(file) === "sitemap.ts")
      .filter((file) => /\blastModified\s*:\s*new\s+Date\s*\(\s*\)/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(buildTimeLastModified).toEqual([])
  })
})

function generateSitemapsBody(source: string) {
  const start = source.search(/export\s+(?:async\s+)?function\s+generateSitemaps\b/)
  if (start < 0) return ""
  return source.slice(start, blockEnd(source, source.indexOf("{", start)))
}

function defaultSitemapBody(source: string) {
  const start = source.search(/export\s+default\s+(?:async\s+)?function\s+sitemap\b/)
  if (start < 0) return ""
  return source.slice(start, blockEnd(source, source.indexOf("{", start)))
}

function blockEnd(source: string, start: number) {
  if (start < 0) return source.length

  let depth = 0
  for (let index = start; index < source.length; index += 1) {
    const char = source[index]
    if (char === "{") depth += 1
    if (char === "}") {
      depth -= 1
      if (depth === 0) return index + 1
    }
  }
  return source.length
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
