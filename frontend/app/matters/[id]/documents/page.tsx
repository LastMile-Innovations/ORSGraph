import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DocumentLibrary } from "@/components/casebuilder/document-library"
import { getMatterState } from "@/lib/casebuilder/api"

export default async function DocumentsPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} dataState={matterState}>
      <DocumentLibrary matter={matter} documents={matter.documents} />
    </MatterShell>
  )
}
