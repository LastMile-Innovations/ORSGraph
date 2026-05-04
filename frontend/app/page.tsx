import type { Metadata } from "next"
import { MarketingLanding } from "@/components/marketing/marketing-landing"

export const metadata: Metadata = {
  title: "ORSGraph - Source-First Legal Workspace",
  description:
    "Research law, structure matters, connect evidence, and move filings through source-backed quality control.",
}

export const unstable_instant = {
  prefetch: "static",
}

export default function LandingPage() {
  return <MarketingLanding />
}
