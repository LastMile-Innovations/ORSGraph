import { notFound } from "next/navigation"
import { DocumentWorkspaceModal } from "@/components/casebuilder/document-workspace-modal"
import { getDocumentWorkspace, getMatterSettingsState, getMatterState } from "@/lib/casebuilder/server-api"

export default async function DocumentModalPage({ params }: PageProps<"/matters/[id]/documents/[docId]">) {
  const { id, docId } = await params
  const [matterState, workspaceState, settingsState] = await Promise.all([
    getMatterState(id),
    getDocumentWorkspace(id, docId),
    getMatterSettingsState(id),
  ])
  const matter = matterState.data
  if (!matter) notFound()
  const workspace = workspaceState.data
  if (!workspace) notFound()

  return <DocumentWorkspaceModal matter={matter} workspace={workspace} settings={settingsState.data?.effective ?? null} />
}
