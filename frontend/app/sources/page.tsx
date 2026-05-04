import { Shell } from "@/components/orsg/shell"
import { SourcesClient } from "@/components/orsg/sources/sources-client"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { getCachedSourcesState } from "@/lib/authority-server-cache"

export default async function SourcesPage() {
  const state = await getCachedSourcesState({ limit: 200 })
  return (
    <Shell>
      <DataStateBanner source={state.source} label="Source index" error={state.error} />
      <SourcesClient sources={state.data.items} />
    </Shell>
  )
}
