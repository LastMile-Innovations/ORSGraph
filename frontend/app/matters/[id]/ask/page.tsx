import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { AskMatter } from "@/components/casebuilder/ask-matter"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function AskMatterPage({ params }: PageProps<"/matters/[id]/ask">) {
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
