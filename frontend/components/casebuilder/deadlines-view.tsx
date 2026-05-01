"use client"

import { useMemo, useState } from "react"
import {
  Plus,
  CalendarClock,
  AlertTriangle,
  CheckCircle2,
  Clock,
  Bell,
  Filter,
  Sparkles,
} from "lucide-react"
import type { Matter, Deadline } from "@/lib/casebuilder/types"
import { computeDeadlines, createDeadline, patchDeadline } from "@/lib/casebuilder/api"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Card } from "@/components/ui/card"
import { Checkbox } from "@/components/ui/checkbox"
import { cn } from "@/lib/utils"

interface DeadlinesViewProps {
  matter: Matter
}

type Filter = "all" | "upcoming" | "overdue" | "complete"

export function DeadlinesView({ matter }: DeadlinesViewProps) {
  const [deadlines, setDeadlines] = useState(matter.deadlines)
  const [filter, setFilter] = useState<Filter>("upcoming")
  const [message, setMessage] = useState<string | null>(null)
  const today = useMemo(() => new Date().toISOString().slice(0, 10), [])

  const filteredDeadlines = useMemo(() => {
    return deadlines
      .filter((d) => {
        if (filter === "all") return true
        if (filter === "complete") return d.status === "complete"
        if (filter === "overdue") return d.status !== "complete" && d.dueDate < today
        if (filter === "upcoming") return d.status !== "complete" && d.dueDate >= today
        return true
      })
      .sort((a, b) => (a.dueDate < b.dueDate ? -1 : 1))
  }, [deadlines, filter, today])

  const counts = useMemo(() => {
    const upcoming = deadlines.filter(
      (d) => d.status !== "complete" && d.dueDate >= today,
    ).length
    const overdue = deadlines.filter(
      (d) => d.status !== "complete" && d.dueDate < today,
    ).length
    const complete = deadlines.filter((d) => d.status === "complete").length
    return { upcoming, overdue, complete, all: deadlines.length }
  }, [deadlines, today])

  async function autoCompute() {
    const result = await computeDeadlines(matter.id)
    if (result.data) {
      setDeadlines((current) => [...result.data!.generated, ...current])
      setMessage(result.data.warnings[0] ?? `${result.data.generated.length} deadlines computed.`)
    } else {
      setMessage(result.error || "Deadline compute failed.")
    }
  }

  async function addDeadline() {
    const title = window.prompt("Deadline title")
    if (!title) return
    const dueDate = window.prompt("Due date (YYYY-MM-DD)", today)
    if (!dueDate) return
    const result = await createDeadline(matter.id, {
      title,
      due_date: dueDate,
      description: "",
      category: "case",
      kind: "manual",
      severity: "info",
      source: "manual",
    })
    if (result.data) {
      setDeadlines((current) => [result.data!, ...current])
      setMessage("Deadline added.")
    } else {
      setMessage(result.error || "Deadline could not be added.")
    }
  }

  async function toggleDeadline(deadline: Deadline) {
    const nextStatus = deadline.status === "complete" ? "open" : "complete"
    const result = await patchDeadline(matter.id, deadline.id, { status: nextStatus })
    if (result.data) {
      setDeadlines((current) => current.map((item) => (item.id === deadline.id ? result.data! : item)))
    } else {
      setMessage(result.error || "Deadline update failed.")
    }
  }

  return (
    <div className="flex flex-col">
      <div className="border-b border-border bg-background px-6 py-4">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <h1 className="text-xl font-semibold tracking-tight text-foreground">
              Deadlines & Tasks
            </h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Court rules, statutes of limitation, and case management orders feed this calendar.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" className="gap-1.5 bg-transparent" onClick={autoCompute}>
              <Sparkles className="h-3.5 w-3.5" />
              Auto-compute
            </Button>
            <Button size="sm" className="gap-1.5" onClick={addDeadline}>
              <Plus className="h-3.5 w-3.5" />
              Add deadline
            </Button>
          </div>
        </div>
        {message && <div className="mt-3 rounded border border-border bg-card px-3 py-2 text-xs text-muted-foreground">{message}</div>}

        <div className="mt-4 grid grid-cols-2 gap-2 md:grid-cols-4">
          <SummaryCard label="Overdue" count={counts.overdue} tone="rose" />
          <SummaryCard label="Upcoming" count={counts.upcoming} tone="amber" />
          <SummaryCard label="Complete" count={counts.complete} tone="emerald" />
          <SummaryCard label="Total" count={counts.all} tone="neutral" />
        </div>

        <div className="mt-4 flex items-center gap-2">
          <Filter className="h-3 w-3 text-muted-foreground" />
          <FilterChip active={filter === "upcoming"} onClick={() => setFilter("upcoming")}>
            Upcoming
          </FilterChip>
          <FilterChip active={filter === "overdue"} onClick={() => setFilter("overdue")}>
            Overdue
          </FilterChip>
          <FilterChip active={filter === "complete"} onClick={() => setFilter("complete")}>
            Complete
          </FilterChip>
          <FilterChip active={filter === "all"} onClick={() => setFilter("all")}>
            All
          </FilterChip>
        </div>
      </div>

      <ScrollArea className="h-[calc(100vh-280px)]">
        <div className="mx-auto max-w-4xl px-6 py-6">
          {filteredDeadlines.length === 0 ? (
            <Card className="flex flex-col items-center gap-2 border-dashed bg-transparent p-12 text-center">
              <CalendarClock className="h-8 w-8 text-muted-foreground" />
              <p className="text-sm font-medium text-foreground">Nothing in this view</p>
              <p className="text-xs text-muted-foreground">
                Switch filters or add a new deadline.
              </p>
            </Card>
          ) : (
            <ul className="space-y-2">
              {filteredDeadlines.map((d) => (
                <DeadlineRow key={d.id} deadline={d} today={today} onToggle={() => toggleDeadline(d)} />
              ))}
            </ul>
          )}
        </div>
      </ScrollArea>
    </div>
  )
}

function FilterChip({
  active,
  onClick,
  children,
}: {
  active: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "rounded-full border px-2.5 py-1 text-[11px] font-medium transition-colors",
        active
          ? "border-foreground bg-foreground text-background"
          : "border-border bg-background text-muted-foreground hover:bg-muted",
      )}
    >
      {children}
    </button>
  )
}

function SummaryCard({
  label,
  count,
  tone,
}: {
  label: string
  count: number
  tone: "rose" | "amber" | "emerald" | "neutral"
}) {
  const colors: Record<typeof tone, string> = {
    rose: "border-rose-500/40 bg-rose-500/5 text-rose-700 dark:text-rose-300",
    amber: "border-amber-500/40 bg-amber-500/5 text-amber-700 dark:text-amber-300",
    emerald: "border-emerald-500/40 bg-emerald-500/5 text-emerald-700 dark:text-emerald-300",
    neutral: "border-border bg-card text-foreground",
  }
  return (
    <div className={cn("rounded-md border p-3", colors[tone])}>
      <div className="text-[11px] font-medium uppercase tracking-wider">{label}</div>
      <div className="mt-1 font-mono text-2xl font-semibold tabular-nums">{count}</div>
    </div>
  )
}

function DeadlineRow({ deadline, today, onToggle }: { deadline: Deadline; today: string; onToggle: () => void }) {
  const isComplete = deadline.status === "complete"
  const isOverdue = !isComplete && deadline.dueDate < today
  const daysUntil = useMemo(() => {
    const diff =
      (new Date(deadline.dueDate).getTime() - new Date(today).getTime()) /
      (1000 * 60 * 60 * 24)
    return Math.round(diff)
  }, [deadline.dueDate, today])

  const dateColor = isComplete
    ? "text-muted-foreground"
    : isOverdue
      ? "text-rose-700 dark:text-rose-400"
      : daysUntil <= 7
        ? "text-amber-700 dark:text-amber-400"
        : "text-foreground"

  return (
    <li
      id={deadline.id}
      className={cn(
        "group flex items-start gap-3 rounded-md border bg-card p-3 transition-colors hover:border-foreground/20",
        isOverdue ? "border-rose-500/40" : "border-border",
        isComplete && "opacity-60",
      )}
    >
      <Checkbox
        className="mt-0.5"
        checked={isComplete}
        aria-label={`Mark ${deadline.title} complete`}
        onCheckedChange={onToggle}
      />
      <div className="min-w-0 flex-1">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <p
              className={cn(
                "text-sm font-semibold leading-tight text-foreground text-pretty",
                isComplete && "line-through",
              )}
            >
              {deadline.title}
            </p>
            <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">
              {deadline.description}
            </p>
          </div>
          <div className="flex shrink-0 flex-col items-end gap-1">
            <div className={cn("flex items-center gap-1.5 text-xs font-mono font-medium", dateColor)}>
              <CalendarClock className="h-3 w-3" />
              {deadline.dueDate}
            </div>
            {!isComplete && (
              <span className={cn("text-[10px]", dateColor)}>
                {isOverdue
                  ? `${Math.abs(daysUntil)}d overdue`
                  : daysUntil === 0
                    ? "Due today"
                    : `in ${daysUntil}d`}
              </span>
            )}
          </div>
        </div>

        <div className="mt-2 flex flex-wrap items-center gap-2">
          {deadline.computedFrom && (
            <Badge variant="outline" className="gap-1 text-[10px]">
              <Sparkles className="h-2.5 w-2.5" />
              {deadline.computedFrom}
            </Badge>
          )}
          <Badge variant="secondary" className="text-[10px] capitalize">
            {deadline.kind.replace(/-/g, " ")}
          </Badge>
          <span className="text-[11px] text-muted-foreground">
            Owner: <span className="font-medium text-foreground">{deadline.owner}</span>
          </span>
          {deadline.statuteRef && (
            <span className="font-mono text-[10px] text-muted-foreground">
              {deadline.statuteRef}
            </span>
          )}
          {isOverdue && (
            <Badge variant="outline" className="gap-1 border-rose-500/40 text-[10px] text-rose-700 dark:text-rose-400">
              <AlertTriangle className="h-2.5 w-2.5" />
              Overdue
            </Badge>
          )}
          {isComplete && (
            <Badge className="gap-1 bg-emerald-600/15 text-emerald-700 hover:bg-emerald-600/15 dark:text-emerald-300">
              <CheckCircle2 className="h-2.5 w-2.5" />
              Complete
            </Badge>
          )}
        </div>

        {deadline.tasks && deadline.tasks.length > 0 && (
          <ul className="mt-3 space-y-1 border-l-2 border-border pl-3">
            {deadline.tasks.map((task) => (
              <li
                key={task.id}
                className="flex items-center gap-2 text-[11px] text-muted-foreground"
              >
                {task.done ? (
                  <CheckCircle2 className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
                ) : (
                  <Clock className="h-3 w-3" />
                )}
                <span className={cn(task.done && "line-through")}>{task.label}</span>
                {task.assignee && (
                  <span className="text-foreground/70">· {task.assignee}</span>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      <Button
        size="sm"
        variant="ghost"
        className="h-7 w-7 shrink-0 p-0 opacity-0 transition-opacity group-hover:opacity-100"
        aria-label="Set reminder"
      >
        <Bell className="h-3.5 w-3.5" />
      </Button>
    </li>
  )
}
