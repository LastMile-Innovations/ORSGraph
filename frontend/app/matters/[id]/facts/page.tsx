import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { FactsBoard } from "@/components/casebuilder/facts-board"
import { getMatterById } from "@/lib/casebuilder/mock-matters"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function FactsPage({ params }: PageProps) {
  const { id } = await params
  const matter = getMatterById(id)
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} activeSection="facts">
      <FactsBoard matter={matter} />
    </MatterShell>
  )
}
