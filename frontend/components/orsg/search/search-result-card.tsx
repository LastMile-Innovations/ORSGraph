"use client"

import Link from "next/link"
import { useState } from "react"
import {
  Check,
  Copy,
  ExternalLink,
  GitBranch,
  MessageSquare,
  Quote,
  Scale,
} from "lucide-react"
import type { LegalStatus, NullableNumber, SearchResult } from "@/lib/types"
import { authorityBadges, authorityReason, formatAuthorityTier } from "@/lib/authority-taxonomy"
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
  const hierarchyBadges = authorityBadges(result)
  const scoreBreakdown = result.score_breakdown
  const scoreParts = [
    ["exact", scoreBreakdown?.exact],
    ["text", result.fulltext_score ?? scoreBreakdown?.keyword],
    ["vector", result.vector_score ?? scoreBreakdown?.vector],
    ["graph", result.graph_score ?? scoreBreakdown?.graph],
    ["expand", scoreBreakdown?.expansion],
    ["rerank", result.rerank_score ?? scoreBreakdown?.rerank],
  ] as const
  const copyValue = result.citation ?? identity
  const resultScore = finiteNumber(result.score)
  const status = legalStatus(result.status)
  const authorityExplanation = result.source_role ? authorityReason(result) : ""
  const citedByCount = finiteNumber(result.graph?.cited_by_count)
  const citationCount = finiteNumber(result.graph?.citation_count)
  const connectedNodeCount = finiteNumber(result.graph?.connected_node_count)

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
            {status && <StatusBadge status={status} />}
            {result.source_backed && <SourceBadge />}
            {hierarchyBadges.map((badge) => (
              <AuthorityBadge key={badge} label={badge} />
            ))}
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

          <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
            {result.authority_level != null && (
              <span>
                authority <span className="text-foreground">{result.authority_level}</span>
              </span>
            )}
            {result.authority_tier && (
              <span>{formatAuthorityTier(result.authority_tier)}</span>
            )}
            {authorityExplanation && (
              <span title={authorityExplanation}>{authorityExplanation}</span>
            )}
            {citedByCount !== undefined && (
              <span className="inline-flex items-center gap-1">
                <Scale className="h-3 w-3" />
                cited by <span className="text-accent">{citedByCount}</span>
              </span>
            )}
            {citationCount !== undefined && (
              <span className="inline-flex items-center gap-1">
                <Quote className="h-3 w-3" />
                cites <span className="text-foreground">{citationCount}</span>
              </span>
            )}
          </div>

          <div className="mt-3 flex flex-wrap items-center gap-2">
            <ScorePill label="relevance" value={resultScore} strong />

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
          {(scoreParts.some(([, value]) => finiteNumber(value) !== undefined) || connectedNodeCount !== undefined || result.source?.provision_id || result.source?.chunk_id) && (
            <details className="mt-2 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
              <summary className="cursor-pointer hover:text-foreground">Advanced details</summary>
              <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1">
                {scoreParts.map(([label, value]) => (
                  <ScorePill key={label} label={label} value={value} />
                ))}
                {connectedNodeCount !== undefined && (
                  <span className="inline-flex items-center gap-1">
                    <GitBranch className="h-3 w-3" />
                    graph nodes <span className="text-foreground">{connectedNodeCount}</span>
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
            </details>
          )}
        </div>

        <div className="hidden h-16 w-1 flex-none overflow-hidden rounded-full bg-muted lg:block">
          <div
            className="w-full bg-primary transition-all"
            style={{ height: `${scoreBarHeight(resultScore)}%` }}
          />
        </div>
      </div>
    </article>
  )
}

function ScorePill({ label, value, strong = false }: { label: string; value?: NullableNumber; strong?: boolean }) {
  const score = finiteNumber(value)
  if (score === undefined) return null
  return (
    <span
      className={cn(
        "rounded border px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
        strong ? "border-primary/30 bg-primary/10 text-primary" : "border-border text-muted-foreground",
      )}
    >
      {label} <span className="text-foreground">{score.toFixed(2)}</span>
    </span>
  )
}

function finiteNumber(value: NullableNumber | undefined) {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined
}

function scoreBarHeight(score: number | undefined) {
  if (score === undefined) return 0
  return Math.min(100, Math.max(8, (score / 10) * 100))
}

const LEGAL_STATUS_VALUES = new Set<string>(["active", "repealed", "renumbered", "amended"])

function legalStatus(value: string | null | undefined): LegalStatus | null {
  return value && LEGAL_STATUS_VALUES.has(value) ? (value as LegalStatus) : null
}

function AuthorityBadge({ label }: { label: string }) {
  return (
    <span className="rounded border border-primary/20 bg-primary/5 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-primary">
      {label}
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
