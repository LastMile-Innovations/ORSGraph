"use client"

import { useEffect, useState } from "react"
import type { StatutePageResponse } from "@/lib/types"
import Link from "next/link"
import { GraphMiniCanvas } from "@/components/orsg/graph-mini-canvas"
import { getGraphNeighborhood } from "@/lib/api"
import type { GraphEdge, GraphNode } from "@/lib/types"

export function GraphTab({ data }: { data: StatutePageResponse }) {
  const [nodes, setNodes] = useState<GraphNode[]>([])
  const [edges, setEdges] = useState<GraphEdge[]>([])
  const [centerId, setCenterId] = useState(data.identity.canonical_id)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    setError(null)
    setLoading(true)
    getGraphNeighborhood({
      citation: data.identity.citation,
      depth: 1,
      limit: 80,
      mode: "legal",
    })
      .then((graph) => {
        if (cancelled) return
        setNodes((graph.nodes ?? []).map(toMiniNode))
        setEdges((graph.edges ?? []).map(toMiniEdge))
        setCenterId(graph.center?.id ?? data.identity.canonical_id)
        setLoading(false)
      })
      .catch((reason) => {
        if (cancelled) return
        setNodes([])
        setEdges([])
        setError(reason instanceof Error ? reason.message : "Graph data unavailable.")
        setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [data.identity.canonical_id, data.identity.citation])

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-border bg-muted/30 px-4 py-2">
        <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          1-hop subgraph · centered on {data.identity.citation}
        </div>
        <Link
          href={`/graph?focus=${encodeURIComponent(data.identity.canonical_id)}`}
          className="font-mono text-xs text-primary hover:underline"
        >
          open in graph explorer →
        </Link>
      </div>
      <div className="border-b border-border bg-background px-4 py-2 text-xs text-muted-foreground">
        Arrows point from the citing authority toward the referenced authority. Edge colors show the relationship type.
      </div>
      {!loading && !error && nodes.length > 0 && edges.length === 0 && (
        <div className="border-b border-warning/30 bg-warning/10 px-4 py-2 text-xs text-warning">
          No graph edges are available for this statute yet. Inline citations may still appear in the text before citation edges are resolved.
        </div>
      )}
      <div className="relative flex-1 bg-background">
        {loading ? (
          <GraphLoadingState />
        ) : nodes.length > 0 ? (
          <GraphMiniCanvas nodes={nodes} edges={edges} centerId={centerId} />
        ) : (
          <div className="flex h-full items-center justify-center p-6 text-center text-sm text-muted-foreground">
            {error ? `Graph data unavailable: ${error}` : "No live graph data returned for this statute."}
          </div>
        )}
      </div>
    </div>
  )
}

function GraphLoadingState() {
  return (
    <div className="flex h-full min-h-[360px] items-center justify-center p-6" aria-live="polite">
      <div className="w-full max-w-xl">
        <div className="mb-4 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          Loading graph neighborhood
        </div>
        <div className="h-72 rounded border border-border bg-card p-6">
          <div className="mx-auto mt-20 h-4 w-32 rounded bg-muted" />
          <div className="mt-6 flex items-center justify-center gap-10">
            <div className="h-3 w-20 rounded bg-muted" />
            <div className="h-8 w-8 rounded-full bg-primary/30" />
            <div className="h-3 w-20 rounded bg-muted" />
          </div>
        </div>
      </div>
    </div>
  )
}

function toMiniNode(node: { id: string; label: string; type: string; status?: string | null }): GraphNode {
  return {
    id: node.id,
    label: node.label,
    type: miniNodeType(node.type),
    status: node.status === "repealed" || node.status === "renumbered" || node.status === "amended" ? node.status : "active",
  }
}

function toMiniEdge(edge: { id: string; source: string; target: string; type: string }): GraphEdge {
  return {
    id: edge.id,
    source: edge.source,
    target: edge.target,
    type: miniEdgeType(edge.type),
  }
}

function miniNodeType(type: string): GraphNode["type"] {
  if (type === "LegalTextIdentity" || type === "LegalTextVersion") return "Statute"
  if (type === "RetrievalChunk") return "Provision"
  if (type === "Commentary" || type === "ConstitutionAnnotated") return "Source"
  if (type === "DefinedTerm") return "Definition"
  if (["Provision", "CitationMention", "Chapter", "Definition", "Exception", "Deadline", "Penalty"].includes(type)) {
    return type as GraphNode["type"]
  }
  return "Provision"
}

function miniEdgeType(type: string): GraphEdge["type"] {
  if (["CITES", "MENTIONS_CITATION", "RESOLVES_TO", "HAS_VERSION", "CONTAINS", "DERIVED_FROM", "DEFINES", "EXCEPTION_TO", "HAS_DEADLINE", "ANNOTATES", "INTERPRETS", "HAS_COMMENTARY"].includes(type)) {
    return type as GraphEdge["type"]
  }
  return "CONTAINS"
}
