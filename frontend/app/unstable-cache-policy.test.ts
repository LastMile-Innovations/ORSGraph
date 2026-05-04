import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)

describe("unstable_cache policy", () => {
  it("uses Cache Components instead of unstable_cache", () => {
    const legacyCacheUsage = sourceFiles
      .filter((file) => /\bunstable_cache\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(legacyCacheUsage).toEqual([])
  })

  it("keeps cacheComponents enabled for use cache directives", () => {
    const config = readFileSync(join(frontendDir, "next.config.mjs"), "utf8")

    expect(config).toContain("cacheComponents: true")
  })

  it("keeps request-time APIs outside use cache scopes", () => {
    const poisonedCacheScopes = sourceFiles
      .flatMap((file) => findPoisonedUseCacheScopes(file))
      .map((file) => relative(frontendDir, file))

    expect(poisonedCacheScopes).toEqual([])
  })

  it("requires cached authority functions to use cacheLife and authority cache tags together", () => {
    const incompleteAuthorityCaches = sourceFiles
      .filter((file) => relative(frontendDir, file) === "lib/authority-server-cache.ts")
      .flatMap((file) => findIncompleteAuthorityCacheScopes(file))
      .map((file) => relative(frontendDir, file))

    expect(incompleteAuthorityCaches).toEqual([])
  })

  it("does not use unstable_cache-style keyParts arrays for use cache functions", () => {
    const legacyKeyParts = sourceFiles
      .filter((file) => /\bkeyParts\b|cacheKeyParts|\[\s*["'][^"']+["']\s*\]\s*,\s*\{[\s\S]*?\btags\s*:/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(legacyKeyParts).toEqual([])
  })
})

function findPoisonedUseCacheScopes(file: string) {
  const source = readFileSync(file, "utf8")
  return cacheDirectiveScopes(source).some((scope) => /\b(?:cookies|headers)\s*\(/.test(scope)) ? [file] : []
}

function findIncompleteAuthorityCacheScopes(file: string) {
  const source = readFileSync(file, "utf8")
  return cacheDirectiveScopes(source).some((scope) => !/\bcacheLife\s*\(/.test(scope) || !/\b(?:tagAuthorityRead|cacheTag)\s*\(/.test(scope)) ? [file] : []
}

function cacheDirectiveScopes(source: string) {
  return Array.from(source.matchAll(/["']use cache(?:: [^"']+)?["']/g)).map((match) => {
    const start = match.index ?? 0
    return source.slice(start, blockEnd(source, source.indexOf("{", Math.max(0, source.lastIndexOf("function", start)))))
  })
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
    return /\.(ts|tsx|mjs)$/.test(entry) && !/(\.d|\.test)\.tsx?$/.test(entry) ? [path] : []
  })
}
