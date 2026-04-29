"use client"

import Link from "next/link"
import type { SourceIndexEntry } from "@/lib/types"
import { SourceTrail } from "@/components/orsg/source-trail"
import { ArrowLeft, ExternalLink, FileText, Database } from "lucide-react"

export function SourceDetailClient({
  source,
  otherSources,
}: {
  source: SourceIndexEntry
  otherSources: SourceIndexEntry[]
}) {
  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="border-b border-border bg-card px-6 py-4">
        <Link
          href="/sources"
          className="inline-flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:text-primary"
        >
          <ArrowLeft className="h-3 w-3" />
          back to sources
        </Link>
        <div className="mt-2 flex flex-col items-start justify-between gap-3 md:flex-row md:items-center">
          <div>
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <Database className="h-3 w-3" />
              source document
            </div>
            <h1 className="mt-1 font-mono text-base text-foreground">{source.source_id}</h1>
            <p className="mt-1 text-sm text-muted-foreground">{source.title}</p>
          </div>
          <a
            href={source.url}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5 rounded border border-border bg-card px-3 py-1.5 font-mono text-xs uppercase tracking-wider hover:border-primary hover:text-primary"
          >
            <ExternalLink className="h-3.5 w-3.5" />
            open original
          </a>
        </div>
      </div>

      <div className="grid flex-1 grid-cols-1 gap-0 overflow-hidden lg:grid-cols-[1fr_360px]">
        <div className="overflow-y-auto p-6">
          <section className="mb-6">
            <h2 className="mb-3 font-mono text-[11px] uppercase tracking-wider text-muted-foreground">
              ingestion
            </h2>
            <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
              <Field label="jurisdiction" value={source.jurisdiction} />
              <Field label="scope" value={source.scope} />
              <Field label="edition" value={String(source.edition_year)} />
              <Field
                label="status"
                value={source.ingestion_status}
                tone={
                  source.ingestion_status === "failed"
                    ? "fail"
                    : source.ingestion_status === "queued"
                    ? "warning"
                    : "success"
                }
              />
            </div>
          </section>

          <section className="mb-6">
            <h2 className="mb-3 font-mono text-[11px] uppercase tracking-wider text-muted-foreground">
              what this source produced
            </h2>
            <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
              <Field label="sections" value="38" />
              <Field label="provisions" value="412" />
              <Field label="chunks" value="1,847" />
              <Field label="citation mentions" value="612" />
            </div>
          </section>

          {source.parser_warnings.length > 0 && (
            <section className="mb-6">
              <h2 className="mb-3 font-mono text-[11px] uppercase tracking-wider text-muted-foreground">
                parser warnings ({source.parser_warnings.length})
              </h2>
              <ul className="space-y-1.5 rounded border border-warning/30 bg-warning/5 p-3">
                {source.parser_warnings.map((w, i) => (
                  <li key={i} className="text-xs text-foreground">
                    <span className="mr-2 font-mono text-[10px] text-warning">[warn]</span>
                    {w}
                  </li>
                ))}
              </ul>
            </section>
          )}

          <section>
            <h2 className="mb-3 font-mono text-[11px] uppercase tracking-wider text-muted-foreground">
              related sources
            </h2>
            <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
              {otherSources.map((s) => (
                <Link
                  key={s.source_id}
                  href={`/sources/${encodeURIComponent(s.source_id)}`}
                  className="group rounded border border-border bg-card p-3 hover:border-primary/40"
                >
                  <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                    <FileText className="h-3 w-3" />
                    {s.scope}
                    <span className="ml-auto tabular-nums">{s.edition_year}</span>
                  </div>
                  <div className="mt-1 line-clamp-1 text-sm text-foreground group-hover:text-primary">
                    {s.title}
                  </div>
                </Link>
              ))}
            </div>
          </section>
        </div>

        <aside className="overflow-y-auto border-l border-border bg-card p-4">
          <SourceTrail source={source} />

          <div className="mt-4 space-y-2 font-mono text-[11px] tabular-nums">
            <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              integrity
            </div>
            <div className="rounded border border-border bg-background p-2">
              <div className="flex justify-between">
                <span className="text-muted-foreground">raw</span>
                <span className="text-foreground">{source.raw_hash ? "ok" : "—"}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">normalized</span>
                <span className={source.normalized_hash ? "text-foreground" : "text-destructive"}>
                  {source.normalized_hash ? "ok" : "missing"}
                </span>
              </div>
              <div className="mt-1 flex justify-between">
                <span className="text-muted-foreground">edition match</span>
                <span className="text-success">ok</span>
              </div>
            </div>
          </div>
        </aside>
      </div>
    </div>
  )
}

function Field({
  label,
  value,
  tone,
}: {
  label: string
  value: string
  tone?: "success" | "warning" | "fail"
}) {
  return (
    <div className="rounded border border-border bg-card p-2">
      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div
        className={`mt-0.5 font-mono text-xs ${
          tone === "success"
            ? "text-success"
            : tone === "warning"
            ? "text-warning"
            : tone === "fail"
            ? "text-destructive"
            : "text-foreground"
        }`}
      >
        {value}
      </div>
    </div>
  )
}
