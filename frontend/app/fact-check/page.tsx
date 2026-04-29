import { Shell } from "@/components/orsg/shell"
import { FactCheckClient } from "@/components/orsg/fact-check/fact-check-client"
import { factCheckReport } from "@/lib/mock-fact-check"

export default function FactCheckPage() {
  return (
    <Shell hideLeftRail>
      <FactCheckClient report={factCheckReport} />
    </Shell>
  )
}
