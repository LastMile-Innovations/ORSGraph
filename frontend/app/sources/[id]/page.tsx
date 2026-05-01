import { Shell } from "@/components/orsg/shell"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { SourceDetailClient } from "@/components/orsg/sources/source-detail-client"
import { getSourceDetailState } from "@/lib/api"
import { notFound } from "next/navigation"

export default async function SourceDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const decoded = decodeURIComponent(id)
  const state = await getSourceDetailState(decoded)
  if (!state.data) notFound()
  return (
    <Shell>
      <DataStateBanner source={state.source} label="Source detail" error={state.error} />
      <SourceDetailClient source={state.data.source} otherSources={state.data.related_sources} />
    </Shell>
  )
}
