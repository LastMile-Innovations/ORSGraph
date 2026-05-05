import { notFound } from "next/navigation"
import { PartyMap } from "@/components/casebuilder/party-map"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function PartiesPage({ params }: PageProps<"/matters/[id]/parties">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  return <PartyMap matter={matter} />
}
