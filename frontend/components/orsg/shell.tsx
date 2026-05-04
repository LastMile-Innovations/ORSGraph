import "server-only"

import { TopNavBoundary } from "./top-nav-boundary"
import { LeftRail } from "./left-rail"
import { MobileLeftRailSheet } from "./mobile-left-rail-sheet"
import { getSidebarState } from "@/lib/api"
import { headers } from "next/headers"
import { Suspense, cache } from "react"

interface ShellProps {
  children: React.ReactNode
  rightPanel?: React.ReactNode
  hideLeftRail?: boolean
}

export async function Shell({ children, rightPanel, hideLeftRail = false }: ShellProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <TopNavBoundary
        leftRailTrigger={
          hideLeftRail ? null : (
            <Suspense fallback={<div className="h-8 w-8 lg:hidden" aria-hidden="true" />}>
              <MobileLeftRailSlot />
            </Suspense>
          )
        }
      />
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
  const sidebarState = await loadSidebarState()
  return <LeftRail initialState={sidebarState} />
}

async function MobileLeftRailSlot() {
  const sidebarState = await loadSidebarState()
  return <MobileLeftRailSheet initialState={sidebarState} />
}

const loadSidebarStateForCookie = cache((cookie: string | null) => {
  return getSidebarState(cookie ? { headers: { cookie } } : undefined)
})

async function loadSidebarState() {
  const cookie = (await headers()).get("cookie")
  return loadSidebarStateForCookie(cookie)
}
