import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { SourceDetailClient } from "@/components/orsg/sources/source-detail-client"
import { getCachedSourceDetailState } from "@/lib/authority-server-cache"
import { notFound } from "next/navigation"

export const unstable_instant = {
  prefetch: "static",
  unstable_disableValidation: true,
}

export default async function SourceDetailPage({ params }: PageProps<"/sources/[id]">) {
  const { id } = await params
  const decoded = decodeURIComponent(id)
  const state = await getCachedSourceDetailState(decoded)
  if (!state.data) notFound()
  return (
    <>
      <DataStateBanner source={state.source} label="Source detail" error={state.error} />
      <SourceDetailClient source={state.data.source} otherSources={state.data.related_sources} />
    </>
  )
}
