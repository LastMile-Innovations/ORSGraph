import { notFound } from "next/navigation"
import { ComplaintBuilderPanel } from "@/components/casebuilder/complaint-builder-panel"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { getMatterState } from "@/lib/casebuilder/api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function ComplaintBuilderPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  const complaintDraft = matter.drafts.find((draft) => draft.kind === "complaint")

  return (
    <MatterShell matter={matter} activeSection="complaint" dataState={matterState}>
      <ComplaintBuilderPanel matter={matter} complaintDraft={complaintDraft} />
    </MatterShell>
  )
}
