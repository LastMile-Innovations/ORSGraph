import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const appFiles = collectSourceFiles(join(frontendDir, "app"))

describe("connection rendering policy", () => {
  it("uses connection() instead of deprecated no-store helpers", () => {
    const deprecatedNoStore = sourceFiles
      .filter((file) => /\b(?:unstable_noStore|noStore)\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(deprecatedNoStore).toEqual([])
  })

  it("awaits connection() wherever it is used", () => {
    const unawaitedConnection = sourceFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bconnection\s*\(/.test(source) && !/\bawait\s+connection\s*\(\s*\)/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(unawaitedConnection).toEqual([])
  })

  it("keeps connection() out of Client Components", () => {
    const clientConnection = sourceFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return hasUseClientDirective(source) && /\bconnection\s*\(/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(clientConnection).toEqual([])
  })

  it("does not prerender request-varying values without an explicit connection() boundary", () => {
    const implicitRuntimeValues = appFiles
      .filter((file) => isPrerenderedConventionFile(file))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return hasImplicitRuntimeValue(source) && !/\bawait\s+connection\s*\(\s*\)/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(implicitRuntimeValues).toEqual([])
  })
})

function hasUseClientDirective(source: string) {
  return /^\s*["']use client["']/.test(source)
}

function hasImplicitRuntimeValue(source: string) {
  return /\b(?:Math\.random|Date\.now)\s*\(/.test(source) || /\bnew\s+Date\s*\(\s*\)/.test(source)
}

function isPrerenderedConventionFile(file: string) {
  const name = basename(file)
  if (/\.test\.(ts|tsx)$/.test(name) || hasUseClientDirective(readFileSync(file, "utf8"))) return false
  if (name === "route.ts" || name === "actions.ts") return false
  return /^(page|layout|template|default|loading|not-found|error|global-error|robots|sitemap)\.(ts|tsx)$/.test(name)
}

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx)$/.test(entry) && !/(\.d|\.test)\.ts$/.test(entry) && !/\.test\.tsx$/.test(entry) ? [path] : []
  })
}
