import { AlertTriangle, Database, FlaskConical, WifiOff } from "lucide-react"
import type { LoadSource } from "@/lib/casebuilder/api"
import { cn } from "@/lib/utils"

export function DataStateBanner({ source, error }: { source?: LoadSource; error?: string }) {
  if (!source || source === "live") return null

  const meta = {
    demo: {
      icon: FlaskConical,
      label: "Demo matter data",
      body: "CaseBuilder is showing seeded local data because the matter API is unavailable or has no matching record.",
      cls: "border-warning/30 bg-warning/10 text-warning",
    },
    offline: {
      icon: WifiOff,
      label: "API offline",
      body: "Live CaseBuilder data could not be loaded.",
      cls: "border-destructive/30 bg-destructive/10 text-destructive",
    },
    error: {
      icon: AlertTriangle,
      label: "Data error",
      body: "CaseBuilder could not load this matter.",
      cls: "border-destructive/30 bg-destructive/10 text-destructive",
    },
  }[source]

  const Icon = meta?.icon ?? Database

  return (
    <div className={cn("border-b px-4 py-2 text-xs", meta?.cls)}>
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <Icon className="h-3.5 w-3.5" />
        <span className="font-mono uppercase tracking-wider">{meta?.label}</span>
        <span className="text-current/80">{meta?.body}</span>
        {error && <span className="truncate font-mono text-[10px] text-current/70">({error})</span>}
      </div>
    </div>
  )
}
