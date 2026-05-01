import { fileURLToPath } from "node:url"
import { defineConfig } from "vitest/config"

const coverageAll = process.env.COVERAGE_ALL === "true"
const baselineCoverageFiles = [
  "components/home/CorpusStatusPanel.tsx",
  "components/home/MetricTile.tsx",
  "lib/casebuilder/routes.ts",
  "lib/data-state.ts",
  "lib/runtime-status.ts",
  "lib/utils.ts",
]

export default defineConfig({
  resolve: {
    alias: {
      "@": fileURLToPath(new URL(".", import.meta.url)),
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./test/setup.ts"],
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "lcov"],
      reportsDirectory: "./coverage",
      include: coverageAll ? ["components/**/*.{ts,tsx}", "lib/**/*.{ts,tsx}", "hooks/**/*.{ts,tsx}"] : baselineCoverageFiles,
      exclude: [
        "components/ui/**",
        "**/*.d.ts",
        "**/*.test.{ts,tsx}",
        "lib/mock-*.ts",
        "lib/**/mock-*.ts",
        "lib/**/mock-*.tsx",
      ],
      thresholds: coverageAll
        ? undefined
        : {
            statements: 80,
            branches: 65,
            functions: 75,
            lines: 80,
          },
    },
  },
})
