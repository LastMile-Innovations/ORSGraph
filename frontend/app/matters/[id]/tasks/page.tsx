import Link from "next/link"
import { notFound } from "next/navigation"
import { AlertTriangle, CalendarClock, CheckCircle2, CheckSquare, Circle, FileText, Scale } from "lucide-react"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { PriorityBadge, TaskStatusBadge } from "@/components/casebuilder/badges"
import { getMatterState } from "@/lib/casebuilder/api"
import { matterClaimsHref, matterDocumentHref } from "@/lib/casebuilder/routes"
import type { CaseTask, TaskStatus } from "@/lib/casebuilder/types"
import { cn } from "@/lib/utils"

interface PageProps {
  params: Promise<{ id: string }>
}

const STATUSES: Array<{ status: TaskStatus; label: string; icon: typeof Circle }> = [
  { status: "todo", label: "To do", icon: Circle },
  { status: "in_progress", label: "In progress", icon: CalendarClock },
  { status: "blocked", label: "Blocked", icon: AlertTriangle },
  { status: "done", label: "Done", icon: CheckCircle2 },
]

export default async function TasksPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  const tasksByStatus = STATUSES.map((status) => ({
    ...status,
    tasks: matter.tasks.filter((task) => task.status === status.status),
  }))
  const openTasks = matter.tasks.filter((task) => task.status !== "done")
  const blockedTasks = matter.tasks.filter((task) => task.status === "blocked")
  const dueSoon = openTasks.filter((task) => task.due_date && task.due_date <= "2026-05-14")

  return (
    <MatterShell matter={matter} activeSection="tasks" dataState={matterState}>
      <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
        <header className="border-b border-border bg-card px-6 py-5">
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div>
              <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                <CheckSquare className="h-3 w-3 text-primary" />
                work queue
              </div>
              <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Tasks</h1>
              <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
                Matter work items generated from deadlines, document gaps, and user-created follow-ups.
              </p>
            </div>
            <div className="rounded border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
              Local task state is read-only until the task mutation API is connected.
            </div>
          </div>

          <div className="mt-5 grid grid-cols-2 gap-px overflow-hidden rounded border border-border bg-border md:grid-cols-4">
            <Metric label="open" value={openTasks.length} />
            <Metric label="due soon" value={dueSoon.length} accent={dueSoon.length ? "text-warning" : "text-success"} />
            <Metric label="blocked" value={blockedTasks.length} accent={blockedTasks.length ? "text-destructive" : "text-success"} />
            <Metric label="complete" value={matter.tasks.length - openTasks.length} accent="text-success" />
          </div>
        </header>

        <main className="px-6 py-6">
          <div className="grid grid-cols-1 gap-4 xl:grid-cols-4">
            {tasksByStatus.map(({ status, label, icon: Icon, tasks }) => (
              <section key={status} className="min-w-0 rounded border border-border bg-card">
                <div className="flex items-center justify-between border-b border-border px-3 py-2.5">
                  <div className="flex items-center gap-2">
                    <Icon className="h-3.5 w-3.5 text-muted-foreground" />
                    <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                      {label}
                    </h2>
                  </div>
                  <span className="font-mono text-[10px] tabular-nums text-muted-foreground">
                    {tasks.length}
                  </span>
                </div>
                <div className="space-y-2 p-3">
                  {tasks.length === 0 ? (
                    <div className="rounded border border-dashed border-border px-3 py-6 text-center text-xs text-muted-foreground">
                      No tasks
                    </div>
                  ) : (
                    tasks.map((task) => <TaskCard key={task.task_id} task={task} matterId={matter.id} />)
                  )}
                </div>
              </section>
            ))}
          </div>
        </main>
      </div>
    </MatterShell>
  )
}

function TaskCard({ task, matterId }: { task: CaseTask; matterId: string }) {
  return (
    <article className="rounded border border-border bg-background p-3">
      <div className="flex items-start justify-between gap-3">
        <h3 className="text-sm font-medium leading-snug text-foreground">{task.title}</h3>
        <input type="checkbox" checked={task.status === "done"} readOnly className="mt-0.5 accent-primary" />
      </div>
      {task.description && (
        <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{task.description}</p>
      )}
      <div className="mt-3 flex flex-wrap items-center gap-1.5">
        <PriorityBadge priority={task.priority} />
        <TaskStatusBadge status={task.status} />
        {task.due_date && (
          <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] tabular-nums text-muted-foreground">
            due {task.due_date}
          </span>
        )}
      </div>
      {(task.related_claim_ids.length > 0 || task.related_document_ids.length > 0) && (
        <div className="mt-3 flex flex-wrap gap-1.5 border-t border-border pt-2">
          {task.related_claim_ids.map((claimId) => (
            <Link
              key={claimId}
              href={matterClaimsHref(matterId, claimId)}
              className="inline-flex items-center gap-1 rounded border border-border px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground hover:border-primary/40 hover:text-primary"
            >
              <Scale className="h-2.5 w-2.5" />
              {claimId}
            </Link>
          ))}
          {task.related_document_ids.map((documentId) => (
            <Link
              key={documentId}
              href={matterDocumentHref(matterId, documentId)}
              className="inline-flex items-center gap-1 rounded border border-border px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground hover:border-primary/40 hover:text-primary"
            >
              <FileText className="h-2.5 w-2.5" />
              {documentId}
            </Link>
          ))}
        </div>
      )}
    </article>
  )
}

function Metric({ label, value, accent = "text-foreground" }: { label: string; value: number; accent?: string }) {
  return (
    <div className="bg-card px-4 py-3">
      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div className={cn("mt-0.5 font-mono text-lg font-semibold tabular-nums", accent)}>
        {value.toLocaleString()}
      </div>
    </div>
  )
}
