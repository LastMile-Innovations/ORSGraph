"use client"

import { useMemo, useRef, useState } from "react"
import Link from "next/link"
import type { FactCheckReport, FactCheckStatus } from "@/lib/types"
import { createDraft, createMatter, factCheckDraft, patchDraft, citationCheckDraft } from "@/lib/casebuilder/api"
import type { CaseCitationCheckFinding, CaseFactCheckFinding } from "@/lib/casebuilder/types"
import { StatusBadge } from "@/components/orsg/badges"
import {
  AlertTriangle,
  CheckCircle2,
  CircleHelp,
  ExternalLink,
  FileSearch,
  Quote,
  Scale,
  Sparkles,
  XCircle,
} from "lucide-react"
import { cn } from "@/lib/utils"

const STATUS_META: Record<
  FactCheckStatus,
  { label: string; icon: typeof CheckCircle2; cls: string; ring: string }
> = {
  supported: {
    label: "supported",
    icon: CheckCircle2,
    cls: "bg-success/15 text-success",
    ring: "ring-success/40",
  },
  partially_supported: {
    label: "partially supported",
    icon: AlertTriangle,
    cls: "bg-warning/15 text-warning",
    ring: "ring-warning/40",
  },
  unsupported: {
    label: "unsupported",
    icon: XCircle,
    cls: "bg-destructive/15 text-destructive",
    ring: "ring-destructive/40",
  },
  contradicted: {
    label: "contradicted",
    icon: XCircle,
    cls: "bg-destructive/20 text-destructive",
    ring: "ring-destructive/50",
  },
  wrong_citation: {
    label: "wrong citation",
    icon: AlertTriangle,
    cls: "bg-warning/20 text-warning",
    ring: "ring-warning/40",
  },
  stale_law: {
    label: "stale law",
    icon: AlertTriangle,
    cls: "bg-warning/15 text-warning",
    ring: "ring-warning/40",
  },
  needs_source: {
    label: "needs source",
    icon: CircleHelp,
    cls: "bg-muted text-muted-foreground",
    ring: "ring-border",
  },
}

export function FactCheckWorkflowClient() {
  const [title, setTitle] = useState("Standalone fact check")
  const [text, setText] = useState("")
  const [pending, setPending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [report, setReport] = useState<FactCheckReport | null>(null)

  async function runFactCheck(event: React.FormEvent) {
    event.preventDefault()
    if (!text.trim()) return
    setPending(true)
    setError(null)
    setReport(null)
    const paragraphs = text.split(/\n\s*\n/).map((paragraph) => paragraph.trim()).filter(Boolean)
    try {
      const matter = await createMatter({
        name: title || "Standalone fact check",
        matter_type: "fact_check",
        user_role: "researcher",
        jurisdiction: "Oregon",
      })
      if (!matter.data) throw new Error(matter.error || "Matter could not be created.")
      const draft = await createDraft(matter.data.id, {
        title,
        draft_type: "memo",
        description: "Standalone fact-check input",
      })
      if (!draft.data) throw new Error(draft.error || "Draft could not be created.")
      await patchDraft(matter.data.id, draft.data.id, {
        sections: [
          {
            id: "section:input",
            heading: "Input",
            body: text,
            citations: [],
            suggestions: [],
            comments: [],
          } as any,
        ],
        paragraphs: paragraphs.map((paragraph, index) => ({
          paragraph_id: `paragraph:${index + 1}`,
          id: `paragraph:${index + 1}`,
          matter_id: matter.data!.id,
          draft_id: draft.data!.id,
          number: index + 1,
          ordinal: index + 1,
          role: "facts",
          text: paragraph,
          authorities: [],
          fact_ids: [],
          evidence_ids: [],
          locked: false,
          review_status: "needs_review",
        } as any)),
      })
      const [facts, citations] = await Promise.all([
        factCheckDraft(matter.data.id, draft.data.id),
        citationCheckDraft(matter.data.id, draft.data.id),
      ])
      if (!facts.data) throw new Error(facts.error || "Fact-check failed.")
      if (!citations.data) throw new Error(citations.error || "Citation-check failed.")
      setReport(buildFactCheckReport(title, paragraphs, facts.data.result ?? [], citations.data.result ?? []))
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Fact-check failed.")
    } finally {
      setPending(false)
    }
  }

  if (report) return <FactCheckClient report={report} />

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-1 flex-col gap-4 p-6">
      <div>
        <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          <FileSearch className="h-3 w-3" />
          fact-check / live
        </div>
        <h1 className="mt-1 text-xl font-semibold">Fact Check</h1>
      </div>
      <form onSubmit={runFactCheck} className="space-y-3">
        <input
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          className="h-9 w-full rounded border border-border bg-card px-3 text-sm"
          placeholder="Document title"
        />
        <textarea
          value={text}
          onChange={(event) => setText(event.target.value)}
          className="min-h-[360px] w-full resize-y rounded border border-border bg-card px-3 py-2 text-sm leading-6"
          placeholder="Paste the draft, complaint, motion, or memo to check..."
        />
        {error && <div className="rounded border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">{error}</div>}
        <button
          type="submit"
          disabled={pending || !text.trim()}
          className="rounded bg-primary px-4 py-2 font-mono text-xs uppercase tracking-wider text-primary-foreground disabled:opacity-50"
        >
          {pending ? "Checking" : "Run live check"}
        </button>
      </form>
    </div>
  )
}

export function FactCheckClient({ report }: { report: FactCheckReport }) {
  const [activeId, setActiveId] = useState<string | null>(report.findings[0]?.finding_id ?? null)
  const [filter, setFilter] = useState<FactCheckStatus | "all">("all")
  const docRef = useRef<HTMLDivElement>(null)
  const findingsRef = useRef<HTMLDivElement>(null)

  const findingsByPara = useMemo(() => {
    const map = new Map<number, FactCheckReport["findings"]>()
    for (const f of report.findings) {
      const arr = map.get(f.paragraph_index) ?? []
      arr.push(f)
      map.set(f.paragraph_index, arr)
    }
    return map
  }, [report.findings])

  const filteredFindings = useMemo(
    () => (filter === "all" ? report.findings : report.findings.filter((f) => f.status === filter)),
    [report.findings, filter],
  )

  function jumpToFinding(id: string, paragraphIndex: number) {
    setActiveId(id)
    const para = docRef.current?.querySelector(`[data-pid="${paragraphIndex}"]`)
    para?.scrollIntoView({ behavior: "smooth", block: "center" })
    const card = findingsRef.current?.querySelector(`[data-fid="${id}"]`)
    card?.scrollIntoView({ behavior: "smooth", block: "center" })
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {/* Header */}
      <div className="border-b border-border bg-card px-6 py-4">
        <div className="flex flex-col items-start justify-between gap-3 lg:flex-row lg:items-center">
          <div className="min-w-0">
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <FileSearch className="h-3 w-3" />
              fact-check / {report.document.doc_type}
            </div>
            <h1 className="mt-1 line-clamp-1 text-base font-semibold leading-tight">
              {report.document.title}
            </h1>
            <div className="mt-1 flex flex-wrap items-center gap-3 font-mono text-[10px] tabular-nums text-muted-foreground">
              <span>{report.document.word_count.toLocaleString()} words</span>
              <span className="text-border">|</span>
              <span>{report.document.paragraphs.length} paragraphs</span>
              <span className="text-border">|</span>
              <span>uploaded {new Date(report.document.uploaded_at).toLocaleString()}</span>
            </div>
          </div>

          <SummaryBar report={report} onFilter={setFilter} active={filter} />
        </div>
      </div>

      {/* 3-pane: doc | findings | citation table */}
      <div className="grid flex-1 grid-cols-1 overflow-hidden lg:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)] xl:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)_360px]">
        {/* Document */}
        <div ref={docRef} className="overflow-y-auto border-r border-border bg-background">
          <div className="mx-auto max-w-3xl p-6 font-serif text-[15px] leading-relaxed">
            {report.document.paragraphs.map((p) => {
              const fs = findingsByPara.get(p.index) ?? []
              const dominant = fs[0]?.status
              const meta = dominant ? STATUS_META[dominant] : null
              return (
                <div
                  key={p.paragraph_id}
                  data-pid={p.index}
                  className={cn(
                    "group relative mb-5 rounded border border-transparent p-3 transition-colors",
                    fs.length > 0 && "border-border hover:border-primary/40",
                    fs.some((f) => f.finding_id === activeId) && "ring-2",
                    fs.some((f) => f.finding_id === activeId) && meta?.ring,
                  )}
                  onClick={() => fs[0] && jumpToFinding(fs[0].finding_id, p.index)}
                  role={fs.length > 0 ? "button" : undefined}
                >
                  <div className="absolute -left-7 top-3 select-none font-mono text-[10px] tabular-nums text-muted-foreground">
                    {String(p.index).padStart(2, "0")}
                  </div>
                  <p className="text-foreground">{p.text}</p>
                  {fs.length > 0 && (
                    <div className="mt-2 flex flex-wrap items-center gap-1.5">
                      {fs.map((f) => (
                        <FindingPill
                          key={f.finding_id}
                          status={f.status}
                          onClick={(e) => {
                            e.stopPropagation()
                            jumpToFinding(f.finding_id, p.index)
                          }}
                        />
                      ))}
                    </div>
                  )}
                </div>
              )
            })}
          </div>
        </div>

        {/* Findings */}
        <div ref={findingsRef} className="overflow-y-auto bg-background">
          <div className="sticky top-0 z-10 flex items-center justify-between border-b border-border bg-card px-4 py-2">
            <div className="font-mono text-[11px] uppercase tracking-wider text-muted-foreground">
              {filter === "all" ? "all findings" : STATUS_META[filter].label} · {filteredFindings.length}
            </div>
            <button
              onClick={() => setFilter("all")}
              className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:text-primary"
            >
              clear filter
            </button>
          </div>

          <div className="space-y-3 p-4">
            {filteredFindings.map((f) => {
              const meta = STATUS_META[f.status]
              const Icon = meta.icon
              const active = f.finding_id === activeId
              return (
                <div
                  key={f.finding_id}
                  data-fid={f.finding_id}
                  onClick={() => jumpToFinding(f.finding_id, f.paragraph_index)}
                  className={cn(
                    "cursor-pointer rounded border bg-card p-3 transition-colors",
                    active ? "border-primary" : "border-border hover:border-primary/40",
                  )}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex items-center gap-2">
                      <span
                        className={cn(
                          "inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
                          meta.cls,
                        )}
                      >
                        <Icon className="h-3 w-3" />
                        {meta.label}
                      </span>
                      <span className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                        ¶ {f.paragraph_index}
                      </span>
                    </div>
                    <div className="font-mono text-[10px] tabular-nums text-muted-foreground">
                      conf {(f.confidence * 100).toFixed(0)}%
                    </div>
                  </div>

                  <p className="mt-2 text-sm text-foreground">{f.claim}</p>

                  <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{f.explanation}</p>

                  {f.suggested_fix && (
                    <div className="mt-2 rounded border border-primary/30 bg-primary/5 p-2">
                      <div className="mb-1 flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-primary">
                        <Sparkles className="h-3 w-3" />
                        suggested fix
                      </div>
                      <p className="text-xs text-foreground">{f.suggested_fix}</p>
                    </div>
                  )}

                  {f.sources.length > 0 && (
                    <div className="mt-2 space-y-1.5 border-t border-border pt-2">
                      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                        sources ({f.sources.length})
                      </div>
                      {f.sources.map((s, i) => (
                        <div
                          key={i}
                          className="rounded border border-border bg-background p-2"
                          onClick={(e) => e.stopPropagation()}
                        >
                          <div className="flex items-center justify-between gap-2">
                            {s.canonical_id ? (
                              <Link
                                href={`/statutes/${s.canonical_id}`}
                                className="font-mono text-xs text-primary hover:underline"
                              >
                                {s.citation}
                              </Link>
                            ) : (
                              <span className="font-mono text-xs text-muted-foreground">{s.citation}</span>
                            )}
                            <StatusBadge status={s.status} />
                          </div>
                          {s.quote && (
                            <p className="mt-1.5 flex gap-1.5 font-serif text-[12px] leading-snug text-muted-foreground">
                              <Quote className="h-3 w-3 flex-shrink-0 mt-0.5" />
                              <span className="italic">{s.quote}</span>
                            </p>
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )
            })}
            {filteredFindings.length === 0 && (
              <div className="rounded border border-dashed border-border bg-card p-8 text-center font-mono text-xs text-muted-foreground">
                no findings match the current filter.
              </div>
            )}
          </div>
        </div>

        {/* Citation table — shown on xl+ */}
        <aside className="hidden overflow-y-auto border-l border-border bg-card xl:block">
          <div className="sticky top-0 border-b border-border bg-card px-4 py-2">
            <div className="font-mono text-[11px] uppercase tracking-wider text-muted-foreground">
              citation table
            </div>
          </div>
          <div className="divide-y divide-border">
            {report.citation_table.map((c, i) => (
              <div key={i} className="p-3">
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <div className="font-mono text-[11px] text-foreground">{c.raw_citation}</div>
                    {c.resolved_citation && c.resolved_citation !== c.raw_citation && (
                      <div className="mt-0.5 font-mono text-[10px] text-muted-foreground">
                        → {c.resolved_citation}
                      </div>
                    )}
                  </div>
                </div>
                <div className="mt-1.5 flex flex-wrap items-center gap-2 font-mono text-[10px] tabular-nums text-muted-foreground">
                  {c.status !== "unresolved" && <StatusBadge status={c.status as any} />}
                  {c.edition_year && <span>ed {c.edition_year}</span>}
                  <span className="ml-auto flex items-center gap-1">
                    {c.occurrences.map((o) => (
                      <button
                        key={o}
                        onClick={() => {
                          const para = docRef.current?.querySelector(`[data-pid="${o}"]`)
                          para?.scrollIntoView({ behavior: "smooth", block: "center" })
                        }}
                        className="rounded border border-border px-1.5 hover:border-primary hover:text-primary"
                      >
                        ¶{o}
                      </button>
                    ))}
                  </span>
                </div>
                {c.canonical_id && (
                  <Link
                    href={`/statutes/${c.canonical_id}`}
                    className="mt-1.5 inline-flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:text-primary"
                  >
                    open
                    <ExternalLink className="h-3 w-3" />
                  </Link>
                )}
              </div>
            ))}
          </div>
        </aside>
      </div>
    </div>
  )
}

function FindingPill({
  status,
  onClick,
}: {
  status: FactCheckStatus
  onClick?: (e: React.MouseEvent) => void
}) {
  const meta = STATUS_META[status]
  const Icon = meta.icon
  return (
    <button
      onClick={onClick}
      className={cn(
        "inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
        meta.cls,
      )}
    >
      <Icon className="h-3 w-3" />
      {meta.label}
    </button>
  )
}

function SummaryBar({
  report,
  onFilter,
  active,
}: {
  report: FactCheckReport
  onFilter: (f: FactCheckStatus | "all") => void
  active: FactCheckStatus | "all"
}) {
  const items: { key: FactCheckStatus | "all"; label: string; count: number; tone: string }[] = [
    { key: "all", label: "all", count: report.summary.total, tone: "text-foreground" },
    { key: "supported", label: "supported", count: report.summary.supported, tone: "text-success" },
    { key: "partially_supported", label: "partial", count: report.summary.partial, tone: "text-warning" },
    { key: "unsupported", label: "unsupported", count: report.summary.unsupported, tone: "text-destructive" },
    {
      key: "contradicted",
      label: "contradicted",
      count: report.summary.contradicted,
      tone: "text-destructive",
    },
    { key: "wrong_citation", label: "wrong cite", count: report.summary.wrong_citation, tone: "text-warning" },
    { key: "stale_law", label: "stale", count: report.summary.stale_law, tone: "text-warning" },
    {
      key: "needs_source",
      label: "needs src",
      count: report.summary.needs_source,
      tone: "text-muted-foreground",
    },
  ]
  return (
    <div className="flex flex-wrap items-center gap-1">
      {items.map((it) => (
        <button
          key={it.key}
          onClick={() => onFilter(it.key)}
          className={cn(
            "flex items-center gap-1 rounded border px-2 py-1 font-mono text-[10px] uppercase tracking-wider",
            active === it.key
              ? "border-primary bg-primary/10 text-primary"
              : "border-border hover:border-primary/40",
          )}
        >
          <Scale className={cn("h-3 w-3", active === it.key ? "" : it.tone)} />
          <span className={active === it.key ? "" : it.tone}>{it.label}</span>
          <span className="tabular-nums">{it.count}</span>
        </button>
      ))}
    </div>
  )
}

function buildFactCheckReport(
  title: string,
  paragraphs: string[],
  factFindings: CaseFactCheckFinding[],
  citationFindings: CaseCitationCheckFinding[],
): FactCheckReport {
  const findings = [
    ...factFindings.map((finding, index) => ({
      finding_id: finding.finding_id,
      paragraph_id: finding.paragraph_id ?? `paragraph:${index + 1}`,
      paragraph_index: paragraphIndex(finding.paragraph_id, index),
      claim: finding.message,
      status: findingStatus(finding.severity),
      confidence: finding.severity === "info" ? 0.9 : 0.68,
      explanation: finding.message,
      suggested_fix: finding.status === "open" ? "Review supporting matter facts and evidence." : null,
      sources: [],
    })),
    ...citationFindings.map((finding, index) => ({
      finding_id: finding.finding_id,
      paragraph_id: `citation:${index + 1}`,
      paragraph_index: index + 1,
      claim: finding.citation || "Citation",
      status: findingStatus(finding.severity),
      confidence: finding.canonical_id ? 0.85 : 0.55,
      explanation: finding.message,
      suggested_fix: finding.status === "open" ? "Review citation resolution before relying on this authority." : null,
      sources: finding.canonical_id
        ? [{
            citation: finding.citation,
            canonical_id: finding.canonical_id,
            quote: null,
            edition_year: new Date().getFullYear(),
            status: "active" as const,
          }]
        : [],
    })),
  ]
  return {
    document: {
      document_id: `fact-check:${Date.now()}`,
      title,
      doc_type: "memo",
      word_count: paragraphs.join(" ").split(/\s+/).filter(Boolean).length,
      uploaded_at: new Date().toISOString(),
      paragraphs: paragraphs.map((paragraph, index) => ({
        paragraph_id: `paragraph:${index + 1}`,
        index: index + 1,
        text: paragraph,
      })),
    },
    findings,
    summary: {
      total: findings.length,
      supported: findings.filter((finding) => finding.status === "supported").length,
      partial: findings.filter((finding) => finding.status === "partially_supported").length,
      unsupported: findings.filter((finding) => finding.status === "unsupported").length,
      contradicted: findings.filter((finding) => finding.status === "contradicted").length,
      wrong_citation: findings.filter((finding) => finding.status === "wrong_citation").length,
      stale_law: findings.filter((finding) => finding.status === "stale_law").length,
      needs_source: findings.filter((finding) => finding.status === "needs_source").length,
    },
    citation_table: citationFindings.map((finding, index) => ({
      raw_citation: finding.citation,
      resolved_citation: finding.canonical_id ? finding.citation : null,
      canonical_id: finding.canonical_id ?? null,
      edition_year: finding.canonical_id ? new Date().getFullYear() : null,
      status: finding.canonical_id ? "active" : "unresolved",
      occurrences: [index + 1],
    })),
  }
}

function paragraphIndex(paragraphId: string | null | undefined, fallback: number) {
  const match = paragraphId?.match(/(\d+)$/)
  return match ? Number.parseInt(match[1], 10) : fallback + 1
}

function findingStatus(severity: string): FactCheckStatus {
  if (severity === "blocking" || severity === "serious") return "unsupported"
  if (severity === "warning") return "partially_supported"
  return "supported"
}
