import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DeadlinesView } from "@/components/casebuilder/deadlines-view"
import { getMatterById } from "@/lib/casebuilder/mock-matters"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function DeadlinesPage({ params }: PageProps) {
  const { id } = await params
  const matter = getMatterById(id)
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} activeSection="deadlines">
      <DeadlinesView matter={matter} />
    </MatterShell>
  )
}
