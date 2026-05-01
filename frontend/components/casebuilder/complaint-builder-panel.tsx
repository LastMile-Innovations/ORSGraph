"use client"

import Link from "next/link"
import { useState } from "react"
import { useRouter } from "next/navigation"
import { FileText, GavelIcon, ListChecks, Sparkles } from "lucide-react"
import type { Draft, Matter } from "@/lib/casebuilder/types"
import { createDraft, generateDraft } from "@/lib/casebuilder/api"
import { matterDraftHref, matterHref } from "@/lib/casebuilder/routes"

interface ComplaintBuilderPanelProps {
  matter: Matter
  complaintDraft?: Draft
}

const STEPS = [
  "Court / caption",
  "Parties",
  "Jurisdiction / venue",
  "Factual background",
  "Claims / counts",
  "Elements per count",
  "Facts supporting each element",
  "Legal authority",
  "Damages / remedies",
  "Prayer for relief",
  "Exhibits",
  "Verification / signature",
  "Filing checklist",
  "Final QC",
  "Export packet",
]

export function ComplaintBuilderPanel({ matter, complaintDraft }: ComplaintBuilderPanelProps) {
  const router = useRouter()
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function createOrGenerateComplaint() {
    setSaving(true)
    setError(null)
    let draftId = complaintDraft?.id
    if (!draftId) {
      const created = await createDraft(matter.id, {
        title: `${matter.shortName || matter.name} complaint`,
        draft_type: "complaint",
        description: "Complaint draft generated from approved facts, claims, and authority.",
      })
      if (!created.data) {
        setSaving(false)
        setError(created.error || "Complaint draft could not be created.")
        return
      }
      draftId = created.data.id
    }
    const generated = await generateDraft(matter.id, draftId)
    setSaving(false)
    if (!generated.data?.result) {
      setError(generated.error || "Complaint scaffold could not be generated.")
      return
    }
    router.push(matterDraftHref(matter.id, generated.data.result.id))
  }

  return (
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
      <header className="border-b border-border bg-card px-6 py-5">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <GavelIcon className="h-3.5 w-3.5 text-primary" />
              complaint builder
            </div>
            <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Complaint Builder</h1>
            <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
              Build a complaint draft from reviewed parties, facts, claims, authority, remedies, exhibits, and QC.
            </p>
          </div>
          <button
            type="button"
            onClick={createOrGenerateComplaint}
            disabled={saving}
            className="flex items-center gap-1.5 rounded bg-primary px-3 py-2 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
          >
            <Sparkles className="h-3.5 w-3.5" />
            {saving ? "building" : complaintDraft ? "regenerate draft" : "create draft"}
          </button>
        </div>
        {error && <p className="mt-3 rounded border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">{error}</p>}
      </header>

      <main className="grid grid-cols-1 gap-4 px-6 py-6 xl:grid-cols-[minmax(0,1fr)_360px]">
        <section className="grid grid-cols-1 gap-3 lg:grid-cols-2">
          <WorkflowTile href={matterHref(matter.id, "parties")} icon={ListChecks} title="1. Parties" body={`${matter.parties.length} parties in the matter graph`} />
          <WorkflowTile href={matterHref(matter.id, "facts")} icon={ListChecks} title="2. Facts" body={`${matter.facts.length} facts ready for review and element mapping`} />
          <WorkflowTile href={matterHref(matter.id, "claims")} icon={GavelIcon} title="3. Claims and authority" body={`${matter.claims.length} claim or defense theories linked to ORS authority`} />
          <WorkflowTile href={complaintDraft ? matterDraftHref(matter.id, complaintDraft.id) : matterDraftHref(matter.id)} icon={FileText} title="4. Draft" body={complaintDraft ? complaintDraft.title : "Create a complaint draft from graph-backed facts"} />
        </section>

        <aside className="rounded border border-border bg-card p-4">
          <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            workflow checklist
          </h2>
          <ol className="mt-3 space-y-2">
            {STEPS.map((step, index) => (
              <li key={step} className="flex items-center gap-2 text-xs">
                <span className="flex h-5 w-5 items-center justify-center rounded bg-muted font-mono text-[10px] text-muted-foreground">
                  {index + 1}
                </span>
                <span className="text-foreground">{step}</span>
              </li>
            ))}
          </ol>
          <p className="mt-4 text-xs leading-relaxed text-muted-foreground">
            The generated draft is a review scaffold, not legal advice or a filing-ready court document.
          </p>
        </aside>
      </main>
    </div>
  )
}

function WorkflowTile({
  href,
  icon: Icon,
  title,
  body,
}: {
  href: string
  icon: typeof FileText
  title: string
  body: string
}) {
  return (
    <Link href={href} className="rounded border border-border bg-card p-4 transition-colors hover:border-primary/40">
      <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
        <Icon className="h-4 w-4 text-primary" />
        {title}
      </div>
      <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{body}</p>
    </Link>
  )
}
