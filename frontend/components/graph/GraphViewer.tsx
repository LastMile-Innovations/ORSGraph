"use client"

import { useEffect, useMemo, useState } from "react"
import { AlertTriangle } from "lucide-react"
import { getGraphNeighborhood } from "@/lib/api"
import { graphEdges as mockEdges, graphNodes as mockNodes } from "@/lib/mock-data"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { GraphCanvas } from "./GraphCanvas"
import { GraphFilterPanel, type GraphFilters } from "./GraphFilterPanel"
import { GraphForceControls, type GraphForces } from "./GraphForceControls"
import { GraphInspector } from "./GraphInspector"
import { GraphLegend } from "./GraphLegend"
import { GraphToolbar } from "./GraphToolbar"
import { LayoutSelector } from "./LayoutSelector"
import { SimilarityThresholdSlider } from "./SimilarityThresholdSlider"
import { NODE_FAMILIES, RELATIONSHIP_FAMILIES } from "./constants"
import type { GraphLayoutName, GraphMode, GraphNode, GraphViewerResponse } from "./types"

const DEFAULT_FOCUS = "or:ors:3.130"

const DEFAULT_FORCES: GraphForces = {
  legal: 80,
  embedding: 20,
  citation: 70,
  semantic: 55,
  history: 35,
  repulsion: 45,
  cluster: 50,
  labelDensity: 25,
  depth: 1,
}

export function GraphViewer() {
  const [focus, setFocus] = useState(DEFAULT_FOCUS)
  const [query, setQuery] = useState(DEFAULT_FOCUS)
  const [mode, setMode] = useState<GraphMode>("legal")
  const [layout, setLayout] = useState<GraphLayoutName>("force")
  const [similarityThreshold, setSimilarityThreshold] = useState(0.78)
  const [forces, setForces] = useState<GraphForces>(DEFAULT_FORCES)
  const [filters, setFilters] = useState<GraphFilters>({
    relationshipFamilies: new Set(["hierarchy", "citations", "semantics", "definitions", "deadlines", "notices", "history"]),
    nodeFamilies: new Set(Object.keys(NODE_FAMILIES)),
    includeChunks: false,
  })
  const [response, setResponse] = useState<GraphViewerResponse>(() => mockGraphResponse())
  const [selectedId, setSelectedId] = useState<string>(DEFAULT_FOCUS)
  const [loading, setLoading] = useState(false)
  const [warning, setWarning] = useState<string | null>(null)
  const [usingDemoGraph, setUsingDemoGraph] = useState(true)
  const [refreshKey, setRefreshKey] = useState(0)

  const relationshipTypes = useMemo(() => flattenFamilies(RELATIONSHIP_FAMILIES, filters.relationshipFamilies), [filters.relationshipFamilies])
  const nodeTypes = useMemo(() => flattenFamilies(NODE_FAMILIES, filters.nodeFamilies), [filters.nodeFamilies])

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    setWarning(null)

    const isCitation = /^(ORS|Chapter)\s+/i.test(focus)
    getGraphNeighborhood({
      id: isCitation ? undefined : focus,
      citation: isCitation ? focus : undefined,
      mode,
      depth: forces.depth,
      limit: 160,
      relationshipTypes,
      nodeTypes: nodeTypes.length === Object.values(NODE_FAMILIES).flat().length ? undefined : nodeTypes,
      includeChunks: filters.includeChunks,
      includeSimilarity: mode === "hybrid" || mode === "embedding_similarity",
      similarityThreshold,
    })
      .then((next) => {
        if (cancelled) return
        setResponse(normalizeResponse(next))
        setSelectedId(next.center?.id ?? focus)
        setUsingDemoGraph(false)
        if (next.layout?.name === "timeline") setLayout("timeline")
        if (next.layout?.name === "radial") setLayout("radial")
      })
      .catch((error) => {
        if (cancelled) return
        setResponse(mockGraphResponse())
        setSelectedId(DEFAULT_FOCUS)
        setUsingDemoGraph(true)
        setWarning(error instanceof Error ? `API unavailable: ${error.message}` : "API unavailable; showing bundled sample graph.")
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [focus, mode, forces.depth, filters.includeChunks, relationshipTypes, nodeTypes, similarityThreshold, refreshKey])

  const selectedNode = useMemo(
    () => response.nodes.find((node) => node.id === selectedId) ?? response.center ?? response.nodes[0],
    [response.nodes, response.center, selectedId],
  )

  function openNode(value: string) {
    setFocus(value)
    setQuery(value)
  }

  function changeMode(next: GraphMode) {
    setMode(next)
    setFilters((current) => ({
      ...current,
      relationshipFamilies: modeDefaultFamilies(next),
    }))
    if (next === "history") setLayout("timeline")
    else if (next === "citation") setLayout("radial")
    else if (next === "embedding_similarity") setLayout("embedding_projection")
    else setLayout("force")
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <DataStateBanner
        source={usingDemoGraph ? "demo" : "live"}
        error={warning ?? undefined}
        label="Graph data"
      />
      <div className="flex min-h-0 flex-1">
      <aside className="hidden w-80 shrink-0 overflow-y-auto border-r border-border bg-card/40 p-4 scrollbar-thin lg:block">
        <div className="mb-5">
          <div className="font-mono text-[11px] uppercase tracking-wider text-muted-foreground">Controls</div>
          <p className="mt-1 text-sm text-muted-foreground">Shape the legal graph, citation dependency, currentness, and semantic neighborhood.</p>
        </div>
        <div className="space-y-6">
          <section>
            <div className="mb-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground">Layout</div>
            <LayoutSelector value={layout} onChange={setLayout} />
          </section>
          <SimilarityThresholdSlider value={similarityThreshold} onChange={setSimilarityThreshold} />
          <GraphForceControls forces={forces} onChange={setForces} />
          <GraphFilterPanel filters={filters} onChange={setFilters} />
        </div>
      </aside>

      <section className="flex min-w-0 flex-1 flex-col">
        <GraphToolbar
          query={query}
          mode={mode}
          nodeCount={response.stats.nodeCount}
          edgeCount={response.stats.edgeCount}
          loading={loading}
          truncated={response.stats.truncated}
          onModeChange={changeMode}
          onOpen={openNode}
          onRefresh={() => setRefreshKey((key) => key + 1)}
        />
        {(warning || response.stats.warnings.length > 0) && (
          <div className="flex items-center gap-2 border-b border-border bg-warning/10 px-4 py-2 text-sm text-warning">
            <AlertTriangle className="h-4 w-4" />
            <span>{warning ?? response.stats.warnings[0]}</span>
          </div>
        )}
        <div className="relative min-h-0 flex-1">
          <GraphCanvas
            nodes={response.nodes}
            edges={response.edges}
            selectedId={selectedNode?.id}
            layout={layout}
            forces={forces}
            onSelect={setSelectedId}
            onRecenter={openNode}
          />
          <div className="absolute bottom-4 left-4 hidden w-64 md:block">
            <GraphLegend />
          </div>
        </div>
      </section>

      <GraphInspector
        node={selectedNode}
        edges={response.edges}
        onSelect={setSelectedId}
        onExpand={openNode}
      />
      </div>
    </div>
  )
}

function flattenFamilies<T extends Record<string, readonly string[]>>(families: T, enabled: Set<string>) {
  return Object.entries(families)
    .filter(([family]) => enabled.has(family))
    .flatMap(([, values]) => [...values])
}

function modeDefaultFamilies(mode: GraphMode) {
  if (mode === "citation") return new Set(["citations"])
  if (mode === "semantic") return new Set(["semantics", "definitions", "deadlines", "penalties", "notices"])
  if (mode === "history") return new Set(["history", "provenance"])
  if (mode === "hybrid") return new Set(["hierarchy", "citations", "semantics", "definitions", "history", "similarity"])
  if (mode === "embedding_similarity") return new Set(["similarity"])
  return new Set(["hierarchy", "citations", "semantics", "definitions", "deadlines", "notices", "history"])
}

function normalizeResponse(value: GraphViewerResponse): GraphViewerResponse {
  return {
    ...value,
    stats: value.stats ?? {
      nodeCount: value.nodes.length,
      edgeCount: value.edges.length,
      truncated: false,
      warnings: [],
    },
    nodes: value.nodes.map((node) => ({ ...node, labels: node.labels ?? [node.type], qcWarnings: node.qcWarnings ?? [] })),
  }
}

function mockGraphResponse(): GraphViewerResponse {
  const nodes: GraphNode[] = mockNodes.map((node) => ({
    id: node.id,
    label: node.label,
    type: node.type === "Statute" ? "LegalTextIdentity" : node.type,
    labels: [node.type === "Statute" ? "LegalTextIdentity" : node.type],
    citation: node.type === "Statute" ? node.label : undefined,
    status: node.status,
    qcWarnings: node.qc_status === "warning" ? ["Sample QC warning"] : [],
    sourceBacked: true,
    href: node.type === "Statute" ? `/statutes/${node.id}` : undefined,
  }))
  const edges = mockEdges.map((edge) => ({
    ...edge,
    label: edge.type,
    kind: "legal",
    sourceBacked: true,
  }))
  return {
    center: nodes[0],
    nodes,
    edges,
    layout: { name: "force" },
    stats: {
      nodeCount: nodes.length,
      edgeCount: edges.length,
      truncated: false,
      warnings: ["Showing sample graph until the API returns a neighborhood."],
    },
  }
}
