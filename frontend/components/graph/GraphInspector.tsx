"use client"

import Link from "next/link"
import { ExternalLink } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import type { GraphEdge, GraphNode } from "./types"

export function GraphInspector({
  node,
  edges,
  onSelect,
  onExpand,
}: {
  node?: GraphNode | null
  edges: GraphEdge[]
  onSelect: (id: string) => void
  onExpand: (id: string) => void
}) {
  const connected = node ? edges.filter((edge) => edge.source === node.id || edge.target === node.id) : []
  const grouped = connected.reduce<Record<string, number>>((acc, edge) => {
    acc[edge.type] = (acc[edge.type] ?? 0) + 1
    return acc
  }, {})

  return (
    <aside className="hidden h-full w-80 shrink-0 flex-col overflow-y-auto border-l border-border bg-card/50 scrollbar-thin xl:flex">
      <section className="border-b border-border p-4">
        <div className="mb-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground">Overview</div>
        {node ? (
          <div className="space-y-3">
            <div>
              <div className="break-words font-mono text-sm font-semibold text-foreground">{node.label}</div>
              <div className="mt-1 break-all font-mono text-[10px] text-muted-foreground">{node.id}</div>
            </div>
            <div className="flex flex-wrap gap-1">
              <Badge variant="outline" className="font-mono text-[10px] uppercase">{node.type}</Badge>
              {node.status && <Badge variant="secondary" className="font-mono text-[10px] uppercase">{node.status}</Badge>}
              {node.sourceBacked && <Badge className="font-mono text-[10px] uppercase">source</Badge>}
            </div>
            {node.textSnippet && <p className="legal-text text-sm text-muted-foreground">{node.textSnippet}</p>}
            <div className="grid grid-cols-2 gap-2 text-xs">
              <Info label="Citation" value={node.citation} />
              <Info label="Chapter" value={node.chapter} />
              <Info label="Confidence" value={node.confidence?.toFixed(2)} />
              <Info label="Similarity" value={node.similarityScore?.toFixed(2)} />
            </div>
            <div className="flex flex-wrap gap-2">
              <Button size="sm" variant="outline" className="font-mono text-xs" onClick={() => onExpand(node.id)}>
                Expand
              </Button>
              {node.href && (
                <Button asChild size="sm" variant="outline" className="font-mono text-xs">
                  <Link href={node.href}>
                    Open <ExternalLink className="h-3.5 w-3.5" />
                  </Link>
                </Button>
              )}
            </div>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">Select a node to inspect its source, relationships, QC, and actions.</p>
        )}
      </section>

      <section className="border-b border-border p-4">
        <div className="mb-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground">Edges</div>
        <div className="mb-3 flex flex-wrap gap-1">
          {Object.entries(grouped).map(([type, count]) => (
            <Badge key={type} variant="outline" className="font-mono text-[10px] uppercase">
              {type}: {count}
            </Badge>
          ))}
        </div>
        <div className="space-y-1">
          {connected.slice(0, 24).map((edge) => {
            const otherId = edge.source === node?.id ? edge.target : edge.source
            return (
              <button
                key={edge.id}
                type="button"
                onClick={() => onSelect(otherId)}
                className="w-full rounded border border-border bg-background/70 p-2 text-left hover:border-primary/50"
              >
                <div className="flex items-center justify-between gap-2 font-mono text-[10px] uppercase text-muted-foreground">
                  <span className="text-primary">{edge.type}</span>
                  <span>{edge.source === node?.id ? "out" : "in"}</span>
                </div>
                <div className="mt-0.5 truncate font-mono text-xs text-foreground">{otherId}</div>
              </button>
            )
          })}
        </div>
      </section>

      <section className="p-4">
        <div className="mb-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground">QC</div>
        {node?.qcWarnings?.length ? (
          <ul className="space-y-1 text-xs text-warning">
            {node.qcWarnings.map((warning, index) => <li key={`${warning}-${index}`}>{warning}</li>)}
          </ul>
        ) : (
          <p className="text-xs text-muted-foreground">No warnings returned for this node.</p>
        )}
      </section>
    </aside>
  )
}

function Info({ label, value }: { label: string; value?: string | null }) {
  return (
    <div className="rounded border border-border bg-background p-2">
      <div className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">{label}</div>
      <div className="mt-0.5 truncate font-mono text-xs text-foreground">{value || "None"}</div>
    </div>
  )
}
