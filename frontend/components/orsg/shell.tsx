import { TopNav } from "./top-nav"
import { LeftRail } from "./left-rail"
import { getSidebarState } from "@/lib/api"
import { Suspense } from "react"
import { headers } from "next/headers"

interface ShellProps {
  children: React.ReactNode
  rightPanel?: React.ReactNode
  hideLeftRail?: boolean
}

export async function Shell({ children, rightPanel, hideLeftRail = false }: ShellProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <TopNav />
      <div className="flex flex-1 overflow-hidden">
        {!hideLeftRail && (
          <div className="hidden shrink-0 lg:flex">
            <Suspense fallback={<div className="w-72 border-r border-border bg-sidebar" />}>
              <LeftRailSlot />
            </Suspense>
          </div>
        )}
        <main id="app-main" className="flex min-w-0 flex-1 flex-col overflow-hidden bg-background" tabIndex={-1}>
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

async function LeftRailSlot() {
  const cookie = (await headers()).get("cookie")
  const sidebarState = await getSidebarState(cookie ? { headers: { cookie } } : undefined)
  return <LeftRail initialState={sidebarState} />
}
