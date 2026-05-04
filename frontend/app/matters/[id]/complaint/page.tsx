import { notFound } from "next/navigation"
import { ComplaintEditorWorkbench } from "@/components/casebuilder/complaint-editor-workbench"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { getComplaintState, getMatterState } from "@/lib/casebuilder/server-api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function ComplaintBuilderPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  const complaintState = await getComplaintState(matter.id)

  return (
    <MatterShell matter={matter} activeSection="complaint" dataState={matterState}>
      <ComplaintEditorWorkbench matter={matter} complaint={complaintState.data} mode="home" />
    </MatterShell>
  )
}
