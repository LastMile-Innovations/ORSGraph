import { notFound } from "next/navigation"
import { DraftsList } from "@/components/casebuilder/drafts-list"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function DraftsPage({ params }: PageProps<"/matters/[id]/drafts">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return <DraftsList matter={matter} />
}
