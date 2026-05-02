"use client"

import { useEffect, useState } from "react"
import { AlertTriangle, CheckCircle2, ChevronRight, Database, Download, FileSearch, GitBranch, Hash, Layers, RotateCcw, ShieldCheck, Sparkles, XCircle } from "lucide-react"
import { getQCReport, getQCSummary, runQCRun, type QCSummary } from "@/lib/api"
import { Button } from "@/components/ui/button"
import type { QCPanel, QCRunSummary } from "@/lib/types"

const CATEGORY_ICON: Record<QCPanel["category"], React.ComponentType<{ className?: string }>> = {
  source: Database,
  parse: FileSearch,
  chunk: Layers,
  citation: Hash,
  graph: GitBranch,
  embedding: Sparkles,
}

export function QCConsoleClient() {
  const [qcCorpus, setQCCorpus] = useState<QCRunSummary | null>(null)
  const [activePanel, setActivePanel] = useState<string>("")
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    getQCSummary()
      .then((summary) => {
        if (cancelled) return
        const next = buildQCRun(summary)
        setQCCorpus(next)
        setActivePanel(next.panels[0]?.panel_id ?? "")
      })
      .catch((reason) => {
        if (!cancelled) setError(reason instanceof Error ? reason.message : "QC summary failed.")
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [])

  async function rerunChecks() {
    setBusy(true)
    setError(null)
    setMessage(null)
    try {
      const run = await runQCRun()
      const next = buildQCRun(run.summary, run.run_id)
      setQCCorpus(next)
      setActivePanel(next.panels[0]?.panel_id ?? "")
      setMessage(`QC run ${run.run_id.replace(/^qc:run:/, "")} completed.`)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "QC run failed.")
    } finally {
      setBusy(false)
    }
  }

  async function exportReport() {
    setBusy(true)
    setError(null)
    setMessage(null)
    try {
      const report = await getQCReport("csv")
      const blob = new Blob([report.content], { type: report.mime_type })
      const url = URL.createObjectURL(blob)
      const anchor = document.createElement("a")
      anchor.href = url
      anchor.download = `${report.report_id.replace(/[:/]/g, "-")}.csv`
      anchor.click()
      URL.revokeObjectURL(url)
      setMessage("QC report exported.")
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "QC export failed.")
    } finally {
      setBusy(false)
    }
  }

  if (loading) {
    return <div className="p-6 text-sm text-muted-foreground">Loading QC summary...</div>
  }

  if (!qcCorpus) {
    return <div className="p-6 text-sm text-destructive">QC summary unavailable{error ? `: ${error}` : "."}</div>
  }

  const panel = qcCorpus.panels.find((p) => p.panel_id === activePanel) ?? qcCorpus.panels[0]
  const passRate = qcCorpus.total_checks > 0 ? qcCorpus.passed / qcCorpus.total_checks : 0
  const unresolvedCitations = panelCount(qcCorpus, "citation", "fail") + panelCount(qcCorpus, "citation", "warning")

  return (
    <div className="flex h-full overflow-hidden">
      {/* Main */}
      <div className="flex-1 min-w-0 flex flex-col overflow-hidden">
        {/* Header / topline */}
        <div className="border-b border-border bg-card px-4 py-5 sm:px-6">
          <div className="flex items-start justify-between gap-4 mb-4">
            <div>
              <div className="mb-2 flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                <ShieldCheck className="h-3.5 w-3.5 text-primary" />
                Quality control
              </div>
              <h1 className="text-2xl font-semibold tracking-normal text-foreground">QC Console</h1>
              <p className="mt-1 text-sm leading-6 text-muted-foreground">
                Run {qcCorpus.run_id.replace(/^qc:run:/, "")} completed in {(qcCorpus.duration_ms / 1000).toFixed(1)}s.
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" className="gap-1.5" disabled={busy} onClick={rerunChecks}>
                <RotateCcw className="h-3.5 w-3.5" />
                Rerun
              </Button>
              <Button size="sm" className="gap-1.5" disabled={busy} onClick={exportReport}>
                <Download className="h-3.5 w-3.5" />
                Export
              </Button>
            </div>
          </div>
          {(error || message) && (
            <div className={`mb-3 rounded border px-3 py-2 text-xs ${error ? "border-destructive/30 bg-destructive/5 text-destructive" : "border-primary/20 bg-primary/5 text-muted-foreground"}`}>
              {error || message}
            </div>
          )}

          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            <KpiCard
              icon={CheckCircle2}
              tone="ok"
              label="pass rate"
              value={`${(passRate * 100).toFixed(2)}%`}
              hint={`${qcCorpus.passed.toLocaleString()} / ${qcCorpus.total_checks.toLocaleString()}`}
            />
            <KpiCard
              icon={AlertTriangle}
              tone="warn"
              label="warnings"
              value={qcCorpus.warnings.toLocaleString()}
              hint={`${((qcCorpus.warnings / qcCorpus.total_checks) * 100).toFixed(2)}% of checks`}
            />
            <KpiCard
              icon={XCircle}
              tone="fail"
              label="failures"
              value={qcCorpus.failures.toLocaleString()}
              hint={`${((qcCorpus.failures / qcCorpus.total_checks) * 100).toFixed(2)}% of checks`}
            />
            <KpiCard
              icon={Hash}
              tone="info"
              label="unresolved citations"
              value={unresolvedCitations.toLocaleString()}
              hint={`${qcCorpus.panels.find((p) => p.category === "citation")?.count.toLocaleString() ?? "0"} citation QC rows`}
            />
          </div>
        </div>

        {/* Two-column: panel list + active panel */}
        <div className="flex-1 grid grid-cols-1 lg:grid-cols-[280px_1fr] overflow-hidden">
          {/* Panel list */}
          <nav className="border-r border-border bg-background/40 overflow-y-auto">
            <div className="p-4 sticky top-0 bg-background/80 backdrop-blur border-b border-border">
              <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground">
                Panels ({qcCorpus.panels.length})
              </div>
            </div>
            <ul className="p-2 space-y-1">
              {qcCorpus.panels.map((p) => {
                const Icon = CATEGORY_ICON[p.category]
                const active = p.panel_id === activePanel
                return (
                  <li key={p.panel_id}>
                    <button
                      onClick={() => setActivePanel(p.panel_id)}
                      className={`w-full text-left rounded-md px-3 py-2.5 transition-colors group ${
                        active ? "bg-accent border border-border" : "hover:bg-accent/50 border border-transparent"
                      }`}
                    >
                      <div className="flex items-center justify-between gap-2">
                        <div className="flex items-center gap-2 min-w-0">
                          <Icon className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                          <span className="text-sm text-foreground truncate">{p.title}</span>
                        </div>
                        <CountBadge status={p.status} count={p.count} />
                      </div>
                      <div className="text-[11px] text-muted-foreground font-mono mt-1 capitalize">
                        {p.category}
                      </div>
                    </button>
                  </li>
                )
              })}
            </ul>
          </nav>

          {/* Active panel */}
          <div className="overflow-y-auto">
            {panel && <PanelView panel={panel} />}
          </div>
        </div>
      </div>
    </div>
  )
}

function buildQCRun(summary: QCSummary, runId = `qc:run:${Date.now()}`): QCRunSummary {
  const panels: QCPanel[] = [
    {
      panel_id: "qc:panel:orphan",
      title: "Orphan records",
      category: "source",
      status: rowStatus(summary.orphan_counts.provisions + summary.orphan_counts.chunks + summary.orphan_counts.citations),
      count: summary.orphan_counts.provisions + summary.orphan_counts.chunks + summary.orphan_counts.citations,
      description: "Nodes that are missing required provenance or parent relationships.",
      rows: [
        qcRow("orphans:provisions", "Provision", summary.orphan_counts.provisions),
        qcRow("orphans:chunks", "RetrievalChunk", summary.orphan_counts.chunks),
        qcRow("orphans:citations", "CitationMention", summary.orphan_counts.citations),
      ].filter((row) => row.level !== "info"),
    },
    {
      panel_id: "qc:panel:duplicates",
      title: "Duplicate identities",
      category: "graph",
      status: rowStatus(summary.duplicate_counts.legal_text_identities + summary.duplicate_counts.provisions + summary.duplicate_counts.cites_relationships),
      count: summary.duplicate_counts.legal_text_identities + summary.duplicate_counts.provisions + summary.duplicate_counts.cites_relationships,
      description: "Duplicate canonical records and duplicated citation edges.",
      rows: [
        qcRow("duplicates:legal-text", "LegalTextIdentity", summary.duplicate_counts.legal_text_identities),
        qcRow("duplicates:provisions", "Provision", summary.duplicate_counts.provisions),
        qcRow("duplicates:cites", "CITES", summary.duplicate_counts.cites_relationships),
      ].filter((row) => row.level !== "info"),
    },
    {
      panel_id: "qc:panel:embedding",
      title: "Embedding readiness",
      category: "embedding",
      status: summary.embedding_readiness.coverage >= 95 || summary.embedding_readiness.total_chunks === 0 ? "pass" : "warning",
      count: Math.max(0, summary.embedding_readiness.total_chunks - summary.embedding_readiness.embedded_chunks),
      description: "Retrieval chunks without embeddings reduce semantic search coverage.",
      rows:
        summary.embedding_readiness.coverage >= 95 || summary.embedding_readiness.total_chunks === 0
          ? []
          : [
              {
                id: "embedding:coverage",
                citation: "RetrievalChunk",
                level: "warning",
                message: `${summary.embedding_readiness.coverage.toFixed(2)}% embedding coverage.`,
              },
            ],
    },
    {
      panel_id: "qc:panel:citations",
      title: "Citation resolution",
      category: "citation",
      status: summary.cites_coverage.coverage >= 95 || summary.cites_coverage.total_citations === 0 ? "pass" : "warning",
      count: Math.max(0, summary.cites_coverage.total_citations - summary.cites_coverage.resolved_citations),
      description: "Citation mentions that have not resolved to a graph target.",
      rows:
        summary.cites_coverage.coverage >= 95 || summary.cites_coverage.total_citations === 0
          ? []
          : [
              {
                id: "citations:coverage",
                citation: "CitationMention",
                level: "warning",
                message: `${summary.cites_coverage.coverage.toFixed(2)}% citation resolution coverage.`,
              },
            ],
    },
  ]
  const totalChecks = panels.reduce((sum, panel) => sum + Math.max(panel.count, 1), 0)
  const failures = panels.filter((panel) => panel.status === "fail").reduce((sum, panel) => sum + panel.count, 0)
  const warnings = panels.filter((panel) => panel.status === "warning").reduce((sum, panel) => sum + panel.count, 0)
  return {
    run_id: runId,
    ran_at: new Date().toISOString(),
    duration_ms: 0,
    status: failures > 0 ? "fail" : warnings > 0 ? "warning" : "pass",
    total_checks: totalChecks,
    passed: Math.max(0, totalChecks - warnings - failures),
    warnings,
    failures,
    panels,
  }
}

function qcRow(id: string, citation: string, count: number) {
  return {
    id,
    citation,
    level: count > 0 ? "fail" as const : "info" as const,
    message: count > 0 ? `${count.toLocaleString()} records need review.` : "Clean.",
  }
}

function rowStatus(count: number): "pass" | "warning" | "fail" {
  return count > 0 ? "fail" : "pass"
}

function panelCount(run: QCRunSummary, category: QCPanel["category"], status: QCPanel["status"]) {
  return run.panels
    .filter((panel) => panel.category === category && panel.status === status)
    .reduce((sum, panel) => sum + panel.count, 0)
}

function KpiCard({
  icon: Icon,
  label,
  value,
  hint,
  tone,
}: {
  icon: React.ComponentType<{ className?: string }>
  label: string
  value: string
  hint?: string
  tone: "ok" | "warn" | "fail" | "info"
}) {
  const toneClass =
    tone === "ok"
      ? "text-emerald-500"
      : tone === "warn"
      ? "text-amber-500"
      : tone === "fail"
      ? "text-rose-500"
      : "text-sky-500"
  return (
    <div className="rounded-lg border border-border bg-background/40 p-3">
      <div className="flex items-center gap-1.5">
        <Icon className={`h-3.5 w-3.5 ${toneClass}`} />
        <span className="text-[10px] font-mono uppercase tracking-wider text-muted-foreground">{label}</span>
      </div>
      <div className="font-mono text-2xl text-foreground mt-1.5 tabular-nums">{value}</div>
      {hint && <div className="text-[11px] text-muted-foreground font-mono mt-0.5">{hint}</div>}
    </div>
  )
}

function CountBadge({ status, count }: { status: QCPanel["status"]; count: number }) {
  if (count === 0) {
    return (
      <span className="inline-flex items-center gap-1 text-[10px] font-mono uppercase tracking-wider text-emerald-500">
        <CheckCircle2 className="h-3 w-3" />
        clean
      </span>
    )
  }
  const cls =
    status === "fail"
      ? "bg-rose-500/15 text-rose-500 border-rose-500/30"
      : status === "warning"
      ? "bg-amber-500/15 text-amber-500 border-amber-500/30"
      : "bg-sky-500/15 text-sky-500 border-sky-500/30"
  return (
    <span className={`text-[10px] font-mono px-1.5 py-0.5 rounded border tabular-nums ${cls}`}>
      {count.toLocaleString()}
    </span>
  )
}

function PanelView({ panel }: { panel: QCPanel }) {
  const Icon = CATEGORY_ICON[panel.category]
  return (
    <div className="px-6 py-6">
      <div className="flex items-center gap-2 text-xs text-muted-foreground font-mono mb-2">
        <span>QC</span>
        <ChevronRight className="h-3 w-3" />
        <span className="capitalize">{panel.category}</span>
        <ChevronRight className="h-3 w-3" />
        <span className="text-foreground">{panel.title}</span>
      </div>

      <div className="flex items-center gap-3 mb-2">
        <div className="rounded-md border border-border bg-card p-2">
          <Icon className="h-4 w-4 text-foreground" />
        </div>
        <h2 className="font-serif text-2xl text-foreground tracking-tight">{panel.title}</h2>
        <CountBadge status={panel.status} count={panel.count} />
      </div>
      <p className="text-sm text-muted-foreground font-mono max-w-2xl">{panel.description}</p>

      <div className="mt-6">
        {panel.rows.length === 0 ? (
          <div className="rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-6 flex items-center gap-3">
            <CheckCircle2 className="h-5 w-5 text-emerald-500 shrink-0" />
            <div>
              <div className="text-sm font-medium text-foreground">No issues detected</div>
              <p className="text-xs text-muted-foreground font-mono mt-0.5">
                All checks in this panel passed against the current corpus snapshot.
              </p>
            </div>
          </div>
        ) : (
          <div className="rounded-lg border border-border overflow-hidden">
            <table className="w-full text-sm">
              <thead className="bg-card border-b border-border">
                <tr className="text-xs font-mono uppercase tracking-wider text-muted-foreground">
                  <th className="text-left px-4 py-2 w-24">level</th>
                  <th className="text-left px-4 py-2">target</th>
                  <th className="text-left px-4 py-2">message</th>
                </tr>
              </thead>
              <tbody>
                {panel.rows.map((row, i) => (
                  <tr key={row.id} className={i % 2 === 0 ? "bg-background" : "bg-card/30"}>
                    <td className="px-4 py-3 align-top">
                      <LevelBadge level={row.level} />
                    </td>
                    <td className="px-4 py-3 align-top font-mono text-xs text-foreground whitespace-nowrap">
                      {row.citation}
                    </td>
                    <td className="px-4 py-3 align-top text-foreground/90">{row.message}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}

function LevelBadge({ level }: { level: "info" | "warning" | "fail" }) {
  const map = {
    info: { cls: "bg-sky-500/15 text-sky-500 border-sky-500/30", Icon: CheckCircle2 },
    warning: { cls: "bg-amber-500/15 text-amber-500 border-amber-500/30", Icon: AlertTriangle },
    fail: { cls: "bg-rose-500/15 text-rose-500 border-rose-500/30", Icon: XCircle },
  } as const
  const { cls, Icon } = map[level]
  return (
    <span className={`inline-flex items-center gap-1 text-[10px] font-mono uppercase tracking-wider px-2 py-1 rounded border ${cls}`}>
      <Icon className="h-3 w-3" />
      {level}
    </span>
  )
}
