import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { EvidenceMatrix } from "@/components/casebuilder/evidence-matrix"
import { getMatterById } from "@/lib/casebuilder/mock-matters"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function EvidencePage({ params }: PageProps) {
  const { id } = await params
  const matter = getMatterById(id)
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} activeSection="evidence">
      <EvidenceMatrix matter={matter} />
    </MatterShell>
  )
}
