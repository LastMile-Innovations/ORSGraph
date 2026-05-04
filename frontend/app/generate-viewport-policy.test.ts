import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const appFiles = collectSourceFiles(appDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))

describe("generateViewport policy", () => {
  it("keeps viewport exports out of Client Components", () => {
    const clientViewportExports = clientFiles
      .filter((file) => /export\s+(?:const\s+viewport|(?:async\s+)?function\s+generateViewport)\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(clientViewportExports).toEqual([])
  })

  it("does not export both viewport and generateViewport from the same segment", () => {
    const duplicateViewportExports = appFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /export\s+const\s+viewport\b/.test(source) && /export\s+(?:async\s+)?function\s+generateViewport\b/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(duplicateViewportExports).toEqual([])
  })

  it("uses static viewport objects when viewport does not need dynamic generation", () => {
    const staticGenerateViewport = appFiles
      .filter((file) => {
        const body = generateViewportBody(readFileSync(file, "utf8"))
        return body && !/\b(?:params|searchParams)\b/.test(body) && !/\b(?:fetch|cookies|headers|connection)\s*\(/.test(body)
      })
      .map((file) => relative(frontendDir, file))

    expect(staticGenerateViewport).toEqual([])
  })

  it("does not make viewport depend on request-time APIs", () => {
    const requestTimeViewport = appFiles
      .filter((file) => /\b(?:cookies|headers|connection)\s*\(/.test(generateViewportBody(readFileSync(file, "utf8"))))
      .map((file) => relative(frontendDir, file))

    expect(requestTimeViewport).toEqual([])
  })

  it("uses cache when generateViewport fetches external data", () => {
    const uncachedViewportFetches = appFiles
      .filter((file) => {
        const body = generateViewportBody(readFileSync(file, "utf8"))
        return /\bfetch\s*\(/.test(body) && !/["']use cache["']/.test(body)
      })
      .map((file) => relative(frontendDir, file))

    expect(uncachedViewportFetches).toEqual([])
  })

  it("keeps themeColor and colorScheme in viewport exports instead of metadata", () => {
    const layout = readFileSync(join(appDir, "layout.tsx"), "utf8")

    expect(layout).toContain("export const viewport")
    expect(viewportExportBody(layout)).toContain("themeColor")
    expect(viewportExportBody(layout)).toContain("colorScheme")
    expect(metadataExportBody(layout)).not.toMatch(/\b(?:themeColor|colorScheme|viewport)\s*:/)
  })
})

function generateViewportBody(source: string) {
  const start = source.search(/export\s+(?:async\s+)?function\s+generateViewport\b/)
  if (start < 0) return ""
  return source.slice(start, blockEnd(source, source.indexOf("{", start)))
}

function viewportExportBody(source: string) {
  return exportConstBody(source, "viewport")
}

function metadataExportBody(source: string) {
  return exportConstBody(source, "metadata")
}

function exportConstBody(source: string, name: string) {
  const start = source.search(new RegExp(`export\\s+const\\s+${name}\\b`))
  if (start < 0) return ""
  const firstBrace = source.indexOf("{", start)
  return source.slice(start, blockEnd(source, firstBrace))
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

function hasUseClientDirective(source: string) {
  return /^\s*["']use client["']/.test(source)
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
