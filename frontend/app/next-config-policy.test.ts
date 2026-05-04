import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const packageJson = JSON.parse(readFileSync(join(frontendDir, "package.json"), "utf8")) as {
  dependencies?: Record<string, string>
  devDependencies?: Record<string, string>
}
const nextConfigPath = join(frontendDir, "next.config.mjs")
const nextConfig = readFileSync(nextConfigPath, "utf8")
const sourceFiles = collectSourceFiles(frontendDir)
const testFiles = collectTestFiles(frontendDir)

describe("next.config policy", () => {
  it("keeps a single supported ESM Next config file", () => {
    const configFiles = readdirSync(frontendDir).filter((entry) => /^next\.config\.(?:js|mjs|ts|cjs|cts)$/.test(entry))

    expect(configFiles).toEqual(["next.config.mjs"])
  })

  it("uses the documented NextConfig JSDoc and default export shape", () => {
    expect(nextConfig).toMatch(/@type\s+\{import\(["']next["']\)\.NextConfig\}/)
    expect(nextConfig).toMatch(/\bconst\s+nextConfig\s*=/)
    expect(nextConfig).toMatch(/\bexport\s+default\s+nextConfig\b/)
  })

  it("does not use unsupported CommonJS config exports", () => {
    expect(nextConfig).not.toMatch(/\bmodule\.exports\b/)
    expect(nextConfig).not.toMatch(/\brequire\s*\(/)
  })

  it("keeps next.config deterministic instead of phase or async generated", () => {
    expect(nextConfig).not.toMatch(/\bPHASE_[A-Z_]+\b/)
    expect(nextConfig).not.toMatch(/\bexport\s+default\s+(?:async\s+)?\(/)
    expect(nextConfig).not.toMatch(/\bmodule\.exports\s*=\s*(?:async\s+)?\(/)
  })

  it("does not inline runtime environment or secrets through config env", () => {
    expect(nextConfig).not.toMatch(/\benv\s*:/)
    expect(nextConfig).not.toMatch(/\bprocess\.env\b/)
  })

  it("keeps the Railway app away from static export and Vercel-specific config", () => {
    expect(nextConfig).not.toMatch(/\boutput\s*:\s*["']export["']/)
    expect(nextConfig).not.toMatch(/\bVERCEL(?:_|$)/)
    expect(nextConfig).not.toMatch(/\bpreferredRegion\b/)
  })

  it("keeps experimental Next config options allow-listed", () => {
    const experimentalBody = extractObjectBody(nextConfig, "experimental")
    const experimentalKeys = Array.from(experimentalBody.matchAll(/^\s*([A-Za-z_$][\w$]*)\s*:/gm)).map((match) => match[1])

    expect(experimentalKeys).toEqual(["optimizePackageImports"])
  })

  it("optimizes only installed packages", () => {
    const installedPackages = new Set([...Object.keys(packageJson.dependencies ?? {}), ...Object.keys(packageJson.devDependencies ?? {})])
    const optimizedPackages = Array.from(nextConfig.matchAll(/["'](@[^"']+)["']/g))
      .map((match) => match[1])
      .filter((name) => name.startsWith("@radix-ui/"))
    const missingPackages = optimizedPackages.filter((name) => !installedPackages.has(name))

    expect(missingPackages).toEqual([])
  })

  it("keeps config-level headers, redirects, and rewrites covered by Next config routing tests", () => {
    const definesConfigRouting = /\b(?:headers|redirects|rewrites)\s*\(\s*\)/.test(nextConfig)
    const hasRoutingHarness = testFiles.some((file) =>
      /\bunstable_getResponseFromNextConfig\b/.test(readFileSync(file, "utf8"))
    )

    expect(definesConfigRouting ? hasRoutingHarness : true).toBe(true)
  })

  it("uses the experimental Next config response helper only from tests", () => {
    const invalidHelperImports = sourceFiles
      .filter((file) => /\bunstable_getResponseFromNextConfig\b/.test(readFileSync(file, "utf8")))
      .filter((file) => !/\.test\.(?:ts|tsx)$/.test(file))
      .map((file) => relative(frontendDir, file))

    expect(invalidHelperImports).toEqual([])
  })
})

function extractObjectBody(source: string, key: string) {
  const start = source.search(new RegExp(`\\b${key}\\s*:\\s*\\{`))
  if (start < 0) return ""

  const openBrace = source.indexOf("{", start)
  let depth = 0
  for (let index = openBrace; index < source.length; index += 1) {
    const char = source[index]
    if (char === "{") depth += 1
    if (char === "}") depth -= 1
    if (depth === 0) return source.slice(openBrace + 1, index)
  }

  return ""
}

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx|mjs)$/.test(entry) ? [path] : []
  })
}

function collectTestFiles(dir: string): string[] {
  return collectSourceFiles(dir).filter((file) => /\.test\.(?:ts|tsx)$/.test(file))
}
