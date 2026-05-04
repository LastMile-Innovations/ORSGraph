import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => hasUseClientDirective(readFileSync(file, "utf8")))
const useRouterFiles = sourceFiles.filter((file) => /\buseRouter\s*\(/.test(readFileSync(file, "utf8")))
const routerNavigationCalls = routerCalls(["push", "replace"])
const routeConventionFiles = new Set([
  "page.tsx",
  "layout.tsx",
  "template.tsx",
  "default.tsx",
  "loading.tsx",
  "error.tsx",
  "global-error.tsx",
  "not-found.tsx",
])
describe("useRouter() policy", () => {
  it("imports useRouter only from next/navigation", () => {
    const invalidImports = sourceFiles
      .filter((file) => /\buseRouter\b/.test(readFileSync(file, "utf8")))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /import\s+\{[^}]*\buseRouter\b[^}]*\}\s+from/.test(source) && !/import\s+\{[^}]*\buseRouter\b[^}]*\}\s+from\s+["']next\/navigation["']/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(invalidImports).toEqual([])
  })

  it("does not use next/router or removed router fields/events", () => {
    const legacyRouterUsage = sourceFiles
      .filter((file) => /from\s+["']next\/router["']|\brouter\.(?:events|pathname|query)\b/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(legacyRouterUsage).toEqual([])
  })

  it("uses useRouter only in Client Components", () => {
    const serverHookCalls = useRouterFiles
      .filter((file) => !clientFiles.includes(file))
      .map((file) => relative(frontendDir, file))

    expect(serverHookCalls).toEqual([])
  })

  it("keeps route convention files from using useRouter directly", () => {
    const routeConventionHookCalls = useRouterFiles
      .filter((file) => file.startsWith(appDir))
      .filter((file) => routeConventionFiles.has(basename(file)))
      .map((file) => relative(frontendDir, file))

    expect(routeConventionHookCalls).toEqual([])
  })

  it("calls useRouter without arguments", () => {
    const callsWithArguments = useRouterFiles
      .filter((file) => /\buseRouter\s*\(\s*[^)\s]/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(callsWithArguments).toEqual([])
  })

  it("keeps literal router destinations internal", () => {
    const invalidLiteralDestinations = routerNavigationCalls
      .map((call) => ({ ...call, literal: stringLiteral(call.args[0] ?? "") }))
      .filter(({ literal }) => literal !== undefined && !isInternalHref(literal))
      .map(({ file, line, literal }) => `${relative(frontendDir, file)}:${line}:${literal}`)

    expect(invalidLiteralDestinations).toEqual([])
  })

  it("does not pass callback URLs or API-provided href fields directly to router navigation", () => {
    const unsafeDynamicDestinations = routerNavigationCalls
      .filter(({ args }) => /\bcallbackUrl\b|\.\s*href\b/.test(args[0] ?? ""))
      .map(({ file, line, args }) => `${relative(frontendDir, file)}:${line}:${args[0]}`)

    expect(unsafeDynamicDestinations).toEqual([])
  })

  it("sanitizes auth callback URLs before client navigation or sign-in", () => {
    const pendingPage = readFileSync(join(frontendDir, "app/auth/pending/page.tsx"), "utf8")
    const pendingClient = readFileSync(join(frontendDir, "app/auth/pending/pending-client.tsx"), "utf8")
    const signInPage = readFileSync(join(frontendDir, "app/auth/signin/page.tsx"), "utf8")
    const signInClient = readFileSync(join(frontendDir, "app/auth/signin/signin-client.tsx"), "utf8")

    expect(pendingPage).toContain("safeCallbackHref(callbackUrl)")
    expect(pendingClient).toContain("safeCallbackUrl")
    expect(pendingClient).toContain("router.replace(safeCallbackUrl)")
    expect(signInPage).toContain("safeCallbackHref(callbackUrl)")
    expect(signInClient).toContain("safeCallbackUrl")
  })

  it("sanitizes API-returned hrefs before router.push", () => {
    const homeSearch = readFileSync(join(frontendDir, "components/home/HeroSearch.tsx"), "utf8")
    const searchClient = readFileSync(join(frontendDir, "components/orsg/search/search-client.tsx"), "utf8")

    expect(homeSearch).toContain("toSafeInternalHref(response.href)")
    expect(searchClient).toContain("toSafeInternalHref(opened.href)")
    expect(searchClient).toContain("toSafeInternalHref(suggestion.href)")
  })

  it("keeps router.refresh argument-free and separate from server-side cache invalidation", () => {
    const refreshWithArguments = routerCalls(["refresh"])
      .filter(({ args }) => args.length > 0)
      .map(({ file, line }) => `${relative(frontendDir, file)}:${line}`)
    const returnedRefreshes = sourceFiles
      .filter((file) => /\breturn\s+router\.refresh\s*\(/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(refreshWithArguments).toEqual([])
    expect(returnedRefreshes).toEqual([])
  })
})

function routerCalls(methods: string[]) {
  const pattern = new RegExp(`\\brouter\\.(${methods.join("|")})\\s*\\(`, "g")

  return sourceFiles.flatMap((file) => {
    const source = readFileSync(file, "utf8")
    const calls: { file: string; line: number; method: string; args: string[] }[] = []
    let match: RegExpExecArray | null
    while ((match = pattern.exec(source))) {
      const argsStart = match.index + match[0].length - 1
      const argsEnd = callEnd(source, argsStart)
      calls.push({
        file,
        line: source.slice(0, match.index).split("\n").length,
        method: match[1] ?? "",
        args: splitTopLevelArgs(source.slice(argsStart + 1, argsEnd - 1)),
      })
      pattern.lastIndex = Math.max(pattern.lastIndex, argsEnd)
    }
    return calls
  })
}

function callEnd(source: string, start: number) {
  let depth = 0
  let quote = ""
  for (let index = start; index < source.length; index += 1) {
    const char = source[index]
    const previous = source[index - 1]
    if (quote) {
      if (char === quote && previous !== "\\") quote = ""
      continue
    }
    if (char === '"' || char === "'" || char === "`") {
      quote = char
      continue
    }
    if (char === "(") depth += 1
    if (char === ")") {
      depth -= 1
      if (depth === 0) return index + 1
    }
  }
  return source.length
}

function splitTopLevelArgs(args: string) {
  const parts: string[] = []
  let depth = 0
  let quote = ""
  let start = 0
  for (let index = 0; index < args.length; index += 1) {
    const char = args[index]
    const previous = args[index - 1]
    if (quote) {
      if (char === quote && previous !== "\\") quote = ""
      continue
    }
    if (char === '"' || char === "'" || char === "`") {
      quote = char
      continue
    }
    if (char === "{" || char === "[" || char === "(") depth += 1
    if (char === "}" || char === "]" || char === ")") depth -= 1
    if (char === "," && depth === 0) {
      parts.push(args.slice(start, index).trim())
      start = index + 1
    }
  }
  const tail = args.slice(start).trim()
  if (tail) parts.push(tail)
  return parts
}

function stringLiteral(value: string) {
  const match = value.match(/^["']([^"']*)["']$/)
  return match?.[1]
}

function isInternalHref(value: string) {
  return value.startsWith("/") && !value.startsWith("//") && !/[\u0000-\u001F\u007F]/.test(value)
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
