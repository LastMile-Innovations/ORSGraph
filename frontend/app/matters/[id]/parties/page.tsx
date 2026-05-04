import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { PartyMap } from "@/components/casebuilder/party-map"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function PartiesPage({ params }: PageProps<"/matters/[id]/parties">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <MatterShell matter={matter} activeSection="parties" dataState={matterState}>
      <PartyMap matter={matter} />
    </MatterShell>
  )
}
