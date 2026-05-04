import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const metadataImageNames = new Set(["icon.tsx", "apple-icon.tsx", "opengraph-image.tsx", "twitter-image.tsx"])

describe("generateImageMetadata policy", () => {
  it("uses generateImageMetadata only from Metadata API image files", () => {
    const invalidFiles = sourceFiles
      .filter((file) => /\bgenerateImageMetadata\b/.test(readFileSync(file, "utf8")))
      .filter((file) => !metadataImageNames.has(basename(file)))
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("keeps ImageResponse generation colocated with metadata image conventions", () => {
    const invalidFiles = sourceFiles
      .filter((file) => /\bImageResponse\b/.test(readFileSync(file, "utf8")))
      .filter((file) => !metadataImageNames.has(basename(file)))
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("requires generated image metadata items to include ids", () => {
    const missingIds = sourceFiles
      .filter((file) => /\bgenerateImageMetadata\b/.test(readFileSync(file, "utf8")))
      .filter((file) => !/\bid\s*:/.test(generateImageMetadataBody(readFileSync(file, "utf8"))))
      .map((file) => relative(frontendDir, file))

    expect(missingIds).toEqual([])
  })

  it("awaits promised id and params props in image generation functions", () => {
    const syncPromisedProps = sourceFiles
      .filter((file) => metadataImageNames.has(basename(file)))
      .flatMap((file) => {
        const source = readFileSync(file, "utf8")
        const body = defaultExportBody(source)
        if (!body) return []

        const violations: string[] = []
        if (defaultExportSignature(source).includes("id") && !/\bawait\s+id\b/.test(body)) {
          violations.push(`${relative(frontendDir, file)}:id`)
        }
        if (defaultExportSignature(source).includes("params") && !/\bawait\s+params\b/.test(body)) {
          violations.push(`${relative(frontendDir, file)}:params`)
        }
        return violations
      })

    expect(syncPromisedProps).toEqual([])
  })
})

function generateImageMetadataBody(source: string) {
  const start = source.search(/export\s+(?:async\s+)?function\s+generateImageMetadata\b/)
  if (start < 0) return ""
  return source.slice(start, blockEnd(source, source.indexOf("{", start)))
}

function defaultExportSignature(source: string) {
  const match = source.match(/export\s+default\s+(?:async\s+)?function\s+\w*\s*\(([\s\S]*?)\)\s*{/)
  return match?.[1] ?? ""
}

function defaultExportBody(source: string) {
  const start = source.search(/export\s+default\s+(?:async\s+)?function\b/)
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
