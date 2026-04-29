import type { MatterSummary } from "@/lib/casebuilder/types"
import { TopNav } from "@/components/orsg/top-nav"
import { MatterSidebar } from "./matter-sidebar"

interface MatterShellProps {
  matter: MatterSummary
  children: React.ReactNode
  rightPanel?: React.ReactNode
  counts?: Parameters<typeof MatterSidebar>[0]["counts"]
}

export function MatterShell({ matter, children, rightPanel, counts }: MatterShellProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
      <TopNav />
      <div className="flex flex-1 overflow-hidden">
        <MatterSidebar matter={matter} counts={counts} />
        <main className="flex flex-1 flex-col overflow-hidden">{children}</main>
        {rightPanel && (
          <aside className="flex w-80 flex-col overflow-hidden border-l border-border bg-card">
            {rightPanel}
          </aside>
        )}
      </div>
    </div>
  )
}
