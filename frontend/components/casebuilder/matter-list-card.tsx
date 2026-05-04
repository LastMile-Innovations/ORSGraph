"use client"

import Link from "next/link"
import {
  AlertTriangle,
  ArrowRight,
  CalendarClock,
  FileText,
  Folder,
  Scale,
} from "lucide-react"
import { DeleteMatterButton } from "@/components/casebuilder/delete-matter-button"
import { matterHref } from "@/lib/casebuilder/routes"
import { cn } from "@/lib/utils"
import type { MatterStatus, MatterSummary } from "@/lib/casebuilder/types"

const STATUS_CLS: Record<MatterStatus, string> = {
  active: "bg-success/15 text-success",
  intake: "bg-primary/15 text-primary",
  stayed: "bg-warning/15 text-warning",
  closed: "bg-muted text-muted-foreground",
  appeal: "bg-accent/20 text-accent",
}

export function MatterListCard({ matter }: { matter: MatterSummary }) {
  return (
    <article className="group flex flex-col rounded border border-border bg-card transition-colors hover:border-primary/40">
      <Link href={matterHref(matter.matter_id)} className="flex flex-1 flex-col gap-3 p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <span>{matter.matter_type.replace(/_/g, " ")}</span>
              <span className="text-border">/</span>
              <span>{matter.case_number ?? "no case #"}</span>
            </div>
            <h3 className="mt-0.5 line-clamp-1 text-base font-semibold text-foreground group-hover:text-primary">
              {matter.name}
            </h3>
            <div className="mt-0.5 line-clamp-1 font-mono text-[11px] text-muted-foreground">
              {matter.court}
            </div>
          </div>
          <span className={cn("rounded px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider", STATUS_CLS[matter.status])}>
            {matter.status}
          </span>
        </div>

        <div className="grid grid-cols-4 gap-2 border-t border-border pt-3">
          <Mini icon={Folder} label="docs" value={matter.document_count} />
          <Mini icon={Scale} label="claims" value={matter.claim_count} />
          <Mini icon={FileText} label="drafts" value={matter.draft_count} />
          <Mini icon={CalendarClock} label="tasks" value={matter.open_task_count} />
        </div>

        {matter.next_deadline ? (
          <div className="flex items-center justify-between rounded border border-border bg-background px-3 py-2">
            <div className="flex items-center gap-2">
              <AlertTriangle
                className={cn(
                  "h-3.5 w-3.5",
                  matter.next_deadline.days_remaining <= 7
                    ? "text-destructive"
                    : matter.next_deadline.days_remaining <= 21
                      ? "text-warning"
                      : "text-muted-foreground",
                )}
              />
              <span className="text-xs text-foreground">{matter.next_deadline.description}</span>
            </div>
            <div className="flex items-center gap-2 font-mono text-[10px] tabular-nums">
              <span className="text-muted-foreground">{matter.next_deadline.due_date}</span>
              <span
                className={cn(
                  "rounded px-1.5 py-0.5",
                  matter.next_deadline.days_remaining <= 7
                    ? "bg-destructive/15 text-destructive"
                    : matter.next_deadline.days_remaining <= 21
                      ? "bg-warning/15 text-warning"
                      : "bg-success/15 text-success",
                )}
              >
                {matter.next_deadline.days_remaining}d
              </span>
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-between rounded border border-dashed border-border px-3 py-2">
            <span className="text-xs text-muted-foreground">No critical deadline yet</span>
            <span className="font-mono text-[10px] uppercase text-muted-foreground">intake</span>
          </div>
        )}
      </Link>

      <div className="flex items-center justify-between gap-3 border-t border-border px-4 py-3 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
        <span>updated {new Date(matter.updated_at).toLocaleDateString()}</span>
        <div className="flex items-center gap-2">
          <Link
            href={matterHref(matter.matter_id)}
            className="inline-flex h-8 items-center gap-1 rounded border border-border bg-background px-2.5 hover:border-primary hover:text-primary"
          >
            Open
            <ArrowRight className="h-3 w-3" />
          </Link>
          <DeleteMatterButton matter={matter} compact className="h-8 px-2.5" />
        </div>
      </div>
    </article>
  )
}

function Mini({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof Folder
  label: string
  value: number
}) {
  return (
    <div className="flex flex-col gap-0.5">
      <div className="flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
        <Icon className="h-3 w-3" />
        {label}
      </div>
      <div className="font-mono text-sm font-semibold tabular-nums text-foreground">{value}</div>
    </div>
  )
}
