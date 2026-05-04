import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const nextConfig = readFileSync(join(frontendDir, "next.config.mjs"), "utf8")
const packageJson = JSON.parse(readFileSync(join(frontendDir, "package.json"), "utf8")) as {
  dependencies?: Record<string, string>
  devDependencies?: Record<string, string>
}
const sourceFiles = collectSourceFiles(frontendDir)
const productionSourceFiles = sourceFiles.filter((file) => !/\.test\.(?:ts|tsx)$/.test(file))
const cacheHandlerFiles = sourceFiles.filter((file) => /(?:^|\/)cache-handlers?\//.test(file) || /cache-handler\.(?:js|mjs|ts)$/.test(file))

describe("cacheHandlers policy", () => {
  it("uses the default in-memory cache handler unless distributed cache storage is explicitly designed", () => {
    expect(nextConfig).not.toMatch(/\bcacheHandlers\s*:/)
    expect(cacheHandlerFiles.map((file) => relative(frontendDir, file))).toEqual([])
  })

  it("does not use remote or named use cache directives without configured handlers", () => {
    const nonDefaultCacheScopes = productionSourceFiles
      .filter((file) => /["']use cache:\s*(?!private\b)[^"']+["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(nonDefaultCacheScopes).toEqual([])
  })

  it("does not expect cacheHandlers to customize private cache scopes", () => {
    const privateCacheScopes = productionSourceFiles
      .filter((file) => /["']use cache:\s*private["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(privateCacheScopes).toEqual([])
  })

  it("keeps cacheComponents enabled while relying on the default handler", () => {
    expect(nextConfig).toContain("cacheComponents: true")
    expect(nextConfig).toContain("cacheLife")
  })

  it("does not combine custom cache handlers with static export", () => {
    expect(nextConfig).not.toMatch(/\boutput\s*:\s*["']export["']/)
  })

  it("does not add external cache storage dependencies without cache handler policy coverage", () => {
    const dependencies = { ...packageJson.dependencies, ...packageJson.devDependencies }
    const externalCachePackages = ["redis", "ioredis", "memcached", "memjs", "@aws-sdk/client-dynamodb"]
    const installedExternalCaches = externalCachePackages.filter((name) => dependencies[name])

    expect(installedExternalCaches).toEqual([])
  })

  it("does not reference external cache URLs from frontend application code", () => {
    const externalCacheEnvReads = productionSourceFiles
      .filter((file) => /\bprocess\.env\.(?:REDIS|MEMCACHE|MEMCACHED|DYNAMODB|CACHE)_/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(externalCacheEnvReads).toEqual([])
  })

  it("does not implement cache handler methods outside a handler module", () => {
    const strayHandlerMethods = productionSourceFiles
      .filter((file) => !cacheHandlerFiles.includes(file))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /\basync\s+refreshTags\s*\(/.test(source) ||
          /\basync\s+getExpiration\s*\(/.test(source) ||
          /\basync\s+updateTags\s*\(/.test(source) ||
          /\bpendingEntry\b/.test(source)
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(strayHandlerMethods).toEqual([])
  })

  it("keeps cached authority functions on default use cache scopes", () => {
    const authorityCache = readFileSync(join(frontendDir, "lib/authority-server-cache.ts"), "utf8")
    const cacheDirectives = Array.from(authorityCache.matchAll(/["']use cache(?:: [^"']+)?["']/g)).map((match) => match[0])

    expect(cacheDirectives).toEqual(['"use cache"', '"use cache"', '"use cache"'])
    expect(authorityCache).not.toContain("use cache: remote")
  })

  it("continues invalidating authority data with release-scoped tags", () => {
    const authorityCache = readFileSync(join(frontendDir, "lib/authority-server-cache.ts"), "utf8")
    const proxyRoute = readFileSync(join(frontendDir, "app/api/ors/[...path]/route.ts"), "utf8")

    expect(authorityCache).toContain("authorityCacheTags(AUTHORITY_RELEASE_ID, keys)")
    expect(proxyRoute).toContain("authorityCacheTags(AUTHORITY_RELEASE_ID)")
    expect(proxyRoute).toContain('revalidateTag(tag, "max")')
  })
})

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next" || entry === "coverage") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx|mjs|js|json)$/.test(entry) ? [path] : []
  })
}
