"use client"

import { useMemo, useState } from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import {
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Sparkles,
  Download,
  FileText,
  Filter,
  Plus,
} from "lucide-react"
import type { Matter, Claim, ClaimElement, ExtractedFact } from "@/lib/casebuilder/types"
import { matterClaimsHref, matterFactsHref } from "@/lib/casebuilder/routes"
import { createEvidence } from "@/lib/casebuilder/api"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Card } from "@/components/ui/card"
import { ConfidenceBadge } from "./badges"
import { cn } from "@/lib/utils"

interface EvidenceMatrixProps {
  matter: Matter
}

type CellState = "supported" | "weak" | "missing" | "rebutted"
type CellEntry = {
  claim: Claim
  element: ClaimElement
  facts: ExtractedFact[]
  state: CellState
}

export function EvidenceMatrix({ matter }: EvidenceMatrixProps) {
  const [selected, setSelected] = useState<CellEntry | null>(null)
  const [showDefenses, setShowDefenses] = useState(true)

  const claims = useMemo(
    () => matter.claims.filter((c) => (showDefenses ? true : c.kind !== "defense")),
    [matter.claims, showDefenses],
  )

  const cells = useMemo(() => {
    const grid: CellEntry[][] = []
    for (const claim of claims) {
      const row: CellEntry[] = []
      for (const element of claim.elements) {
        const facts = matter.facts.filter((f) => element.supportingFactIds.includes(f.id))
        let state: CellState = "missing"
        if (element.status === "rebutted") state = "rebutted"
        else if (facts.length === 0) state = "missing"
        else if (facts.some((f) => f.disputed)) state = "weak"
        else if (facts.every((f) => f.confidence >= 0.8)) state = "supported"
        else state = "weak"
        row.push({ claim, element, facts, state })
      }
      grid.push(row)
    }
    return grid
  }, [claims, matter.facts])

  const stats = useMemo(() => {
    let supported = 0
    let weak = 0
    let missing = 0
    let rebutted = 0
    for (const row of cells) {
      for (const cell of row) {
        if (cell.state === "supported") supported++
        else if (cell.state === "weak") weak++
        else if (cell.state === "missing") missing++
        else rebutted++
      }
    }
    return { supported, weak, missing, rebutted, total: supported + weak + missing + rebutted }
  }, [cells])

  return (
    <div className="flex flex-col">
      {/* Header */}
      <div className="border-b border-border bg-background px-6 py-4">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <h1 className="text-xl font-semibold tracking-tight text-foreground">
              Evidence Matrix
            </h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Map facts to claim elements. Identify gaps before opposing counsel does.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" className="gap-1.5 bg-transparent">
              <Sparkles className="h-3.5 w-3.5" />
              Suggest gaps
            </Button>
            <Button variant="outline" size="sm" className="gap-1.5 bg-transparent">
              <Download className="h-3.5 w-3.5" />
              Export grid
            </Button>
          </div>
        </div>

        <div className="mt-4 grid grid-cols-2 gap-2 md:grid-cols-4">
          <StatCard label="Supported" count={stats.supported} total={stats.total} state="supported" />
          <StatCard label="Weak" count={stats.weak} total={stats.total} state="weak" />
          <StatCard label="Missing" count={stats.missing} total={stats.total} state="missing" />
          <StatCard label="Rebutted" count={stats.rebutted} total={stats.total} state="rebutted" />
        </div>

        <div className="mt-3 flex items-center gap-2 text-xs">
          <Filter className="h-3 w-3 text-muted-foreground" />
          <button
            onClick={() => setShowDefenses((v) => !v)}
            className="rounded-full border border-border bg-background px-2.5 py-1 text-[11px] font-medium text-muted-foreground hover:bg-muted"
          >
            {showDefenses ? "Hide" : "Show"} defenses
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_380px]">
        {/* Matrix */}
        <div className="border-r border-border">
          <ScrollArea className="h-[calc(100vh-280px)]">
            <div className="p-6">
              <div className="space-y-6">
                {cells.map((row, rowIdx) => {
                  const claim = row[0]?.claim
                  if (!claim) return null
                  return (
                    <Card key={claim.id} className="overflow-hidden border-border/60">
                      <div className="border-b border-border bg-muted/30 px-4 py-2.5">
                        <div className="flex items-start justify-between gap-3">
                          <div>
                            <Badge variant="outline" className="text-[9px] uppercase">
                              {claim.kind}
                            </Badge>
                            <h3 className="mt-1 text-sm font-semibold text-foreground">
                              {claim.title}
                            </h3>
                            <p className="mt-0.5 text-xs text-muted-foreground">{claim.cause}</p>
                          </div>
                          <Link
                            href={matterClaimsHref(matter.id, claim.id)}
                            className="text-[11px] text-muted-foreground hover:text-foreground hover:underline"
                          >
                            View claim →
                          </Link>
                        </div>
                      </div>

                      <div className="divide-y divide-border">
                        {row.map((cell, cellIdx) => {
                          const isSelected =
                            selected?.claim.id === cell.claim.id &&
                            selected?.element.id === cell.element.id
                          return (
                            <button
                              key={cell.element.id}
                              onClick={() => setSelected(cell)}
                              className={cn(
                                "flex w-full items-start gap-3 px-4 py-3 text-left transition-colors",
                                isSelected ? "bg-muted/60" : "hover:bg-muted/30",
                              )}
                            >
                              <CellStateIcon state={cell.state} />
                              <div className="min-w-0 flex-1">
                                <div className="flex items-baseline justify-between gap-2">
                                  <p className="text-sm font-medium text-foreground">
                                    <span className="mr-2 font-mono text-[10px] text-muted-foreground">
                                      {rowIdx + 1}.{cellIdx + 1}
                                    </span>
                                    {cell.element.title}
                                  </p>
                                  <span className="font-mono text-[10px] text-muted-foreground">
                                    {cell.facts.length} fact{cell.facts.length === 1 ? "" : "s"}
                                  </span>
                                </div>
                                <p className="mt-0.5 line-clamp-2 text-xs leading-relaxed text-muted-foreground">
                                  {cell.element.description}
                                </p>
                              </div>
                            </button>
                          )
                        })}
                      </div>
                    </Card>
                  )
                })}
              </div>
            </div>
          </ScrollArea>
        </div>

        {/* Detail panel */}
        <aside className="bg-card">
          {selected ? (
            <CellDetail entry={selected} matter={matter} />
          ) : (
            <div className="flex h-full flex-col items-center justify-center gap-2 p-8 text-center">
              <FileText className="h-8 w-8 text-muted-foreground" />
              <p className="text-sm font-medium text-foreground">Select an element</p>
              <p className="text-xs text-muted-foreground">
                Click any cell to inspect supporting facts.
              </p>
            </div>
          )}
        </aside>
      </div>
    </div>
  )
}

function StatCard({
  label,
  count,
  total,
  state,
}: {
  label: string
  count: number
  total: number
  state: CellState
}) {
  const pct = total ? Math.round((count / total) * 100) : 0
  const colors: Record<CellState, string> = {
    supported: "border-success/40 bg-success/5 text-success",
    weak: "border-warning/40 bg-warning/5 text-warning",
    missing: "border-destructive/40 bg-destructive/5 text-destructive",
    rebutted: "border-case-muted/40 bg-case-muted/5 text-case-muted",
  }
  return (
    <div className={cn("rounded-md border p-3", colors[state])}>
      <div className="flex items-center justify-between text-[11px] font-medium uppercase tracking-wider">
        <span>{label}</span>
        <span>{pct}%</span>
      </div>
      <div className="mt-1 font-mono text-2xl font-semibold tabular-nums">
        {count}
        <span className="text-sm text-muted-foreground"> / {total}</span>
      </div>
    </div>
  )
}

function CellStateIcon({ state }: { state: CellState }) {
  if (state === "supported") {
    return <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-success" />
  }
  if (state === "weak") {
    return <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-warning" />
  }
  if (state === "rebutted") {
    return <XCircle className="mt-0.5 h-4 w-4 shrink-0 text-destructive" />
  }
  return (
    <span className="mt-0.5 inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-full border-2 border-dashed border-destructive/60" />
  )
}

function CellDetail({ entry, matter }: { entry: CellEntry; matter: Matter }) {
  const router = useRouter()
  const { claim, element, facts, state } = entry
  const [documentId, setDocumentId] = useState(matter.documents[0]?.id ?? "")
  const [factId, setFactId] = useState(facts[0]?.id ?? matter.facts[0]?.id ?? "")
  const [relation, setRelation] = useState<"supports" | "contradicts">("supports")
  const [quote, setQuote] = useState("")
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const linkedEvidence = matter.evidence.filter((evidence) =>
    facts.some(
      (fact) =>
        evidence.supports_fact_ids.includes(fact.id) ||
        evidence.contradicts_fact_ids.includes(fact.id),
    ),
  )

  async function onCreateEvidence() {
    if (!documentId || !factId || !quote.trim()) {
      setError("Choose a document, fact, and quote.")
      return
    }
    setSaving(true)
    setError(null)
    const result = await createEvidence(matter.id, {
      document_id: documentId,
      quote: quote.trim(),
      source_span: "manual quote",
      evidence_type: "document_text",
      strength: "moderate",
      confidence: 0.75,
      supports_fact_ids: relation === "supports" ? [factId] : [],
      contradicts_fact_ids: relation === "contradicts" ? [factId] : [],
    })
    setSaving(false)
    if (!result.data) {
      setError(result.error || "Evidence could not be created.")
      return
    }
    setQuote("")
    router.refresh()
  }

  return (
    <ScrollArea className="h-[calc(100vh-280px)]">
      <div className="space-y-5 p-5">
        <div>
          <Badge variant="outline" className="text-[9px] uppercase">
            {claim.kind} · element
          </Badge>
          <h2 className="mt-1.5 text-base font-semibold leading-snug text-foreground text-pretty">
            {element.title}
          </h2>
          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
            {element.description}
          </p>
        </div>

        <div className="flex items-center gap-2">
          <CellStateIcon state={state} />
          <span className="text-sm font-medium capitalize text-foreground">{state}</span>
          <span className="text-xs text-muted-foreground">
            · {facts.length} supporting fact{facts.length === 1 ? "" : "s"}
          </span>
        </div>

        {element.legalAuthority && (
          <div className="rounded-md border border-border bg-muted/30 p-3 text-xs">
            <p className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              Legal authority
            </p>
            <p className="mt-1 leading-relaxed text-foreground">{element.legalAuthority}</p>
          </div>
        )}

        <div>
          <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Supporting facts
          </h3>
          {facts.length === 0 ? (
            <Card className="mt-2 border-dashed bg-transparent p-4 text-center">
              <p className="text-xs text-muted-foreground">No supporting facts linked yet.</p>
              <Button size="sm" variant="outline" className="mt-2 h-7 gap-1 bg-transparent text-[11px]">
                <Sparkles className="h-3 w-3" />
                Suggest evidence
              </Button>
            </Card>
          ) : (
            <ul className="mt-2 space-y-1.5">
              {facts.map((fact) => (
                <li key={fact.id}>
                  <Link
                    href={matterFactsHref(matter.id, fact.id)}
                    className="block rounded-md border border-border bg-background p-2.5 text-xs transition-colors hover:border-foreground/20 hover:bg-muted/40"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <p className="font-medium leading-tight text-foreground">{fact.statement}</p>
                      <ConfidenceBadge value={fact.confidence} size="sm" />
                    </div>
                    {fact.date && (
                      <p className="mt-1 font-mono text-[10px] text-muted-foreground">
                        {fact.date}
                      </p>
                    )}
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div>
          <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Evidence links
          </h3>
          {linkedEvidence.length > 0 && (
            <ul className="mt-2 space-y-1.5">
              {linkedEvidence.map((evidence) => (
                <li key={evidence.evidence_id} className="rounded-md border border-border bg-background p-2.5 text-xs">
                  <p className="line-clamp-3 leading-relaxed text-foreground">{evidence.quote}</p>
                  <p className="mt-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                    {evidence.strength} · {Math.round(evidence.confidence * 100)}%
                  </p>
                </li>
              ))}
            </ul>
          )}
          <div className="mt-2 space-y-2 rounded-md border border-border bg-background p-3">
            <div className="grid gap-2">
              <select
                value={documentId}
                onChange={(event) => setDocumentId(event.target.value)}
                className="rounded border border-border bg-card px-3 py-2 text-xs"
              >
                {matter.documents.map((document) => (
                  <option key={document.id} value={document.id}>
                    {document.title}
                  </option>
                ))}
              </select>
              <select
                value={factId}
                onChange={(event) => setFactId(event.target.value)}
                className="rounded border border-border bg-card px-3 py-2 text-xs"
              >
                {matter.facts.map((fact) => (
                  <option key={fact.id} value={fact.id}>
                    {fact.statement.slice(0, 90)}
                  </option>
                ))}
              </select>
              <select
                value={relation}
                onChange={(event) => setRelation(event.target.value as "supports" | "contradicts")}
                className="rounded border border-border bg-card px-3 py-2 font-mono text-xs"
              >
                <option value="supports">supports</option>
                <option value="contradicts">contradicts</option>
              </select>
              <textarea
                value={quote}
                onChange={(event) => setQuote(event.target.value)}
                rows={3}
                placeholder="Evidence quote or description"
                className="rounded border border-border bg-card px-3 py-2 text-xs focus:border-primary focus:outline-none"
              />
            </div>
            <div className="flex items-center justify-between gap-3">
              <p className="text-xs text-destructive">{error}</p>
              <Button size="sm" onClick={onCreateEvidence} disabled={saving || matter.documents.length === 0 || matter.facts.length === 0}>
                <Plus className="mr-1 h-3 w-3" />
                {saving ? "Saving" : "Add evidence"}
              </Button>
            </div>
          </div>
        </div>

        <Card className="border-border/60 bg-muted/20 p-3">
          <div className="flex items-start gap-2">
            <Sparkles className="mt-0.5 h-3.5 w-3.5 shrink-0 text-foreground" />
            <div className="text-xs">
              <p className="font-medium text-foreground">Evidence suggestion</p>
              <p className="mt-1 leading-relaxed text-muted-foreground">
                {state === "missing"
                  ? "No supporting facts yet. Consider deposing the property manager and pulling tenant complaint logs."
                  : state === "weak"
                    ? "Strengthen by adding a contemporaneous email or expert declaration."
                    : state === "rebutted"
                      ? "Opposition has filed a counter-fact. Consider impeachment evidence."
                      : "Element fully supported. Consider stipulating to streamline trial."}
              </p>
            </div>
          </div>
        </Card>
      </div>
    </ScrollArea>
  )
}
