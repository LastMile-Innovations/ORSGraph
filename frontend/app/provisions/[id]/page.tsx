import { notFound } from "next/navigation"
import { Shell } from "@/components/orsg/shell"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { ProvisionInspectorClient } from "@/components/orsg/provision/provision-inspector-client"
import { getCachedProvisionInspectorDataState } from "@/lib/authority-server-cache"

export const unstable_instant = {
  prefetch: "static",
  unstable_disableValidation: true,
}

export default async function ProvisionPage({ params }: PageProps<"/provisions/[id]">) {
  const { id } = await params
  const decoded = decodeURIComponent(id)
  const state = await getCachedProvisionInspectorDataState(decoded)
  const data = state.data
  if (!data) notFound()
  return (
    <Shell>
      <DataStateBanner source={state.source} error={state.error} label="Provision data" />
      <ProvisionInspectorClient data={data} />
    </Shell>
  )
}
