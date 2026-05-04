import { getServerSession } from "next-auth"
import { NextRequest, NextResponse } from "next/server"
import {
  authorityCacheControl,
  authorityHotsetObjectPath,
  authorityReadPolicy,
  joinUrlPath,
  normalizedAuthorityRequest,
} from "@/lib/authority-hotset.mjs"
import { authOptions } from "@/lib/auth"
import { orsBackendApiBaseUrl } from "@/lib/ors-backend-api-url"

type RouteContext = {
  params: Promise<{
    path?: string[]
  }>
}

const HOP_BY_HOP_HEADERS = new Set([
  "accept-encoding",
  "connection",
  "content-length",
  "cookie",
  "host",
  "keep-alive",
  "proxy-authenticate",
  "proxy-authorization",
  "te",
  "trailer",
  "transfer-encoding",
  "upgrade",
])

const HOTSET_BASE_URL = (process.env.ORS_AUTHORITY_HOTSET_BASE_URL || "").replace(/\/$/, "")
const HOTSET_RELEASE_ID = releaseIdFromHotsetBaseUrl(HOTSET_BASE_URL)
const API_KEY = process.env.ORS_API_KEY

export async function GET(request: NextRequest, context: RouteContext) {
  return forwardAuthorityRead(request, context)
}

export async function HEAD(request: NextRequest, context: RouteContext) {
  return forwardAuthorityRead(request, context)
}

async function forwardAuthorityRead(request: NextRequest, context: RouteContext) {
  const startedAt = Date.now()
  const { path = [] } = await context.params
  const policy = authorityReadPolicy(path, request.nextUrl.searchParams)
  if (!policy.allowed) {
    return NextResponse.json(
      { error: "Not found" },
      {
        status: 404,
        headers: {
          "cache-control": "no-store",
          "x-ors-authority-origin": "blocked",
          "x-ors-authority-policy": policy.reason,
        },
      },
    )
  }

  const normalized = normalizedAuthorityRequest(path, request.nextUrl.searchParams)
  const hotsetResponse = policy.hotsetEligible ? await fetchHotset(path, request, normalized, startedAt) : null
  if (hotsetResponse) return hotsetResponse

  const upstreamUrl = new URL(`${orsBackendApiBaseUrl()}/${normalized.encodedPath}`)
  upstreamUrl.search = normalized.normalizedSearch

  const session = await getServerSession(authOptions)
  const headers = new Headers()
  request.headers.forEach((value, key) => {
    const normalized = key.toLowerCase()
    if (!HOP_BY_HOP_HEADERS.has(normalized) && normalized !== "authorization") {
      headers.set(key, value)
    }
  })
  headers.set("accept", request.headers.get("accept") || "application/json")
  headers.set("Accept-Encoding", "identity")
  if (session?.accessToken) {
    headers.set("Authorization", `Bearer ${session.accessToken}`)
  } else if (API_KEY && !headers.has("x-api-key")) {
    headers.set("x-api-key", API_KEY)
  }

  const response = await fetch(upstreamUrl, {
    method: request.method,
    headers,
    cache: policy.cacheable ? "force-cache" : "no-store",
  })

  const responseHeaders = authorityHeaders(response.headers, {
    authorityOrigin: "backend",
    cacheable: policy.cacheable,
    hotsetPath: policy.hotsetEligible ? authorityHotsetObjectPath(path, request.nextUrl.searchParams) : null,
    normalizedPathAndSearch: normalized.normalizedPathAndSearch,
    startedAt,
  })
  return new NextResponse(request.method === "HEAD" ? null : response.body, {
    status: response.status,
    statusText: response.statusText,
    headers: responseHeaders,
  })
}

async function fetchHotset(
  path: string[],
  request: NextRequest,
  normalized: ReturnType<typeof normalizedAuthorityRequest>,
  startedAt: number,
) {
  if (!HOTSET_BASE_URL || !["GET", "HEAD"].includes(request.method)) return null

  const hotsetPath = authorityHotsetObjectPath(path, request.nextUrl.searchParams)
  const hotsetUrl = new URL(joinUrlPath(HOTSET_BASE_URL, hotsetPath))

  const response = await fetch(hotsetUrl, {
    method: request.method,
    headers: { accept: "application/json" },
    cache: "force-cache",
  }).catch(() => null)

  if (!response?.ok) return null
  return new NextResponse(request.method === "HEAD" ? null : response.body, {
    status: response.status,
    statusText: response.statusText,
    headers: authorityHeaders(response.headers, {
      authorityOrigin: "r2-hotset",
      cacheable: true,
      hotsetPath,
      normalizedPathAndSearch: normalized.normalizedPathAndSearch,
      startedAt,
    }),
  })
}

function authorityHeaders(
  source: Headers,
  options: {
    authorityOrigin: "backend" | "r2-hotset"
    cacheable: boolean
    hotsetPath: string | null
    normalizedPathAndSearch: string
    startedAt: number
  },
) {
  const headers = new Headers(source)
  headers.delete("content-encoding")
  headers.delete("content-length")
  headers.delete("transfer-encoding")
  headers.set("cache-control", authorityCacheControl(undefined, undefined, options.cacheable))
  headers.set("x-ors-authority-origin", options.authorityOrigin)
  headers.set("x-ors-authority-policy", options.cacheable ? "cacheable" : "no-store")
  headers.set("x-ors-authority-normalized-key", options.normalizedPathAndSearch)
  headers.set("x-ors-authority-timing-ms", String(Date.now() - options.startedAt))
  headers.set("cf-cache-status", source.get("cf-cache-status") || (options.authorityOrigin === "r2-hotset" ? "HIT" : "BYPASS"))
  if (options.hotsetPath) headers.set("x-ors-authority-hotset-path", options.hotsetPath)
  if (!headers.has("x-ors-corpus-release") && HOTSET_RELEASE_ID) {
    headers.set("x-ors-corpus-release", HOTSET_RELEASE_ID)
  }
  headers.set("vary", "accept, accept-encoding")
  return headers
}

function releaseIdFromHotsetBaseUrl(baseUrl: string) {
  if (!baseUrl) return ""
  const segment = baseUrl.split("/").filter(Boolean).at(-1) || ""
  try {
    return decodeURIComponent(segment)
  } catch {
    return segment
  }
}
