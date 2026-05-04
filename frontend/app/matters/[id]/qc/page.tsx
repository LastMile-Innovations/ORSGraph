import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { MatterQcPanel } from "@/components/casebuilder/matter-qc-panel"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function MatterQcPage({ params }: PageProps<"/matters/[id]/qc">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <MatterShell matter={matter} activeSection="qc" dataState={matterState}>
      <MatterQcPanel matter={matter} />
    </MatterShell>
  )
}
