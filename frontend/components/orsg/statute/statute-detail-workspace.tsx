"use client"

import { Suspense, useEffect, useState } from "react"
import type { StatutePageResponse } from "@/lib/types"
import { StatuteHeader } from "./statute-header"
import { StatuteInspectorDrawer, StatuteRightInspector } from "./statute-right-inspector"
import { StatuteTabs } from "./statute-tabs"
import { statuteLoadedStateFor } from "./load-state"

export function StatuteDetailWorkspace({
  data,
  initialTab,
}: {
  data: StatutePageResponse
  initialTab?: string
}) {
  const [statuteData, setStatuteData] = useState(data)
  const [loadedState, setLoadedState] = useState(() => statuteLoadedStateFor(data))

  useEffect(() => {
    setStatuteData(data)
    setLoadedState(statuteLoadedStateFor(data))
  }, [data])

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
        <StatuteHeader
          data={statuteData}
          loadedState={loadedState}
          inspectorAction={<StatuteInspectorDrawer data={statuteData} loadedState={loadedState} />}
        />
        <Suspense fallback={<StatuteTabsFallback />}>
          <StatuteTabs data={statuteData} initialTab={initialTab} onDataChange={setStatuteData} onLoadedChange={setLoadedState} />
        </Suspense>
      </div>
      <aside className="hidden w-[26rem] shrink-0 flex-col overflow-hidden border-l border-border bg-card xl:flex">
        <StatuteRightInspector data={statuteData} loadedState={loadedState} />
      </aside>
    </div>
  )
}

function StatuteTabsFallback() {
  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex gap-2 border-b border-border bg-card px-6 py-3">
        {Array.from({ length: 6 }).map((_, index) => (
          <div key={index} className="h-7 w-20 animate-pulse rounded bg-muted" />
        ))}
      </div>
      <div className="space-y-3 p-6">
        <div className="h-4 w-32 animate-pulse rounded bg-muted" />
        <div className="h-24 animate-pulse rounded border border-border bg-card" />
        <div className="h-24 animate-pulse rounded border border-border bg-card" />
      </div>
    </div>
  )
}
