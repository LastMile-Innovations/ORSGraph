"use client"

import { useEffect, useMemo, useRef, useState } from "react"
import { AlertTriangle } from "lucide-react"
import { getGraphNeighborhood } from "@/lib/api"
import type { DataSource } from "@/lib/data-state"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet"
import { GraphCanvas } from "./GraphCanvas"
import { GraphFilterPanel, type GraphFilters } from "./GraphFilterPanel"
import { GraphForceControls, type GraphForces } from "./GraphForceControls"
import { GraphInspector } from "./GraphInspector"
import { GraphLegend } from "./GraphLegend"
import { GraphToolbar } from "./GraphToolbar"
import { LayoutSelector } from "./LayoutSelector"
import { PathFinderPanel } from "./PathFinderPanel"
import { SimilarityThresholdSlider } from "./SimilarityThresholdSlider"
import { NODE_FAMILIES, RELATIONSHIP_FAMILIES } from "./constants"
import type { GraphLayoutName, GraphMode, GraphViewerResponse } from "./types"

const DEFAULT_FOCUS = "or:ors:3.130"

const DEFAULT_FORCES: GraphForces = {
  repulsion: 45,
  cluster: 50,
  labelDensity: 25,
  depth: 1,
}

export function GraphViewer({
  initialFocus,
  initialMode = "legal",
}: {
  initialFocus?: string
  initialMode?: GraphMode
}) {
  const initialNode = normalizeFocus(initialFocus) ?? DEFAULT_FOCUS
  const [focus, setFocus] = useState(initialNode)
  const [query, setQuery] = useState(initialNode)
  const [mode, setMode] = useState<GraphMode>(initialMode)
  const [layout, setLayout] = useState<GraphLayoutName>(() => modeDefaultLayout(initialMode))
  const [similarityThreshold, setSimilarityThreshold] = useState(0.78)
  const [forces, setForces] = useState<GraphForces>(DEFAULT_FORCES)
  const [filters, setFilters] = useState<GraphFilters>({
    relationshipFamilies: modeDefaultFamilies(initialMode),
    nodeFamilies: new Set(Object.keys(NODE_FAMILIES)),
    includeChunks: false,
  })
  const [response, setResponse] = useState<GraphViewerResponse>(() => emptyGraphResponse())
  const [selectedId, setSelectedId] = useState<string>(initialNode)
  const [loading, setLoading] = useState(false)
  const [warning, setWarning] = useState<string | null>(null)
  const [source, setSource] = useState<DataSource>("empty")
  const [refreshKey, setRefreshKey] = useState(0)
  const [controlsOpen, setControlsOpen] = useState(false)
  const [inspectorOpen, setInspectorOpen] = useState(false)
  const previousInitialFocus = useRef(initialFocus)
  const previousInitialMode = useRef(initialMode)

  const relationshipTypes = useMemo(() => flattenFamilies(RELATIONSHIP_FAMILIES, filters.relationshipFamilies), [filters.relationshipFamilies])
  const nodeTypes = useMemo(() => flattenFamilies(NODE_FAMILIES, filters.nodeFamilies), [filters.nodeFamilies])

  useEffect(() => {
    if (previousInitialFocus.current === initialFocus) return
    previousInitialFocus.current = initialFocus
    const next = normalizeFocus(initialFocus)
    if (!next) return
    setFocus(next)
    setQuery(next)
    setSelectedId(next)
  }, [initialFocus])

  useEffect(() => {
    if (previousInitialMode.current === initialMode) return
    previousInitialMode.current = initialMode
    setMode(initialMode)
    setFilters((current) => ({
      ...current,
      relationshipFamilies: modeDefaultFamilies(initialMode),
    }))
    setLayout(modeDefaultLayout(initialMode))
  }, [initialMode])

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
        setSource(next.nodes.length > 0 ? "live" : "empty")
        if (next.layout?.name === "timeline") setLayout("timeline")
        if (next.layout?.name === "radial") setLayout("radial")
      })
      .catch((error) => {
        if (cancelled) return
        setResponse(emptyGraphResponse())
        setSelectedId(focus)
        setSource("error")
        setWarning(error instanceof Error ? `Graph API unavailable: ${error.message}` : "Graph API unavailable.")
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
    const next = normalizeFocus(value)
    if (!next) return
    setFocus(next)
    setQuery(next)
    setSelectedId(next)
    updateGraphUrl(next, mode)
  }

  function changeMode(next: GraphMode) {
    setMode(next)
    setFilters((current) => ({
      ...current,
      relationshipFamilies: modeDefaultFamilies(next),
    }))
    setLayout(modeDefaultLayout(next))
    updateGraphUrl(focus, next)
  }

  function renderControls() {
    return (
      <GraphControls
        focus={focus}
        mode={mode}
        layout={layout}
        similarityThreshold={similarityThreshold}
        forces={forces}
        filters={filters}
        onLayoutChange={setLayout}
        onSimilarityThresholdChange={setSimilarityThreshold}
        onForcesChange={setForces}
        onFiltersChange={setFilters}
      />
    )
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <DataStateBanner
        source={source}
        error={warning ?? undefined}
        label="Graph data"
      />
      <div className="flex min-h-0 flex-1">
        <aside className="hidden w-80 shrink-0 overflow-y-auto border-r border-border bg-card/40 p-4 scrollbar-thin lg:block">
          <div className="mb-5">
            <div className="font-mono text-[11px] uppercase tracking-wider text-muted-foreground">Controls</div>
            <p className="mt-1 text-sm text-muted-foreground">Shape the legal graph, citation dependency, currentness, and semantic neighborhood.</p>
          </div>
          {renderControls()}
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
            onOpenControls={() => setControlsOpen(true)}
            onOpenInspector={() => setInspectorOpen(true)}
            onRefresh={() => setRefreshKey((key) => key + 1)}
          />
          {(warning || response.stats.warnings.length > 0) && (
            <div className="flex items-center gap-2 border-b border-border bg-warning/10 px-4 py-2 text-sm text-warning">
              <AlertTriangle className="h-4 w-4" />
              <span>{warning ?? response.stats.warnings[0]}</span>
            </div>
          )}
          <div className="relative min-h-0 flex-1">
            {response.nodes.length > 0 ? (
              <GraphCanvas
                nodes={response.nodes}
                edges={response.edges}
                selectedId={selectedNode?.id}
                layout={layout}
                forces={forces}
                onSelect={setSelectedId}
                onRecenter={openNode}
              />
            ) : (
              <div className="flex h-full items-center justify-center p-8 text-center text-sm text-muted-foreground">
                No graph neighborhood returned for this focus.
              </div>
            )}
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
      <Sheet open={controlsOpen} onOpenChange={setControlsOpen}>
        <SheetContent side="left" className="w-[min(92vw,28rem)] gap-0 p-0 sm:max-w-md">
          <SheetHeader className="border-b border-border pr-12">
            <SheetTitle>Graph controls</SheetTitle>
            <SheetDescription>Layout, depth, filters, and path finding.</SheetDescription>
          </SheetHeader>
          <div className="min-h-0 flex-1 overflow-y-auto p-4">
            {renderControls()}
          </div>
        </SheetContent>
      </Sheet>
      <Sheet open={inspectorOpen} onOpenChange={setInspectorOpen}>
        <SheetContent side="right" className="w-[min(92vw,28rem)] gap-0 p-0 sm:max-w-md">
          <SheetHeader className="border-b border-border pr-12">
            <SheetTitle>Graph inspector</SheetTitle>
            <SheetDescription>Selected node details, relationships, and QC.</SheetDescription>
          </SheetHeader>
          <GraphInspector
            node={selectedNode}
            edges={response.edges}
            onSelect={setSelectedId}
            onExpand={openNode}
            className="flex h-auto min-h-0 w-full flex-1 border-l-0 bg-transparent"
          />
        </SheetContent>
      </Sheet>
    </div>
  )
}

function GraphControls({
  focus,
  mode,
  layout,
  similarityThreshold,
  forces,
  filters,
  onLayoutChange,
  onSimilarityThresholdChange,
  onForcesChange,
  onFiltersChange,
}: {
  focus: string
  mode: GraphMode
  layout: GraphLayoutName
  similarityThreshold: number
  forces: GraphForces
  filters: GraphFilters
  onLayoutChange: (layout: GraphLayoutName) => void
  onSimilarityThresholdChange: (value: number) => void
  onForcesChange: (forces: GraphForces) => void
  onFiltersChange: (filters: GraphFilters) => void
}) {
  return (
    <div className="space-y-6">
      <section>
        <div className="mb-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground">Layout</div>
        <LayoutSelector value={layout} onChange={onLayoutChange} />
      </section>
      {modeSupportsSimilarity(mode) && (
        <SimilarityThresholdSlider value={similarityThreshold} onChange={onSimilarityThresholdChange} />
      )}
      <section>
        <div className="mb-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground">Layout controls</div>
        <GraphForceControls forces={forces} onChange={onForcesChange} />
      </section>
      <GraphFilterPanel filters={filters} onChange={onFiltersChange} />
      <PathFinderPanel mode={mode} initialFrom={focus} />
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

function modeDefaultLayout(mode: GraphMode): GraphLayoutName {
  if (mode === "history") return "timeline"
  if (mode === "citation") return "radial"
  if (mode === "embedding_similarity") return "embedding_projection"
  return "force"
}

function modeSupportsSimilarity(mode: GraphMode) {
  return mode === "hybrid" || mode === "embedding_similarity"
}

function normalizeFocus(value: string | undefined) {
  const next = value?.trim()
  return next || undefined
}

function updateGraphUrl(focus: string, mode: GraphMode) {
  if (typeof window === "undefined") return
  const url = new URL(window.location.href)
  url.searchParams.set("focus", focus)
  if (mode === "legal") url.searchParams.delete("mode")
  else url.searchParams.set("mode", mode)
  window.history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`)
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

function emptyGraphResponse(): GraphViewerResponse {
  return {
    center: null,
    nodes: [],
    edges: [],
    layout: { name: "force" },
    stats: {
      nodeCount: 0,
      edgeCount: 0,
      truncated: false,
      warnings: [],
    },
  }
}
