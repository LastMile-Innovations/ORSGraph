"use client"

import { useEffect, useState } from "react"
import type { StatutePageResponse } from "@/lib/types"
import { StatuteHeader } from "./statute-header"
import { StatuteInspectorDrawer, StatuteRightInspector } from "./statute-right-inspector"
import { StatuteTabs } from "./statute-tabs"

export function StatuteDetailWorkspace({
  data,
  initialTab,
}: {
  data: StatutePageResponse
  initialTab?: string
}) {
  const [statuteData, setStatuteData] = useState(data)

  useEffect(() => {
    setStatuteData(data)
  }, [data])

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
        <StatuteHeader data={statuteData} inspectorAction={<StatuteInspectorDrawer data={statuteData} />} />
        <StatuteTabs data={statuteData} initialTab={initialTab} onDataChange={setStatuteData} />
      </div>
      <aside className="hidden w-80 shrink-0 flex-col overflow-hidden border-l border-border bg-card xl:flex">
        <StatuteRightInspector data={statuteData} />
      </aside>
    </div>
  )
}
