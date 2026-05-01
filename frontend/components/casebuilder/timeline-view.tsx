"use client"

import { useMemo, useState } from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import {
  CalendarClock,
  FileText,
  AlertTriangle,
  Gavel,
  CheckCircle2,
  Filter,
  Download,
  Plus,
} from "lucide-react"
import type { Matter } from "@/lib/casebuilder/types"
import { matterDocumentHref, matterFactsHref, matterHref } from "@/lib/casebuilder/routes"
import { createTimelineEvent } from "@/lib/casebuilder/api"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Card } from "@/components/ui/card"
import { cn } from "@/lib/utils"

interface TimelineViewProps {
  matter: Matter
}

type TimelineEntry = {
  id: string
  date: string
  kind: "event" | "fact" | "document" | "deadline" | "milestone"
  title: string
  description?: string
  href?: string
  meta?: string
  disputed?: boolean
  status?: string
}

const KIND_CONFIG: Record<TimelineEntry["kind"], { color: string; icon: typeof FileText; label: string }> = {
  event: {
    color: "border-cyan-500/40 bg-cyan-500/10 text-cyan-700 dark:text-cyan-300",
    icon: CalendarClock,
    label: "Event",
  },
  fact: {
    color: "border-blue-500/40 bg-blue-500/10 text-blue-700 dark:text-blue-300",
    icon: CheckCircle2,
    label: "Fact",
  },
  document: {
    color: "border-purple-500/40 bg-purple-500/10 text-purple-700 dark:text-purple-300",
    icon: FileText,
    label: "Document",
  },
  deadline: {
    color: "border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300",
    icon: AlertTriangle,
    label: "Deadline",
  },
  milestone: {
    color: "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
    icon: Gavel,
    label: "Milestone",
  },
}

export function TimelineView({ matter }: TimelineViewProps) {
  const router = useRouter()
  const [activeKinds, setActiveKinds] = useState<Set<TimelineEntry["kind"]>>(
    new Set(["event", "fact", "document", "deadline", "milestone"]),
  )
  const [showCreate, setShowCreate] = useState(false)
  const [eventDate, setEventDate] = useState("")
  const [eventTitle, setEventTitle] = useState("")
  const [eventKind, setEventKind] = useState("other")
  const [eventDescription, setEventDescription] = useState("")
  const [sourceDocumentId, setSourceDocumentId] = useState("")
  const [linkedFactId, setLinkedFactId] = useState("")
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const entries: TimelineEntry[] = useMemo(() => {
    const out: TimelineEntry[] = []

    for (const event of matter.timeline) {
      out.push({
        id: event.id,
        date: event.date,
        kind: "event",
        title: event.title,
        description: event.description,
        disputed: event.disputed,
        status: event.status,
        meta: event.kind,
      })
    }
    for (const fact of matter.facts) {
      if (!fact.date) continue
      out.push({
        id: fact.id,
        date: fact.date,
        kind: "fact",
        title: fact.statement,
        meta: fact.tags.slice(0, 2).join(" · "),
        disputed: fact.disputed,
        href: matterFactsHref(matter.id, fact.id),
      })
    }
    for (const doc of matter.documents) {
      if (!doc.dateFiled && !doc.dateUploaded) continue
      out.push({
        id: doc.id,
        date: doc.dateFiled ?? doc.dateUploaded,
        kind: "document",
        title: doc.title,
        description: `${doc.kind} · ${doc.party}`,
        meta: doc.summary,
        href: matterDocumentHref(matter.id, doc.id),
      })
    }
    for (const deadline of matter.deadlines) {
      out.push({
        id: deadline.id,
        date: deadline.dueDate,
        kind: "deadline",
        title: deadline.title,
        description: deadline.description,
        status: deadline.status,
        meta: `Owner: ${deadline.owner}`,
        href: `${matterHref(matter.id, "deadlines")}#${encodeURIComponent(deadline.id)}`,
      })
    }
    for (const ms of matter.milestones) {
      out.push({
        id: ms.id,
        date: ms.date,
        kind: "milestone",
        title: ms.title,
        description: ms.description,
      })
    }

    return out
      .filter((e) => activeKinds.has(e.kind))
      .sort((a, b) => (a.date < b.date ? -1 : 1))
  }, [matter, activeKinds])

  const grouped = useMemo(() => {
    const map = new Map<string, TimelineEntry[]>()
    for (const e of entries) {
      const month = e.date.slice(0, 7) // YYYY-MM
      if (!map.has(month)) map.set(month, [])
      map.get(month)!.push(e)
    }
    return Array.from(map.entries())
  }, [entries])

  const toggleKind = (kind: TimelineEntry["kind"]) => {
    setActiveKinds((prev) => {
      const next = new Set(prev)
      if (next.has(kind)) next.delete(kind)
      else next.add(kind)
      return next
    })
  }

  async function onCreateEvent() {
    if (!eventDate || !eventTitle.trim()) {
      setError("Add a date and title before creating the event.")
      return
    }
    setSaving(true)
    setError(null)
    const result = await createTimelineEvent(matter.id, {
      date: eventDate,
      title: eventTitle.trim(),
      kind: eventKind,
      description: eventDescription.trim() || undefined,
      source_document_id: sourceDocumentId || undefined,
      linked_fact_ids: linkedFactId ? [linkedFactId] : [],
    })
    setSaving(false)
    if (!result.data) {
      setError(result.error || "Timeline event could not be created.")
      return
    }
    setShowCreate(false)
    setEventDate("")
    setEventTitle("")
    setEventKind("other")
    setEventDescription("")
    setSourceDocumentId("")
    setLinkedFactId("")
    router.refresh()
  }

  return (
    <div className="flex flex-col">
      {/* Header */}
      <div className="border-b border-border bg-background px-6 py-4">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <h1 className="text-xl font-semibold tracking-tight text-foreground">Timeline</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              {entries.length} events across {grouped.length} months
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button size="sm" className="gap-1.5" onClick={() => setShowCreate((value) => !value)}>
              <Plus className="h-3.5 w-3.5" />
              Add event
            </Button>
            <Button variant="outline" size="sm" className="gap-1.5 bg-transparent">
              <Download className="h-3.5 w-3.5" />
              Export
            </Button>
          </div>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Filter className="h-3 w-3" />
            Show:
          </span>
          {(Object.keys(KIND_CONFIG) as Array<TimelineEntry["kind"]>).map((kind) => {
            const cfg = KIND_CONFIG[kind]
            const Icon = cfg.icon
            const active = activeKinds.has(kind)
            return (
              <button
                key={kind}
                onClick={() => toggleKind(kind)}
                className={cn(
                  "flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-medium transition-colors",
                  active
                    ? cfg.color
                    : "border-border bg-background text-muted-foreground hover:bg-muted",
                )}
              >
                <Icon className="h-3 w-3" />
                {cfg.label}
              </button>
            )
          })}
        </div>

        {showCreate && (
          <div className="mt-4 grid gap-3 rounded-md border border-border bg-card p-3 md:grid-cols-[140px_minmax(0,1fr)_160px]">
            <input
              type="date"
              value={eventDate}
              onChange={(event) => setEventDate(event.target.value)}
              className="rounded border border-border bg-background px-3 py-2 text-xs focus:border-primary focus:outline-none"
            />
            <input
              value={eventTitle}
              onChange={(event) => setEventTitle(event.target.value)}
              placeholder="Event title"
              className="rounded border border-border bg-background px-3 py-2 text-xs focus:border-primary focus:outline-none"
            />
            <select
              value={eventKind}
              onChange={(event) => setEventKind(event.target.value)}
              className="rounded border border-border bg-background px-3 py-2 font-mono text-xs"
            >
              {["other", "communication", "filing", "service", "payment", "notice", "incident", "meeting", "court"].map((kind) => (
                <option key={kind} value={kind}>
                  {kind}
                </option>
              ))}
            </select>
            <textarea
              value={eventDescription}
              onChange={(event) => setEventDescription(event.target.value)}
              placeholder="Description or notes"
              rows={3}
              className="rounded border border-border bg-background px-3 py-2 text-xs focus:border-primary focus:outline-none md:col-span-3"
            />
            <select
              value={sourceDocumentId}
              onChange={(event) => setSourceDocumentId(event.target.value)}
              className="rounded border border-border bg-background px-3 py-2 text-xs md:col-span-2"
            >
              <option value="">No source document</option>
              {matter.documents.map((document) => (
                <option key={document.id} value={document.id}>
                  {document.title}
                </option>
              ))}
            </select>
            <select
              value={linkedFactId}
              onChange={(event) => setLinkedFactId(event.target.value)}
              className="rounded border border-border bg-background px-3 py-2 text-xs"
            >
              <option value="">No linked fact</option>
              {matter.facts.map((fact) => (
                <option key={fact.id} value={fact.id}>
                  {fact.statement.slice(0, 80)}
                </option>
              ))}
            </select>
            <div className="flex items-center justify-between gap-3 md:col-span-3">
              <p className="text-xs text-destructive">{error}</p>
              <Button size="sm" onClick={onCreateEvent} disabled={saving}>
                {saving ? "Saving" : "Create event"}
              </Button>
            </div>
          </div>
        )}
      </div>

      {/* Timeline */}
      <ScrollArea className="h-[calc(100vh-200px)]">
        <div className="mx-auto max-w-4xl px-6 py-8">
          {grouped.length === 0 && (
            <Card className="flex flex-col items-center gap-2 border-dashed bg-transparent p-12 text-center">
              <CalendarClock className="h-8 w-8 text-muted-foreground" />
              <p className="text-sm font-medium text-foreground">No timeline events</p>
              <p className="text-xs text-muted-foreground">
                Adjust filters or add facts and deadlines.
              </p>
            </Card>
          )}

          {grouped.map(([month, items]) => (
            <section key={month} className="relative">
              <div className="sticky top-0 z-10 -mx-6 mb-4 bg-background/95 px-6 py-2 backdrop-blur">
                <h2 className="font-mono text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                  {formatMonth(month)} · {items.length} event{items.length === 1 ? "" : "s"}
                </h2>
              </div>

              <ol className="relative space-y-4 border-l-2 border-border pl-6">
                {items.map((entry) => (
                  <TimelineItem key={`${entry.kind}-${entry.id}`} entry={entry} />
                ))}
              </ol>
            </section>
          ))}
        </div>
      </ScrollArea>
    </div>
  )
}

function TimelineItem({ entry }: { entry: TimelineEntry }) {
  const cfg = KIND_CONFIG[entry.kind]
  const Icon = cfg.icon

  const content = (
    <div className="space-y-1.5">
      <div className="flex items-center gap-2 text-[11px]">
        <span className="font-mono text-muted-foreground">{entry.date}</span>
        <Badge variant="outline" className={cn("text-[9px] uppercase tracking-wider", cfg.color)}>
          {cfg.label}
        </Badge>
        {entry.disputed && (
          <Badge variant="outline" className="border-amber-500/40 text-[9px] text-amber-600 dark:text-amber-400">
            Disputed
          </Badge>
        )}
        {entry.status && (
          <Badge variant="outline" className="text-[9px] capitalize">
            {entry.status}
          </Badge>
        )}
      </div>
      <p className="text-sm font-medium leading-snug text-foreground text-pretty">{entry.title}</p>
      {entry.description && (
        <p className="text-xs leading-relaxed text-muted-foreground">{entry.description}</p>
      )}
      {entry.meta && (
        <p className="font-mono text-[10px] text-muted-foreground/80">{entry.meta}</p>
      )}
    </div>
  )

  return (
    <li className="relative">
      <span
        className={cn(
          "absolute -left-[33px] top-1 flex h-5 w-5 items-center justify-center rounded-full border-2 border-background ring-1 ring-border",
          cfg.color,
        )}
      >
        <Icon className="h-2.5 w-2.5" />
      </span>
      {entry.href ? (
        <Link
          href={entry.href}
          className="block rounded-md border border-border bg-card p-3 transition-colors hover:border-foreground/20 hover:bg-muted/30"
        >
          {content}
        </Link>
      ) : (
        <div className="rounded-md border border-border bg-card p-3">{content}</div>
      )}
    </li>
  )
}

function formatMonth(month: string): string {
  const [y, m] = month.split("-")
  const date = new Date(Number(y), Number(m) - 1, 1)
  return date.toLocaleDateString("en-US", { month: "long", year: "numeric" })
}
