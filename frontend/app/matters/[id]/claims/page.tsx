import { notFound } from "next/navigation"
import { ClaimsBuilder } from "@/components/casebuilder/claims-builder"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function ClaimsPage({ params }: PageProps<"/matters/[id]/claims">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()
  return <ClaimsBuilder matter={matter} />
}
