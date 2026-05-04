import type { MetadataRoute } from "next"
import { siteOrigin } from "./metadata"

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    {
      url: siteOrigin(),
      changeFrequency: "weekly",
      priority: 1,
    },
  ]
}
