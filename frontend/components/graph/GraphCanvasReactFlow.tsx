"use client"

import { GraphCanvasSigma } from "./GraphCanvasSigma"
import type { GraphForces } from "./GraphForceControls"
import type { GraphEdge, GraphLayoutName, GraphNode } from "./types"

export function GraphCanvasReactFlow({
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
  return (
    <GraphCanvasSigma
      nodes={nodes}
      edges={edges}
      selectedId={selectedId}
      layout={layout}
      forces={forces}
      onSelect={onSelect}
      onRecenter={onRecenter}
    />
  )
}
