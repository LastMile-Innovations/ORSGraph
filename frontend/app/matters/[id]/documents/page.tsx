import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DocumentLibrary } from "@/components/casebuilder/document-library"
import { getDocumentsByMatter, getMatterById } from "@/lib/casebuilder/mock-matters"

export default async function DocumentsPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const matter = getMatterById(id)
  if (!matter) notFound()
  const documents = getDocumentsByMatter(id)
  return (
    <MatterShell matter={matter}>
      <DocumentLibrary matter={matter} documents={documents} />
    </MatterShell>
  )
}
