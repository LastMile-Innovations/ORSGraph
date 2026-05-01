import { Shell } from "@/components/orsg/shell"
import { ComplaintClient } from "@/components/orsg/complaint/complaint-client"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { complaintAnalysis } from "@/lib/mock-complaint"

export default function ComplaintPage() {
  return (
    <Shell hideLeftRail>
      <DataStateBanner source="demo" label="Complaint analyzer demo" />
      <ComplaintClient analysis={complaintAnalysis} />
    </Shell>
  )
}
