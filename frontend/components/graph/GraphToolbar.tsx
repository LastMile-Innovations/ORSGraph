"use client"

import { AlertTriangle, PanelRightOpen, RefreshCw, SlidersHorizontal } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { GraphModeSelector } from "./GraphModeSelector"
import { GraphSearchBox } from "./GraphSearchBox"
import type { GraphMode, GraphViewScope } from "./types"

export function GraphToolbar({
  query,
  mode,
  nodeCount,
  edgeCount,
  loading,
  truncated,
  viewScope,
  onModeChange,
  onOpen,
  onOpenAdvanced,
  onOpenInspector,
  onRefresh,
}: {
  query: string
  mode: GraphMode
  nodeCount: number
  edgeCount: number
  loading: boolean
  truncated: boolean
  viewScope: GraphViewScope
  onModeChange: (mode: GraphMode) => void
  onOpen: (value: string) => void
  onOpenAdvanced: () => void
  onOpenInspector: () => void
  onRefresh: () => void
}) {
  return (
    <header className="border-b border-border bg-card/95 px-4 py-3 backdrop-blur">
      <div className="flex flex-col gap-3 2xl:flex-row 2xl:items-center 2xl:justify-between">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <h1 className="font-serif text-2xl tracking-tight text-foreground">ORSGraph Atlas</h1>
            {truncated && <AlertTriangle className="h-4 w-4 text-warning" />}
          </div>
          <div className="mt-2 flex flex-wrap gap-1.5">
            <MetricChip label="Nodes" value={nodeCount} />
            <MetricChip label="Edges" value={edgeCount} />
            <Badge variant={viewScope === "full" ? "default" : "outline"} className="font-mono text-[10px] uppercase">
              {viewScope === "full" ? "Full graph" : "Neighborhood"}
            </Badge>
            <Badge variant="secondary" className="font-mono text-[10px] uppercase">
              {loading ? "Loading" : modeLabel(mode)}
            </Badge>
            {truncated && (
              <Badge variant="outline" className="border-warning/50 text-warning font-mono text-[10px] uppercase">
                Truncated
              </Badge>
            )}
          </div>
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-2 xl:flex-row xl:items-center xl:justify-end">
          <div className="xl:w-96">
            <GraphSearchBox value={query} onSubmit={onOpen} />
          </div>
          <GraphModeSelector value={mode} onChange={onModeChange} />
          <div className="flex gap-1">
            <Button type="button" variant="outline" size="sm" onClick={onOpenAdvanced} aria-label="Open advanced graph controls">
              <SlidersHorizontal className="h-4 w-4" />
              <span>Advanced</span>
            </Button>
            <Button type="button" variant="outline" size="icon-sm" onClick={onOpenInspector} className="xl:hidden" aria-label="Open graph inspector">
              <PanelRightOpen className="h-4 w-4" />
            </Button>
          </div>
          <Button type="button" variant="outline" size="icon-sm" onClick={onRefresh} aria-label="Refresh graph">
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        </div>
      </div>
    </header>
  )
}

function MetricChip({ label, value }: { label: string; value: number }) {
  return (
    <Badge variant="outline" className="gap-1 font-mono text-[10px] uppercase">
      <span className="text-muted-foreground">{label}</span>
      <span className="text-foreground tabular-nums">{value.toLocaleString()}</span>
    </Badge>
  )
}

function modeLabel(mode: GraphMode) {
  return mode.replaceAll("_", " ")
}
