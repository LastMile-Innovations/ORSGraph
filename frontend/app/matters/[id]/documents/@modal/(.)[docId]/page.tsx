import { notFound } from "next/navigation"
import { DocumentWorkspaceModal } from "@/components/casebuilder/document-workspace-modal"
import { getDocumentWorkspace, getMatterState } from "@/lib/casebuilder/server-api"

interface PageProps {
  params: Promise<{ id: string; docId: string }>
}

export default async function DocumentModalPage({ params }: PageProps) {
  const { id, docId } = await params
  const [matterState, workspaceState] = await Promise.all([
    getMatterState(id),
    getDocumentWorkspace(id, docId),
  ])
  const matter = matterState.data
  if (!matter) notFound()
  const workspace = workspaceState.data
  if (!workspace) notFound()

  return <DocumentWorkspaceModal matter={matter} workspace={workspace} />
}
