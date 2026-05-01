"use client"

import { GraphCanvas } from "./GraphCanvas"
import type { GraphEdge, GraphNode } from "./types"

export function MiniGraph({
  nodes,
  edges,
  selectedId,
}: {
  nodes: GraphNode[]
  edges: GraphEdge[]
  selectedId?: string
}) {
  return (
    <div className="h-64 overflow-hidden rounded border border-border bg-background">
      <GraphCanvas
        nodes={nodes}
        edges={edges}
        selectedId={selectedId}
        layout="radial"
        forces={{
          repulsion: 25,
          cluster: 35,
          labelDensity: 12,
          depth: 1,
        }}
        onSelect={() => undefined}
        onRecenter={() => undefined}
      />
    </div>
  )
}
