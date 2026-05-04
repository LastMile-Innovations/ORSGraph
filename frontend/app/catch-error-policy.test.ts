import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)

describe("component error boundary policy", () => {
  it("does not keep custom React class error boundaries alongside Next error handling", () => {
    const customClassBoundaries = filesMatching(/class\s+\w+\s+extends\s+(?:React\.)?Component|componentDidCatch|getDerivedStateFromError/)

    expect(customClassBoundaries).toEqual([])
  })

  it("uses unstable_catchError only from Client Components", () => {
    const nonClientCatchError = sourceFiles
      .filter((file) => /unstable_catchError|from\s+["']next\/error["']/.test(readFileSync(file, "utf8")))
      .filter((file) => !isClientFile(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(nonClientCatchError).toEqual([])
  })

  it("does not wrap route error convention files with unstable_catchError", () => {
    const wrappedRouteErrors = sourceFiles
      .filter((file) => file.startsWith(join(frontendDir, "app")))
      .filter((file) => basename(file) === "error.tsx" || basename(file) === "global-error.tsx")
      .filter((file) => /unstable_catchError|from\s+["']next\/error["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(wrappedRouteErrors).toEqual([])
  })

  it("keeps route error recovery wired to unstable_retry instead of reset", () => {
    const routeErrorFiles = sourceFiles
      .filter((file) => file.startsWith(join(frontendDir, "app")))
      .filter((file) => basename(file) === "error.tsx" || basename(file) === "global-error.tsx")
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return source.includes("unstable_retry") && /\breset\s*:|\breset\s*\(\s*\)/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(routeErrorFiles).toEqual([])
  })
})

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
