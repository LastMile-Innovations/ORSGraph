import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const appFiles = sourceFiles.filter((file) => file.startsWith(appDir))
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const redirectFiles = appFiles.filter((file) => /\bredirect\s*\(/.test(readFileSync(file, "utf8")))

describe("redirect() policy", () => {
  it("imports redirect and RedirectType only from next/navigation", () => {
    const invalidImports = appFiles
      .filter((file) => /\b(?:redirect|RedirectType)\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\b(?:redirect|RedirectType)\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\b(?:redirect|RedirectType)\b[^}]*\}\s+from\s+["']next\/navigation["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("does not use return redirect()", () => {
    const returnedRedirects = redirectFiles
      .filter((file) => /\breturn\s+redirect\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(returnedRedirects).toEqual([])
  })

  it("does not hand-roll NEXT_REDIRECT errors", () => {
    const manualRedirectErrors = sourceFiles
      .filter((file) => /\bNEXT_REDIRECT\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(manualRedirectErrors).toEqual([])
  })

  it("does not call redirect() inside try blocks", () => {
    const redirectInsideTry = redirectFiles
      .filter((file) => hasRedirectInsideTry(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(redirectInsideTry).toEqual([])
  })

  it("keeps Client Component redirects out of event handlers and effects", () => {
    const imperativeClientRedirects = clientFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /\bon[A-Z][A-Za-z]*\s*=\s*\{[\s\S]*?\bredirect\s*\(/.test(source) ||
          /\buseEffect\s*\([\s\S]*?\bredirect\s*\(/.test(source)
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(imperativeClientRedirects).toEqual([])
  })

  it("uses notFound() instead of redirect() for missing resources", () => {
    const missingResourceRedirects = redirectFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bif\s*\([^)]*![^)]*\)\s*\{?\s*redirect\s*\(/.test(source) || /\bif\s*\([^)]*\.ok\s*===\s*false[^)]*\)\s*\{?\s*redirect\s*\(/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(missingResourceRedirects).toEqual([])
  })

  it("keeps static legacy aliases on permanentRedirect()", () => {
    const temporaryStaticAliases = redirectFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return basename(file) === "page.tsx" && /export\s+default\s+function\s+\w+\s*\(\s*\)\s*\{[\s\S]*?\bredirect\s*\(\s*["']\//.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(temporaryStaticAliases).toEqual([])
  })

  it("uses RedirectType constants instead of literal redirect history types", () => {
    const literalRedirectTypes = redirectFiles
      .filter((file) => /\bredirect\s*\([^,\n]+,\s*["'](?:push|replace)["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(literalRedirectTypes).toEqual([])
  })
})

function hasRedirectInsideTry(source: string) {
  let start = source.search(/\btry\s*\{/)
  while (start >= 0) {
    const bodyStart = source.indexOf("{", start)
    const body = source.slice(bodyStart, blockEnd(source, bodyStart))
    if (/\bredirect\s*\(/.test(body)) return true
    start = source.slice(start + 1).search(/\btry\s*\{/)
    if (start >= 0) start += bodyStart + 1
  }
  return false
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
