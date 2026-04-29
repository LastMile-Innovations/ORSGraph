import { Shell } from "@/components/orsg/shell"
import { GraphViewer } from "@/components/graph/GraphViewer"

export default function GraphPage() {
  return (
    <Shell hideLeftRail>
      <GraphViewer />
    </Shell>
  )
}
