import { notFound } from "next/navigation"
import { Shell } from "@/components/orsg/shell"
import { StatuteHeader } from "@/components/orsg/statute/statute-header"
import { StatuteTabs } from "@/components/orsg/statute/statute-tabs"
import { StatuteInspectorDrawer, StatuteRightInspector } from "@/components/orsg/statute/statute-right-inspector"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { getStatutePageDataState } from "@/lib/api"

type StatuteDetailParams = {
  tab?: string
}

export default async function StatutePage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>
  searchParams: Promise<StatuteDetailParams>
}) {
  const { id } = await params
  const query = await searchParams
  const decoded = decodeURIComponent(id)
  const state = await getStatutePageDataState(decoded)
  const data = state.data
  if (!data) notFound()

  return (
    <Shell rightPanel={<StatuteRightInspector data={data} />}>
      <div className="flex flex-1 flex-col overflow-hidden">
        <DataStateBanner source={state.source} error={state.error} label="Statute data" />
        <StatuteHeader data={data} inspectorAction={<StatuteInspectorDrawer data={data} />} />
        <StatuteTabs data={data} initialTab={query.tab} />
      </div>
    </Shell>
  )
}
