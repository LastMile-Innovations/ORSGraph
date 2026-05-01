import { NextRequest, NextResponse } from "next/server"

export function proxy(request: NextRequest) {
  const { pathname, search } = request.nextUrl

  if (pathname === "/matters") {
    return redirect(request, `/casebuilder${search}`)
  }

  if (pathname === "/matters/new") {
    return redirect(request, `/casebuilder/new${search}`)
  }

  if (pathname.startsWith("/matters/")) {
    return redirect(request, `/casebuilder/matters${pathname.slice("/matters".length)}${search}`)
  }

  return NextResponse.next()
}

function redirect(request: NextRequest, pathname: string) {
  const url = request.nextUrl.clone()
  url.pathname = pathname.split("?")[0] ?? pathname
  url.search = pathname.includes("?") ? pathname.slice(pathname.indexOf("?")) : ""
  return NextResponse.redirect(url)
}

export const config = {
  matcher: ["/matters/:path*"],
}
