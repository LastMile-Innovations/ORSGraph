import { notFound } from "next/navigation"
import { ComplaintEditorWorkbench } from "@/components/casebuilder/complaint-editor-workbench"
import { getComplaintState, getMatterState } from "@/lib/casebuilder/server-api"
import type { ComplaintWorkspaceSection } from "@/lib/casebuilder/routes"

const WORKSPACES: ComplaintWorkspaceSection[] = ["editor", "outline", "claims", "evidence", "qc", "preview", "export", "history"]

export default async function ComplaintWorkspacePage({ params }: PageProps<"/matters/[id]/complaint/[workspace]">) {
  const { id, workspace } = await params
  if (!isComplaintWorkspaceSection(workspace)) notFound()
  const [matterState, complaintState] = await Promise.all([
    getMatterState(id),
    getComplaintState(id),
  ])
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <ComplaintEditorWorkbench
      matter={matter}
      complaint={complaintState.data}
      mode={workspace}
    />
  )
}

function isComplaintWorkspaceSection(value: string): value is ComplaintWorkspaceSection {
  return WORKSPACES.includes(value as ComplaintWorkspaceSection)
}
