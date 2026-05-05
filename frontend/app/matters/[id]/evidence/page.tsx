import { notFound } from "next/navigation"
import { EvidenceMatrix } from "@/components/casebuilder/evidence-matrix"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function EvidencePage({ params }: PageProps<"/matters/[id]/evidence">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return <EvidenceMatrix matter={matter} />
}
