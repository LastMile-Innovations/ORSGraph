"use client"

import Link from "next/link"
import type { StatutePageResponse } from "@/lib/types"
import { Type, ShieldAlert, Clock, Scale, ArrowDownRight, ArrowUpLeft, AlertTriangle, FileText } from "lucide-react"
import { QCBadge } from "@/components/orsg/badges"

export function StatuteRightInspector({ data }: { data: StatutePageResponse }) {
  return (
    <div className="flex h-full flex-col overflow-y-auto scrollbar-thin">
      <div className="border-b border-border px-4 py-3">
        <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          intelligence panel
        </h2>
        <p className="mt-1 text-sm text-foreground">{data.identity.citation}</p>
      </div>

      <Panel
        title="Definitions"
        icon={Type}
        count={data.definitions.length}
        accent="text-chart-1"
      >
        {data.definitions.length === 0 ? (
          <Empty>No definitions</Empty>
        ) : (
          <ul className="space-y-2">
            {data.definitions.map((d) => (
              <li key={d.definition_id} className="text-xs">
                <span className="font-serif italic text-foreground">"{d.term}"</span>
                <p className="mt-0.5 line-clamp-2 text-muted-foreground">{d.text}</p>
                <span className="mt-0.5 block font-mono text-[10px] text-primary">
                  {d.source_provision}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Panel>

      <Panel
        title="Exceptions"
        icon={ShieldAlert}
        count={data.exceptions.length}
        accent="text-warning"
      >
        {data.exceptions.length === 0 ? (
          <Empty>No exceptions</Empty>
        ) : (
          <ul className="space-y-2">
            {data.exceptions.map((e) => (
              <li key={e.exception_id} className="text-xs">
                <p className="line-clamp-3 text-foreground">{e.text}</p>
                <span className="mt-0.5 block font-mono text-[10px] text-primary">
                  {e.source_provision}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Panel>

      <Panel
        title="Deadlines"
        icon={Clock}
        count={data.deadlines.length}
        accent="text-chart-3"
      >
        {data.deadlines.length === 0 ? (
          <Empty>No deadlines</Empty>
        ) : (
          <ul className="space-y-2">
            {data.deadlines.map((d) => (
              <li key={d.deadline_id} className="text-xs">
                <div className="flex items-baseline gap-2">
                  <span className="font-mono text-sm font-semibold text-chart-3">{d.duration}</span>
                  <span className="text-foreground">{d.description}</span>
                </div>
                <p className="mt-0.5 line-clamp-2 text-muted-foreground">{d.trigger}</p>
                <span className="mt-0.5 block font-mono text-[10px] text-primary">
                  {d.source_provision}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Panel>

      <Panel title="Penalties" icon={Scale} count={data.penalties.length} accent="text-destructive">
        {data.penalties.length === 0 ? (
          <Empty>No penalties</Empty>
        ) : (
          <ul className="space-y-2">
            {data.penalties.map((p) => (
              <li key={p.penalty_id} className="text-xs">
                <span className="mr-1.5 inline-block rounded bg-destructive/15 px-1.5 py-0.5 font-mono text-[10px] uppercase text-destructive">
                  {p.category}
                </span>
                <span className="text-foreground">{p.description}</span>
                <span className="mt-0.5 block font-mono text-[10px] text-primary">
                  {p.source_provision}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Panel>

      <Panel
        title="Cites"
        icon={ArrowDownRight}
        count={data.outbound_citations.length}
        accent="text-accent"
      >
        <ul className="space-y-1">
          {data.outbound_citations.slice(0, 6).map((c, i) => (
            <li key={i} className="flex items-center gap-2 text-xs">
              {c.target_canonical_id ? (
                <Link
                  href={`/statutes/${c.target_canonical_id}`}
                  className="font-mono text-primary hover:underline"
                >
                  {c.target_citation}
                </Link>
              ) : (
                <>
                  <AlertTriangle className="h-3 w-3 text-warning" />
                  <span className="font-mono text-warning">{c.target_citation}</span>
                </>
              )}
            </li>
          ))}
        </ul>
      </Panel>

      <Panel
        title="Cited by"
        icon={ArrowUpLeft}
        count={data.inbound_citations.length}
        accent="text-accent"
      >
        <ul className="space-y-1">
          {data.inbound_citations.slice(0, 6).map((c, i) => (
            <li key={i} className="text-xs">
              <Link
                href={`/statutes/${c.source_canonical_id}`}
                className="font-mono text-primary hover:underline"
              >
                {c.source_citation}
              </Link>
              <p className="line-clamp-1 text-muted-foreground">{c.source_title}</p>
            </li>
          ))}
        </ul>
      </Panel>

      <Panel title="Chunks" icon={FileText} count={data.chunks.length}>
        <ul className="space-y-1 font-mono text-[10px] uppercase tracking-wide">
          {Object.entries(
            data.chunks.reduce<Record<string, number>>((acc, c) => {
              acc[c.chunk_type] = (acc[c.chunk_type] || 0) + 1
              return acc
            }, {}),
          ).map(([type, count]) => (
            <li key={type} className="flex items-center justify-between">
              <span className="text-muted-foreground">{type.replace(/_/g, " ")}</span>
              <span className="tabular-nums text-foreground">{count}</span>
            </li>
          ))}
        </ul>
      </Panel>

      <div className="border-t border-border px-4 py-3">
        <h3 className="mb-2 flex items-center justify-between font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          <span>QC notes</span>
          <QCBadge status={data.qc.status} />
        </h3>
        {data.qc.notes.length === 0 ? (
          <Empty>All checks passed</Empty>
        ) : (
          <ul className="space-y-1.5">
            {data.qc.notes.map((n) => (
              <li key={n.note_id} className="flex items-start gap-1.5 text-xs">
                <AlertTriangle
                  className={`mt-0.5 h-3 w-3 flex-none ${
                    n.level === "fail" ? "text-destructive" : "text-warning"
                  }`}
                />
                <span className="text-muted-foreground">{n.message}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}

function Panel({
  title,
  icon: Icon,
  count,
  accent = "text-muted-foreground",
  children,
}: {
  title: string
  icon: typeof Type
  count: number
  accent?: string
  children: React.ReactNode
}) {
  return (
    <section className="border-b border-border px-4 py-3">
      <h3 className="mb-2 flex items-center justify-between font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        <span className="flex items-center gap-1.5">
          <Icon className={`h-3 w-3 ${accent}`} />
          {title}
        </span>
        <span className="tabular-nums">{count}</span>
      </h3>
      {children}
    </section>
  )
}

function Empty({ children }: { children: React.ReactNode }) {
  return <p className="text-xs text-muted-foreground">{children}</p>
}
