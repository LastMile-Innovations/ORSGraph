import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DeadlinesView } from "@/components/casebuilder/deadlines-view"
import { getMatterState } from "@/lib/casebuilder/server-api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function DeadlinesPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} activeSection="deadlines" dataState={matterState}>
      <DeadlinesView matter={matter} />
    </MatterShell>
  )
}
