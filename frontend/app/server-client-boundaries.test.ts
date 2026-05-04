import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const sourceFiles = collectSourceFiles(frontendDir)
const clientFiles = sourceFiles.filter((file) => isClientFile(readFileSync(file, "utf8")))

const appConventionFilenames = new Set([
  "page.tsx",
  "layout.tsx",
  "loading.tsx",
  "template.tsx",
  "default.tsx",
  "not-found.tsx",
  "error.tsx",
  "global-error.tsx",
])

const serverOnlyImportPatterns = [
  /from\s+["']next\/headers["']/,
  /from\s+["']next\/cache["']/,
  /from\s+["']@\/lib\/auth["']/,
  /from\s+["']@\/lib\/authority-server-cache["']/,
  /from\s+["']@\/lib\/casebuilder\/server-api["']/,
  /from\s+["']@\/lib\/ors-backend-api-url["']/,
  /import\s+["']server-only["']/,
]

describe("server/client component boundaries", () => {
  it("keeps route pages, layouts, loading states, templates, and defaults as Server Components", () => {
    const clientRouteConventions = clientFiles
      .filter((file) => file.startsWith(appDir))
      .filter((file) => appConventionFilenames.has(basename(file)))
      .map((file) => relative(frontendDir, file))
      .filter((file) => {
        if (file.endsWith("/error.tsx") || file === "app/global-error.tsx") return false
        return true
      })

    expect(clientRouteConventions).toEqual([])
  })

  it("does not import server-only modules into Client Components", () => {
    const poisonedImports = clientFiles
      .filter((file) => serverOnlyImportPatterns.some((pattern) => pattern.test(readFileSync(file, "utf8"))))
      .map((file) => relative(frontendDir, file))

    expect(poisonedImports).toEqual([])
  })

  it("does not read private environment variables from Client Components", () => {
    const privateEnvReads = clientFiles
      .flatMap((file) => {
        const content = readFileSync(file, "utf8")
        return Array.from(content.matchAll(/\bprocess\.env\.([A-Z0-9_]+)/g))
          .map((match) => match[1])
          .filter((name) => name && !name.startsWith("NEXT_PUBLIC_"))
          .map((name) => `${relative(frontendDir, file)}:${name}`)
      })

    expect(privateEnvReads).toEqual([])
  })

  it("keeps the shared API URL helper aligned to Railway instead of Vercel", () => {
    const helper = readFileSync(join(frontendDir, "lib/ors-api-url.ts"), "utf8")

    expect(helper).toContain("RAILWAY_PUBLIC_DOMAIN")
    expect(helper).not.toContain("VERCEL_URL")
  })
})

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
