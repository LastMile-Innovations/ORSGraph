import { notFound } from "next/navigation"
import { DocumentWorkspace } from "@/components/casebuilder/document-workspace"
import { getDocumentWorkspace, getMatterSettingsState, getMatterState } from "@/lib/casebuilder/server-api"

export default async function DocumentPage({ params }: PageProps<"/matters/[id]/documents/[docId]">) {
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

  return <DocumentWorkspace matter={matter} workspace={workspace} settings={settingsState.data?.effective ?? null} />
}
