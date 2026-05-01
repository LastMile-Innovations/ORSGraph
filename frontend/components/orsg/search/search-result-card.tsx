"use client"

import Link from "next/link"
import { useState } from "react"
import {
  AlertTriangle,
  Check,
  Copy,
  ExternalLink,
  GitBranch,
  MessageSquare,
  Quote,
  Scale,
} from "lucide-react"
import type { SearchResult } from "@/lib/types"
import { StatusBadge, SemanticBadge, SourceBadge } from "@/components/orsg/badges"
import { cn } from "@/lib/utils"

interface SearchResultCardProps {
  result: SearchResult
}

export function SearchResultCard({ result }: SearchResultCardProps) {
  const [copied, setCopied] = useState(false)
  const identity = result.citation ?? result.id ?? result.source_id ?? result.source_provision ?? "result"
  const href = result.href || `/statutes/${encodeURIComponent(identity)}`
  const kind = result.kind ?? result.result_type ?? "result"
  const semanticTypes = result.semantic_types ?? []
  const qcWarnings = result.qc_warnings ?? []
  const scoreParts = [
    ["exact", result.score_breakdown?.exact],
    ["text", result.fulltext_score ?? result.score_breakdown?.keyword],
    ["vector", result.vector_score ?? result.score_breakdown?.vector],
    ["graph", result.graph_score ?? result.score_breakdown?.graph],
    ["expand", result.score_breakdown?.expansion],
    ["rerank", result.rerank_score ?? result.score_breakdown?.rerank],
  ] as const
  const copyValue = result.citation ?? identity

  const copyCitation = async () => {
    try {
      await navigator.clipboard.writeText(copyValue)
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1200)
    } catch (error) {
      console.info("Copy citation failed", error)
    }
  }

  return (
    <article className="group border-b border-border px-6 py-4 transition-colors hover:bg-muted/30">
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="rounded border border-border bg-muted/40 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
              {formatKind(kind)}
            </span>
            {result.rank_source && (
              <span className={cn(
                "rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
                result.rank_source === "exact"
                  ? "bg-primary/15 text-primary"
                  : "bg-accent/10 text-accent",
              )}>
                {result.rank_source}
              </span>
            )}
            <Link href={href} className="font-mono text-base font-semibold text-primary hover:underline">
              {identity}
            </Link>
            {result.title && <span className="text-sm font-medium text-foreground">{result.title}</span>}
            {result.chapter && (
              <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                Chapter {result.chapter}
              </span>
            )}
            <StatusBadge status={result.status as any} />
            {result.source_backed && <SourceBadge />}
          </div>

          {semanticTypes.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1.5">
              {semanticTypes.map((type) => (
                <SemanticBadge key={type} type={type} />
              ))}
            </div>
          )}

          <p className="mt-2 max-w-5xl text-sm leading-relaxed text-foreground">
            {result.snippet}
          </p>

          {qcWarnings.length > 0 && (
            <div className="mt-2 flex items-center gap-1.5 text-[10px] uppercase tracking-wide text-warning">
              <AlertTriangle className="h-3 w-3" />
              {qcWarnings.join(", ")}
            </div>
          )}

          <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
            {result.graph?.cited_by_count !== undefined && (
              <span className="inline-flex items-center gap-1">
                <Scale className="h-3 w-3" />
                cited by <span className="text-accent">{result.graph.cited_by_count}</span>
              </span>
            )}
            {result.graph?.citation_count !== undefined && (
              <span className="inline-flex items-center gap-1">
                <Quote className="h-3 w-3" />
                cites <span className="text-foreground">{result.graph.citation_count}</span>
              </span>
            )}
            {result.graph?.connected_node_count !== undefined && (
              <span className="inline-flex items-center gap-1">
                <GitBranch className="h-3 w-3" />
                graph nodes <span className="text-foreground">{result.graph.connected_node_count}</span>
              </span>
            )}
            {result.source?.provision_id && (
              <span title={result.source.provision_id}>
                provision <span className="text-foreground">{shortId(result.source.provision_id)}</span>
              </span>
            )}
            {result.source?.chunk_id && (
              <span title={result.source.chunk_id}>
                chunk <span className="text-foreground">{shortId(result.source.chunk_id)}</span>
              </span>
            )}
          </div>

          <div className="mt-3 flex flex-wrap items-center gap-2">
            <ScorePill label="final" value={result.score} strong />
            {scoreParts.map(([label, value]) => (
              <ScorePill key={label} label={label} value={value} />
            ))}

            <div className="ml-auto flex items-center gap-3 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
              <Link href={href} className="flex items-center gap-1 transition-colors hover:text-primary">
                <ExternalLink className="h-3 w-3" /> open
              </Link>
              <button
                type="button"
                onClick={copyCitation}
                className="flex items-center gap-1 transition-colors hover:text-primary"
                title="Copy citation"
              >
                {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
                {copied ? "copied" : "copy"}
              </button>
              <Link
                href={`/ask?q=${encodeURIComponent(`${identity} ${result.title || ""}`)}`}
                className="flex items-center gap-1 transition-colors hover:text-primary"
              >
                <MessageSquare className="h-3 w-3" /> ask
              </Link>
            </div>
          </div>
        </div>

        <div className="hidden h-16 w-1 flex-none overflow-hidden rounded-full bg-muted lg:block">
          <div
            className="w-full bg-primary transition-all"
            style={{ height: `${Math.min(100, Math.max(8, (result.score / 10) * 100))}%` }}
          />
        </div>
      </div>
    </article>
  )
}

function ScorePill({ label, value, strong = false }: { label: string; value?: number; strong?: boolean }) {
  if (value === undefined || Number.isNaN(value)) return null
  return (
    <span
      className={cn(
        "rounded border px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
        strong ? "border-primary/30 bg-primary/10 text-primary" : "border-border text-muted-foreground",
      )}
    >
      {label} <span className="text-foreground">{value.toFixed(2)}</span>
    </span>
  )
}

function shortId(value: string) {
  if (value.length <= 28) return value
  return `${value.slice(0, 12)}...${value.slice(-10)}`
}

function formatKind(kind: string) {
  return kind
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/_/g, " ")
    .toLowerCase()
}
