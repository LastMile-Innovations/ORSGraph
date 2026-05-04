"use client"

import { useMemo, useState } from "react"
import { useRouter } from "next/navigation"
import { RotateCcw, Save, Settings } from "lucide-react"
import { patchMatterConfig, type PatchMatterConfigInput } from "@/lib/casebuilder/api"
import type { CaseBuilderMatterSettings, CaseBuilderMatterSettingsResponse, MatterSide, MatterStatus, MatterSummary, MatterType } from "@/lib/casebuilder/types"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"

const INHERIT = "__inherit__"
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
const STATUSES: Array<{ value: MatterStatus; label: string }> = [
  { value: "intake", label: "Intake" },
  { value: "active", label: "Active" },
  { value: "stayed", label: "Stayed" },
  { value: "closed", label: "Closed" },
  { value: "appeal", label: "Appeal" },
]
const ROLES: Array<{ value: MatterSide; label: string }> = [
  { value: "plaintiff", label: "Plaintiff" },
  { value: "defendant", label: "Defendant" },
  { value: "petitioner", label: "Petitioner" },
  { value: "respondent", label: "Respondent" },
  { value: "neutral", label: "Neutral" },
  { value: "researcher", label: "Researcher" },
]
const CONFIDENTIALITY = ["private", "filed", "public", "sealed"] as const
const DOCUMENT_TYPES = ["complaint", "answer", "motion", "order", "contract", "lease", "email", "letter", "notice", "medical", "police", "agency_record", "public_record", "spreadsheet", "photo", "screenshot", "audio_transcript", "receipt", "invoice", "evidence", "exhibit", "other"] as const
const TRANSCRIPT_PRESETS = ["unclear", "unclear_masked", "verbatim_multilingual", "legal", "medical", "financial", "technical", "code_switching", "customer_support"] as const
const EXPORT_FORMATS = ["pdf", "docx", "html", "markdown", "text", "json"] as const

interface MatterDetailsDraft {
  name: string
  matter_type: MatterType
  status: MatterStatus
  user_role: MatterSide
  jurisdiction: string
  court: string
  case_number: string
}

export function MatterSettingsClient({ initial }: { initial: CaseBuilderMatterSettingsResponse }) {
  const router = useRouter()
  const [details, setDetails] = useState(detailsFromMatter(initial.matter))
  const [settings, setSettings] = useState(initial.settings)
  const [saved, setSaved] = useState({ details: detailsFromMatter(initial.matter), settings: initial.settings })
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const dirty = useMemo(() => JSON.stringify({ details, settings }) !== JSON.stringify(saved), [details, saved, settings])

  function updateDetails(patch: Partial<MatterDetailsDraft>) {
    setDetails((current) => ({ ...current, ...patch }))
  }

  function updateSettings(patch: Partial<CaseBuilderMatterSettings>) {
    setSettings((current) => ({ ...current, ...patch }))
  }

  async function save() {
    setBusy(true)
    setError(null)
    setMessage(null)
    const input: PatchMatterConfigInput = {
      matter: {
        name: details.name,
        matter_type: details.matter_type,
        status: details.status,
        user_role: details.user_role,
        jurisdiction: details.jurisdiction,
        court: details.court,
        case_number: details.case_number || null,
      },
      settings: matterSettingsPatch(settings),
    }
    const result = await patchMatterConfig(initial.matter.matter_id, input)
    setBusy(false)
    if (!result.data) {
      setError(result.error ?? "Could not save matter settings.")
      return
    }
    const nextDetails = detailsFromMatter(result.data.matter)
    setDetails(nextDetails)
    setSettings(result.data.settings)
    setSaved({ details: nextDetails, settings: result.data.settings })
    setMessage("Matter settings saved.")
    router.refresh()
  }

  return (
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
      <header className="border-b border-border bg-card px-6 py-5">
        <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
          <div>
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <Settings className="h-3.5 w-3.5 text-primary" />
              Matter settings
            </div>
            <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">{initial.matter.name}</h1>
            <div className="mt-1 font-mono text-[10px] text-muted-foreground">
              Effective privacy: {initial.effective.default_confidentiality} / upload indexing: {initial.effective.auto_index_uploads ? "on" : "off"}
            </div>
          </div>
          <SaveBar dirty={dirty} busy={busy} onReset={() => {
            setDetails(saved.details)
            setSettings(saved.settings)
          }} onSave={save} />
        </div>
        <StatusLine message={message} error={error} />
      </header>

      <div className="px-6 py-6">
        <Tabs defaultValue="details">
          <TabsList className="flex h-auto w-full flex-wrap justify-start rounded-md">
            <TabsTrigger value="details">Details</TabsTrigger>
            <TabsTrigger value="intake">Intake</TabsTrigger>
            <TabsTrigger value="transcript">Transcripts</TabsTrigger>
            <TabsTrigger value="ai">AI</TabsTrigger>
            <TabsTrigger value="export">Export</TabsTrigger>
          </TabsList>

          <TabsContent value="details" className="mt-4">
            <SettingsPanel title="Matter details">
              <Field label="Matter name">
                <TextInput value={details.name} onChange={(value) => updateDetails({ name: value })} />
              </Field>
              <Field label="Status">
                <Select value={details.status} onChange={(value) => updateDetails({ status: value as MatterStatus })} options={STATUSES} />
              </Field>
              <Field label="Matter type">
                <Select value={details.matter_type} onChange={(value) => updateDetails({ matter_type: value as MatterType })} options={MATTER_TYPES} />
              </Field>
              <Field label="Your role">
                <Select value={details.user_role} onChange={(value) => updateDetails({ user_role: value as MatterSide })} options={ROLES} />
              </Field>
              <Field label="Jurisdiction">
                <TextInput value={details.jurisdiction} onChange={(value) => updateDetails({ jurisdiction: value })} />
              </Field>
              <Field label="Court">
                <TextInput value={details.court} onChange={(value) => updateDetails({ court: value })} />
              </Field>
              <Field label="Case number">
                <TextInput value={details.case_number} onChange={(value) => updateDetails({ case_number: value })} />
              </Field>
            </SettingsPanel>
          </TabsContent>

          <TabsContent value="intake" className="mt-4">
            <SettingsPanel title="Intake and privacy">
              <Field label="Default confidentiality">
                <InheritedSelect value={settings.default_confidentiality} inherited={initial.effective.default_confidentiality} onChange={(value) => updateSettings({ default_confidentiality: value })} options={CONFIDENTIALITY.map(option)} />
              </Field>
              <Field label="Fallback document type">
                <InheritedSelect value={settings.default_document_type} inherited={initial.effective.default_document_type} onChange={(value) => updateSettings({ default_document_type: value as CaseBuilderMatterSettings["default_document_type"] })} options={DOCUMENT_TYPES.map(option)} />
              </Field>
              <InheritedToggle label="Auto-index uploads" value={settings.auto_index_uploads} inherited={initial.effective.auto_index_uploads} onChange={(value) => updateSettings({ auto_index_uploads: value })} />
              <InheritedToggle label="Auto-import complaints" value={settings.auto_import_complaints} inherited={initial.effective.auto_import_complaints} onChange={(value) => updateSettings({ auto_import_complaints: value })} />
              <InheritedToggle label="Preserve folder paths" value={settings.preserve_folder_paths} inherited={initial.effective.preserve_folder_paths} onChange={(value) => updateSettings({ preserve_folder_paths: value })} />
            </SettingsPanel>
          </TabsContent>

          <TabsContent value="transcript" className="mt-4">
            <SettingsPanel title="Transcript defaults">
              <InheritedToggle label="Redact PII" value={settings.transcript_redact_pii} inherited={initial.effective.transcript_redact_pii} onChange={(value) => updateSettings({ transcript_redact_pii: value })} />
              <InheritedToggle label="Speaker labels" value={settings.transcript_speaker_labels} inherited={initial.effective.transcript_speaker_labels} onChange={(value) => updateSettings({ transcript_speaker_labels: value })} />
              <InheritedToggle label="Remove audio tags" value={settings.transcript_remove_audio_tags} inherited={initial.effective.transcript_remove_audio_tags} onChange={(value) => updateSettings({ transcript_remove_audio_tags: value })} />
              <Field label="Default view">
                <InheritedSelect value={settings.transcript_default_view} inherited={initial.effective.transcript_default_view} onChange={(value) => updateSettings({ transcript_default_view: value })} options={["redacted", "raw"].map(option)} />
              </Field>
              <Field label="Prompt preset">
                <InheritedSelect value={settings.transcript_prompt_preset} inherited={initial.effective.transcript_prompt_preset} onChange={(value) => updateSettings({ transcript_prompt_preset: value })} options={TRANSCRIPT_PRESETS.map(option)} />
              </Field>
            </SettingsPanel>
          </TabsContent>

          <TabsContent value="ai" className="mt-4">
            <SettingsPanel title="AI and timeline">
              <InheritedToggle label="Timeline suggestions" value={settings.timeline_suggestions_enabled} inherited={initial.effective.timeline_suggestions_enabled} onChange={(value) => updateSettings({ timeline_suggestions_enabled: value })} />
              <InheritedToggle label="AI timeline enrichment" value={settings.ai_timeline_enrichment_enabled} inherited={initial.effective.ai_timeline_enrichment_enabled} onChange={(value) => updateSettings({ ai_timeline_enrichment_enabled: value })} />
            </SettingsPanel>
          </TabsContent>

          <TabsContent value="export" className="mt-4">
            <SettingsPanel title="Export defaults">
              <Field label="Default format">
                <InheritedSelect value={settings.export_default_format} inherited={initial.effective.export_default_format} onChange={(value) => updateSettings({ export_default_format: value })} options={EXPORT_FORMATS.map(option)} />
              </Field>
              <InheritedToggle label="Include exhibits" value={settings.export_include_exhibits} inherited={initial.effective.export_include_exhibits} onChange={(value) => updateSettings({ export_include_exhibits: value })} />
              <InheritedToggle label="Include QC report" value={settings.export_include_qc_report} inherited={initial.effective.export_include_qc_report} onChange={(value) => updateSettings({ export_include_qc_report: value })} />
            </SettingsPanel>
          </TabsContent>
        </Tabs>
      </div>
    </div>
  )
}

function detailsFromMatter(matter: MatterSummary): MatterDetailsDraft {
  return {
    name: matter.name,
    matter_type: matter.matter_type,
    status: matter.status,
    user_role: matter.user_role,
    jurisdiction: matter.jurisdiction,
    court: matter.court,
    case_number: matter.case_number ?? "",
  }
}

function matterSettingsPatch(settings: CaseBuilderMatterSettings): NonNullable<PatchMatterConfigInput["settings"]> {
  return {
    default_confidentiality: settings.default_confidentiality ?? null,
    default_document_type: settings.default_document_type ?? null,
    auto_index_uploads: settings.auto_index_uploads ?? null,
    auto_import_complaints: settings.auto_import_complaints ?? null,
    preserve_folder_paths: settings.preserve_folder_paths ?? null,
    timeline_suggestions_enabled: settings.timeline_suggestions_enabled ?? null,
    ai_timeline_enrichment_enabled: settings.ai_timeline_enrichment_enabled ?? null,
    transcript_redact_pii: settings.transcript_redact_pii ?? null,
    transcript_speaker_labels: settings.transcript_speaker_labels ?? null,
    transcript_default_view: settings.transcript_default_view ?? null,
    transcript_prompt_preset: settings.transcript_prompt_preset ?? null,
    transcript_remove_audio_tags: settings.transcript_remove_audio_tags ?? null,
    export_default_format: settings.export_default_format ?? null,
    export_include_exhibits: settings.export_include_exhibits ?? null,
    export_include_qc_report: settings.export_include_qc_report ?? null,
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
    <div className={cn("mt-3 rounded-md border px-3 py-2 text-sm", error ? "border-destructive/30 bg-destructive/10 text-destructive" : "border-primary/20 bg-primary/10 text-primary")}>
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
  return <input value={value} onChange={(event) => onChange(event.target.value)} className="h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary" />
}

function Select({ value, onChange, options }: { value: string; onChange: (value: string) => void; options: Array<{ value: string; label: string }> }) {
  return (
    <select value={value} onChange={(event) => onChange(event.target.value)} className="h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary">
      {options.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
    </select>
  )
}

function InheritedSelect({ value, inherited, onChange, options }: { value?: string | null; inherited: string; onChange: (value: string | null) => void; options: Array<{ value: string; label: string }> }) {
  return (
    <select value={value ?? INHERIT} onChange={(event) => onChange(event.target.value === INHERIT ? null : event.target.value)} className="h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary">
      <option value={INHERIT}>Inherit ({inherited})</option>
      {options.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
    </select>
  )
}

function InheritedToggle({ label, value, inherited, onChange }: { label: string; value?: boolean | null; inherited: boolean; onChange: (value: boolean | null) => void }) {
  const selectValue = value == null ? INHERIT : value ? "true" : "false"
  return (
    <label className="grid gap-1.5">
      <span className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{label}</span>
      <select
        value={selectValue}
        onChange={(event) => onChange(event.target.value === INHERIT ? null : event.target.value === "true")}
        className="h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary"
      >
        <option value={INHERIT}>Inherit ({inherited ? "on" : "off"})</option>
        <option value="true">On</option>
        <option value="false">Off</option>
      </select>
    </label>
  )
}
