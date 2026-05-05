import { notFound } from "next/navigation"
import { DocumentLibrary } from "@/components/casebuilder/document-library"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function DocumentsPage({ params }: PageProps<"/matters/[id]/documents">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return <DocumentLibrary matter={matter} documents={matter.documents} />
}
