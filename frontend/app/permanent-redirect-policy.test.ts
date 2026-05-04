import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const appFiles = sourceFiles.filter((file) => file.startsWith(appDir))
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const permanentRedirectFiles = appFiles.filter((file) => /\bpermanentRedirect\s*\(/.test(readFileSync(file, "utf8")))
const redirectFiles = appFiles.filter((file) => /\bredirect\s*\(/.test(readFileSync(file, "utf8")))

describe("permanentRedirect() policy", () => {
  it("imports permanentRedirect and RedirectType only from next/navigation", () => {
    const invalidImports = appFiles
      .filter((file) => /\b(?:permanentRedirect|RedirectType)\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\b(?:permanentRedirect|RedirectType)\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\b(?:permanentRedirect|RedirectType)\b[^}]*\}\s+from\s+["']next\/navigation["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("keeps permanentRedirect() out of Client Components unless explicitly reviewed", () => {
    const clientPermanentRedirects = clientFiles
      .filter((file) => /\bpermanentRedirect\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(clientPermanentRedirects).toEqual([])
  })

  it("does not use return permanentRedirect()", () => {
    const returnedPermanentRedirects = permanentRedirectFiles
      .filter((file) => /\breturn\s+permanentRedirect\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(returnedPermanentRedirects).toEqual([])
  })

  it("does not hand-roll NEXT_REDIRECT errors", () => {
    const manualRedirectErrors = sourceFiles
      .filter((file) => /\bNEXT_REDIRECT\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(manualRedirectErrors).toEqual([])
  })

  it("uses notFound() instead of permanentRedirect() for missing resources", () => {
    const missingResourcePermanentRedirects = permanentRedirectFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bif\s*\([^)]*![^)]*\)\s*\{?\s*permanentRedirect\s*\(/.test(source) || /\bif\s*\([^)]*\.ok\s*===\s*false[^)]*\)\s*\{?\s*permanentRedirect\s*\(/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(missingResourcePermanentRedirects).toEqual([])
  })

  it("uses permanentRedirect() for static legacy alias pages", () => {
    const temporaryStaticAliases = redirectFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return basename(file) === "page.tsx" && /^\s*import\s+\{\s*redirect\s*\}\s+from\s+["']next\/navigation["']/m.test(source) && /export\s+default\s+function\s+\w+\s*\(\s*\)\s*\{[\s\S]*?\bredirect\s*\(\s*["']\//.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(temporaryStaticAliases).toEqual([])
  })

  it("uses RedirectType constants instead of literal redirect history types", () => {
    const literalRedirectTypes = appFiles
      .filter((file) => /\bpermanentRedirect\s*\([^,\n]+,\s*["'](?:push|replace)["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(literalRedirectTypes).toEqual([])
  })
})

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
