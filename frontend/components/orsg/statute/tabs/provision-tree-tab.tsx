"use client"

import { useState } from "react"
import Link from "next/link"
import type { Provision, StatutePageResponse } from "@/lib/types"
import { cn } from "@/lib/utils"
import { ChevronDown, ChevronRight, ExternalLink } from "lucide-react"
import { QCBadge, SignalBadge } from "@/components/orsg/badges"

export function ProvisionTreeTab({ data }: { data: StatutePageResponse }) {
  return (
    <div className="px-6 py-6">
      <div className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        Provision tree · {data.identity.citation}
      </div>
      <div className="rounded border border-border bg-card">
        {data.provisions.map((p) => (
          <ProvisionRow key={p.provision_id} provision={p} depth={0} />
        ))}
      </div>
    </div>
  )
}

function ProvisionRow({ provision, depth }: { provision: Provision; depth: number }) {
  const [open, setOpen] = useState(depth < 1)
  const hasChildren = !!provision.children?.length

  return (
    <div>
      <div
        className={cn(
          "flex items-start gap-2 border-b border-border px-3 py-2.5 transition-colors hover:bg-muted/50",
          depth > 0 && "border-l-2 border-l-border bg-card",
        )}
        style={{ paddingLeft: `${0.75 + depth * 1.25}rem` }}
      >
        <button
          onClick={() => setOpen(!open)}
          className={cn(
            "mt-0.5 flex h-4 w-4 flex-none items-center justify-center rounded text-muted-foreground hover:text-foreground",
            !hasChildren && "invisible",
          )}
          aria-label={open ? "Collapse" : "Expand"}
        >
          {open ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
        </button>

        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <Link
              href={`/provisions/${encodeURIComponent(provision.provision_id)}`}
              className="font-mono text-sm font-medium text-primary hover:underline"
            >
              {provision.display_citation}
            </Link>
            <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
              {provision.provision_type}
            </span>
            {provision.signals.map((s) => (
              <SignalBadge key={s} signal={s} />
            ))}
            <QCBadge status={provision.qc_status} />
          </div>
          <p className="mt-1 line-clamp-2 text-sm text-foreground">{provision.text_preview}</p>
          <div className="mt-1.5 flex items-center gap-3 font-mono text-[10px] tabular-nums text-muted-foreground">
            <span>cites: <span className="text-foreground">{provision.cites_count}</span></span>
            <span>cited by: <span className="text-accent">{provision.cited_by_count}</span></span>
            <span>chunks: <span className="text-foreground">{provision.chunk_count}</span></span>
            <Link
              href={`/provisions/${encodeURIComponent(provision.provision_id)}`}
              className="ml-auto flex items-center gap-1 hover:text-primary"
            >
              <ExternalLink className="h-3 w-3" />
              inspect
            </Link>
          </div>
        </div>
      </div>

      {open && hasChildren && (
        <div>
          {provision.children!.map((c) => (
            <ProvisionRow key={c.provision_id} provision={c} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
  )
}
