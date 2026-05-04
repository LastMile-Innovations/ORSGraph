import { notFound } from "next/navigation"
import { Shell } from "@/components/orsg/shell"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { ProvisionInspectorClient } from "@/components/orsg/provision/provision-inspector-client"
import { getCachedProvisionInspectorDataState } from "@/lib/authority-server-cache"

export default async function ProvisionPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const decoded = decodeURIComponent(id)
  const state = await getCachedProvisionInspectorDataState(decoded)
  const data = state.data
  if (!data) return notFound()
  return (
    <Shell>
      <DataStateBanner source={state.source} error={state.error} label="Provision data" />
      <ProvisionInspectorClient data={data} />
    </Shell>
  )
}
