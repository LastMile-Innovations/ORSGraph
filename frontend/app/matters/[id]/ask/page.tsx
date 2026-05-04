import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { AskMatter } from "@/components/casebuilder/ask-matter"
import { getMatterState } from "@/lib/casebuilder/server-api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function AskMatterPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} activeSection="ask" dataState={matterState}>
      <AskMatter matter={matter} />
    </MatterShell>
  )
}
