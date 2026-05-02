"use client"

import { GraphCanvasSigma } from "./GraphCanvasSigma"
import type { GraphForces } from "./GraphForceControls"
import type { GraphEdge, GraphLayoutName, GraphNode, GraphViewScope } from "./types"

export function GraphCanvasReactFlow({
  nodes,
  edges,
  selectedId,
  layout,
  forces,
  viewScope = "neighborhood",
  onSelect,
  onRecenter,
}: {
  nodes: GraphNode[]
  edges: GraphEdge[]
  selectedId?: string
  layout: GraphLayoutName
  forces: GraphForces
  viewScope?: GraphViewScope
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
      viewScope={viewScope}
      onSelect={onSelect}
      onRecenter={onRecenter}
    />
  )
}
