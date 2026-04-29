import type { StatutePageResponse } from "@/lib/types"
import { ShieldAlert } from "lucide-react"

export function ExceptionsTab({ data }: { data: StatutePageResponse }) {
  if (data.exceptions.length === 0 && data.penalties.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center px-6 py-16 text-sm text-muted-foreground">
        No exceptions or penalties detected.
      </div>
    )
  }
  return (
    <div className="grid grid-cols-1 gap-px bg-border lg:grid-cols-2">
      <section className="bg-card">
        <header className="border-b border-border px-4 py-2.5">
          <h3 className="font-mono text-xs uppercase tracking-widest text-muted-foreground">
            exceptions · {data.exceptions.length}
          </h3>
        </header>
        <ul className="divide-y divide-border">
          {data.exceptions.map((e) => (
            <li key={e.exception_id} className="flex items-start gap-3 px-4 py-3">
              <ShieldAlert className="mt-0.5 h-4 w-4 flex-none text-warning" />
              <div className="min-w-0 flex-1">
                <p className="text-sm text-foreground">{e.text}</p>
                <div className="mt-1.5 flex flex-wrap gap-x-3 gap-y-1 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                  <span>applies to: <span className="text-primary">{e.applies_to_provision}</span></span>
                  <span>source: <span className="text-primary">{e.source_provision}</span></span>
                </div>
              </div>
            </li>
          ))}
        </ul>
      </section>

      <section className="bg-card">
        <header className="border-b border-border px-4 py-2.5">
          <h3 className="font-mono text-xs uppercase tracking-widest text-muted-foreground">
            penalties · {data.penalties.length}
          </h3>
        </header>
        <ul className="divide-y divide-border">
          {data.penalties.map((p) => (
            <li key={p.penalty_id} className="flex items-start gap-3 px-4 py-3">
              <span
                className={`mt-1 inline-flex items-center rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide ${
                  p.category === "criminal"
                    ? "bg-destructive/15 text-destructive"
                    : p.category === "civil"
                      ? "bg-warning/15 text-warning"
                      : "bg-muted text-muted-foreground"
                }`}
              >
                {p.category}
              </span>
              <div className="min-w-0 flex-1">
                <p className="text-sm text-foreground">{p.description}</p>
                <div className="mt-1 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                  source: <span className="text-primary">{p.source_provision}</span>
                </div>
              </div>
            </li>
          ))}
        </ul>
      </section>
    </div>
  )
}
