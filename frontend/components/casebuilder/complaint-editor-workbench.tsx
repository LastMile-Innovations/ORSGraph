"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import {
  AlertTriangle,
  BookOpen,
  CheckCircle2,
  Download,
  FileText,
  GavelIcon,
  History,
  Keyboard,
  Layers3,
  Link2,
  ListChecks,
  MousePointer2,
  Plus,
  Save,
  Scale,
  ShieldCheck,
  Sparkles,
} from "lucide-react"
import type {
  ComplaintCaption,
  ComplaintDraft,
  ChangeSet,
  CompareVersionsResponse,
  Matter,
  PleadingParagraph,
  SignatureBlock,
  VersionSnapshot,
  WorkProductArtifact,
} from "@/lib/casebuilder/types"
import {
  createComplaint,
  createComplaintCount,
  createComplaintParagraph,
  compareWorkProductVersions,
  createWorkProductSnapshot,
  exportWorkProduct,
  getWorkProductExportHistory,
  getWorkProductHistory,
  getWorkProductSnapshots,
  linkComplaintSupport,
  patchComplaint,
  patchComplaintFinding,
  patchComplaintParagraph,
  previewComplaint,
  renumberComplaintParagraphs,
  restoreWorkProductVersion,
  runComplaintAiCommand,
  runComplaintQc,
} from "@/lib/casebuilder/api"
import { matterClaimsHref, matterComplaintHref, matterDocumentHref, matterFactsHref, type ComplaintWorkspaceSection } from "@/lib/casebuilder/routes"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Separator } from "@/components/ui/separator"
import { sanitizePreviewHtml } from "@/lib/safe-html"
import { RichEditor } from "./rich-editor"
import { cn } from "@/lib/utils"

interface ComplaintEditorWorkbenchProps {
  matter: Matter
  complaint: ComplaintDraft | null
  mode: ComplaintWorkspaceSection | "home"
}

type WorkbenchTab = "support" | "authority" | "rules" | "format" | "ai" | "history"

export function ComplaintEditorWorkbench({ matter, complaint: initialComplaint, mode }: ComplaintEditorWorkbenchProps) {
  const router = useRouter()
  const [complaint, setComplaint] = useState(initialComplaint)
  const [selectedId, setSelectedId] = useState(initialComplaint?.paragraphs[0]?.paragraph_id ?? "")
  const [tab, setTab] = useState<WorkbenchTab>("support")
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)
  const [newParagraphText, setNewParagraphText] = useState("")
  const [newCountTitle, setNewCountTitle] = useState("")
  const [citationText, setCitationText] = useState("")
  const [previewHtml, setPreviewHtml] = useState<string | null>(null)
  const [exportFormat, setExportFormat] = useState("html")

  const selectedParagraph = useMemo(
    () => complaint?.paragraphs.find((paragraph) => paragraph.paragraph_id === selectedId) ?? complaint?.paragraphs[0] ?? null,
    [complaint, selectedId],
  )

  async function runAction(action: () => Promise<string | null>) {
    setPending(true)
    setError(null)
    setMessage(null)
    const problem = await action()
    setPending(false)
    if (problem) setError(problem)
  }

  async function ensureComplaint() {
    if (complaint) return complaint
    const created = await createComplaint(matter.id, { title: `${matter.shortName || matter.name} complaint` })
    if (!created.data) throw new Error(created.error || "Complaint could not be created.")
    setComplaint(created.data)
    setSelectedId(created.data.paragraphs[0]?.paragraph_id ?? "")
    return created.data
  }

  const createStructuredComplaint = () =>
    runAction(async () => {
      try {
        const created = await ensureComplaint()
        setMessage("Structured complaint created.")
        router.replace(matterComplaintHref(matter.id, "editor", { id: created.paragraphs[0]?.paragraph_id }))
        router.refresh()
      } catch (err) {
        return err instanceof Error ? err.message : "Complaint could not be created."
      }
      return null
    })

  const saveCaption = (caption: ComplaintCaption, signature: SignatureBlock) =>
    runAction(async () => {
      try {
        const current = await ensureComplaint()
        const result = await patchComplaint(matter.id, current.complaint_id, { caption, signature, setup_stage: "editor" })
        if (!result.data) return result.error || "Caption could not be saved."
        setComplaint(result.data)
        setMessage("Caption saved.")
        router.refresh()
      } catch (err) {
        return err instanceof Error ? err.message : "Caption could not be saved."
      }
      return null
    })

  const saveParagraph = (paragraph: PleadingParagraph, text: string) =>
    runAction(async () => {
      if (!complaint) return "Create the complaint first."
      const result = await patchComplaintParagraph(matter.id, complaint.complaint_id, paragraph.paragraph_id, { text })
      if (!result.data) return result.error || "Paragraph could not be saved."
      setComplaint(result.data)
      setMessage(`Paragraph ${paragraph.number} saved.`)
      router.refresh()
      return null
    })

  const addParagraph = () =>
    runAction(async () => {
      const current = await ensureComplaint()
      const text = newParagraphText.trim()
      if (!text) return "Add paragraph text first."
      const result = await createComplaintParagraph(matter.id, current.complaint_id, {
        section_id: current.sections.find((section) => section.section_type === "facts")?.section_id,
        role: "factual_allegation",
        text,
      })
      if (!result.data) return result.error || "Paragraph could not be added."
      setComplaint(result.data)
      setNewParagraphText("")
      setSelectedId(result.data.paragraphs.at(-1)?.paragraph_id ?? "")
      setMessage("Paragraph added.")
      router.refresh()
      return null
    })

  const addCount = () =>
    runAction(async () => {
      const current = await ensureComplaint()
      const claim = matter.claims.find((item) => item.kind !== "defense")
      const result = await createComplaintCount(matter.id, current.complaint_id, {
        title: newCountTitle.trim() || claim?.title || `Count ${current.counts.length + 1}`,
        claim_id: claim?.id,
        legal_theory: claim?.theory || claim?.legal_theory,
        element_ids: claim?.elements.map((element) => element.id),
      })
      if (!result.data) return result.error || "Count could not be added."
      setComplaint(result.data)
      setNewCountTitle("")
      setMessage("Count added.")
      router.refresh()
      return null
    })

  const linkSelectedFact = (factId: string) =>
    runAction(async () => {
      if (!complaint || !selectedParagraph) return "Select a paragraph first."
      const result = await linkComplaintSupport(matter.id, complaint.complaint_id, {
        target_type: "paragraph",
        target_id: selectedParagraph.paragraph_id,
        fact_id: factId,
        relation: "supports",
      })
      if (!result.data) return result.error || "Fact could not be linked."
      setComplaint(result.data)
      setMessage("Fact linked.")
      router.refresh()
      return null
    })

  const linkSelectedEvidence = (evidenceId: string) =>
    runAction(async () => {
      if (!complaint || !selectedParagraph) return "Select a paragraph first."
      const result = await linkComplaintSupport(matter.id, complaint.complaint_id, {
        target_type: "paragraph",
        target_id: selectedParagraph.paragraph_id,
        evidence_id: evidenceId,
        relation: "supports",
      })
      if (!result.data) return result.error || "Evidence could not be linked."
      setComplaint(result.data)
      setMessage("Evidence linked.")
      router.refresh()
      return null
    })

  const insertCitation = () =>
    runAction(async () => {
      if (!complaint || !selectedParagraph) return "Select a paragraph first."
      if (!citationText.trim()) return "Enter a citation first."
      const result = await linkComplaintSupport(matter.id, complaint.complaint_id, {
        target_type: "paragraph",
        target_id: selectedParagraph.paragraph_id,
        citation: citationText.trim(),
        canonical_id: citationText.trim(),
      })
      if (!result.data) return result.error || "Citation could not be inserted."
      setComplaint(result.data)
      setCitationText("")
      setMessage("Citation inserted for review.")
      router.refresh()
      return null
    })

  const runQc = () =>
    runAction(async () => {
      const current = await ensureComplaint()
      const result = await runComplaintQc(matter.id, current.complaint_id)
      if (!result.data) return result.error || "Complaint QC could not run."
      const refreshed = await patchComplaint(matter.id, current.complaint_id, {})
      if (refreshed.data) setComplaint(refreshed.data)
      setTab("rules")
      setMessage(result.data.message)
      router.refresh()
      return null
    })

  const resolveFinding = (findingId: string, status: "resolved" | "ignored" | "open") =>
    runAction(async () => {
      if (!complaint) return "Create the complaint first."
      const result = await patchComplaintFinding(matter.id, complaint.complaint_id, findingId, status)
      if (!result.data) return result.error || "Finding could not be updated."
      setComplaint(result.data)
      setMessage(`Finding marked ${status}.`)
      router.refresh()
      return null
    })

  const loadPreview = () =>
    runAction(async () => {
      const current = await ensureComplaint()
      const result = await previewComplaint(matter.id, current.complaint_id)
      if (!result.data) return result.error || "Preview could not be generated."
      setPreviewHtml(result.data.html)
      setMessage(result.data.review_label)
      return null
    })

  const createExport = () =>
    runAction(async () => {
      const current = await ensureComplaint()
      const result = await exportWorkProduct(matter.id, current.complaint_id, {
        format: exportFormat,
        profile: "clean_filing_copy",
        mode: "review_needed",
        include_exhibits: true,
        include_qc_report: true,
      })
      if (!result.data) return result.error || "Export could not be generated."
      setMessage(`${result.data.format.toUpperCase()} export generated for review.`)
      router.refresh()
      return null
    })

  const runAiCommand = (command: string) =>
    runAction(async () => {
      const current = await ensureComplaint()
      const result = await runComplaintAiCommand(matter.id, current.complaint_id, {
        command,
        target_id: selectedParagraph?.paragraph_id,
      })
      if (!result.data?.result) return result.error || "Command could not run."
      setComplaint(result.data.result)
      setTab("ai")
      setMessage(result.data.message)
      router.refresh()
      return null
    })

  if (!complaint) {
    return (
      <div className="flex flex-1 flex-col overflow-y-auto">
        <ComplaintHeader matter={matter} complaint={complaint} mode={mode} pending={pending} onRunQc={runQc} onPreview={loadPreview} onExport={createExport} />
        <main className="mx-auto flex w-full max-w-3xl flex-1 flex-col justify-center px-6 py-10">
          <div className="rounded-md border border-border bg-card p-6">
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <GavelIcon className="h-3.5 w-3.5 text-primary" />
              structured complaint
            </div>
            <h1 className="mt-2 text-xl font-semibold text-foreground">Create the complaint editor file</h1>
            <p className="mt-2 text-sm text-muted-foreground">
              This creates a complaint AST with caption, sections, paragraphs, counts, QC, preview, export, history, and review-needed states.
            </p>
            {error && <p className="mt-4 rounded border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">{error}</p>}
            <button
              type="button"
              onClick={createStructuredComplaint}
              disabled={pending}
              className="mt-5 inline-flex items-center gap-2 rounded bg-primary px-3 py-2 text-xs font-semibold text-primary-foreground disabled:opacity-60"
            >
              <Plus className="h-3.5 w-3.5" />
              {pending ? "Creating" : "Create complaint"}
            </button>
          </div>
        </main>
      </div>
    )
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <ComplaintHeader matter={matter} complaint={complaint} mode={mode} pending={pending} onRunQc={runQc} onPreview={loadPreview} onExport={createExport} />
      {(message || error) && (
        <div className={cn("border-b px-6 py-2 text-xs", error ? "border-destructive/30 bg-destructive/5 text-destructive" : "border-primary/20 bg-primary/5 text-muted-foreground")}>
          {error || message}
        </div>
      )}
      <div className="grid min-h-0 flex-1 grid-cols-1 overflow-hidden lg:grid-cols-[260px_minmax(0,1fr)_360px]">
        <ComplaintOutline matter={matter} complaint={complaint} selectedId={selectedParagraph?.paragraph_id ?? ""} mode={mode} onSelect={setSelectedId} />
        <main className="min-w-0 overflow-y-auto bg-background">
          {mode === "home" || mode === "editor" ? (
            <EditorPanel
              complaint={complaint}
              selectedParagraph={selectedParagraph}
              newParagraphText={newParagraphText}
              newCountTitle={newCountTitle}
              pending={pending}
              onParagraphText={setNewParagraphText}
              onCountTitle={setNewCountTitle}
              onSaveCaption={saveCaption}
              onSaveParagraph={saveParagraph}
              onAddParagraph={addParagraph}
              onAddCount={addCount}
              onRenumber={() =>
                runAction(async () => {
                  const result = await renumberComplaintParagraphs(matter.id, complaint.complaint_id)
                  if (!result.data) return result.error || "Paragraphs could not be renumbered."
                  setComplaint(result.data)
                  setMessage("Paragraphs renumbered.")
                  router.refresh()
                  return null
                })
              }
            />
          ) : null}
          {mode === "outline" ? <OutlinePanel complaint={complaint} onSelect={setSelectedId} /> : null}
          {mode === "claims" ? <CountsPanel matter={matter} complaint={complaint} onAddCount={addCount} newCountTitle={newCountTitle} onCountTitle={setNewCountTitle} /> : null}
          {mode === "evidence" ? <EvidencePanel matter={matter} complaint={complaint} selectedParagraph={selectedParagraph} onLinkFact={linkSelectedFact} onLinkEvidence={linkSelectedEvidence} /> : null}
          {mode === "qc" ? <QcPanel findings={complaint.findings} onResolve={resolveFinding} onRunQc={runQc} /> : null}
          {mode === "preview" ? <PreviewPanel complaint={complaint} previewHtml={previewHtml} onPreview={loadPreview} /> : null}
          {mode === "export" ? <ExportPanel matterId={matter.id} complaint={complaint} exportFormat={exportFormat} onFormat={setExportFormat} onExport={createExport} /> : null}
          {mode === "history" ? <CaseHistoryPanel matterId={matter.id} complaint={complaint} onRefresh={() => router.refresh()} /> : null}
        </main>
        <ComplaintInspector
          matter={matter}
          complaint={complaint}
          selectedParagraph={selectedParagraph}
          tab={tab}
          onTab={setTab}
          citationText={citationText}
          onCitationText={setCitationText}
          onLinkFact={linkSelectedFact}
          onLinkEvidence={linkSelectedEvidence}
          onInsertCitation={insertCitation}
          onResolve={resolveFinding}
          onAiCommand={runAiCommand}
        />
      </div>
    </div>
  )
}

function ComplaintHeader({
  matter,
  complaint,
  mode,
  pending,
  onRunQc,
  onPreview,
  onExport,
}: {
  matter: Matter
  complaint: ComplaintDraft | null
  mode: ComplaintWorkspaceSection | "home"
  pending: boolean
  onRunQc: () => void
  onPreview: () => void
  onExport: () => void
}) {
  const sections: Array<{ id: ComplaintWorkspaceSection; label: string; icon: typeof FileText }> = [
    { id: "editor", label: "Editor", icon: FileText },
    { id: "outline", label: "Outline", icon: ListChecks },
    { id: "claims", label: "Counts", icon: Scale },
    { id: "evidence", label: "Support", icon: Link2 },
    { id: "qc", label: "QC", icon: ShieldCheck },
    { id: "preview", label: "Preview", icon: BookOpen },
    { id: "export", label: "Export", icon: Download },
    { id: "history", label: "History", icon: History },
  ]
  return (
    <header className="border-b border-border bg-card px-6 py-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <GavelIcon className="h-3.5 w-3.5 text-primary" />
            complaint editor
            <span className="rounded bg-warning/15 px-1.5 py-0.5 text-warning">review needed</span>
          </div>
          <h1 className="mt-1 truncate text-lg font-semibold text-foreground">{complaint?.title ?? `${matter.shortName || matter.name} complaint`}</h1>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
            <span>{complaint?.paragraphs.length ?? 0} paragraphs</span>
            <span>{complaint?.counts.length ?? 0} counts</span>
            <span>{complaint?.findings.filter((finding) => finding.status === "open").length ?? 0} open findings</span>
            <span>{complaint?.setup_stage ?? "not created"}</span>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button type="button" onClick={onRunQc} disabled={pending || !complaint} className="toolbar-button">
            <ShieldCheck className="h-3.5 w-3.5" />
            QC
          </button>
          <button type="button" onClick={onPreview} disabled={pending || !complaint} className="toolbar-button">
            <BookOpen className="h-3.5 w-3.5" />
            Preview
          </button>
          <button type="button" onClick={onExport} disabled={pending || !complaint} className="toolbar-button">
            <Download className="h-3.5 w-3.5" />
            Export
          </button>
        </div>
      </div>
      <nav className="mt-3 flex gap-1 overflow-x-auto">
        {sections.map((section) => {
          const Icon = section.icon
          const active = mode === section.id || (mode === "home" && section.id === "editor")
          return (
            <Link key={section.id} href={matterComplaintHref(matter.id, section.id)} className={cn("inline-flex items-center gap-1.5 rounded px-2.5 py-1.5 text-xs", active ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground")}>
              <Icon className="h-3.5 w-3.5" />
              {section.label}
            </Link>
          )
        })}
      </nav>
    </header>
  )
}

function ComplaintOutline({ matter, complaint, selectedId, mode, onSelect }: { matter: Matter; complaint: ComplaintDraft; selectedId: string; mode: string; onSelect: (id: string) => void }) {
  return (
    <aside className="hidden min-h-0 overflow-y-auto border-r border-border bg-card lg:block">
      <div className="border-b border-border p-3">
        <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">progress</div>
        <div className="mt-2 grid grid-cols-3 gap-1 text-center font-mono text-[10px]">
          <span className="rounded bg-muted px-1 py-1">{complaint.paragraphs.length} ¶</span>
          <span className="rounded bg-muted px-1 py-1">{complaint.counts.length} counts</span>
          <span className="rounded bg-muted px-1 py-1">{complaint.findings.filter((finding) => finding.status === "open").length} flags</span>
        </div>
      </div>
      <div className="p-2">
        {complaint.sections.map((section) => (
          <div key={section.section_id} className="mb-3">
            <Link href={matterComplaintHref(matter.id, "outline", { id: section.section_id, type: "section" })} className="block px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              {section.title}
            </Link>
            {complaint.paragraphs
              .filter((paragraph) => paragraph.section_id === section.section_id)
              .map((paragraph) => (
                <button
                  key={paragraph.paragraph_id}
                  type="button"
                  onClick={() => onSelect(paragraph.paragraph_id)}
                  className={cn("block w-full rounded px-2 py-1.5 text-left text-xs", selectedId === paragraph.paragraph_id ? "bg-muted text-foreground" : "text-muted-foreground hover:bg-muted/60 hover:text-foreground")}
                >
                  <span className="mr-1 font-mono text-[10px]">{paragraph.number}</span>
                  <span className="line-clamp-1">{paragraph.text}</span>
                </button>
              ))}
          </div>
        ))}
      </div>
      {mode !== "evidence" && (
        <div className="border-t border-border p-3 text-[11px] text-muted-foreground">
          <Link href={matterFactsHref(matter.id)} className="hover:text-foreground">Facts</Link>
          <span> · </span>
          <Link href={matterClaimsHref(matter.id)} className="hover:text-foreground">Claims</Link>
        </div>
      )}
    </aside>
  )
}

function EditorPanel({
  complaint,
  selectedParagraph,
  newParagraphText,
  newCountTitle,
  pending,
  onParagraphText,
  onCountTitle,
  onSaveCaption,
  onSaveParagraph,
  onAddParagraph,
  onAddCount,
  onRenumber,
}: {
  complaint: ComplaintDraft
  selectedParagraph: PleadingParagraph | null
  newParagraphText: string
  newCountTitle: string
  pending: boolean
  onParagraphText: (value: string) => void
  onCountTitle: (value: string) => void
  onSaveCaption: (caption: ComplaintCaption, signature: SignatureBlock) => void
  onSaveParagraph: (paragraph: PleadingParagraph, text: string) => void
  onAddParagraph: () => void
  onAddCount: () => void
  onRenumber: () => void
}) {
  const [caption, setCaption] = useState(complaint.caption)
  const [signature, setSignature] = useState(complaint.signature)
  const [paragraphText, setParagraphText] = useState(selectedParagraph?.text ?? "")

  useEffect(() => {
    setParagraphText(selectedParagraph?.text ?? "")
  }, [selectedParagraph?.paragraph_id, selectedParagraph?.text])

  return (
    <div className="space-y-6 p-6">
      <section className="rounded-lg border bg-card shadow-sm">
        <div className="flex items-center justify-between border-b px-5 py-3">
          <div className="flex items-center gap-2">
            <GavelIcon className="h-4 w-4 text-primary" />
            <h2 className="text-sm font-semibold text-foreground">Pleading Caption</h2>
          </div>
          <Button size="sm" className="h-8 gap-1.5" disabled={pending} onClick={() => onSaveCaption(caption, signature)}>
            <Save className="h-3.5 w-3.5" />
            Save Caption
          </Button>
        </div>
        <div className="p-5">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-1">
              <Label className="text-[10px] uppercase tracking-wider text-muted-foreground">Court</Label>
              <Input value={caption.court_name} onChange={(event) => setCaption({ ...caption, court_name: event.target.value })} className="h-9" placeholder="In the Circuit Court of..." />
            </div>
            <div className="space-y-1">
              <Label className="text-[10px] uppercase tracking-wider text-muted-foreground">County</Label>
              <Input value={caption.county} onChange={(event) => setCaption({ ...caption, county: event.target.value })} className="h-9" placeholder="Multnomah" />
            </div>
            <div className="space-y-1">
              <Label className="text-[10px] uppercase tracking-wider text-muted-foreground">Document Title</Label>
              <Input value={caption.document_title} onChange={(event) => setCaption({ ...caption, document_title: event.target.value })} className="h-9" placeholder="COMPLAINT" />
            </div>
            <div className="space-y-1">
              <Label className="text-[10px] uppercase tracking-wider text-muted-foreground">Case Number</Label>
              <Input value={caption.case_number ?? ""} onChange={(event) => setCaption({ ...caption, case_number: event.target.value })} className="h-9" placeholder="Pending" />
            </div>
            <div className="space-y-1">
              <Label className="text-[10px] uppercase tracking-wider text-muted-foreground">Plaintiffs</Label>
              <Input value={caption.plaintiff_names.join(", ")} onChange={(event) => setCaption({ ...caption, plaintiff_names: splitNames(event.target.value) })} className="h-9" placeholder="John Doe, Jane Smith" />
            </div>
            <div className="space-y-1">
              <Label className="text-[10px] uppercase tracking-wider text-muted-foreground">Defendants</Label>
              <Input value={caption.defendant_names.join(", ")} onChange={(event) => setCaption({ ...caption, defendant_names: splitNames(event.target.value) })} className="h-9" placeholder="Acme Corp" />
            </div>
          </div>
          
          <Separator className="my-5" />
          
          <div className="grid gap-4 md:grid-cols-3">
            <div className="space-y-1">
              <Label className="text-[10px] uppercase tracking-wider text-muted-foreground">Attorney Name</Label>
              <Input value={signature.name} onChange={(event) => setSignature({ ...signature, name: event.target.value })} className="h-9" />
            </div>
            <div className="space-y-1">
              <Label className="text-[10px] uppercase tracking-wider text-muted-foreground">Email</Label>
              <Input value={signature.email} onChange={(event) => setSignature({ ...signature, email: event.target.value })} className="h-9" />
            </div>
            <div className="space-y-1">
              <Label className="text-[10px] uppercase tracking-wider text-muted-foreground">Phone</Label>
              <Input value={signature.phone} onChange={(event) => setSignature({ ...signature, phone: event.target.value })} className="h-9" />
            </div>
          </div>
        </div>
      </section>

      <div className="grid gap-6 lg:grid-cols-[1fr_300px]">
        <div className="space-y-6">
          <section className="rounded-lg border bg-card shadow-sm">
            <div className="flex items-center justify-between border-b px-5 py-3">
              <div className="flex items-center gap-2">
                <FileText className="h-4 w-4 text-primary" />
                <h2 className="text-sm font-semibold text-foreground">
                  {selectedParagraph ? `Edit Paragraph ${selectedParagraph.number}` : "Select a Paragraph"}
                </h2>
              </div>
              {selectedParagraph && (
                <Button 
                  size="sm" 
                  className="h-8 gap-1.5" 
                  disabled={pending || paragraphText === selectedParagraph.text} 
                  onClick={() => onSaveParagraph(selectedParagraph, paragraphText)}
                >
                  <Save className="h-3.5 w-3.5" />
                  {paragraphText === selectedParagraph.text ? "Saved" : "Save Changes"}
                </Button>
              )}
            </div>
            <div className="p-5">
              {selectedParagraph ? (
                <RichEditor
                  content={paragraphText}
                  onChange={setParagraphText}
                  placeholder="Enter allegation text..."
                  minHeight="200px"
                />
              ) : (
                <div className="flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
                  <MousePointer2 className="mb-2 h-8 w-8 opacity-20" />
                  <p className="text-sm">Select a paragraph from the outline to edit.</p>
                </div>
              )}
            </div>
          </section>

          <section className="rounded-lg border border-dashed bg-muted/20 p-6">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                <Plus className="h-4 w-4 text-primary" />
                Add New Allegation
              </div>
              <Button size="sm" variant="outline" className="h-7 text-[10px] uppercase tracking-wider" onClick={onRenumber} disabled={pending}>
                <ListChecks className="mr-1.5 h-3 w-3" />
                Renumber All
              </Button>
            </div>
            <div className="space-y-4">
              <RichEditor
                content={newParagraphText}
                onChange={onParagraphText}
                placeholder="Type the next numbered allegation..."
                minHeight="100px"
              />
              <div className="flex justify-end">
                <Button className="gap-1.5" size="sm" disabled={pending || !newParagraphText.trim()} onClick={onAddParagraph}>
                  <Plus className="h-3.5 w-3.5" />
                  Insert Into Complaint
                </Button>
              </div>
            </div>
          </section>
        </div>

        <div className="space-y-6">
          <section className="rounded-lg border bg-card p-5 shadow-sm">
            <div className="flex items-center gap-2 mb-4">
              <Layers3 className="h-4 w-4 text-primary" />
              <h2 className="text-sm font-semibold text-foreground">Add Legal Count</h2>
            </div>
            <div className="space-y-3">
              <Input 
                value={newCountTitle} 
                onChange={(event) => onCountTitle(event.target.value)} 
                placeholder="Count Title (e.g., Negligence)" 
                className="h-9"
              />
              <Button className="w-full gap-1.5" size="sm" onClick={onAddCount} disabled={pending}>
                <Plus className="h-3.5 w-3.5" />
                Add Count
              </Button>
            </div>
          </section>

          <div className="rounded-lg border bg-primary/5 p-5 text-[11px] text-primary/80 leading-relaxed italic">
            "Complaints should be concise, numbered, and focused on material facts that support your legal theories."
          </div>
        </div>
      </div>
    </div>
  )
}

function OutlinePanel({ complaint, onSelect }: { complaint: ComplaintDraft; onSelect: (id: string) => void }) {
  return (
    <div className="p-4 md:p-6">
      <div className="space-y-3">
        {complaint.sections.map((section) => (
          <section key={section.section_id} className="rounded-md border border-border bg-card">
            <div className="border-b border-border px-4 py-3">
              <h2 className="text-sm font-semibold text-foreground">{section.title}</h2>
              <p className="mt-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{section.paragraph_ids.length} paragraphs</p>
            </div>
            <div className="divide-y divide-border">
              {complaint.paragraphs.filter((paragraph) => paragraph.section_id === section.section_id).map((paragraph) => (
                <button key={paragraph.paragraph_id} type="button" onClick={() => onSelect(paragraph.paragraph_id)} className="flex w-full items-start gap-3 px-4 py-3 text-left hover:bg-muted/50">
                  <span className="font-mono text-xs text-muted-foreground">{paragraph.number}</span>
                  <span className="text-sm text-foreground">{paragraph.text}</span>
                </button>
              ))}
            </div>
          </section>
        ))}
      </div>
    </div>
  )
}

function CountsPanel({ matter, complaint, newCountTitle, onCountTitle, onAddCount }: { matter: Matter; complaint: ComplaintDraft; newCountTitle: string; onCountTitle: (value: string) => void; onAddCount: () => void }) {
  return (
    <div className="space-y-4 p-4 md:p-6">
      <div className="rounded-md border border-border bg-card p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-foreground">Counts</h2>
            <p className="mt-1 text-xs text-muted-foreground">{matter.claims.filter((claim) => claim.kind !== "defense").length} matter claims available</p>
          </div>
          <div className="flex gap-2">
            <input value={newCountTitle} onChange={(event) => onCountTitle(event.target.value)} className="input-like" placeholder="Count title" />
            <button type="button" onClick={onAddCount} className="toolbar-button"><Plus className="h-3.5 w-3.5" />Add</button>
          </div>
        </div>
      </div>
      {complaint.counts.map((count) => (
        <section key={count.count_id} className="rounded-md border border-border bg-card p-4">
          <div className="flex items-start justify-between gap-3">
            <div>
              <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">Count {count.ordinal}</div>
              <h3 className="mt-1 text-sm font-semibold text-foreground">{count.title}</h3>
              <p className="mt-1 text-xs text-muted-foreground">{count.legal_theory || "Legal theory needs review."}</p>
            </div>
            <span className={cn("rounded px-2 py-1 font-mono text-[10px] uppercase", count.health === "supported_needs_review" ? "bg-success/15 text-success" : "bg-warning/15 text-warning")}>{count.health.replace(/_/g, " ")}</span>
          </div>
          {count.weaknesses.length > 0 && <p className="mt-3 text-xs text-warning">{count.weaknesses.join(" · ")}</p>}
        </section>
      ))}
    </div>
  )
}

function EvidencePanel({ matter, selectedParagraph, onLinkFact, onLinkEvidence }: { matter: Matter; complaint: ComplaintDraft; selectedParagraph: PleadingParagraph | null; onLinkFact: (id: string) => void; onLinkEvidence: (id: string) => void }) {
  return (
    <div className="grid gap-4 p-4 md:grid-cols-2 md:p-6">
      <section className="rounded-md border border-border bg-card">
        <div className="border-b border-border px-4 py-3"><h2 className="text-sm font-semibold text-foreground">Facts</h2></div>
        <div className="divide-y divide-border">
          {matter.facts.slice(0, 20).map((fact) => (
            <button key={fact.id} type="button" onClick={() => onLinkFact(fact.id)} disabled={!selectedParagraph} className="block w-full px-4 py-3 text-left text-sm hover:bg-muted/50 disabled:opacity-50">
              {fact.statement}
            </button>
          ))}
        </div>
      </section>
      <section className="rounded-md border border-border bg-card">
        <div className="border-b border-border px-4 py-3"><h2 className="text-sm font-semibold text-foreground">Evidence</h2></div>
        <div className="divide-y divide-border">
          {matter.evidence.slice(0, 20).map((evidence) => (
            <button key={evidence.evidence_id} type="button" onClick={() => onLinkEvidence(evidence.evidence_id)} disabled={!selectedParagraph} className="block w-full px-4 py-3 text-left text-sm hover:bg-muted/50 disabled:opacity-50">
              {evidence.quote || evidence.evidence_id}
            </button>
          ))}
        </div>
      </section>
    </div>
  )
}

function QcPanel({ findings, onResolve, onRunQc }: { findings: ComplaintDraft["findings"]; onResolve: (id: string, status: "resolved" | "ignored" | "open") => void; onRunQc: () => void }) {
  return (
    <div className="space-y-4 p-4 md:p-6">
      <div className="flex items-center justify-between gap-3 rounded-md border border-border bg-card p-4">
        <div>
          <h2 className="text-sm font-semibold text-foreground">Complaint QC</h2>
          <p className="mt-1 text-xs text-muted-foreground">{findings.filter((finding) => finding.status === "open").length} open findings</p>
        </div>
        <button type="button" onClick={onRunQc} className="toolbar-button"><ShieldCheck className="h-3.5 w-3.5" />Run QC</button>
      </div>
      <div className="divide-y divide-border rounded-md border border-border bg-card">
        {findings.length === 0 ? <p className="p-4 text-sm text-muted-foreground">No persisted complaint findings.</p> : findings.map((finding) => (
          <article key={finding.finding_id} className="p-4">
            <div className="flex items-start gap-3">
              {finding.status === "open" ? <AlertTriangle className="mt-0.5 h-4 w-4 text-warning" /> : <CheckCircle2 className="mt-0.5 h-4 w-4 text-success" />}
              <div className="min-w-0 flex-1">
                <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{finding.severity} · {finding.category}</div>
                <p className="mt-1 text-sm text-foreground">{finding.message}</p>
                <p className="mt-1 text-xs text-muted-foreground">{finding.suggested_fix}</p>
                <div className="mt-3 flex gap-2">
                  <button type="button" onClick={() => onResolve(finding.finding_id, "resolved")} className="toolbar-button">Resolve</button>
                  <button type="button" onClick={() => onResolve(finding.finding_id, "ignored")} className="toolbar-button">Ignore</button>
                </div>
              </div>
            </div>
          </article>
        ))}
      </div>
    </div>
  )
}

function PreviewPanel({ complaint, previewHtml, onPreview }: { complaint: ComplaintDraft; previewHtml: string | null; onPreview: () => void }) {
  return (
    <div className="p-4 md:p-6">
      <div className="mb-4 flex items-center justify-between rounded-md border border-border bg-card p-4">
        <div>
          <h2 className="text-sm font-semibold text-foreground">Court Paper Preview</h2>
          <p className="mt-1 text-xs text-muted-foreground">{complaint.formatting_profile.name} · review needed</p>
        </div>
        <button type="button" onClick={onPreview} className="toolbar-button"><BookOpen className="h-3.5 w-3.5" />Generate</button>
      </div>
      <div className="min-h-[680px] rounded border border-border bg-white p-8 text-black shadow-sm">
        {previewHtml ? <div dangerouslySetInnerHTML={{ __html: sanitizePreviewHtml(previewHtml) }} /> : <pre className="whitespace-pre-wrap text-sm">{complaint.paragraphs.map((paragraph) => `${paragraph.number}. ${paragraph.text}`).join("\n\n")}</pre>}
      </div>
    </div>
  )
}

function ExportPanel({
  matterId,
  complaint,
  exportFormat,
  onFormat,
  onExport,
}: {
  matterId: string
  complaint: ComplaintDraft
  exportFormat: string
  onFormat: (value: string) => void
  onExport: () => Promise<void> | void
}) {
  const [exports, setExports] = useState<WorkProductArtifact[]>([])
  const [loadingExports, setLoadingExports] = useState(false)

  const loadExports = useCallback(async () => {
    setLoadingExports(true)
    const result = await getWorkProductExportHistory(matterId, complaint.complaint_id)
    setExports(result.data)
    setLoadingExports(false)
  }, [matterId, complaint.complaint_id])

  useEffect(() => {
    void loadExports()
  }, [loadExports])

  const latestExport = exports[exports.length - 1]
  const changedSinceExport = latestExport?.changed_since_export === true

  return (
    <div className="space-y-4 p-4 md:p-6">
      <section className="rounded-md border border-border bg-card p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-foreground">Export</h2>
            <p className="mt-1 text-xs text-muted-foreground">
              {changedSinceExport ? "Draft changed since last export." : latestExport ? "Latest export matches the current draft hashes." : "No locked export snapshot yet."}
            </p>
          </div>
          <div className="flex gap-2">
            <select value={exportFormat} onChange={(event) => onFormat(event.target.value)} className="input-like">
              <option value="html">HTML</option>
              <option value="markdown">Markdown</option>
              <option value="text">Plain text</option>
              <option value="json">JSON AST</option>
              <option value="docx">DOCX skeleton</option>
              <option value="pdf">PDF skeleton</option>
            </select>
            <button
              type="button"
              onClick={async () => {
                await onExport()
                await loadExports()
              }}
              className="toolbar-button"
            >
              <Download className="h-3.5 w-3.5" />Generate
            </button>
          </div>
        </div>
      </section>
      <div className="divide-y divide-border rounded-md border border-border bg-card">
        {loadingExports ? <p className="p-4 text-sm text-muted-foreground">Loading export history...</p> : null}
        {!loadingExports && exports.length === 0 ? <p className="p-4 text-sm text-muted-foreground">No complaint exports yet.</p> : exports.map((artifact) => (
          <article key={artifact.artifact_id} className="p-4">
            <div className="flex items-center justify-between gap-3">
              <div>
                <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{artifact.format} · {artifact.status}</div>
                <p className="mt-1 text-sm text-foreground">{artifact.profile}</p>
              </div>
              <div className="text-right">
                <span className="block font-mono text-xs text-muted-foreground">{artifact.page_count} pages</span>
                {artifact.changed_since_export ? <span className="mt-1 block text-[11px] text-warning">Draft changed</span> : null}
              </div>
            </div>
            {artifact.snapshot_id ? <p className="mt-2 break-all font-mono text-[10px] text-muted-foreground">Snapshot {artifact.snapshot_id}</p> : null}
            {artifact.warnings.length > 0 && <p className="mt-2 text-xs text-warning">{artifact.warnings.join(" ")}</p>}
          </article>
        ))}
      </div>
    </div>
  )
}

function CaseHistoryPanel({
  matterId,
  complaint,
  onRefresh,
}: {
  matterId: string
  complaint: ComplaintDraft
  onRefresh: () => void
}) {
  const [history, setHistory] = useState<ChangeSet[]>([])
  const [snapshots, setSnapshots] = useState<VersionSnapshot[]>([])
  const [exports, setExports] = useState<WorkProductArtifact[]>([])
  const [selectedSnapshotId, setSelectedSnapshotId] = useState("")
  const [compareResult, setCompareResult] = useState<CompareVersionsResponse | null>(null)
  const [restorePlan, setRestorePlan] = useState<{ snapshotId: string; warnings: string[] } | null>(null)
  const [busy, setBusy] = useState(false)
  const [panelError, setPanelError] = useState<string | null>(null)

  const loadHistory = useCallback(async () => {
    setBusy(true)
    setPanelError(null)
    const [historyResult, snapshotResult, exportResult] = await Promise.all([
      getWorkProductHistory(matterId, complaint.complaint_id),
      getWorkProductSnapshots(matterId, complaint.complaint_id),
      getWorkProductExportHistory(matterId, complaint.complaint_id),
    ])
    setHistory(historyResult.data)
    setSnapshots(snapshotResult.data)
    setExports(exportResult.data)
    setSelectedSnapshotId((current) => current || snapshotResult.data.at(-1)?.snapshot_id || "")
    setPanelError(historyResult.error || snapshotResult.error || exportResult.error || null)
    setBusy(false)
  }, [matterId, complaint.complaint_id])

  useEffect(() => {
    void loadHistory()
  }, [loadHistory])

  const latestExport = exports.at(-1)
  const changedSinceExport = latestExport?.changed_since_export === true

  const makeSnapshot = async () => {
    setBusy(true)
    const result = await createWorkProductSnapshot(matterId, complaint.complaint_id, {
      title: "Manual Case History snapshot",
      message: "User-created complaint milestone snapshot.",
    })
    if (!result.data) setPanelError(result.error || "Snapshot could not be created.")
    await loadHistory()
  }

  const compareSelected = async () => {
    if (!selectedSnapshotId) return
    setBusy(true)
    setRestorePlan(null)
    const result = await compareWorkProductVersions(matterId, complaint.complaint_id, {
      from: selectedSnapshotId,
      layers: ["text"],
    })
    if (result.data) {
      setCompareResult(result.data)
      setPanelError(null)
    } else {
      setPanelError(result.error || "Versions could not be compared.")
    }
    setBusy(false)
  }

  const dryRunRestore = async () => {
    if (!selectedSnapshotId) return
    setBusy(true)
    const result = await restoreWorkProductVersion(matterId, complaint.complaint_id, {
      snapshot_id: selectedSnapshotId,
      scope: "complaint",
      dry_run: true,
    })
    if (result.data) {
      setRestorePlan({ snapshotId: selectedSnapshotId, warnings: result.data.warnings })
      setPanelError(null)
    } else {
      setPanelError(result.error || "Restore preview could not be prepared.")
    }
    setBusy(false)
  }

  const applyRestore = async () => {
    if (!restorePlan) return
    setBusy(true)
    const result = await restoreWorkProductVersion(matterId, complaint.complaint_id, {
      snapshot_id: restorePlan.snapshotId,
      scope: "complaint",
      dry_run: false,
    })
    if (!result.data) {
      setPanelError(result.error || "Version could not be restored.")
    } else {
      setRestorePlan(null)
      setCompareResult(null)
      onRefresh()
      await loadHistory()
    }
    setBusy(false)
  }

  return (
    <div className="space-y-4 p-4 md:p-6">
      <section className="rounded-md border border-border bg-card p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-foreground">Case History</h2>
            <p className="mt-1 text-xs text-muted-foreground">
              {history.length} changes · {snapshots.length} snapshots · {changedSinceExport ? "changed since export" : latestExport ? "export current" : "no export lock"}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button type="button" className="toolbar-button" disabled={busy} onClick={loadHistory}>
              <History className="h-3.5 w-3.5" />
              Refresh
            </button>
            <button type="button" className="toolbar-button" disabled={busy} onClick={makeSnapshot}>
              <Save className="h-3.5 w-3.5" />
              Snapshot
            </button>
          </div>
        </div>
        {panelError ? <p className="mt-3 rounded border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">{panelError}</p> : null}
      </section>

      <section className="rounded-md border border-border bg-card p-4">
        <div className="flex flex-wrap items-center gap-2">
          <select value={selectedSnapshotId} onChange={(event) => setSelectedSnapshotId(event.target.value)} className="input-like max-w-full flex-1">
            <option value="">Select snapshot</option>
            {snapshots.slice().reverse().map((snapshot) => (
              <option key={snapshot.snapshot_id} value={snapshot.snapshot_id}>
                #{snapshot.sequence_number} {snapshot.title}
              </option>
            ))}
          </select>
          <button type="button" className="toolbar-button" disabled={busy || !selectedSnapshotId} onClick={compareSelected}>
            <FileText className="h-3.5 w-3.5" />
            Compare
          </button>
          <button type="button" className="toolbar-button" disabled={busy || !selectedSnapshotId} onClick={dryRunRestore}>
            <CheckCircle2 className="h-3.5 w-3.5" />
            Restore
          </button>
        </div>
        {restorePlan ? (
          <div className="mt-3 rounded border border-warning/30 bg-warning/5 p-3">
            <div className="font-mono text-[10px] uppercase tracking-widest text-warning">restore preview</div>
            {restorePlan.warnings.length > 0 ? (
              <ul className="mt-2 space-y-1 text-xs text-foreground">
                {restorePlan.warnings.map((warning) => <li key={warning}>{warning}</li>)}
              </ul>
            ) : (
              <p className="mt-2 text-xs text-muted-foreground">No restore warnings returned.</p>
            )}
            <div className="mt-3 flex gap-2">
              <button type="button" className="toolbar-button" disabled={busy} onClick={applyRestore}>Apply restore</button>
              <button type="button" className="toolbar-button" disabled={busy} onClick={() => setRestorePlan(null)}>Cancel</button>
            </div>
          </div>
        ) : null}
      </section>

      {compareResult ? (
        <section className="rounded-md border border-border bg-card">
          <div className="border-b border-border px-4 py-3">
            <h2 className="text-sm font-semibold text-foreground">Compare</h2>
            <p className="mt-1 text-xs text-muted-foreground">{compareResult.summary.user_summary || `${compareResult.summary.text_changes} text changes`}</p>
          </div>
          <div className="divide-y divide-border">
            {compareResult.text_diffs.filter((diff) => diff.status !== "unchanged").length === 0 ? (
              <p className="p-4 text-sm text-muted-foreground">No text changes in this comparison.</p>
            ) : compareResult.text_diffs.filter((diff) => diff.status !== "unchanged").map((diff) => (
              <article key={`${diff.target_type}:${diff.target_id}`} className="grid gap-3 p-4 md:grid-cols-2">
                <div>
                  <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">before · {diff.status}</div>
                  <p className="mt-2 whitespace-pre-wrap text-sm text-foreground">{diff.before || "No prior text."}</p>
                </div>
                <div>
                  <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">after</div>
                  <p className="mt-2 whitespace-pre-wrap text-sm text-foreground">{diff.after || "Removed."}</p>
                </div>
              </article>
            ))}
          </div>
        </section>
      ) : null}

      <section className="rounded-md border border-border bg-card">
        <div className="border-b border-border px-4 py-3">
          <h2 className="text-sm font-semibold text-foreground">Timeline</h2>
        </div>
        <div className="divide-y divide-border">
          {history.length === 0 ? <p className="p-4 text-sm text-muted-foreground">No canonical history yet.</p> : history.slice().reverse().map((changeSet) => (
            <article key={changeSet.change_set_id} className="p-4">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{changeSet.source} · {formatCaseHistoryTime(changeSet.created_at)}</div>
                  <h3 className="mt-1 text-sm font-semibold text-foreground">{changeSet.title}</h3>
                  <p className="mt-1 text-xs text-muted-foreground">{changeSet.summary}</p>
                </div>
                <span className={cn("rounded px-2 py-1 text-[11px]", changeSet.legal_impact.blocking_issues_added.length > 0 ? "bg-destructive/10 text-destructive" : changeSet.legal_impact.qc_warnings_added.length > 0 ? "bg-warning/10 text-warning" : "bg-muted text-muted-foreground")}>
                  {changeSet.legal_impact.blocking_issues_added.length > 0 ? "blocking" : changeSet.legal_impact.qc_warnings_added.length > 0 ? "warning" : "tracked"}
                </span>
              </div>
              <div className="mt-2 flex flex-wrap gap-2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                {changeSet.legal_impact.affected_facts.length > 0 ? <span>{changeSet.legal_impact.affected_facts.length} facts</span> : null}
                {changeSet.legal_impact.affected_evidence.length > 0 ? <span>{changeSet.legal_impact.affected_evidence.length} evidence</span> : null}
                {changeSet.legal_impact.affected_authorities.length > 0 ? <span>{changeSet.legal_impact.affected_authorities.length} authorities</span> : null}
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  )
}

function ComplaintInspector({
  matter,
  complaint,
  selectedParagraph,
  tab,
  onTab,
  citationText,
  onCitationText,
  onLinkFact,
  onLinkEvidence,
  onInsertCitation,
  onResolve,
  onAiCommand,
}: {
  matter: Matter
  complaint: ComplaintDraft
  selectedParagraph: PleadingParagraph | null
  tab: WorkbenchTab
  onTab: (tab: WorkbenchTab) => void
  citationText: string
  onCitationText: (value: string) => void
  onLinkFact: (id: string) => void
  onLinkEvidence: (id: string) => void
  onInsertCitation: () => void
  onResolve: (id: string, status: "resolved" | "ignored" | "open") => void
  onAiCommand: (command: string) => void
}) {
  const tabs: Array<{ id: WorkbenchTab; label: string; icon: typeof Link2 }> = [
    { id: "support", label: "Support", icon: Link2 },
    { id: "authority", label: "Authority", icon: BookOpen },
    { id: "rules", label: "Rules", icon: ShieldCheck },
    { id: "format", label: "Format", icon: FileText },
    { id: "ai", label: "AI", icon: Sparkles },
    { id: "history", label: "History", icon: History },
  ]
  return (
    <aside className="min-h-0 overflow-y-auto border-l border-border bg-card">
      <div className="border-b border-border p-3">
        <div className="grid grid-cols-3 gap-1">
          {tabs.map((item) => {
            const Icon = item.icon
            return (
              <button key={item.id} type="button" onClick={() => onTab(item.id)} className={cn("flex items-center justify-center gap-1 rounded px-2 py-1.5 text-[11px]", tab === item.id ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground hover:text-foreground")}>
                <Icon className="h-3 w-3" />
                {item.label}
              </button>
            )
          })}
        </div>
      </div>
      <div className="space-y-4 p-4">
        {selectedParagraph && (
          <div className="rounded border border-border bg-background p-3">
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">Selected ¶ {selectedParagraph.number}</div>
            <p className="mt-1 line-clamp-4 text-sm text-foreground">{selectedParagraph.text}</p>
          </div>
        )}
        {tab === "support" && (
          <div className="space-y-3">
            {matter.facts.slice(0, 4).map((fact) => <button key={fact.id} type="button" onClick={() => onLinkFact(fact.id)} className="inspector-row">{fact.statement}</button>)}
            {matter.evidence.slice(0, 4).map((evidence) => <button key={evidence.evidence_id} type="button" onClick={() => onLinkEvidence(evidence.evidence_id)} className="inspector-row">{evidence.quote || evidence.evidence_id}</button>)}
          </div>
        )}
        {tab === "authority" && (
          <div>
            <input value={citationText} onChange={(event) => onCitationText(event.target.value)} className="input-like" placeholder="Citation" />
            <button type="button" onClick={onInsertCitation} className="mt-2 toolbar-button"><BookOpen className="h-3.5 w-3.5" />Insert citation</button>
          </div>
        )}
        {tab === "rules" && (
          <div className="space-y-2">
            {complaint.findings.filter((finding) => finding.status === "open").slice(0, 6).map((finding) => (
              <div key={finding.finding_id} className="rounded border border-border bg-background p-3">
                <div className="font-mono text-[10px] uppercase tracking-wider text-warning">{finding.severity}</div>
                <p className="mt-1 text-xs text-foreground">{finding.message}</p>
                <button type="button" onClick={() => onResolve(finding.finding_id, "resolved")} className="mt-2 toolbar-button">Resolve</button>
              </div>
            ))}
          </div>
        )}
        {tab === "format" && (
          <div className="space-y-2 text-xs text-muted-foreground">
            <p>{complaint.formatting_profile.name}</p>
            <p>Line numbers: {complaint.formatting_profile.line_numbers ? "on" : "off"}</p>
            <p>Double spaced: {complaint.formatting_profile.double_spaced ? "yes" : "no"}</p>
            <p>First-page blank: {complaint.formatting_profile.first_page_top_blank_inches} in</p>
          </div>
        )}
        {tab === "ai" && (
          <div className="space-y-2">
            {complaint.ai_commands.slice(0, 8).map((command) => (
              <button key={command.command_id} type="button" onClick={() => onAiCommand(command.command_id)} className="inspector-row">
                <Sparkles className="h-3.5 w-3.5 text-primary" />
                <span>{command.label}</span>
              </button>
            ))}
          </div>
        )}
        {tab === "history" && (
          <div className="space-y-2">
            {complaint.history.slice().reverse().slice(0, 8).map((event) => (
              <div key={event.event_id} className="rounded border border-border bg-background p-3">
                <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{event.event_type}</div>
                <p className="mt-1 text-xs text-foreground">{event.summary}</p>
              </div>
            ))}
          </div>
        )}
        <div className="rounded border border-border bg-background p-3">
          <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <Keyboard className="h-3.5 w-3.5" />
            commands
          </div>
          <div className="mt-2 grid grid-cols-2 gap-1">
            <Link href={matterComplaintHref(matter.id, "editor", { id: selectedParagraph?.paragraph_id })} className="toolbar-button">Editor</Link>
            <Link href={matterComplaintHref(matter.id, "qc", { id: selectedParagraph?.paragraph_id })} className="toolbar-button">QC</Link>
            {selectedParagraph?.fact_ids[0] && <Link href={matterFactsHref(matter.id, selectedParagraph.fact_ids[0])} className="toolbar-button">Fact</Link>}
            {matter.documents[0] && <Link href={matterDocumentHref(matter.id, matter.documents[0].id)} className="toolbar-button">Source</Link>}
          </div>
        </div>
      </div>
    </aside>
  )
}

function splitNames(value: string) {
  return value.split(",").map((part) => part.trim()).filter(Boolean)
}

function formatCaseHistoryTime(value: string) {
  if (!value) return "unknown time"
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString([], { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" })
}
