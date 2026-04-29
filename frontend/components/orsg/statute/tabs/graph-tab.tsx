import type { StatutePageResponse } from "@/lib/types"
import Link from "next/link"
import { GraphMiniCanvas } from "@/components/orsg/graph-mini-canvas"
import { graphNodes, graphEdges } from "@/lib/mock-data"

export function GraphTab({ data }: { data: StatutePageResponse }) {
  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-border bg-muted/30 px-4 py-2">
        <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          1-hop subgraph · centered on {data.identity.citation}
        </div>
        <Link
          href="/graph"
          className="font-mono text-xs text-primary hover:underline"
        >
          open in graph explorer →
        </Link>
      </div>
      <div className="relative flex-1 bg-background">
        <GraphMiniCanvas nodes={graphNodes} edges={graphEdges} centerId={data.identity.canonical_id} />
      </div>
    </div>
  )
}
