const DEFAULT_API_PROXY_BASE_URL = "/api/ors"
const DEFAULT_AUTHORITY_PROXY_BASE_URL = "/api/authority"

export function orsApiBaseUrl() {
  const base = process.env.NEXT_PUBLIC_ORS_API_PROXY_BASE_URL || DEFAULT_API_PROXY_BASE_URL
  if (base.startsWith("/") && typeof window === "undefined") {
    return `${serverOrigin()}${base}`
  }
  return base
}

export function orsAuthorityApiBaseUrl() {
  const base = process.env.NEXT_PUBLIC_ORS_AUTHORITY_API_PROXY_BASE_URL || DEFAULT_AUTHORITY_PROXY_BASE_URL
  if (base.startsWith("/") && typeof window === "undefined") {
    return `${serverOrigin()}${base}`
  }
  return base
}

function serverOrigin() {
  if (process.env.NEXTAUTH_URL) return process.env.NEXTAUTH_URL.replace(/\/$/, "")
  if (process.env.RAILWAY_PUBLIC_DOMAIN) return `https://${process.env.RAILWAY_PUBLIC_DOMAIN}`
  if (process.env.VERCEL_URL) return `https://${process.env.VERCEL_URL}`
  return "http://localhost:3000"
}
