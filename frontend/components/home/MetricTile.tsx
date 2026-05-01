import Link from "next/link"
import { cn } from "@/lib/utils"

export interface MetricTileProps {
  label: string
  value: number | string
  helper?: string
  state: "ok" | "warning" | "error" | "unknown"
  href?: string
}

export function MetricTile({ label, value, helper, state, href }: MetricTileProps) {
  const content = (
    <div className="flex h-full flex-col">
      <h3 className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{label}</h3>
      <div className="mb-1 flex items-baseline gap-2">
        <span className="font-mono text-2xl font-semibold tabular-nums text-foreground">{formatMetricValue(value)}</span>
      </div>
      {helper && (
        <p className={cn(
          "mt-auto border-t border-border pt-2 text-xs",
          state === "warning" ? "text-warning" :
          state === "error" ? "text-destructive" :
          state === "ok" ? "text-success" :
          "text-muted-foreground"
        )}>
          {helper}
        </p>
      )}
    </div>
  )

  const className = "rounded-md border border-border bg-card p-4 transition-colors hover:border-primary/40"

  if (href) {
    return (
      <Link href={href} className={cn(className, "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60")}>
        {content}
      </Link>
    )
  }

  return <div className={className}>{content}</div>
}

function formatMetricValue(value: number | string) {
  if (typeof value === "number") return new Intl.NumberFormat().format(value)
  return value
}
