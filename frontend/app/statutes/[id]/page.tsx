import { notFound } from "next/navigation"
import Link from "next/link"
import { Shell } from "@/components/orsg/shell"
import { StatuteHeader } from "@/components/orsg/statute/statute-header"
import { StatuteTabs } from "@/components/orsg/statute/statute-tabs"
import { StatuteInspectorDrawer, StatuteRightInspector } from "@/components/orsg/statute/statute-right-inspector"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { getCachedStatutePageDataState } from "@/lib/authority-server-cache"
import type { DataSource } from "@/lib/data-state"

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
  const state = await getCachedStatutePageDataState(decoded)
  const data = state.data
  if (!data && state.source === "empty") notFound()
  if (!data) {
    return (
      <Shell>
        <DataStateBanner source={state.source} error={state.error} label="Statute data" />
        <StatuteUnavailable id={decoded} source={state.source} error={state.error} />
      </Shell>
    )
  }

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

function StatuteUnavailable({ id, source, error }: { id: string; source: DataSource; error?: string }) {
  return (
    <div className="flex flex-1 items-center justify-center bg-background p-6">
      <div className="w-full max-w-lg rounded border border-border bg-card p-6">
        <p className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          {source === "offline" ? "API offline" : "API error"}
        </p>
        <h1 className="mt-2 text-base font-semibold text-foreground">Statute data unavailable</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          The live ORSGraph API did not return data for <span className="font-mono text-foreground">{id}</span>.
          {error ? ` ${error}` : ""}
        </p>
        <Link
          href="/statutes"
          className="mt-4 inline-flex h-9 items-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground hover:bg-primary/90"
        >
          Statute index
        </Link>
      </div>
    </div>
  )
}
