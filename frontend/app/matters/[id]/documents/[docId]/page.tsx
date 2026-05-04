import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DocumentWorkspace } from "@/components/casebuilder/document-workspace"
import { getDocumentWorkspace, getMatterState } from "@/lib/casebuilder/server-api"

export default async function DocumentPage({ params }: PageProps<"/matters/[id]/documents/[docId]">) {
  const { id, docId } = await params
  const [matterState, workspaceState] = await Promise.all([
    getMatterState(id),
    getDocumentWorkspace(id, docId),
  ])
  const matter = matterState.data
  if (!matter) notFound()
  const workspace = workspaceState.data
  if (!workspace) notFound()

  return (
    <MatterShell
      matter={matter}
      activeSection="documents"
      dataState={{ source: workspaceState.source, error: workspaceState.error ?? matterState.error }}
    >
      <DocumentWorkspace matter={matter} workspace={workspace} />
    </MatterShell>
  )
}
