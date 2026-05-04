import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { EvidenceMatrix } from "@/components/casebuilder/evidence-matrix"
import { getMatterState } from "@/lib/casebuilder/server-api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function EvidencePage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} activeSection="evidence" dataState={matterState}>
      <EvidenceMatrix matter={matter} />
    </MatterShell>
  )
}
