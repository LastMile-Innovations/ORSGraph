"use client"

import { Suspense, useState } from "react"
import type { MatterSummary } from "@/lib/casebuilder/types"
import type { LoadSource } from "@/lib/casebuilder/api"
import { TopNavBoundary } from "@/components/orsg/top-nav-boundary"
import { MatterSidebar, MatterSidebarSheet } from "./matter-sidebar"
import { MatterUploadControls } from "./matter-upload-controls"
import { DataStateBanner } from "./data-state-banner"
import { Button } from "@/components/ui/button"
import { Maximize2, Minimize2 } from "lucide-react"
import { cn } from "@/lib/utils"

interface MatterShellProps {
  matter: MatterSummary
  children: React.ReactNode
  rightPanel?: React.ReactNode
  counts?: Parameters<typeof MatterSidebar>[0]["counts"]
  activeSection?: string
  dataState?: { source: LoadSource; error?: string }
}

export function MatterShell({ matter, children, rightPanel, counts, dataState }: MatterShellProps) {
  const [isFocusMode, setIsFocusMode] = useState(false)

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
      <div className="relative flex items-center justify-between border-b border-border pr-4">
        <div className="min-w-0 flex-1">
          <TopNavBoundary />
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setIsFocusMode(!isFocusMode)}
            className="h-8 w-8 p-0 text-muted-foreground hover:text-foreground"
            title={isFocusMode ? "Exit Focus Mode" : "Enter Focus Mode"}
          >
            {isFocusMode ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
          </Button>
        </div>
      </div>
      <DataStateBanner source={dataState?.source} error={dataState?.error} />
      <div className="flex flex-1 overflow-hidden">
        <div className={cn(
          "shrink-0 transition-all duration-300 ease-in-out md:flex",
          isFocusMode ? "hidden w-0" : "w-64"
        )}>
          {!isFocusMode && (
            <Suspense fallback={<MatterSidebarFallback />}>
              <MatterSidebar matter={matter} counts={counts} />
            </Suspense>
          )}
        </div>
        <main id="app-main" className="flex min-w-0 flex-1 flex-col overflow-hidden" tabIndex={-1}>
          {!isFocusMode && (
            <Suspense fallback={<div className="border-b border-border bg-card px-3 py-2 md:hidden" aria-hidden="true" />}>
              <MatterSidebarSheet matter={matter} counts={counts} />
            </Suspense>
          )}
          <MatterUploadControls matterId={matter.matter_id} />
          {children}
        </main>
        {rightPanel && !isFocusMode && (
          <aside className="hidden w-80 shrink-0 flex-col overflow-hidden border-l border-border bg-card xl:flex">
            {rightPanel}
          </aside>
        )}
      </div>
    </div>
  )
}

function MatterSidebarFallback() {
  return (
    <aside className="hidden w-60 shrink-0 border-r border-sidebar-border bg-sidebar p-3 md:block" aria-hidden="true">
      <div className="h-4 w-24 rounded bg-sidebar-accent/70" />
      <div className="mt-4 h-8 w-40 rounded bg-sidebar-accent/70" />
      <div className="mt-6 space-y-2">
        {Array.from({ length: 10 }).map((_, index) => (
          <div key={index} className="h-6 rounded bg-sidebar-accent/60" />
        ))}
      </div>
    </aside>
  )
}
