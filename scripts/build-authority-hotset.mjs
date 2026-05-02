#!/usr/bin/env node

import { mkdir, readFile, writeFile } from "node:fs/promises"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import {
  authorityHotsetObjectPath,
  normalizedAuthorityRequest,
} from "../frontend/lib/authority-hotset.mjs"

const DEFAULT_BASE_URL = "http://localhost:8080/api/v1"
const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)))
const baseUrl = (
  process.env.ORS_AUTHORITY_BASE_URL ||
  process.env.ORS_API_BASE_URL ||
  process.env.NEXT_PUBLIC_ORS_API_BASE_URL ||
  DEFAULT_BASE_URL
).replace(/\/$/, "")
const manifestPath = process.env.ORS_CORPUS_RELEASE_MANIFEST_PATH || join(repoRoot, "data/graph/corpus_release.json")
const outDir = process.env.ORS_AUTHORITY_HOTSET_OUT_DIR || join(repoRoot, "data/authority-hotset")
const releaseId = await readReleaseId(manifestPath)

const endpoints = [
  "/home",
  "/stats",
  "/featured-statutes",
  "/analytics/home",
  "/statutes?limit=60&offset=0",
  "/search/open?q=ORS%2090.320&authority_family=ORS",
  "/search/open?q=ORS%2090.300&authority_family=ORS",
  "/search?q=landlord%20repair&type=all&mode=keyword&limit=10&offset=0&rerank=false",
  "/search?q=tenant%20notice%20repair&type=all&mode=hybrid&limit=10&offset=0&rerank=false",
  "/statutes/ORS%2090.320/page",
  "/statutes/ORS%2090.320/provisions",
  "/statutes/ORS%2090.300/page",
  "/statutes/ORS%2090.300/provisions",
  "/graph/neighborhood?citation=ORS%2090.320&depth=1&limit=40&mode=legal",
  "/rules/applicable?jurisdiction=or%3Astate&date=2026-05-02&type=complaint",
]

const generated = []
const failures = []

for (const endpoint of endpoints) {
  const target = new URL(endpoint, `${baseUrl}/`)
  const path = target.pathname.replace(/^\/+/, "").split("/").filter(Boolean).map(decodeURIComponent)
  const objectPath = authorityHotsetObjectPath(path, target.searchParams, releaseId)
  const normalized = normalizedAuthorityRequest(path, target.searchParams)
  const outputPath = join(outDir, objectPath)

  try {
    const response = await fetch(target, { headers: { accept: "application/json" } })
    const text = await response.text()
    if (!response.ok) throw new Error(`${response.status} ${response.statusText}: ${text.slice(0, 200)}`)

    const json = JSON.parse(text)
    validateAuthorityPayload(endpoint, json)
    await mkdir(dirname(outputPath), { recursive: true })
    const bytes = Buffer.byteLength(JSON.stringify(json))
    await writeFile(outputPath, `${JSON.stringify(json, null, 2)}\n`)
    generated.push({
      endpoint,
      normalized_key: normalized.normalizedPathAndSearch,
      object_path: objectPath,
      output_path: outputPath,
      bytes,
      source_status: response.headers.get("x-ors-cache") || "origin",
      corpus_release_id: response.headers.get("x-ors-corpus-release") || releaseId,
    })
    console.log(`hotset ${endpoint} -> ${outputPath}`)
  } catch (error) {
    failures.push({ endpoint, output_path: outputPath, error: error instanceof Error ? error.message : String(error) })
    console.error(`fail ${endpoint}: ${error instanceof Error ? error.message : String(error)}`)
  }
}

const manifest = {
  schema_version: "orsgraph.authority_hotset.v1",
  release_id: releaseId,
  generated_at: new Date().toISOString(),
  base_url: baseUrl,
  endpoint_count: endpoints.length,
  generated_count: generated.length,
  failure_count: failures.length,
  generated,
  failures,
}

const manifestOut = join(outDir, encodeURIComponent(releaseId), "manifest.json")
await mkdir(dirname(manifestOut), { recursive: true })
await writeFile(manifestOut, `${JSON.stringify(manifest, null, 2)}\n`)
console.log(`manifest -> ${manifestOut}`)

if (failures.length > 0) {
  process.exitCode = 1
}

async function readReleaseId(path) {
  try {
    const manifest = JSON.parse(await readFile(path, "utf8"))
    return String(manifest.release_id || "release:unversioned")
  } catch {
    return "release:unversioned"
  }
}

function validateAuthorityPayload(endpoint, payload) {
  for (const forbidden of ["accessToken", "authorization", "cookie", "password", "secret"]) {
    if (containsPrivateKey(payload, forbidden.toLowerCase())) {
      throw new Error(`payload contains private-looking field: ${forbidden}`)
    }
  }

  if (endpoint.startsWith("/search?")) {
    if (!Array.isArray(payload.results)) throw new Error("search payload missing results array")
    const hasUnbackedTopResult = payload.results.slice(0, 3).some((result) => result.source_backed === false)
    if (hasUnbackedTopResult) throw new Error("top search results must stay source-backed")
  }

  if (endpoint.includes("/statutes/") && endpoint.endsWith("/page")) {
    if (!payload.identity?.citation || !payload.current_version) {
      throw new Error("statute page payload missing identity/current_version")
    }
  }
}

function containsPrivateKey(value, forbidden) {
  if (!value || typeof value !== "object") return false
  if (Array.isArray(value)) return value.some((item) => containsPrivateKey(item, forbidden))
  return Object.entries(value).some(([key, child]) => key.toLowerCase().includes(forbidden) || containsPrivateKey(child, forbidden))
}
