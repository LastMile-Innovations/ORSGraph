"use client"

import Link from "next/link"
import type { AuthoritySuggestion } from "@/lib/types"
import { authorityBadges, authorityReason } from "@/lib/authority-taxonomy"
import { StatusBadge, SignalBadge } from "./badges"
import { ArrowRight, Plus } from "lucide-react"

interface Props {
  authority: AuthoritySuggestion
  onAdd?: (a: AuthoritySuggestion) => void
  showAddButton?: boolean
  className?: string
}

export function AuthorityCard({ authority, onAdd, showAddButton = false, className }: Props) {
  return (
    <div
      className={`group rounded border border-border bg-card p-3 transition-colors hover:border-primary/40 ${className ?? ""}`}
    >
      <div className="mb-1.5 flex items-start justify-between gap-2">
        <div className="min-w-0">
          <Link
            href={`/statutes/${authority.canonical_id}`}
            className="font-mono text-sm font-medium text-primary hover:underline"
          >
            {authority.citation}
          </Link>
          <h3 className="mt-0.5 line-clamp-1 text-sm leading-snug text-foreground">
            {authority.title}
          </h3>
        </div>
        <div className="flex flex-shrink-0 items-center gap-1">
          <StatusBadge status={authority.status} />
        </div>
      </div>

      <p className="mb-2 line-clamp-2 font-serif text-[13px] leading-snug text-muted-foreground">
        {authority.snippet}
      </p>

      <div className="flex flex-wrap items-center gap-1.5">
        {authorityBadges(authority).map((badge) => (
          <span
            key={badge}
            className="rounded border border-primary/20 bg-primary/5 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-primary"
          >
            {badge}
          </span>
        ))}
        {authority.signals.map((s) => (
          <SignalBadge key={s} signal={s} />
        ))}
      </div>

      <div className="mt-2 flex items-center justify-between border-t border-border pt-2">
        <div className="flex items-center gap-3 font-mono text-[10px] tabular-nums text-muted-foreground">
          <span>{authorityReason(authority)}</span>
          <span className="text-border">|</span>
          <span>edition {authority.edition_year}</span>
          <span className="text-border">|</span>
          <span>cites {authority.cites_count}</span>
          <span className="text-border">|</span>
          <span>cited-by {authority.cited_by_count}</span>
        </div>
        <div className="flex items-center gap-1">
          {showAddButton && onAdd && (
            <button
              onClick={() => onAdd(authority)}
              className="flex items-center gap-1 rounded border border-border px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:border-primary hover:text-primary"
            >
              <Plus className="h-3 w-3" />
              insert
            </button>
          )}
          <Link
            href={`/statutes/${authority.canonical_id}`}
            className="flex items-center gap-1 rounded px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:text-primary"
          >
            open
            <ArrowRight className="h-3 w-3" />
          </Link>
        </div>
      </div>
    </div>
  )
}
