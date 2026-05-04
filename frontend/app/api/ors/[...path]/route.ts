import { getServerSession } from "next-auth"
import { revalidateTag } from "next/cache"
import { NextRequest, NextResponse } from "next/server"
import { authOptions } from "@/lib/auth"
import { authorityCacheTags } from "@/lib/authority-hotset.mjs"
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
const AUTHORITY_RELEASE_ID = releaseIdFromHotsetBaseUrl(process.env.ORS_AUTHORITY_HOTSET_BASE_URL || "")

export async function GET(request: NextRequest, context: RouteContext) {
  return forwardRequest(request, context)
}

export async function POST(request: NextRequest, context: RouteContext) {
  return forwardRequest(request, context)
}

export async function PATCH(request: NextRequest, context: RouteContext) {
  return forwardRequest(request, context)
}

export async function DELETE(request: NextRequest, context: RouteContext) {
  return forwardRequest(request, context)
}

export async function PUT(request: NextRequest, context: RouteContext) {
  return forwardRequest(request, context)
}

async function forwardRequest(request: NextRequest, context: RouteContext) {
  const { path = [] } = await context.params
  const publicAuthRequest = isPublicAuthRequest(request.method, path)
  const session = await getServerSession(authOptions)
  if (!session?.accessToken && !publicAuthRequest) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 })
  }

  const upstreamUrl = new URL(`${orsBackendApiBaseUrl()}/${path.map(encodeURIComponent).join("/")}`)
  upstreamUrl.search = request.nextUrl.search

  const headers = new Headers()
  request.headers.forEach((value, key) => {
    if (!HOP_BY_HOP_HEADERS.has(key.toLowerCase())) {
      headers.set(key, value)
    }
  })
  headers.set("Accept-Encoding", "identity")
  if (session?.accessToken) {
    headers.set("Authorization", `Bearer ${session.accessToken}`)
  }

  const init: RequestInit = {
    method: request.method,
    headers,
    cache: "no-store",
  }

  if (!["GET", "HEAD"].includes(request.method)) {
    init.body = await request.arrayBuffer()
  }

  const response = await fetch(upstreamUrl, init)
  if (response.ok && isAdminJobDetailPath(path)) {
    await revalidateAuthorityAfterSuccessfulMutatingJob(response.clone())
  }
  const responseHeaders = new Headers(response.headers)
  responseHeaders.delete("content-encoding")
  responseHeaders.delete("content-length")
  responseHeaders.delete("transfer-encoding")

  return new NextResponse(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers: responseHeaders,
  })
}

function isAdminJobDetailPath(path: string[]) {
  return path.length >= 3 && path[0] === "admin" && path[1] === "jobs"
}

async function revalidateAuthorityAfterSuccessfulMutatingJob(response: Response) {
  const body = (await response.json().catch(() => null)) as unknown
  if (!body || typeof body !== "object" || Array.isArray(body)) return

  const detail = body as { job?: { status?: unknown; is_read_only?: unknown } }
  if (detail.job?.status !== "succeeded" || detail.job.is_read_only !== false) return

  for (const tag of authorityCacheTags(AUTHORITY_RELEASE_ID)) {
    revalidateTag(tag, "max")
  }
}

function releaseIdFromHotsetBaseUrl(baseUrl: string) {
  const segment = baseUrl.replace(/\/$/, "").split("/").filter(Boolean).at(-1) || ""
  try {
    return decodeURIComponent(segment)
  } catch {
    return segment
  }
}

function isPublicAuthRequest(method: string, path: string[]) {
  return (
    (method === "POST" && path.length === 2 && path[0] === "auth" && path[1] === "access-request") ||
    (method === "GET" && path.length === 3 && path[0] === "auth" && path[1] === "invites")
  )
}
