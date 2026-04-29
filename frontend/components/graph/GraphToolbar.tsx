"use client"

import { AlertTriangle, RefreshCw } from "lucide-react"
import { Button } from "@/components/ui/button"
import { GraphModeSelector } from "./GraphModeSelector"
import { GraphSearchBox } from "./GraphSearchBox"
import type { GraphMode } from "./types"

export function GraphToolbar({
  query,
  mode,
  nodeCount,
  edgeCount,
  loading,
  truncated,
  onModeChange,
  onOpen,
  onRefresh,
}: {
  query: string
  mode: GraphMode
  nodeCount: number
  edgeCount: number
  loading: boolean
  truncated: boolean
  onModeChange: (mode: GraphMode) => void
  onOpen: (value: string) => void
  onRefresh: () => void
}) {
  return (
    <header className="border-b border-border bg-card px-4 py-3">
      <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <h1 className="font-serif text-2xl tracking-tight text-foreground">ORSGraph Atlas</h1>
            {truncated && <AlertTriangle className="h-4 w-4 text-warning" />}
          </div>
          <p className="font-mono text-xs text-muted-foreground">
            {nodeCount} nodes / {edgeCount} edges / {loading ? "loading" : "bounded neighborhood"}
          </p>
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-2 lg:flex-row lg:items-center lg:justify-end">
          <div className="lg:w-80">
            <GraphSearchBox value={query} onSubmit={onOpen} />
          </div>
          <GraphModeSelector value={mode} onChange={onModeChange} />
          <Button type="button" variant="outline" size="icon-sm" onClick={onRefresh} aria-label="Refresh graph">
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        </div>
      </div>
    </header>
  )
}
