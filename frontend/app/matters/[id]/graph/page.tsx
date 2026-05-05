import { notFound } from "next/navigation"
import { MatterGraphView } from "@/components/casebuilder/matter-graph-view"
import { getMatterGraphState, getMatterState } from "@/lib/casebuilder/server-api"

export default async function MatterGraphPage({ params }: PageProps<"/matters/[id]/graph">) {
  const { id } = await params
  const [matterState, graphState] = await Promise.all([
    getMatterState(id),
    getMatterGraphState(id),
  ])
  const matter = matterState.data
  if (!matter) notFound()

  return <MatterGraphView matter={matter} graph={graphState.data} error={graphState.error} />
}
