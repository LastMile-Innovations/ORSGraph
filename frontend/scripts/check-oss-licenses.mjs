import { readFileSync, existsSync } from "node:fs"
import { resolve } from "node:path"

const repoRoot = resolve(import.meta.dirname, "../..")
const frontendRoot = resolve(import.meta.dirname, "..")

const deniedPackagePatterns = [
  /onlyoffice/i,
  /collabora/i,
  /pspdfkit/i,
  /apryse/i,
  /pdftron/i,
  /aspose/i,
  /groupdocs/i,
  /syncfusion/i,
  /devexpress/i,
  /telerik/i,
  /foxit/i,
  /nutrient/i,
]

const deniedTextPatterns = [
  /commercial[-_\s]?only/i,
  /proprietary/i,
  /trial[-_\s]?license/i,
  /closed[-_\s]?source/i,
]

const agplPattern = /\bAGPL\b|Affero General Public License/i
const allowAgpl = process.env.ALLOW_AGPL_DOCUMENT_STACK === "true"

const filesToScan = [
  resolve(frontendRoot, "package.json"),
  resolve(frontendRoot, "pnpm-lock.yaml"),
  resolve(repoRoot, "Cargo.toml"),
  resolve(repoRoot, "Cargo.lock"),
  resolve(repoRoot, "crates/orsgraph-api/Cargo.toml"),
].filter(existsSync)

const packageJson = JSON.parse(readFileSync(resolve(frontendRoot, "package.json"), "utf8"))
const packageNames = [
  ...Object.keys(packageJson.dependencies ?? {}),
  ...Object.keys(packageJson.devDependencies ?? {}),
  ...Object.keys(packageJson.optionalDependencies ?? {}),
]

const failures = []
for (const name of packageNames) {
  if (deniedPackagePatterns.some((pattern) => pattern.test(name))) {
    failures.push(`Denied document dependency: ${name}`)
  }
}

for (const file of filesToScan) {
  const text = readFileSync(file, "utf8")
  for (const pattern of deniedTextPatterns) {
    if (pattern.test(text)) {
      failures.push(`Denied license marker in ${relative(file)}: ${pattern}`)
    }
  }
  if (!allowAgpl && agplPattern.test(text)) {
    failures.push(`AGPL dependency/service marker in ${relative(file)} requires ALLOW_AGPL_DOCUMENT_STACK=true`)
  }
  for (const pattern of deniedPackagePatterns) {
    if (pattern.test(text)) {
      failures.push(`Denied document stack marker in ${relative(file)}: ${pattern}`)
    }
  }
}

if (failures.length) {
  console.error("OSS license gate failed:")
  for (const failure of failures) console.error(`- ${failure}`)
  process.exit(1)
}

console.log(`OSS license gate passed (${filesToScan.length} manifests scanned).`)

function relative(path) {
  return path.startsWith(repoRoot) ? path.slice(repoRoot.length + 1) : path
}
