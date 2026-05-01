import { FeaturedStatute } from "@/lib/types"
import Link from "next/link"
import { Book } from "lucide-react"
import { SemanticBadge, SourceBadge, StatusBadge } from "@/components/orsg/badges"

export function FeaturedStatutesGrid({ statutes }: { statutes: FeaturedStatute[] }) {
  if (!statutes || statutes.length === 0) return null

  return (
    <section className="mb-12">
      <h2 className="mb-4 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">featured statutes</h2>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        {statutes.map(statute => (
          <Link 
            key={statute.citation} 
            href={statute.href}
            className="flex min-h-40 flex-col rounded-md border border-border bg-card p-4 transition-colors hover:border-primary/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
          >
            <div className="mb-2 flex items-start justify-between gap-3">
              <div className="flex min-w-0 items-center gap-2 font-mono text-sm font-medium text-primary">
                <Book className="h-4 w-4 shrink-0" />
                {statute.citation}
              </div>
              <span className="shrink-0 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{statute.chapter}</span>
            </div>
            <h3 className="mb-4 line-clamp-2 font-semibold text-foreground">{statute.title}</h3>

            <div className="mt-auto flex flex-wrap gap-1.5">
              {statute.semanticTypes.map(type => (
                <SemanticBadge key={type} type={type} />
              ))}
              {statute.status !== "unknown" && <StatusBadge status={statute.status} />}
              {statute.sourceBacked && <SourceBadge />}
              {typeof statute.citedByCount === "number" && (
                <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                  {statute.citedByCount.toLocaleString()} cited by
                </span>
              )}
            </div>
          </Link>
        ))}
      </div>
    </section>
  )
}
