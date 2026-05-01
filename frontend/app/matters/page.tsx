import Link from "next/link"
import { Shell } from "@/components/orsg/shell"
import { DataStateBanner } from "@/components/casebuilder/data-state-banner"
import { getMatterSummariesState } from "@/lib/casebuilder/api"
import { matterHref, newMatterHref } from "@/lib/casebuilder/routes"
import {
  AlertTriangle,
  ArrowRight,
  Briefcase,
  CalendarClock,
  FileText,
  Folder,
  GavelIcon,
  Plus,
  Scale,
  Sparkles,
  Upload,
} from "lucide-react"
import { cn } from "@/lib/utils"
import type { MatterStatus } from "@/lib/casebuilder/types"

const STATUS_CLS: Record<MatterStatus, string> = {
  active: "bg-success/15 text-success",
  intake: "bg-primary/15 text-primary",
  stayed: "bg-warning/15 text-warning",
  closed: "bg-muted text-muted-foreground",
  appeal: "bg-accent/20 text-accent",
}

export default async function MattersPage() {
  const matterState = await getMatterSummariesState()
  const matters = matterState.data
  const totals = matters.reduce(
    (acc, m) => ({
      documents: acc.documents + m.document_count,
      facts: acc.facts + m.fact_count,
      drafts: acc.drafts + m.draft_count,
      tasks: acc.tasks + m.open_task_count,
    }),
    { documents: 0, facts: 0, drafts: 0, tasks: 0 },
  )

  return (
    <Shell hideLeftRail>
      <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
        <DataStateBanner source={matterState.source} error={matterState.error} />
        {/* Hero */}
        <section className="border-b border-border bg-card px-6 py-10">
          <div className="mx-auto max-w-6xl">
            <div className="mb-2 flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <Briefcase className="h-3 w-3 text-primary" />
              CaseBuilder
              <span className="text-border">/</span>
              <span>cursor for law</span>
            </div>
            <h1 className="text-balance font-mono text-3xl font-semibold tracking-tight text-foreground">
              Your matters, structured.
            </h1>
            <p className="mt-2 max-w-2xl text-pretty text-sm text-muted-foreground">
              Import every file. CaseBuilder extracts facts, builds a case graph, links evidence to claims,
              fact-checks every paragraph, and turns what you have into legal work product — anchored to the
              ORSGraph authority layer.
            </p>

            <div className="mt-6 flex flex-wrap gap-2">
              <Link
                href={newMatterHref()}
                className="flex items-center gap-1.5 rounded bg-primary px-3 py-2 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90"
              >
                <Plus className="h-3.5 w-3.5" />
                new matter
              </Link>
              <Link
                href={newMatterHref("fight")}
                className="flex items-center gap-1.5 rounded border border-border bg-background px-3 py-2 font-mono text-xs uppercase tracking-wider hover:border-primary hover:text-primary"
              >
                <Upload className="h-3.5 w-3.5" />
                fight a complaint
              </Link>
              <Link
                href={newMatterHref("build")}
                className="flex items-center gap-1.5 rounded border border-border bg-background px-3 py-2 font-mono text-xs uppercase tracking-wider hover:border-primary hover:text-primary"
              >
                <GavelIcon className="h-3.5 w-3.5" />
                build a complaint
              </Link>
              <Link
                href="/fact-check"
                className="flex items-center gap-1.5 rounded border border-border bg-background px-3 py-2 font-mono text-xs uppercase tracking-wider hover:border-primary hover:text-primary"
              >
                <Sparkles className="h-3.5 w-3.5" />
                fact-check a draft
              </Link>
            </div>

            <div className="mt-6 grid grid-cols-2 gap-px overflow-hidden rounded border border-border bg-border md:grid-cols-5">
              <Stat label="matters" value={matters.length} />
              <Stat label="documents" value={totals.documents} />
              <Stat label="facts extracted" value={totals.facts} />
              <Stat label="open drafts" value={totals.drafts} accent="text-primary" />
              <Stat label="open tasks" value={totals.tasks} accent="text-warning" />
            </div>
          </div>
        </section>

        {/* Matters list */}
        <section className="px-6 py-8">
          <div className="mx-auto max-w-6xl">
            <h2 className="mb-3 flex items-baseline justify-between font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <span>your matters</span>
              <span className="normal-case text-muted-foreground">
                sorted by recent activity
              </span>
            </h2>
            <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
              {matters.map((m) => (
                <Link
                  key={m.matter_id}
                  href={matterHref(m.matter_id)}
                  className="group flex flex-col gap-3 rounded border border-border bg-card p-4 transition-colors hover:border-primary/40"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                        <span>{m.matter_type.replace(/_/g, " ")}</span>
                        <span className="text-border">/</span>
                        <span>{m.case_number ?? "no case #"}</span>
                      </div>
                      <h3 className="mt-0.5 line-clamp-1 text-base font-semibold text-foreground group-hover:text-primary">
                        {m.name}
                      </h3>
                      <div className="mt-0.5 line-clamp-1 font-mono text-[11px] text-muted-foreground">
                        {m.court}
                      </div>
                    </div>
                    <span className={cn("rounded px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider", STATUS_CLS[m.status])}>
                      {m.status}
                    </span>
                  </div>

                  <div className="grid grid-cols-4 gap-2 border-t border-border pt-3">
                    <Mini icon={Folder} label="docs" value={m.document_count} />
                    <Mini icon={Scale} label="claims" value={m.claim_count} />
                    <Mini icon={FileText} label="drafts" value={m.draft_count} />
                    <Mini icon={CalendarClock} label="tasks" value={m.open_task_count} />
                  </div>

                  {m.next_deadline ? (
                    <div className="flex items-center justify-between rounded border border-border bg-background px-3 py-2">
                      <div className="flex items-center gap-2">
                        <AlertTriangle
                          className={cn(
                            "h-3.5 w-3.5",
                            m.next_deadline.days_remaining <= 7
                              ? "text-destructive"
                              : m.next_deadline.days_remaining <= 21
                                ? "text-warning"
                                : "text-muted-foreground",
                          )}
                        />
                        <span className="text-xs text-foreground">{m.next_deadline.description}</span>
                      </div>
                      <div className="flex items-center gap-2 font-mono text-[10px] tabular-nums">
                        <span className="text-muted-foreground">{m.next_deadline.due_date}</span>
                        <span
                          className={cn(
                            "rounded px-1.5 py-0.5",
                            m.next_deadline.days_remaining <= 7
                              ? "bg-destructive/15 text-destructive"
                              : m.next_deadline.days_remaining <= 21
                                ? "bg-warning/15 text-warning"
                                : "bg-success/15 text-success",
                          )}
                        >
                          {m.next_deadline.days_remaining}d
                        </span>
                      </div>
                    </div>
                  ) : (
                    <div className="flex items-center justify-between rounded border border-dashed border-border px-3 py-2">
                      <span className="text-xs text-muted-foreground">No critical deadline yet</span>
                      <span className="font-mono text-[10px] uppercase text-muted-foreground">intake</span>
                    </div>
                  )}

                  <div className="flex items-center justify-between font-mono text-[10px] uppercase tracking-wide text-muted-foreground group-hover:text-primary">
                    <span>updated {new Date(m.updated_at).toLocaleDateString()}</span>
                    <span className="flex items-center gap-1">
                      open
                      <ArrowRight className="h-3 w-3" />
                    </span>
                  </div>
                </Link>
              ))}

              {/* New matter card */}
              <Link
                href={newMatterHref()}
                className="group flex flex-col items-center justify-center gap-2 rounded border-2 border-dashed border-border bg-background p-6 text-center hover:border-primary/40 hover:text-primary"
              >
                <Plus className="h-6 w-6 text-muted-foreground group-hover:text-primary" />
                <div className="font-mono text-[11px] uppercase tracking-widest text-muted-foreground group-hover:text-primary">
                  new matter
                </div>
                <p className="max-w-xs text-xs text-muted-foreground">
                  Drop a complaint, contract, or any case file. CaseBuilder will create the matter and start
                  extracting parties, facts, and deadlines.
                </p>
              </Link>
            </div>
          </div>
        </section>

        {/* Killer workflows */}
        <section className="border-t border-border bg-card px-6 py-8">
          <div className="mx-auto max-w-6xl">
            <h2 className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              killer workflows
            </h2>
            <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
              <Workflow
                title="Fight this complaint"
                steps={["Upload complaint", "Map allegations", "Build admit/deny grid", "Draft answer + counterclaims", "Fact-check"]}
                href="/complaint"
                icon={GavelIcon}
              />
              <Workflow
                title="Build my complaint"
                steps={["Tell what happened", "Upload evidence", "Map elements to facts", "Find authority", "Draft + fact-check"]}
                href={newMatterHref("build")}
                icon={FileText}
              />
              <Workflow
                title="Fact-check my draft"
                steps={["Paste or upload draft", "Resolve citations", "Check support against evidence", "Flag fixes"]}
                href="/fact-check"
                icon={Sparkles}
              />
            </div>
          </div>
        </section>
      </div>
    </Shell>
  )
}

function Stat({
  label,
  value,
  accent = "text-foreground",
}: {
  label: string
  value: number | string
  accent?: string
}) {
  const display = typeof value === "number" ? value.toLocaleString() : value
  return (
    <div className="bg-card px-4 py-3">
      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div className={cn("mt-0.5 font-mono text-lg font-semibold tabular-nums", accent)}>{display}</div>
    </div>
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

function Workflow({
  title,
  steps,
  href,
  icon: Icon,
}: {
  title: string
  steps: string[]
  href: string
  icon: typeof Folder
}) {
  return (
    <Link
      href={href}
      className="group flex flex-col gap-2 rounded border border-border bg-background p-4 hover:border-primary/40"
    >
      <div className="flex items-center justify-between">
        <div className="flex h-8 w-8 items-center justify-center rounded bg-muted text-foreground group-hover:bg-primary group-hover:text-primary-foreground">
          <Icon className="h-4 w-4" />
        </div>
        <ArrowRight className="h-3.5 w-3.5 text-muted-foreground group-hover:text-primary" />
      </div>
      <h3 className="text-sm font-semibold">{title}</h3>
      <ol className="ml-3 list-decimal space-y-0.5 text-xs text-muted-foreground">
        {steps.map((s) => (
          <li key={s}>{s}</li>
        ))}
      </ol>
    </Link>
  )
}
