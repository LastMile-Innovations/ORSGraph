import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { DraftEditor } from "@/components/casebuilder/draft-editor"
import { getMatterState } from "@/lib/casebuilder/server-api"
import { decodeRouteSegment } from "@/lib/casebuilder/routes"

export default async function DraftPage({ params }: PageProps<"/matters/[id]/drafts/[draftId]">) {
  const { id, draftId } = await params
  const decodedDraftId = decodeRouteSegment(draftId)
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  const draft = matter.drafts.find((d) => d.id === decodedDraftId || d.draft_id === decodedDraftId)
  if (!draft) notFound()

  return (
    <MatterShell matter={matter} activeSection="drafts" dataState={matterState}>
      <DraftEditor matter={matter} draft={draft} />
    </MatterShell>
  )
}
