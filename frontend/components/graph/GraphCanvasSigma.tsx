"use client"

import { useEffect, useMemo, useRef, useState } from "react"
import { NODE_COLORS } from "./constants"
import type { GraphForces } from "./GraphForceControls"
import type { GraphEdge, GraphLayoutName, GraphNode } from "./types"

const WIDTH = 1400
const HEIGHT = 850

type Point = { x: number; y: number }

export function GraphCanvasSigma({
  nodes,
  edges,
  selectedId,
  layout,
  forces,
  onSelect,
  onRecenter,
}: {
  nodes: GraphNode[]
  edges: GraphEdge[]
  selectedId?: string
  layout: GraphLayoutName
  forces: GraphForces
  onSelect: (id: string) => void
  onRecenter: (id: string) => void
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const [hoveredId, setHoveredId] = useState<string | null>(null)
  const [rendererReady, setRendererReady] = useState(false)
  const positions = useMemo(() => computeLayout(nodes, edges, selectedId, layout, forces), [nodes, edges, selectedId, layout, forces])
  const selectedNeighborIds = useMemo(() => {
    if (!selectedId) return new Set<string>()
    return new Set(edges.flatMap((edge) => edge.source === selectedId ? [edge.target] : edge.target === selectedId ? [edge.source] : []))
  }, [edges, selectedId])

  useEffect(() => {
    let cancelled = false
    let renderer: { kill: () => void } | undefined
    const container = containerRef.current
    if (!container || typeof window === "undefined") return undefined

    setRendererReady(false)

    async function mountSigma() {
      try {
        const [{ default: Graph }, { default: Sigma }] = await Promise.all([
          import("graphology"),
          import("sigma"),
        ])
        if (cancelled || !container) return

        const graph = new Graph({ multi: true, type: "directed" })
        for (const node of nodes) {
          const point = positions[node.id] ?? { x: WIDTH / 2, y: HEIGHT / 2 }
          graph.addNode(node.id, {
            label: shortLabel(node.label),
            x: (point.x - WIDTH / 2) / 100,
            y: (point.y - HEIGHT / 2) / 100,
            size: node.id === selectedId ? 16 : 8 + (node.similarityScore ?? 0) * 5,
            color: NODE_COLORS[node.type] ?? "#94a3b8",
            nodeType: node.type,
            qc: Boolean(node.qcWarnings?.length),
          })
        }
        for (const edge of edges) {
          if (!graph.hasNode(edge.source) || !graph.hasNode(edge.target)) continue
          graph.addDirectedEdgeWithKey(edge.id, edge.source, edge.target, {
            label: edge.label ?? edge.type,
            color: edge.style?.color ?? edgeColor(edge),
            size: edge.style?.width ?? 1,
            edgeType: edge.type,
            dashed: edge.style?.dashed || edge.kind === "semantic_similarity",
          })
        }

        renderer = new Sigma(graph, container, {
          allowInvalidContainer: true,
          renderEdgeLabels: false,
          labelDensity: Math.max(0.08, forces.labelDensity / 100),
          defaultEdgeType: "arrow",
          nodeReducer: (node: string, data: Record<string, unknown>) => {
            const active = node === selectedId || selectedNeighborIds.has(node)
            return {
              ...data,
              label: active || forces.labelDensity > 40 ? data.label : "",
              highlighted: active,
              size: active ? Number(data.size ?? 8) * 1.35 : data.size,
              borderColor: data.qc ? "#f59e0b" : undefined,
            }
          },
          edgeReducer: (_edge: string, data: Record<string, unknown>) => ({
            ...data,
            hidden: false,
          }),
        })

        renderer.on("enterNode", ({ node }: { node: string }) => setHoveredId(node))
        renderer.on("leaveNode", () => setHoveredId(null))
        renderer.on("clickNode", ({ node }: { node: string }) => onSelect(node))
        renderer.on("doubleClickNode", ({ node }: { node: string }) => onRecenter(node))

        if (!cancelled) setRendererReady(true)
      } catch {
        if (!cancelled) setRendererReady(false)
      }
    }

    mountSigma()
    return () => {
      cancelled = true
      renderer?.kill()
    }
  }, [nodes, edges, positions, selectedId, selectedNeighborIds, forces.labelDensity, onSelect, onRecenter])

  return (
    <div className="relative h-full min-h-[520px] overflow-hidden bg-background">
      <div
        ref={containerRef}
        className={`absolute inset-0 ${rendererReady ? "opacity-100" : "pointer-events-none opacity-0"}`}
        style={{
          backgroundImage:
            "linear-gradient(to right, color-mix(in oklch, var(--border) 35%, transparent) 1px, transparent 1px), linear-gradient(to bottom, color-mix(in oklch, var(--border) 35%, transparent) 1px, transparent 1px)",
          backgroundSize: "42px 42px",
        }}
      />
      {!rendererReady && (
      <svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} className="h-full w-full" role="img" aria-label="ORSGraph Atlas graph visualization">
        <defs>
          <pattern id="atlas-grid" width="42" height="42" patternUnits="userSpaceOnUse">
            <path d="M 42 0 L 0 0 0 42" fill="none" stroke="currentColor" strokeWidth="0.5" className="text-border" opacity="0.35" />
          </pattern>
          <marker id="atlas-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
            <path d="M 0 0 L 10 5 L 0 10 z" fill="#94a3b8" />
          </marker>
        </defs>
        <rect width={WIDTH} height={HEIGHT} fill="url(#atlas-grid)" />

        <g>
          {edges.map((edge) => {
            const source = positions[edge.source]
            const target = positions[edge.target]
            if (!source || !target) return null
            const active = edge.source === selectedId || edge.target === selectedId || edge.source === hoveredId || edge.target === hoveredId
            const color = edge.style?.color ?? edgeColor(edge)
            return (
              <g key={edge.id}>
                <line
                  x1={source.x}
                  y1={source.y}
                  x2={target.x}
                  y2={target.y}
                  stroke={color}
                  strokeWidth={active ? (edge.style?.width ?? 1.2) + 1.2 : edge.style?.width ?? 1}
                  strokeOpacity={active ? 0.9 : 0.35}
                  strokeDasharray={edge.style?.dashed || edge.kind === "semantic_similarity" ? "7 7" : undefined}
                  markerEnd="url(#atlas-arrow)"
                />
                {active && (
                  <text x={(source.x + target.x) / 2} y={(source.y + target.y) / 2 - 5} textAnchor="middle" className="fill-muted-foreground font-mono" fontSize="10">
                    {edge.label ?? edge.type}
                  </text>
                )}
              </g>
            )
          })}
        </g>

        <g>
          {nodes.map((node, index) => {
            const point = positions[node.id]
            if (!point) return null
            const selected = node.id === selectedId
            const neighbor = selectedNeighborIds.has(node.id)
            const hovered = hoveredId === node.id
            const radius = selected ? 15 : Math.max(7, Math.min(14, 7 + (node.metrics?.degree ?? 0) * 0.35 + (node.similarityScore ?? 0) * 5))
            const showLabel = selected || hovered || neighbor || index < Math.max(10, forces.labelDensity)
            return (
              <g
                key={node.id}
                transform={`translate(${point.x},${point.y})`}
                className="cursor-pointer"
                onMouseEnter={() => setHoveredId(node.id)}
                onMouseLeave={() => setHoveredId(null)}
                onClick={() => onSelect(node.id)}
                onDoubleClick={() => onRecenter(node.id)}
              >
                <circle
                  r={radius + (selected ? 7 : 3)}
                  fill={selected ? "#22d3ee" : neighbor ? "#60a5fa" : "#000000"}
                  opacity={selected ? 0.18 : neighbor ? 0.1 : 0}
                />
                <circle
                  r={radius}
                  fill={NODE_COLORS[node.type] ?? "#94a3b8"}
                  stroke={node.qcWarnings?.length ? "#f59e0b" : selected ? "#22d3ee" : "#0f172a"}
                  strokeWidth={selected ? 3 : node.qcWarnings?.length ? 2.5 : 1.5}
                  strokeDasharray={node.sourceBacked === false ? "3 3" : undefined}
                />
                {showLabel && (
                  <text y={-(radius + 8)} textAnchor="middle" className="fill-foreground font-mono" fontSize={selected ? "13" : "10"}>
                    {shortLabel(node.label)}
                  </text>
                )}
                {(selected || hovered) && (
                  <text y={radius + 16} textAnchor="middle" className="fill-muted-foreground font-mono" fontSize="9">
                    {node.type}
                  </text>
                )}
              </g>
            )
          })}
        </g>
      </svg>
      )}
      <div className="absolute left-3 top-3 rounded border border-border bg-card/90 px-2 py-1 font-mono text-[10px] uppercase tracking-wide text-muted-foreground backdrop-blur">
        {rendererReady ? "sigma webgl" : "svg fallback"} / {layout.replace("_", " ")}
      </div>
    </div>
  )
}

function computeLayout(nodes: GraphNode[], edges: GraphEdge[], selectedId: string | undefined, layout: GraphLayoutName, forces: GraphForces) {
  if (layout === "timeline") return timelineLayout(nodes)
  if (layout === "hierarchical") return hierarchicalLayout(nodes)
  if (layout === "embedding_projection") return projectionLayout(nodes)
  return radialForceLayout(nodes, edges, selectedId, layout, forces)
}

function radialForceLayout(nodes: GraphNode[], edges: GraphEdge[], selectedId: string | undefined, layout: GraphLayoutName, forces: GraphForces) {
  const centerId = selectedId && nodes.some((node) => node.id === selectedId) ? selectedId : nodes[0]?.id
  const center = { x: WIDTH / 2, y: HEIGHT / 2 }
  const positions: Record<string, Point> = {}
  if (!centerId) return positions
  positions[centerId] = center

  const adjacency = new Map<string, Set<string>>()
  for (const edge of edges) {
    if (!adjacency.has(edge.source)) adjacency.set(edge.source, new Set())
    if (!adjacency.has(edge.target)) adjacency.set(edge.target, new Set())
    adjacency.get(edge.source)?.add(edge.target)
    adjacency.get(edge.target)?.add(edge.source)
  }

  const direct = [...(adjacency.get(centerId) ?? new Set())]
  const remaining = nodes.map((node) => node.id).filter((id) => id !== centerId && !direct.includes(id))
  placeRing(positions, direct, center, layout === "radial" ? 190 : 165 + forces.repulsion, 0)
  placeRing(positions, remaining, center, layout === "radial" ? 330 : 290 + forces.cluster, Math.PI / Math.max(remaining.length, 1))
  return positions
}

function placeRing(positions: Record<string, Point>, ids: string[], center: Point, radius: number, offset: number) {
  ids.forEach((id, index) => {
    const angle = (Math.PI * 2 * index) / Math.max(ids.length, 1) + offset
    positions[id] = {
      x: center.x + Math.cos(angle) * radius,
      y: center.y + Math.sin(angle) * radius,
    }
  })
}

function hierarchicalLayout(nodes: GraphNode[]) {
  const groups = groupBy(nodes, (node) => node.type)
  const positions: Record<string, Point> = {}
  const types = Object.keys(groups)
  types.forEach((type, col) => {
    const list = groups[type] ?? []
    list.forEach((node, row) => {
      positions[node.id] = {
        x: 160 + col * Math.max(150, (WIDTH - 320) / Math.max(types.length - 1, 1)),
        y: 100 + row * Math.max(44, (HEIGHT - 200) / Math.max(list.length - 1, 1)),
      }
    })
  })
  return positions
}

function timelineLayout(nodes: GraphNode[]) {
  const positions: Record<string, Point> = {}
  nodes.forEach((node, index) => {
    positions[node.id] = {
      x: 90 + index * Math.max(70, (WIDTH - 180) / Math.max(nodes.length - 1, 1)),
      y: HEIGHT / 2 + (index % 2 === 0 ? -70 : 70),
    }
  })
  return positions
}

function projectionLayout(nodes: GraphNode[]) {
  const positions: Record<string, Point> = {}
  nodes.forEach((node, index) => {
    const hash = stableHash(node.id)
    positions[node.id] = {
      x: 120 + (hash % 1000) / 1000 * (WIDTH - 240),
      y: 90 + ((hash / 1000) % 1000) / 1000 * (HEIGHT - 180),
    }
    if (index === 0) positions[node.id] = { x: WIDTH / 2, y: HEIGHT / 2 }
  })
  return positions
}

function groupBy<T>(items: T[], getKey: (item: T) => string) {
  return items.reduce<Record<string, T[]>>((acc, item) => {
    const key = getKey(item)
    acc[key] = [...(acc[key] ?? []), item]
    return acc
  }, {})
}

function edgeColor(edge: GraphEdge) {
  if (edge.kind === "semantic_similarity") return "#22d3ee"
  if (edge.kind === "history") return "#f59e0b"
  if (edge.type.includes("CITES") || edge.type.includes("RESOLVES")) return "#60a5fa"
  if (edge.type.includes("DEFIN")) return "#a78bfa"
  if (edge.type.includes("DEADLINE")) return "#f59e0b"
  return "#94a3b8"
}

function shortLabel(value: string) {
  return value.length > 28 ? `${value.slice(0, 25)}...` : value
}

function stableHash(value: string) {
  let hash = 0
  for (let i = 0; i < value.length; i += 1) hash = (hash * 31 + value.charCodeAt(i)) >>> 0
  return hash
}
