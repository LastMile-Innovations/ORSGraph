import type { MetadataRoute } from "next"

export default function robots(): MetadataRoute.Robots {
  const sitemap = new URL("/sitemap.xml", siteOrigin()).toString()

  return {
    rules: {
      userAgent: "*",
      allow: "/",
      disallow: [
        "/admin/",
        "/api/",
        "/auth/",
        "/casebuilder/",
        "/dashboard/",
        "/matters/",
        "/onboarding/",
      ],
    },
    sitemap,
  }
}

function siteOrigin() {
  if (process.env.NEXTAUTH_URL) return process.env.NEXTAUTH_URL.replace(/\/$/, "")
  if (process.env.RAILWAY_PUBLIC_DOMAIN) return `https://${process.env.RAILWAY_PUBLIC_DOMAIN}`
  return "http://localhost:3000"
}
