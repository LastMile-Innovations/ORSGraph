import { getServerSession } from "next-auth"
import { NextRequest, NextResponse } from "next/server"
import { authOptions } from "@/lib/auth"
import { orsBackendApiBaseUrl } from "@/lib/ors-api-url"

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

function isPublicAuthRequest(method: string, path: string[]) {
  return (
    (method === "POST" && path.length === 2 && path[0] === "auth" && path[1] === "access-request") ||
    (method === "GET" && path.length === 3 && path[0] === "auth" && path[1] === "invites")
  )
}
