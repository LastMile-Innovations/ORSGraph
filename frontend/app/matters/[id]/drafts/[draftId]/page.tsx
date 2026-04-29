import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DraftEditor } from "@/components/casebuilder/draft-editor"
import { getMatterById } from "@/lib/casebuilder/mock-matters"

interface PageProps {
  params: Promise<{ id: string; draftId: string }>
}

export default async function DraftPage({ params }: PageProps) {
  const { id, draftId } = await params
  const matter = getMatterById(id)
  if (!matter) notFound()
  const draft = matter.drafts.find((d) => d.id === draftId)
  if (!draft) notFound()

  return (
    <MatterShell matter={matter} activeSection="drafts">
      <DraftEditor matter={matter} draft={draft} />
    </MatterShell>
  )
}
