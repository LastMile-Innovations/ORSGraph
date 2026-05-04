import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DocumentWorkspace } from "@/components/casebuilder/document-workspace"
import { getDocumentWorkspace, getMatterState } from "@/lib/casebuilder/server-api"

interface PageProps {
  params: Promise<{ id: string; docId: string }>
}

export default async function DocumentPage({ params }: PageProps) {
  const { id, docId } = await params
  const matterState = await getMatterState(id)
  const workspaceState = await getDocumentWorkspace(id, docId)
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
