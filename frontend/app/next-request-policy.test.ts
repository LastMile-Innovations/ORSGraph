import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const requestHandlerFiles = sourceFiles.filter((file) => basename(file) === "proxy.ts" || basename(file) === "route.ts")
const nextRequestFiles = sourceFiles.filter((file) => /\bNextRequest\b/.test(readFileSync(file, "utf8")))

describe("NextRequest policy", () => {
  it("imports NextRequest only from next/server", () => {
    const invalidImports = nextRequestFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return !/from\s+["']next\/server["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("uses NextRequest only from Proxy and Route Handlers", () => {
    const invalidFiles = nextRequestFiles
      .filter((file) => basename(file) !== "proxy.ts" && basename(file) !== "route.ts")
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("uses NextRequest when reading nextUrl", () => {
    const untypedNextUrlReads = requestHandlerFiles
      .filter((file) => /\b\w+\.nextUrl\b/.test(readFileSync(file, "utf8")))
      .filter((file) => !/\bNextRequest\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(untypedNextUrlReads).toEqual([])
  })

  it("does not use removed request ip or geo helpers", () => {
    const removedHelpers = sourceFiles
      .filter((file) => /\b\w+\.(?:ip|geo)\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(removedHelpers).toEqual([])
  })

  it("does not use Pages Router i18n properties on nextUrl", () => {
    const pagesRouterI18n = requestHandlerFiles
      .filter((file) => /\b\w+\.nextUrl\.(?:locale|locales|defaultLocale|domainLocale)\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(pagesRouterI18n).toEqual([])
  })

  it("does not mutate incoming request cookies", () => {
    const requestCookieMutations = sourceFiles
      .filter((file) => /\b\w+\.cookies\.(?:set|delete|clear)\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(requestCookieMutations).toEqual([])
  })

  it("clones nextUrl before mutating redirects or rewrites", () => {
    const directNextUrlMutations = requestHandlerFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /\b\w+\.nextUrl\.(?:pathname|search|hash|host|hostname|protocol)\s*=/.test(source) ||
          /\b\w+\.nextUrl\.searchParams\.(?:set|append|delete|sort)\s*\(/.test(source)
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(directNextUrlMutations).toEqual([])
  })

  it("uses nextUrl instead of reparsing request.url in NextRequest handlers", () => {
    const reparsedNextRequestUrls = nextRequestFiles
      .filter((file) => /\bnew\s+URL\s*\(\s*\w+\.url\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(reparsedNextRequestUrls).toEqual([])
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
