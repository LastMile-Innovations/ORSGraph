import { Shell } from "@/components/orsg/shell"
import { FactCheckWorkflowClient } from "@/components/orsg/fact-check/fact-check-client"

export default function FactCheckPage() {
  return (
    <Shell hideLeftRail>
      <FactCheckWorkflowClient />
    </Shell>
  )
}
