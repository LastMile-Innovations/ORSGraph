"use client"

import { useMemo, useState } from "react"
import Link from "next/link"
import { GitBranch, Search } from "lucide-react"
import { GraphMiniCanvas } from "@/components/orsg/graph-mini-canvas"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { graphNodes, graphEdges, statuteIndex } from "@/lib/mock-data"
import { StatusBadge, QCBadge } from "@/components/orsg/badges"
import type { GraphEdge, GraphNode } from "@/lib/types"

const NODE_TYPES: GraphNode["type"][] = [
  "Statute",
  "Provision",
  "CitationMention",
  "Definition",
  "Exception",
  "Deadline",
  "Penalty",
]

const EDGE_TYPES: GraphEdge["type"][] = [
  "CITES",
  "MENTIONS_CITATION",
  "RESOLVES_TO",
  "HAS_VERSION",
  "CONTAINS",
  "DEFINES",
  "EXCEPTION_TO",
  "HAS_DEADLINE",
]

export function GraphExplorerClient() {
  const [enabledNodeTypes, setEnabledNodeTypes] = useState<Set<string>>(new Set(NODE_TYPES))
  const [enabledEdgeTypes, setEnabledEdgeTypes] = useState<Set<string>>(new Set(EDGE_TYPES))
  const [focusId, setFocusId] = useState<string>(graphNodes[0]?.id ?? "")
  const [query, setQuery] = useState("")
  const [hop, setHop] = useState(2)

  const filteredNodes = useMemo(
    () => graphNodes.filter((n) => enabledNodeTypes.has(n.type)),
    [enabledNodeTypes],
  )

  const filteredEdges = useMemo(
    () => graphEdges.filter((e) => enabledEdgeTypes.has(e.type)),
    [enabledEdgeTypes],
  )

  // BFS within `hop` distance from focus
  const visibleSet = useMemo(() => {
    if (!focusId) return new Set(filteredNodes.map((n) => n.id))
    const visited = new Set<string>([focusId])
    let frontier = new Set<string>([focusId])
    for (let i = 0; i < hop; i++) {
      const next = new Set<string>()
      for (const id of frontier) {
        for (const e of filteredEdges) {
          if (e.source === id && !visited.has(e.target)) next.add(e.target)
          if (e.target === id && !visited.has(e.source)) next.add(e.source)
        }
      }
      next.forEach((id) => visited.add(id))
      frontier = next
    }
    return visited
  }, [focusId, hop, filteredEdges, filteredNodes])

  const visibleNodes = useMemo(
    () => filteredNodes.filter((n) => visibleSet.has(n.id)),
    [filteredNodes, visibleSet],
  )
  const visibleEdges = useMemo(
    () => filteredEdges.filter((e) => visibleSet.has(e.source) && visibleSet.has(e.target)),
    [filteredEdges, visibleSet],
  )

  const focusNode = graphNodes.find((n) => n.id === focusId)

  const searchResults = useMemo(() => {
    if (!query.trim()) return [] as GraphNode[]
    const q = query.toLowerCase()
    return graphNodes.filter((n) => n.label.toLowerCase().includes(q) || n.id.toLowerCase().includes(q)).slice(0, 8)
  }, [query])

  return (
    <div className="flex h-full">
      {/* Filters rail */}
      <aside className="w-72 shrink-0 border-r border-border bg-card/40 overflow-y-auto">
        <div className="p-4 border-b border-border">
          <div className="flex items-center gap-2 mb-3">
            <GitBranch className="h-4 w-4 text-muted-foreground" />
            <h2 className="text-sm font-mono uppercase tracking-wider text-foreground">Citation graph</h2>
          </div>
          <p className="text-xs text-muted-foreground">
            Explore CITES, RESOLVES_TO, and CONTAINS edges across the corpus. Filter by node and edge type to focus the
            topology.
          </p>
        </div>

        <FilterGroup title="Node types" all={NODE_TYPES} enabled={enabledNodeTypes} setEnabled={setEnabledNodeTypes} />
        <FilterGroup title="Edge types" all={EDGE_TYPES} enabled={enabledEdgeTypes} setEnabled={setEnabledEdgeTypes} />

        <div className="p-4 border-b border-border">
          <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-2">Hop depth</div>
          <div className="flex items-center gap-1">
            {[1, 2, 3, 4].map((h) => (
              <Button
                key={h}
                size="sm"
                variant={hop === h ? "default" : "outline"}
                className="font-mono"
                onClick={() => setHop(h)}
              >
                {h}
              </Button>
            ))}
          </div>
        </div>

        <div className="p-4 border-b border-border space-y-3">
          <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground">Focus node</div>
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-muted-foreground" />
            <Input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="search nodes…"
              className="h-9 pl-8 font-mono text-xs bg-background"
            />
          </div>
          {searchResults.length > 0 && (
            <ul className="rounded-md border border-border bg-background overflow-hidden">
              {searchResults.map((n) => (
                <li key={n.id}>
                  <button
                    onClick={() => {
                      setFocusId(n.id)
                      setQuery("")
                    }}
                    className="w-full px-3 py-2 text-left text-xs hover:bg-accent flex items-center justify-between gap-2"
                  >
                    <span className="font-mono truncate">{n.label}</span>
                    <span className="text-muted-foreground text-[10px] uppercase">{n.type}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
          {focusNode && (
            <div className="rounded-md border border-border bg-background/60 p-3">
              <div className="text-[10px] font-mono uppercase tracking-wider text-muted-foreground">current focus</div>
              <div className="font-mono text-sm text-foreground mt-1 truncate">{focusNode.label}</div>
              <div className="text-xs text-muted-foreground mt-0.5 font-mono">{focusNode.type}</div>
            </div>
          )}
        </div>

        <div className="p-4">
          <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-2">Quick focus</div>
          <ul className="space-y-1">
            {statuteIndex.slice(0, 6).map((s) => {
              const nodeId = `n:${s.canonical_id}`
              return (
                <li key={s.canonical_id}>
                  <button
                    onClick={() => setFocusId(nodeId)}
                    className={`w-full text-left rounded px-2 py-1.5 text-xs font-mono transition-colors ${
                      focusId === nodeId ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
                    }`}
                  >
                    {s.citation}
                  </button>
                </li>
              )
            })}
          </ul>
        </div>
      </aside>

      {/* Canvas */}
      <div className="flex-1 min-w-0 flex flex-col overflow-hidden">
        <div className="border-b border-border bg-card px-6 py-4 flex items-center justify-between gap-4">
          <div>
            <h1 className="font-serif text-2xl tracking-tight text-foreground">Citation Graph Explorer</h1>
            <p className="text-xs text-muted-foreground font-mono mt-0.5">
              {visibleNodes.length} nodes · {visibleEdges.length} edges · {hop}-hop neighborhood
            </p>
          </div>
          {focusNode?.type === "Statute" && (
            <Button asChild variant="outline" size="sm" className="font-mono">
              <Link href={`/statutes/${focusNode.id.replace(/^n:/, "")}`}>open statute →</Link>
            </Button>
          )}
        </div>

        <div className="flex-1 p-6 overflow-hidden">
          <div className="h-full rounded-lg border border-border bg-background/40">
            <GraphMiniCanvas
              nodes={visibleNodes}
              edges={visibleEdges}
              focusId={focusId}
              onSelect={(id) => setFocusId(id)}
            />
          </div>
        </div>
      </div>

      {/* Right detail */}
      <aside className="hidden xl:flex w-80 border-l border-border bg-card/40 flex-col overflow-y-auto">
        <div className="p-4 border-b border-border">
          <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-2">Node detail</div>
          {focusNode ? (
            <div className="space-y-2">
              <div className="font-mono text-sm text-foreground break-all">{focusNode.label}</div>
              <div className="flex items-center gap-2">
                <span className="text-xs px-2 py-0.5 rounded border border-border bg-background font-mono">
                  {focusNode.type}
                </span>
                {focusNode.status && <StatusBadge status={focusNode.status} />}
                {focusNode.qc_status && <QCBadge status={focusNode.qc_status} />}
              </div>
              <div className="font-mono text-[10px] text-muted-foreground break-all">{focusNode.id}</div>
            </div>
          ) : (
            <p className="text-xs text-muted-foreground">Select a node to inspect.</p>
          )}
        </div>

        <div className="p-4 border-b border-border">
          <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-2">
            Edges from focus
          </div>
          <ul className="space-y-1">
            {focusId &&
              graphEdges
                .filter((e) => e.source === focusId || e.target === focusId)
                .slice(0, 12)
                .map((e) => {
                  const out = e.source === focusId
                  const otherId = out ? e.target : e.source
                  const other = graphNodes.find((n) => n.id === otherId)
                  return (
                    <li key={e.id}>
                      <button
                        onClick={() => setFocusId(otherId)}
                        className="w-full text-left rounded border border-border bg-background/60 hover:border-primary/40 p-2"
                      >
                        <div className="flex items-center justify-between gap-2 text-[10px] font-mono uppercase tracking-wider">
                          <span className="text-primary">{e.type}</span>
                          <span className="text-muted-foreground">{out ? "→" : "←"}</span>
                        </div>
                        <div className="font-mono text-xs text-foreground mt-0.5 truncate">
                          {other?.label ?? otherId}
                        </div>
                      </button>
                    </li>
                  )
                })}
          </ul>
        </div>
      </aside>
    </div>
  )
}

function FilterGroup({
  title,
  all,
  enabled,
  setEnabled,
}: {
  title: string
  all: string[]
  enabled: Set<string>
  setEnabled: (s: Set<string>) => void
}) {
  function toggle(t: string) {
    const next = new Set(enabled)
    if (next.has(t)) next.delete(t)
    else next.add(t)
    setEnabled(next)
  }
  return (
    <div className="p-4 border-b border-border">
      <div className="flex items-center justify-between mb-2">
        <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground">{title}</div>
        <button
          onClick={() => setEnabled(new Set(enabled.size === all.length ? [] : all))}
          className="text-[10px] font-mono uppercase text-muted-foreground hover:text-foreground"
        >
          {enabled.size === all.length ? "none" : "all"}
        </button>
      </div>
      <div className="flex flex-wrap gap-1">
        {all.map((t) => (
          <button
            key={t}
            onClick={() => toggle(t)}
            className={`text-[10px] font-mono uppercase tracking-wider px-2 py-1 rounded border transition-colors ${
              enabled.has(t)
                ? "border-primary/40 bg-primary/10 text-foreground"
                : "border-border bg-background text-muted-foreground hover:text-foreground"
            }`}
          >
            {t}
          </button>
        ))}
      </div>
    </div>
  )
}
