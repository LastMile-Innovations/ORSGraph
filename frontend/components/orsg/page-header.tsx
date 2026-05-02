import type { LucideIcon } from "lucide-react"
import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

interface PageHeaderProps {
  icon: LucideIcon
  eyebrow: string
  title: string
  description?: string
  actions?: ReactNode
  stats?: Array<{
    label: string
    value: string | number
    tone?: "default" | "primary" | "accent" | "success" | "warning" | "destructive"
  }>
  className?: string
}

const statToneClass = {
  default: "text-foreground",
  primary: "text-primary",
  accent: "text-accent",
  success: "text-success",
  warning: "text-warning",
  destructive: "text-destructive",
} as const

export function PageHeader({
  icon: Icon,
  eyebrow,
  title,
  description,
  actions,
  stats,
  className,
}: PageHeaderProps) {
  return (
    <section className={cn("border-b border-border bg-card px-4 py-6 sm:px-6 lg:px-8", className)}>
      <div className="mx-auto flex w-full max-w-7xl flex-col gap-5">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
          <div className="min-w-0">
            <div className="mb-2 flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <Icon className="h-3.5 w-3.5 text-primary" />
              <span>{eyebrow}</span>
            </div>
            <h1 className="text-balance text-2xl font-semibold tracking-normal text-foreground sm:text-3xl">
              {title}
            </h1>
            {description && (
              <p className="mt-2 max-w-3xl text-pretty text-sm leading-6 text-muted-foreground">
                {description}
              </p>
            )}
          </div>
          {actions && <div className="flex shrink-0 flex-wrap gap-2">{actions}</div>}
        </div>

        {stats && stats.length > 0 && (
          <div className="grid grid-cols-2 gap-px overflow-hidden rounded-md border border-border bg-border sm:grid-cols-3 lg:grid-cols-5">
            {stats.map((stat) => (
              <div key={stat.label} className="min-w-0 bg-background px-4 py-3">
                <div className="truncate font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                  {stat.label}
                </div>
                <div
                  className={cn(
                    "mt-1 font-mono text-lg font-semibold tabular-nums",
                    statToneClass[stat.tone ?? "default"],
                  )}
                >
                  {typeof stat.value === "number" ? stat.value.toLocaleString() : stat.value}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  )
}
