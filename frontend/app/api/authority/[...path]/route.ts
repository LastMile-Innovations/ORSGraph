import { NextRequest, NextResponse } from "next/server"
import { orsBackendApiBaseUrl } from "@/lib/ors-api-url"

type RouteContext = {
  params: Promise<{
    path?: string[]
  }>
}

const HOP_BY_HOP_HEADERS = new Set([
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

const AUTHORITY_CACHE_SECONDS = Number(process.env.ORS_AUTHORITY_ROUTE_CACHE_SECONDS || 3600)
const AUTHORITY_SWR_SECONDS = Number(process.env.ORS_AUTHORITY_ROUTE_SWR_SECONDS || 86400)
const HOTSET_BASE_URL = (process.env.ORS_AUTHORITY_HOTSET_BASE_URL || "").replace(/\/$/, "")

export async function GET(request: NextRequest, context: RouteContext) {
  return forwardAuthorityRead(request, context)
}

export async function HEAD(request: NextRequest, context: RouteContext) {
  return forwardAuthorityRead(request, context)
}

async function forwardAuthorityRead(request: NextRequest, context: RouteContext) {
  const { path = [] } = await context.params
  const hotsetResponse = await fetchHotset(path, request)
  if (hotsetResponse) return hotsetResponse

  const upstreamUrl = new URL(`${orsBackendApiBaseUrl()}/${path.map(encodeURIComponent).join("/")}`)
  upstreamUrl.search = request.nextUrl.search

  const headers = new Headers()
  request.headers.forEach((value, key) => {
    const normalized = key.toLowerCase()
    if (!HOP_BY_HOP_HEADERS.has(normalized) && normalized !== "authorization") {
      headers.set(key, value)
    }
  })
  headers.set("accept", request.headers.get("accept") || "application/json")

  const response = await fetch(upstreamUrl, {
    method: request.method,
    headers,
    cache: "force-cache",
  })

  const responseHeaders = authorityHeaders(response.headers, "backend")
  return new NextResponse(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers: responseHeaders,
  })
}

async function fetchHotset(path: string[], request: NextRequest) {
  if (!HOTSET_BASE_URL || request.method !== "GET") return null

  const hotsetUrl = new URL(`${HOTSET_BASE_URL}/${path.map(encodeURIComponent).join("/")}.json`)
  hotsetUrl.search = request.nextUrl.search

  const response = await fetch(hotsetUrl, {
    headers: { accept: "application/json" },
    cache: "force-cache",
  }).catch(() => null)

  if (!response?.ok) return null
  return new NextResponse(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers: authorityHeaders(response.headers, "r2-hotset"),
  })
}

function authorityHeaders(source: Headers, authorityOrigin: "backend" | "r2-hotset") {
  const headers = new Headers(source)
  headers.delete("content-encoding")
  headers.delete("content-length")
  headers.delete("transfer-encoding")
  headers.set(
    "cache-control",
    `public, s-maxage=${AUTHORITY_CACHE_SECONDS}, stale-while-revalidate=${AUTHORITY_SWR_SECONDS}`,
  )
  headers.set("x-ors-authority-origin", authorityOrigin)
  headers.set("vary", "accept-encoding")
  return headers
}
