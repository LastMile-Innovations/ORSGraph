"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import {
  Activity,
  AlertTriangle,
  ArrowLeft,
  BookOpen,
  Calendar,
  CheckSquare,
  FileText,
  Folder,
  GavelIcon,
  GitGraphIcon,
  ListChecks,
  Microscope,
  PackageCheck,
  Scale,
  ShieldCheck,
  Sparkles,
  Users,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { casebuilderHomeHref, matterHref } from "@/lib/casebuilder/routes"
import type { MatterSummary } from "@/lib/casebuilder/types"

interface MatterSidebarProps {
  matter: MatterSummary
  counts?: {
    documents?: number
    facts?: number
    events?: number
    evidence?: number
    claims?: number
    defenses?: number
    drafts?: number
    deadlines?: number
    tasks?: number
  }
}

export function MatterSidebar({ matter, counts = {} }: MatterSidebarProps) {
  const pathname = usePathname()
  const base = matterHref(matter.matter_id)

  const groups: { title: string; items: { href: string; label: string; icon: typeof Folder; count?: number; accent?: boolean }[] }[] = [
    {
      title: "matter",
      items: [
        { href: base, label: "Dashboard", icon: Activity },
        { href: `${base}/ask`, label: "Ask matter", icon: Sparkles, accent: true },
      ],
    },
    {
      title: "evidence layer",
      items: [
        { href: `${base}/documents`, label: "Documents", icon: Folder, count: counts.documents ?? matter.document_count },
        { href: `${base}/parties`, label: "Parties", icon: Users },
        { href: `${base}/facts`, label: "Facts", icon: ListChecks, count: counts.facts ?? matter.fact_count },
        { href: `${base}/timeline`, label: "Timeline", icon: Calendar, count: counts.events },
        { href: `${base}/evidence`, label: "Evidence matrix", icon: Microscope, count: counts.evidence ?? matter.evidence_count },
      ],
    },
    {
      title: "legal layer",
      items: [
        { href: `${base}/claims`, label: "Claims & defenses", icon: Scale, count: counts.claims ?? matter.claim_count },
        { href: `${base}/deadlines`, label: "Deadlines", icon: AlertTriangle, count: counts.deadlines },
        { href: `${base}/authorities`, label: "Authorities", icon: BookOpen },
        { href: `${base}/graph`, label: "Graph", icon: GitGraphIcon },
        { href: `${base}/qc`, label: "QC", icon: ShieldCheck },
      ],
    },
    {
      title: "work product",
      items: [
        { href: `${base}/complaint`, label: "Complaint builder", icon: GavelIcon },
        { href: `${base}/drafts`, label: "Drafts", icon: FileText, count: counts.drafts ?? matter.draft_count },
        { href: `${base}/tasks`, label: "Tasks", icon: CheckSquare, count: counts.tasks ?? matter.open_task_count },
        { href: `${base}/export`, label: "Exports", icon: PackageCheck },
      ],
    },
  ]

  return (
    <aside className="flex h-full w-60 flex-col overflow-hidden border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
      <div className="border-b border-sidebar-border px-3 py-3">
        <Link
          href={casebuilderHomeHref()}
          className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-widest text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="h-3 w-3" />
          all matters
        </Link>
        <div className="mt-2 flex items-start gap-2">
          <div className="mt-0.5 flex h-7 w-7 flex-shrink-0 items-center justify-center rounded bg-primary/15 font-mono text-[10px] text-primary">
            <GavelIcon className="h-3.5 w-3.5" />
          </div>
          <div className="min-w-0">
            <div className="truncate text-sm font-medium text-foreground">{matter.name}</div>
            <div className="font-mono text-[10px] tabular-nums text-muted-foreground">
              {matter.case_number ?? "no case #"}
            </div>
          </div>
        </div>
        <div className="mt-2 flex flex-wrap gap-1">
          <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
            {matter.matter_type.replace(/_/g, " ")}
          </span>
          <span
            className={cn(
              "rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
              matter.status === "active" && "bg-success/15 text-success",
              matter.status === "intake" && "bg-primary/15 text-primary",
              matter.status === "stayed" && "bg-warning/15 text-warning",
              matter.status === "closed" && "bg-muted text-muted-foreground",
              matter.status === "appeal" && "bg-accent/20 text-accent",
            )}
          >
            {matter.status}
          </span>
          <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
            role: {matter.user_role}
          </span>
        </div>
      </div>

      <nav className="flex-1 overflow-y-auto scrollbar-thin py-2">
        {groups.map((group) => (
          <div key={group.title} className="mb-2">
            <div className="px-3 py-1 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              {group.title}
            </div>
            <div className="space-y-px">
              {group.items.map((item) => {
                const active =
                  pathname === item.href || (item.href !== base && pathname.startsWith(item.href))
                const Icon = item.icon
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    className={cn(
                      "flex items-center justify-between px-3 py-1.5 text-xs transition-colors",
                      active
                        ? "bg-sidebar-accent text-sidebar-accent-foreground"
                        : "text-muted-foreground hover:bg-sidebar-accent hover:text-foreground",
                      item.accent && !active && "text-primary",
                    )}
                  >
                    <span className="flex items-center gap-2">
                      <Icon className="h-3.5 w-3.5" />
                      {item.label}
                    </span>
                    {typeof item.count === "number" && item.count > 0 && (
                      <span className="font-mono text-[10px] tabular-nums text-muted-foreground">
                        {item.count}
                      </span>
                    )}
                  </Link>
                )
              })}
            </div>
          </div>
        ))}
      </nav>

      {matter.next_deadline && (
        <div className="border-t border-sidebar-border bg-sidebar p-3">
          <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            next deadline
          </div>
          <div className="mt-1 text-xs text-foreground">{matter.next_deadline.description}</div>
          <div className="mt-0.5 flex items-center justify-between font-mono text-[10px] tabular-nums">
            <span className="text-muted-foreground">{matter.next_deadline.due_date}</span>
            <span
              className={cn(
                matter.next_deadline.days_remaining <= 7
                  ? "text-destructive"
                  : matter.next_deadline.days_remaining <= 21
                    ? "text-warning"
                    : "text-success",
              )}
            >
              {matter.next_deadline.days_remaining}d
            </span>
          </div>
        </div>
      )}
    </aside>
  )
}
