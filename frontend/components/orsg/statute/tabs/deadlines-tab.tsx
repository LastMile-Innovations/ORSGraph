import type { StatutePageResponse } from "@/lib/types"
import { Clock } from "lucide-react"

export function DeadlinesTab({ data }: { data: StatutePageResponse }) {
  if (data.deadlines.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center px-6 py-16 text-sm text-muted-foreground">
        No deadlines detected for this statute.
      </div>
    )
  }
  return (
    <div className="px-6 py-6">
      <div className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        {data.deadlines.length} deadlines extracted
      </div>
      <ul className="space-y-2">
        {data.deadlines.map((d) => (
          <li
            key={d.deadline_id}
            className="flex items-start gap-3 rounded border border-border bg-card p-4"
          >
            <div className="flex h-9 w-9 flex-none items-center justify-center rounded bg-chart-3/15 text-chart-3">
              <Clock className="h-4 w-4" />
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex items-baseline gap-3">
                <span className="font-mono text-lg font-semibold text-chart-3">{d.duration}</span>
                <span className="text-sm text-foreground">{d.description}</span>
              </div>
              <p className="mt-1 text-sm text-muted-foreground">
                <span className="font-mono text-[10px] uppercase tracking-wide">trigger:</span>{" "}
                {d.trigger}
              </p>
              <div className="mt-2 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                source: <span className="text-primary">{d.source_provision}</span>
              </div>
            </div>
          </li>
        ))}
      </ul>
    </div>
  )
}
