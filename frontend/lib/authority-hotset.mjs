import { createHash } from "node:crypto"

const IGNORED_QUERY_KEYS = new Set(["_", "t", "ts", "cache_bust", "cacheBust"])
const PRIVATE_PREFIXES = new Set(["admin", "api", "ask", "auth", "casebuilder", "matters", "qc", "sidebar"])
const HOTSET_EXACT_PATHS = new Set([
  "analytics/home",
  "featured-statutes",
  "home",
  "rules/applicable",
  "search/open",
  "sources",
  "stats",
])

export const DEFAULT_AUTHORITY_CACHE_SECONDS = 3600
export const DEFAULT_AUTHORITY_SWR_SECONDS = 86400

export function normalizeAuthoritySearchParams(searchParams) {
  const grouped = new Map()
  for (const [rawKey, rawValue] of searchParams.entries()) {
    const key = rawKey.trim()
    if (!key || IGNORED_QUERY_KEYS.has(key) || key.startsWith("utm_")) continue
    const value = rawValue.trim()
    if (!grouped.has(key)) grouped.set(key, [])
    grouped.get(key).push(value)
  }

  return [...grouped.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .flatMap(([key, values]) =>
      values
        .sort((left, right) => left.localeCompare(right))
        .map((value) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`),
    )
    .join("&")
}

export function normalizeAuthorityPath(path) {
  return path
    .map((segment) => safeDecodeURIComponent(String(segment)))
    .filter(Boolean)
    .map((segment) => encodeURIComponent(segment))
    .join("/")
}

export function normalizedAuthorityRequest(path, searchParams) {
  const encodedPath = normalizeAuthorityPath(path)
  const normalizedSearch = normalizeAuthoritySearchParams(searchParams)
  return {
    encodedPath,
    normalizedSearch,
    normalizedPathAndSearch: normalizedSearch ? `${encodedPath}?${normalizedSearch}` : encodedPath,
  }
}

export function authorityReadPolicy(path, searchParams = new URLSearchParams()) {
  const first = safeDecodeURIComponent(String(path[0] ?? "")).toLowerCase()
  const key = path.map((segment) => safeDecodeURIComponent(String(segment))).join("/")
  if (!first || PRIVATE_PREFIXES.has(first)) {
    return { allowed: false, cacheable: false, hotsetEligible: false, reason: "private_or_unknown" }
  }

  if (key === "search/suggest") {
    return { allowed: true, cacheable: false, hotsetEligible: false, reason: "interactive_suggest" }
  }

  if (key === "search") {
    const hasQuery = Boolean(searchParams.get("q")?.trim())
    return { allowed: hasQuery, cacheable: hasQuery, hotsetEligible: hasQuery, reason: hasQuery ? "search" : "missing_query" }
  }

  const hotsetEligible =
    HOTSET_EXACT_PATHS.has(key) ||
    first === "statutes" ||
    first === "provisions" ||
    (first === "graph" && path[1] === "neighborhood") ||
    first === "rules"

  const allowed =
    hotsetEligible ||
    first === "sources" ||
    (first === "graph" && ["full", "path"].includes(String(path[1] ?? "")))

  return {
    allowed,
    cacheable: hotsetEligible,
    hotsetEligible,
    reason: allowed ? "authority_read" : "not_authority_read",
  }
}

export function authorityHotsetObjectPath(path, searchParams = new URLSearchParams(), releaseId = "") {
  const normalized = normalizedAuthorityRequest(path, searchParams)
  const basePath = normalized.encodedPath || "index"
  const querySuffix = normalized.normalizedSearch ? `/__query-${shortHash(normalized.normalizedSearch)}` : ""
  const releasePrefix = releaseId.trim() ? `${encodeURIComponent(releaseId.trim())}/` : ""
  return `${releasePrefix}${basePath}${querySuffix}.json`
}

export function authorityCacheControl(cacheSeconds, swrSeconds, cacheable = true) {
  if (!cacheable) return "no-store"
  const sMaxAge = positiveNumber(cacheSeconds, DEFAULT_AUTHORITY_CACHE_SECONDS)
  const swr = positiveNumber(swrSeconds, DEFAULT_AUTHORITY_SWR_SECONDS)
  return `public, max-age=0, s-maxage=${sMaxAge}, stale-while-revalidate=${swr}`
}

export function authorityCacheTags(releaseId, keys = []) {
  const release = releaseId?.trim() || "release:unversioned"
  return [`authority:${release}`, ...keys.filter(Boolean)]
}

export function joinUrlPath(baseUrl, path) {
  return `${baseUrl.replace(/\/+$/, "")}/${path.replace(/^\/+/, "")}`
}

function shortHash(value) {
  return createHash("sha256").update(value).digest("hex").slice(0, 16)
}

function positiveNumber(value, fallback) {
  return Number.isFinite(value) && value > 0 ? Math.floor(value) : fallback
}

function safeDecodeURIComponent(value) {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
}
