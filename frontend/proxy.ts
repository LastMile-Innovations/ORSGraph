import { NextRequest, NextResponse } from "next/server"
import { getToken } from "next-auth/jwt"

export async function proxy(request: NextRequest) {
  const { pathname, search } = request.nextUrl

  if (isPublicPath(pathname)) {
    return NextResponse.next()
  }

  if (pathname === "/matters") {
    return redirect(request, `/casebuilder${search}`)
  }

  if (pathname === "/matters/new") {
    return redirect(request, `/casebuilder/new${search}`)
  }

  if (pathname.startsWith("/matters/")) {
    return redirect(request, `/casebuilder/matters${pathname.slice("/matters".length)}${search}`)
  }

  const token = await getToken({ req: request, secret: process.env.NEXTAUTH_SECRET })
  if (!token) {
    const url = request.nextUrl.clone()
    url.pathname = "/auth/signin"
    url.search = ""
    url.searchParams.set("callbackUrl", publicCallbackUrl(request))
    return NextResponse.redirect(url)
  }

  const accessStatus = typeof token.accessStatus === "string" ? token.accessStatus : "unknown"
  const isAdmin = token.isAdmin === true
  if (accessStatus !== "active" && !isAdmin && !pathname.startsWith("/auth/pending")) {
    const url = request.nextUrl.clone()
    url.pathname = "/auth/pending"
    url.search = ""
    url.searchParams.set("callbackUrl", publicCallbackUrl(request))
    return NextResponse.redirect(url)
  }

  return NextResponse.next()
}

function isPublicPath(pathname: string) {
  return (
    pathname === "/" ||
    pathname.startsWith("/marketing/") ||
    pathname.startsWith("/auth/signin") ||
    pathname.startsWith("/auth/request-access") ||
    pathname.startsWith("/auth/error") ||
    pathname.startsWith("/auth/invite")
  )
}

function redirect(request: NextRequest, pathname: string) {
  const url = request.nextUrl.clone()
  url.pathname = pathname.split("?")[0] ?? pathname
  url.search = pathname.includes("?") ? pathname.slice(pathname.indexOf("?")) : ""
  return NextResponse.redirect(url)
}

function publicCallbackUrl(request: NextRequest) {
  const origin =
    process.env.NEXTAUTH_URL?.replace(/\/$/, "") ||
    `${request.headers.get("x-forwarded-proto") || request.nextUrl.protocol.replace(":", "")}://${request.headers.get("x-forwarded-host") || request.headers.get("host") || request.nextUrl.host}`
  return new URL(`${request.nextUrl.pathname}${request.nextUrl.search}`, origin).href
}

export const config = {
  matcher: [
    "/((?!api/|_next/static|_next/image|favicon.ico|robots.txt|sitemap.xml).*)",
  ],
}
