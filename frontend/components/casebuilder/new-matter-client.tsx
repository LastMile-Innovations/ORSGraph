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
  Sparkles,
  Upload,
} from "lucide-react"
import { cn } from "@/lib/utils"
import type { CaseBuilderUserSettings, Matter, MatterSide, MatterType, PatchCaseBuilderMatterSettingsInput } from "@/lib/casebuilder/types"
import { createMatter } from "@/lib/casebuilder/api"
import { matterHref } from "@/lib/casebuilder/routes"
import {
  dataTransferToUploadCandidates,
  filesToUploadCandidates,
  type UploadCandidate,
} from "@/lib/casebuilder/upload-folders"
import { trackConversionEvent } from "@/lib/conversion-events"
import { useCaseBuilderUploads } from "./upload-provider"

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

const ROLES: { id: MatterSide; label: string }[] = [
  { id: "plaintiff", label: "Plaintiff" },
  { id: "defendant", label: "Defendant" },
  { id: "petitioner", label: "Petitioner" },
  { id: "respondent", label: "Respondent" },
  { id: "neutral", label: "Neutral" },
  { id: "researcher", label: "Researcher" },
]

const DOCUMENT_TYPES = ["complaint", "answer", "motion", "order", "contract", "lease", "email", "letter", "notice", "medical", "police", "agency_record", "public_record", "spreadsheet", "photo", "screenshot", "audio_transcript", "receipt", "invoice", "evidence", "exhibit", "other"] as const
const CONFIDENTIALITY = ["private", "filed", "public", "sealed"] as const
const TRANSCRIPT_PRESETS = ["unclear", "unclear_masked", "verbatim_multilingual", "legal", "medical", "financial", "technical", "code_switching", "customer_support"] as const
const EXPORT_FORMATS = ["pdf", "docx", "html", "markdown", "text", "json"] as const

const FALLBACK_SETTINGS: Pick<
  CaseBuilderUserSettings,
  | "default_matter_type"
  | "default_user_role"
  | "default_jurisdiction"
  | "default_court"
  | "default_confidentiality"
  | "default_document_type"
  | "auto_index_uploads"
  | "auto_import_complaints"
  | "preserve_folder_paths"
  | "timeline_suggestions_enabled"
  | "ai_timeline_enrichment_enabled"
  | "transcript_redact_pii"
  | "transcript_speaker_labels"
  | "transcript_default_view"
  | "transcript_prompt_preset"
  | "transcript_remove_audio_tags"
  | "export_default_format"
  | "export_include_exhibits"
  | "export_include_qc_report"
> = {
  default_matter_type: "civil",
  default_user_role: "neutral",
  default_jurisdiction: "Oregon",
  default_court: "",
  default_confidentiality: "private",
  default_document_type: "other",
  auto_index_uploads: true,
  auto_import_complaints: true,
  preserve_folder_paths: true,
  timeline_suggestions_enabled: true,
  ai_timeline_enrichment_enabled: true,
  transcript_redact_pii: true,
  transcript_speaker_labels: true,
  transcript_default_view: "redacted",
  transcript_prompt_preset: "unclear",
  transcript_remove_audio_tags: true,
  export_default_format: "pdf",
  export_include_exhibits: true,
  export_include_qc_report: true,
}

export function NewMatterClient({ initialIntent, settings }: { initialIntent: Intent; settings?: CaseBuilderUserSettings | null }) {
  const router = useRouter()
  const defaults = settings ?? FALLBACK_SETTINGS
  const { enqueueMatterIntake } = useCaseBuilderUploads()
  const fileInputRef = useRef<HTMLInputElement>(null)
  const folderInputRef = useRef<HTMLInputElement>(null)
  const [intent, setIntent] = useState<Intent>(initialIntent)
  const [name, setName] = useState("")
  const [type, setType] = useState<MatterType>(defaults.default_matter_type)
  const [userRole, setUserRole] = useState<MatterSide>(intentRole(initialIntent, defaults.default_user_role))
  const [jurisdiction, setJurisdiction] = useState(defaults.default_jurisdiction)
  const [court, setCourt] = useState(defaults.default_court === "Unassigned" ? "" : defaults.default_court)
  const [defaultConfidentiality, setDefaultConfidentiality] = useState(defaults.default_confidentiality)
  const [defaultDocumentType, setDefaultDocumentType] = useState(defaults.default_document_type)
  const [autoIndexUploads, setAutoIndexUploads] = useState(defaults.auto_index_uploads)
  const [autoImportComplaints, setAutoImportComplaints] = useState(defaults.auto_import_complaints)
  const [preserveFolderPaths, setPreserveFolderPaths] = useState(defaults.preserve_folder_paths)
  const [timelineSuggestionsEnabled, setTimelineSuggestionsEnabled] = useState(defaults.timeline_suggestions_enabled)
  const [aiTimelineEnrichmentEnabled, setAiTimelineEnrichmentEnabled] = useState(defaults.ai_timeline_enrichment_enabled)
  const [transcriptRedactPii, setTranscriptRedactPii] = useState(defaults.transcript_redact_pii)
  const [transcriptSpeakerLabels, setTranscriptSpeakerLabels] = useState(defaults.transcript_speaker_labels)
  const [transcriptDefaultView, setTranscriptDefaultView] = useState(defaults.transcript_default_view)
  const [transcriptPromptPreset, setTranscriptPromptPreset] = useState(defaults.transcript_prompt_preset)
  const [transcriptRemoveAudioTags, setTranscriptRemoveAudioTags] = useState(defaults.transcript_remove_audio_tags)
  const [exportDefaultFormat, setExportDefaultFormat] = useState(defaults.export_default_format)
  const [exportIncludeExhibits, setExportIncludeExhibits] = useState(defaults.export_include_exhibits)
  const [exportIncludeQcReport, setExportIncludeQcReport] = useState(defaults.export_include_qc_report)
  const [story, setStory] = useState("")
  const [files, setFiles] = useState<UploadCandidate[]>([])
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createdMatter, setCreatedMatter] = useState<Matter | null>(null)
  const [uploadMessage, setUploadMessage] = useState<string | null>(null)
  const [dragActive, setDragActive] = useState(false)
  const canCreate = name.trim().length > 0 && !submitting

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
      user_role: userRole,
      jurisdiction: jurisdiction.trim() || defaults.default_jurisdiction,
      court: court.trim() || undefined,
      settings: initialSettingsPatch(defaults, {
        default_confidentiality: defaultConfidentiality,
        default_document_type: defaultDocumentType,
        auto_index_uploads: autoIndexUploads,
        auto_import_complaints: autoImportComplaints,
        preserve_folder_paths: preserveFolderPaths,
        timeline_suggestions_enabled: timelineSuggestionsEnabled,
        ai_timeline_enrichment_enabled: aiTimelineEnrichmentEnabled,
        transcript_redact_pii: transcriptRedactPii,
        transcript_speaker_labels: transcriptSpeakerLabels,
        transcript_default_view: transcriptDefaultView,
        transcript_prompt_preset: transcriptPromptPreset,
        transcript_remove_audio_tags: transcriptRemoveAudioTags,
        export_default_format: exportDefaultFormat,
        export_include_exhibits: exportIncludeExhibits,
        export_include_qc_report: exportIncludeQcReport,
      }),
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
    const uploadBatchId = enqueueMatterIntake(matterId, files, {
      storyText: intent === "build" ? story : undefined,
      label: "Matter intake",
      autoIndex: autoIndexUploads,
      importComplaints: autoImportComplaints,
      defaultConfidentiality,
      defaultDocumentType,
    })
    setSubmitting(false)
    if (uploadBatchId) {
      setUploadMessage("Upload started. You can keep working while CaseBuilder stores and indexes the intake files.")
    }
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
                    onClick={() => {
                      setIntent(i.id)
                      setUserRole(intentRole(i.id, defaults.default_user_role))
                    }}
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
              <Field label="Matter name *">
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
              <Field label="Your role">
                <select
                  value={userRole}
                  onChange={(e) => setUserRole(e.target.value as MatterSide)}
                  className="w-full rounded border border-border bg-background px-3 py-2 font-mono text-xs"
                >
                  {ROLES.map((role) => (
                    <option key={role.id} value={role.id}>
                      {role.label}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Jurisdiction">
                <input
                  value={jurisdiction}
                  onChange={(e) => setJurisdiction(e.target.value)}
                  className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
                />
              </Field>
              <Field label="Court / venue (optional)">
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
              <div className="text-sm font-medium text-foreground">Drag files here or browse files</div>
              <p className="max-w-md text-[11px] text-muted-foreground">
                Upload individual files for private storage and viewing. Use Upload folder below when you want to preserve folder paths.
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

          <Section step={4} title="Matter config">
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              <Field label="Default confidentiality">
                <select value={defaultConfidentiality} onChange={(event) => setDefaultConfidentiality(event.target.value)} className="w-full rounded border border-border bg-background px-3 py-2 font-mono text-xs">
                  {CONFIDENTIALITY.map((value) => <option key={value} value={value}>{value}</option>)}
                </select>
              </Field>
              <Field label="Fallback document type">
                <select value={defaultDocumentType} onChange={(event) => setDefaultDocumentType(event.target.value as CaseBuilderUserSettings["default_document_type"])} className="w-full rounded border border-border bg-background px-3 py-2 font-mono text-xs">
                  {DOCUMENT_TYPES.map((value) => <option key={value} value={value}>{value.replace(/_/g, " ")}</option>)}
                </select>
              </Field>
              <Toggle label="Auto-index uploads" checked={autoIndexUploads} onChange={setAutoIndexUploads} />
              <Toggle label="Auto-import complaints" checked={autoImportComplaints} onChange={setAutoImportComplaints} />
              <Toggle label="Preserve folder paths" checked={preserveFolderPaths} onChange={setPreserveFolderPaths} />
              <Toggle label="Timeline suggestions" checked={timelineSuggestionsEnabled} onChange={setTimelineSuggestionsEnabled} />
              <Toggle label="AI timeline enrichment" checked={aiTimelineEnrichmentEnabled} onChange={setAiTimelineEnrichmentEnabled} />
              <Toggle label="Redacted transcript default" checked={transcriptRedactPii} onChange={setTranscriptRedactPii} />
              <Toggle label="Transcript speaker labels" checked={transcriptSpeakerLabels} onChange={setTranscriptSpeakerLabels} />
              <Toggle label="Remove transcript audio tags" checked={transcriptRemoveAudioTags} onChange={setTranscriptRemoveAudioTags} />
              <Field label="Transcript view">
                <select value={transcriptDefaultView} onChange={(event) => setTranscriptDefaultView(event.target.value)} className="w-full rounded border border-border bg-background px-3 py-2 font-mono text-xs">
                  <option value="redacted">redacted</option>
                  <option value="raw">raw</option>
                </select>
              </Field>
              <Field label="Transcript prompt">
                <select value={transcriptPromptPreset} onChange={(event) => setTranscriptPromptPreset(event.target.value)} className="w-full rounded border border-border bg-background px-3 py-2 font-mono text-xs">
                  {TRANSCRIPT_PRESETS.map((value) => <option key={value} value={value}>{value.replace(/_/g, " ")}</option>)}
                </select>
              </Field>
              <Field label="Export format">
                <select value={exportDefaultFormat} onChange={(event) => setExportDefaultFormat(event.target.value)} className="w-full rounded border border-border bg-background px-3 py-2 font-mono text-xs">
                  {EXPORT_FORMATS.map((value) => <option key={value} value={value}>{value}</option>)}
                </select>
              </Field>
              <Toggle label="Include exhibits in exports" checked={exportIncludeExhibits} onChange={setExportIncludeExhibits} />
              <Toggle label="Include QC report in exports" checked={exportIncludeQcReport} onChange={setExportIncludeQcReport} />
            </div>
          </Section>

          {/* Tell what happened (build mode) */}
          {intent === "build" && (
            <Section step={5} title="What happened? (optional)">
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
                  disabled={!canCreate}
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
          </div>

          <p className="text-center font-mono text-[10px] text-muted-foreground">
            CaseBuilder can organize legal information, but it is not a lawyer and does not make filings court-ready.
          </p>
        </div>
      </div>
    </div>
  )
}

function formatUnknownError(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function intentRole(intent: Intent, fallback: MatterSide): MatterSide {
  if (intent === "fight") return "defendant"
  if (intent === "build") return "plaintiff"
  return fallback
}

function initialSettingsPatch(
  defaults: typeof FALLBACK_SETTINGS,
  values: {
    default_confidentiality: string
    default_document_type: CaseBuilderUserSettings["default_document_type"]
    auto_index_uploads: boolean
    auto_import_complaints: boolean
    preserve_folder_paths: boolean
    timeline_suggestions_enabled: boolean
    ai_timeline_enrichment_enabled: boolean
    transcript_redact_pii: boolean
    transcript_speaker_labels: boolean
    transcript_default_view: string
    transcript_prompt_preset: string
    transcript_remove_audio_tags: boolean
    export_default_format: string
    export_include_exhibits: boolean
    export_include_qc_report: boolean
  },
) {
  const patch: PatchCaseBuilderMatterSettingsInput = {}
  for (const [key, value] of Object.entries(values)) {
    if (defaults[key as keyof typeof defaults] !== value) {
      ;(patch as Record<string, string | boolean | null | undefined>)[key] = value
    }
  }
  return Object.keys(patch).length > 0 ? patch : undefined
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

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string
  checked: boolean
  onChange: (value: boolean) => void
}) {
  return (
    <label className="flex min-h-10 items-center justify-between gap-3 rounded border border-border bg-background px-3 py-2 text-sm text-foreground">
      <span>{label}</span>
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} className="h-4 w-4 accent-primary" />
    </label>
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
