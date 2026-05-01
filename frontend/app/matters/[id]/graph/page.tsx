import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { MatterGraphView } from "@/components/casebuilder/matter-graph-view"
import { getMatterGraphState, getMatterState } from "@/lib/casebuilder/api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function MatterGraphPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  const graphState = await getMatterGraphState(matter.id)

  return (
    <MatterShell matter={matter} activeSection="graph" dataState={matterState.source === "live" ? graphState : matterState}>
      <MatterGraphView matter={matter} graph={graphState.data} error={graphState.error} />
    </MatterShell>
  )
}
