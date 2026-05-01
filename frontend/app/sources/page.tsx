import { Shell } from "@/components/orsg/shell"
import { SourcesClient } from "@/components/orsg/sources/sources-client"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { getSourcesState } from "@/lib/api"

export default async function SourcesPage() {
  const state = await getSourcesState({ limit: 200 })
  return (
    <Shell>
      <DataStateBanner source={state.source} label="Source index" error={state.error} />
      <SourcesClient sources={state.data.items} />
    </Shell>
  )
}
