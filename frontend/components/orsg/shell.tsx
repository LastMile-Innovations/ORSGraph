import { TopNav } from "./top-nav"
import { LeftRail } from "./left-rail"

interface ShellProps {
  children: React.ReactNode
  rightPanel?: React.ReactNode
  hideLeftRail?: boolean
}

export function Shell({ children, rightPanel, hideLeftRail = false }: ShellProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
      <TopNav />
      <div className="flex flex-1 overflow-hidden">
        {!hideLeftRail && <LeftRail />}
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
