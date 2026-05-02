"use client"

import Link from "next/link"
import { useState } from "react"
import { ArrowLeft, ChevronRight, FileText, GitBranch, Hash, ScrollText, Sparkles } from "lucide-react"
import { Button } from "@/components/ui/button"
import { StatusBadge, QCBadge, SignalBadge, ChunkTypeBadge } from "@/components/orsg/badges"
import type { ProvisionInspectorData } from "@/lib/types"

const TABS = [
  { id: "text", label: "Text" },
  { id: "chunks", label: "Chunks" },
  { id: "citations", label: "Citations" },
  { id: "definitions", label: "Definitions" },
  { id: "exceptions", label: "Exceptions" },
  { id: "deadlines", label: "Deadlines" },
  { id: "qc", label: "QC" },
] as const

type TabId = (typeof TABS)[number]["id"]

export function ProvisionInspectorClient({ data }: { data: ProvisionInspectorData }) {
  const [tab, setTab] = useState<TabId>("text")
  const p = data.provision

  return (
    <div className="flex h-full">
      {/* Main pane */}
      <div className="flex-1 min-w-0 flex flex-col overflow-hidden">
        {/* Header */}
        <div className="border-b border-border bg-card px-6 py-4">
          <div className="flex items-center gap-2 text-xs text-muted-foreground font-mono mb-2">
            <Link href="/dashboard" className="hover:text-foreground">orsgraph</Link>
            <ChevronRight className="h-3 w-3" />
            <Link href={`/statutes/${data.parent_statute.canonical_id}`} className="hover:text-foreground">
              {data.parent_statute.citation}
            </Link>
            {data.ancestors.map((a) => (
              <span key={a.provision_id} className="flex items-center gap-2">
                <ChevronRight className="h-3 w-3" />
                <Link
                  href={`/provisions/${encodeURIComponent(a.provision_id)}`}
                  className="hover:text-foreground"
                >
                  {a.citation}
                </Link>
              </span>
            ))}
            <ChevronRight className="h-3 w-3" />
            <span className="text-foreground">{p.display_citation}</span>
          </div>

          <div className="flex items-start justify-between gap-4">
            <div className="min-w-0">
              <div className="flex items-center gap-3 mb-1">
                <Link
                  href={`/statutes/${data.parent_statute.canonical_id}`}
                  className="text-xs font-mono text-muted-foreground hover:text-foreground inline-flex items-center gap-1"
                >
                  <ArrowLeft className="h-3 w-3" />
                  back to statute
                </Link>
              </div>
              <h1 className="font-serif text-3xl tracking-tight text-foreground">{p.display_citation}</h1>
              <div className="text-sm text-muted-foreground mt-1 font-mono">{p.provision_id}</div>
            </div>

            <div className="flex flex-col items-end gap-2">
              <div className="flex items-center gap-2">
                <StatusBadge status={p.status} />
                <QCBadge status={p.qc_status} />
              </div>
              <span className="text-xs text-muted-foreground capitalize">{p.provision_type}</span>
            </div>
          </div>

          <div className="flex items-center gap-2 mt-3 flex-wrap">
            {p.signals.map((s) => (
              <SignalBadge key={s} signal={s} />
            ))}
          </div>

          {/* Tabs */}
          <div className="flex items-center gap-1 mt-4 border-b border-border -mb-4 overflow-x-auto">
            {TABS.map((t) => (
              <button
                key={t.id}
                onClick={() => setTab(t.id)}
                className={`px-3 py-2 text-sm font-medium border-b-2 transition-colors whitespace-nowrap ${
                  tab === t.id
                    ? "border-primary text-foreground"
                    : "border-transparent text-muted-foreground hover:text-foreground"
                }`}
              >
                {t.label}
              </button>
            ))}
          </div>
        </div>

        {/* Tab body */}
        <div className="flex-1 overflow-y-auto px-6 py-6">
          {tab === "text" && (
            <div className="max-w-3xl">
              <p className="font-serif text-base leading-relaxed text-foreground whitespace-pre-wrap">{p.text}</p>

              {data.children.length > 0 && (
                <div className="mt-8 border-t border-border pt-6">
                  <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-3">
                    Children
                  </div>
                  <ul className="space-y-2">
                    {data.children.map((c) => (
                      <li key={c.provision_id}>
                        <Link
                          href={`/provisions/${encodeURIComponent(c.provision_id)}`}
                          className="block rounded-md border border-border bg-card hover:border-primary/40 hover:bg-accent/40 p-3 transition-colors"
                        >
                          <div className="flex items-center justify-between gap-3">
                            <span className="font-mono text-sm text-foreground">{c.display_citation}</span>
                            <QCBadge status={c.qc_status} />
                          </div>
                          <p className="text-xs text-muted-foreground mt-1 line-clamp-2">{c.text_preview}</p>
                        </Link>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}

          {tab === "chunks" && (
            <div className="space-y-3 max-w-4xl">
              {data.chunks.length === 0 && (
                <p className="text-sm text-muted-foreground">No chunks indexed for this provision.</p>
              )}
              {data.chunks.map((c) => (
                <div key={c.chunk_id} className="rounded-md border border-border bg-card p-4">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <ChunkTypeBadge type={c.chunk_type} />
                      <span className="font-mono text-xs text-muted-foreground">{c.chunk_id}</span>
                    </div>
                    <div className="flex items-center gap-3 text-xs text-muted-foreground font-mono">
                      <span>w={c.search_weight.toFixed(2)}</span>
                      <span>conf={c.parser_confidence.toFixed(2)}</span>
                      <span className={c.embedded ? "text-emerald-500" : "text-amber-500"}>
                        {c.embedded ? "embedded" : "pending"}
                      </span>
                    </div>
                  </div>
                  <p className="text-sm text-foreground/90 leading-relaxed font-serif">{c.text}</p>
                </div>
              ))}
            </div>
          )}

          {tab === "citations" && (
            <div className="grid gap-6 lg:grid-cols-2 max-w-5xl">
              <div>
                <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-2">
                  Outbound ({data.outbound_citations.length})
                </div>
                <div className="space-y-2">
                  {data.outbound_citations.map((c, i) => (
                    <div key={i} className="rounded-md border border-border bg-card p-3">
                      <div className="flex items-center justify-between">
                        <span className="font-mono text-sm text-foreground">{c.target_citation}</span>
                        <span className={`text-xs ${c.resolved ? "text-emerald-500" : "text-rose-500"}`}>
                          {c.resolved ? "resolved" : "unresolved"}
                        </span>
                      </div>
                      <p className="text-xs text-muted-foreground mt-1">"{c.context_snippet}"</p>
                    </div>
                  ))}
                </div>
              </div>
              <div>
                <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-2">
                  Inbound ({data.inbound_citations.length})
                </div>
                <div className="space-y-2">
                  {data.inbound_citations.map((c, i) => (
                    <Link
                      key={i}
                      href={`/statutes/${c.source_canonical_id}`}
                      className="block rounded-md border border-border bg-card p-3 hover:border-primary/40 transition-colors"
                    >
                      <div className="font-mono text-sm text-foreground">{c.source_citation}</div>
                      <div className="text-xs text-muted-foreground mt-0.5">{c.source_title}</div>
                      <p className="text-xs text-muted-foreground/80 mt-1">"{c.context_snippet}"</p>
                    </Link>
                  ))}
                </div>
              </div>
            </div>
          )}

          {tab === "definitions" && (
            <DefList items={data.definitions.map((d) => ({ title: d.term, body: d.text, source: d.source_provision }))} />
          )}
          {tab === "exceptions" && (
            <DefList
              items={data.exceptions.map((e) => ({
                title: `Applies to ${e.applies_to_provision}`,
                body: e.text,
                source: e.source_provision,
              }))}
            />
          )}
          {tab === "deadlines" && (
            <DefList
              items={data.deadlines.map((d) => ({
                title: `${d.duration} — ${d.description}`,
                body: `Trigger: ${d.trigger}`,
                source: d.source_provision,
              }))}
            />
          )}

          {tab === "qc" && (
            <div className="space-y-2 max-w-3xl">
              {data.qc_notes.length === 0 && (
                <p className="text-sm text-muted-foreground">No QC notes for this provision.</p>
              )}
              {data.qc_notes.map((n) => (
                <div key={n.note_id} className="rounded-md border border-border bg-card p-3">
                  <div className="flex items-center gap-2 text-xs font-mono uppercase tracking-wider text-muted-foreground mb-1">
                    <span>{n.category}</span>
                    <span className="text-foreground/50">•</span>
                    <span
                      className={
                        n.level === "fail"
                          ? "text-rose-500"
                          : n.level === "warning"
                          ? "text-amber-500"
                          : "text-sky-500"
                      }
                    >
                      {n.level}
                    </span>
                  </div>
                  <p className="text-sm text-foreground">{n.message}</p>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Right inspector */}
      <aside className="hidden xl:flex w-80 border-l border-border bg-card/40 flex-col overflow-y-auto">
        <div className="p-4 border-b border-border">
          <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-3">
            Edges
          </div>
          <div className="grid grid-cols-2 gap-2 text-xs">
            <Stat icon={GitBranch} label="cites" value={p.cites_count} />
            <Stat icon={GitBranch} label="cited by" value={p.cited_by_count} />
            <Stat icon={Hash} label="chunks" value={p.chunk_count} />
            <Stat icon={Sparkles} label="signals" value={p.signals.length} />
          </div>
        </div>

        {data.siblings.length > 0 && (
          <div className="p-4 border-b border-border">
            <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-3">
              Siblings
            </div>
            <ul className="space-y-1">
              {data.siblings.map((s) => (
                <li key={s.provision_id}>
                  <Link
                    href={`/provisions/${encodeURIComponent(s.provision_id)}`}
                    className="block rounded px-2 py-1.5 text-sm hover:bg-accent text-foreground"
                  >
                    <span className="font-mono text-xs">{s.citation}</span>
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        )}

        <div className="p-4 border-b border-border">
          <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-3">
            Quick links
          </div>
          <div className="flex flex-col gap-1.5">
            <Button asChild variant="outline" size="sm" className="justify-start font-mono text-xs">
              <Link href={`/statutes/${data.parent_statute.canonical_id}`}>
                <ScrollText className="h-3.5 w-3.5" />
                full statute
              </Link>
            </Button>
            <Button asChild variant="outline" size="sm" className="justify-start font-mono text-xs">
              <Link href={`/graph?focus=${encodeURIComponent(p.provision_id)}`}>
                <GitBranch className="h-3.5 w-3.5" />
                view in graph
              </Link>
            </Button>
            <Button asChild variant="outline" size="sm" className="justify-start font-mono text-xs">
              <Link href={`/ask?q=${encodeURIComponent("What is " + p.display_citation + "?")}`}>
                <Sparkles className="h-3.5 w-3.5" />
                ask about this
              </Link>
            </Button>
          </div>
        </div>

        <div className="p-4 mt-auto">
          <div className="text-xs font-mono uppercase tracking-wider text-muted-foreground mb-2">
            Source
          </div>
          <div className="rounded-md border border-border bg-background/40 p-3 text-xs font-mono space-y-1">
            <div className="flex justify-between gap-2">
              <span className="text-muted-foreground">statute</span>
              <span className="text-foreground truncate">{data.parent_statute.citation}</span>
            </div>
            <div className="flex justify-between gap-2">
              <span className="text-muted-foreground">edition</span>
              <span className="text-foreground">{data.parent_statute.edition}</span>
            </div>
            <div className="flex justify-between gap-2">
              <span className="text-muted-foreground">type</span>
              <span className="text-foreground">{p.provision_type}</span>
            </div>
          </div>
        </div>
      </aside>
    </div>
  )
}

function Stat({
  icon: Icon,
  label,
  value,
}: {
  icon: React.ComponentType<{ className?: string }>
  label: string
  value: number
}) {
  return (
    <div className="rounded-md border border-border bg-background/40 p-2">
      <div className="flex items-center gap-1.5 text-muted-foreground mb-0.5">
        <Icon className="h-3 w-3" />
        <span className="font-mono uppercase tracking-wider text-[10px]">{label}</span>
      </div>
      <div className="text-lg font-mono text-foreground">{value}</div>
    </div>
  )
}

function DefList({ items }: { items: { title: string; body: string; source: string }[] }) {
  if (items.length === 0)
    return <p className="text-sm text-muted-foreground">Nothing to show.</p>
  return (
    <div className="space-y-3 max-w-3xl">
      {items.map((it, i) => (
        <div key={i} className="rounded-md border border-border bg-card p-4">
          <div className="flex items-center gap-2 mb-1">
            <FileText className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="font-mono text-sm text-foreground">{it.title}</span>
          </div>
          <p className="text-sm text-foreground/90 font-serif leading-relaxed">{it.body}</p>
          <div className="text-xs text-muted-foreground font-mono mt-2">source: {it.source}</div>
        </div>
      ))}
    </div>
  )
}
