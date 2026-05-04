import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const requestHandlerFiles = sourceFiles.filter((file) => basename(file) === "proxy.ts" || basename(file) === "route.ts")
const userAgentHelperFiles = sourceFiles.filter((file) => {
  const source = readFileSync(file, "utf8")
  return /\buserAgent\s*\(/.test(source) || /import\s*\{[^}]*\buserAgent\b[^}]*\}\s*from\s*["']next\/server["']/.test(source)
})

describe("userAgent policy", () => {
  it("imports userAgent only from next/server", () => {
    const invalidImports = sourceFiles
      .filter((file) => /import\s*\{[^}]*\buserAgent\b[^}]*\}/.test(readFileSync(file, "utf8")))
      .filter((file) => !/from\s+["']next\/server["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("uses the userAgent helper only from Proxy and Route Handlers", () => {
    const invalidFiles = userAgentHelperFiles
      .filter((file) => basename(file) !== "proxy.ts" && basename(file) !== "route.ts")
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("passes typed NextRequest values into userAgent", () => {
    const untypedUsage = userAgentHelperFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\buserAgent\s*\(/.test(source) && !/\bNextRequest\b/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(untypedUsage).toEqual([])
  })

  it("does not parse user-agent headers directly in application code", () => {
    const directUserAgentReads = sourceFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /\bheaders\s*\(\s*\)\.get\s*\(\s*["']user-agent["']\s*\)/i.test(source) ||
          /\bawait\s+headers\s*\(\s*\)\s*\)\.get\s*\(\s*["']user-agent["']\s*\)/i.test(source) ||
          /\b\w+\.headers\.get\s*\(\s*["']user-agent["']\s*\)/i.test(source)
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(directUserAgentReads).toEqual([])
  })

  it("does not read navigator.userAgent from Client Components", () => {
    const browserUserAgentReads = sourceFiles
      .filter((file) => /\bnavigator\.userAgent\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(browserUserAgentReads).toEqual([])
  })

  it("does not use bot detection as an authorization control", () => {
    const botAccessControls = requestHandlerFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /\bisBot\b[\s\S]{0,120}\b(?:redirect|rewrite|notFound)\s*\(/.test(source) ||
          /\bisBot\b[\s\S]{0,120}\bstatus\s*:\s*(?:401|403|404)\b/.test(source)
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(botAccessControls).toEqual([])
  })

  it("does not persist user-agent derived values into cookies or response headers", () => {
    const persistedUserAgentState = requestHandlerFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /\b(?:browser|device|engine|os|cpu|isBot)\b[\s\S]{0,160}\b(?:cookies\.set|headers\.set)\s*\(/.test(source) ||
          /\b(?:cookies\.set|headers\.set)\s*\([\s\S]{0,160}\b(?:browser|device|engine|os|cpu|isBot)\b/.test(source)
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(persistedUserAgentState).toEqual([])
  })

  it("does not use user-agent data as cache tags", () => {
    const userAgentCacheTags = sourceFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /\b(?:browser|device|engine|os|cpu|isBot)\b[\s\S]{0,160}\bcacheTag\s*\(/.test(source) ||
          /\bnext\s*:\s*\{[\s\S]{0,160}\btags\s*:\s*\[[\s\S]{0,160}\b(?:browser|device|engine|os|cpu|isBot)\b/.test(source)
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(userAgentCacheTags).toEqual([])
  })

  it("does not rewrite routes based on user-agent device data", () => {
    const deviceBasedRewrites = requestHandlerFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bdevice\b[\s\S]{0,240}\bNextResponse\.rewrite\s*\(/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(deviceBasedRewrites).toEqual([])
  })

  it("allows robots metadata userAgent fields without treating them as helper usage", () => {
    const robotsMetadataFiles = sourceFiles
      .filter((file) => /userAgent\s*:/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(robotsMetadataFiles).toContain("app/robots.ts")
  })
})

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx)$/.test(entry) && !/(\.d|\.test)\.tsx?$/.test(entry) ? [path] : []
  })
}
