"use client"

import dynamic from "next/dynamic"
import { usePathname, useRouter, useSearchParams } from "next/navigation"
import { useEffect, useMemo, useState, useTransition, type Dispatch, type SetStateAction } from "react"
import type { Chunk, InboundCitation, OutboundCitation, Provision, StatutePageResponse } from "@/lib/types"
import { getChunks, getCitations, getHistory, getSemantics } from "@/lib/api"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { cn } from "@/lib/utils"
import { statuteLoadedStateFor, type StatuteLoadedState } from "./load-state"

const TextTab = dynamic(() => import("./tabs/text-tab").then((mod) => mod.TextTab), { loading: TabLoading })
const ProvisionTreeTab = dynamic(() => import("./tabs/provision-tree-tab").then((mod) => mod.ProvisionTreeTab), { loading: TabLoading })
const CitationsTab = dynamic(() => import("./tabs/citations-tab").then((mod) => mod.CitationsTab), { loading: TabLoading })
const DefinitionsTab = dynamic(() => import("./tabs/definitions-tab").then((mod) => mod.DefinitionsTab), { loading: TabLoading })
const DeadlinesTab = dynamic(() => import("./tabs/deadlines-tab").then((mod) => mod.DeadlinesTab), { loading: TabLoading })
const ExceptionsTab = dynamic(() => import("./tabs/exceptions-tab").then((mod) => mod.ExceptionsTab), { loading: TabLoading })
const ChunksTab = dynamic(() => import("./tabs/chunks-tab").then((mod) => mod.ChunksTab), { loading: TabLoading })
const VersionsTab = dynamic(() => import("./tabs/versions-tab").then((mod) => mod.VersionsTab), { loading: TabLoading })
const SourceTab = dynamic(() => import("./tabs/source-tab").then((mod) => mod.SourceTab), { loading: TabLoading })
const GraphTab = dynamic(() => import("./tabs/graph-tab").then((mod) => mod.GraphTab), { loading: TabLoading })

const TABS = [
  { id: "text", label: "Text" },
  { id: "tree", label: "Provision tree" },
  { id: "citations", label: "Citations" },
  { id: "definitions", label: "Definitions" },
  { id: "deadlines", label: "Deadlines" },
  { id: "exceptions", label: "Exceptions" },
  { id: "chunks", label: "Chunks" },
  { id: "versions", label: "Versions" },
  { id: "source", label: "Source" },
  { id: "graph", label: "Graph" },
] as const

type TabId = (typeof TABS)[number]["id"]

export function StatuteTabs({
  data,
  initialTab,
  onDataChange,
  onLoadedChange,
}: {
  data: StatutePageResponse
  initialTab?: string
  onDataChange: Dispatch<SetStateAction<StatutePageResponse>>
  onLoadedChange?: (state: StatuteLoadedState) => void
}) {
  const router = useRouter()
  const pathname = usePathname()
  const searchParams = useSearchParams()
  const [, startTransition] = useTransition()
  const [active, setActive] = useState<TabId>(isTabId(initialTab) ? initialTab : "text")
  const [loaded, setLoaded] = useState(() => statuteLoadedStateFor(data))
  const [loadingTab, setLoadingTab] = useState<TabId | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)

  const citationId = data.identity.canonical_id || data.identity.citation
  const activeData = useMemo(() => data, [data])

  useEffect(() => {
    setActive(isTabId(initialTab) ? initialTab : "text")
  }, [data.identity.canonical_id, initialTab])

  useEffect(() => {
    setLoaded(statuteLoadedStateFor(data))
    setLoadingTab(null)
    setLoadError(null)
    // Reset only when the statute changes; empty successful lazy responses still count as loaded.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data.identity.canonical_id])

  useEffect(() => {
    onLoadedChange?.(loaded)
  }, [loaded, onLoadedChange])

  const handleTabChange = (value: string) => {
    if (!isTabId(value)) return
    setActive(value)
    startTransition(() => {
      const next = new URLSearchParams(searchParams.toString())
      if (value === "text") {
        next.delete("tab")
      } else {
        next.set("tab", value)
      }
      const query = next.toString()
      router.replace(query ? `${pathname}?${query}` : pathname, { scroll: false })
    })
  }

  async function loadForTab(tab: TabId) {
    if (tab === "citations" && !loaded.citations) {
      await loadCitations()
    }
    if ((tab === "definitions" || tab === "deadlines" || tab === "exceptions") && !loaded.semantics) {
      await loadSemantics(tab)
    }
    if (tab === "chunks" && !loaded.chunks) {
      await loadChunks()
    }
    if (tab === "versions" && !loaded.history) {
      await loadHistory(tab)
    }
  }

  async function loadCitations() {
    setLoadingTab("citations")
    setLoadError(null)
    try {
      const citations = await getCitations(citationId)
      const outbound = (citations.outbound ?? []).map(mapOutboundCitation)
      const inbound = (citations.inbound ?? []).map(mapInboundCitation)
      onDataChange((current) => ({
        ...current,
        outbound_citations: outbound,
        inbound_citations: inbound,
        summary_counts: updateSummaryCounts(current, {
          citation_counts: { outbound: outbound.length, inbound: inbound.length },
        }),
      }))
      setLoaded((current) => ({ ...current, citations: true }))
    } catch (error) {
      setLoadError(dataErrorMessage(error))
    } finally {
      setLoadingTab(null)
    }
  }

  async function loadSemantics(tab: TabId) {
    setLoadingTab(tab)
    setLoadError(null)
    try {
      const semantics = await getSemantics(citationId)
      const definitions = (semantics.definitions ?? []).map((item, index) => ({
        definition_id: `definition:${index}`,
        term: item.term,
        text: item.text,
        source_provision: item.source_provision,
        scope: item.scope || data.identity.citation,
      }))
      const exceptions = (semantics.exceptions ?? []).map((item, index) => ({
        exception_id: `exception:${index}`,
        text: item.text,
        applies_to_provision: item.source_provision,
        source_provision: item.source_provision,
      }))
      const deadlines = (semantics.deadlines ?? []).map((item, index) => ({
        deadline_id: `deadline:${index}`,
        description: item.description,
        duration: item.duration,
        trigger: item.trigger,
        source_provision: item.source_provision,
      }))
      const penalties = (semantics.penalties ?? []).map((item, index) => ({
        penalty_id: `penalty:${index}`,
        description: item.text,
        category: "administrative" as const,
        source_provision: item.source_provision,
      }))
      onDataChange((current) => ({
        ...current,
        definitions,
        exceptions,
        deadlines,
        penalties,
        summary_counts: updateSummaryCounts(current, {
          semantic_counts: {
            definitions: definitions.length,
            exceptions: exceptions.length,
            deadlines: deadlines.length,
            penalties: penalties.length,
          },
        }),
      }))
      setLoaded((current) => ({ ...current, semantics: true }))
    } catch (error) {
      setLoadError(dataErrorMessage(error))
    } finally {
      setLoadingTab(null)
    }
  }

  async function loadChunks() {
    setLoadingTab("chunks")
    setLoadError(null)
    try {
      const response = await getChunks(citationId)
      onDataChange((current) => ({
        ...current,
        chunks: (response.chunks ?? []).map(mapChunk),
      }))
      setLoaded((current) => ({ ...current, chunks: true }))
    } catch (error) {
      setLoadError(dataErrorMessage(error))
    } finally {
      setLoadingTab(null)
    }
  }

  async function loadHistory(tab: TabId) {
    setLoadingTab(tab)
    setLoadError(null)
    try {
      const history = await getHistory(citationId)
      onDataChange((current) => ({
        ...current,
        source_notes: history.source_notes ?? [],
      }))
      setLoaded((current) => ({ ...current, history: true }))
    } catch (error) {
      setLoadError(dataErrorMessage(error))
    } finally {
      setLoadingTab(null)
    }
  }

  useEffect(() => {
    void loadForTab(active)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active])

  return (
    <Tabs value={active} onValueChange={handleTabChange} className="flex min-h-0 flex-1 flex-col gap-0 overflow-hidden">
      <div className="border-b border-border bg-card px-4">
        <TabsList className="h-auto w-full justify-start overflow-x-auto rounded-none bg-transparent p-0 scrollbar-thin">
          {TABS.map((tab) => {
            const count = getTabCount(tab.id, activeData, loaded)
            return (
              <TabsTrigger
                key={tab.id}
                value={tab.id}
                className="relative h-10 flex-none rounded-none border-0 bg-transparent px-3 py-2 text-xs data-[state=active]:bg-transparent data-[state=active]:text-primary data-[state=active]:shadow-none"
              >
                {tab.label}
                {count !== null && (
                  <span
                    className={cn(
                      "rounded px-1 font-mono text-[10px] tabular-nums",
                      active === tab.id ? "bg-primary/15 text-primary" : "bg-muted text-muted-foreground",
                    )}
                  >
                    {count}
                  </span>
                )}
                {active === tab.id && <span className="absolute inset-x-0 bottom-0 h-0.5 bg-primary" />}
              </TabsTrigger>
            )
          })}
        </TabsList>
      </div>

      {(loadingTab || loadError) && (
        <div className="border-b border-border bg-muted/30 px-4 py-2 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
          {loadError ? `Could not load tab data: ${loadError}` : `Loading ${loadingTab ?? active} data`}
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-y-auto scrollbar-thin">
        <TabsContent value="text" className="m-0 h-full"><TextTab data={activeData} /></TabsContent>
        <TabsContent value="tree" className="m-0 h-full"><ProvisionTreeTab data={activeData} /></TabsContent>
        <TabsContent value="citations" className="m-0 h-full">{loadingTab === "citations" ? <TabPendingPlaceholder tab="citations" count={getTabCount("citations", activeData, loaded)} /> : <CitationsTab data={activeData} />}</TabsContent>
        <TabsContent value="definitions" className="m-0 h-full">{loadingTab === "definitions" ? <TabPendingPlaceholder tab="definitions" count={getTabCount("definitions", activeData, loaded)} /> : <DefinitionsTab data={activeData} />}</TabsContent>
        <TabsContent value="deadlines" className="m-0 h-full">{loadingTab === "deadlines" ? <TabPendingPlaceholder tab="deadlines" count={getTabCount("deadlines", activeData, loaded)} /> : <DeadlinesTab data={activeData} />}</TabsContent>
        <TabsContent value="exceptions" className="m-0 h-full">{loadingTab === "exceptions" ? <TabPendingPlaceholder tab="exceptions" count={getTabCount("exceptions", activeData, loaded)} /> : <ExceptionsTab data={activeData} />}</TabsContent>
        <TabsContent value="chunks" className="m-0 h-full">{loadingTab === "chunks" ? <TabPendingPlaceholder tab="chunks" count={getTabCount("chunks", activeData, loaded)} /> : <ChunksTab data={activeData} />}</TabsContent>
        <TabsContent value="versions" className="m-0 h-full"><VersionsTab data={activeData} /></TabsContent>
        <TabsContent value="source" className="m-0 h-full"><SourceTab data={activeData} /></TabsContent>
        <TabsContent value="graph" className="m-0 h-full"><GraphTab data={activeData} /></TabsContent>
      </div>
    </Tabs>
  )
}

function TabLoading() {
  return <div className="px-6 py-8 text-sm text-muted-foreground">Loading...</div>
}

function TabPendingPlaceholder({ tab, count }: { tab: TabId; count: number | null }) {
  const rows = Math.max(2, Math.min(count ?? 3, 6))
  return (
    <div className="px-6 py-6" aria-live="polite">
      <div className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        Loading {tab} data{typeof count === "number" && count > 0 ? ` · ${count} expected` : ""}
      </div>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        {Array.from({ length: rows }).map((_, index) => (
          <div key={index} className="rounded border border-border bg-card p-4">
            <div className="h-3 w-20 rounded bg-muted" />
            <div className="mt-3 h-4 w-2/3 rounded bg-muted" />
            <div className="mt-2 h-3 w-full rounded bg-muted" />
            <div className="mt-2 h-3 w-5/6 rounded bg-muted" />
          </div>
        ))}
      </div>
    </div>
  )
}

function updateSummaryCounts(
  current: StatutePageResponse,
  next: {
    citation_counts?: { outbound: number; inbound: number }
    semantic_counts?: Partial<NonNullable<StatutePageResponse["summary_counts"]>["semantic_counts"]>
  },
): StatutePageResponse["summary_counts"] {
  const previous = current.summary_counts
  return {
    provision_count: previous?.provision_count ?? countProvisions(current.provisions),
    citation_counts: next.citation_counts ?? previous?.citation_counts ?? {
      outbound: current.outbound_citations.length,
      inbound: current.inbound_citations.length,
    },
    semantic_counts: {
      obligations: previous?.semantic_counts.obligations ?? 0,
      exceptions: previous?.semantic_counts.exceptions ?? current.exceptions.length,
      deadlines: previous?.semantic_counts.deadlines ?? current.deadlines.length,
      penalties: previous?.semantic_counts.penalties ?? current.penalties.length,
      definitions: previous?.semantic_counts.definitions ?? current.definitions.length,
      ...next.semantic_counts,
    },
  }
}

function getTabCount(id: TabId, data: StatutePageResponse, loaded?: StatuteLoadedState): number | null {
  switch (id) {
    case "tree":
      return data.summary_counts?.provision_count ?? countProvisions(data.provisions)
    case "citations":
      if (loaded?.citations) return data.outbound_citations.length + data.inbound_citations.length
      return (data.summary_counts?.citation_counts.outbound ?? data.outbound_citations.length)
        + (data.summary_counts?.citation_counts.inbound ?? data.inbound_citations.length)
    case "definitions":
      if (loaded?.semantics) return data.definitions.length
      return data.summary_counts?.semantic_counts.definitions ?? data.definitions.length
    case "deadlines":
      if (loaded?.semantics) return data.deadlines.length
      return data.summary_counts?.semantic_counts.deadlines ?? data.deadlines.length
    case "exceptions":
      if (loaded?.semantics) return data.exceptions.length + data.penalties.length
      return (data.summary_counts?.semantic_counts.exceptions ?? data.exceptions.length)
        + (data.summary_counts?.semantic_counts.penalties ?? data.penalties.length)
    case "chunks":
      return data.chunks.length
    case "versions":
      return data.versions.length
    default:
      return null
  }
}

function countProvisions(provisions: Provision[]): number {
  let count = 0
  const walk = (provision: Provision) => {
    count += 1
    provision.children?.forEach(walk)
  }
  provisions.forEach(walk)
  return count
}

function isTabId(value?: string): value is TabId {
  return !!value && TABS.some((tab) => tab.id === value)
}

function mapOutboundCitation(citation: any): OutboundCitation {
  return {
    target_canonical_id: citation.target_canonical_id,
    target_citation: citation.target_citation,
    context_snippet: citation.context_snippet,
    source_provision: citation.source_provision,
    resolved: Boolean(citation.resolved),
  }
}

function mapInboundCitation(citation: any): InboundCitation {
  return {
    source_canonical_id: citation.target_canonical_id ?? "",
    source_citation: citation.target_citation,
    source_title: citation.target_citation,
    source_provision: citation.source_provision,
    context_snippet: citation.context_snippet,
  }
}

function mapChunk(chunk: any): Chunk {
  return {
    chunk_id: chunk.chunk_id,
    chunk_type: normalizeChunkType(chunk.chunk_type),
    source_kind: chunk.source_kind === "statute" ? "statute" : "provision",
    source_id: chunk.source_id ?? "",
    text: chunk.text ?? "",
    embedding_policy: normalizeEmbeddingPolicy(chunk.embedding_policy),
    answer_policy: normalizeAnswerPolicy(chunk.answer_policy),
    search_weight: Number(chunk.search_weight ?? 1),
    embedded: Boolean(chunk.embedded),
    parser_confidence: Number(chunk.parser_confidence ?? 1),
  }
}

function normalizeEmbeddingPolicy(value?: string): Chunk["embedding_policy"] {
  return value === "secondary" || value === "none" ? value : "primary"
}

function normalizeChunkType(value?: string): Chunk["chunk_type"] {
  const allowed = [
    "full_statute",
    "contextual_provision",
    "definition_block",
    "exception_block",
    "deadline_block",
    "penalty_block",
    "citation_context",
  ]
  return allowed.includes(value ?? "") ? value as Chunk["chunk_type"] : "contextual_provision"
}

function normalizeAnswerPolicy(value?: string): Chunk["answer_policy"] {
  return value === "preferred" || value === "context_only" ? value : "supporting"
}

function dataErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}
