import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const appFiles = sourceFiles.filter((file) => file.startsWith(join(frontendDir, "app")))
const clientFiles = sourceFiles.filter((file) => isClientFile(readFileSync(file, "utf8")))

describe("after() policy", () => {
  it("uses the stable after API instead of unstable_after", () => {
    expect(filesMatching(/\bunstable_after\b/)).toEqual([])
  })

  it("does not call after from Client Components", () => {
    const clientAfterUsage = clientFiles
      .filter((file) => /\bafter\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(clientAfterUsage).toEqual([])
  })

  it("does not read request APIs inside Server Component after callbacks", () => {
    const unsafeServerComponentUsage = serverComponentFiles()
      .filter((file) => hasAfterCallbackRequestApiRead(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(unsafeServerComponentUsage).toEqual([])
  })

  it("does not install custom @next/request-context shims for Railway Node hosting", () => {
    expect(filesMatching(/Symbol\.for\(["']@next\/request-context["']\)/)).toEqual([])
  })
})

function hasAfterCallbackRequestApiRead(content: string) {
  const afterCallbackPattern = /\bafter\s*\(\s*(?:async\s*)?\([^)]*\)\s*=>\s*{[\s\S]*?(?:\bcookies\s*\(|\bheaders\s*\()/m
  return afterCallbackPattern.test(content)
}

function serverComponentFiles() {
  return appFiles.filter((file) => {
    if (isClientFile(readFileSync(file, "utf8"))) return false
    if (basename(file) === "route.ts") return false
    if (basename(file) === "actions.ts") return false
    return /\.(ts|tsx)$/.test(file)
  })
}

function filesMatching(pattern: RegExp) {
  return sourceFiles
    .filter((file) => pattern.test(readFileSync(file, "utf8")))
    .map((file) => relative(frontendDir, file))
}

function isClientFile(content: string) {
  return /^\s*["']use client["']/.test(content)
}

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx)$/.test(entry) && !/\.test\.(ts|tsx)$/.test(entry) ? [path] : []
  })
}
