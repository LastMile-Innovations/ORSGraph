import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { TimelineView } from "@/components/casebuilder/timeline-view"
import { getMatterById } from "@/lib/casebuilder/mock-matters"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function TimelinePage({ params }: PageProps) {
  const { id } = await params
  const matter = getMatterById(id)
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} activeSection="timeline">
      <TimelineView matter={matter} />
    </MatterShell>
  )
}
