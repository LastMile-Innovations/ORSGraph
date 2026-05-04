import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const appDir = join(process.cwd(), "app")
const routeFiles = collectRouteFiles(appDir)

describe("route segment config policy", () => {
  it("does not use dynamicParams while cacheComponents is enabled", () => {
    expect(exportsMatching(/export\s+const\s+dynamicParams\b/)).toEqual([])
  })

  it("does not opt route segments into the edge runtime", () => {
    expect(exportsMatching(/export\s+const\s+runtime\s*=\s*["']edge["']/)).toEqual([])
  })

  it("does not use platform-specific preferredRegion hints on Railway", () => {
    expect(exportsMatching(/export\s+const\s+preferredRegion\b/)).toEqual([])
  })
})

function exportsMatching(pattern: RegExp) {
  return routeFiles
    .filter((file) => pattern.test(readFileSync(file, "utf8")))
    .map((file) => relative(appDir, file))
}

function collectRouteFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectRouteFiles(path)
    return /\.(ts|tsx)$/.test(entry) && !/\.test\.(ts|tsx)$/.test(entry) ? [path] : []
  })
}
