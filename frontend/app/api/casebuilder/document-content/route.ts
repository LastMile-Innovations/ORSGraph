import { getServerSession } from "next-auth"
import type { NextRequest } from "next/server"
import { authOptions } from "@/lib/auth"
import { orsBackendApiBaseUrl } from "@/lib/ors-backend-api-url"

const API_KEY = process.env.ORS_API_KEY
const NO_STORE_HEADERS = { "cache-control": "no-store" }

export async function GET(request: NextRequest) {
  return forwardDocumentContent(request, "GET")
}

export async function HEAD(request: NextRequest) {
  return forwardDocumentContent(request, "HEAD")
}

async function forwardDocumentContent(request: NextRequest, method: "GET" | "HEAD") {
  const session = await getServerSession(authOptions)
  if (!session?.accessToken && !API_KEY) {
    return Response.json({ error: "Unauthorized" }, { status: 401, headers: NO_STORE_HEADERS })
  }

  const matterId = request.nextUrl.searchParams.get("matterId")
  const documentId = request.nextUrl.searchParams.get("documentId")
  if (!matterId || !documentId) {
    return Response.json({ error: "matterId and documentId are required" }, { status: 400, headers: NO_STORE_HEADERS })
  }

  const headers = new Headers()
  if (session?.accessToken) {
    headers.set("Authorization", `Bearer ${session.accessToken}`)
  } else if (API_KEY) {
    headers.set("x-api-key", API_KEY)
  }

  const upstream = await fetch(
    `${orsBackendApiBaseUrl()}/matters/${encodeURIComponent(matterId)}/documents/${encodeURIComponent(documentId)}/content`,
    {
      cache: "no-store",
      headers,
      method,
    },
  )

  if (!upstream.ok) {
    const body = await upstream.json().catch(() => ({}))
    return Response.json(
      { error: body.error || `Document content unavailable: ${upstream.status}` },
      { status: upstream.status, headers: NO_STORE_HEADERS },
    )
  }

  const responseHeaders = new Headers()
  for (const name of ["content-type", "content-disposition", "content-length", "accept-ranges"]) {
    const value = upstream.headers.get(name)
    if (value) responseHeaders.set(name, value)
  }
  responseHeaders.set("cache-control", "no-store")

  return new Response(method === "HEAD" ? null : upstream.body, {
    status: upstream.status,
    headers: responseHeaders,
  })
}
