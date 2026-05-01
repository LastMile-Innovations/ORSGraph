"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { useMemo, useState } from "react"
import { AlertTriangle, CalendarClock, CheckCircle2, CheckSquare, Circle, FileText, Plus, Scale } from "lucide-react"
import type { CaseTask, Matter, TaskStatus } from "@/lib/casebuilder/types"
import { createTask, patchTask } from "@/lib/casebuilder/api"
import { matterClaimsHref, matterDocumentHref } from "@/lib/casebuilder/routes"
import { PriorityBadge, TaskStatusBadge } from "./badges"
import { cn } from "@/lib/utils"

const STATUSES: Array<{ status: TaskStatus; label: string; icon: typeof Circle }> = [
  { status: "todo", label: "To do", icon: Circle },
  { status: "in_progress", label: "In progress", icon: CalendarClock },
  { status: "blocked", label: "Blocked", icon: AlertTriangle },
  { status: "done", label: "Done", icon: CheckCircle2 },
]

export function TasksBoard({ matter }: { matter: Matter }) {
  const router = useRouter()
  const [tasks, setTasks] = useState(matter.tasks)
  const [title, setTitle] = useState("")
  const [description, setDescription] = useState("")
  const [priority, setPriority] = useState<"high" | "med" | "low">("med")
  const [dueDate, setDueDate] = useState("")
  const [saving, setSaving] = useState(false)
  const [message, setMessage] = useState<string | null>(null)

  const tasksByStatus = useMemo(
    () => STATUSES.map((status) => ({ ...status, tasks: tasks.filter((task) => task.status === status.status) })),
    [tasks],
  )
  const openTasks = tasks.filter((task) => task.status !== "done")
  const blockedTasks = tasks.filter((task) => task.status === "blocked")
  const dueSoon = openTasks.filter((task) => task.due_date && task.due_date <= "2026-05-14")

  async function addTask() {
    if (!title.trim()) {
      setMessage("Add a task title first.")
      return
    }
    setSaving(true)
    setMessage(null)
    const result = await createTask(matter.id, {
      title: title.trim(),
      description: description.trim() || undefined,
      priority,
      due_date: dueDate || undefined,
      status: "todo",
      source: "user",
    })
    setSaving(false)
    if (!result.data) {
      setMessage(result.error || "Task could not be created.")
      return
    }
    setTasks((current) => [result.data!, ...current])
    setTitle("")
    setDescription("")
    setPriority("med")
    setDueDate("")
    setMessage("Task added.")
    router.refresh()
  }

  async function updateTask(task: CaseTask, input: Partial<CaseTask>, success: string) {
    const result = await patchTask(matter.id, task.task_id, input)
    if (!result.data) {
      setMessage(result.error || "Task update failed.")
      return
    }
    setTasks((current) => current.map((item) => (item.task_id === task.task_id ? result.data! : item)))
    setMessage(success)
    router.refresh()
  }

  return (
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
              Create, complete, block, reopen, and prioritize matter work items.
            </p>
          </div>
          {message && (
            <div className="rounded border border-border bg-background px-3 py-2 text-xs text-muted-foreground">
              {message}
            </div>
          )}
        </div>

        <div className="mt-5 grid grid-cols-2 gap-px overflow-hidden rounded border border-border bg-border md:grid-cols-4">
          <Metric label="open" value={openTasks.length} />
          <Metric label="due soon" value={dueSoon.length} accent={dueSoon.length ? "text-warning" : "text-success"} />
          <Metric label="blocked" value={blockedTasks.length} accent={blockedTasks.length ? "text-destructive" : "text-success"} />
          <Metric label="complete" value={tasks.length - openTasks.length} accent="text-success" />
        </div>

        <div className="mt-4 grid gap-2 rounded border border-border bg-background p-3 md:grid-cols-[minmax(0,1fr)_120px_150px_auto]">
          <input
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder="Task title"
            className="rounded border border-border bg-card px-3 py-2 text-xs focus:border-primary focus:outline-none"
          />
          <select value={priority} onChange={(event) => setPriority(event.target.value as "high" | "med" | "low")} className="rounded border border-border bg-card px-3 py-2 font-mono text-xs">
            <option value="high">high</option>
            <option value="med">med</option>
            <option value="low">low</option>
          </select>
          <input
            type="date"
            value={dueDate}
            onChange={(event) => setDueDate(event.target.value)}
            className="rounded border border-border bg-card px-3 py-2 text-xs"
          />
          <button
            type="button"
            onClick={addTask}
            disabled={saving}
            className="inline-flex items-center justify-center gap-1.5 rounded bg-primary px-3 py-2 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90 disabled:opacity-60"
          >
            <Plus className="h-3.5 w-3.5" />
            {saving ? "saving" : "add"}
          </button>
          <textarea
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            placeholder="Description or next action"
            rows={2}
            className="rounded border border-border bg-card px-3 py-2 text-xs focus:border-primary focus:outline-none md:col-span-4"
          />
        </div>
      </header>

      <main className="px-6 py-6">
        <div className="grid grid-cols-1 gap-4 xl:grid-cols-4">
          {tasksByStatus.map(({ status, label, icon: Icon, tasks: columnTasks }) => (
            <section key={status} className="min-w-0 rounded border border-border bg-card">
              <div className="flex items-center justify-between border-b border-border px-3 py-2.5">
                <div className="flex items-center gap-2">
                  <Icon className="h-3.5 w-3.5 text-muted-foreground" />
                  <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{label}</h2>
                </div>
                <span className="font-mono text-[10px] tabular-nums text-muted-foreground">{columnTasks.length}</span>
              </div>
              <div className="space-y-2 p-3">
                {columnTasks.length === 0 ? (
                  <div className="rounded border border-dashed border-border px-3 py-6 text-center text-xs text-muted-foreground">
                    No tasks
                  </div>
                ) : (
                  columnTasks.map((task) => (
                    <TaskCard
                      key={task.task_id}
                      task={task}
                      matterId={matter.id}
                      onPatch={(input, success) => updateTask(task, input, success)}
                    />
                  ))
                )}
              </div>
            </section>
          ))}
        </div>
      </main>
    </div>
  )
}

function TaskCard({
  task,
  matterId,
  onPatch,
}: {
  task: CaseTask
  matterId: string
  onPatch: (input: Partial<CaseTask>, success: string) => void
}) {
  const nextStatus = task.status === "done" ? "todo" : "done"
  return (
    <article className="rounded border border-border bg-background p-3">
      <div className="flex items-start justify-between gap-3">
        <h3 className="text-sm font-medium leading-snug text-foreground">{task.title}</h3>
        <input
          type="checkbox"
          checked={task.status === "done"}
          onChange={() => onPatch({ status: nextStatus }, nextStatus === "done" ? "Task completed." : "Task reopened.")}
          className="mt-0.5 accent-primary"
        />
      </div>
      {task.description && <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{task.description}</p>}
      <div className="mt-3 flex flex-wrap items-center gap-1.5">
        <PriorityBadge priority={task.priority} />
        <TaskStatusBadge status={task.status} />
        {task.due_date && (
          <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] tabular-nums text-muted-foreground">
            due {task.due_date}
          </span>
        )}
      </div>
      <div className="mt-3 flex flex-wrap gap-1.5 border-t border-border pt-2">
        {(["todo", "in_progress", "blocked", "done"] as TaskStatus[]).map((status) => (
          <button
            key={status}
            type="button"
            onClick={() => onPatch({ status }, `Task moved to ${status.replace("_", " ")}.`)}
            className={cn(
              "rounded border px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider",
              task.status === status
                ? "border-primary text-primary"
                : "border-border text-muted-foreground hover:border-primary/40 hover:text-primary",
            )}
          >
            {status.replace("_", " ")}
          </button>
        ))}
      </div>
      {(task.related_claim_ids.length > 0 || task.related_document_ids.length > 0) && (
        <div className="mt-3 flex flex-wrap gap-1.5 border-t border-border pt-2">
          {task.related_claim_ids.map((claimId) => (
            <Link key={claimId} href={matterClaimsHref(matterId, claimId)} className="inline-flex items-center gap-1 rounded border border-border px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground hover:border-primary/40 hover:text-primary">
              <Scale className="h-2.5 w-2.5" />
              {claimId}
            </Link>
          ))}
          {task.related_document_ids.map((documentId) => (
            <Link key={documentId} href={matterDocumentHref(matterId, documentId)} className="inline-flex items-center gap-1 rounded border border-border px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground hover:border-primary/40 hover:text-primary">
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
      <div className={cn("mt-0.5 font-mono text-lg font-semibold tabular-nums", accent)}>{value.toLocaleString()}</div>
    </div>
  )
}
