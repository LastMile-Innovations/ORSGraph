import { AlertTriangle, Database, FlaskConical, WifiOff } from "lucide-react"
import type { DataSource } from "@/lib/data-state"
import { cn } from "@/lib/utils"

interface DataStateBannerProps {
  source?: DataSource
  label?: string
  error?: string
  className?: string
}

export function DataStateBanner({ source, label, error, className }: DataStateBannerProps) {
  if (!source || source === "live") return null

  const meta = {
    mock: {
      icon: Database,
      label: label ?? "Bundled fallback data",
      body: "Live API data is unavailable, so this view is using bundled read-only data.",
      cls: "border-warning/30 bg-warning/10 text-warning",
    },
    demo: {
      icon: FlaskConical,
      label: label ?? "Demo data",
      body: "This workflow is seeded and read-only until the API endpoint is connected.",
      cls: "border-primary/30 bg-primary/10 text-primary",
    },
    offline: {
      icon: WifiOff,
      label: label ?? "API offline",
      body: "The live API could not be reached. This view is limited to data the API already returned.",
      cls: "border-destructive/30 bg-destructive/10 text-destructive",
    },
    empty: {
      icon: Database,
      label: label ?? "No live data",
      body: "The live API returned no records for this view.",
      cls: "border-border bg-muted/40 text-muted-foreground",
    },
    error: {
      icon: AlertTriangle,
      label: label ?? "Data error",
      body: "This view could not load live data.",
      cls: "border-destructive/30 bg-destructive/10 text-destructive",
    },
  }[source]

  const Icon = meta.icon

  return (
    <div className={cn("border-b px-4 py-2 text-xs", meta.cls, className)}>
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <Icon className="h-3.5 w-3.5" />
        <span className="font-mono uppercase tracking-wider">{meta.label}</span>
        <span className="text-current/80">{meta.body}</span>
        {error && <span className="truncate font-mono text-[10px] text-current/70">({error})</span>}
      </div>
    </div>
  )
}
