import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { describe, expect, it } from "vitest"

const frontendDir = process.cwd()
const eslintConfig = readFileSync(join(frontendDir, "eslint.config.mjs"), "utf8")
const nextConfig = readFileSync(join(frontendDir, "next.config.mjs"), "utf8")
const packageJson = JSON.parse(readFileSync(join(frontendDir, "package.json"), "utf8")) as {
  scripts?: Record<string, string>
  dependencies?: Record<string, string>
  devDependencies?: Record<string, string>
}
const sourceFiles = collectSourceFiles(frontendDir)
const allowedRuleOverrides = [
  "@typescript-eslint/no-explicit-any",
  "react-hooks/purity",
  "react-hooks/set-state-in-effect",
  "react/no-unescaped-entities",
]
const allowedInlineDisableRules = new Set(["@typescript-eslint/no-unused-vars", "react-hooks/exhaustive-deps"])

describe("ESLint policy", () => {
  it("uses the Next flat config with Core Web Vitals and TypeScript rules", () => {
    expect(eslintConfig).toContain('from "eslint-config-next/core-web-vitals"')
    expect(eslintConfig).toContain('from "eslint-config-next/typescript"')
    expect(eslintConfig).toContain("...nextVitals")
    expect(eslintConfig).toContain("...nextTypescript")
    expect(eslintConfig).toMatch(/\bexport\s+default\s+config\b/)
  })

  it("keeps the ESLint CLI path instead of removed next lint", () => {
    expect(packageJson.scripts?.lint).toBe("eslint .")
    expect(Object.values(packageJson.scripts ?? {}).filter((script) => /\bnext\s+lint\b/.test(script))).toEqual([])
  })

  it("does not use the removed eslint option in next.config", () => {
    expect(nextConfig).not.toMatch(/\beslint\s*:/)
  })

  it("keeps the Next ESLint dependency installed as a dev dependency", () => {
    expect(packageJson.devDependencies?.eslint).toBeDefined()
    expect(packageJson.devDependencies?.["eslint-config-next"]).toBeDefined()
    expect(packageJson.dependencies?.eslint).toBeUndefined()
    expect(packageJson.dependencies?.["eslint-config-next"]).toBeUndefined()
  })

  it("does not keep legacy ESLint config files beside the flat config", () => {
    const legacyConfigs = readdirSync(frontendDir).filter((entry) =>
      /^(?:\.eslintrc(?:\.(?:js|cjs|json|yml|yaml))?|eslint\.config\.(?:js|cjs|ts))$/.test(entry)
    )

    expect(legacyConfigs).toEqual([])
  })

  it("keeps generated and build output ignored by ESLint", () => {
    for (const ignoredPath of [".next/**", "out/**", "build/**", "coverage/**", "next-env.d.ts", "tsconfig.tsbuildinfo"]) {
      expect(eslintConfig).toContain(`"${ignoredPath}"`)
    }
  })

  it("keeps rule overrides bounded and visible", () => {
    const disabledRules = Array.from(eslintConfig.matchAll(/["']([^"']+)["']\s*:\s*["']off["']/g)).map((match) => match[1])

    expect(disabledRules.sort()).toEqual([...allowedRuleOverrides].sort())
  })

  it("does not disable Next.js rules in config or inline comments", () => {
    const disabledNextRules = sourceFiles
      .filter((file) => /eslint-disable[^\n]*@next\/next\//.test(readFileSync(file, "utf8")))
      .map((file) => relative(frontendDir, file))

    expect(eslintConfig).not.toMatch(/@next\/next\/[^"']+["']\s*:\s*["']off["']/)
    expect(disabledNextRules).toEqual([])
  })

  it("keeps inline eslint disables allow-listed", () => {
    const unexpectedDisables = sourceFiles.flatMap((file) => {
      const source = readFileSync(file, "utf8")
      return Array.from(source.matchAll(/eslint-disable(?:-next-line)?\s+([^\n*]+)/g)).flatMap((match) => {
        const rules = match[1]
          .split(/,\s*/)
          .map((rule) => rule.trim())
          .filter(Boolean)
        return rules
          .filter((rule) => !allowedInlineDisableRules.has(rule))
          .map((rule) => `${relative(frontendDir, file)}:${rule}`)
      })
    })

    expect(unexpectedDisables).toEqual([])
  })

  it("does not opt out of Core Web Vitals image, script, or head rules by pattern", () => {
    const riskyPatterns = sourceFiles
      .filter((file) => {
        const source = readFileSync(file, "utf8")
        return (
          /from\s+["']next\/head["']/.test(source) ||
          /from\s+["']next\/document["']/.test(source) ||
          /<img[\s>]/.test(source) ||
          /<script[\s>]/.test(source)
        )
      })
      .map((file) => relative(frontendDir, file))

    expect(riskyPatterns).toEqual([])
  })

  it("keeps lint in the aggregate check before typecheck and build", () => {
    expect(packageJson.scripts?.check).toMatch(/^pnpm run lint && pnpm run typecheck && .*pnpm run build/)
  })
})

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    if (entry === "node_modules" || entry === ".next" || entry === "coverage") return []

    const path = join(dir, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) return collectSourceFiles(path)
    return /\.(ts|tsx|mjs)$/.test(entry) && !/\.test\.(ts|tsx)$/.test(entry) ? [path] : []
  })
}
