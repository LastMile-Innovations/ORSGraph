"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { useState } from "react"
import { AlertCircle, ArrowRight, Briefcase, FileText, GavelIcon, Loader2, Sparkles, Upload } from "lucide-react"
import { cn } from "@/lib/utils"
import type { MatterType } from "@/lib/casebuilder/types"
import { createMatter, uploadBinaryFile, uploadTextFile } from "@/lib/casebuilder/api"
import { matterHref } from "@/lib/casebuilder/routes"

type Intent = "fight" | "build" | "blank"

const INTENTS: { id: Intent; label: string; icon: typeof Briefcase; description: string }[] = [
  {
    id: "fight",
    label: "Fight a complaint",
    icon: GavelIcon,
    description:
      "You've been served. Drop the complaint and any evidence — CaseBuilder will extract claims, build an admit/deny grid, and draft an answer + counterclaims.",
  },
  {
    id: "build",
    label: "Build a complaint",
    icon: FileText,
    description:
      "Tell us what happened and upload your evidence. CaseBuilder will identify possible claims, map elements to facts, find legal authority, and draft a complaint.",
  },
  {
    id: "blank",
    label: "Blank matter",
    icon: Briefcase,
    description: "Start with an empty matter and add files as you go.",
  },
]

const TYPES: { id: MatterType; label: string }[] = [
  { id: "civil", label: "Civil" },
  { id: "landlord_tenant", label: "Landlord / Tenant" },
  { id: "employment", label: "Employment" },
  { id: "small_claims", label: "Small Claims" },
  { id: "family", label: "Family" },
  { id: "admin", label: "Administrative" },
  { id: "criminal", label: "Criminal" },
  { id: "appeal", label: "Appeal" },
  { id: "other", label: "Other" },
]

export function NewMatterClient({ initialIntent }: { initialIntent: Intent }) {
  const router = useRouter()
  const [intent, setIntent] = useState<Intent>(initialIntent)
  const [name, setName] = useState("")
  const [type, setType] = useState<MatterType>("civil")
  const [court, setCourt] = useState("")
  const [story, setStory] = useState("")
  const [files, setFiles] = useState<File[]>([])
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  function onSelectFiles(list: FileList | null) {
    if (!list) return
    setFiles([...files, ...Array.from(list)])
  }

  async function onCreateMatter() {
    const trimmedName = name.trim()
    if (!trimmedName) {
      setError("Add a matter name before creating the workspace.")
      return
    }

    setSubmitting(true)
    setError(null)

    const result = await createMatter({
      name: trimmedName,
      matter_type: type,
      user_role: intent === "fight" ? "defendant" : intent === "build" ? "plaintiff" : "neutral",
      jurisdiction: "Oregon",
      court: court.trim() || undefined,
    })

    if (!result.data) {
      setSubmitting(false)
      setError(result.error || "CaseBuilder API did not create the matter.")
      return
    }

    const matterId = result.data.id || result.data.matter_id
    const uploads = []

    if (story.trim()) {
      uploads.push(
        uploadTextFile(matterId, {
          filename: "case-narrative.txt",
          mime_type: "text/plain",
          document_type: "evidence",
          folder: "Intake",
          confidentiality: "private",
          text: story.trim(),
        }),
      )
    }

    for (const file of files) {
      uploads.push(
        uploadBinaryFile(matterId, file, {
          document_type: file.name.match(/\.csv$/i) ? "spreadsheet" : "evidence",
          folder: "Uploads",
          confidentiality: "private",
        }),
      )
    }

    await Promise.allSettled(uploads)
    router.push(matterHref(matterId))
  }

  return (
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
      <div className="border-b border-border bg-card px-6 py-8">
        <div className="mx-auto max-w-3xl">
          <div className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            CaseBuilder · new matter
          </div>
          <h1 className="font-mono text-2xl font-semibold tracking-tight text-foreground">
            Create a matter.
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Pick an intent. CaseBuilder will preconfigure the workflow, sidebar, and starter tasks.
          </p>
        </div>
      </div>

      <div className="px-6 py-6">
        <div className="mx-auto max-w-3xl space-y-6">
          {/* Intent */}
          <Section step={1} title="What are you doing?">
            <div className="grid grid-cols-1 gap-2 md:grid-cols-3">
              {INTENTS.map((i) => {
                const Icon = i.icon
                const active = intent === i.id
                return (
                  <button
                    key={i.id}
                    onClick={() => setIntent(i.id)}
                    className={cn(
                      "flex flex-col gap-2 rounded border p-4 text-left transition-colors",
                      active
                        ? "border-primary bg-primary/5"
                        : "border-border bg-background hover:border-primary/40",
                    )}
                  >
                    <Icon className={cn("h-4 w-4", active ? "text-primary" : "text-muted-foreground")} />
                    <div className={cn("text-sm font-medium", active ? "text-primary" : "text-foreground")}>
                      {i.label}
                    </div>
                    <div className="text-[11px] leading-relaxed text-muted-foreground">{i.description}</div>
                  </button>
                )
              })}
            </div>
          </Section>

          {/* Matter info */}
          <Section step={2} title="Matter details">
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              <Field label="Matter name">
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g. Smith v. ABC Property Management"
                  className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
                />
              </Field>
              <Field label="Matter type">
                <select
                  value={type}
                  onChange={(e) => setType(e.target.value as MatterType)}
                  className="w-full rounded border border-border bg-background px-3 py-2 font-mono text-xs"
                >
                  {TYPES.map((t) => (
                    <option key={t.id} value={t.id}>
                      {t.label}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Court / venue (optional)" className="md:col-span-2">
                <input
                  value={court}
                  onChange={(e) => setCourt(e.target.value)}
                  placeholder="e.g. Multnomah County Circuit Court"
                  className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
                />
              </Field>
            </div>
          </Section>

          {/* Upload */}
          <Section step={3} title="Drop your files">
            <label
              htmlFor="file-input"
              className="flex cursor-pointer flex-col items-center gap-2 rounded border-2 border-dashed border-border bg-background p-8 text-center transition-colors hover:border-primary/40"
            >
              <Upload className="h-8 w-8 text-muted-foreground" />
              <div className="text-sm font-medium text-foreground">Drag & drop or browse</div>
              <p className="max-w-md text-[11px] text-muted-foreground">
                PDFs, Word, emails, images, screenshots, audio transcripts, court filings, contracts, leases,
                medical records, police reports, agency records, spreadsheets, calendars, chat logs.
              </p>
              <input
                id="file-input"
                type="file"
                multiple
                hidden
                onChange={(e) => onSelectFiles(e.target.files)}
              />
            </label>

            {files.length > 0 && (
              <div className="mt-3 space-y-1">
                {files.map((f, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between rounded border border-border bg-background px-3 py-2 font-mono text-xs"
                  >
                    <div className="flex items-center gap-2">
                      <FileText className="h-3.5 w-3.5 text-primary" />
                      <span>{f.name}</span>
                    </div>
                    <span className="font-mono text-[10px] tabular-nums text-muted-foreground">
                      {(f.size / 1024).toFixed(1)} KB
                    </span>
                  </div>
                ))}
              </div>
            )}
          </Section>

          {/* Tell what happened (build mode) */}
          {intent === "build" && (
            <Section step={4} title="What happened? (optional)">
              <textarea
                value={story}
                onChange={(event) => setStory(event.target.value)}
                placeholder="Tell us the story in plain English. Dates, parties, what they did, what you want."
                rows={6}
                className="w-full rounded border border-border bg-background px-3 py-2 text-sm leading-relaxed focus:border-primary focus:outline-none"
              />
            </Section>
          )}

          {/* CTA */}
          <div className="rounded border border-border bg-card p-4">
            <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
              <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                <Sparkles className="h-3.5 w-3.5 text-primary" />
                Live matter creation. Text files can be extracted now; binary files are stored privately for later parsing/OCR.
              </div>
              <div className="flex items-center gap-2">
                <Link
                  href={matterHref("matter:smith-abc")}
                  title="Open the seeded CaseBuilder demo matter"
                  className="rounded border border-border px-3 py-2 font-mono text-xs uppercase tracking-wider text-muted-foreground hover:bg-muted hover:text-foreground"
                >
                  open seeded demo
                </Link>
                <button
                  type="button"
                  onClick={onCreateMatter}
                  disabled={submitting}
                  className="flex items-center gap-1.5 rounded bg-primary px-4 py-2 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {submitting ? (
                    <>
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                      creating
                    </>
                  ) : (
                    <>
                      create matter
                      <ArrowRight className="h-3.5 w-3.5" />
                    </>
                  )}
                </button>
              </div>
            </div>
            {error && (
              <div className="mt-3 flex items-start gap-2 rounded border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive">
                <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                <span>{error}</span>
              </div>
            )}
          </div>

          <p className="text-center font-mono text-[10px] text-muted-foreground">
            CaseBuilder can organize legal information, but it is not a lawyer and does not make filings court-ready.
          </p>
        </div>
      </div>
    </div>
  )
}

function Section({
  step,
  title,
  children,
}: {
  step: number
  title: string
  children: React.ReactNode
}) {
  return (
    <section className="rounded border border-border bg-card p-4">
      <div className="mb-3 flex items-center gap-2">
        <div className="flex h-5 w-5 items-center justify-center rounded-full bg-primary font-mono text-[10px] tabular-nums text-primary-foreground">
          {step}
        </div>
        <h2 className="text-sm font-medium text-foreground">{title}</h2>
      </div>
      {children}
    </section>
  )
}

function Field({
  label,
  className,
  children,
}: {
  label: string
  className?: string
  children: React.ReactNode
}) {
  return (
    <label className={cn("flex flex-col gap-1.5", className)}>
      <span className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{label}</span>
      {children}
    </label>
  )
}
