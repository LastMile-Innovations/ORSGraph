import { TopNav } from "./top-nav"
import { LeftRail } from "./left-rail"
import { getSidebarState } from "@/lib/api"

interface ShellProps {
  children: React.ReactNode
  rightPanel?: React.ReactNode
  hideLeftRail?: boolean
}

export async function Shell({ children, rightPanel, hideLeftRail = false }: ShellProps) {
  const sidebarState = hideLeftRail ? null : await getSidebarState()

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
      <TopNav />
      <div className="flex flex-1 overflow-hidden">
        {!hideLeftRail && (
          <div className="hidden shrink-0 lg:flex">
            <LeftRail initialState={sidebarState} />
          </div>
        )}
        <main id="app-main" className="flex min-w-0 flex-1 flex-col overflow-hidden" tabIndex={-1}>
          {children}
        </main>
        {rightPanel && (
          <aside className="hidden w-80 shrink-0 flex-col overflow-hidden border-l border-border bg-card xl:flex">
            {rightPanel}
          </aside>
        )}
      </div>
    </div>
  )
}
