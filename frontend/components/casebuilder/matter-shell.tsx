import type { MatterSummary } from "@/lib/casebuilder/types"
import type { LoadSource } from "@/lib/casebuilder/api"
import { TopNav } from "@/components/orsg/top-nav"
import { MatterSidebar, MatterSidebarSheet } from "./matter-sidebar"
import { DataStateBanner } from "./data-state-banner"

interface MatterShellProps {
  matter: MatterSummary
  children: React.ReactNode
  rightPanel?: React.ReactNode
  counts?: Parameters<typeof MatterSidebar>[0]["counts"]
  activeSection?: string
  dataState?: { source: LoadSource; error?: string }
}

export function MatterShell({ matter, children, rightPanel, counts, dataState }: MatterShellProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
      <TopNav />
      <DataStateBanner source={dataState?.source} error={dataState?.error} />
      <div className="flex flex-1 overflow-hidden">
        <div className="hidden shrink-0 md:flex">
          <MatterSidebar matter={matter} counts={counts} />
        </div>
        <main id="app-main" className="flex flex-1 flex-col overflow-hidden" tabIndex={-1}>
          <MatterSidebarSheet matter={matter} counts={counts} />
          {children}
        </main>
        {rightPanel && (
          <aside className="flex w-80 flex-col overflow-hidden border-l border-border bg-card">
            {rightPanel}
          </aside>
        )}
      </div>
    </div>
  )
}
