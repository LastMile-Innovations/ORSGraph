import type { StatutePageResponse } from "@/lib/types"
import { StatusBadge, QCBadge } from "@/components/orsg/badges"
import { Button } from "@/components/ui/button"
import { Star, MessageSquare, FolderPlus, ExternalLink } from "lucide-react"

export function StatuteHeader({ data }: { data: StatutePageResponse }) {
  const { identity, current_version, qc, inbound_citations, outbound_citations } = data

  return (
    <header className="border-b border-border bg-card">
      <div className="px-6 pt-5 pb-3">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <span>{identity.jurisdiction}</span>
              <span className="text-border">/</span>
              <span>{identity.corpus}</span>
              <span className="text-border">/</span>
              <span>chapter {identity.chapter}</span>
              <span className="text-border">/</span>
              <span>edition {identity.edition}</span>
            </div>
            <div className="mt-1 flex items-baseline gap-3">
              <h1 className="font-mono text-2xl font-semibold tracking-tight text-foreground">
                {identity.citation}
              </h1>
              <h2 className="text-xl text-foreground">{identity.title}</h2>
            </div>
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <StatusBadge status={identity.status} />
              <QCBadge status={qc.status} />
              <span className="inline-flex items-center rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                effective {current_version.effective_date}
              </span>
              <span className="inline-flex items-center rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                {data.versions.length} versions
              </span>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" className="h-8 gap-1.5 bg-transparent">
              <Star className="h-3.5 w-3.5" />
              Save
            </Button>
            <Button variant="outline" size="sm" className="h-8 gap-1.5 bg-transparent">
              <FolderPlus className="h-3.5 w-3.5" />
              Add to matter
            </Button>
            <Button size="sm" className="h-8 gap-1.5">
              <MessageSquare className="h-3.5 w-3.5" />
              Ask about this
            </Button>
          </div>
        </div>

        {/* Quick metrics row */}
        <div className="mt-4 flex flex-wrap items-center gap-x-6 gap-y-1.5 border-t border-border pt-3 font-mono text-[11px] tabular-nums">
          <Metric label="provisions" value={countProvisions(data)} />
          <Metric label="cites" value={outbound_citations.length} accent />
          <Metric label="cited by" value={inbound_citations.length} accent />
          <Metric label="definitions" value={data.definitions.length} />
          <Metric label="exceptions" value={data.exceptions.length} />
          <Metric label="deadlines" value={data.deadlines.length} />
          <Metric label="penalties" value={data.penalties.length} />
          <Metric label="chunks" value={data.chunks.length} />
          <Metric
            label="qc"
            value={`${qc.passed_checks}/${qc.total_checks}`}
            warn={qc.status === "warning"}
            fail={qc.status === "fail"}
          />
          <a
            href={data.source_documents[0]?.url}
            target="_blank"
            rel="noreferrer"
            className="ml-auto flex items-center gap-1 text-muted-foreground hover:text-primary"
          >
            <ExternalLink className="h-3 w-3" />
            official source
          </a>
        </div>
      </div>
    </header>
  )
}

function Metric({
  label,
  value,
  accent,
  warn,
  fail,
}: {
  label: string
  value: string | number
  accent?: boolean
  warn?: boolean
  fail?: boolean
}) {
  const valueCls = fail
    ? "text-destructive"
    : warn
      ? "text-warning"
      : accent
        ? "text-accent"
        : "text-foreground"
  return (
    <div className="flex items-baseline gap-1.5">
      <span className="text-muted-foreground uppercase tracking-wide">{label}</span>
      <span className={`font-semibold ${valueCls}`}>{value}</span>
    </div>
  )
}

function countProvisions(data: StatutePageResponse): number {
  let count = 0
  function walk(p: any) {
    count++
    if (p.children) p.children.forEach(walk)
  }
  data.provisions.forEach(walk)
  return count
}
