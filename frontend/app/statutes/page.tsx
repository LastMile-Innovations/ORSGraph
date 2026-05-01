import Link from "next/link"
import { Shell } from "@/components/orsg/shell"
import { getStatuteIndex } from "@/lib/api"
import { StatusBadge } from "@/components/orsg/badges"
import { ChevronRight, BookOpen } from "lucide-react"

export default async function StatuteIndexPage() {
  const statutes = await getStatuteIndex()
  // Group by chapter for a corpus directory listing
  const grouped: Record<string, typeof statutes> = {}
  for (const s of statutes) {
    if (!grouped[s.chapter]) grouped[s.chapter] = []
    grouped[s.chapter].push(s)
  }

  return (
    <Shell>
      <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
        <header className="border-b border-border bg-card px-6 py-5">
          <div className="flex items-baseline gap-3">
            <BookOpen className="h-5 w-5 text-primary" />
            <h1 className="font-mono text-lg font-semibold">Oregon Revised Statutes</h1>
            <span className="font-mono text-xs uppercase tracking-wide text-muted-foreground">
              edition 2025 / {statutes.length} indexed sections
            </span>
          </div>
          <p className="mt-1 text-sm text-muted-foreground">
            Browse the corpus by chapter. Click a section to open its statute intelligence page.
          </p>
        </header>

        <div className="grid flex-1 grid-cols-1 gap-px overflow-y-auto bg-border md:grid-cols-2 lg:grid-cols-3">
          {Object.entries(grouped).map(([chapter, items]) => (
            <section key={chapter} className="bg-card">
              <div className="flex items-center justify-between border-b border-border px-4 py-2">
                <h2 className="font-mono text-xs uppercase tracking-wider text-muted-foreground">
                  Chapter {chapter}
                </h2>
                <span className="font-mono text-[10px] tabular-nums text-muted-foreground">
                  {items.length} sections
                </span>
              </div>
              <ul className="divide-y divide-border">
                {items.map((s) => (
                  <li key={s.canonical_id}>
                    <Link
                      href={`/statutes/${s.canonical_id}`}
                      className="flex items-center justify-between px-4 py-2.5 hover:bg-muted"
                    >
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span className="font-mono text-sm font-medium text-primary">
                            {s.citation}
                          </span>
                          <StatusBadge status={s.status} />
                        </div>
                        <p className="mt-0.5 truncate text-sm text-foreground">{s.title}</p>
                      </div>
                      <ChevronRight className="h-4 w-4 text-muted-foreground" />
                    </Link>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      </div>
    </Shell>
  )
}
