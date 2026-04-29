"use client"

import { useMemo, useState } from "react"
import type { GraphNode, GraphEdge } from "@/lib/types"
import { cn } from "@/lib/utils"

const NODE_TYPE_COLOR: Record<GraphNode["type"], string> = {
  Statute: "fill-primary",
  Provision: "fill-chart-1",
  CitationMention: "fill-accent",
  Chapter: "fill-chart-2",
  Definition: "fill-chart-1",
  Exception: "fill-warning",
  Deadline: "fill-chart-3",
  Penalty: "fill-destructive",
}

const EDGE_TYPE_COLOR: Record<GraphEdge["type"], string> = {
  CITES: "stroke-accent",
  MENTIONS_CITATION: "stroke-accent",
  RESOLVES_TO: "stroke-primary",
  HAS_VERSION: "stroke-muted-foreground",
  CONTAINS: "stroke-muted-foreground",
  DERIVED_FROM: "stroke-muted-foreground",
  DEFINES: "stroke-chart-1",
  EXCEPTION_TO: "stroke-warning",
  HAS_DEADLINE: "stroke-chart-3",
}

interface Props {
  nodes: GraphNode[]
  edges: GraphEdge[]
  centerId?: string
  /** Alias for centerId — also drives the highlighted/selected node. */
  focusId?: string
  height?: number
  onSelect?: (id: string) => void
}

// Simple radial layout: center node fixed, others arranged on rings.
function layoutRadial(nodes: GraphNode[], edges: GraphEdge[], centerId?: string) {
  const W = 800
  const H = 500
  const cx = W / 2
  const cy = H / 2

  const center = centerId
    ? nodes.find((n) => n.id === centerId) ?? nodes[0]
    : nodes[0]

  const others = nodes.filter((n) => n.id !== center.id)

  // Group by node type for nicer rings.
  const grouped: Record<string, GraphNode[]> = {}
  for (const n of others) {
    if (!grouped[n.type]) grouped[n.type] = []
    grouped[n.type].push(n)
  }

  const positions: Record<string, { x: number; y: number }> = {
    [center.id]: { x: cx, y: cy },
  }

  const groupKeys = Object.keys(grouped)
  let ringIdx = 0
  for (const key of groupKeys) {
    const list = grouped[key]
    const radius = 130 + ringIdx * 70
    const angleStep = (Math.PI * 2) / Math.max(list.length, 1)
    const angleOffset = ringIdx * 0.3
    list.forEach((n, i) => {
      const a = i * angleStep + angleOffset
      positions[n.id] = {
        x: cx + Math.cos(a) * radius,
        y: cy + Math.sin(a) * radius,
      }
    })
    ringIdx++
  }

  return { positions, viewBox: `0 0 ${W} ${H}`, W, H, centerId: center.id }
}

export function GraphMiniCanvas({ nodes, edges, centerId, focusId, height = 500, onSelect }: Props) {
  const effectiveCenter = focusId ?? centerId
  const layout = useMemo(
    () => layoutRadial(nodes, edges, effectiveCenter),
    [nodes, edges, effectiveCenter],
  )
  const [hover, setHover] = useState<string | null>(null)

  return (
    <div className="relative h-full w-full" style={{ minHeight: height }}>
      <svg
        viewBox={layout.viewBox}
        preserveAspectRatio="xMidYMid meet"
        className="h-full w-full"
      >
        {/* grid background */}
        <defs>
          <pattern id="grid-mini" width="40" height="40" patternUnits="userSpaceOnUse">
            <path
              d="M 40 0 L 0 0 0 40"
              className="stroke-border"
              fill="none"
              strokeWidth="0.5"
              strokeOpacity="0.4"
            />
          </pattern>
          <marker
            id="arrow"
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerWidth="6"
            markerHeight="6"
            orient="auto-start-reverse"
          >
            <path d="M 0 0 L 10 5 L 0 10 z" className="fill-accent" />
          </marker>
        </defs>
        <rect width="100%" height="100%" fill="url(#grid-mini)" />

        {/* edges */}
        <g>
          {edges.map((e) => {
            const a = layout.positions[e.source]
            const b = layout.positions[e.target]
            if (!a || !b) return null
            const isHover = hover === e.source || hover === e.target
            return (
              <g key={e.id}>
                <line
                  x1={a.x}
                  y1={a.y}
                  x2={b.x}
                  y2={b.y}
                  className={cn(
                    EDGE_TYPE_COLOR[e.type],
                    isHover ? "opacity-100" : "opacity-50",
                  )}
                  strokeWidth={isHover ? 1.5 : 0.8}
                  markerEnd="url(#arrow)"
                />
                {isHover && (
                  <text
                    x={(a.x + b.x) / 2}
                    y={(a.y + b.y) / 2}
                    className="fill-muted-foreground"
                    fontSize="9"
                    fontFamily="monospace"
                    textAnchor="middle"
                  >
                    {e.type}
                  </text>
                )}
              </g>
            )
          })}
        </g>

        {/* nodes */}
        <g>
          {nodes.map((n) => {
            const p = layout.positions[n.id]
            if (!p) return null
            const isCenter = n.id === layout.centerId
            const isHover = hover === n.id
            return (
              <g
                key={n.id}
                transform={`translate(${p.x},${p.y})`}
                className="cursor-pointer"
                onMouseEnter={() => setHover(n.id)}
                onMouseLeave={() => setHover(null)}
                onClick={() => onSelect?.(n.id)}
              >
                <circle
                  r={isCenter ? 10 : 6}
                  className={cn(
                    NODE_TYPE_COLOR[n.type],
                    "stroke-background",
                    (isCenter || isHover) && "drop-shadow-md",
                  )}
                  strokeWidth={isCenter ? 3 : 2}
                />
                <text
                  y={isCenter ? -16 : -10}
                  textAnchor="middle"
                  className={cn(
                    "fill-foreground font-mono",
                    isCenter ? "font-semibold" : "",
                  )}
                  fontSize={isCenter ? "11" : "9"}
                >
                  {n.label}
                </text>
                {(isCenter || isHover) && (
                  <text
                    y={isCenter ? 22 : 16}
                    textAnchor="middle"
                    className="fill-muted-foreground font-mono"
                    fontSize="8"
                  >
                    {n.type}
                  </text>
                )}
              </g>
            )
          })}
        </g>
      </svg>

      {/* Legend */}
      <div className="absolute bottom-3 left-3 rounded border border-border bg-card/95 p-2 text-[10px] backdrop-blur">
        <div className="mb-1 font-mono uppercase tracking-wider text-muted-foreground">edges</div>
        <div className="grid grid-cols-2 gap-x-3 gap-y-0.5">
          <LegendRow color="bg-accent" label="CITES" />
          <LegendRow color="bg-chart-1" label="DEFINES" />
          <LegendRow color="bg-warning" label="EXCEPTION_TO" />
          <LegendRow color="bg-chart-3" label="HAS_DEADLINE" />
        </div>
      </div>
    </div>
  )
}

function LegendRow({ color, label }: { color: string; label: string }) {
  return (
    <div className="flex items-center gap-1.5 font-mono">
      <span className={cn("h-0.5 w-3", color)} />
      <span className="text-muted-foreground">{label}</span>
    </div>
  )
}
