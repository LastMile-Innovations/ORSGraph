import { notFound } from "next/navigation"
import { ComplaintEditorWorkbench } from "@/components/casebuilder/complaint-editor-workbench"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { getComplaintState, getMatterState } from "@/lib/casebuilder/server-api"
import type { ComplaintWorkspaceSection } from "@/lib/casebuilder/routes"

const WORKSPACES: ComplaintWorkspaceSection[] = ["editor", "outline", "claims", "evidence", "qc", "preview", "export", "history"]

interface PageProps {
  params: Promise<{ id: string; workspace: string }>
}

export default async function ComplaintWorkspacePage({ params }: PageProps) {
  const { id, workspace } = await params
  if (!isComplaintWorkspaceSection(workspace)) notFound()
  const [matterState, complaintState] = await Promise.all([
    getMatterState(id),
    getComplaintState(id),
  ])
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <MatterShell matter={matter} activeSection="complaint" dataState={matterState}>
      <ComplaintEditorWorkbench
        matter={matter}
        complaint={complaintState.data}
        mode={workspace}
      />
    </MatterShell>
  )
}

function isComplaintWorkspaceSection(value: string): value is ComplaintWorkspaceSection {
  return WORKSPACES.includes(value as ComplaintWorkspaceSection)
}
