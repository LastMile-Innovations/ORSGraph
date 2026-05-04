import { NextRequest, NextResponse } from "next/server"
import { getToken } from "next-auth/jwt"

const PUBLIC_PATHS = [
  "/",
  "/auth/signin",
  "/auth/request-access",
  "/auth/error",
]

const PUBLIC_PATH_PREFIXES = [
  "/marketing/",
  "/auth/invite",
]

export async function proxy(request: NextRequest) {
  const { pathname, search } = request.nextUrl

  if (isPublicPath(pathname)) {
    return NextResponse.next()
  }

  const legacyMatterPath = canonicalMatterPath(pathname, search)
  if (legacyMatterPath) {
    return redirect(request, legacyMatterPath)
  }

  const token = await getToken({ req: request, secret: process.env.NEXTAUTH_SECRET })
  if (!token) {
    const url = request.nextUrl.clone()
    url.pathname = "/auth/signin"
    url.search = ""
    url.searchParams.set("callbackUrl", callbackPath(request))
    return NextResponse.redirect(url)
  }

  const accessStatus = typeof token.accessStatus === "string" ? token.accessStatus : "unknown"
  const isAdmin = token.isAdmin === true
  if (accessStatus !== "active" && !isAdmin && !pathname.startsWith("/auth/pending")) {
    const url = request.nextUrl.clone()
    url.pathname = "/auth/pending"
    url.search = ""
    url.searchParams.set("callbackUrl", callbackPath(request))
    return NextResponse.redirect(url)
  }

  return NextResponse.next()
}

function isPublicPath(pathname: string) {
  return PUBLIC_PATHS.includes(pathname) || PUBLIC_PATH_PREFIXES.some((prefix) => matchesPathOrDescendant(pathname, prefix))
}

function matchesPathOrDescendant(pathname: string, prefix: string) {
  if (prefix.endsWith("/")) return pathname.startsWith(prefix)
  return pathname === prefix || pathname.startsWith(`${prefix}/`)
}

function canonicalMatterPath(pathname: string, search: string) {
  if (pathname === "/matters") return `/casebuilder${search}`
  if (pathname === "/matters/new") return `/casebuilder/new${search}`
  if (pathname.startsWith("/matters/")) return `/casebuilder/matters${pathname.slice("/matters".length)}${search}`
  return null
}

function redirect(request: NextRequest, pathname: string) {
  const url = request.nextUrl.clone()
  url.pathname = pathname.split("?")[0] ?? pathname
  url.search = pathname.includes("?") ? pathname.slice(pathname.indexOf("?")) : ""
  return NextResponse.redirect(url)
}

function callbackPath(request: NextRequest) {
  return `${request.nextUrl.pathname}${request.nextUrl.search}`
}

export const config = {
  matcher: [
    "/((?!api|_next/static|_next/image|favicon.ico|robots.txt|sitemap.xml).*)",
  ],
}
