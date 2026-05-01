"use client"

import dynamic from "next/dynamic"
import { usePathname, useRouter, useSearchParams } from "next/navigation"
import { useEffect, useMemo, useState, useTransition } from "react"
import type { Chunk, InboundCitation, OutboundCitation, Provision, StatutePageResponse } from "@/lib/types"
import { getChunks, getCitations, getHistory, getSemantics } from "@/lib/api"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { cn } from "@/lib/utils"

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
const QCTab = dynamic(() => import("./tabs/qc-tab").then((mod) => mod.QCTab), { loading: TabLoading })

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
  { id: "qc", label: "QC" },
] as const

type TabId = (typeof TABS)[number]["id"]

export function StatuteTabs({ data, initialTab }: { data: StatutePageResponse; initialTab?: string }) {
  const router = useRouter()
  const pathname = usePathname()
  const searchParams = useSearchParams()
  const [isPending, startTransition] = useTransition()
  const [active, setActive] = useState<TabId>(isTabId(initialTab) ? initialTab : "text")
  const [tabData, setTabData] = useState(data)
  const [loaded, setLoaded] = useState({
    citations: data.outbound_citations.length > 0 || data.inbound_citations.length > 0,
    semantics: data.definitions.length > 0 || data.deadlines.length > 0 || data.exceptions.length > 0 || data.penalties.length > 0,
    chunks: data.chunks.length > 0,
    history: Boolean(data.source_notes?.length),
  })
  const [loadingTab, setLoadingTab] = useState<TabId | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)

  const citationId = data.identity.canonical_id || data.identity.citation
  const activeData = useMemo(() => tabData, [tabData])

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
      await loadSemantics()
    }
    if (tab === "chunks" && !loaded.chunks) {
      await loadChunks()
    }
    if ((tab === "source" || tab === "versions" || tab === "qc") && !loaded.history) {
      await loadHistory()
    }
  }

  async function loadCitations() {
    setLoadingTab("citations")
    setLoadError(null)
    try {
      const citations = await getCitations(citationId)
      setTabData((current) => ({
        ...current,
        outbound_citations: (citations.outbound ?? []).map(mapOutboundCitation),
        inbound_citations: (citations.inbound ?? []).map(mapInboundCitation),
      }))
      setLoaded((current) => ({ ...current, citations: true }))
    } catch (error) {
      setLoadError(dataErrorMessage(error))
    } finally {
      setLoadingTab(null)
    }
  }

  async function loadSemantics() {
    setLoadingTab(active)
    setLoadError(null)
    try {
      const semantics = await getSemantics(citationId)
      setTabData((current) => ({
        ...current,
        definitions: (semantics.definitions ?? []).map((item, index) => ({
          definition_id: `definition:${index}`,
          term: item.term,
          text: item.text,
          source_provision: item.source_provision,
          scope: item.scope || current.identity.citation,
        })),
        exceptions: (semantics.exceptions ?? []).map((item, index) => ({
          exception_id: `exception:${index}`,
          text: item.text,
          applies_to_provision: item.source_provision,
          source_provision: item.source_provision,
        })),
        deadlines: (semantics.deadlines ?? []).map((item, index) => ({
          deadline_id: `deadline:${index}`,
          description: item.description,
          duration: item.duration,
          trigger: item.trigger,
          source_provision: item.source_provision,
        })),
        penalties: (semantics.penalties ?? []).map((item, index) => ({
          penalty_id: `penalty:${index}`,
          description: item.text,
          category: "administrative",
          source_provision: item.source_provision,
        })),
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
      setTabData((current) => ({
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

  async function loadHistory() {
    setLoadingTab(active)
    setLoadError(null)
    try {
      const history = await getHistory(citationId)
      setTabData((current) => ({
        ...current,
        source_notes: history.source_notes ?? [],
        qc: {
          ...current.qc,
          status: history.source_notes?.length ? "warning" : current.qc.status,
          notes: (history.source_notes ?? []).map((message, index) => ({
            note_id: `source-note:${index}`,
            level: "info",
            category: "source",
            message,
            related_id: current.identity.canonical_id,
          })),
        },
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
            const count = getTabCount(tab.id, activeData)
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

      {(loadingTab || isPending || loadError) && (
        <div className="border-b border-border bg-muted/30 px-4 py-2 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
          {loadError ? `Could not load tab data: ${loadError}` : `Loading ${loadingTab ?? active} data`}
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-y-auto scrollbar-thin">
        <TabsContent value="text" className="m-0 h-full"><TextTab data={activeData} /></TabsContent>
        <TabsContent value="tree" className="m-0 h-full"><ProvisionTreeTab data={activeData} /></TabsContent>
        <TabsContent value="citations" className="m-0 h-full"><CitationsTab data={activeData} /></TabsContent>
        <TabsContent value="definitions" className="m-0 h-full"><DefinitionsTab data={activeData} /></TabsContent>
        <TabsContent value="deadlines" className="m-0 h-full"><DeadlinesTab data={activeData} /></TabsContent>
        <TabsContent value="exceptions" className="m-0 h-full"><ExceptionsTab data={activeData} /></TabsContent>
        <TabsContent value="chunks" className="m-0 h-full"><ChunksTab data={activeData} /></TabsContent>
        <TabsContent value="versions" className="m-0 h-full"><VersionsTab data={activeData} /></TabsContent>
        <TabsContent value="source" className="m-0 h-full"><SourceTab data={activeData} /></TabsContent>
        <TabsContent value="graph" className="m-0 h-full"><GraphTab data={activeData} /></TabsContent>
        <TabsContent value="qc" className="m-0 h-full"><QCTab data={activeData} /></TabsContent>
      </div>
    </Tabs>
  )
}

function TabLoading() {
  return <div className="px-6 py-8 text-sm text-muted-foreground">Loading...</div>
}

function getTabCount(id: TabId, data: StatutePageResponse): number | null {
  switch (id) {
    case "tree":
      return data.summary_counts?.provision_count ?? countProvisions(data.provisions)
    case "citations":
      return (data.summary_counts?.citation_counts.outbound ?? data.outbound_citations.length)
        + (data.summary_counts?.citation_counts.inbound ?? data.inbound_citations.length)
    case "definitions":
      return data.summary_counts?.semantic_counts.definitions ?? data.definitions.length
    case "deadlines":
      return data.summary_counts?.semantic_counts.deadlines ?? data.deadlines.length
    case "exceptions":
      return (data.summary_counts?.semantic_counts.exceptions ?? data.exceptions.length)
        + (data.summary_counts?.semantic_counts.penalties ?? data.penalties.length)
    case "chunks":
      return data.chunks.length
    case "versions":
      return data.versions.length
    case "qc":
      return data.qc.notes.length
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
