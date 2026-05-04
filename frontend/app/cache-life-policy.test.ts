import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const allowedCacheLifeProfiles = new Set(["authorityShell", "authorityDetail"])

describe("cacheLife policy", () => {
  it("keeps cacheComponents enabled with named authority cache profiles", () => {
    const config = readFileSync(join(frontendDir, "next.config.mjs"), "utf8")

    expect(config).toContain("cacheComponents: true")
    expect(config).toContain("authorityShell")
    expect(config).toContain("authorityDetail")
  })

  it("uses cacheLife only inside explicit use cache scopes", () => {
    const unscopedCacheLife = sourceFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bcacheLife\s*\(/.test(source) && !/["']use cache(?:: [^"']+)?["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(unscopedCacheLife).toEqual([])
  })

  it("gives every use cache scope an explicit cacheLife", () => {
    const missingCacheLife = sourceFiles
      .flatMap((file) => findUseCacheScopesWithoutCacheLife(file))
      .map((file) => relative(frontendDir, file))

    expect(missingCacheLife).toEqual([])
  })

  it("uses semantic cacheLife profiles and avoids short-lived dynamic-hole profiles", () => {
    const invalidProfiles = sourceFiles.flatMap((file) => {
      const source = readFileSync(file, "utf8")
      return Array.from(source.matchAll(/\bcacheLife\s*\(\s*["']([^"']+)["']/g))
        .map((match) => match[1])
        .filter((profile) => profile && !allowedCacheLifeProfiles.has(profile))
        .map((profile) => `${relative(frontendDir, file)}:${profile}`)
    })

    expect(invalidProfiles).toEqual([])
  })
})

function findUseCacheScopesWithoutCacheLife(file: string) {
  const source = readFileSync(file, "utf8")
  return Array.from(source.matchAll(/["']use cache(?:: [^"']+)?["']/g)).flatMap((match) => {
    const start = match.index ?? 0
    const end = source.indexOf("\n}", start)
    const scope = source.slice(start, end >= 0 ? end + 2 : source.length)
    return /\bcacheLife\s*\(/.test(scope) ? [] : [file]
  })
}

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx|mjs)$/.test(entry) && !/\.test\.(ts|tsx)$/.test(entry) ? [path] : []
  })
}
