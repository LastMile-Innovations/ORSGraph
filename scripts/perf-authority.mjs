#!/usr/bin/env node

const DEFAULT_BASE_URL = "http://localhost:8080/api/v1"
const baseUrl = (
  process.env.ORS_AUTHORITY_BASE_URL ||
  process.env.ORS_API_BASE_URL ||
  process.env.NEXT_PUBLIC_ORS_API_BASE_URL ||
  DEFAULT_BASE_URL
).replace(/\/$/, "")

const iterations = Number(process.env.ORS_AUTHORITY_PERF_ITERATIONS || 20)
const citation = process.env.ORS_AUTHORITY_PERF_CITATION || "ORS 90.320"
const encodedCitation = encodeURIComponent(citation)

const cases = [
  {
    name: "citation_open",
    path: `/search/open?q=${encodedCitation}`,
    budget_ms: 75,
  },
  {
    name: "keyword_search",
    path: `/search?q=${encodeURIComponent("landlord repair")}&type=all&mode=keyword&limit=10`,
    budget_ms: 250,
  },
  {
    name: "hybrid_search_no_model",
    path: `/search?q=${encodeURIComponent("tenant notice repair")}&type=all&mode=hybrid&limit=10&rerank=false`,
    budget_ms: 250,
  },
  {
    name: "semantic_search_cached_embedding",
    path: `/search?q=${encodeURIComponent("habitability repair duty")}&type=all&mode=semantic&limit=10&rerank=false`,
    budget_ms: 400,
    warmups: 2,
  },
  {
    name: "statute_page",
    path: `/statutes/${encodedCitation}/page`,
    budget_ms: 150,
  },
  {
    name: "provision_tree",
    path: `/statutes/${encodedCitation}/provisions`,
    budget_ms: 150,
  },
  {
    name: "rules_applicability",
    path: `/rules/applicable?jurisdiction=or:state&date=2026-05-02&type=complaint`,
    budget_ms: 150,
  },
  {
    name: "graph_mini_neighborhood",
    path: `/graph/neighborhood?citation=${encodedCitation}&depth=1&limit=40&mode=legal`,
    budget_ms: 250,
  },
]

const results = []

for (const testCase of cases) {
  for (let i = 0; i < (testCase.warmups ?? 1); i += 1) {
    await timedFetch(testCase.path).catch(() => null)
  }

  const samples = []
  let lastStatus = null
  let lastRelease = null
  let failures = 0

  for (let i = 0; i < iterations; i += 1) {
    const result = await timedFetch(testCase.path).catch((error) => ({ error }))
    if (result.error) {
      failures += 1
      continue
    }
    samples.push(result.ms)
    lastStatus = result.cacheStatus
    lastRelease = result.releaseId
  }

  const summary = summarize(samples)
  results.push({
    name: testCase.name,
    path: testCase.path,
    samples: samples.length,
    failures,
    budget_ms: testCase.budget_ms,
    p50_ms: summary.p50,
    p95_ms: summary.p95,
    cache_status: lastStatus,
    corpus_release_id: lastRelease,
    pass: samples.length > 0 && summary.p95 <= testCase.budget_ms && failures === 0,
  })
}

const failed = results.filter((result) => !result.pass)
const report = {
  base_url: baseUrl,
  iterations,
  generated_at: new Date().toISOString(),
  passed: failed.length === 0,
  results,
}

console.log(JSON.stringify(report, null, 2))
if (failed.length > 0 && process.env.ORS_AUTHORITY_PERF_ENFORCE === "1") {
  process.exitCode = 1
}

async function timedFetch(path) {
  const started = performance.now()
  const response = await fetch(`${baseUrl}${path}`, {
    headers: { accept: "application/json" },
  })
  const text = await response.text()
  const ms = performance.now() - started
  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText}: ${text.slice(0, 200)}`)
  }
  return {
    ms,
    cacheStatus: response.headers.get("x-ors-cache") || response.headers.get("x-ors-authority-origin"),
    releaseId: response.headers.get("x-ors-corpus-release"),
  }
}

function summarize(samples) {
  if (samples.length === 0) return { p50: null, p95: null }
  const sorted = [...samples].sort((left, right) => left - right)
  return {
    p50: round(percentile(sorted, 0.5)),
    p95: round(percentile(sorted, 0.95)),
  }
}

function percentile(sorted, p) {
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * p) - 1)
  return sorted[index]
}

function round(value) {
  return Math.round(value * 10) / 10
}
