"use client"

import { useMemo, useState } from "react"
import Link from "next/link"
import {
  ArrowLeft,
  FileText,
  Sparkles,
  Tag,
  Calendar,
  User,
  MapPin,
  DollarSign,
  Quote,
  AlertTriangle,
  CheckCircle2,
  Eye,
  EyeOff,
  Download,
  MessageSquare,
  Link2,
  FileDigit,
} from "lucide-react"
import type {
  Matter,
  MatterDocument,
  ExtractedEntity,
  ExtractedFact,
  DocumentClause,
  DocumentChunk,
} from "@/lib/casebuilder/types"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Separator } from "@/components/ui/separator"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { ConfidenceBadge, MatterStatusBadge } from "./badges"
import { cn } from "@/lib/utils"

const ENTITY_COLORS: Record<ExtractedEntity["type"], string> = {
  person: "bg-blue-500/15 text-blue-700 dark:text-blue-300 border-blue-500/30",
  org: "bg-purple-500/15 text-purple-700 dark:text-purple-300 border-purple-500/30",
  date: "bg-amber-500/15 text-amber-700 dark:text-amber-300 border-amber-500/30",
  money: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border-emerald-500/30",
  location: "bg-rose-500/15 text-rose-700 dark:text-rose-300 border-rose-500/30",
  legalCitation: "bg-indigo-500/15 text-indigo-700 dark:text-indigo-300 border-indigo-500/30",
  obligation: "bg-orange-500/15 text-orange-700 dark:text-orange-300 border-orange-500/30",
  party: "bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border-cyan-500/30",
}

const ENTITY_ICONS: Record<ExtractedEntity["type"], typeof User> = {
  person: User,
  org: User,
  date: Calendar,
  money: DollarSign,
  location: MapPin,
  legalCitation: Quote,
  obligation: AlertTriangle,
  party: User,
}

interface DocumentViewerProps {
  matter: Matter
  document: MatterDocument
}

type InspectorTab = "extractions" | "clauses" | "facts" | "chunks" | "issues"

export function DocumentViewer({ matter, document }: DocumentViewerProps) {
  const [showHighlights, setShowHighlights] = useState(true)
  const [activeTab, setActiveTab] = useState<InspectorTab>("extractions")
  const [selectedEntity, setSelectedEntity] = useState<string | null>(null)
  const [hoveredEntity, setHoveredEntity] = useState<string | null>(null)

  const entityById = useMemo(
    () => new Map(document.entities.map((e) => [e.id, e])),
    [document.entities],
  )

  const renderedChunks = useMemo(() => {
    return document.chunks.map((chunk) => ({
      chunk,
      segments: buildHighlightedSegments(chunk, document.entities, document.clauses),
    }))
  }, [document.chunks, document.entities, document.clauses])

  return (
    <TooltipProvider delayDuration={200}>
      <div className="flex flex-col">
        {/* Document Header */}
        <div className="border-b border-border bg-card px-6 py-4">
          <div className="flex items-center gap-3 text-xs text-muted-foreground">
            <Link
              href={`/matters/${matter.id}/documents`}
              className="flex items-center gap-1 hover:text-foreground"
            >
              <ArrowLeft className="h-3.5 w-3.5" />
              Documents
            </Link>
          </div>
          <div className="mt-2 flex items-start justify-between gap-4">
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <FileText className="h-5 w-5 shrink-0 text-muted-foreground" />
                <h1 className="truncate text-xl font-semibold text-foreground text-balance">
                  {document.title}
                </h1>
              </div>
              <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <Badge variant="outline" className="font-mono text-[10px]">
                  {document.kind}
                </Badge>
                <MatterStatusBadge status={document.status} />
                <span>·</span>
                <span>{document.pageCount} pages</span>
                <span>·</span>
                <span>{document.fileSize}</span>
                <span>·</span>
                <span>Filed {document.dateFiled ?? "—"}</span>
                <span>·</span>
                <span>From: {document.party}</span>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowHighlights((v) => !v)}
                className="gap-1.5"
              >
                {showHighlights ? <Eye className="h-3.5 w-3.5" /> : <EyeOff className="h-3.5 w-3.5" />}
                Highlights
              </Button>
              <Button variant="outline" size="sm" className="gap-1.5 bg-transparent">
                <Download className="h-3.5 w-3.5" />
                Export
              </Button>
              <Button size="sm" className="gap-1.5">
                <Sparkles className="h-3.5 w-3.5" />
                Re-extract
              </Button>
            </div>
          </div>
        </div>

        {/* Two-pane layout */}
        <div className="grid grid-cols-1 gap-0 lg:grid-cols-[minmax(0,1fr)_420px]">
          {/* Document text pane */}
          <div className="border-r border-border bg-background">
            <ScrollArea className="h-[calc(100vh-180px)]">
              <article className="mx-auto max-w-3xl px-8 py-10">
                <header className="mb-8 border-b border-border pb-6">
                  <p className="font-mono text-xs uppercase tracking-wider text-muted-foreground">
                    {document.party} · {document.dateFiled ?? document.dateUploaded}
                  </p>
                  <h2 className="mt-2 text-2xl font-semibold tracking-tight text-foreground text-pretty">
                    {document.title}
                  </h2>
                  {document.summary && (
                    <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
                      {document.summary}
                    </p>
                  )}
                </header>

                <div className="space-y-6 font-serif text-[15px] leading-7 text-foreground">
                  {renderedChunks.map(({ chunk, segments }) => (
                    <ChunkBlock
                      key={chunk.id}
                      chunk={chunk}
                      segments={segments}
                      showHighlights={showHighlights}
                      selectedEntity={selectedEntity}
                      hoveredEntity={hoveredEntity}
                      onEntityClick={(id) => {
                        setSelectedEntity(id)
                        setActiveTab("extractions")
                      }}
                      onEntityHover={setHoveredEntity}
                      entityById={entityById}
                    />
                  ))}
                </div>

                <footer className="mt-10 flex items-center justify-between border-t border-border pt-6 text-xs text-muted-foreground">
                  <span>Page {document.pageCount} of {document.pageCount}</span>
                  <span className="font-mono">{document.id}</span>
                </footer>
              </article>
            </ScrollArea>
          </div>

          {/* AI Inspector */}
          <aside className="bg-card">
            <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as InspectorTab)}>
              <div className="border-b border-border px-3 pt-3">
                <TabsList className="grid w-full grid-cols-5 bg-muted/40">
                  <TabsTrigger value="extractions" className="text-[11px]">
                    Entities
                  </TabsTrigger>
                  <TabsTrigger value="clauses" className="text-[11px]">
                    Clauses
                  </TabsTrigger>
                  <TabsTrigger value="facts" className="text-[11px]">
                    Facts
                  </TabsTrigger>
                  <TabsTrigger value="chunks" className="text-[11px]">
                    Chunks
                  </TabsTrigger>
                  <TabsTrigger value="issues" className="text-[11px]">
                    Issues
                  </TabsTrigger>
                </TabsList>
              </div>

              <ScrollArea className="h-[calc(100vh-232px)]">
                <TabsContent value="extractions" className="m-0 p-4">
                  <ExtractionsPanel
                    entities={document.entities}
                    selectedEntity={selectedEntity}
                    onSelect={setSelectedEntity}
                    onHover={setHoveredEntity}
                  />
                </TabsContent>
                <TabsContent value="clauses" className="m-0 p-4">
                  <ClausesPanel clauses={document.clauses} />
                </TabsContent>
                <TabsContent value="facts" className="m-0 p-4">
                  <FactsPanel facts={document.linkedFacts} matter={matter} />
                </TabsContent>
                <TabsContent value="chunks" className="m-0 p-4">
                  <ChunksPanel chunks={document.chunks} />
                </TabsContent>
                <TabsContent value="issues" className="m-0 p-4">
                  <IssuesPanel document={document} />
                </TabsContent>
              </ScrollArea>
            </Tabs>
          </aside>
        </div>
      </div>
    </TooltipProvider>
  )
}

/* -------------------------------------------------------------------------- */
/*                              Chunk + segments                              */
/* -------------------------------------------------------------------------- */

type Segment =
  | { kind: "text"; text: string }
  | { kind: "entity"; text: string; entity: ExtractedEntity }
  | { kind: "clause"; text: string; clause: DocumentClause }

function buildHighlightedSegments(
  chunk: DocumentChunk,
  entities: ExtractedEntity[],
  clauses: DocumentClause[],
): Segment[] {
  // Collect spans that fall within this chunk.
  type Span = { start: number; end: number; kind: "entity" | "clause"; ref: any }
  const spans: Span[] = []

  for (const entity of entities) {
    for (const span of entity.spans) {
      if (span.chunkId !== chunk.id) continue
      spans.push({ start: span.start, end: span.end, kind: "entity", ref: entity })
    }
  }
  for (const clause of clauses) {
    if (clause.chunkId !== chunk.id) continue
    spans.push({ start: clause.start, end: clause.end, kind: "clause", ref: clause })
  }

  spans.sort((a, b) => a.start - b.start || b.end - b.start - (a.end - a.start))

  const segments: Segment[] = []
  let cursor = 0
  for (const span of spans) {
    if (span.start < cursor) continue // skip overlaps
    if (span.start > cursor) {
      segments.push({ kind: "text", text: chunk.text.slice(cursor, span.start) })
    }
    const text = chunk.text.slice(span.start, span.end)
    if (span.kind === "entity") {
      segments.push({ kind: "entity", text, entity: span.ref })
    } else {
      segments.push({ kind: "clause", text, clause: span.ref })
    }
    cursor = span.end
  }
  if (cursor < chunk.text.length) {
    segments.push({ kind: "text", text: chunk.text.slice(cursor) })
  }
  return segments
}

interface ChunkBlockProps {
  chunk: DocumentChunk
  segments: Segment[]
  showHighlights: boolean
  selectedEntity: string | null
  hoveredEntity: string | null
  onEntityClick: (id: string) => void
  onEntityHover: (id: string | null) => void
  entityById: Map<string, ExtractedEntity>
}

function ChunkBlock({
  chunk,
  segments,
  showHighlights,
  selectedEntity,
  hoveredEntity,
  onEntityClick,
  onEntityHover,
}: ChunkBlockProps) {
  return (
    <div id={chunk.id} className="group relative">
      {chunk.heading && (
        <h3 className="mb-2 font-sans text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          {chunk.heading}
        </h3>
      )}
      <p>
        {segments.map((seg, i) => {
          if (seg.kind === "text") return <span key={i}>{seg.text}</span>
          if (seg.kind === "entity") {
            const isActive =
              selectedEntity === seg.entity.id || hoveredEntity === seg.entity.id
            return (
              <Tooltip key={i}>
                <TooltipTrigger asChild>
                  <span
                    className={cn(
                      "cursor-pointer rounded-sm border px-0.5 transition-colors",
                      showHighlights ? ENTITY_COLORS[seg.entity.type] : "border-transparent",
                      isActive && "ring-2 ring-ring ring-offset-1 ring-offset-background",
                      !showHighlights && "hover:bg-muted",
                    )}
                    onClick={() => onEntityClick(seg.entity.id)}
                    onMouseEnter={() => onEntityHover(seg.entity.id)}
                    onMouseLeave={() => onEntityHover(null)}
                  >
                    {seg.text}
                  </span>
                </TooltipTrigger>
                <TooltipContent side="top" className="max-w-xs">
                  <div className="space-y-1">
                    <p className="font-mono text-[10px] uppercase tracking-wider opacity-70">
                      {seg.entity.type}
                    </p>
                    <p className="text-sm font-medium">{seg.entity.value}</p>
                    {seg.entity.normalized && (
                      <p className="text-xs opacity-80">{seg.entity.normalized}</p>
                    )}
                    <p className="text-[10px] opacity-60">
                      Confidence: {Math.round(seg.entity.confidence * 100)}%
                    </p>
                  </div>
                </TooltipContent>
              </Tooltip>
            )
          }
          // clause
          return (
            <span
              key={i}
              className={cn(
                "rounded-sm transition-colors",
                showHighlights && "bg-primary/10 underline decoration-primary/40 decoration-dotted underline-offset-4",
              )}
              title={seg.clause.label}
            >
              {seg.text}
            </span>
          )
        })}
      </p>
    </div>
  )
}

/* -------------------------------------------------------------------------- */
/*                                  Panels                                    */
/* -------------------------------------------------------------------------- */

function ExtractionsPanel({
  entities,
  selectedEntity,
  onSelect,
  onHover,
}: {
  entities: ExtractedEntity[]
  selectedEntity: string | null
  onSelect: (id: string) => void
  onHover: (id: string | null) => void
}) {
  const grouped = useMemo(() => {
    const map = new Map<ExtractedEntity["type"], ExtractedEntity[]>()
    for (const e of entities) {
      if (!map.has(e.type)) map.set(e.type, [])
      map.get(e.type)!.push(e)
    }
    return Array.from(map.entries())
  }, [entities])

  if (entities.length === 0) {
    return <EmptyPanel icon={Tag} title="No entities extracted" />
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-xs font-medium text-muted-foreground">
          {entities.length} entities · {grouped.length} types
        </p>
        <Button variant="ghost" size="sm" className="h-7 gap-1 text-[11px]">
          <Sparkles className="h-3 w-3" />
          Re-run
        </Button>
      </div>
      {grouped.map(([type, items]) => {
        const Icon = ENTITY_ICONS[type]
        return (
          <div key={type} className="space-y-1.5">
            <div className="flex items-center gap-2">
              <Icon className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="text-xs font-semibold capitalize text-foreground">
                {type === "legalCitation" ? "Citations" : type}
              </span>
              <span className="text-[10px] text-muted-foreground">{items.length}</span>
            </div>
            <ul className="space-y-1">
              {items.map((entity) => {
                const isSelected = selectedEntity === entity.id
                return (
                  <li key={entity.id}>
                    <button
                      onClick={() => onSelect(entity.id)}
                      onMouseEnter={() => onHover(entity.id)}
                      onMouseLeave={() => onHover(null)}
                      className={cn(
                        "flex w-full items-start justify-between gap-2 rounded-md border border-transparent px-2 py-1.5 text-left text-xs transition-colors",
                        "hover:border-border hover:bg-muted/40",
                        isSelected && "border-border bg-muted",
                      )}
                    >
                      <div className="min-w-0 flex-1">
                        <p className="truncate font-medium text-foreground">{entity.value}</p>
                        {entity.normalized && entity.normalized !== entity.value && (
                          <p className="truncate text-[10px] text-muted-foreground">
                            → {entity.normalized}
                          </p>
                        )}
                      </div>
                      <ConfidenceBadge value={entity.confidence} size="sm" />
                    </button>
                  </li>
                )
              })}
            </ul>
          </div>
        )
      })}
    </div>
  )
}

function ClausesPanel({ clauses }: { clauses: DocumentClause[] }) {
  if (clauses.length === 0) {
    return <EmptyPanel icon={FileDigit} title="No clauses identified" />
  }
  return (
    <div className="space-y-3">
      <p className="text-xs font-medium text-muted-foreground">
        {clauses.length} clauses identified
      </p>
      {clauses.map((clause) => (
        <Card key={clause.id} className="border-border/60">
          <CardHeader className="px-3 py-2.5">
            <div className="flex items-start justify-between gap-2">
              <div>
                <Badge variant="outline" className="font-mono text-[10px] capitalize">
                  {clause.type}
                </Badge>
                <CardTitle className="mt-1.5 text-sm font-semibold leading-tight">
                  {clause.label}
                </CardTitle>
              </div>
              <ConfidenceBadge value={clause.confidence} size="sm" />
            </div>
          </CardHeader>
          <CardContent className="px-3 pb-3 pt-0">
            <p className="text-xs leading-relaxed text-muted-foreground">{clause.summary}</p>
            {clause.linkedProvisionIds && clause.linkedProvisionIds.length > 0 && (
              <div className="mt-2 flex flex-wrap gap-1">
                {clause.linkedProvisionIds.map((pid) => (
                  <Link
                    key={pid}
                    href={`/provisions/${pid}`}
                    className="inline-flex items-center gap-1 rounded border border-border bg-muted/40 px-1.5 py-0.5 font-mono text-[10px] text-foreground hover:bg-muted"
                  >
                    <Link2 className="h-2.5 w-2.5" />
                    {pid}
                  </Link>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

function FactsPanel({
  facts,
  matter,
}: {
  facts: ExtractedFact[]
  matter: Matter
}) {
  if (facts.length === 0) {
    return <EmptyPanel icon={CheckCircle2} title="No facts linked" />
  }
  return (
    <div className="space-y-2">
      <p className="text-xs font-medium text-muted-foreground">
        {facts.length} facts derived from this document
      </p>
      {facts.map((fact) => (
        <Link
          key={fact.id}
          href={`/matters/${matter.id}/facts#${fact.id}`}
          className="block rounded-md border border-border bg-background p-3 text-xs transition-colors hover:border-foreground/20 hover:bg-muted/40"
        >
          <div className="flex items-start justify-between gap-2">
            <p className="font-medium leading-tight text-foreground">{fact.statement}</p>
            <ConfidenceBadge value={fact.confidence} size="sm" />
          </div>
          {fact.date && (
            <p className="mt-1 font-mono text-[10px] text-muted-foreground">{fact.date}</p>
          )}
          <div className="mt-2 flex flex-wrap gap-1">
            {fact.tags.map((t) => (
              <Badge key={t} variant="secondary" className="text-[9px]">
                {t}
              </Badge>
            ))}
          </div>
        </Link>
      ))}
    </div>
  )
}

function ChunksPanel({ chunks }: { chunks: DocumentChunk[] }) {
  return (
    <div className="space-y-2">
      <p className="text-xs font-medium text-muted-foreground">
        {chunks.length} indexed chunks
      </p>
      {chunks.map((chunk) => (
        <a
          key={chunk.id}
          href={`#${chunk.id}`}
          className="block rounded-md border border-border bg-background p-2.5 text-xs transition-colors hover:border-foreground/20 hover:bg-muted/40"
        >
          <div className="flex items-center justify-between">
            <span className="font-mono text-[10px] text-muted-foreground">
              {chunk.id} · p.{chunk.page}
            </span>
            <span className="text-[10px] text-muted-foreground">{chunk.tokens} tok</span>
          </div>
          {chunk.heading && (
            <p className="mt-1 font-medium text-foreground">{chunk.heading}</p>
          )}
          <p className="mt-1 line-clamp-2 leading-relaxed text-muted-foreground">{chunk.text}</p>
        </a>
      ))}
    </div>
  )
}

function IssuesPanel({ document }: { document: MatterDocument }) {
  const issues = document.issues ?? []
  if (issues.length === 0) {
    return (
      <EmptyPanel
        icon={CheckCircle2}
        title="No issues detected"
        description="Extraction passed all quality checks."
      />
    )
  }
  return (
    <div className="space-y-2">
      <p className="text-xs font-medium text-muted-foreground">
        {issues.length} issue{issues.length === 1 ? "" : "s"} need review
      </p>
      {issues.map((issue) => (
        <div
          key={issue.id}
          className="rounded-md border border-amber-500/30 bg-amber-500/5 p-3 text-xs"
        >
          <div className="flex items-start gap-2">
            <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-600 dark:text-amber-400" />
            <div className="min-w-0 flex-1">
              <p className="font-medium text-foreground">{issue.title}</p>
              <p className="mt-1 leading-relaxed text-muted-foreground">{issue.detail}</p>
              <div className="mt-2 flex items-center gap-2">
                <Button size="sm" variant="outline" className="h-6 gap-1 bg-transparent text-[10px]">
                  <MessageSquare className="h-3 w-3" />
                  Resolve
                </Button>
                <span className="font-mono text-[10px] capitalize text-muted-foreground">
                  {issue.severity}
                </span>
              </div>
            </div>
          </div>
        </div>
      ))}
    </div>
  )
}

function EmptyPanel({
  icon: Icon,
  title,
  description,
}: {
  icon: typeof Tag
  title: string
  description?: string
}) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 px-4 py-12 text-center">
      <div className="flex h-10 w-10 items-center justify-center rounded-full bg-muted">
        <Icon className="h-5 w-5 text-muted-foreground" />
      </div>
      <p className="text-sm font-medium text-foreground">{title}</p>
      {description && <p className="text-xs text-muted-foreground">{description}</p>}
    </div>
  )
}

/* eslint-disable-next-line @typescript-eslint/no-unused-vars */
const _separator = Separator
