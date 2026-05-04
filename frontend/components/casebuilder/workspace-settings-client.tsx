"use client"

import { useMemo, useState } from "react"
import { useRouter } from "next/navigation"
import { RotateCcw, Save, SlidersHorizontal } from "lucide-react"
import { patchCaseBuilderSettings } from "@/lib/casebuilder/api"
import type { CaseBuilderUserSettings, CaseBuilderUserSettingsResponse, MatterSide, MatterType, PatchCaseBuilderUserSettingsInput } from "@/lib/casebuilder/types"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"

const MATTER_TYPES: Array<{ value: MatterType; label: string }> = [
  { value: "civil", label: "Civil" },
  { value: "landlord_tenant", label: "Landlord / tenant" },
  { value: "employment", label: "Employment" },
  { value: "small_claims", label: "Small claims" },
  { value: "family", label: "Family" },
  { value: "admin", label: "Administrative" },
  { value: "criminal", label: "Criminal" },
  { value: "appeal", label: "Appeal" },
  { value: "fact_check", label: "Fact check" },
  { value: "complaint_analysis", label: "Complaint analysis" },
  { value: "other", label: "Other" },
]

const ROLES: Array<{ value: MatterSide; label: string }> = [
  { value: "plaintiff", label: "Plaintiff" },
  { value: "defendant", label: "Defendant" },
  { value: "petitioner", label: "Petitioner" },
  { value: "respondent", label: "Respondent" },
  { value: "neutral", label: "Neutral" },
  { value: "researcher", label: "Researcher" },
]

const DOCUMENT_TYPES = [
  "complaint",
  "answer",
  "motion",
  "order",
  "contract",
  "lease",
  "email",
  "letter",
  "notice",
  "medical",
  "police",
  "agency_record",
  "public_record",
  "spreadsheet",
  "photo",
  "screenshot",
  "audio_transcript",
  "receipt",
  "invoice",
  "evidence",
  "exhibit",
  "other",
] as const

const CONFIDENTIALITY = ["private", "filed", "public", "sealed"] as const
const TRANSCRIPT_PRESETS = ["unclear", "unclear_masked", "verbatim_multilingual", "legal", "medical", "financial", "technical", "code_switching", "customer_support"] as const
const EXPORT_FORMATS = ["pdf", "docx", "html", "markdown", "text", "json"] as const

export function WorkspaceSettingsClient({ initial }: { initial: CaseBuilderUserSettingsResponse }) {
  const router = useRouter()
  const [settings, setSettings] = useState(initial.settings)
  const [saved, setSaved] = useState(initial.settings)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const dirty = useMemo(() => JSON.stringify(settings) !== JSON.stringify(saved), [settings, saved])

  function update(patch: Partial<CaseBuilderUserSettings>) {
    setSettings((current) => ({ ...current, ...patch }))
  }

  async function save() {
    setBusy(true)
    setError(null)
    setMessage(null)
    const result = await patchCaseBuilderSettings(settingsPatch(settings))
    setBusy(false)
    if (!result.data) {
      setError(result.error ?? "Could not save CaseBuilder settings.")
      return
    }
    setSettings(result.data.settings)
    setSaved(result.data.settings)
    setMessage("Settings saved.")
    router.refresh()
  }

  return (
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
      <header className="border-b border-border bg-card px-6 py-5">
        <div className="mx-auto flex max-w-6xl flex-col gap-3 md:flex-row md:items-end md:justify-between">
          <div>
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <SlidersHorizontal className="h-3.5 w-3.5 text-primary" />
              CaseBuilder settings
            </div>
            <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">
              {settings.workspace_label || "Workspace defaults"}
            </h1>
            <div className="mt-1 font-mono text-[10px] text-muted-foreground">
              {initial.principal.email ?? initial.principal.subject}
            </div>
          </div>
          <SaveBar dirty={dirty} busy={busy} onReset={() => setSettings(saved)} onSave={save} />
        </div>
        <StatusLine message={message} error={error} />
      </header>

      <div className="px-6 py-6">
        <Tabs defaultValue="account" className="mx-auto max-w-6xl">
          <TabsList className="flex h-auto w-full flex-wrap justify-start rounded-md">
            <TabsTrigger value="account">Account</TabsTrigger>
            <TabsTrigger value="matter">Matter defaults</TabsTrigger>
            <TabsTrigger value="intake">Intake</TabsTrigger>
            <TabsTrigger value="transcript">Transcripts</TabsTrigger>
            <TabsTrigger value="ai">AI</TabsTrigger>
            <TabsTrigger value="export">Export</TabsTrigger>
          </TabsList>

          <TabsContent value="account" className="mt-4">
            <SettingsPanel title="Account">
              <Field label="Workspace label">
                <TextInput value={settings.workspace_label ?? ""} onChange={(value) => update({ workspace_label: value || null })} />
              </Field>
              <Field label="Display name">
                <TextInput value={settings.display_name ?? ""} onChange={(value) => update({ display_name: value || null })} />
              </Field>
              <ReadOnlyGrid rows={[
                ["Subject", initial.principal.subject],
                ["Email", initial.principal.email ?? "not provided"],
                ["Roles", initial.principal.roles.join(", ") || "none"],
              ]} />
            </SettingsPanel>
          </TabsContent>

          <TabsContent value="matter" className="mt-4">
            <SettingsPanel title="Matter defaults">
              <Field label="Default matter type">
                <Select value={settings.default_matter_type} onChange={(value) => update({ default_matter_type: value as MatterType })} options={MATTER_TYPES} />
              </Field>
              <Field label="Default role">
                <Select value={settings.default_user_role} onChange={(value) => update({ default_user_role: value as MatterSide })} options={ROLES} />
              </Field>
              <Field label="Default jurisdiction">
                <TextInput value={settings.default_jurisdiction} onChange={(value) => update({ default_jurisdiction: value })} />
              </Field>
              <Field label="Default court">
                <TextInput value={settings.default_court} onChange={(value) => update({ default_court: value })} />
              </Field>
            </SettingsPanel>
          </TabsContent>

          <TabsContent value="intake" className="mt-4">
            <SettingsPanel title="Intake and privacy">
              <Field label="Default confidentiality">
                <Select value={settings.default_confidentiality} onChange={(value) => update({ default_confidentiality: value })} options={CONFIDENTIALITY.map(option)} />
              </Field>
              <Field label="Fallback document type">
                <Select value={settings.default_document_type} onChange={(value) => update({ default_document_type: value as CaseBuilderUserSettings["default_document_type"] })} options={DOCUMENT_TYPES.map(option)} />
              </Field>
              <Toggle label="Auto-index uploads" checked={settings.auto_index_uploads} onChange={(value) => update({ auto_index_uploads: value })} />
              <Toggle label="Auto-import complaints" checked={settings.auto_import_complaints} onChange={(value) => update({ auto_import_complaints: value })} />
              <Toggle label="Preserve folder paths" checked={settings.preserve_folder_paths} onChange={(value) => update({ preserve_folder_paths: value })} />
            </SettingsPanel>
          </TabsContent>

          <TabsContent value="transcript" className="mt-4">
            <SettingsPanel title="Transcript defaults">
              <Toggle label="Redact PII by default" checked={settings.transcript_redact_pii} onChange={(value) => update({ transcript_redact_pii: value })} />
              <Toggle label="Speaker labels" checked={settings.transcript_speaker_labels} onChange={(value) => update({ transcript_speaker_labels: value })} />
              <Toggle label="Remove audio tags" checked={settings.transcript_remove_audio_tags} onChange={(value) => update({ transcript_remove_audio_tags: value })} />
              <Field label="Default view">
                <Select value={settings.transcript_default_view} onChange={(value) => update({ transcript_default_view: value })} options={["redacted", "raw"].map(option)} />
              </Field>
              <Field label="Prompt preset">
                <Select value={settings.transcript_prompt_preset} onChange={(value) => update({ transcript_prompt_preset: value })} options={TRANSCRIPT_PRESETS.map(option)} />
              </Field>
            </SettingsPanel>
          </TabsContent>

          <TabsContent value="ai" className="mt-4">
            <SettingsPanel title="AI and timeline">
              <Toggle label="Timeline suggestions" checked={settings.timeline_suggestions_enabled} onChange={(value) => update({ timeline_suggestions_enabled: value })} />
              <Toggle label="AI timeline enrichment" checked={settings.ai_timeline_enrichment_enabled} onChange={(value) => update({ ai_timeline_enrichment_enabled: value })} />
            </SettingsPanel>
          </TabsContent>

          <TabsContent value="export" className="mt-4">
            <SettingsPanel title="Export defaults">
              <Field label="Default format">
                <Select value={settings.export_default_format} onChange={(value) => update({ export_default_format: value })} options={EXPORT_FORMATS.map(option)} />
              </Field>
              <Toggle label="Include exhibits" checked={settings.export_include_exhibits} onChange={(value) => update({ export_include_exhibits: value })} />
              <Toggle label="Include QC report" checked={settings.export_include_qc_report} onChange={(value) => update({ export_include_qc_report: value })} />
            </SettingsPanel>
          </TabsContent>
        </Tabs>
      </div>
    </div>
  )
}

function settingsPatch(settings: CaseBuilderUserSettings): PatchCaseBuilderUserSettingsInput {
  return {
    workspace_label: settings.workspace_label ?? null,
    display_name: settings.display_name ?? null,
    default_matter_type: settings.default_matter_type,
    default_user_role: settings.default_user_role,
    default_jurisdiction: settings.default_jurisdiction,
    default_court: settings.default_court,
    default_confidentiality: settings.default_confidentiality,
    default_document_type: settings.default_document_type,
    auto_index_uploads: settings.auto_index_uploads,
    auto_import_complaints: settings.auto_import_complaints,
    preserve_folder_paths: settings.preserve_folder_paths,
    timeline_suggestions_enabled: settings.timeline_suggestions_enabled,
    ai_timeline_enrichment_enabled: settings.ai_timeline_enrichment_enabled,
    transcript_redact_pii: settings.transcript_redact_pii,
    transcript_speaker_labels: settings.transcript_speaker_labels,
    transcript_default_view: settings.transcript_default_view,
    transcript_prompt_preset: settings.transcript_prompt_preset,
    transcript_remove_audio_tags: settings.transcript_remove_audio_tags,
    export_default_format: settings.export_default_format,
    export_include_exhibits: settings.export_include_exhibits,
    export_include_qc_report: settings.export_include_qc_report,
  }
}

function option(value: string) {
  return { value, label: value.replace(/_/g, " ") }
}

function SaveBar({ dirty, busy, onReset, onSave }: { dirty: boolean; busy: boolean; onReset: () => void; onSave: () => void }) {
  return (
    <div className="flex items-center gap-2">
      <Button type="button" variant="outline" size="sm" disabled={!dirty || busy} onClick={onReset}>
        <RotateCcw className="h-4 w-4" />
        Reset
      </Button>
      <Button type="button" size="sm" disabled={!dirty || busy} onClick={onSave}>
        <Save className="h-4 w-4" />
        {busy ? "Saving" : "Save"}
      </Button>
    </div>
  )
}

function StatusLine({ message, error }: { message: string | null; error: string | null }) {
  if (!message && !error) return null
  return (
    <div className={cn("mx-auto mt-3 max-w-6xl rounded-md border px-3 py-2 text-sm", error ? "border-destructive/30 bg-destructive/10 text-destructive" : "border-primary/20 bg-primary/10 text-primary")}>
      {error ?? message}
    </div>
  )
}

function SettingsPanel({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="rounded-md border border-border bg-card p-4">
      <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{title}</h2>
      <div className="mt-4 grid gap-4 md:grid-cols-2">{children}</div>
    </section>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="grid gap-1.5">
      <span className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{label}</span>
      {children}
    </label>
  )
}

function TextInput({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return (
    <input
      value={value}
      onChange={(event) => onChange(event.target.value)}
      className="h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary"
    />
  )
}

function Select({ value, onChange, options }: { value: string; onChange: (value: string) => void; options: Array<{ value: string; label: string }> }) {
  return (
    <select value={value} onChange={(event) => onChange(event.target.value)} className="h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary">
      {options.map((item) => (
        <option key={item.value} value={item.value}>{item.label}</option>
      ))}
    </select>
  )
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) {
  return (
    <label className="flex min-h-10 items-center justify-between gap-4 rounded-md border border-border bg-background px-3 py-2">
      <span className="text-sm text-foreground">{label}</span>
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} className="h-4 w-4 accent-primary" />
    </label>
  )
}

function ReadOnlyGrid({ rows }: { rows: Array<[string, string]> }) {
  return (
    <div className="rounded-md border border-border bg-background md:col-span-2">
      {rows.map(([label, value]) => (
        <div key={label} className="grid grid-cols-[8rem_1fr] gap-3 border-b border-border px-3 py-2 last:border-b-0">
          <span className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{label}</span>
          <span className="truncate text-sm text-foreground">{value}</span>
        </div>
      ))}
    </div>
  )
}
