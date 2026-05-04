import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const appFiles = collectSourceFiles(appDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))

describe("generateMetadata policy", () => {
  it("keeps metadata exports out of Client Components", () => {
    const clientMetadataExports = clientFiles
      .filter((file) => /export\s+(?:const\s+metadata|(?:async\s+)?function\s+generateMetadata)\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(clientMetadataExports).toEqual([])
  })

  it("does not export both metadata and generateMetadata from the same segment", () => {
    const duplicateMetadataExports = appFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /export\s+const\s+metadata\b/.test(source) && /export\s+(?:async\s+)?function\s+generateMetadata\b/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(duplicateMetadataExports).toEqual([])
  })

  it("uses static metadata objects when metadata does not need dynamic generation", () => {
    const staticGenerateMetadata = appFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        const body = generateMetadataBody(source)
        return body && !/await\s+(?:params|searchParams|parent)\b/.test(body) && !/\b(?:fetch|cookies|headers|connection)\s*\(/.test(body)
      })
      .map((file) => relative(frontendDir, file))

    expect(staticGenerateMetadata).toEqual([])
  })

  it("awaits promised generateMetadata props and parent metadata", () => {
    const syncPromisedMetadataProps = appFiles.flatMap((file) => {
      const source = readFileSync(file, "utf8")
      const body = generateMetadataBody(source)
      if (!body) return []

      const signature = generateMetadataSignature(source)
      const violations: string[] = []
      if (signature.includes("params") && !/\bawait\s+params\b/.test(body)) {
        violations.push(`${relative(frontendDir, file)}:params`)
      }
      if (signature.includes("searchParams") && !/\bawait\s+searchParams\b/.test(body)) {
        violations.push(`${relative(frontendDir, file)}:searchParams`)
      }
      if (signature.includes("parent") && !/\bawait\s+parent\b/.test(body)) {
        violations.push(`${relative(frontendDir, file)}:parent`)
      }
      return violations
    })

    expect(syncPromisedMetadataProps).toEqual([])
  })

  it("keeps deprecated metadata fields in viewport configuration instead of metadata", () => {
    const deprecatedMetadataFields = appFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        const body = metadataExportBody(source)
        return /\b(?:themeColor|colorScheme|viewport)\s*:/.test(body)
      })
      .map((file) => relative(frontendDir, file))

    expect(deprecatedMetadataFields).toEqual([])
  })

  it("sets a Railway-aware metadataBase in the root layout", () => {
    const layout = readFileSync(join(appDir, "layout.tsx"), "utf8")
    const metadata = readFileSync(join(appDir, "metadata.ts"), "utf8")

    expect(layout).toContain("metadataBase: new URL(siteOrigin())")
    expect(metadata).toContain("RAILWAY_PUBLIC_DOMAIN")
    expect(metadata).not.toContain("VERCEL_URL")
  })

  it("does not preserve scaffold generator metadata", () => {
    const scaffoldGeneratorMetadata = appFiles
      .filter((file) => /generator\s*:\s*["']v0\.app["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(scaffoldGeneratorMetadata).toEqual([])
  })
})

function generateMetadataSignature(source: string) {
  const match = source.match(/export\s+(?:async\s+)?function\s+generateMetadata\s*\(([\s\S]*?)\)\s*(?::[^{]+)?{/)
  return match?.[1] ?? ""
}

function generateMetadataBody(source: string) {
  const start = source.search(/export\s+(?:async\s+)?function\s+generateMetadata\b/)
  if (start < 0) return ""
  return source.slice(start, blockEnd(source, source.indexOf("{", start)))
}

function metadataExportBody(source: string) {
  const start = source.search(/export\s+const\s+metadata\b/)
  if (start < 0) return ""
  const statementEnd = source.indexOf("\n}\n", start)
  return source.slice(start, statementEnd >= 0 ? statementEnd + 3 : source.length)
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
