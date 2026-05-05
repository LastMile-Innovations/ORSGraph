import type { Metadata } from "next"
import { DraftStudioClient } from "@/components/orsg/draft/draft-studio-client"
import { complaintAnalysis } from "@/lib/mock-complaint"

export const metadata: Metadata = {
  title: "Draft Studio",
  description: "Draft and review an Oregon answer with authority, evidence, and QC context.",
}

export default function DraftPage() {
  return <DraftStudioClient analysis={complaintAnalysis} />
}
