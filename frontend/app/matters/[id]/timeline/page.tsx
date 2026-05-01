import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { TimelineView } from "@/components/casebuilder/timeline-view"
import { getMatterState } from "@/lib/casebuilder/api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function TimelinePage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} activeSection="timeline" dataState={matterState}>
      <TimelineView matter={matter} />
    </MatterShell>
  )
}
