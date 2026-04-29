"use client"

import { useMemo, useState } from "react"
import Link from "next/link"
import type { SourceIndexEntry } from "@/lib/types"
import { AlertTriangle, CheckCircle2, Clock, Database, ExternalLink, Search, XCircle } from "lucide-react"
import { cn } from "@/lib/utils"

type StatusFilter = "all" | "ingested" | "queued" | "failed" | "warnings"

export function SourcesClient({ sources }: { sources: SourceIndexEntry[] }) {
  const [query, setQuery] = useState("")
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all")
  const [editionFilter, setEditionFilter] = useState<number | "all">("all")

  const editions = useMemo(
    () => Array.from(new Set(sources.map((s) => s.edition_year))).sort((a, b) => b - a),
    [sources],
  )

  const filtered = useMemo(() => {
    return sources.filter((s) => {
      if (query) {
        const q = query.toLowerCase()
        if (
          !s.title.toLowerCase().includes(q) &&
          !s.scope.toLowerCase().includes(q) &&
          !s.source_id.toLowerCase().includes(q)
        )
          return false
      }
      if (statusFilter === "warnings" && s.parser_warnings.length === 0) return false
      if (statusFilter !== "all" && statusFilter !== "warnings" && s.ingestion_status !== statusFilter) return false
      if (editionFilter !== "all" && s.edition_year !== editionFilter) return false
      return true
    })
  }, [sources, query, statusFilter, editionFilter])

  const stats = useMemo(() => {
    return {
      total: sources.length,
      ingested: sources.filter((s) => s.ingestion_status === "ingested").length,
      queued: sources.filter((s) => s.ingestion_status === "queued").length,
      failed: sources.filter((s) => s.ingestion_status === "failed").length,
      warnings: sources.filter((s) => s.parser_warnings.length > 0).length,
      total_bytes: sources.reduce((acc, s) => acc + s.byte_size, 0),
    }
  }, [sources])

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {/* Header */}
      <div className="flex flex-col gap-3 border-b border-border bg-card px-6 py-4 md:flex-row md:items-center md:justify-between">
        <div>
          <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <Database className="h-3 w-3" />
            sources / corpus index
          </div>
          <h1 className="mt-1 text-xl font-semibold">Source Documents</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">
            Every statute, provision, and chunk traces to one of these documents.
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-3 font-mono text-[10px] tabular-nums">
          <Stat label="total" value={stats.total} />
          <Stat label="ingested" value={stats.ingested} tone="success" />
          <Stat label="queued" value={stats.queued} tone="warning" />
          <Stat label="failed" value={stats.failed} tone="fail" />
          <Stat label="size" value={formatBytes(stats.total_bytes)} />
        </div>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap items-center gap-2 border-b border-border bg-background px-6 py-3">
        <div className="relative flex-1 min-w-[16rem]">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="filter by title, scope, or source_id..."
            className="h-8 w-full rounded border border-border bg-card pl-8 pr-3 font-mono text-xs text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none"
          />
        </div>

        <FilterPill active={statusFilter === "all"} onClick={() => setStatusFilter("all")}>
          all
        </FilterPill>
        <FilterPill active={statusFilter === "ingested"} onClick={() => setStatusFilter("ingested")}>
          ingested
        </FilterPill>
        <FilterPill active={statusFilter === "queued"} onClick={() => setStatusFilter("queued")}>
          queued
        </FilterPill>
        <FilterPill active={statusFilter === "failed"} onClick={() => setStatusFilter("failed")}>
          failed
        </FilterPill>
        <FilterPill active={statusFilter === "warnings"} onClick={() => setStatusFilter("warnings")}>
          warnings
        </FilterPill>

        <div className="ml-auto flex items-center gap-2">
          <span className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">edition</span>
          <select
            value={editionFilter}
            onChange={(e) =>
              setEditionFilter(e.target.value === "all" ? "all" : Number.parseInt(e.target.value, 10))
            }
            className="h-7 rounded border border-border bg-card px-2 font-mono text-xs"
          >
            <option value="all">all</option>
            {editions.map((y) => (
              <option key={y} value={y}>
                {y}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-auto">
        <table className="w-full text-xs">
          <thead className="sticky top-0 bg-card">
            <tr className="border-b border-border font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              <th className="px-4 py-2 text-left">source_id</th>
              <th className="px-4 py-2 text-left">title</th>
              <th className="px-4 py-2 text-left">scope</th>
              <th className="px-4 py-2 text-right">edition</th>
              <th className="px-4 py-2 text-right">size</th>
              <th className="px-4 py-2 text-left">retrieved</th>
              <th className="px-4 py-2 text-left">parser</th>
              <th className="px-4 py-2 text-left">status</th>
              <th className="px-4 py-2"></th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((s) => (
              <tr
                key={s.source_id}
                className="border-b border-border transition-colors hover:bg-muted/40"
              >
                <td className="px-4 py-2 font-mono text-[11px] text-muted-foreground">
                  <Link href={`/sources/${encodeURIComponent(s.source_id)}`} className="hover:text-primary">
                    {s.source_id.replace("src:ors:", "ors:").slice(0, 28)}
                  </Link>
                </td>
                <td className="max-w-[20rem] px-4 py-2">
                  <Link
                    href={`/sources/${encodeURIComponent(s.source_id)}`}
                    className="line-clamp-1 text-foreground hover:text-primary"
                  >
                    {s.title}
                  </Link>
                </td>
                <td className="px-4 py-2 font-mono text-[11px] text-muted-foreground">{s.scope}</td>
                <td className="px-4 py-2 text-right font-mono tabular-nums">{s.edition_year}</td>
                <td className="px-4 py-2 text-right font-mono tabular-nums text-muted-foreground">
                  {s.byte_size > 0 ? formatBytes(s.byte_size) : "—"}
                </td>
                <td className="px-4 py-2 font-mono text-[11px] tabular-nums text-muted-foreground">
                  {formatDate(s.retrieved_at)}
                </td>
                <td className="px-4 py-2 font-mono text-[11px] text-muted-foreground">{s.parser_profile}</td>
                <td className="px-4 py-2">
                  <IngestionPill source={s} />
                </td>
                <td className="px-4 py-2 text-right">
                  <a
                    href={s.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:text-primary"
                    onClick={(e) => e.stopPropagation()}
                  >
                    raw
                    <ExternalLink className="h-3 w-3" />
                  </a>
                </td>
              </tr>
            ))}
            {filtered.length === 0 && (
              <tr>
                <td colSpan={9} className="px-4 py-12 text-center font-mono text-xs text-muted-foreground">
                  no sources match these filters.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}

function Stat({ label, value, tone }: { label: string; value: number | string; tone?: "success" | "warning" | "fail" }) {
  return (
    <div className="flex flex-col">
      <span className="text-[10px] uppercase tracking-wider text-muted-foreground">{label}</span>
      <span
        className={cn(
          "text-base tabular-nums",
          tone === "success" && "text-success",
          tone === "warning" && "text-warning",
          tone === "fail" && "text-destructive",
        )}
      >
        {value}
      </span>
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
        "rounded px-2 py-1 font-mono text-[10px] uppercase tracking-wider transition-colors",
        active
          ? "bg-primary/10 text-primary"
          : "border border-border text-muted-foreground hover:border-primary hover:text-foreground",
      )}
    >
      {children}
    </button>
  )
}

function IngestionPill({ source }: { source: SourceIndexEntry }) {
  if (source.ingestion_status === "failed") {
    return (
      <span className="inline-flex items-center gap-1 rounded bg-destructive/15 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-destructive">
        <XCircle className="h-3 w-3" />
        failed
      </span>
    )
  }
  if (source.ingestion_status === "queued") {
    return (
      <span className="inline-flex items-center gap-1 rounded bg-warning/15 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-warning">
        <Clock className="h-3 w-3" />
        queued
      </span>
    )
  }
  if (source.parser_warnings.length > 0) {
    return (
      <span className="inline-flex items-center gap-1 rounded bg-warning/15 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-warning">
        <AlertTriangle className="h-3 w-3" />
        {source.parser_warnings.length} warn
      </span>
    )
  }
  return (
    <span className="inline-flex items-center gap-1 rounded bg-success/15 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-success">
      <CheckCircle2 className="h-3 w-3" />
      ingested
    </span>
  )
}

function formatBytes(n: number) {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`
  return `${(n / (1024 * 1024)).toFixed(1)} MB`
}

function formatDate(iso: string) {
  try {
    const d = new Date(iso)
    return d.toISOString().slice(0, 10)
  } catch {
    return iso
  }
}
