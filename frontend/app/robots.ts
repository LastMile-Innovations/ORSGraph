import type { MetadataRoute } from "next"
import { siteOrigin } from "./metadata"

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
