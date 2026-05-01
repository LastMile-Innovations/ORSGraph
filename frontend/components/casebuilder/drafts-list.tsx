"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { useState } from "react"
import { Plus, Sparkles, FileText, Search, Clock, CheckCircle2 } from "lucide-react"
import type { Matter } from "@/lib/casebuilder/types"
import { matterComplaintHref, matterDraftHref } from "@/lib/casebuilder/routes"
import { createDraft, generateDraft } from "@/lib/casebuilder/api"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { cn } from "@/lib/utils"

interface DraftsListProps {
  matter: Matter
}

const DRAFT_TEMPLATES = [
  {
    id: "tpl-motion-summary",
    label: "Motion for Summary Judgment",
    draftType: "motion",
    description: "MSJ with statement of undisputed facts and legal argument.",
  },
  {
    id: "tpl-discovery",
    label: "Discovery Requests",
    draftType: "legal_memo",
    description: "Interrogatories, RFPs, and RFAs tailored to your claims.",
  },
  {
    id: "tpl-demand",
    label: "Demand Letter",
    draftType: "demand_letter",
    description: "Pre-litigation demand citing supporting facts and damages.",
  },
  {
    id: "tpl-deposition",
    label: "Deposition Outline",
    draftType: "legal_memo",
    description: "Topic-by-topic deposition outline with exhibits and goals.",
  },
  {
    id: "tpl-brief",
    label: "Trial Brief",
    draftType: "legal_memo",
    description: "Pre-trial brief with statement of case, evidentiary issues, jury instructions.",
  },
]

export function DraftsList({ matter }: DraftsListProps) {
  const router = useRouter()
  const [query, setQuery] = useState("")
  const [showCreate, setShowCreate] = useState(false)
  const [title, setTitle] = useState("")
  const [draftType, setDraftType] = useState("motion")
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const filtered = matter.drafts.filter(
    (d) => d.kind !== "complaint" && (!query || d.title.toLowerCase().includes(query.toLowerCase())),
  )

  async function createAndOpen(input: { title: string; draft_type: string; description?: string }, shouldGenerate = false) {
    if (!input.title.trim()) {
      setError("Add a draft title.")
      return
    }
    setSaving(true)
    setError(null)
    const created = await createDraft(matter.id, input)
    if (!created.data) {
      setSaving(false)
      setError(created.error || "Draft could not be created.")
      return
    }
    let draftId = created.data.id
    if (shouldGenerate) {
      const generated = await generateDraft(matter.id, created.data.id)
      if (generated.data?.result) draftId = generated.data.result.id
    }
    setSaving(false)
    router.push(matterDraftHref(matter.id, draftId))
  }

  return (
    <div className="flex flex-col">
      <div className="border-b border-border bg-background px-6 py-4">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <h1 className="text-xl font-semibold tracking-tight text-foreground">
              Drafting Studio
            </h1>
            <p className="mt-1 text-sm text-muted-foreground">
              AI-assisted drafting with citation grounding. Every claim, fact, and authority is
              one click away.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button asChild variant="outline" size="sm" className="gap-1.5 bg-transparent">
              <Link href={matterComplaintHref(matter.id, "editor")}>
                <Sparkles className="h-3.5 w-3.5" />
                Complaint editor
              </Link>
            </Button>
            <Button size="sm" className="gap-1.5" onClick={() => setShowCreate((value) => !value)}>
              <Plus className="h-3.5 w-3.5" />
              New draft
            </Button>
          </div>
        </div>

        <div className="relative mt-4 max-w-sm">
          <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search drafts..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="h-8 pl-8 text-xs"
          />
        </div>

        {showCreate && (
          <div className="mt-4 grid gap-2 rounded border border-border bg-card p-3 md:grid-cols-[minmax(0,1fr)_180px_auto]">
            <input
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder="Draft title"
              className="rounded border border-border bg-background px-3 py-2 text-xs focus:border-primary focus:outline-none"
            />
            <select
              value={draftType}
              onChange={(event) => setDraftType(event.target.value)}
              className="rounded border border-border bg-background px-3 py-2 font-mono text-xs"
            >
              {["answer", "motion", "declaration", "demand_letter", "legal_memo", "exhibit_list"].map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
            <Button
              size="sm"
              disabled={saving}
              onClick={() => createAndOpen({ title, draft_type: draftType, description: "User-created draft." })}
            >
              {saving ? "Saving" : "Create"}
            </Button>
            {error && <p className="text-xs text-destructive md:col-span-3">{error}</p>}
          </div>
        )}
      </div>

      <ScrollArea className="h-[calc(100vh-220px)]">
        <div className="px-6 py-6">
          {/* Existing drafts */}
          {filtered.length > 0 && (
            <section>
              <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Your drafts ({filtered.length})
              </h2>
              <ul className="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-3">
                {filtered.map((draft) => (
                  <li key={draft.id}>
                    <Link
                      href={matterDraftHref(matter.id, draft.id)}
                      className="group block"
                    >
                      <Card className="h-full p-4 transition-colors group-hover:border-foreground/30 group-hover:bg-muted/30">
                        <div className="flex items-start justify-between gap-2">
                          <FileText className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
                          <DraftStatus status={draft.status} />
                        </div>
                        <h3 className="mt-3 text-sm font-semibold leading-tight text-foreground text-pretty">
                          {draft.title}
                        </h3>
                        <p className="mt-1 line-clamp-2 text-xs leading-relaxed text-muted-foreground">
                          {draft.description}
                        </p>
                        <div className="mt-3 flex items-center justify-between text-[10px] text-muted-foreground">
                          <span className="flex items-center gap-1 font-mono">
                            <Clock className="h-2.5 w-2.5" />
                            {draft.lastEdited}
                          </span>
                          <span className="font-mono">
                            {draft.sections.length} sections · {draft.wordCount.toLocaleString()} words
                          </span>
                        </div>
                      </Card>
                    </Link>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {/* Templates */}
          <section className={cn(filtered.length > 0 && "mt-10")}>
            <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Start from a template
            </h2>
            <ul className="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-3">
              {DRAFT_TEMPLATES.map((tpl) => (
                <li key={tpl.id}>
                  <button
                    className="group w-full text-left"
                    disabled={saving}
                    onClick={() =>
                      createAndOpen(
                        {
                          title: tpl.label,
                          draft_type: tpl.draftType,
                          description: tpl.description,
                        },
                        false,
                      )
                    }
                  >
                    <Card className="h-full border-dashed bg-transparent p-4 transition-colors group-hover:border-foreground/30 group-hover:bg-muted/30">
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex h-8 w-8 items-center justify-center rounded-md bg-muted">
                          <FileText className="h-4 w-4 text-muted-foreground" />
                        </div>
                        <Badge variant="outline" className="gap-1 text-[10px]">
                          <Sparkles className="h-2.5 w-2.5" />
                          AI
                        </Badge>
                      </div>
                      <h3 className="mt-3 text-sm font-semibold text-foreground">{tpl.label}</h3>
                      <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                        {tpl.description}
                      </p>
                    </Card>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        </div>
      </ScrollArea>
    </div>
  )
}

function DraftStatus({ status }: { status: string }) {
  const variants: Record<string, { label: string; className: string; icon: typeof CheckCircle2 }> = {
    draft: {
      label: "Draft",
      className: "bg-muted text-muted-foreground",
      icon: FileText,
    },
    review: {
      label: "In review",
      className: "bg-amber-500/15 text-amber-700 dark:text-amber-300",
      icon: Clock,
    },
    final: {
      label: "Final",
      className: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
      icon: CheckCircle2,
    },
    filed: {
      label: "Filed",
      className: "bg-blue-500/15 text-blue-700 dark:text-blue-300",
      icon: CheckCircle2,
    },
  }
  const v = variants[status] ?? variants.draft
  const Icon = v.icon
  return (
    <span className={cn("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium", v.className)}>
      <Icon className="h-2.5 w-2.5" />
      {v.label}
    </span>
  )
}
