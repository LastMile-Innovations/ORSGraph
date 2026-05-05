import { notFound } from "next/navigation"
import { MatterQcPanel } from "@/components/casebuilder/matter-qc-panel"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function MatterQcPage({ params }: PageProps<"/matters/[id]/qc">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  return <MatterQcPanel matter={matter} />
}
