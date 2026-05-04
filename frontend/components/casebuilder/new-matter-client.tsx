"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { useRef, useState } from "react"
import {
  AlertCircle,
  ArrowRight,
  Briefcase,
  CheckCircle2,
  FileText,
  FolderUp,
  GavelIcon,
  Loader2,
  RefreshCcw,
  Sparkles,
  Upload,
} from "lucide-react"
import { cn } from "@/lib/utils"
import type { Matter, MatterType } from "@/lib/casebuilder/types"
import { createMatter, runMatterIndex, uploadBinaryFile, uploadTextFile } from "@/lib/casebuilder/api"
import { matterHref } from "@/lib/casebuilder/routes"
import {
  createUploadBatchId,
  dataTransferToUploadCandidates,
  filesToUploadCandidates,
  type UploadCandidate,
} from "@/lib/casebuilder/upload-folders"
import { trackConversionEvent } from "@/lib/conversion-events"

type Intent = "fight" | "build" | "blank"
type IntakeRowStatus = "queued" | "uploading" | "stored" | "indexing" | "indexed" | "skipped" | "failed"
type IntakeRowKind = "story" | "file"

interface IntakeUploadRow {
  id: string
  kind: IntakeRowKind
  label: string
  relativePath: string
  status: IntakeRowStatus
  message: string
  file?: File
  storyText?: string
  documentId?: string
}

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
  const fileInputRef = useRef<HTMLInputElement>(null)
  const folderInputRef = useRef<HTMLInputElement>(null)
  const [intent, setIntent] = useState<Intent>(initialIntent)
  const [name, setName] = useState("")
  const [type, setType] = useState<MatterType>("civil")
  const [court, setCourt] = useState("")
  const [story, setStory] = useState("")
  const [files, setFiles] = useState<UploadCandidate[]>([])
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createdMatter, setCreatedMatter] = useState<Matter | null>(null)
  const [uploadRows, setUploadRows] = useState<IntakeUploadRow[]>([])
  const [uploadMessage, setUploadMessage] = useState<string | null>(null)
  const [dragActive, setDragActive] = useState(false)

  function onSelectFiles(list: FileList | null) {
    if (!list) return
    setFiles((current) => [...current, ...filesToUploadCandidates(list)])
  }

  async function onDropFiles(event: React.DragEvent<HTMLElement>) {
    event.preventDefault()
    setDragActive(false)
    try {
      const candidates = await dataTransferToUploadCandidates(event.dataTransfer)
      if (candidates.length > 0) {
        setFiles((current) => [...current, ...candidates])
      }
    } catch (dropError) {
      setError(`Could not read dropped files: ${formatUnknownError(dropError)}`)
    }
  }

  async function onCreateMatter() {
    const trimmedName = name.trim()
    if (!trimmedName) {
      setError("Add a matter name before creating the workspace.")
      return
    }

    setSubmitting(true)
    setError(null)
    setUploadMessage(null)

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

    setCreatedMatter(result.data)
    trackConversionEvent("first_matter_created", {
      intent,
      matter_type: type,
      uploaded_files: files.length,
      has_story: Boolean(story.trim()),
    })

    const matterId = result.data.id || result.data.matter_id
    const uploadBatchId = createUploadBatchId(files.some((item) => item.relativePath.includes("/")) ? "folder" : "batch")
    const rows = buildUploadRows(uploadBatchId, story, files)
    setUploadRows(rows)

    if (rows.length === 0) {
      setSubmitting(false)
      router.push(matterHref(matterId))
      return
    }

    const resultSummary = await processUploadRows(matterId, rows)
    setSubmitting(false)

    if (resultSummary.failed > 0 || resultSummary.skipped > 0) {
      setError(
        `${resultSummary.failed + resultSummary.skipped} intake item${
          resultSummary.failed + resultSummary.skipped === 1 ? "" : "s"
        } need attention. The matter was created and you can retry failed rows or continue to the workspace.`,
      )
      setUploadMessage(
        `${resultSummary.stored} stored, ${resultSummary.indexed} indexed${
          resultSummary.skipped ? `, ${resultSummary.skipped} skipped` : ""
        }${resultSummary.failed ? `, ${resultSummary.failed} failed` : ""}.`,
      )
      return
    }

    router.push(matterHref(matterId))
  }

  async function retryRow(row: IntakeUploadRow) {
    const matterId = createdMatter?.id || createdMatter?.matter_id
    if (!matterId) {
      setError("Create the matter before retrying uploads.")
      return
    }

    setSubmitting(true)
    setError(null)
    setUploadMessage(null)
    const resultSummary = row.documentId
      ? {
          stored: 0,
          ...(await indexUploadedDocuments(matterId, [row.documentId])),
        }
      : await processUploadRows(matterId, [resetUploadRow(row)])
    setSubmitting(false)

    if (resultSummary.failed > 0 || resultSummary.skipped > 0) {
      setError(`${row.relativePath}: retry did not complete. Check the row message or continue to the workspace.`)
      return
    }
    setUploadMessage("Retry completed.")
  }

  async function processUploadRows(matterId: string, rows: IntakeUploadRow[]) {
    const markdownDocumentIds: string[] = []
    let stored = 0
    let indexed = 0
    let skipped = 0
    let failed = 0

    for (const row of rows) {
      updateUploadRow(row.id, { status: "uploading", message: "Uploading privately" })

      const result = await uploadIntakeRow(matterId, row)

      if (!result.data?.document_id) {
        failed += 1
        updateUploadRow(row.id, {
          status: "failed",
          message: result.error || "Upload failed.",
        })
        continue
      }

      stored += 1
      if (rowIsMarkdownIndexable(row)) {
        markdownDocumentIds.push(result.data.document_id)
      }
      updateUploadRow(row.id, {
        status: "stored",
        documentId: result.data.document_id,
        message: rowIsMarkdownIndexable(row) ? "Stored privately" : "Stored privately; Markdown-only indexing skipped",
      })
    }

    if (markdownDocumentIds.length > 0) {
      const indexSummary = await indexUploadedDocuments(matterId, markdownDocumentIds)
      indexed += indexSummary.indexed
      skipped += indexSummary.skipped
      failed += indexSummary.failed
    }

    return { stored, indexed, skipped, failed }
  }

  async function indexUploadedDocuments(matterId: string, documentIds: string[]) {
    setUploadRows((current) =>
      current.map((row) =>
        row.documentId && documentIds.includes(row.documentId)
          ? { ...row, status: "indexing", message: "Indexing" }
          : row,
      ),
    )
    const indexResult = await runMatterIndex(matterId, { document_ids: documentIds })
    if (!indexResult.data) {
      setUploadRows((current) =>
        current.map((row) =>
          row.documentId && documentIds.includes(row.documentId)
            ? { ...row, status: "failed", message: indexResult.error || "Index run failed." }
            : row,
        ),
      )
      return { indexed: 0, skipped: 0, failed: documentIds.length }
    }

    let indexed = 0
    let skipped = 0
    let failed = 0
    const patches = new Map<string, Pick<IntakeUploadRow, "status" | "message">>()
    const byDocument = new Map(indexResult.data.results.map((item) => [item.document_id, item]))
    for (const documentId of documentIds) {
      const result = byDocument.get(documentId)
      if (!result) {
        failed += 1
        patches.set(documentId, {
          status: "failed",
          message: "Index run did not return a result for this document.",
        })
      } else if (result.status === "indexed") {
        indexed += 1
        patches.set(documentId, { status: "indexed", message: result.message || "Indexed" })
      } else if (result.status === "failed") {
        failed += 1
        patches.set(documentId, { status: "failed", message: result.message || "Indexing failed" })
      } else {
        skipped += 1
        patches.set(documentId, { status: "skipped", message: result.message || "Stored; indexing skipped" })
      }
    }

    setUploadRows((current) =>
      current.map((row) => {
        if (!row.documentId) return row
        const patch = patches.get(row.documentId)
        return patch ? { ...row, ...patch } : row
      }),
    )
    return { indexed, skipped, failed }
  }

  function updateUploadRow(id: string, patch: Partial<IntakeUploadRow>) {
    setUploadRows((current) => current.map((row) => (row.id === id ? { ...row, ...patch } : row)))
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
              onDragOver={(event) => {
                event.preventDefault()
                setDragActive(true)
              }}
              onDragLeave={() => setDragActive(false)}
              onDrop={(event) => void onDropFiles(event)}
              className={cn(
                "flex cursor-pointer flex-col items-center gap-2 rounded border-2 border-dashed bg-background p-8 text-center transition-colors hover:border-primary/40",
                dragActive ? "border-primary bg-primary/5" : "border-border",
              )}
            >
              <Upload className="h-8 w-8 text-muted-foreground" />
              <div className="text-sm font-medium text-foreground">Drag & drop or browse</div>
              <p className="max-w-md text-[11px] text-muted-foreground">
                Upload any matter file for private storage and viewing. Markdown files are indexed now; other formats stay view-only.
              </p>
              <input
                ref={fileInputRef}
                id="file-input"
                type="file"
                multiple
                hidden
                onChange={(event) => {
                  onSelectFiles(event.target.files)
                  event.currentTarget.value = ""
                }}
              />
            </label>
            <input
              ref={folderInputRef}
              type="file"
              multiple
              hidden
              {...({ webkitdirectory: "", directory: "" } as Record<string, string>)}
              onChange={(event) => {
                onSelectFiles(event.target.files)
                event.currentTarget.value = ""
              }}
            />
            <div className="mt-2 flex justify-center">
              <button
                type="button"
                onClick={() => folderInputRef.current?.click()}
                className="inline-flex items-center gap-1.5 rounded border border-border px-3 py-1.5 font-mono text-xs uppercase tracking-wider text-muted-foreground hover:bg-muted hover:text-foreground"
              >
                <FolderUp className="h-3.5 w-3.5" />
                upload folder
              </button>
            </div>

            {files.length > 0 && (
              <div className="mt-3 space-y-1">
                {files.map((candidate, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between rounded border border-border bg-background px-3 py-2 font-mono text-xs"
                  >
                    <div className="flex items-center gap-2">
                      <FileText className="h-3.5 w-3.5 text-primary" />
                      <span>{candidate.relativePath}</span>
                    </div>
                    <span className="font-mono text-[10px] tabular-nums text-muted-foreground">
                      {(candidate.file.size / 1024).toFixed(1)} KB
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
                Live matter creation. Markdown files are indexed now; other formats are stored privately for viewing.
              </div>
              <div className="flex items-center gap-2">
                <Link
                  href={matterHref("matter:smith-abc")}
                  title="Open the seeded CaseBuilder demo matter"
                  className="rounded border border-border px-3 py-2 font-mono text-xs uppercase tracking-wider text-muted-foreground hover:bg-muted hover:text-foreground"
                >
                  open seeded demo
                </Link>
                {createdMatter && (
                  <Link
                    href={matterHref(createdMatter.id || createdMatter.matter_id)}
                    className="rounded border border-primary/30 px-3 py-2 font-mono text-xs uppercase tracking-wider text-primary hover:bg-primary/10"
                  >
                    continue
                  </Link>
                )}
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
            {uploadMessage && (
              <div className="mt-3 flex items-start gap-2 rounded border border-primary/20 bg-primary/5 p-3 text-xs text-muted-foreground">
                <CheckCircle2 className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" />
                <span>{uploadMessage}</span>
              </div>
            )}
            {uploadRows.length > 0 && (
              <div className="mt-3 rounded border border-border bg-background p-3">
                <div className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                  intake batch
                </div>
                <div className="max-h-44 space-y-1 overflow-y-auto pr-1">
                  {uploadRows.map((row) => (
                    <div key={row.id} className="flex items-center gap-2 rounded border border-border px-2 py-1 text-[11px]">
                      <span className={cn("h-2 w-2 rounded-full", uploadStatusClass(row.status))} />
                      <span className="min-w-0 flex-1 truncate font-mono" title={row.relativePath}>
                        {row.relativePath}
                      </span>
                      <span className="shrink-0 text-muted-foreground">{row.status}</span>
                      {row.status === "failed" && (
                        <button
                          type="button"
                          onClick={() => void retryRow(row)}
                          disabled={submitting}
                          className="inline-flex shrink-0 items-center gap-1 rounded border border-border px-1.5 py-0.5 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted disabled:opacity-50"
                        >
                          <RefreshCcw className="h-3 w-3" />
                          retry
                        </button>
                      )}
                      <span className="hidden min-w-0 max-w-48 truncate text-muted-foreground md:inline">{row.message}</span>
                    </div>
                  ))}
                </div>
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

async function uploadIntakeRow(matterId: string, row: IntakeUploadRow) {
  try {
    if (row.kind === "story") {
      return await uploadTextFile(matterId, {
        filename: "case-narrative.md",
        mime_type: "text/markdown",
        document_type: "evidence",
        folder: "Intake",
        confidentiality: "private",
        relative_path: "Intake/case-narrative.md",
        upload_batch_id: batchIdFromRow(row.id),
        text: row.storyText ?? "",
      })
    }
    if (!row.file) {
      return { data: null, error: "File is no longer available for retry." }
    }
    return await uploadBinaryFile(matterId, row.file, {
      document_type: row.file.name.match(/\.csv$/i) ? "spreadsheet" : "evidence",
      confidentiality: "private",
      relative_path: row.relativePath,
      upload_batch_id: batchIdFromRow(row.id),
    })
  } catch (uploadError) {
    return { data: null, error: formatUnknownError(uploadError) }
  }
}

function buildUploadRows(uploadBatchId: string, story: string, files: UploadCandidate[]): IntakeUploadRow[] {
  const rows: IntakeUploadRow[] = []
  if (story.trim()) {
    rows.push({
      id: `${uploadBatchId}:story`,
      kind: "story",
      label: "Case narrative",
      relativePath: "Intake/case-narrative.md",
      status: "queued",
      message: "Ready",
      storyText: story.trim(),
    })
  }
  files.forEach((candidate, index) => {
    rows.push({
      id: `${uploadBatchId}:file:${index}`,
      kind: "file",
      label: candidate.file.name,
      relativePath: candidate.relativePath,
      status: "queued",
      message: "Ready",
      file: candidate.file,
    })
  })
  return rows
}

function rowIsMarkdownIndexable(row: IntakeUploadRow) {
  if (row.kind === "story") return true
  return Boolean(row.file && isMarkdownIndexableFile(row.file.name, row.file.type))
}

function isMarkdownIndexableFile(filename: string, mimeType?: string | null) {
  return /\.(md|markdown)$/i.test(filename) || mimeType?.toLowerCase() === "text/markdown"
}

function resetUploadRow(row: IntakeUploadRow): IntakeUploadRow {
  return {
    ...row,
    status: "queued",
    message: "Ready",
    documentId: undefined,
  }
}

function batchIdFromRow(rowId: string): string {
  const parts = rowId.split(":")
  return parts.length >= 3 ? parts.slice(0, 3).join(":") : rowId
}

function uploadStatusClass(status: IntakeRowStatus) {
  switch (status) {
    case "indexed":
      return "bg-success"
    case "stored":
    case "skipped":
      return "bg-warning"
    case "failed":
      return "bg-destructive"
    case "uploading":
    case "indexing":
      return "bg-primary animate-pulse"
    default:
      return "bg-muted-foreground/40"
  }
}

function formatUnknownError(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
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
