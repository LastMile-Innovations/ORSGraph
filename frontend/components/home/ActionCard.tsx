import Link from "next/link"
import { HomeAction } from "@/lib/types"
import type { LucideIcon } from "lucide-react"
import { Activity, ArrowRight, BookOpen, CheckCircle2, Clock3, MessageSquare, Network, Search, ShieldCheck } from "lucide-react"
import { cn } from "@/lib/utils"

const IconMap: Record<string, LucideIcon> = {
  Search,
  MessageSquare,
  BookOpen,
  Network,
  ShieldCheck,
  Activity,
}

const statusClass: Record<NonNullable<HomeAction["status"]>, string> = {
  ready: "bg-success/15 text-success",
  coming_soon: "bg-muted text-muted-foreground",
  internal: "bg-primary/15 text-primary",
  warning: "bg-warning/15 text-warning",
}

const statusIcon: Record<NonNullable<HomeAction["status"]>, LucideIcon> = {
  ready: CheckCircle2,
  coming_soon: Clock3,
  internal: Activity,
  warning: ShieldCheck,
}

export function ActionCard({ action }: { action: HomeAction }) {
  const Icon = IconMap[action.icon] || Search
  const StatusIcon = action.status ? statusIcon[action.status] : undefined
  const isPrimary = action.variant === "primary"

  return (
    <Link 
      href={action.href}
      className={cn(
        "group relative flex min-h-52 flex-col rounded-md border p-5 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60",
        isPrimary 
          ? "border-primary/40 bg-primary/10 hover:border-primary/70"
          : "border-border bg-card hover:border-primary/40"
      )}
    >
      <div className="mb-4 flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-center gap-3">
          <div className={cn(
            "flex h-10 w-10 shrink-0 items-center justify-center rounded-md",
            isPrimary ? "bg-primary/15 text-primary" : "bg-muted text-muted-foreground group-hover:text-primary"
          )}>
            <Icon className="h-5 w-5" />
          </div>
          <h3 className="min-w-0 text-sm font-semibold text-foreground">
            {action.title}
          </h3>
        </div>
        {action.status && StatusIcon && (
          <span className={cn("inline-flex shrink-0 items-center gap-1 rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide", statusClass[action.status])}>
            <StatusIcon className="h-3 w-3" />
            {action.status.replace("_", " ")}
          </span>
        )}
      </div>
      <p className="mb-5 flex-grow text-sm leading-6 text-muted-foreground">
        {action.description}
      </p>
      <div className="mt-auto flex items-end justify-between gap-3">
        <div className="flex flex-wrap gap-1.5">
          {action.badges?.map(badge => (
            <span
              key={badge}
              className="rounded border border-border bg-background px-2 py-1 font-mono text-[10px] uppercase tracking-wide text-muted-foreground"
            >
              {badge}
            </span>
          ))}
        </div>
        <ArrowRight className="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-primary" />
      </div>
    </Link>
  )
}
