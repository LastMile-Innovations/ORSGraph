import Link from "next/link"
import { DataStateBanner } from "@/components/casebuilder/data-state-banner"
import { MatterListCard } from "@/components/casebuilder/matter-list-card"
import { PageHeader } from "@/components/orsg/page-header"
import { getMatterSummariesState } from "@/lib/casebuilder/server-api"
import { newMatterHref } from "@/lib/casebuilder/routes"
import {
  ArrowRight,
  Briefcase,
  FileText,
  Folder,
  GavelIcon,
  Plus,
  Sparkles,
  Upload,
} from "lucide-react"

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
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
      <DataStateBanner source={matterState.source} error={matterState.error} />
      <PageHeader
        icon={Briefcase}
        eyebrow="CaseBuilder / matters"
        title="Your matters, structured."
        description="Import files, preserve folder context, extract facts, build claims, track deadlines, and draft work product against the ORSGraph authority layer."
        actions={
          <>
            <Link
              href={newMatterHref()}
              className="inline-flex min-h-10 items-center gap-1.5 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary/90"
            >
              <Plus className="h-4 w-4" />
              New matter
            </Link>
            <Link
              href={newMatterHref("fight")}
              className="inline-flex min-h-10 items-center gap-1.5 rounded-md border border-border bg-background px-4 text-sm font-medium hover:border-primary/50 hover:text-primary"
            >
              <Upload className="h-4 w-4" />
              Fight a complaint
            </Link>
            <Link
              href={newMatterHref("build")}
              className="inline-flex min-h-10 items-center gap-1.5 rounded-md border border-border bg-background px-4 text-sm font-medium hover:border-primary/50 hover:text-primary"
            >
              <GavelIcon className="h-4 w-4" />
              Build a complaint
            </Link>
          </>
        }
        stats={[
          { label: "matters", value: matters.length },
          { label: "documents", value: totals.documents },
          { label: "facts extracted", value: totals.facts },
          { label: "open drafts", value: totals.drafts, tone: "primary" },
          { label: "open tasks", value: totals.tasks, tone: "warning" },
        ]}
      />

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
                <MatterListCard key={m.matter_id} matter={m} />
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

        {/* Core workflows */}
        <section className="border-t border-border bg-card px-6 py-8">
          <div className="mx-auto max-w-6xl">
            <h2 className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              core workflows
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
