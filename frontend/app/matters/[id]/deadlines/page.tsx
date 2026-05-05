import { notFound } from "next/navigation"
import { DeadlinesView } from "@/components/casebuilder/deadlines-view"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function DeadlinesPage({ params }: PageProps<"/matters/[id]/deadlines">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return <DeadlinesView matter={matter} />
}
