"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { useState } from "react"
import {
  Activity,
  AlertTriangle,
  ArrowLeft,
  BookOpen,
  Calendar,
  CheckSquare,
  FileText,
  Files,
  Folder,
  GavelIcon,
  GitGraphIcon,
  ListChecks,
  Menu,
  Microscope,
  PackageCheck,
  Scale,
  Settings,
  ShieldCheck,
  Sparkles,
  Users,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { casebuilderHomeHref, matterHref, matterWorkProductsHref } from "@/lib/casebuilder/routes"
import type { Matter, MatterSummary } from "@/lib/casebuilder/types"
import { Button } from "@/components/ui/button"
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet"

interface MatterSidebarProps {
  matter: MatterSummary
  counts?: {
    documents?: number
    parties?: number
    facts?: number
    events?: number
    evidence?: number
    claims?: number
    defenses?: number
    drafts?: number
    workProducts?: number
    deadlines?: number
    tasks?: number
  }
  className?: string
  onNavigate?: () => void
}

type MatterSidebarMatter = MatterSummary & Partial<Matter>

export function MatterSidebar({ matter, counts = {}, className, onNavigate }: MatterSidebarProps) {
  const pathname = usePathname()
  const base = matterHref(matter.matter_id)
  const resolvedCounts = resolveMatterCounts(matter as MatterSidebarMatter, counts)

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
        { href: `${base}/documents`, label: "Documents", icon: Folder, count: resolvedCounts.documents },
        { href: `${base}/parties`, label: "Parties", icon: Users, count: resolvedCounts.parties },
        { href: `${base}/facts`, label: "Facts", icon: ListChecks, count: resolvedCounts.facts },
        { href: `${base}/timeline`, label: "Timeline", icon: Calendar, count: resolvedCounts.events },
        { href: `${base}/evidence`, label: "Evidence matrix", icon: Microscope, count: resolvedCounts.evidence },
      ],
    },
    {
      title: "legal layer",
      items: [
        { href: `${base}/claims`, label: "Claims & defenses", icon: Scale, count: resolvedCounts.claims + resolvedCounts.defenses },
        { href: `${base}/deadlines`, label: "Deadlines", icon: AlertTriangle, count: resolvedCounts.deadlines },
        { href: `${base}/authorities`, label: "Authorities", icon: BookOpen },
        { href: `${base}/graph`, label: "Graph", icon: GitGraphIcon },
        { href: `${base}/qc`, label: "QC", icon: ShieldCheck },
      ],
    },
    {
      title: "work product",
      items: [
        { href: matterWorkProductsHref(matter.matter_id), label: "Work product", icon: Files, count: resolvedCounts.workProducts },
        { href: `${base}/complaint`, label: "Complaint editor", icon: GavelIcon },
        { href: `${base}/drafts`, label: "Drafts", icon: FileText, count: resolvedCounts.drafts },
        { href: `${base}/tasks`, label: "Tasks", icon: CheckSquare, count: resolvedCounts.tasks },
        { href: `${base}/export`, label: "Exports", icon: PackageCheck },
        { href: `${base}/settings`, label: "Settings", icon: Settings },
      ],
    },
  ]

  return (
    <aside className={cn("flex h-full w-64 flex-col overflow-hidden border-r border-sidebar-border bg-sidebar text-sidebar-foreground", className)}>
      <div className="border-b border-sidebar-border px-3 py-3">
        <Link
          href={casebuilderHomeHref()}
          onClick={onNavigate}
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

      <nav aria-label="Matter navigation" className="flex-1 overflow-y-auto scrollbar-thin py-2">
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
                    aria-current={active ? "page" : undefined}
                    onClick={onNavigate}
                    className={cn(
                      "mx-2 flex items-center justify-between rounded-md px-2.5 py-1.5 text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60",
                      active
                        ? "bg-primary/10 text-primary"
                        : "text-muted-foreground hover:bg-sidebar-accent hover:text-foreground",
                      item.accent && !active && "text-primary",
                    )}
                  >
                    <span className="flex min-w-0 items-center gap-2">
                      <Icon className="h-3.5 w-3.5 shrink-0" />
                      <span className="truncate">{item.label}</span>
                    </span>
                    {typeof item.count === "number" && item.count > 0 && (
                      <span className={cn(
                        "ml-2 rounded bg-background px-1.5 py-0.5 font-mono text-[10px] tabular-nums",
                        active ? "text-primary" : "text-muted-foreground",
                      )}>
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

export function MatterSidebarSheet({ matter, counts }: Pick<MatterSidebarProps, "matter" | "counts">) {
  const [open, setOpen] = useState(false)

  return (
    <div className="flex items-center gap-2 border-b border-border bg-background px-3 py-2 md:hidden">
      <Sheet open={open} onOpenChange={setOpen}>
        <SheetTrigger asChild>
          <Button variant="outline" size="sm" className="h-8 gap-2">
            <Menu className="h-3.5 w-3.5" />
            Matter
          </Button>
        </SheetTrigger>
        <SheetContent side="left" className="w-[20rem] max-w-[88vw] gap-0 border-sidebar-border bg-sidebar p-0 text-sidebar-foreground">
          <SheetHeader className="sr-only">
            <SheetTitle>Matter navigation</SheetTitle>
          </SheetHeader>
          <MatterSidebar
            matter={matter}
            counts={counts}
            className="w-full border-r-0"
            onNavigate={() => setOpen(false)}
          />
        </SheetContent>
      </Sheet>
      <div className="min-w-0">
        <div className="truncate text-sm font-medium text-foreground">{matter.name}</div>
        <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          {matter.status}
        </div>
      </div>
    </div>
  )
}

function resolveMatterCounts(matter: MatterSidebarMatter, counts: NonNullable<MatterSidebarProps["counts"]>) {
  const claims = matter.claims?.filter((claim) => claim.kind !== "defense") ?? []
  const pendingTimelineSuggestions =
    matter.timeline_suggestions?.filter((suggestion) => suggestion.status === "suggested" || suggestion.status === "needs_attention").length ?? 0

  return {
    documents: counts.documents ?? matter.documents?.length ?? matter.document_count,
    parties: counts.parties ?? matter.parties?.length ?? 0,
    facts: counts.facts ?? matter.facts?.length ?? matter.fact_count,
    events: counts.events ?? (matter.timeline?.length ?? 0) + pendingTimelineSuggestions,
    evidence: counts.evidence ?? matter.evidence?.length ?? matter.evidence_count,
    claims: counts.claims ?? (claims.length || matter.claim_count),
    defenses: counts.defenses ?? matter.defenses?.length ?? 0,
    drafts: counts.drafts ?? matter.drafts?.length ?? matter.draft_count,
    workProducts: counts.workProducts ?? matter.work_products?.length ?? 0,
    deadlines: counts.deadlines ?? matter.deadlines?.filter((deadline) => deadline.status === "open").length ?? 0,
    tasks: counts.tasks ?? matter.tasks?.filter((task) => task.status !== "done").length ?? matter.open_task_count,
  }
}
