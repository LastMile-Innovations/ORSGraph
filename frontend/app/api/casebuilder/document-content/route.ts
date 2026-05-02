import { getServerSession } from "next-auth"
import { NextRequest } from "next/server"
import { authOptions } from "@/lib/auth"
import { orsBackendApiBaseUrl } from "@/lib/ors-api-url"

const API_KEY = process.env.ORS_API_KEY

export async function GET(request: NextRequest) {
  const session = await getServerSession(authOptions)
  if (!session?.accessToken && !API_KEY) {
    return Response.json({ error: "Unauthorized" }, { status: 401 })
  }

  const matterId = request.nextUrl.searchParams.get("matterId")
  const documentId = request.nextUrl.searchParams.get("documentId")
  if (!matterId || !documentId) {
    return Response.json({ error: "matterId and documentId are required" }, { status: 400 })
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
    },
  )

  if (!upstream.ok) {
    const body = await upstream.json().catch(() => ({}))
    return Response.json(
      { error: body.error || `Document content unavailable: ${upstream.status}` },
      { status: upstream.status },
    )
  }

  const responseHeaders = new Headers()
  for (const name of ["content-type", "content-disposition", "content-length", "accept-ranges"]) {
    const value = upstream.headers.get(name)
    if (value) responseHeaders.set(name, value)
  }
  responseHeaders.set("cache-control", "no-store")

  return new Response(upstream.body, {
    status: upstream.status,
    headers: responseHeaders,
  })
}
