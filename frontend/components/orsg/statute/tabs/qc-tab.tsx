import type { StatutePageResponse } from "@/lib/types"
import { QCBadge } from "@/components/orsg/badges"
import { AlertTriangle, CheckCircle2, XCircle, Info } from "lucide-react"

export function QCTab({ data }: { data: StatutePageResponse }) {
  const qc = data.qc
  return (
    <div className="px-6 py-6">
      <div className="mb-4 flex items-center gap-3">
        <QCBadge status={qc.status} size="md" />
        <span className="font-mono text-sm tabular-nums text-foreground">
          {qc.passed_checks}/{qc.total_checks} checks passed
        </span>
      </div>

      {qc.notes.length === 0 ? (
        <div className="rounded border border-border bg-card p-6 text-center text-sm text-muted-foreground">
          No QC issues detected for this statute.
        </div>
      ) : (
        <ul className="space-y-2">
          {qc.notes.map((note) => {
            const Icon =
              note.level === "fail"
                ? XCircle
                : note.level === "warning"
                  ? AlertTriangle
                  : Info
            const color =
              note.level === "fail"
                ? "text-destructive"
                : note.level === "warning"
                  ? "text-warning"
                  : "text-muted-foreground"
            return (
              <li
                key={note.note_id}
                className="flex items-start gap-3 rounded border border-border bg-card p-4"
              >
                <Icon className={`mt-0.5 h-4 w-4 flex-none ${color}`} />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className={`font-mono text-[10px] uppercase tracking-wide ${color}`}>
                      {note.level}
                    </span>
                    <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                      {note.category}
                    </span>
                    {note.related_id && (
                      <span className="font-mono text-xs text-primary">{note.related_id}</span>
                    )}
                  </div>
                  <p className="mt-1 text-sm text-foreground">{note.message}</p>
                </div>
              </li>
            )
          })}
        </ul>
      )}

      {/* Per-provision QC heatmap */}
      <div className="mt-6">
        <h3 className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          per-provision qc status
        </h3>
        <div className="overflow-hidden rounded border border-border bg-card">
          <ProvisionQCList data={data} />
        </div>
      </div>
    </div>
  )
}

function ProvisionQCList({ data }: { data: StatutePageResponse }) {
  const flat: { citation: string; qc: any; status: string; signals: any[] }[] = []
  function walk(p: any) {
    flat.push({ citation: p.display_citation, qc: p.qc_status, status: p.status, signals: p.signals })
    if (p.children) p.children.forEach(walk)
  }
  data.provisions.forEach(walk)

  return (
    <ul className="divide-y divide-border">
      {flat.map((row) => (
        <li
          key={row.citation}
          className="flex items-center justify-between px-4 py-2 hover:bg-muted/30"
        >
          <div className="flex items-center gap-3">
            <CheckIndicator status={row.qc} />
            <span className="font-mono text-xs text-primary">{row.citation}</span>
          </div>
          <div className="flex items-center gap-2">
            {row.signals.map((s: string) => (
              <span
                key={s}
                className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground"
              >
                {s}
              </span>
            ))}
          </div>
        </li>
      ))}
    </ul>
  )
}

function CheckIndicator({ status }: { status: "pass" | "warning" | "fail" }) {
  if (status === "pass") return <CheckCircle2 className="h-3.5 w-3.5 text-success" />
  if (status === "warning") return <AlertTriangle className="h-3.5 w-3.5 text-warning" />
  return <XCircle className="h-3.5 w-3.5 text-destructive" />
}
