"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { AlertTriangle, GitBranch, RotateCcw } from "lucide-react"
import { getFullGraph, getGraphNeighborhood } from "@/lib/api"
import { classifyFallbackSource, type DataSource } from "@/lib/data-state"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
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
import type { GraphLayoutName, GraphMode, GraphNode, GraphViewerResponse, GraphViewScope } from "./types"

const DEFAULT_FOCUS = "or:ors:3.130"

const DEFAULT_FORCES: GraphForces = {
  repulsion: 45,
  cluster: 50,
  labelDensity: 25,
  depth: 1,
}

const FULL_GRAPH_FORCES: GraphForces = {
  repulsion: 30,
  cluster: 55,
  labelDensity: 8,
  depth: 2,
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
  const [viewScope, setViewScope] = useState<GraphViewScope>("neighborhood")
  const [layout, setLayout] = useState<GraphLayoutName>(() => modeDefaultLayout(initialMode))
  const [similarityThreshold, setSimilarityThreshold] = useState(0.78)
  const [forces, setForces] = useState<GraphForces>(DEFAULT_FORCES)
  const [filters, setFilters] = useState<GraphFilters>({
    relationshipFamilies: modeDefaultFamilies(initialMode),
    nodeFamilies: allNodeFamilies(),
    includeChunks: false,
  })
  const [response, setResponse] = useState<GraphViewerResponse>(() => emptyGraphResponse())
  const [selectedId, setSelectedId] = useState<string>(initialNode)
  const [loading, setLoading] = useState(false)
  const [warning, setWarning] = useState<string | null>(null)
  const [source, setSource] = useState<DataSource>("empty")
  const [refreshKey, setRefreshKey] = useState(0)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [inspectorOpen, setInspectorOpen] = useState(false)
  const [fullConfirmOpen, setFullConfirmOpen] = useState(false)
  const previousInitialFocus = useRef(initialFocus)
  const previousInitialMode = useRef(initialMode)

  const relationshipTypes = useMemo(() => flattenFamilies(RELATIONSHIP_FAMILIES, filters.relationshipFamilies), [filters.relationshipFamilies])
  const nodeTypes = useMemo(() => flattenFamilies(NODE_FAMILIES, filters.nodeFamilies), [filters.nodeFamilies])

  const loadFullGraph = useCallback(async () => {
    setFullConfirmOpen(false)
    setAdvancedOpen(false)
    setWarning(null)
    setLoading(true)
    setViewScope("full")
    setLayout("force")
    setForces(FULL_GRAPH_FORCES)
    setFilters({
      relationshipFamilies: allRelationshipFamilies(),
      nodeFamilies: allNodeFamilies(),
      includeChunks: true,
    })

    try {
      const next = await getFullGraph({
        includeChunks: true,
        includeSimilarity: true,
      })
      const normalized = normalizeResponse(next)
      setResponse(normalized)
      const nextSelected = normalized.center?.id ?? normalized.nodes[0]?.id ?? DEFAULT_FOCUS
      setSelectedId(nextSelected)
      setSource(normalized.nodes.length > 0 ? "live" : "empty")
      if (normalized.stats.warnings.length > 0) setWarning(normalized.stats.warnings[0])
    } catch (error) {
      setResponse(emptyGraphResponse())
      setSelectedId(DEFAULT_FOCUS)
      setSource(classifyFallbackSource(error))
      setWarning(error instanceof Error ? `Full graph unavailable: ${error.message}` : "Full graph unavailable.")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (previousInitialFocus.current === initialFocus) return
    previousInitialFocus.current = initialFocus
    const next = normalizeFocus(initialFocus)
    if (!next) return
    setViewScope("neighborhood")
    setFocus(next)
    setQuery(next)
    setSelectedId(next)
  }, [initialFocus])

  useEffect(() => {
    if (previousInitialMode.current === initialMode) return
    previousInitialMode.current = initialMode
    setViewScope("neighborhood")
    setMode(initialMode)
    setFilters((current) => ({
      ...current,
      relationshipFamilies: modeDefaultFamilies(initialMode),
    }))
    setLayout(modeDefaultLayout(initialMode))
  }, [initialMode])

  useEffect(() => {
    if (viewScope !== "neighborhood") return undefined

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
        setSource(classifyFallbackSource(error))
        setWarning(error instanceof Error ? `Graph API unavailable: ${error.message}` : "Graph API unavailable.")
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [focus, mode, forces.depth, filters.includeChunks, relationshipTypes, nodeTypes, similarityThreshold, refreshKey, viewScope])

  const selectedNode = useMemo(
    () => response.nodes.find((node) => node.id === selectedId) ?? response.center ?? response.nodes[0],
    [response.nodes, response.center, selectedId],
  )

  function openNode(value: string) {
    const next = normalizeFocus(value)
    if (!next) return
    setViewScope("neighborhood")
    setFocus(next)
    setQuery(next)
    setSelectedId(next)
    updateGraphUrl(next, mode)
  }

  function changeMode(next: GraphMode) {
    setViewScope("neighborhood")
    setMode(next)
    setFilters((current) => ({
      ...current,
      relationshipFamilies: modeDefaultFamilies(next),
    }))
    setLayout(modeDefaultLayout(next))
    updateGraphUrl(focus, next)
  }

  function refreshGraph() {
    if (viewScope === "full") {
      void loadFullGraph()
      return
    }
    setRefreshKey((key) => key + 1)
  }

  function resetToNeighborhood() {
    setViewScope("neighborhood")
    setForces(DEFAULT_FORCES)
    setFilters({
      relationshipFamilies: modeDefaultFamilies(mode),
      nodeFamilies: allNodeFamilies(),
      includeChunks: false,
    })
    setLayout(modeDefaultLayout(mode))
    setFocus(DEFAULT_FOCUS)
    setQuery(DEFAULT_FOCUS)
    setSelectedId(DEFAULT_FOCUS)
    updateGraphUrl(DEFAULT_FOCUS, mode)
  }

  function renderControls() {
    return (
      <GraphControls
        focus={focus}
        mode={mode}
        layout={layout}
        viewScope={viewScope}
        loading={loading}
        similarityThreshold={similarityThreshold}
        forces={forces}
        filters={filters}
        onLayoutChange={setLayout}
        onSimilarityThresholdChange={setSimilarityThreshold}
        onForcesChange={setForces}
        onFiltersChange={setFilters}
        onLoadFullGraph={() => setFullConfirmOpen(true)}
        onResetNeighborhood={resetToNeighborhood}
      />
    )
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <DataStateBanner
        source={source}
        error={warning ?? undefined}
        label="Graph data"
      />
      <div className="flex min-h-0 flex-1">
        <section className="flex min-w-0 flex-1 flex-col">
          <GraphToolbar
            query={query}
            mode={mode}
            nodeCount={response.stats.nodeCount}
            edgeCount={response.stats.edgeCount}
            loading={loading}
            truncated={response.stats.truncated}
            viewScope={viewScope}
            onModeChange={changeMode}
            onOpen={openNode}
            onOpenAdvanced={() => setAdvancedOpen(true)}
            onOpenInspector={() => setInspectorOpen(true)}
            onRefresh={refreshGraph}
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
                viewScope={viewScope}
                onSelect={setSelectedId}
                onRecenter={openNode}
              />
            ) : (
              <GraphEmptyState
                loading={loading}
                viewScope={viewScope}
                onRetry={refreshGraph}
                onReset={resetToNeighborhood}
              />
            )}
            <div className="absolute bottom-4 left-4 hidden w-64 md:block">
              <GraphLegend compact={viewScope === "full"} />
            </div>
            {selectedNode && (
              <SelectedNodeSummary node={selectedNode} edges={response.edges} viewScope={viewScope} />
            )}
          </div>
        </section>

        <GraphInspector
          node={selectedNode}
          edges={response.edges}
          onSelect={setSelectedId}
          onExpand={openNode}
        />
      </div>
      <Sheet open={advancedOpen} onOpenChange={setAdvancedOpen}>
        <SheetContent side="left" className="w-[min(94vw,32rem)] gap-0 p-0 sm:max-w-lg">
          <SheetHeader className="border-b border-border pr-12">
            <SheetTitle>Advanced graph</SheetTitle>
            <SheetDescription>Layout, filters, paths, and full corpus rendering.</SheetDescription>
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
      <AlertDialog open={fullConfirmOpen} onOpenChange={setFullConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Render full graph?</AlertDialogTitle>
            <AlertDialogDescription>
              This loads every available node and edge from Neo4j into the canvas. Large corpora can take a while to draw.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={() => void loadFullGraph()}>
              Render full graph
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}

function GraphControls({
  focus,
  mode,
  layout,
  viewScope,
  loading,
  similarityThreshold,
  forces,
  filters,
  onLayoutChange,
  onSimilarityThresholdChange,
  onForcesChange,
  onFiltersChange,
  onLoadFullGraph,
  onResetNeighborhood,
}: {
  focus: string
  mode: GraphMode
  layout: GraphLayoutName
  viewScope: GraphViewScope
  loading: boolean
  similarityThreshold: number
  forces: GraphForces
  filters: GraphFilters
  onLayoutChange: (layout: GraphLayoutName) => void
  onSimilarityThresholdChange: (value: number) => void
  onForcesChange: (forces: GraphForces) => void
  onFiltersChange: (filters: GraphFilters) => void
  onLoadFullGraph: () => void
  onResetNeighborhood: () => void
}) {
  return (
    <div className="space-y-6">
      <section className="rounded border border-border bg-background p-3">
        <div className="mb-3 flex items-center justify-between gap-3">
          <div>
            <div className="font-mono text-[11px] uppercase tracking-wide text-muted-foreground">Scope</div>
            <div className="mt-1 font-mono text-xs uppercase text-foreground">{viewScope === "full" ? "Full graph" : "Focused neighborhood"}</div>
          </div>
          <Badge variant={viewScope === "full" ? "default" : "outline"} className="font-mono text-[10px] uppercase">
            {viewScope}
          </Badge>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button type="button" size="sm" onClick={onLoadFullGraph} disabled={loading}>
            <GitBranch className="h-4 w-4" />
            Load full graph
          </Button>
          <Button type="button" variant="outline" size="sm" onClick={onResetNeighborhood}>
            <RotateCcw className="h-4 w-4" />
            Default focus
          </Button>
        </div>
      </section>
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

function GraphEmptyState({
  loading,
  viewScope,
  onRetry,
  onReset,
}: {
  loading: boolean
  viewScope: GraphViewScope
  onRetry: () => void
  onReset: () => void
}) {
  return (
    <div className="flex h-full min-h-[520px] items-center justify-center p-8 text-center">
      <div className="max-w-sm rounded border border-border bg-card/85 p-5 shadow-sm backdrop-blur">
        <div className="font-mono text-[11px] uppercase tracking-wide text-muted-foreground">
          {loading ? "Loading graph" : "No graph returned"}
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          {viewScope === "full" ? "The full graph request returned no nodes." : "This focus has no visible neighborhood with the current filters."}
        </p>
        <div className="mt-4 flex justify-center gap-2">
          <Button type="button" size="sm" onClick={onRetry} disabled={loading}>
            Retry
          </Button>
          <Button type="button" variant="outline" size="sm" onClick={onReset}>
            Default focus
          </Button>
        </div>
      </div>
    </div>
  )
}

function SelectedNodeSummary({
  node,
  edges,
  viewScope,
}: {
  node: GraphNode
  edges: GraphViewerResponse["edges"]
  viewScope: GraphViewScope
}) {
  const connected = edges.filter((edge) => edge.source === node.id || edge.target === node.id).length

  return (
    <div className="absolute right-4 top-4 hidden max-w-xs rounded border border-border bg-card/95 p-3 shadow-sm backdrop-blur md:block xl:hidden">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="truncate font-mono text-sm font-semibold text-foreground">{node.label}</div>
          <div className="mt-1 truncate font-mono text-[10px] uppercase text-muted-foreground">{node.type}</div>
        </div>
        <Badge variant={viewScope === "full" ? "default" : "outline"} className="font-mono text-[10px] uppercase">
          {connected}
        </Badge>
      </div>
      {node.textSnippet && <p className="mt-2 line-clamp-2 text-xs text-muted-foreground">{node.textSnippet}</p>}
    </div>
  )
}

function flattenFamilies<T extends Record<string, readonly string[]>>(families: T, enabled: Set<string>) {
  return Object.entries(families)
    .filter(([family]) => enabled.has(family))
    .flatMap(([, values]) => [...values])
}

function allRelationshipFamilies() {
  return new Set(Object.keys(RELATIONSHIP_FAMILIES))
}

function allNodeFamilies() {
  return new Set(Object.keys(NODE_FAMILIES))
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
