import type { StatutePageResponse } from "@/lib/types"

export function VersionsTab({ data }: { data: StatutePageResponse }) {
  return (
    <div className="px-6 py-6">
      <div className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        {data.versions.length} versions on record
      </div>
      <ol className="relative space-y-3 border-l-2 border-border pl-6">
        {data.versions.map((v) => (
          <li key={v.version_id} className="relative">
            <span
              className={`absolute -left-[1.625rem] top-2 h-3 w-3 rounded-full border-2 ${
                v.is_current
                  ? "border-primary bg-primary"
                  : "border-border bg-background"
              }`}
            />
            <div
              className={`rounded border p-4 ${
                v.is_current ? "border-primary/40 bg-primary/5" : "border-border bg-card"
              }`}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-sm font-semibold text-foreground">
                    {v.effective_date.slice(0, 4)}
                  </span>
                  {v.is_current && (
                    <span className="inline-flex items-center rounded bg-primary px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-primary-foreground">
                      current
                    </span>
                  )}
                </div>
                <span className="font-mono text-[10px] tabular-nums text-muted-foreground">
                  {v.version_id}
                </span>
              </div>
              <div className="mt-1 flex gap-4 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                <span>effective: <span className="text-foreground">{v.effective_date}</span></span>
                <span>
                  ends:{" "}
                  <span className="text-foreground">{v.end_date ?? "—"}</span>
                </span>
              </div>
              {v.is_current && (
                <p className="mt-2 line-clamp-3 text-xs text-muted-foreground">{data.current_version.text.slice(0, 240)}…</p>
              )}
            </div>
          </li>
        ))}
      </ol>
    </div>
  )
}
