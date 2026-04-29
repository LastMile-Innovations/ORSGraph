"use client"

import { NODE_COLORS } from "./constants"

const EDGE_LEGEND = [
  { label: "CITES", color: "#60a5fa", dashed: false },
  { label: "EXPRESSES", color: "#34d399", dashed: false },
  { label: "DEFINES", color: "#a78bfa", dashed: false },
  { label: "HISTORY", color: "#f59e0b", dashed: false },
  { label: "SIMILAR_TO", color: "#22d3ee", dashed: true },
]

export function GraphLegend({ compact = false }: { compact?: boolean }) {
  const nodes = ["LegalTextIdentity", "Provision", "Obligation", "Deadline", "Penalty", "Definition", "SourceNote"]
  return (
    <div className="rounded border border-border bg-card/95 p-3 shadow-sm backdrop-blur">
      <div className="mb-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground">Legend</div>
      <div className={compact ? "grid grid-cols-2 gap-2" : "space-y-3"}>
        <div className="space-y-1">
          {nodes.map((node) => (
            <div key={node} className="flex items-center gap-2 text-xs">
              <span className="h-2.5 w-2.5 rounded-full border border-background" style={{ backgroundColor: NODE_COLORS[node] }} />
              <span className="truncate font-mono text-muted-foreground">{node}</span>
            </div>
          ))}
        </div>
        <div className="space-y-1">
          {EDGE_LEGEND.map((edge) => (
            <div key={edge.label} className="flex items-center gap-2 text-xs">
              <span
                className="h-0 w-5 border-t"
                style={{ borderColor: edge.color, borderTopStyle: edge.dashed ? "dashed" : "solid" }}
              />
              <span className="font-mono text-muted-foreground">{edge.label}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
