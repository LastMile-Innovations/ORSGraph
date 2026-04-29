import { Shell } from "@/components/orsg/shell"
import { ComplaintClient } from "@/components/orsg/complaint/complaint-client"
import { complaintAnalysis } from "@/lib/mock-complaint"

export default function ComplaintPage() {
  return (
    <Shell hideLeftRail>
      <ComplaintClient analysis={complaintAnalysis} />
    </Shell>
  )
}
