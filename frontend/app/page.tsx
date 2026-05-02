import type { Metadata } from "next"
import { MarketingLanding } from "@/components/marketing/marketing-landing"

export const metadata: Metadata = {
  title: "ORSGraph - Source-First Legal OS",
  description:
    "A legal operating environment for transforming statutes, evidence, authorities, and filings into source-backed case work.",
}

export default function LandingPage() {
  return <MarketingLanding />
}
