import { readdirSync, readFileSync, statSync } from "node:fs"
import { basename, join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const appDir = join(frontendDir, "app")
const tsconfig = JSON.parse(readFileSync(join(frontendDir, "tsconfig.json"), "utf8")) as {
  compilerOptions?: Record<string, unknown>
  include?: string[]
  exclude?: string[]
}
const packageJson = JSON.parse(readFileSync(join(frontendDir, "package.json"), "utf8")) as {
  dependencies?: Record<string, string>
  devDependencies?: Record<string, string>
}
const nextConfig = readFileSync(join(frontendDir, "next.config.mjs"), "utf8")
const nextEnv = readFileSync(join(frontendDir, "next-env.d.ts"), "utf8")
const sourceFiles = collectSourceFiles(frontendDir)
const appRouteFiles = collectSourceFiles(appDir).filter((file) => !/\.test\.(?:ts|tsx)$/.test(file))
const dynamicRouteFiles = appRouteFiles.filter((file) => /\[[^/]+\]/.test(file))

describe("TypeScript policy", () => {
  it("uses the Next TypeScript plugin with strict app settings", () => {
    expect(tsconfig.compilerOptions?.strict).toBe(true)
    expect(tsconfig.compilerOptions?.noEmit).toBe(true)
    expect(tsconfig.compilerOptions?.isolatedModules).toBe(true)
    expect(tsconfig.compilerOptions?.moduleResolution).toBe("bundler")
    expect(tsconfig.compilerOptions?.plugins).toEqual([{ name: "next" }])
  })

  it("includes generated Next route types in the main tsconfig", () => {
    expect(tsconfig.include).toEqual(
      expect.arrayContaining(["next-env.d.ts", ".next/types/**/*.ts", ".next/dev/types/**/*.ts"])
    )
  })

  it("keeps incremental type checking in Next's cache directory", () => {
    expect(tsconfig.compilerOptions?.incremental).toBe(true)
    expect(tsconfig.compilerOptions?.tsBuildInfoFile).toBe(".next/cache/tsconfig.tsbuildinfo")
  })

  it("keeps custom declarations outside the generated next-env file", () => {
    expect(nextEnv).toContain('import "./.next/types/routes.d.ts";')
    expect(nextEnv).toContain("This file should not be edited")
    expect(nextEnv).not.toMatch(/\bdeclare\s+(?:global|module|namespace)\b/)
  })

  it("does not keep a legacy jsconfig beside tsconfig", () => {
    const jsconfigFiles = readdirSync(frontendDir).filter((entry) => entry === "jsconfig.json")

    expect(jsconfigFiles).toEqual([])
  })

  it("does not bypass TypeScript errors during production builds", () => {
    expect(nextConfig).not.toMatch(/\bignoreBuildErrors\s*:\s*true\b/)
  })

  it("does not use alternate build tsconfig files without a policy update", () => {
    const alternateTsconfigs = readdirSync(frontendDir).filter((entry) => /^tsconfig\..+\.json$/.test(entry))

    expect(alternateTsconfigs).toEqual([])
    expect(nextConfig).not.toMatch(/\btsconfigPath\s*:/)
  })

  it("uses route-aware helpers for dynamic App Router files that receive route props", () => {
    const untypedDynamicFiles = dynamicRouteFiles
      .filter((file) => ["layout.tsx", "page.tsx", "route.ts"].includes(basename(file)))
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bparams\b/.test(source) && !/\b(?:PageProps|LayoutProps|RouteContext)<["']\//.test(source)
      })
      .map((file) => relative(appDir, file))

    expect(untypedDynamicFiles).toEqual([])
  })

  it("uses RouteContext helpers for dynamic Route Handlers", () => {
    const untypedRouteHandlers = dynamicRouteFiles
      .filter((file) => basename(file) === "route.ts")
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /\bparams\b/.test(source) && !/\bRouteContext<["']\//.test(source)
      })
      .map((file) => relative(appDir, file))

    expect(untypedRouteHandlers).toEqual([])
  })

  it("keeps async Server Component dependency versions modern enough", () => {
    const dependencies = { ...packageJson.dependencies, ...packageJson.devDependencies }

    expect(compareVersion(dependencies.typescript, "5.1.3")).toBeGreaterThanOrEqual(0)
    expect(compareVersion(dependencies["@types/react"], "18.2.8")).toBeGreaterThanOrEqual(0)
  })

  it("does not import generated .next types from application code", () => {
    const generatedTypeImports = sourceFiles
      .filter((file) => basename(file) !== "next-env.d.ts")
      .filter((file) => /\.tsx?$/.test(file))
      .filter((file) => /from\s+["'][^"']*\.next\/types|import\s*\([^)]*["'][^"']*\.next\/types/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(generatedTypeImports).toEqual([])
  })
})

function compareVersion(rawVersion: string | undefined, minimum: string) {
  const version = parseVersion(rawVersion)
  const min = parseVersion(minimum)

  for (let index = 0; index < Math.max(version.length, min.length); index += 1) {
    const current = version[index] ?? 0
    const expected = min[index] ?? 0
    if (current !== expected) return current - expected
  }

  return 0
}

function parseVersion(rawVersion: string | undefined) {
  return (rawVersion?.match(/\d+(?:\.\d+)*/) ?? ["0"])[0].split(".").map(Number)
}

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx|mjs|json)$/.test(entry) ? [path] : []
  })
}
