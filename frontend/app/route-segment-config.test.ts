import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const appDir = join(process.cwd(), "app")
const proxyFile = join(process.cwd(), "proxy.ts")
const routeFiles = collectRouteFiles(appDir)

describe("route segment config policy", () => {
  it("does not use removed cache-control segment config while cacheComponents is enabled", () => {
    expect(exportsMatching(/export\s+const\s+(dynamic|revalidate|fetchCache)\b/)).toEqual([])
  })

  it("does not use dynamicParams while cacheComponents is enabled", () => {
    expect(exportsMatching(/export\s+const\s+dynamicParams\b/)).toEqual([])
  })

  it("uses the implicit Node.js runtime instead of route-level runtime exports", () => {
    expect(exportsMatching(/export\s+const\s+runtime\b/)).toEqual([])
  })

  it("does not use the removed experimental_ppr segment config", () => {
    expect(exportsMatching(/export\s+const\s+experimental_ppr\b/)).toEqual([])
  })

  it("does not use deprecated experimental edge runtime config", () => {
    expect(sourceFilesMatching(/runtime\s*=\s*["']experimental-edge["']/)).toEqual([])
  })

  it("does not use platform-specific preferredRegion hints on Railway", () => {
    expect(exportsMatching(/export\s+const\s+preferredRegion\b/)).toEqual([])
  })

  it("does not export unsupported runtime config from proxy", () => {
    expect(readFileSync(proxyFile, "utf8")).not.toMatch(/export\s+const\s+runtime\b/)
  })

  it("keeps maxDuration as the only route segment config currently in use", () => {
    expect(exportsMatching(/export\s+const\s+maxDuration\b/)).toEqual(["auth/request-access/page.tsx"])
  })
})

function exportsMatching(pattern: RegExp) {
  return routeFiles
    .filter((file) => pattern.test(readFileSync(file, "utf8")))
    .map((file) => relative(appDir, file))
}

function sourceFilesMatching(pattern: RegExp) {
  return [...routeFiles, proxyFile]
    .filter((file) => pattern.test(readFileSync(file, "utf8")))
    .map((file) => relative(process.cwd(), file))
}

function collectRouteFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectRouteFiles(path)
    return /\.(ts|tsx)$/.test(entry) && !/\.test\.(ts|tsx)$/.test(entry) ? [path] : []
  })
}
