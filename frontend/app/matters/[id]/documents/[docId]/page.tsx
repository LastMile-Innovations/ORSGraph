import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DocumentViewer } from "@/components/casebuilder/document-viewer"
import { getMatterById } from "@/lib/casebuilder/mock-matters"

interface PageProps {
  params: Promise<{ id: string; docId: string }>
}

export default async function DocumentPage({ params }: PageProps) {
  const { id, docId } = await params
  const matter = getMatterById(id)
  if (!matter) notFound()
  const document = matter.documents.find((d) => d.id === docId)
  if (!document) notFound()

  return (
    <MatterShell matter={matter} activeSection="documents">
      <DocumentViewer matter={matter} document={document} />
    </MatterShell>
  )
}
