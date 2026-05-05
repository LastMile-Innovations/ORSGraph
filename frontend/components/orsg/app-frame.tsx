"use client"

import { usePathname } from "next/navigation"
import { useEffect, useMemo, useState, type ReactNode } from "react"
import { getSidebarState, type SidebarData } from "@/lib/api"
import type { DataState } from "@/lib/data-state"
import { LeftRail } from "./left-rail"
import { MobileLeftRailSheet } from "./mobile-left-rail-sheet"
import { TopNavBoundary } from "./top-nav-boundary"

type AppFrameMode = "none" | "workspace" | "workspace-no-rail"

interface AppFrameProps {
  children: ReactNode
}

export function AppFrame({ children }: AppFrameProps) {
  const pathname = usePathname() || "/"
  const mode = useMemo(() => appFrameMode(pathname), [pathname])
  const showLeftRail = mode === "workspace"
  const [sidebarState, setSidebarState] = useState<DataState<SidebarData | null> | null>(null)

  useEffect(() => {
    if (!showLeftRail || sidebarState) return

    let disposed = false
    getSidebarState()
      .then((nextState) => {
        if (!disposed) setSidebarState(nextState)
      })
      .catch((error) => {
        if (disposed) return
        setSidebarState({
          source: "error",
          data: null,
          error: error instanceof Error ? error.message : "Sidebar unavailable",
        })
      })

    return () => {
      disposed = true
    }
  }, [showLeftRail, sidebarState])

  if (mode === "none") return <>{children}</>

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <TopNavBoundary
        leftRailTrigger={
          showLeftRail ? (
            sidebarState ? (
              <MobileLeftRailSheet initialState={sidebarState} />
            ) : (
              <div className="h-8 w-8 lg:hidden" aria-hidden="true" />
            )
          ) : null
        }
      />
      <div className="flex flex-1 overflow-hidden">
        {showLeftRail && (
          <div className="hidden shrink-0 lg:flex">
            {sidebarState ? <LeftRail initialState={sidebarState} /> : <LeftRailLoading />}
          </div>
        )}
        <main id="app-main" className="flex min-w-0 flex-1 flex-col overflow-hidden bg-background" tabIndex={-1}>
          {children}
        </main>
      </div>
    </div>
  )
}

function LeftRailLoading() {
  return (
    <aside className="flex h-full w-64 flex-col border-r border-sidebar-border bg-sidebar p-3" aria-hidden="true">
      <div className="h-8 rounded-md bg-sidebar-accent/70" />
      <div className="mt-4 space-y-2">
        {Array.from({ length: 8 }).map((_, index) => (
          <div key={index} className="h-7 rounded bg-sidebar-accent/60" />
        ))}
      </div>
      <div className="mt-auto space-y-2">
        <div className="h-3 w-28 rounded bg-sidebar-accent/50" />
        <div className="h-3 w-20 rounded bg-sidebar-accent/50" />
      </div>
    </aside>
  )
}

function appFrameMode(pathname: string): AppFrameMode {
  const path = normalizePath(pathname)

  if (path === "/" || path.startsWith("/auth") || path === "/onboarding") return "none"
  if (isMatterWorkspacePath(path)) return "none"
  if (isNoRailWorkspacePath(path)) return "workspace-no-rail"

  return "workspace"
}

function normalizePath(pathname: string) {
  if (pathname === "/") return pathname
  return pathname.replace(/\/+$/, "")
}

function isMatterWorkspacePath(pathname: string) {
  if (pathname.startsWith("/matters/") && pathname !== "/matters/new") return true
  if (pathname.startsWith("/casebuilder/matters/")) return true
  return false
}

function isNoRailWorkspacePath(pathname: string) {
  return (
    pathname === "/casebuilder" ||
    pathname === "/casebuilder/new" ||
    pathname === "/casebuilder/settings" ||
    pathname === "/complaint" ||
    pathname === "/dashboard" ||
    pathname === "/draft" ||
    pathname === "/fact-check" ||
    pathname === "/graph" ||
    pathname === "/matters" ||
    pathname === "/matters/new"
  )
}
