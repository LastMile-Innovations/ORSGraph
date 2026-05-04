import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const sourceFiles = collectSourceFiles(frontendDir)
const responseHandlerFiles = sourceFiles.filter((file) => basename(file) === "proxy.ts" || basename(file) === "route.ts")
const nextResponseFiles = sourceFiles.filter((file) => /\bNextResponse\b/.test(readFileSync(file, "utf8")))

describe("NextResponse policy", () => {
  it("imports NextResponse only from next/server", () => {
    const invalidImports = nextResponseFiles
      .filter((file) => !/from\s+["']next\/server["']/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("uses NextResponse only from Proxy and Route Handlers", () => {
    const invalidFiles = nextResponseFiles
      .filter((file) => basename(file) !== "proxy.ts" && basename(file) !== "route.ts")
      .map((file) => relative(frontendDir, file))

    expect(invalidFiles).toEqual([])
  })

  it("does not expose proxy request headers to clients with NextResponse.next({ headers })", () => {
    const leakedProxyHeaders = responseHandlerFiles
      .filter((file) => /\bNextResponse\.next\s*\(\s*\{\s*headers\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(leakedProxyHeaders).toEqual([])
  })

  it("does not blindly forward all incoming request headers upstream", () => {
    const copiedIncomingHeaders = responseHandlerFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /\bnew\s+Headers\s*\(\s*(?:request|req)\.headers\s*\)/.test(source) ||
          /\brequest\s*:\s*\{\s*headers\s*:\s*(?:request|req)\.headers\s*\}/.test(source)
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(copiedIncomingHeaders).toEqual([])
  })

  it("redirects and rewrites with URL objects instead of string paths", () => {
    const stringRedirectsOrRewrites = responseHandlerFiles
      .filter((file) => /\bNextResponse\.(?:redirect|rewrite)\s*\(\s*["'`]/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(stringRedirectsOrRewrites).toEqual([])
  })

  it("uses cloned nextUrl values for same-origin redirects and rewrites", () => {
    const reparsedRedirectUrls = responseHandlerFiles
      .filter((file) => /\bNextResponse\.(?:redirect|rewrite)\s*\(\s*new\s+URL\s*\([^)]*\brequest\.url\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(reparsedRedirectUrls).toEqual([])
  })

  it("sets explicit statuses for JSON error responses", () => {
    const implicitErrorStatuses = responseHandlerFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\b(?:NextResponse|Response)\.json\s*\(\s*\{[\s\S]*?\berror\s*:/.test(source) && !/\bstatus\s*:\s*[45]\d\d\b/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(implicitErrorStatuses).toEqual([])
  })

  it("strips encoded upstream body headers before streaming with NextResponse", () => {
    const unsafeStreamedResponses = responseHandlerFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /\bnew\s+NextResponse\s*\([\s\S]*?\bresponse\.body\b/.test(source) &&
          !(
            /\.delete\s*\(\s*["']content-encoding["']\s*\)/.test(source) &&
            /\.delete\s*\(\s*["']content-length["']\s*\)/.test(source) &&
            /\.delete\s*\(\s*["']transfer-encoding["']\s*\)/.test(source)
          )
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(unsafeStreamedResponses).toEqual([])
  })

  it("returns null response bodies for HEAD requests when using NextResponse", () => {
    const headBodies = responseHandlerFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bexport\s+async\s+function\s+HEAD\b/.test(source) && /\bnew\s+NextResponse\s*\(/.test(source) && !/request\.method\s*===\s*["']HEAD["']\s*\?\s*null/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(headBodies).toEqual([])
  })

  it("mutates response cookies only from Proxy or Route Handlers", () => {
    const invalidCookieMutations = sourceFiles
      .filter((file) => !responseHandlerFiles.includes(file))
      .filter((file) => /\b\w+\.cookies\.(?:set|delete)\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(invalidCookieMutations).toEqual([])
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
