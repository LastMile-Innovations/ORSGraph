"use client"

import { useEffect, useMemo, useState } from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import {
  Search,
  Filter,
  Plus,
  FileText,
  Sparkles,
  CheckCircle2,
  Circle,
  AlertTriangle,
  Tag,
} from "lucide-react"
import type { Matter, ExtractedFact } from "@/lib/casebuilder/types"
import { matterClaimsHref, matterDocumentHref } from "@/lib/casebuilder/routes"
import { approveFact, createFact, patchFact } from "@/lib/casebuilder/api"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import { ScrollArea } from "@/components/ui/scroll-area"
import { ConfidenceBadge, FactStatusBadge } from "./badges"
import { cn } from "@/lib/utils"

interface FactsBoardProps {
  matter: Matter
}

type FilterMode = "all" | "disputed" | "supported" | "needs-review"

export function FactsBoard({ matter }: FactsBoardProps) {
  const [facts, setFacts] = useState(matter.facts)
  const [query, setQuery] = useState("")
  const [filter, setFilter] = useState<FilterMode>("all")
  const [selected, setSelected] = useState<string | null>(facts[0]?.id ?? null)
  const [message, setMessage] = useState<string | null>(null)
  const noisyFacts = useMemo(() => facts.filter((fact) => isLikelyExtractionNoise(fact.statement)), [facts])

  const filtered = useMemo(() => {
    return facts.filter((f) => {
      const matchesQuery =
        !query ||
        f.statement.toLowerCase().includes(query.toLowerCase()) ||
        f.tags.some((t) => t.toLowerCase().includes(query.toLowerCase()))
      const matchesFilter =
        filter === "all" ||
        (filter === "disputed" && f.disputed) ||
        (filter === "supported" && f.status === "supported") ||
        (filter === "needs-review" && (f.status === "proposed" || f.needs_verification || f.confidence < 0.7))
      return matchesQuery && matchesFilter
    })
  }, [facts, query, filter])

  const selectedFact = filtered.find((f) => f.id === selected) ?? filtered[0] ?? null
  const filterCounts = useMemo(() => {
    const matchesQuery = (fact: ExtractedFact) =>
      !query ||
      fact.statement.toLowerCase().includes(query.toLowerCase()) ||
      fact.tags.some((tag) => tag.toLowerCase().includes(query.toLowerCase()))
    const queryFacts = facts.filter(matchesQuery)
    return {
      all: queryFacts.length,
      supported: queryFacts.filter((f) => f.status === "supported").length,
      disputed: queryFacts.filter((f) => f.disputed).length,
      needsReview: queryFacts.filter((f) => f.status === "proposed" || f.needs_verification || f.confidence < 0.7).length,
    }
  }, [facts, query])

  useEffect(() => {
    if (filtered.length === 0) {
      if (selected !== null) setSelected(null)
      return
    }
    if (!filtered.some((fact) => fact.id === selected)) {
      setSelected(filtered[0].id)
    }
  }, [filtered, selected])

  async function addFact() {
    const statement = window.prompt("Fact statement")
    if (!statement) return
    const result = await createFact(matter.id, {
      statement,
      status: "proposed",
      confidence: 0.6,
      source_document_ids: [],
      source_evidence_ids: [],
    })
    if (result.data) {
      setFacts((current) => [result.data!, ...current])
      setSelected(result.data.id)
      setMessage("Fact added.")
    } else {
      setMessage(result.error || "Fact could not be added.")
    }
  }

  return (
    <div className="flex flex-col">
      {/* Header */}
      <div className="border-b border-border bg-background px-6 py-4">
        <div className="flex items-end justify-between gap-4">
          <div>
            <h1 className="text-xl font-semibold tracking-tight text-foreground">Facts</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              {facts.length} facts extracted from {matter.documents.length} documents
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" className="gap-1.5 bg-transparent" disabled title="Use the Documents page extraction action for live extraction.">
              <Sparkles className="h-3.5 w-3.5" />
              Auto-extract
            </Button>
            <Button size="sm" className="gap-1.5" onClick={addFact}>
              <Plus className="h-3.5 w-3.5" />
              Add fact
            </Button>
          </div>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <div className="relative max-w-sm flex-1">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="Search facts, tags, dates..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              className="h-8 pl-8 text-xs"
            />
          </div>
          <FilterPill active={filter === "all"} onClick={() => setFilter("all")}>
            All ({filterCounts.all})
          </FilterPill>
          <FilterPill active={filter === "supported"} onClick={() => setFilter("supported")}>
            Supported ({filterCounts.supported})
          </FilterPill>
          <FilterPill active={filter === "disputed"} onClick={() => setFilter("disputed")}>
            Disputed ({filterCounts.disputed})
          </FilterPill>
          <FilterPill
            active={filter === "needs-review"}
            onClick={() => setFilter("needs-review")}
          >
            Needs review ({filterCounts.needsReview})
          </FilterPill>
        </div>
        {noisyFacts.length > 0 && (
          <div className="mt-3 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
            <span className="font-medium text-foreground">{noisyFacts.length} extracted fact{noisyFacts.length === 1 ? "" : "s"} may be document headings or table fragments.</span>{" "}
            Review and edit them before using facts for claims, timeline, or drafts.
          </div>
        )}
        {message && <div className="mt-3 rounded border border-border bg-card px-3 py-2 text-xs text-muted-foreground">{message}</div>}
      </div>

      {/* Two-pane: list + detail */}
      <div className="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_420px]">
        <div className="border-r border-border">
          <ScrollArea className="h-[calc(100vh-200px)]">
            <ul className="divide-y divide-border">
              {filtered.map((fact) => (
                <FactRow
                  key={fact.id}
                  fact={fact}
                  active={selectedFact?.id === fact.id}
                  onClick={() => setSelected(fact.id)}
                />
              ))}
              {filtered.length === 0 && (
                <li className="flex flex-col items-center gap-2 px-6 py-16 text-center">
                  <Filter className="h-8 w-8 text-muted-foreground" />
                  <p className="text-sm font-medium text-foreground">No facts match</p>
                  <p className="text-xs text-muted-foreground">
                    Try a different filter or search term.
                  </p>
                </li>
              )}
            </ul>
          </ScrollArea>
        </div>

        <aside className="bg-card">
          {selectedFact ? (
            <FactDetail key={selectedFact.id} fact={selectedFact} matter={matter} />
          ) : (
            <div className="flex h-full items-center justify-center p-8 text-center">
              <p className="text-sm text-muted-foreground">Select a fact to inspect</p>
            </div>
          )}
        </aside>
      </div>
    </div>
  )
}

function FilterPill({
  active,
  onClick,
  children,
}: {
  active: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "rounded-full border px-2.5 py-1 text-[11px] font-medium transition-colors",
        active
          ? "border-foreground bg-foreground text-background"
          : "border-border bg-background text-muted-foreground hover:bg-muted",
      )}
    >
      {children}
    </button>
  )
}

function FactRow({
  fact,
  active,
  onClick,
}: {
  fact: ExtractedFact
  active: boolean
  onClick: () => void
}) {
  return (
    <li
      id={fact.id}
      onClick={onClick}
      className={cn(
        "cursor-pointer px-6 py-3 transition-colors",
        active ? "bg-muted/60" : "hover:bg-muted/30",
      )}
    >
      <div className="flex items-start gap-3">
        <FactStatusIcon fact={fact} />
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium leading-snug text-foreground text-pretty">
            {fact.statement}
          </p>
          <div className="mt-1.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px] text-muted-foreground">
            {fact.date && <span className="font-mono">{fact.date}</span>}
            {fact.date && <span>·</span>}
            <span>
              {fact.sourceDocumentIds.length} source
              {fact.sourceDocumentIds.length === 1 ? "" : "s"}
            </span>
            {fact.tags.slice(0, 3).map((t) => (
              <Badge key={t} variant="secondary" className="text-[9px] font-normal">
                {t}
              </Badge>
            ))}
            {isLikelyExtractionNoise(fact.statement) && (
              <Badge variant="outline" className="border-warning/40 text-[9px] font-normal text-warning">
                format review
              </Badge>
            )}
          </div>
        </div>
        <ConfidenceBadge value={fact.confidence} size="sm" />
      </div>
    </li>
  )
}

function FactStatusIcon({ fact }: { fact: ExtractedFact }) {
  if (fact.disputed) {
    return <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-warning" />
  }
  if (fact.confidence >= 0.85) {
    return <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-success" />
  }
  return <Circle className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
}

function FactDetail({ fact, matter }: { fact: ExtractedFact; matter: Matter }) {
  const router = useRouter()
  const sources = matter.documents.filter((d) => fact.sourceDocumentIds.includes(d.id))
  const linkedClaims = matter.claims.filter((c) => c.supportingFactIds.includes(fact.id))
  const [editText, setEditText] = useState(fact.statement)
  const [saving, setSaving] = useState(false)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  async function runMutation(action: () => Promise<{ data: ExtractedFact | null; error?: string }>, success: string) {
    setSaving(true)
    setMessage(null)
    setError(null)
    const result = await action()
    setSaving(false)
    if (!result.data) {
      setError(result.error || "Fact update failed.")
      return
    }
    setMessage(success)
    router.refresh()
  }

  return (
    <ScrollArea className="h-[calc(100vh-200px)]">
      <div className="space-y-5 p-5">
        <div>
          <div className="flex items-center gap-2">
            <FactStatusIcon fact={fact} />
            <p className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              {fact.id}
            </p>
          </div>
          <p className="mt-2 text-base font-semibold leading-snug text-foreground text-pretty">
            {fact.statement}
          </p>
          <div className="mt-3 flex items-center gap-2">
            <FactStatusBadge status={fact.status} />
            <ConfidenceBadge value={fact.confidence} />
            {fact.disputed && (
              <Badge variant="outline" className="gap-1 border-warning/40 text-warning">
                <AlertTriangle className="h-3 w-3" />
                Disputed
              </Badge>
            )}
          </div>
        </div>

        <div className="space-y-2 rounded-md border border-border bg-background p-3">
          <label className="block font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            Review statement
          </label>
          <textarea
            value={editText}
            onChange={(event) => setEditText(event.target.value)}
            rows={4}
            className="w-full rounded border border-border bg-card px-3 py-2 text-sm leading-relaxed text-foreground focus:border-primary focus:outline-none"
          />
          <div className="flex flex-wrap items-center gap-2">
            <Button
              size="sm"
              disabled={saving}
              onClick={() => runMutation(() => approveFact(matter.id, fact.id), "Fact approved.")}
            >
              Approve
            </Button>
            <Button
              size="sm"
              variant="outline"
              disabled={saving || editText.trim() === fact.statement}
              onClick={() =>
                runMutation(
                  () => patchFact(matter.id, fact.id, { statement: editText.trim() }),
                  "Fact statement saved.",
                )
              }
            >
              Save edit
            </Button>
            <Button
              size="sm"
              variant="outline"
              disabled={saving}
              onClick={() =>
                runMutation(
                  () => patchFact(matter.id, fact.id, { status: "disputed" }),
                  "Fact marked disputed.",
                )
              }
            >
              Mark disputed
            </Button>
            <Button
              size="sm"
              variant="outline"
              disabled={saving}
              onClick={() =>
                runMutation(
                  () => patchFact(matter.id, fact.id, { status: "rejected" }),
                  "Fact rejected.",
                )
              }
            >
              Reject
            </Button>
          </div>
          {(message || error) && (
            <p className={cn("text-xs", error ? "text-destructive" : "text-muted-foreground")}>
              {error || message}
            </p>
          )}
        </div>

        <DetailRow label="Date">
          {fact.date ? <span className="font-mono">{fact.date}</span> : "—"}
        </DetailRow>
        <DetailRow label="Tags">
          <div className="flex flex-wrap gap-1">
            {fact.tags.map((t) => (
              <Badge key={t} variant="secondary" className="text-[10px]">
                <Tag className="mr-1 h-2.5 w-2.5" />
                {t}
              </Badge>
            ))}
          </div>
        </DetailRow>

        <div>
          <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Sources
          </h3>
          <ul className="mt-2 space-y-1.5">
            {sources.map((doc) => {
              const citation = fact.citations.find((c) => c.documentId === doc.id)
              return (
                <li key={doc.id}>
                  <Link
                    href={matterDocumentHref(matter.id, doc.id, citation?.chunkId)}
                    className="flex items-start gap-2 rounded-md border border-border bg-background p-2.5 text-xs transition-colors hover:border-foreground/20 hover:bg-muted/40"
                  >
                    <FileText className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                    <div className="min-w-0 flex-1">
                      <p className="truncate font-medium text-foreground">{doc.title}</p>
                      <p className="mt-0.5 text-[10px] text-muted-foreground">
                        {citation?.snippet ?? doc.summary}
                      </p>
                      {citation && (
                        <p className="mt-1 font-mono text-[10px] text-muted-foreground">
                          {citation.chunkId} · p.{citation.page}
                        </p>
                      )}
                    </div>
                  </Link>
                </li>
              )
            })}
          </ul>
        </div>

        {fact.source_spans && fact.source_spans.length > 0 && (
          <div>
            <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Source spans
            </h3>
            <ul className="mt-2 space-y-1.5">
              {fact.source_spans.map((span) => (
                <li key={span.source_span_id} className="rounded-md border border-border bg-background p-2.5 text-xs">
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-mono text-[10px] text-muted-foreground">
                      {span.source_span_id}
                    </span>
                    <span className="font-mono text-[10px] text-muted-foreground">
                      {span.page ? `p.${span.page}` : "no page"}
                    </span>
                  </div>
                  {span.quote && (
                    <p className="mt-1 line-clamp-4 leading-relaxed text-foreground">
                      “{span.quote}”
                    </p>
                  )}
                  <p className="mt-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                    {span.extraction_method} · {Math.round(span.confidence * 100)}%
                  </p>
                </li>
              ))}
            </ul>
          </div>
        )}

        {linkedClaims.length > 0 && (
          <div>
            <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Used in claims
            </h3>
            <ul className="mt-2 space-y-1">
              {linkedClaims.map((claim) => (
                <li key={claim.id}>
                  <Link
                    href={matterClaimsHref(matter.id, claim.id)}
                    className="block rounded-md border border-border bg-background p-2 text-xs hover:border-foreground/20 hover:bg-muted/40"
                  >
                    <Badge variant="outline" className="text-[9px] capitalize">
                      {claim.kind}
                    </Badge>
                    <p className="mt-1 font-medium text-foreground">{claim.title}</p>
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        )}

        <Card className="border-border/60 bg-muted/20 p-3">
          <div className="flex items-start gap-2">
            <Sparkles className="mt-0.5 h-3.5 w-3.5 shrink-0 text-foreground" />
            <div className="text-xs">
              <p className="font-medium text-foreground">{isLikelyExtractionNoise(fact.statement) ? "Extraction review" : "AI suggestion"}</p>
              <p className="mt-1 leading-relaxed text-muted-foreground">
                {isLikelyExtractionNoise(fact.statement)
                  ? "This looks like a heading, markdown, or table fragment. Rewrite it into a complete fact sentence or reject it before using it downstream."
                  : (
                      <>
                        Cross-reference with <span className="font-mono">{fact.sourceDocumentIds[0] ?? "DOC-001"}</span> to confirm timing. Consider adding a deposition transcript to strengthen.
                      </>
                    )}
              </p>
            </div>
          </div>
        </Card>
      </div>
    </ScrollArea>
  )
}

function isLikelyExtractionNoise(statement: string) {
  const trimmed = statement.trim()
  return (
    trimmed.startsWith("#") ||
    trimmed.startsWith("|") ||
    trimmed.includes("` |") ||
    trimmed.includes("| ---") ||
    /^[A-Z\s#]{18,}$/.test(trimmed)
  )
}

function DetailRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-3 border-b border-border pb-2 text-xs">
      <span className="font-medium text-muted-foreground">{label}</span>
      <div className="text-right text-foreground">{children}</div>
    </div>
  )
}
