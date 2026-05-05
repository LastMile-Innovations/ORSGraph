import Link from "next/link"
import {
  AlertTriangle,
  ArrowRight,
  Calendar,
  CheckCircle2,
  ClipboardList,
  FileText,
  Folder,
  GavelIcon,
  Microscope,
  Scale,
  Sparkles,
  Upload,
  Users,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { formatOptionalDate } from "@/lib/casebuilder/display"
import { isValidDateValue } from "@/lib/casebuilder/readiness"
import { matterHref } from "@/lib/casebuilder/routes"
import { DeleteMatterButton } from "./delete-matter-button"
import type {
  CaseClaim,
  CaseDefense,
  CaseDocument,
  CaseDraft,
  CaseEvent,
  CaseFact,
  CaseDeadline,
  CaseTask,
  TimelineSuggestion,
  MatterParty,
  MatterSummary,
} from "@/lib/casebuilder/types"
import { ClaimStatusBadge, DefenseStatusBadge, FactStatusBadge, PriorityBadge, RiskBadge, TaskStatusBadge } from "./badges"

interface Props {
  matter: MatterSummary
  parties: MatterParty[]
  documents: CaseDocument[]
  facts: CaseFact[]
  events: CaseEvent[]
  claims: CaseClaim[]
  defenses: CaseDefense[]
  deadlines: CaseDeadline[]
  tasks: CaseTask[]
  drafts: CaseDraft[]
  timelineSuggestions?: TimelineSuggestion[]
}

export function MatterDashboard({
  matter,
  parties,
  documents,
  facts,
  events,
  claims,
  defenses,
  deadlines,
  tasks,
  drafts,
  timelineSuggestions = [],
}: Props) {
  const factBreakdown = facts.reduce<Record<string, number>>((acc, f) => {
    acc[f.status] = (acc[f.status] ?? 0) + 1
    return acc
  }, {})
  const supportedFacts = factBreakdown.supported ?? 0
  const factHealth = facts.length > 0 ? Math.round((supportedFacts / facts.length) * 100) : 0

  const openTasks = tasks.filter((t) => t.status !== "done")
  const factsNeedingReview = facts.filter((fact) => fact.status === "proposed" || fact.needs_verification || fact.confidence < 0.7).length
  const today = new Date().toISOString().slice(0, 10)
  const upcomingEvents = [...events]
    .filter((e) => isValidDateValue(e.date) && e.date >= today)
    .sort((a, b) => a.date.localeCompare(b.date))
    .slice(0, 5)
  const pendingTimelineSuggestions = timelineSuggestions.filter((suggestion) => suggestion.status === "suggested" || suggestion.status === "needs_attention")
  const invalidTimelineItems = events.filter((event) => !isValidDateValue(event.date)).length + timelineSuggestions.filter((suggestion) => !isValidDateValue(suggestion.date)).length
  const criticalDeadlines = deadlines.filter((d) => d.severity === "critical" && d.status === "open")
  const recentDocs = [...documents].sort((a, b) => b.uploaded_at.localeCompare(a.uploaded_at)).slice(0, 5)
  const legalTheoryCount = claims.length + defenses.length
  const authorityLinks =
    claims.reduce((sum, claim) => sum + (claim.authorities?.length ?? 0), 0)
    + defenses.reduce((sum, defense) => sum + (defense.authorities?.length ?? 0), 0)

  const base = matterHref(matter.matter_id)
  const setupRecommendations = [
    factsNeedingReview > 0
      ? {
          title: `Review ${factsNeedingReview} extracted fact${factsNeedingReview === 1 ? "" : "s"}`,
          body: "Approve, edit, or reject extracted facts before relying on them.",
          href: `${base}/facts`,
        }
      : null,
    pendingTimelineSuggestions.length > 0
      ? {
          title: `Review ${pendingTimelineSuggestions.length} timeline suggestion${pendingTimelineSuggestions.length === 1 ? "" : "s"}`,
          body: "Promote accurate suggestions and repair uncertain dates.",
          href: `${base}/timeline`,
        }
      : null,
    legalTheoryCount === 0
      ? {
          title: "Create claims or defenses",
          body: "Evidence, authorities, drafts, and QC checks need legal theories.",
          href: `${base}/claims`,
        }
      : null,
    legalTheoryCount > 0 && authorityLinks === 0
      ? {
          title: "Link authorities",
          body: "Legal theories exist, but none have source-backed authorities attached.",
          href: `${base}/authorities`,
        }
      : null,
    deadlines.length === 0
      ? {
          title: "Add or compute deadlines",
          body: "Deadline surfaces need court, case, and trigger-date inputs.",
          href: `${base}/deadlines`,
        }
      : null,
  ].filter(Boolean) as Array<{ title: string; body: string; href: string }>

  return (
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
      {/* Hero / matter header */}
      <header className="border-b border-border bg-card px-6 py-5">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
          <div className="min-w-0">
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <GavelIcon className="h-3 w-3 text-primary" />
              <span>matter</span>
              <span className="text-border">/</span>
              <span>{matter.case_number ?? "no case #"}</span>
              <span className="text-border">/</span>
              <span>{matter.court}</span>
            </div>
            <h1 className="mt-1 text-balance text-xl font-semibold tracking-tight text-foreground">
              {matter.name}
            </h1>
            <div className="mt-1 flex flex-wrap items-center gap-3 font-mono text-[10px] tabular-nums text-muted-foreground">
              <span>created {formatOptionalDate(matter.created_at)}</span>
              <span className="text-border">|</span>
              <span>updated {formatOptionalDate(matter.updated_at)}</span>
              <span className="text-border">|</span>
              <span>jurisdiction: {matter.jurisdiction}</span>
              <span className="text-border">|</span>
              <span>your role: {matter.user_role}</span>
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            <Link
              href={`${base}/ask`}
              className="flex items-center gap-1.5 rounded bg-primary px-3 py-1.5 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90"
            >
              <Sparkles className="h-3.5 w-3.5" />
              ask matter
            </Link>
            <Link
              href={`${base}/documents`}
              className="flex items-center gap-1.5 rounded border border-border bg-background px-3 py-1.5 font-mono text-xs uppercase tracking-wider hover:border-primary hover:text-primary"
            >
              <Upload className="h-3.5 w-3.5" />
              add files
            </Link>
            <Link
              href={`${base}/drafts`}
              className="flex items-center gap-1.5 rounded border border-border bg-background px-3 py-1.5 font-mono text-xs uppercase tracking-wider hover:border-primary hover:text-primary"
            >
              <FileText className="h-3.5 w-3.5" />
              drafts
            </Link>
            <DeleteMatterButton matter={matter} className="h-auto px-3 py-1.5" />
          </div>
        </div>

        {/* Stat tiles */}
        <div className="mt-5 grid grid-cols-2 gap-px overflow-hidden rounded border border-border bg-border md:grid-cols-4 lg:grid-cols-9">
          <Stat icon={Folder} label="documents" value={documents.length} />
          <Stat icon={Users} label="parties" value={parties.length} />
          <Stat icon={Microscope} label="facts" value={facts.length} />
          <Stat icon={Calendar} label="events / suggestions" value={`${events.length} / ${pendingTimelineSuggestions.length}`} />
          <Stat icon={Scale} label="claims" value={claims.length} accent="text-primary" />
          <Stat icon={ClipboardList} label="defenses" value={defenses.length} accent="text-accent" />
          <Stat icon={ClipboardList} label="open tasks" value={openTasks.length} accent={openTasks.length > 0 ? "text-warning" : "text-foreground"} />
          <Stat icon={FileText} label="drafts" value={drafts.length} />
          <Stat
            icon={AlertTriangle}
            label="critical deadlines"
            value={criticalDeadlines.length}
            accent={criticalDeadlines.length > 0 ? "text-destructive" : "text-foreground"}
          />
        </div>
      </header>

      {/* Two-column body */}
      <div className="flex-1 px-6 py-6">
        <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
          {/* LEFT: Deadlines + Tasks + Documents */}
          <div className="space-y-4 xl:col-span-1">
            <Panel
              title="critical deadlines"
              icon={AlertTriangle}
              tone="destructive"
              action={{ href: `${base}/deadlines`, label: "all deadlines" }}
            >
              <div className="space-y-2">
                {criticalDeadlines.length === 0 && (
                  <div className="rounded border border-dashed border-border p-3 text-center text-xs text-muted-foreground">
                    No critical deadlines
                  </div>
                )}
                {criticalDeadlines.map((d) => (
                  <div
                    key={d.deadline_id}
                    className={cn(
                      "rounded border bg-background p-3",
                      d.days_remaining <= 7 ? "border-destructive/40 bg-destructive/5" : "border-border",
                    )}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <div className="text-xs text-foreground">{d.description}</div>
                        <div className="mt-0.5 font-mono text-[10px] tabular-nums text-muted-foreground">
                          due {d.due_date} · {d.source_citation}
                        </div>
                      </div>
                      <span
                        className={cn(
                          "flex-shrink-0 rounded px-1.5 py-0.5 font-mono text-[10px] tabular-nums uppercase",
                          d.days_remaining <= 7
                            ? "bg-destructive/15 text-destructive"
                            : d.days_remaining <= 21
                              ? "bg-warning/15 text-warning"
                              : "bg-success/15 text-success",
                        )}
                      >
                        {d.days_remaining}d
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </Panel>

            <Panel
              title="open tasks"
              icon={ClipboardList}
              action={{ href: `${base}/tasks`, label: "all tasks" }}
            >
              <div className="space-y-1">
                {openTasks.length === 0 && (
                  <div className="rounded border border-dashed border-border p-3 text-center text-xs text-muted-foreground">
                    No saved tasks
                    {setupRecommendations.length > 0 ? ` · ${setupRecommendations.length} recommended setup item${setupRecommendations.length === 1 ? "" : "s"}` : ""}
                  </div>
                )}
                {openTasks.slice(0, 5).map((t) => (
                  <div
                    key={t.task_id}
                    className="flex items-start gap-2 rounded border border-border bg-background p-2"
                  >
                    <div className="mt-0.5 flex-shrink-0">
                      <input type="checkbox" className="accent-primary" disabled />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="text-xs text-foreground">{t.title}</div>
                      <div className="mt-1 flex flex-wrap items-center gap-1.5">
                        <PriorityBadge priority={t.priority} />
                        <TaskStatusBadge status={t.status} />
                        {t.due_date && (
                          <span className="font-mono text-[10px] tabular-nums text-muted-foreground">
                            due {t.due_date}
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
                {setupRecommendations.length > 0 && (
                  <div className="mt-2 space-y-1.5">
                    <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">recommended setup</div>
                    {setupRecommendations.slice(0, 4).map((item) => (
                      <Link
                        key={item.title}
                        href={item.href}
                        className="block rounded border border-warning/25 bg-warning/5 p-2 text-xs hover:border-warning/50"
                      >
                        <div className="font-medium text-foreground">{item.title}</div>
                        <div className="mt-0.5 line-clamp-2 text-muted-foreground">{item.body}</div>
                      </Link>
                    ))}
                  </div>
                )}
              </div>
            </Panel>

            <Panel
              title="recent documents"
              icon={Folder}
              action={{ href: `${base}/documents`, label: "all documents" }}
            >
              <div className="space-y-1">
                {recentDocs.map((d) => (
                  <Link
                    key={d.document_id}
                    href={`${base}/documents/${d.document_id}`}
                    className="flex items-center gap-2 rounded border border-border bg-background p-2 hover:border-primary/40"
                  >
                    <FileText className="h-3.5 w-3.5 flex-shrink-0 text-muted-foreground" />
                    <div className="min-w-0 flex-1">
                      <div className="truncate font-mono text-[11px] text-foreground">{d.filename}</div>
                      <div className="mt-0.5 flex items-center gap-2 font-mono text-[10px] tabular-nums text-muted-foreground">
                        <span className="uppercase">{d.document_type}</span>
                        <span>·</span>
                        <span>{d.facts_extracted} facts</span>
                        {d.contradictions_flagged > 0 && (
                          <span className="text-destructive">{d.contradictions_flagged} flags</span>
                        )}
                      </div>
                    </div>
                  </Link>
                ))}
              </div>
            </Panel>
          </div>

          {/* CENTER: Claims & Defenses */}
          <div className="space-y-4 xl:col-span-1">
            <Panel
              title="claims (counterclaims)"
              icon={Scale}
              action={{ href: `${base}/claims`, label: "manage" }}
            >
              <div className="space-y-2">
                {claims.map((c) => {
                  const satisfied = c.elements.filter((e) => e.satisfied).length
                  const pct = c.elements.length > 0 ? Math.round((satisfied / c.elements.length) * 100) : 0
                  return (
                    <Link
                      key={c.claim_id}
                      href={`${base}/claims#${c.claim_id}`}
                      className="block rounded border border-border bg-background p-3 hover:border-primary/40"
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div>
                          <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                            {c.count_label}
                          </div>
                          <div className="mt-0.5 text-xs font-medium text-foreground">{c.name}</div>
                        </div>
                        <div className="flex items-center gap-1">
                          <ClaimStatusBadge status={c.status} />
                          <RiskBadge level={c.risk_level ?? c.risk} />
                        </div>
                      </div>
                      <div className="mt-2">
                        <div className="flex items-center justify-between font-mono text-[10px] tabular-nums text-muted-foreground">
                          <span>elements satisfied</span>
                          <span>
                            {satisfied}/{c.elements.length}
                          </span>
                        </div>
                        <div className="mt-1 h-1 overflow-hidden rounded bg-border">
                          <div
                            className={cn(
                              "h-full",
                              pct >= 80 ? "bg-success" : pct >= 50 ? "bg-primary" : "bg-warning",
                            )}
                            style={{ width: `${pct}%` }}
                          />
                        </div>
                      </div>
                    </Link>
                  )
                })}
              </div>
            </Panel>

            <Panel
              title="defenses"
              icon={ClipboardList}
              action={{ href: `${base}/claims`, label: "manage" }}
            >
              <div className="space-y-2">
                {defenses.map((d) => (
                  <Link
                    key={d.defense_id}
                    href={`${base}/claims#${d.defense_id}`}
                    className="block rounded border border-border bg-background p-3 hover:border-primary/40"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <div className="text-xs font-medium text-foreground">{d.name}</div>
                        <div className="mt-0.5 line-clamp-2 text-[11px] text-muted-foreground">{d.basis}</div>
                      </div>
                      <div className="flex flex-shrink-0 flex-col items-end gap-1">
                        <DefenseStatusBadge status={d.status} />
                        <RiskBadge level={d.viability} />
                      </div>
                    </div>
                  </Link>
                ))}
              </div>
            </Panel>
          </div>

          {/* RIGHT: Facts + Drafts + Timeline */}
          <div className="space-y-4 xl:col-span-1">
            <Panel
              title="fact health"
              icon={CheckCircle2}
              action={{ href: `${base}/facts`, label: "fact table" }}
            >
              <div className="rounded border border-border bg-background p-3">
                <div className="flex items-baseline justify-between font-mono">
                  <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
                    supported / total
                  </span>
                  <span className="text-lg font-semibold tabular-nums text-foreground">
                    {supportedFacts}/{facts.length}
                  </span>
                </div>
                <div className="mt-1 h-1.5 overflow-hidden rounded bg-border">
                  <div
                    className={cn(
                      "h-full",
                      factHealth >= 70 ? "bg-success" : factHealth >= 40 ? "bg-primary" : "bg-warning",
                    )}
                    style={{ width: `${factHealth}%` }}
                  />
                </div>
                <div className="mt-3 grid grid-cols-2 gap-1">
                  {Object.entries(factBreakdown).map(([k, v]) => (
                    <div key={k} className="flex items-center justify-between rounded bg-muted/30 px-2 py-1">
                      <FactStatusBadge status={k as never} />
                      <span className="font-mono text-[11px] tabular-nums text-foreground">{v}</span>
                    </div>
                  ))}
                </div>
              </div>
            </Panel>

            <Panel
              title="drafts in flight"
              icon={FileText}
              action={{ href: `${base}/drafts`, label: "all drafts" }}
            >
              <div className="space-y-2">
                {drafts.map((d) => {
                  const total = Object.values(d.factcheck_summary).reduce((a, b) => a + b, 0)
                  const supported = d.factcheck_summary.supported
                  const issues = total - supported - d.factcheck_summary.unchecked
                  const pct = total > 0 ? Math.round((supported / total) * 100) : 0
                  return (
                    <Link
                      key={d.draft_id}
                      href={`${base}/drafts/${d.draft_id}`}
                      className="block rounded border border-border bg-background p-3 hover:border-primary/40"
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <div className="text-xs font-medium text-foreground">{d.title}</div>
                          <div className="mt-0.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                            {d.draft_type} · {d.word_count} words · {d.paragraphs.length} paragraphs
                          </div>
                        </div>
                        <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                          {d.status}
                        </span>
                      </div>
                      <div className="mt-2 grid grid-cols-3 gap-2 font-mono text-[10px] tabular-nums">
                        <div className="rounded bg-success/10 px-2 py-1 text-success">
                          <div className="uppercase tracking-wider opacity-70">supported</div>
                          <div className="text-sm font-semibold">{supported}</div>
                        </div>
                        <div className="rounded bg-warning/10 px-2 py-1 text-warning">
                          <div className="uppercase tracking-wider opacity-70">issues</div>
                          <div className="text-sm font-semibold">{issues}</div>
                        </div>
                        <div className="rounded bg-muted/40 px-2 py-1 text-muted-foreground">
                          <div className="uppercase tracking-wider opacity-70">unchecked</div>
                          <div className="text-sm font-semibold">{d.factcheck_summary.unchecked}</div>
                        </div>
                      </div>
                      <div className="mt-2 flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                        <span>fact-check coverage</span>
                        <span className="ml-auto tabular-nums text-foreground">{pct}%</span>
                      </div>
                    </Link>
                  )
                })}
              </div>
            </Panel>

            <Panel
              title="upcoming timeline"
              icon={Calendar}
              action={{ href: `${base}/timeline`, label: "full timeline" }}
            >
              {(pendingTimelineSuggestions.length > 0 || invalidTimelineItems > 0) && (
                <div className="mb-3 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
                  {pendingTimelineSuggestions.length} suggestion{pendingTimelineSuggestions.length === 1 ? "" : "s"} awaiting review
                  {invalidTimelineItems > 0 ? ` · ${invalidTimelineItems} invalid date item${invalidTimelineItems === 1 ? "" : "s"} need repair` : ""}
                </div>
              )}
              <ol className="relative space-y-3 border-l border-border pl-4">
                {upcomingEvents.map((e) => (
                  <li key={e.event_id} className="relative">
                    <span
                      className={cn(
                        "absolute -left-[1.2rem] top-1 h-2 w-2 rounded-full",
                        e.disputed ? "bg-warning" : e.category === "court_event" ? "bg-primary" : "bg-muted-foreground",
                      )}
                    />
                    <div className="font-mono text-[10px] tabular-nums uppercase tracking-wider text-muted-foreground">
                      {e.date} · {e.category.replace(/_/g, " ")}
                    </div>
                    <div className="text-xs text-foreground">{e.description}</div>
                  </li>
                ))}
              </ol>
            </Panel>
          </div>
        </div>
      </div>
    </div>
  )
}

function Stat({
  icon: Icon,
  label,
  value,
  accent = "text-foreground",
}: {
  icon: typeof Folder
  label: string
  value: number | string
  accent?: string
}) {
  return (
    <div className="bg-card px-4 py-3">
      <div className="flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
        <Icon className="h-3 w-3" />
        {label}
      </div>
      <div className={cn("mt-0.5 font-mono text-lg font-semibold tabular-nums", accent)}>{value}</div>
    </div>
  )
}

function Panel({
  title,
  icon: Icon,
  children,
  action,
  tone,
}: {
  title: string
  icon: typeof Folder
  children: React.ReactNode
  action?: { href: string; label: string }
  tone?: "destructive" | "warning" | "default"
}) {
  return (
    <div className="rounded border border-border bg-card">
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <div className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-widest">
          <Icon
            className={cn(
              "h-3 w-3",
              tone === "destructive" ? "text-destructive" : "text-muted-foreground",
            )}
          />
          <span className={tone === "destructive" ? "text-destructive" : "text-muted-foreground"}>
            {title}
          </span>
        </div>
        {action && (
          <Link
            href={action.href}
            className="flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:text-primary"
          >
            {action.label}
            <ArrowRight className="h-3 w-3" />
          </Link>
        )}
      </div>
      <div className="p-3">{children}</div>
    </div>
  )
}
