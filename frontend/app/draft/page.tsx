import type { Metadata } from "next"
import { Shell } from "@/components/orsg/shell"
import { DraftStudioClient } from "@/components/orsg/draft/draft-studio-client"
import { complaintAnalysis } from "@/lib/mock-complaint"

export const metadata: Metadata = {
  title: "Draft Studio | ORSGraph",
  description: "Draft and review an Oregon answer with authority, evidence, and QC context.",
}

export default function DraftPage() {
  return (
    <Shell hideLeftRail>
      <DraftStudioClient analysis={complaintAnalysis} />
    </Shell>
  )
}
