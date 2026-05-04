"use client"

import { useEffect, useMemo, useState, type ReactNode } from "react"
import Link from "next/link"
import { usePathname, useRouter, useSearchParams } from "next/navigation"
import {
  AlertTriangle,
  CalendarClock,
  CheckCircle2,
  Download,
  ExternalLink,
  FileText,
  Filter,
  Gavel,
  Pencil,
  Plus,
  RotateCcw,
  Save,
  Sparkles,
  X,
} from "lucide-react"
import type { Matter, TimelineSuggestion } from "@/lib/casebuilder/types"
import {
  matterDocumentHref,
  matterFactsHref,
  matterHref,
  matterWorkProductHref,
} from "@/lib/casebuilder/routes"
import { approveTimelineSuggestion, createTimelineEvent, patchTimelineSuggestion, suggestTimeline } from "@/lib/casebuilder/api"
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

type SuggestionDraft = {
  date: string
  dateText: string
  title: string
  description: string
  kind: string
  status: string
  sourceDocumentId: string
  sourceSpanIds: string
  textChunkIds: string
  linkedFactIds: string
  linkedClaimIds: string
  warnings: string
}

const REVIEW_STATUSES = ["suggested", "approved", "rejected", "disputed", "all"]
const EVENT_KINDS = ["other", "communication", "filing", "service", "payment", "notice", "incident", "meeting", "court"]

const KIND_CONFIG: Record<TimelineEntry["kind"], { color: string; icon: typeof FileText; label: string }> = {
  event: {
    color: "border-case-timeline/40 bg-case-timeline/10 text-case-timeline",
    icon: CalendarClock,
    label: "Event",
  },
  fact: {
    color: "border-case-evidence/40 bg-case-evidence/10 text-case-evidence",
    icon: CheckCircle2,
    label: "Fact",
  },
  document: {
    color: "border-case-document/40 bg-case-document/10 text-case-document",
    icon: FileText,
    label: "Document",
  },
  deadline: {
    color: "border-case-deadline/40 bg-case-deadline/10 text-case-deadline",
    icon: AlertTriangle,
    label: "Deadline",
  },
  milestone: {
    color: "border-case-authority/40 bg-case-authority/10 text-case-authority",
    icon: Gavel,
    label: "Milestone",
  },
}

export function TimelineView({ matter }: TimelineViewProps) {
  const router = useRouter()
  const pathname = usePathname()
  const searchParams = useSearchParams()
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
  const [reviewStatus, setReviewStatus] = useState(() => safeReviewStatus(searchParams.get("status")))
  const [sourceType, setSourceType] = useState(() => searchParams.get("source") ?? "all")
  const [agentRunId, setAgentRunId] = useState(() => searchParams.get("agentRun") ?? "all")
  const [pendingSuggestionId, setPendingSuggestionId] = useState<string | null>(null)
  const [reviewMessage, setReviewMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setReviewStatus(safeReviewStatus(searchParams.get("status")))
    setSourceType(searchParams.get("source") ?? "all")
    setAgentRunId(searchParams.get("agentRun") ?? "all")
  }, [searchParams])

  const documentsById = useMemo(() => {
    const map = new Map<string, Matter["documents"][number]>()
    for (const document of matter.documents) {
      map.set(document.id, document)
      map.set(document.document_id, document)
    }
    return map
  }, [matter.documents])

  const factsById = useMemo(() => {
    const map = new Map<string, Matter["facts"][number]>()
    for (const fact of matter.facts) {
      map.set(fact.id, fact)
      if (fact.fact_id) map.set(fact.fact_id, fact)
    }
    return map
  }, [matter.facts])

  const claimsById = useMemo(() => {
    const map = new Map<string, Matter["claims"][number]>()
    for (const claim of matter.claims) {
      map.set(claim.id, claim)
      const claimId = (claim as { claim_id?: string }).claim_id
      if (claimId) map.set(claimId, claim)
    }
    return map
  }, [matter.claims])

  const sourceTypes = useMemo(() => {
    return Array.from(new Set((matter.timeline_suggestions ?? []).map((suggestion) => suggestion.source_type))).sort()
  }, [matter.timeline_suggestions])

  const agentRunIds = useMemo(() => {
    return Array.from(
      new Set(
        [
          ...(matter.timeline_agent_runs ?? []).map((run) => run.agent_run_id),
          ...(matter.timeline_suggestions ?? []).map((suggestion) => suggestion.agent_run_id),
        ].filter((value): value is string => Boolean(value)),
      ),
    ).sort()
  }, [matter.timeline_agent_runs, matter.timeline_suggestions])

  const latestAgentRun = useMemo(() => {
    return [...(matter.timeline_agent_runs ?? [])].sort((left, right) => {
      const leftTime = agentRunTimestamp(left)
      const rightTime = agentRunTimestamp(right)
      return rightTime - leftTime
    })[0]
  }, [matter.timeline_agent_runs])

  const filteredSuggestions = useMemo(() => {
    return (matter.timeline_suggestions ?? []).filter((suggestion) => {
      if (reviewStatus === "disputed" && suggestion.warnings.length === 0) return false
      if (reviewStatus !== "all" && reviewStatus !== "disputed" && suggestion.status !== reviewStatus) return false
      if (sourceType !== "all" && suggestion.source_type !== sourceType) return false
      if (agentRunId !== "all" && suggestion.agent_run_id !== agentRunId) return false
      return true
    })
  }, [agentRunId, matter.timeline_suggestions, reviewStatus, sourceType])

  const pendingSuggestionCount = useMemo(() => {
    return (matter.timeline_suggestions ?? []).filter(
      (suggestion) => suggestion.status === "suggested" || suggestion.status === "needs_attention",
    ).length
  }, [matter.timeline_suggestions])

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
      const month = e.date.slice(0, 7)
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

  function updateReviewFilters(next: { status?: string; source?: string; agentRun?: string; suggestionId?: string }) {
    const nextStatus = next.status ?? reviewStatus
    const nextSource = next.source ?? sourceType
    const nextAgentRun = next.agentRun ?? agentRunId
    setReviewStatus(safeReviewStatus(nextStatus))
    setSourceType(nextSource)
    setAgentRunId(nextAgentRun)
    const params = new URLSearchParams(searchParams.toString())
    setOptionalParam(params, "status", nextStatus, "suggested")
    setOptionalParam(params, "source", nextSource, "all")
    setOptionalParam(params, "agentRun", nextAgentRun, "all")
    const query = params.toString()
    const hash = next.suggestionId ? `#${encodeURIComponent(next.suggestionId)}` : ""
    router.replace(`${pathname}${query ? `?${query}` : ""}${hash}`, { scroll: false })
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
    setReviewMessage("Timeline event created.")
    router.refresh()
  }

  async function onSuggestTimeline() {
    setSaving(true)
    setError(null)
    setReviewMessage(null)
    const result = await suggestTimeline(matter.id, { limit: 100 })
    setSaving(false)
    if (!result.data) {
      setError(result.error || "Timeline suggestions could not be generated.")
      return
    }
    const first = result.data.suggestions[0]
    const providerMode = result.data.agent_run?.provider_mode ?? result.data.mode
    setReviewMessage(`${result.data.suggestions.length} timeline suggestion${result.data.suggestions.length === 1 ? "" : "s"} ready for review (${providerMode}).`)
    if (first) {
      updateReviewFilters({
        status: "suggested",
        source: first.source_type,
        agentRun: first.agent_run_id ?? result.data.agent_run?.agent_run_id ?? "all",
        suggestionId: first.suggestion_id,
      })
    }
    router.refresh()
  }

  async function onPatchSuggestion(suggestion: TimelineSuggestion, draft: SuggestionDraft) {
    setPendingSuggestionId(suggestion.suggestion_id)
    setError(null)
    setReviewMessage(null)
    const result = await patchTimelineSuggestion(matter.id, suggestion.suggestion_id, patchFromDraft(draft))
    setPendingSuggestionId(null)
    if (!result.data) {
      setError(result.error || "Timeline suggestion could not be updated.")
      return false
    }
    setReviewMessage("Timeline suggestion updated.")
    updateReviewFilters({
      status: result.data.status,
      source: result.data.source_type,
      agentRun: result.data.agent_run_id ?? "all",
      suggestionId: result.data.suggestion_id,
    })
    router.refresh()
    return true
  }

  async function onApproveSuggestion(suggestion: TimelineSuggestion) {
    setPendingSuggestionId(suggestion.suggestion_id)
    setError(null)
    setReviewMessage(null)
    const result = await approveTimelineSuggestion(matter.id, suggestion.suggestion_id)
    setPendingSuggestionId(null)
    if (!result.data) {
      setError(result.error || "Timeline suggestion could not be approved.")
      return
    }
    setReviewMessage("Timeline event approved.")
    updateReviewFilters({
      status: "approved",
      source: result.data.suggestion.source_type,
      agentRun: result.data.suggestion.agent_run_id ?? "all",
      suggestionId: result.data.suggestion.suggestion_id,
    })
    router.refresh()
  }

  async function onRejectSuggestion(suggestion: TimelineSuggestion) {
    setPendingSuggestionId(suggestion.suggestion_id)
    setError(null)
    setReviewMessage(null)
    const result = await patchTimelineSuggestion(matter.id, suggestion.suggestion_id, { status: "rejected" })
    setPendingSuggestionId(null)
    if (!result.data) {
      setError(result.error || "Timeline suggestion could not be rejected.")
      return
    }
    setReviewMessage("Timeline suggestion rejected.")
    updateReviewFilters({
      status: "rejected",
      source: result.data.source_type,
      agentRun: result.data.agent_run_id ?? "all",
      suggestionId: result.data.suggestion_id,
    })
    router.refresh()
  }

  return (
    <div className="flex flex-col">
      <div className="border-b border-border bg-background px-6 py-4">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <h1 className="text-xl font-semibold tracking-tight text-foreground">Timeline</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              {entries.length} events across {grouped.length} months · {pendingSuggestionCount} suggestion{pendingSuggestionCount === 1 ? "" : "s"} waiting
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button size="sm" className="gap-1.5" onClick={() => setShowCreate((value) => !value)}>
              <Plus className="h-3.5 w-3.5" />
              Add event
            </Button>
            <Button variant="outline" size="sm" className="gap-1.5 bg-transparent" onClick={onSuggestTimeline} disabled={saving}>
              <Sparkles className="h-3.5 w-3.5" />
              {saving ? "Suggesting" : "Suggest"}
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5 bg-transparent"
              disabled
              title="Timeline export will be available after review queue export is wired."
            >
              <Download className="h-3.5 w-3.5" />
              Export soon
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
                  active ? cfg.color : "border-border bg-background text-muted-foreground hover:bg-muted",
                )}
              >
                <Icon className="h-3 w-3" />
                {cfg.label}
              </button>
            )
          })}
        </div>

        <section className="mt-4 rounded-md border border-border bg-card" aria-label="Timeline suggestion review queue">
          <div className="space-y-3 border-b border-border px-3 py-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h2 className="text-sm font-medium text-foreground">Review queue</h2>
                <p className="text-xs text-muted-foreground">
                  {filteredSuggestions.length} candidate{filteredSuggestions.length === 1 ? "" : "s"} from graph, documents, and AST sources
                </p>
              </div>
              {agentRunIds.length > 0 && (
                <select
                  value={agentRunId}
                  onChange={(event) => updateReviewFilters({ agentRun: event.target.value })}
                  className="rounded border border-border bg-background px-2 py-1 font-mono text-[11px]"
                  aria-label="Filter by agent run"
                >
                  <option value="all">all agent runs</option>
                  {agentRunIds.map((value) => (
                    <option key={value} value={value}>
                      {shortId(value)}
                    </option>
                  ))}
                </select>
              )}
            </div>
            {latestAgentRun && (
              <div className="grid gap-2 rounded-md border border-border bg-background/60 p-2 text-xs md:grid-cols-[minmax(0,1fr)_auto]">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-1.5">
                    <Badge variant="outline" className="text-[9px] uppercase">
                      {latestAgentRun.provider_mode}
                    </Badge>
                    <Badge variant="outline" className="text-[9px]">
                      {latestAgentRun.provider || "disabled"}
                    </Badge>
                    {latestAgentRun.model && (
                      <Badge variant="outline" className="text-[9px]">
                        {latestAgentRun.model}
                      </Badge>
                    )}
                    <span className="font-medium text-foreground">{latestAgentRun.status}</span>
                    <span className="text-muted-foreground">scope {latestAgentRun.scope_type || latestAgentRun.subject_type}</span>
                  </div>
                  <p className="mt-1 truncate text-muted-foreground">{latestAgentRun.message}</p>
                  {latestAgentRun.warnings.length > 0 && (
                    <p className="mt-1 text-warning">{latestAgentRun.warnings[0]}</p>
                  )}
                </div>
                <div className="flex flex-wrap gap-1.5 font-mono text-[11px] text-muted-foreground md:justify-end">
                  <span>det {latestAgentRun.deterministic_candidate_count}</span>
                  <span>stored {latestAgentRun.stored_suggestion_count}</span>
                  <span>enriched {latestAgentRun.provider_enriched_count}</span>
                  <span>rejected {latestAgentRun.provider_rejected_count}</span>
                </div>
              </div>
            )}
            <div className="flex flex-wrap gap-2">
              {REVIEW_STATUSES.map((value) => (
                <FilterChip
                  key={value}
                  active={reviewStatus === value}
                  label={value}
                  onClick={() => updateReviewFilters({ status: value })}
                />
              ))}
            </div>
            <div className="flex flex-wrap gap-2">
              <FilterChip active={sourceType === "all"} label="all sources" onClick={() => updateReviewFilters({ source: "all" })} />
              {sourceTypes.map((value) => (
                <FilterChip key={value} active={sourceType === value} label={value} onClick={() => updateReviewFilters({ source: value })} />
              ))}
            </div>
          </div>
          <div className="max-h-[36rem] space-y-3 overflow-y-auto p-3">
            {filteredSuggestions.map((suggestion) => (
              <TimelineSuggestionCard
                key={suggestion.suggestion_id}
                suggestion={suggestion}
                documentsById={documentsById}
                factsById={factsById}
                claimsById={claimsById}
                pending={pendingSuggestionId === suggestion.suggestion_id}
                onSave={(draft) => onPatchSuggestion(suggestion, draft)}
                onApprove={() => onApproveSuggestion(suggestion)}
                onReject={() => onRejectSuggestion(suggestion)}
              />
            ))}
            {filteredSuggestions.length === 0 && (
              <div className="rounded border border-dashed border-border p-4 text-sm text-muted-foreground">
                No suggestions match the current filters.
              </div>
            )}
          </div>
        </section>
        {(reviewMessage || error) && (
          <p className={cn("mt-2 text-xs", error ? "text-destructive" : "text-muted-foreground")}>{error || reviewMessage}</p>
        )}

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
              {EVENT_KINDS.map((kind) => (
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

function FilterChip({ active, label, onClick }: { active: boolean; label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "rounded-full border px-2.5 py-1 text-[11px] font-medium transition-colors",
        active ? "border-primary/40 bg-primary/10 text-primary" : "border-border bg-background text-muted-foreground hover:bg-muted",
      )}
    >
      {label}
    </button>
  )
}

function TimelineSuggestionCard({
  suggestion,
  documentsById,
  factsById,
  claimsById,
  pending,
  onSave,
  onApprove,
  onReject,
}: {
  suggestion: TimelineSuggestion
  documentsById: Map<string, Matter["documents"][number]>
  factsById: Map<string, Matter["facts"][number]>
  claimsById: Map<string, Matter["claims"][number]>
  pending: boolean
  onSave: (draft: SuggestionDraft) => Promise<boolean>
  onApprove: () => void
  onReject: () => void
}) {
  const approved = suggestion.status === "approved"
  const rejected = suggestion.status === "rejected"
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(() => draftFromSuggestion(suggestion))

  useEffect(() => {
    if (!editing) setDraft(draftFromSuggestion(suggestion))
  }, [editing, suggestion])

  const baseline = draftFromSuggestion(suggestion)
  const dirty = !draftsEqual(draft, baseline)
  const document = suggestion.source_document_id ? documentsById.get(suggestion.source_document_id) : null
  const linkedFacts = suggestion.linked_fact_ids.map((id) => factsById.get(id)).filter(Boolean)
  const linkedClaims = suggestion.linked_claim_ids.map((id) => claimsById.get(id)).filter(Boolean)
  const sourceHref = suggestion.source_document_id
    ? matterDocumentHref(suggestion.matter_id, suggestion.source_document_id, suggestion.source_span_ids[0])
    : null
  const workProductHref = suggestion.work_product_id
    ? matterWorkProductHref(
        suggestion.matter_id,
        suggestion.work_product_id,
        undefined,
        suggestion.block_id ? { id: suggestion.block_id } : undefined,
      )
    : null

  async function saveDraft() {
    const saved = await onSave(draft)
    if (saved) setEditing(false)
  }

  return (
    <article id={suggestion.suggestion_id} tabIndex={-1} className="rounded-md border border-border bg-background p-3 scroll-mt-32">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 space-y-1">
          <div className="flex flex-wrap items-center gap-1.5 font-mono text-[10px] text-muted-foreground">
            <span>{suggestion.date}</span>
            <span>source date: {suggestion.date_text}</span>
            <Badge variant="outline" className="text-[9px]">
              {Math.round(suggestion.date_confidence * 100)}%
            </Badge>
            <Badge variant="outline" className="text-[9px]">
              {suggestion.source_type}
            </Badge>
            {suggestion.agent_run_id && (
              <Badge variant="outline" className="text-[9px]">
                agent {shortId(suggestion.agent_run_id)}
              </Badge>
            )}
            {suggestion.index_run_id && (
              <Badge variant="outline" className="text-[9px]">
                index {shortId(suggestion.index_run_id)}
              </Badge>
            )}
            {suggestion.warnings.length > 0 && (
              <Badge variant="outline" className="border-warning/40 text-[9px] text-warning">
                review
              </Badge>
            )}
          </div>
          <h3 className="text-sm font-medium leading-snug text-foreground">{suggestion.title}</h3>
        </div>
        <Badge variant={approved ? "default" : rejected ? "secondary" : "outline"} className="shrink-0 text-[9px] uppercase">
          {suggestion.status}
        </Badge>
      </div>

      {editing ? (
        <div className="mt-3 grid gap-2 md:grid-cols-2">
          <LabeledInput label="Date">
            <input className={fieldClassName} type="date" value={draft.date} onChange={(event) => setDraft({ ...draft, date: event.target.value })} />
          </LabeledInput>
          <LabeledInput label="Date text">
            <input className={fieldClassName} value={draft.dateText} onChange={(event) => setDraft({ ...draft, dateText: event.target.value })} />
          </LabeledInput>
          <LabeledInput label="Title">
            <input className={fieldClassName} value={draft.title} onChange={(event) => setDraft({ ...draft, title: event.target.value })} />
          </LabeledInput>
          <LabeledInput label="Kind">
            <select className={fieldClassName} value={draft.kind} onChange={(event) => setDraft({ ...draft, kind: event.target.value })}>
              {EVENT_KINDS.map((kind) => (
                <option key={kind} value={kind}>
                  {kind}
                </option>
              ))}
            </select>
          </LabeledInput>
          <LabeledInput label="Status">
            <select className={fieldClassName} value={draft.status} onChange={(event) => setDraft({ ...draft, status: event.target.value })}>
              {["suggested", "approved", "rejected", "needs_attention"].map((status) => (
                <option key={status} value={status}>
                  {status}
                </option>
              ))}
            </select>
          </LabeledInput>
          <LabeledInput label="Source document">
            <select
              className={fieldClassName}
              value={draft.sourceDocumentId}
              onChange={(event) => setDraft({ ...draft, sourceDocumentId: event.target.value })}
            >
              <option value="">No source document</option>
              {Array.from(documentsById.values())
                .filter((document, index, all) => all.findIndex((candidate) => candidate.id === document.id) === index)
                .map((document) => (
                  <option key={document.id} value={document.id}>
                    {document.title}
                  </option>
                ))}
            </select>
          </LabeledInput>
          <LabeledTextarea label="Source quote" value={draft.description} onChange={(value) => setDraft({ ...draft, description: value })} />
          <LabeledTextarea label="Warnings" value={draft.warnings} onChange={(value) => setDraft({ ...draft, warnings: value })} />
          <LabeledTextarea label="Source span IDs" value={draft.sourceSpanIds} onChange={(value) => setDraft({ ...draft, sourceSpanIds: value })} />
          <LabeledTextarea label="Text chunk IDs" value={draft.textChunkIds} onChange={(value) => setDraft({ ...draft, textChunkIds: value })} />
          <LabeledTextarea label="Linked fact IDs" value={draft.linkedFactIds} onChange={(value) => setDraft({ ...draft, linkedFactIds: value })} />
          <LabeledTextarea label="Linked claim IDs" value={draft.linkedClaimIds} onChange={(value) => setDraft({ ...draft, linkedClaimIds: value })} />
        </div>
      ) : (
        <div className="mt-3 grid gap-3 md:grid-cols-[minmax(0,1fr)_260px]">
          <div className="space-y-3">
            {suggestion.description && (
              <blockquote className="rounded border border-border bg-muted/30 px-3 py-2 text-xs leading-relaxed text-foreground">
                {suggestion.description}
              </blockquote>
            )}
            {suggestion.warnings.length > 0 && (
              <div className="rounded border border-warning/30 bg-warning/5 px-3 py-2 text-xs text-warning">
                {suggestion.warnings.join(" · ")}
              </div>
            )}
            {(suggestion.agent_explanation || suggestion.cluster_id || suggestion.agent_confidence != null) && (
              <div className="rounded border border-info/30 bg-info/5 px-3 py-2 text-xs text-info">
                <div className="flex flex-wrap gap-2 font-mono text-[10px] uppercase">
                  {suggestion.cluster_id && <span>cluster {shortId(suggestion.cluster_id)}</span>}
                  {suggestion.agent_confidence != null && <span>agent {Math.round(suggestion.agent_confidence * 100)}%</span>}
                  {suggestion.duplicate_of_suggestion_id && <span>duplicate {shortId(suggestion.duplicate_of_suggestion_id)}</span>}
                </div>
                {suggestion.agent_explanation && <p className="mt-1 leading-relaxed">{suggestion.agent_explanation}</p>}
              </div>
            )}
            <div className="flex flex-wrap gap-1.5">
              {suggestion.source_span_ids.map((id) => (
                <IdBadge key={id} label="span" value={id} />
              ))}
              {suggestion.text_chunk_ids.map((id) => (
                <IdBadge key={id} label="chunk" value={id} />
              ))}
              {suggestion.work_product_id && <IdBadge label="work product" value={suggestion.work_product_id} />}
              {suggestion.block_id && <IdBadge label="block" value={suggestion.block_id} />}
            </div>
          </div>
          <div className="space-y-2 rounded border border-border bg-card p-2 text-xs">
            <MetadataRow label="Document" value={document?.title ?? suggestion.source_document_id ?? "None"} href={sourceHref} />
            <MetadataRow
              label="Facts"
              value={linkedFacts.length ? linkedFacts.map((fact) => fact?.statement).join(" · ") : suggestion.linked_fact_ids.join(", ") || "None"}
            />
            <MetadataRow
              label="Claims"
              value={linkedClaims.length ? linkedClaims.map((claim) => claim?.title).join(" · ") : suggestion.linked_claim_ids.join(", ") || "None"}
            />
            {workProductHref && <MetadataRow label="AST source" value={suggestion.block_id ?? suggestion.work_product_id ?? "Open"} href={workProductHref} />}
            {suggestion.dedupe_key && <MetadataRow label="Dedupe" value={shortId(suggestion.dedupe_key)} />}
          </div>
        </div>
      )}

      <div className="mt-3 flex flex-wrap items-center gap-2">
        {editing ? (
          <>
            <Button size="sm" variant="outline" className="h-7 gap-1 bg-transparent text-xs" onClick={saveDraft} disabled={pending || !dirty}>
              <Save className="h-3.5 w-3.5" />
              Save changes
            </Button>
            <Button size="sm" variant="ghost" className="h-7 gap-1 text-xs" onClick={() => setDraft(baseline)} disabled={pending || !dirty}>
              <RotateCcw className="h-3.5 w-3.5" />
              Reset
            </Button>
            <Button size="sm" variant="ghost" className="h-7 text-xs" onClick={() => setEditing(false)} disabled={pending}>
              Done
            </Button>
          </>
        ) : (
          <Button size="sm" variant="ghost" className="h-7 gap-1 text-xs" onClick={() => setEditing(true)} disabled={pending}>
            <Pencil className="h-3.5 w-3.5" />
            Edit
          </Button>
        )}
        <Button
          size="sm"
          variant="outline"
          className="h-7 gap-1 bg-transparent text-xs"
          onClick={onApprove}
          disabled={pending || approved || dirty}
          title={dirty ? "Save edits before approving." : undefined}
        >
          <CheckCircle2 className="h-3.5 w-3.5" />
          Approve
        </Button>
        <Button size="sm" variant="ghost" className="h-7 gap-1 text-xs" onClick={onReject} disabled={pending || rejected}>
          <X className="h-3.5 w-3.5" />
          Reject
        </Button>
        {sourceHref && (
          <Link href={sourceHref} className="ml-auto inline-flex items-center gap-1 text-xs text-primary hover:underline">
            source
            <ExternalLink className="h-3 w-3" />
          </Link>
        )}
      </div>
    </article>
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
          <Badge variant="outline" className="border-warning/40 text-[9px] text-warning">
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
      {entry.description && <p className="text-xs leading-relaxed text-muted-foreground">{entry.description}</p>}
      {entry.meta && <p className="font-mono text-[10px] text-muted-foreground/80">{entry.meta}</p>}
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

function LabeledInput({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="space-y-1 text-[11px] font-medium text-muted-foreground">
      <span>{label}</span>
      {children}
    </label>
  )
}

function LabeledTextarea({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <label className="space-y-1 text-[11px] font-medium text-muted-foreground">
      <span>{label}</span>
      <textarea className={cn(fieldClassName, "min-h-16")} value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  )
}

function MetadataRow({ label, value, href }: { label: string; value: string; href?: string | null }) {
  return (
    <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-2">
      <span className="text-muted-foreground">{label}</span>
      {href ? (
        <Link href={href} className="truncate text-primary hover:underline">
          {value}
        </Link>
      ) : (
        <span className="truncate text-foreground">{value}</span>
      )}
    </div>
  )
}

function IdBadge({ label, value }: { label: string; value: string }) {
  return (
    <Badge variant="outline" className="max-w-full gap-1 text-[9px]">
      <span className="text-muted-foreground">{label}</span>
      <span className="truncate font-mono">{shortId(value)}</span>
    </Badge>
  )
}

const fieldClassName = "w-full rounded border border-border bg-background px-2 py-1.5 text-xs text-foreground focus:border-primary focus:outline-none"

function draftFromSuggestion(suggestion: TimelineSuggestion): SuggestionDraft {
  return {
    date: suggestion.date,
    dateText: suggestion.date_text,
    title: suggestion.title,
    description: suggestion.description ?? "",
    kind: suggestion.kind,
    status: suggestion.status,
    sourceDocumentId: suggestion.source_document_id ?? "",
    sourceSpanIds: idsToText(suggestion.source_span_ids),
    textChunkIds: idsToText(suggestion.text_chunk_ids),
    linkedFactIds: idsToText(suggestion.linked_fact_ids),
    linkedClaimIds: idsToText(suggestion.linked_claim_ids),
    warnings: idsToText(suggestion.warnings),
  }
}

function patchFromDraft(draft: SuggestionDraft) {
  return {
    date: draft.date,
    date_text: draft.dateText,
    title: draft.title,
    description: draft.description || null,
    kind: draft.kind,
    status: draft.status,
    source_document_id: draft.sourceDocumentId || null,
    source_span_ids: textToIds(draft.sourceSpanIds),
    text_chunk_ids: textToIds(draft.textChunkIds),
    linked_fact_ids: textToIds(draft.linkedFactIds),
    linked_claim_ids: textToIds(draft.linkedClaimIds),
    warnings: textToIds(draft.warnings),
  }
}

function draftsEqual(a: SuggestionDraft, b: SuggestionDraft) {
  return JSON.stringify(a) === JSON.stringify(b)
}

function idsToText(values: string[]) {
  return values.join("\n")
}

function textToIds(value: string) {
  return value
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean)
}

function setOptionalParam(params: URLSearchParams, key: string, value: string, emptyValue: string) {
  if (!value || value === emptyValue) params.delete(key)
  else params.set(key, value)
}

function safeReviewStatus(value: string | null) {
  return value && REVIEW_STATUSES.includes(value) ? value : "suggested"
}

function agentRunTimestamp(run: Matter["timeline_agent_runs"][number]) {
  return Date.parse(run.completed_at ?? run.started_at ?? run.created_at ?? "") || 0
}

function shortId(value: string) {
  if (value.length <= 28) return value
  return `${value.slice(0, 16)}…${value.slice(-8)}`
}

function formatMonth(month: string): string {
  const [y, m] = month.split("-")
  const date = new Date(Number(y), Number(m) - 1, 1)
  return date.toLocaleDateString("en-US", { month: "long", year: "numeric" })
}
