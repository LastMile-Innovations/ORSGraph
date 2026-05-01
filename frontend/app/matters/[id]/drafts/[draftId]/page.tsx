import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DraftEditor } from "@/components/casebuilder/draft-editor"
import { getMatterState } from "@/lib/casebuilder/api"

interface PageProps {
  params: Promise<{ id: string; draftId: string }>
}

export default async function DraftPage({ params }: PageProps) {
  const { id, draftId } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  const draft = matter.drafts.find((d) => d.id === draftId)
  if (!draft) notFound()

  return (
    <MatterShell matter={matter} activeSection="drafts" dataState={matterState}>
      <DraftEditor matter={matter} draft={draft} />
    </MatterShell>
  )
}
