import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const metadataImageNames = new Set(["icon.tsx", "apple-icon.tsx", "opengraph-image.tsx", "twitter-image.tsx"])
const imageResponseFiles = sourceFiles.filter((file) => /\bImageResponse\b/.test(readFileSync(file, "utf8")))

describe("ImageResponse policy", () => {
  it("imports ImageResponse from next/og only", () => {
    const invalidImports = sourceFiles
      .filter((file) => /ImageResponse/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /from\s+["'](?:next\/server|@vercel\/og)["']/.test(source) || !/from\s+["']next\/og["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("generates framework images only from Metadata API image files", () => {
    const invalidFiles = imageResponseFiles
      .filter((file) => !metadataImageNames.has(basename(file)))
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("declares metadata image dimensions and content type for generated images", () => {
    const missingMetadata = imageResponseFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return !/export\s+const\s+size\s*=/.test(source) || !/export\s+const\s+contentType\s*=\s*["']image\/png["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(missingMetadata).toEqual([])
  })

  it("passes explicit width and height options to ImageResponse", () => {
    const missingDimensions = imageResponseFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        const options = imageResponseOptions(source)
        return !/\.\.\.size\b/.test(options) && !(/\bwidth\s*:/.test(options) && /\bheight\s*:/.test(options))
      })
      .map((file) => relative(frontendDir, file))

    expect(missingDimensions).toEqual([])
  })

  it("avoids unsupported Satori layout patterns in generated image JSX", () => {
    const unsupportedCss = imageResponseFiles
      .filter((file) => /display\s*:\s*["']grid["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(unsupportedCss).toEqual([])
  })

  it("does not ship debug ImageResponse output", () => {
    const debugImages = imageResponseFiles
      .filter((file) => /\bdebug\s*:\s*true\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(debugImages).toEqual([])
  })

  it("uses supported font formats when embedding ImageResponse fonts", () => {
    const unsupportedFonts = imageResponseFiles
      .filter((file) => /\.(?:woff2|eot|svg)["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(unsupportedFonts).toEqual([])
  })
})

function imageResponseOptions(source: string) {
  const start = source.search(/\bnew\s+ImageResponse\s*\(/)
  if (start < 0) return ""
  const comma = source.indexOf(",", start)
  const firstBrace = source.indexOf("{", comma)
  if (comma < 0 || firstBrace < 0) return ""
  return source.slice(firstBrace, blockEnd(source, firstBrace))
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
