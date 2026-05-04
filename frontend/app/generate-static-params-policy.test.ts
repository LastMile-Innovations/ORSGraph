import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const appFiles = collectSourceFiles(appDir)
const generateStaticParamFiles = appFiles.filter((file) =>
  /\bexport\s+(?:async\s+)?function\s+generateStaticParams\b/.test(readFileSync(file, "utf8")),
)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))

describe("generateStaticParams policy", () => {
  it("declares generateStaticParams only from dynamic page, layout, or route files", () => {
    const invalidFiles = generateStaticParamFiles
      .filter((file) => {
        const name = basename(file)
        const relativePath = relative(appDir, file)
        const allowedConvention = name === "page.tsx" || name === "layout.tsx" || name === "route.ts"
        return !allowedConvention || !/\[[^/]+\]/.test(relativePath)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("keeps generateStaticParams out of Client Components", () => {
    const clientStaticParamGenerators = clientFiles
      .filter((file) => /\bexport\s+(?:async\s+)?function\s+generateStaticParams\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(clientStaticParamGenerators).toEqual([])
  })

  it("does not rely on empty generateStaticParams with Cache Components enabled", () => {
    const emptyStaticParamLists = generateStaticParamFiles
      .filter((file) => {
        const body = generateStaticParamsBody(readFileSync(file, "utf8"))
        return /\breturn\s+\[\s*\]/.test(body) || /\breturn\s+Array\s*\.\s*from\s*\(\s*\{\s*length\s*:\s*0\s*[,}]/.test(body)
      })
      .map((file) => relative(frontendDir, file))

    expect(emptyStaticParamLists).toEqual([])
  })

  it("keeps runtime fallback controls out of Cache Components routes", () => {
    const fallbackRouteControls = appFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bexport\s+const\s+dynamicParams\b/.test(source) || /\bforce-static\b/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(fallbackRouteControls).toEqual([])
  })

  it("does not await parent params inside generateStaticParams", () => {
    const awaitedParentParams = generateStaticParamFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        const signature = generateStaticParamsSignature(source)
        const body = generateStaticParamsBody(source)
        return signature.includes("params") && /\bawait\s+(?:options\.)?params\b/.test(body)
      })
      .map((file) => relative(frontendDir, file))

    expect(awaitedParentParams).toEqual([])
  })

  it("returns objects keyed by route segment names when params are literal", () => {
    const missingLiteralSegmentKeys = generateStaticParamFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        const body = generateStaticParamsBody(source)
        const literalReturn = body.match(/\breturn\s+\[([\s\S]*?)\]/)
        if (!literalReturn) return false

        const segmentKeys = dynamicSegmentKeys(relative(appDir, file))
        return segmentKeys.length > 0 && !segmentKeys.some((key) => new RegExp(`\\b${escapeRegExp(key)}\\s*:`).test(literalReturn[1]))
      })
      .map((file) => relative(frontendDir, file))

    expect(missingLiteralSegmentKeys).toEqual([])
  })
})

function generateStaticParamsSignature(source: string) {
  const match = source.match(/export\s+(?:async\s+)?function\s+generateStaticParams\s*\(([\s\S]*?)\)\s*(?::[^{]+)?{/)
  return match?.[1] ?? ""
}

function generateStaticParamsBody(source: string) {
  const start = source.search(/export\s+(?:async\s+)?function\s+generateStaticParams\b/)
  if (start < 0) return ""
  return source.slice(start, blockEnd(source, source.indexOf("{", start)))
}

function dynamicSegmentKeys(path: string) {
  const matches = path.matchAll(/\[(?:\.\.\.)?([^\]]+)\]/g)
  return Array.from(matches, (match) => match[1])
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

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
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
