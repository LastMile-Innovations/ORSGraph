export function siteOrigin() {
  if (process.env.NEXTAUTH_URL) return process.env.NEXTAUTH_URL.replace(/\/$/, "")
  if (process.env.RAILWAY_PUBLIC_DOMAIN) return `https://${process.env.RAILWAY_PUBLIC_DOMAIN}`
  return "http://localhost:3000"
}
