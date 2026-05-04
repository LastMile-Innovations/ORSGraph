import { readdirSync, statSync } from "node:fs"
import { join, relative, sep } from "node:path"
import { describe, expect, it } from "vitest"

const appDir = join(process.cwd(), "app")

describe("route group policy", () => {
  it("keeps route groups from creating duplicate URL paths", () => {
    const routes = publicRouteFiles(appDir).filter((file) => !hasInterceptingSegment(file))
    const byUrl = new Map<string, string[]>()

    for (const routeFile of routes) {
      const urlPath = resolvedUrlPath(routeFile)
      byUrl.set(urlPath, [...(byUrl.get(urlPath) ?? []), relative(appDir, routeFile)])
    }

    const duplicates = [...byUrl.entries()]
      .filter(([, files]) => files.length > 1)
      .map(([urlPath, files]) => ({ urlPath, files }))

    expect(duplicates).toEqual([])
  })

  it("keeps a single top-level root layout for client-side navigation continuity", () => {
    const rootLayouts = layoutFiles(appDir).filter(isRootLayout)

    expect(rootLayouts.map((file) => relative(appDir, file))).toEqual(["layout.tsx"])
  })
})

function publicRouteFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return publicRouteFiles(path)
    return entry === "page.tsx" || entry === "route.ts" ? [path] : []
  })
}

function layoutFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return layoutFiles(path)
    return entry === "layout.tsx" ? [path] : []
  })
}

function resolvedUrlPath(routeFile: string) {
  const segments = relative(appDir, routeFile).split(sep).slice(0, -1)
  const urlSegments = segments
    .filter((segment) => !isRouteGroup(segment))
    .filter((segment) => !segment.startsWith("@"))
    .map((segment) => {
      if (segment.startsWith("(.)")) return segment.slice("(.)".length)
      if (segment.startsWith("(..)")) return segment.slice("(..)".length)
      if (segment.startsWith("(...)")) return segment.slice("(...)".length)
      return segment
    })
    .filter(Boolean)

  return `/${urlSegments.join("/")}`.replace(/\/$/, "") || "/"
}

function isRouteGroup(segment: string) {
  return /^\([^)]+\)$/.test(segment)
}

function hasInterceptingSegment(routeFile: string) {
  return relative(appDir, routeFile)
    .split(sep)
    .some((segment) => segment.startsWith("(.)") || segment.startsWith("(..)") || segment.startsWith("(...)"))
}

function isRootLayout(layoutFile: string) {
  const segments = relative(appDir, layoutFile).split(sep).slice(0, -1)
  return segments.every(isRouteGroup)
}
