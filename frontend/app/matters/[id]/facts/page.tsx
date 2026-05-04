import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { FactsBoard } from "@/components/casebuilder/facts-board"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function FactsPage({ params }: PageProps<"/matters/[id]/facts">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} activeSection="facts" dataState={matterState}>
      <FactsBoard matter={matter} />
    </MatterShell>
  )
}
