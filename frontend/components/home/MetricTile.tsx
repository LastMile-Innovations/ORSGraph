import { cn } from "@/lib/utils"

export interface MetricTileProps {
  label: string
  value: number | string
  helper?: string
  state: "ok" | "warning" | "error" | "unknown"
  href?: string
}

export function MetricTile({ label, value, helper, state }: MetricTileProps) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-4 flex flex-col hover:border-zinc-700 transition-colors">
      <h3 className="text-xs font-medium uppercase tracking-wider text-zinc-500 mb-2">{label}</h3>
      <div className="flex items-baseline gap-2 mb-1">
        <span className="text-2xl font-bold text-zinc-100">{value.toLocaleString()}</span>
      </div>
      {helper && (
        <p className={cn(
          "text-xs mt-auto pt-2 border-t border-zinc-800/50",
          state === "warning" ? "text-amber-500" :
          state === "error" ? "text-red-500" :
          "text-zinc-500"
        )}>
          {helper}
        </p>
      )}
    </div>
  )
}
