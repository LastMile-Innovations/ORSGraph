import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(appDir)

describe("auth interrupt convention policy", () => {
  it("does not enable experimental auth interrupts for the production Railway app", () => {
    const nextConfig = readFileSync(join(frontendDir, "next.config.mjs"), "utf8")

    expect(nextConfig).not.toMatch(/\bauthInterrupts\s*:\s*true\b/)
  })

  it("does not add unauthorized convention files before auth interrupts are production-ready", () => {
    const unauthorizedFiles = sourceFiles
      .filter((file) => /(^|\/)unauthorized\.(ts|tsx|js|jsx)$/.test(file))
      .map((file) => relative(appDir, file))

    expect(unauthorizedFiles).toEqual([])
  })

  it("does not add forbidden convention files before auth interrupts are production-ready", () => {
    const forbiddenFiles = sourceFiles
      .filter((file) => /(^|\/)forbidden\.(ts|tsx|js|jsx)$/.test(file))
      .map((file) => relative(appDir, file))

    expect(forbiddenFiles).toEqual([])
  })

  it("keeps unauthenticated route handling in proxy and auth pages instead of unauthorized interrupts", () => {
    const unauthorizedCalls = sourceFiles
      .filter((file) => /\bunauthorized\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(appDir, file))

    expect(unauthorizedCalls).toEqual([])
  })

  it("keeps authorization failures explicit instead of using experimental forbidden interrupts", () => {
    const forbiddenCalls = sourceFiles
      .filter((file) => /\bforbidden\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(appDir, file))

    expect(forbiddenCalls).toEqual([])
  })
})

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx)$/.test(entry) && !/\.test\.(ts|tsx)$/.test(entry) ? [path] : []
  })
}
