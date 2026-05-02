const DEFAULT_API_PROXY_BASE_URL = "/api/ors"

export function orsApiBaseUrl() {
  const base = process.env.NEXT_PUBLIC_ORS_API_PROXY_BASE_URL || DEFAULT_API_PROXY_BASE_URL
  if (base.startsWith("/") && typeof window === "undefined") {
    return `${serverOrigin()}${base}`
  }
  return base
}

export function orsBackendApiBaseUrl() {
  return (
    process.env.ORS_API_BASE_URL ||
    process.env.NEXT_PUBLIC_ORS_API_BASE_URL ||
    "http://localhost:8080/api/v1"
  ).replace(/\/$/, "")
}

function serverOrigin() {
  if (process.env.NEXTAUTH_URL) return process.env.NEXTAUTH_URL.replace(/\/$/, "")
  if (process.env.RAILWAY_PUBLIC_DOMAIN) return `https://${process.env.RAILWAY_PUBLIC_DOMAIN}`
  if (process.env.VERCEL_URL) return `https://${process.env.VERCEL_URL}`
  return "http://localhost:3000"
}

