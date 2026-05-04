import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const packageJson = JSON.parse(readFileSync(join(frontendDir, "package.json"), "utf8")) as {
  scripts?: Record<string, string>
}
const nextConfig = readFileSync(join(frontendDir, "next.config.mjs"), "utf8")
const postcssConfig = readFileSync(join(frontendDir, "postcss.config.mjs"), "utf8")
const sourceFiles = collectFiles(frontendDir).filter((file) => !/\.test\.(?:ts|tsx)$/.test(file))
const cssFiles = sourceFiles.filter((file) => /\.(?:css|scss|sass)$/.test(file))
const cssModuleFiles = sourceFiles.filter((file) => /\.module\.css$/.test(file))

describe("Turbopack policy", () => {
  it("uses the Next 16 default Turbopack scripts instead of opting into webpack", () => {
    expect(packageJson.scripts?.dev).toBe("next dev")
    expect(packageJson.scripts?.build).toBe("next build")

    const webpackScripts = Object.entries(packageJson.scripts ?? {})
      .filter(([, script]) => /\b--webpack\b/.test(script))
      .map(([name]) => name)

    expect(webpackScripts).toEqual([])
  })

  it("does not define ignored webpack configuration under next.config", () => {
    expect(nextConfig).not.toMatch(/\bwebpack\s*\(/)
    expect(nextConfig).not.toMatch(/\bwebpack\s*:/)
  })

  it("does not add Turbopack configuration without an explicit policy update", () => {
    expect(nextConfig).not.toMatch(/\bturbopack\s*:/)
  })

  it("does not suppress Turbopack issues", () => {
    expect(nextConfig).not.toMatch(/\bignoreIssue\s*:/)
  })

  it("does not enable unsupported or legacy experimental bundler flags", () => {
    for (const flag of ["urlImports", "esmExternals", "nextScriptWorkers", "fallbackNodePolyfills"]) {
      expect(nextConfig).not.toMatch(new RegExp(`\\b${flag}\\s*:`))
    }
  })

  it("does not introduce Babel config files that change the Turbopack transform path", () => {
    const babelConfigs = readdirSync(frontendDir).filter((entry) =>
      /^(?:\.babelrc(?:\.(?:js|cjs|mjs|json))?|babel\.config\.(?:js|cjs|mjs|json|ts|mts|cts))$/.test(entry)
    )

    expect(babelConfigs).toEqual([])
  })

  it("keeps PostCSS config in the Turbopack-supported ESM shape", () => {
    expect(postcssConfig).toContain("@tailwindcss/postcss")
    expect(postcssConfig).toMatch(/\bexport\s+default\s+config\b/)
    expect(postcssConfig).not.toMatch(/\bmodule\.exports\b/)
  })

  it("does not use Sass custom functions or legacy tilde imports", () => {
    expect(nextConfig).not.toMatch(/\bsassOptions\s*:[\s\S]*\bfunctions\s*:/)

    const legacySassImports = cssFiles
      .filter((file) => /@import\s+["']~/.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(legacySassImports).toEqual([])
  })

  it("does not use CSS Modules features unsupported by Turbopack", () => {
    const unsupportedCssModules = cssModuleFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /(?:^|\n)\s*@value\b/.test(source) ||
          /(?:^|\n)\s*:(?:import|export)\b/.test(source) ||
          /(?:^|\n)\s*:(?:local|global)\s*(?:\{|,|\n)/.test(source) ||
          /\bcomposes\s*:[^;]+from\s+["'][^"']+\.css["']/.test(source) ||
          /@import\s+["'][^"']+\.css["']/.test(source)
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(unsupportedCssModules).toEqual([])
  })

  it("does not use unsupported bundler magic comments", () => {
    const unsupportedMagicComments = sourceFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /webpackOptional\s*:\s*true/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(unsupportedMagicComments).toEqual([])
  })

  it("keeps optional or ignored dynamic imports out of application code", () => {
    const ignoredDynamicImports = sourceFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return /(?:webpackIgnore|turbopackIgnore|turbopackOptional)\s*:\s*true/.test(source)
      })
      .map((file) => relative(frontendDir, file))

    expect(ignoredDynamicImports).toEqual([])
  })
})

function collectFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next" || entry === "coverage") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectFiles(path)
    return /\.(ts|tsx|mjs|json|css|scss|sass)$/.test(entry) ? [path] : []
  })
}
