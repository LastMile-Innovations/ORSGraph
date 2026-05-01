import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DocumentViewer } from "@/components/casebuilder/document-viewer"
import { getMatterState } from "@/lib/casebuilder/api"

interface PageProps {
  params: Promise<{ id: string; docId: string }>
}

export default async function DocumentPage({ params }: PageProps) {
  const { id, docId } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  const document = matter.documents.find((d) => d.id === docId)
  if (!document) notFound()

  return (
    <MatterShell matter={matter} activeSection="documents" dataState={matterState}>
      <DocumentViewer matter={matter} document={document} />
    </MatterShell>
  )
}
