import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { FactsBoard } from "@/components/casebuilder/facts-board"
import { getMatterState } from "@/lib/casebuilder/server-api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function FactsPage({ params }: PageProps) {
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
