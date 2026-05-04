import { notFound } from "next/navigation"
import { ComplaintEditorWorkbench } from "@/components/casebuilder/complaint-editor-workbench"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { getComplaintState, getMatterState } from "@/lib/casebuilder/server-api"

export default async function ComplaintBuilderPage({ params }: PageProps<"/matters/[id]/complaint">) {
  const { id } = await params
  const [matterState, complaintState] = await Promise.all([
    getMatterState(id),
    getComplaintState(id),
  ])
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <MatterShell matter={matter} activeSection="complaint" dataState={matterState}>
      <ComplaintEditorWorkbench matter={matter} complaint={complaintState.data} mode="home" />
    </MatterShell>
  )
}
