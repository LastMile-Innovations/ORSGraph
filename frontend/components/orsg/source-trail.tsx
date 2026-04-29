import type { SourceDocument } from "@/lib/types"
import { ExternalLink, AlertTriangle } from "lucide-react"
import { cn } from "@/lib/utils"

interface Props {
  source: SourceDocument
  compact?: boolean
  className?: string
}

export function SourceTrail({ source, compact = false, className }: Props) {
  return (
    <div className={cn("rounded border border-border bg-card", className)}>
      <div className="flex items-center justify-between border-b border-border px-3 py-1.5">
        <span className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
          source trail
        </span>
        <span className="font-mono text-[10px] tabular-nums text-muted-foreground">
          {source.edition_year}
        </span>
      </div>
      <div className="space-y-1.5 p-3 font-mono text-[11px] tabular-nums">
        <Row label="source_id" value={source.source_id} mono />
        <Row
          label="url"
          value={
            <a
              href={source.url}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-primary hover:underline"
            >
              <span className="max-w-[18ch] truncate">{prettyHost(source.url)}</span>
              <ExternalLink className="h-3 w-3" />
            </a>
          }
        />
        <Row label="retrieved_at" value={formatDate(source.retrieved_at)} />
        {!compact && (
          <>
            <Row label="raw_hash" value={shortHash(source.raw_hash)} mono />
            <Row label="normalized_hash" value={shortHash(source.normalized_hash)} mono />
          </>
        )}
        <Row label="parser_profile" value={source.parser_profile} mono />
        {source.parser_warnings.length > 0 && (
          <div className="mt-2 rounded border border-warning/30 bg-warning/10 p-2">
            <div className="mb-1 flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-warning">
              <AlertTriangle className="h-3 w-3" />
              parser warnings
            </div>
            <ul className="space-y-0.5 pl-1 text-[11px] text-foreground">
              {source.parser_warnings.map((w, i) => (
                <li key={i}>• {w}</li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  )
}

function Row({
  label,
  value,
  mono,
}: {
  label: string
  value: React.ReactNode
  mono?: boolean
}) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <span className="text-muted-foreground">{label}</span>
      <span className={cn("text-right text-foreground", mono && "font-mono")}>{value}</span>
    </div>
  )
}

function shortHash(h: string) {
  if (!h) return "—"
  const noPrefix = h.replace(/^[a-z0-9]+:/i, "")
  return `${h.split(":")[0] ?? ""}:${noPrefix.slice(0, 8)}…${noPrefix.slice(-4)}`
}

function prettyHost(url: string) {
  try {
    const u = new URL(url)
    return u.host + (u.pathname.length > 1 ? u.pathname.slice(0, 24) : "")
  } catch {
    return url
  }
}

function formatDate(iso: string) {
  try {
    const d = new Date(iso)
    return d.toISOString().replace("T", " ").slice(0, 16) + "Z"
  } catch {
    return iso
  }
}
