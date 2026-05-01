import { Shell } from "@/components/orsg/shell"
import { FactCheckClient } from "@/components/orsg/fact-check/fact-check-client"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { factCheckReport } from "@/lib/mock-fact-check"

export default function FactCheckPage() {
  return (
    <Shell hideLeftRail>
      <DataStateBanner source="demo" label="Fact-check demo" />
      <FactCheckClient report={factCheckReport} />
    </Shell>
  )
}
