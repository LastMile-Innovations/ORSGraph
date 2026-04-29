import Link from "next/link"
import type { StatutePageResponse } from "@/lib/types"
import { ArrowDownRight, ArrowUpLeft, AlertTriangle } from "lucide-react"

export function CitationsTab({ data }: { data: StatutePageResponse }) {
  return (
    <div className="grid grid-cols-1 gap-px bg-border lg:grid-cols-2">
      {/* Outbound */}
      <section className="bg-card">
        <header className="flex items-center justify-between border-b border-border px-4 py-2.5">
          <h3 className="flex items-center gap-2 font-mono text-xs uppercase tracking-widest text-muted-foreground">
            <ArrowDownRight className="h-3.5 w-3.5 text-accent" />
            outbound · this statute cites
          </h3>
          <span className="font-mono text-[11px] tabular-nums text-foreground">
            {data.outbound_citations.length}
          </span>
        </header>
        <ul className="divide-y divide-border">
          {data.outbound_citations.map((c, i) => (
            <li key={i} className="px-4 py-3 hover:bg-muted/50">
              <div className="flex items-center gap-2">
                {c.target_canonical_id ? (
                  <Link
                    href={`/statutes/${c.target_canonical_id}`}
                    className="font-mono text-sm font-medium text-primary hover:underline"
                  >
                    {c.target_citation}
                  </Link>
                ) : (
                  <span className="font-mono text-sm text-warning">{c.target_citation}</span>
                )}
                <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                  from {c.source_provision}
                </span>
                {!c.resolved && (
                  <span className="ml-auto inline-flex items-center gap-1 rounded bg-warning/15 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-warning">
                    <AlertTriangle className="h-3 w-3" />
                    unresolved
                  </span>
                )}
              </div>
              <p className="mt-1 text-sm text-foreground">
                <span className="text-muted-foreground">…</span>
                {c.context_snippet}
                <span className="text-muted-foreground">…</span>
              </p>
            </li>
          ))}
        </ul>
      </section>

      {/* Inbound */}
      <section className="bg-card">
        <header className="flex items-center justify-between border-b border-border px-4 py-2.5">
          <h3 className="flex items-center gap-2 font-mono text-xs uppercase tracking-widest text-muted-foreground">
            <ArrowUpLeft className="h-3.5 w-3.5 text-accent" />
            inbound · this statute is cited by
          </h3>
          <span className="font-mono text-[11px] tabular-nums text-foreground">
            {data.inbound_citations.length}
          </span>
        </header>
        <ul className="divide-y divide-border">
          {data.inbound_citations.map((c, i) => (
            <li key={i} className="px-4 py-3 hover:bg-muted/50">
              <div className="flex items-center gap-2">
                <Link
                  href={`/statutes/${c.source_canonical_id}`}
                  className="font-mono text-sm font-medium text-primary hover:underline"
                >
                  {c.source_citation}
                </Link>
                <span className="text-sm text-foreground">{c.source_title}</span>
              </div>
              <div className="mt-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                from {c.source_provision}
              </div>
              <p className="mt-1 text-sm text-foreground">
                <span className="text-muted-foreground">…</span>
                {c.context_snippet}
                <span className="text-muted-foreground">…</span>
              </p>
            </li>
          ))}
        </ul>
      </section>
    </div>
  )
}
