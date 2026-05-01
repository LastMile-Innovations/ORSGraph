"use client"

import { useState } from "react"
import { AlertTriangle, CheckCircle2, ChevronRight, Database, FileSearch, GitBranch, Hash, Layers, Sparkles, XCircle } from "lucide-react"
import { qcCorpus, corpusStatus } from "@/lib/mock-data"
import { Button } from "@/components/ui/button"
import type { QCPanel } from "@/lib/types"

const CATEGORY_ICON: Record<QCPanel["category"], React.ComponentType<{ className?: string }>> = {
  source: Database,
  parse: FileSearch,
  chunk: Layers,
  citation: Hash,
  graph: GitBranch,
  embedding: Sparkles,
}

export function QCConsoleClient() {
  const [activePanel, setActivePanel] = useState<string>(qcCorpus.panels[0]?.panel_id ?? "")
  const panel = qcCorpus.panels.find((p) => p.panel_id === activePanel) ?? qcCorpus.panels[0]
  const passRate = qcCorpus.passed / qcCorpus.total_checks

  return (
    <div className="flex h-full overflow-hidden">
      {/* Main */}
      <div className="flex-1 min-w-0 flex flex-col overflow-hidden">
        {/* Header / topline */}
        <div className="border-b border-border bg-card px-6 py-5">
          <div className="flex items-start justify-between gap-4 mb-4">
            <div>
              <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-1">
                Quality control
              </div>
              <h1 className="font-serif text-3xl tracking-tight text-foreground">QC Console</h1>
              <p className="text-sm text-muted-foreground mt-1 font-mono">
                run {qcCorpus.run_id.replace(/^qc:run:/, "")} · {(qcCorpus.duration_ms / 1000).toFixed(1)}s
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" className="font-mono text-xs">
                rerun checks
              </Button>
              <Button size="sm" className="font-mono text-xs">
                export report
              </Button>
            </div>
          </div>

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
              value={corpusStatus.citations.unresolved.toLocaleString()}
              hint={`of ${corpusStatus.citations.total.toLocaleString()} mentions`}
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
