import { Shell } from "@/components/orsg/shell"
import { ComplaintWorkflowClient } from "@/components/orsg/complaint/complaint-client"

export default function ComplaintPage() {
  return (
    <Shell hideLeftRail>
      <ComplaintWorkflowClient />
    </Shell>
  )
}
