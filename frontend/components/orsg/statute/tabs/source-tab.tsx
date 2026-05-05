import type { StatutePageResponse } from "@/lib/types"
import { ExternalLink, FileText } from "lucide-react"

export function SourceTab({ data }: { data: StatutePageResponse }) {
  return (
    <div className="px-6 py-6">
      <div className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        Source trail · {data.source_documents.length} documents
      </div>
      <div className="space-y-3">
        {data.source_documents.map((s) => (
          <div key={s.source_id} className="overflow-hidden rounded border border-border bg-card">
            <header className="flex items-center justify-between border-b border-border px-4 py-2.5">
              <div className="flex items-center gap-2">
                <FileText className="h-4 w-4 text-primary" />
                <span className="font-mono text-sm text-foreground">{s.source_id}</span>
              </div>
              <a
                href={normalizeExternalUrl(s.url)}
                target="_blank"
                rel="noreferrer"
                className="flex items-center gap-1 font-mono text-xs text-primary hover:underline"
              >
                <ExternalLink className="h-3 w-3" />
                open original
              </a>
            </header>
            <dl className="grid grid-cols-1 gap-px bg-border md:grid-cols-2">
              <SourceField label="url" value={normalizeExternalUrl(s.url)} mono />
              <SourceField label="retrieved at" value={s.retrieved_at} mono />
              <SourceField label="edition year" value={String(s.edition_year)} />
              <SourceField label="parser profile" value={s.parser_profile} mono />
              <SourceField label="raw hash" value={s.raw_hash} mono truncate />
              <SourceField label="normalized hash" value={s.normalized_hash} mono truncate />
            </dl>
            {s.parser_warnings.length > 0 && (
              <div className="border-t border-border bg-warning/5 px-4 py-2 font-mono text-xs text-warning">
                {s.parser_warnings.length} parser warning(s)
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  )
}

function SourceField({
  label,
  value,
  mono,
  truncate,
}: {
  label: string
  value: string
  mono?: boolean
  truncate?: boolean
}) {
  return (
    <div className="bg-card px-4 py-2">
      <dt className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
        {label}
      </dt>
      <dd
        className={`mt-0.5 text-sm text-foreground ${mono ? "font-mono text-xs" : ""} ${
          truncate ? "truncate" : "break-all"
        }`}
      >
        {value || "Not available"}
      </dd>
    </div>
  )
}

function normalizeExternalUrl(value: string) {
  if (!value) return ""
  if (/^https?:\/\//i.test(value)) return value
  return `https://${value.replace(/^\/+/, "")}`
}
