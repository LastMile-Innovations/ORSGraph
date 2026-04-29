import type { StatutePageResponse } from "@/lib/types"
import { Type } from "lucide-react"

export function DefinitionsTab({ data }: { data: StatutePageResponse }) {
  if (data.definitions.length === 0) {
    return <EmptyTab label="No definitions detected for this statute." />
  }
  return (
    <div className="px-6 py-6">
      <div className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        {data.definitions.length} definitions extracted from {data.identity.citation}
      </div>
      <ul className="grid grid-cols-1 gap-3 md:grid-cols-2">
        {data.definitions.map((d) => (
          <li
            key={d.definition_id}
            className="rounded border border-border bg-card p-4 hover:border-primary/40"
          >
            <div className="flex items-center gap-2">
              <Type className="h-3.5 w-3.5 text-chart-1" />
              <span className="font-mono text-xs uppercase tracking-wide text-muted-foreground">
                term
              </span>
            </div>
            <h4 className="mt-1 font-serif text-lg italic text-foreground">{d.term}</h4>
            <p className="mt-2 text-sm leading-relaxed text-foreground">{d.text}</p>
            <div className="mt-3 flex items-center justify-between border-t border-border pt-2 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
              <span>source: <span className="text-primary">{d.source_provision}</span></span>
              <span>scope: {d.scope}</span>
            </div>
          </li>
        ))}
      </ul>
    </div>
  )
}

function EmptyTab({ label }: { label: string }) {
  return (
    <div className="flex flex-1 items-center justify-center px-6 py-16 text-sm text-muted-foreground">
      {label}
    </div>
  )
}
