"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import {
  AlertTriangle,
  ArrowLeft,
  BookOpen,
  CalendarClock,
  CheckCircle2,
  ChevronRight,
  Clock,
  Download,
  FileText,
  GitBranch,
  History,
  Layers3,
  Link2,
  ListChecks,
  Microscope,
  Plus,
  Save,
  ShieldCheck,
  Sparkles,
} from "lucide-react"
import type {
  AIEditAudit,
  ChangeSet,
  Matter,
  VersionSnapshot,
  WorkProduct,
  WorkProductAnchor,
  WorkProductArtifact,
  WorkProductBlock,
  WorkProductFinding,
  WorkProductPreviewResponse,
} from "@/lib/casebuilder/types"
import {
  createWorkProductBlock,
  deleteWorkProductSupport,
  exportWorkProduct,
  getWorkProductAiAudit,
  getWorkProductExportHistory,
  getWorkProductHistory,
  getWorkProductSnapshots,
  linkWorkProductSupport,
  linkWorkProductTextRange,
  patchWorkProduct,
  patchWorkProductBlock,
  patchWorkProductSupport,
  previewWorkProduct,
  runWorkProductAiCommand,
  runWorkProductQc,
  suggestTimeline,
  validateWorkProductAst,
  workProductAstFromMarkdown,
  workProductAstToMarkdown,
} from "@/lib/casebuilder/api"
import {
  matterTimelineHref,
  matterWorkProductHref,
  matterWorkProductsHref,
  type WorkProductWorkspaceSection,
} from "@/lib/casebuilder/routes"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Textarea } from "@/components/ui/textarea"
import { cn } from "@/lib/utils"

interface WorkProductWorkbenchProps {
  matter: Matter
  workProduct: WorkProduct
  mode: WorkProductWorkspaceSection | "overview"
}

interface SelectedTextRange {
  blockId: string
  startOffset: number
  endOffset: number
  quote: string
}

const WORKSPACE_LINKS: { section: WorkProductWorkspaceSection | "overview"; label: string; icon: typeof FileText }[] = [
  { section: "overview", label: "Overview", icon: FileText },
  { section: "editor", label: "Editor", icon: ListChecks },
  { section: "qc", label: "QC", icon: ShieldCheck },
  { section: "preview", label: "Preview", icon: Layers3 },
  { section: "export", label: "Export", icon: Download },
  { section: "history", label: "History", icon: History },
]

const SUPPORT_RELATIONS = [
  "supports",
  "partially_supports",
  "contradicts",
  "context_only",
  "impeaches",
  "authenticates",
]

export function WorkProductWorkbench({ matter, workProduct: initialWorkProduct, mode }: WorkProductWorkbenchProps) {
  const router = useRouter()
  const [workProduct, setWorkProduct] = useState(initialWorkProduct)
  const [blockDrafts, setBlockDrafts] = useState<Record<string, string>>(() => blockDraftMap(initialWorkProduct.blocks))
  const [newBlockTitle, setNewBlockTitle] = useState("")
  const [newBlockText, setNewBlockText] = useState("")
  const [exportFormat, setExportFormat] = useState("html")
  const [preview, setPreview] = useState<WorkProductPreviewResponse | null>(null)
  const [markdown, setMarkdown] = useState<string | null>(null)
  const [historyItems, setHistoryItems] = useState<ChangeSet[]>([])
  const [snapshots, setSnapshots] = useState<VersionSnapshot[]>([])
  const [exports, setExports] = useState<WorkProductArtifact[]>(initialWorkProduct.artifacts)
  const [aiAudit, setAiAudit] = useState<AIEditAudit[]>([])
  const [markdownDraft, setMarkdownDraft] = useState("")
  const [selectedBlockId, setSelectedBlockId] = useState(initialWorkProduct.blocks[0]?.id ?? "")
  const [selectedTextRange, setSelectedTextRange] = useState<SelectedTextRange | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)

  useEffect(() => {
    setWorkProduct(initialWorkProduct)
    setBlockDrafts(blockDraftMap(initialWorkProduct.blocks))
    setExports(initialWorkProduct.artifacts)
    setSelectedBlockId((current) => current || initialWorkProduct.blocks[0]?.id || "")
    setSelectedTextRange(null)
  }, [initialWorkProduct])

  useEffect(() => {
    if (mode === "preview" && !preview && !pending) {
      void loadPreview()
    }
    if (mode === "history" && historyItems.length === 0 && !pending) {
      void loadHistory()
    }
    // The guarded calls avoid repeated fetches while still loading direct deep links.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode])

  const blocks = useMemo(
    () => [...workProduct.blocks].sort((a, b) => a.order_index - b.order_index || a.ordinal - b.ordinal),
    [workProduct.blocks],
  )
  const selectedBlock = useMemo(
    () => blocks.find((block) => block.id === selectedBlockId) ?? blocks[0] ?? null,
    [blocks, selectedBlockId],
  )

  const openFindings = useMemo(
    () => workProduct.findings.filter((finding) => finding.status === "open"),
    [workProduct.findings],
  )
  const selectedWarnings = useMemo(
    () =>
      [
        ...workProduct.document_ast.citations
          .filter((citation) => citation.status !== "resolved")
          .map((citation) => `Citation needs review: ${citation.raw_text}`),
        ...workProduct.document_ast.exhibits
          .filter((exhibit) => exhibit.status !== "attached")
          .map((exhibit) => `Exhibit needs review: ${exhibit.label}`),
      ].slice(0, 4),
    [workProduct.document_ast.citations, workProduct.document_ast.exhibits],
  )

  const updateSelectedTextRange = useCallback((nextRange: SelectedTextRange | null) => {
    setSelectedTextRange((currentRange) =>
      sameSelectedTextRange(currentRange, nextRange) ? currentRange : nextRange,
    )
  }, [])

  async function runAction(action: () => Promise<string | null>) {
    setPending(true)
    setError(null)
    setMessage(null)
    const problem = await action()
    setPending(false)
    if (problem) setError(problem)
  }

  async function saveBlock(block: WorkProductBlock) {
    await runAction(async () => {
      const result = await patchWorkProductBlock(matter.id, workProduct.id, block.id, {
        block_type: block.block_type || block.type,
        role: block.role,
        title: block.title,
        text: blockDrafts[block.id] ?? block.text,
        locked: block.locked,
        review_status: block.review_status,
      })
      if (!result.data) return result.error || "Block could not be saved."
      setWorkProduct(result.data)
      setBlockDrafts(blockDraftMap(result.data.blocks))
      setMessage(`${block.title || "Block"} saved.`)
      router.refresh()
      return null
    })
  }

  async function addBlock() {
    await runAction(async () => {
      const text = newBlockText.trim()
      if (!text) return "Add text for the new block."
      const result = await createWorkProductBlock(matter.id, workProduct.id, {
        block_type: "section",
        role: "analysis",
        title: newBlockTitle.trim() || "New section",
        text,
      })
      if (!result.data) return result.error || "Block could not be added."
      setWorkProduct(result.data)
      setBlockDrafts(blockDraftMap(result.data.blocks))
      setNewBlockTitle("")
      setNewBlockText("")
      setMessage("Block added.")
      router.refresh()
      return null
    })
  }

  async function runQc() {
    await runAction(async () => {
      const validation = await validateWorkProductAst(matter.id, workProduct.id)
      if (!validation.data) return validation.error || "AST validation could not run."
      const result = await runWorkProductQc(matter.id, workProduct.id)
      if (!result.data) return result.error || "QC could not run."
      if (result.data.result) {
        setWorkProduct((current) => ({
          ...current,
          findings: result.data?.result ?? current.findings,
          document_ast: {
            ...current.document_ast,
            rule_findings: result.data?.result ?? current.document_ast.rule_findings,
          },
        }))
      }
      const errorCount = validation.data.errors.length
      const warningCount = validation.data.warnings.length
      setMessage(`${result.data.message} AST validation found ${errorCount} errors and ${warningCount} warnings.`)
      router.refresh()
      return null
    })
  }

  async function loadPreview() {
    await runAction(async () => {
      const [previewResult, markdownResult] = await Promise.all([
        previewWorkProduct(matter.id, workProduct.id),
        workProductAstToMarkdown(matter.id, workProduct.id),
      ])
      if (!previewResult.data) return previewResult.error || "Preview could not be generated."
      setPreview(previewResult.data)
      setMarkdown(markdownResult.data?.markdown ?? null)
      setMarkdownDraft(markdownResult.data?.markdown ?? "")
      setMessage(previewResult.data.review_label)
      return null
    })
  }

  async function loadMarkdownForEdit() {
    await runAction(async () => {
      const result = await workProductAstToMarkdown(matter.id, workProduct.id)
      if (!result.data) return result.error || "Markdown projection could not be loaded."
      setMarkdown(result.data.markdown)
      setMarkdownDraft(result.data.markdown)
      setMessage(result.data.warnings.length ? result.data.warnings[0] : "Markdown projection loaded.")
      return null
    })
  }

  async function applyMarkdownDraft() {
    await runAction(async () => {
      if (!markdownDraft.trim()) return "Load or enter Markdown before applying it."
      const converted = await workProductAstFromMarkdown(matter.id, workProduct.id, { markdown: markdownDraft })
      if (!converted.data) return converted.error || "Markdown could not be converted to AST."
      const saved = await patchWorkProduct(matter.id, workProduct.id, {
        document_ast: converted.data.document_ast,
      })
      if (!saved.data) return saved.error || "Converted AST could not be saved."
      setWorkProduct(saved.data)
      setBlockDrafts(blockDraftMap(saved.data.blocks))
      setMarkdown(markdownDraft)
      setMessage(
        converted.data.warnings.length
          ? `Saved with warning: ${converted.data.warnings[0]}`
          : "Markdown applied to the canonical AST.",
      )
      router.refresh()
      return null
    })
  }

  async function createExport() {
    await runAction(async () => {
      const result = await exportWorkProduct(matter.id, workProduct.id, {
        format: exportFormat,
        profile: "review",
        mode: "review",
        include_qc_report: true,
      })
      if (!result.data) return result.error || "Export could not be created."
      setExports((current) => [result.data!, ...current.filter((artifact) => artifact.id !== result.data!.id)])
      setMessage(`${result.data.format.toUpperCase()} export created for review.`)
      router.refresh()
      return null
    })
  }

  async function loadHistory() {
    await runAction(async () => {
      const [historyResult, snapshotResult, exportResult, auditResult] = await Promise.all([
        getWorkProductHistory(matter.id, workProduct.id),
        getWorkProductSnapshots(matter.id, workProduct.id),
        getWorkProductExportHistory(matter.id, workProduct.id),
        getWorkProductAiAudit(matter.id, workProduct.id),
      ])
      setHistoryItems(historyResult.data)
      setSnapshots(snapshotResult.data)
      setExports(exportResult.data)
      setAiAudit(auditResult.data)
      const firstError = historyResult.error || snapshotResult.error || exportResult.error || auditResult.error
      if (firstError) return firstError
      setMessage("History loaded.")
      return null
    })
  }

  async function runTemplateCommand(command: string, targetId?: string) {
    await runAction(async () => {
      const result = await runWorkProductAiCommand(matter.id, workProduct.id, {
        command,
        target_id: targetId,
      })
      if (!result.data) return result.error || "Command could not run."
      if (result.data.result) setWorkProduct(result.data.result)
      setMessage(result.data.message)
      router.refresh()
      return null
    })
  }

  async function suggestWorkProductTimeline() {
    await runAction(async () => {
      const result = await suggestTimeline(matter.id, {
        work_product_id: workProduct.id,
        block_id: selectedBlock?.id,
        limit: 50,
      })
      if (!result.data) return result.error || "Timeline suggestions could not be generated."
      const first = result.data.suggestions[0]
      const providerMode = result.data.agent_run?.provider_mode ?? result.data.mode
      setMessage(`${result.data.suggestions.length} timeline suggestion${result.data.suggestions.length === 1 ? "" : "s"} ready for review (${providerMode}).`)
      router.push(
        matterTimelineHref(matter.id, {
          suggestionId: first?.suggestion_id,
          status: first ? "suggested" : undefined,
          sourceType: first?.source_type,
          agentRunId: first?.agent_run_id ?? result.data.agent_run?.agent_run_id,
        }),
      )
      return null
    })
  }

  async function linkSelectedBlockSupport(input: {
    targetType: string
    targetId: string
    relation?: string
    citation?: string
    canonicalId?: string
    quote?: string
  }) {
    await runAction(async () => {
      if (!selectedBlock) return "Select a work product block first."
      if (!input.targetId.trim()) return "Choose a support target first."
      const blockId = selectedBlock.block_id || selectedBlock.id
      const result = await linkWorkProductSupport(matter.id, workProduct.id, {
        block_id: blockId,
        anchor_type: input.targetType === "authority" ? "authority" : input.targetType,
        relation: input.relation || "supports",
        target_type: input.targetType,
        target_id: input.targetId,
        citation: input.citation,
        canonical_id: input.canonicalId,
        quote: input.quote,
      })
      if (!result.data) return result.error || "Support could not be linked."
      setWorkProduct(result.data)
      setBlockDrafts(blockDraftMap(result.data.blocks))
      setMessage(`Linked ${input.targetType.replace(/_/g, " ")} to ${selectedBlock.title || "selected block"}.`)
      router.refresh()
      return null
    })
  }

  async function linkSelectedTextRange(input: {
    targetType: string
    targetId: string
    relation?: string
    citation?: string
    canonicalId?: string
    exhibitLabel?: string
    documentId?: string
    pageRange?: string
  }) {
    await runAction(async () => {
      if (!selectedTextRange) return "Select text in a work product block first."
      if (!input.targetId.trim()) return "Choose a range target first."
      const result = await linkWorkProductTextRange(matter.id, workProduct.id, {
        block_id: selectedTextRange.blockId,
        start_offset: selectedTextRange.startOffset,
        end_offset: selectedTextRange.endOffset,
        quote: selectedTextRange.quote,
        target_type: input.targetType,
        target_id: input.targetId,
        relation: input.relation || "supports",
        citation: input.citation,
        canonical_id: input.canonicalId,
        exhibit_label: input.exhibitLabel,
        document_id: input.documentId,
        page_range: input.pageRange,
      })
      if (!result.data) return result.error || "Selected text could not be linked."
      setWorkProduct(result.data)
      setBlockDrafts(blockDraftMap(result.data.blocks))
      setSelectedTextRange(null)
      setMessage("Selected text linked.")
      router.refresh()
      return null
    })
  }

  async function updateSupportRelation(anchorId: string, relation: string) {
    await runAction(async () => {
      const result = await patchWorkProductSupport(matter.id, workProduct.id, anchorId, { relation })
      if (!result.data) return result.error || "Support relation could not be updated."
      setWorkProduct(result.data)
      setBlockDrafts(blockDraftMap(result.data.blocks))
      setMessage("Support relation updated.")
      router.refresh()
      return null
    })
  }

  async function removeSupportLink(anchor: WorkProductAnchor) {
    await runAction(async () => {
      const anchorId = anchor.anchor_id || anchor.id
      const result = await deleteWorkProductSupport(matter.id, workProduct.id, anchorId)
      if (!result.data) return result.error || "Support link could not be removed."
      setWorkProduct(result.data)
      setBlockDrafts(blockDraftMap(result.data.blocks))
      setMessage("Support link removed.")
      router.refresh()
      return null
    })
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="border-b border-border bg-card px-6 py-4">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Link href={matterWorkProductsHref(matter.id)} className="flex items-center gap-1 hover:text-foreground">
            <ArrowLeft className="h-3.5 w-3.5" />
            Work product
          </Link>
          <ChevronRight className="h-3 w-3" />
          <span className="truncate text-foreground">{workProduct.title}</span>
        </div>

        <div className="mt-3 flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="truncate text-lg font-semibold text-foreground">{workProduct.title}</h1>
              <Badge variant="outline" className="text-[10px] capitalize">
                {workProduct.product_type.replace(/_/g, " ")}
              </Badge>
              <StatusPill status={workProduct.status} />
            </div>
            <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
              <span>{blocks.length} blocks</span>
              <span>·</span>
              <span>{openFindings.length} open findings</span>
              <span>·</span>
              <span>{exports.length} exports</span>
              <span>·</span>
              <span className="font-mono tabular-nums">{workProduct.updated_at || workProduct.created_at}</span>
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" className="gap-1.5 bg-transparent" disabled={pending} onClick={() => runTemplateCommand("summarize_support")}>
              <Sparkles className="h-3.5 w-3.5" />
              Assist
            </Button>
            <Button variant="outline" size="sm" className="gap-1.5 bg-transparent" disabled={pending} onClick={suggestWorkProductTimeline}>
              <CalendarClock className="h-3.5 w-3.5" />
              Timeline
            </Button>
            <Button size="sm" className="gap-1.5" disabled={pending} onClick={runQc}>
              <ShieldCheck className="h-3.5 w-3.5" />
              Run QC
            </Button>
          </div>
        </div>

        <nav aria-label="Work product navigation" className="mt-4 flex gap-1 overflow-x-auto">
          {WORKSPACE_LINKS.map((item) => {
            const Icon = item.icon
            const active = item.section === mode
            const href =
              item.section === "overview"
                ? matterWorkProductHref(matter.id, workProduct.id)
                : matterWorkProductHref(matter.id, workProduct.id, item.section)
            return (
              <Link
                key={item.section}
                href={href}
                aria-current={active ? "page" : undefined}
                className={cn(
                  "inline-flex h-8 items-center gap-1.5 rounded-md px-3 text-xs transition-colors",
                  active ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                <Icon className="h-3.5 w-3.5" />
                {item.label}
              </Link>
            )
          })}
        </nav>

        {(message || error) && (
          <div className={cn("mt-3 rounded-md border px-3 py-2 text-xs", error ? "border-destructive/30 bg-destructive/10 text-destructive" : "border-success/20 bg-success/10 text-success")}>
            {error || message}
          </div>
        )}
      </header>

      <ScrollArea className="flex-1">
        <main className="grid min-h-full gap-0 xl:grid-cols-[260px_minmax(0,1fr)_340px]">
          <aside className="border-b border-border bg-background p-4 xl:border-b-0 xl:border-r">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Outline</h2>
            <div className="mt-3 space-y-1">
              {blocks.map((block) => (
                <a
                  key={block.id}
                  href={`#${encodeURIComponent(block.id)}`}
                  onClick={() => setSelectedBlockId(block.id)}
                  className={cn(
                    "block rounded-md px-2 py-1.5 text-xs hover:bg-muted hover:text-foreground",
                    selectedBlock?.id === block.id ? "bg-primary/10 text-primary" : "text-muted-foreground",
                  )}
                >
                  <span className="font-mono text-[10px] tabular-nums">{block.ordinal || block.order_index + 1}</span>
                  <span className="ml-2">{block.title || block.role}</span>
                </a>
              ))}
            </div>
          </aside>

          <section className="min-w-0 p-4 lg:p-6">
            {mode === "overview" && (
              <OverviewPanel
                workProduct={workProduct}
                warnings={selectedWarnings}
                onPreview={loadPreview}
                pending={pending}
              />
            )}
            {mode === "editor" && (
              <EditorPanel
                blocks={blocks}
                drafts={blockDrafts}
                selectedBlockId={selectedBlock?.id ?? ""}
                onSelectBlock={setSelectedBlockId}
                onTextRangeSelect={updateSelectedTextRange}
                pending={pending}
                onChange={(blockId, text) => setBlockDrafts((current) => ({ ...current, [blockId]: text }))}
                onSave={saveBlock}
                newBlockTitle={newBlockTitle}
                newBlockText={newBlockText}
                onNewBlockTitle={setNewBlockTitle}
                onNewBlockText={setNewBlockText}
                onAddBlock={addBlock}
                markdownDraft={markdownDraft}
                onMarkdownDraft={setMarkdownDraft}
                onLoadMarkdown={loadMarkdownForEdit}
                onApplyMarkdown={applyMarkdownDraft}
              />
            )}
            {mode === "qc" && (
              <QcPanel findings={workProduct.findings} warnings={selectedWarnings} pending={pending} onRun={runQc} />
            )}
            {mode === "preview" && (
              <PreviewPanel preview={preview} markdown={markdown} pending={pending} onLoad={loadPreview} />
            )}
            {mode === "export" && (
              <ExportPanel
                format={exportFormat}
                onFormat={setExportFormat}
                exports={exports}
                pending={pending}
                onExport={createExport}
              />
            )}
            {mode === "history" && (
              <HistoryPanel
                historyItems={historyItems}
                snapshots={snapshots}
                exports={exports}
                aiAudit={aiAudit}
                pending={pending}
                onLoad={loadHistory}
              />
            )}
          </section>

          <aside className="border-t border-border bg-card p-4 xl:border-l xl:border-t-0">
            <Inspector
              workProduct={workProduct}
              matter={matter}
              selectedBlock={selectedBlock}
              selectedTextRange={selectedTextRange}
              openFindings={openFindings}
              onLinkSupport={linkSelectedBlockSupport}
              onLinkTextRange={linkSelectedTextRange}
              onUpdateSupportRelation={updateSupportRelation}
              onRemoveSupport={removeSupportLink}
              onRunCommand={runTemplateCommand}
              pending={pending}
            />
          </aside>
        </main>
      </ScrollArea>
    </div>
  )
}

function OverviewPanel({
  workProduct,
  warnings,
  onPreview,
  pending,
}: {
  workProduct: WorkProduct
  warnings: string[]
  onPreview: () => void
  pending: boolean
}) {
  return (
    <div className="space-y-4">
      <Card className="p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-foreground">Document health</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Current AST state, support links, citations, exhibits, rule findings, and export readiness.
            </p>
          </div>
          <Button variant="outline" size="sm" className="gap-1.5 bg-transparent" disabled={pending} onClick={onPreview}>
            <Layers3 className="h-3.5 w-3.5" />
            Preview
          </Button>
        </div>
        <div className="mt-4 grid gap-2 sm:grid-cols-4">
          <Metric label="Blocks" value={workProduct.blocks.length} />
          <Metric label="Links" value={workProduct.document_ast.links.length} />
          <Metric label="Citations" value={workProduct.document_ast.citations.length} />
          <Metric label="Findings" value={workProduct.findings.filter((finding) => finding.status === "open").length} warn />
        </div>
      </Card>

      {warnings.length > 0 && (
        <Card className="border-warning/30 p-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-warning">
            <AlertTriangle className="h-4 w-4" />
            Review needed
          </div>
          <ul className="mt-3 space-y-2">
            {warnings.map((warning) => (
              <li key={warning} className="rounded-md bg-warning/10 px-3 py-2 text-xs text-warning">
                {warning}
              </li>
            ))}
          </ul>
        </Card>
      )}
    </div>
  )
}

function EditorPanel({
  blocks,
  drafts,
  selectedBlockId,
  onSelectBlock,
  onTextRangeSelect,
  pending,
  onChange,
  onSave,
  newBlockTitle,
  newBlockText,
  onNewBlockTitle,
  onNewBlockText,
  onAddBlock,
  markdownDraft,
  onMarkdownDraft,
  onLoadMarkdown,
  onApplyMarkdown,
}: {
  blocks: WorkProductBlock[]
  drafts: Record<string, string>
  selectedBlockId: string
  onSelectBlock: (blockId: string) => void
  onTextRangeSelect: (range: SelectedTextRange | null) => void
  pending: boolean
  onChange: (blockId: string, text: string) => void
  onSave: (block: WorkProductBlock) => void
  newBlockTitle: string
  newBlockText: string
  onNewBlockTitle: (value: string) => void
  onNewBlockText: (value: string) => void
  onAddBlock: () => void
  markdownDraft: string
  onMarkdownDraft: (value: string) => void
  onLoadMarkdown: () => void
  onApplyMarkdown: () => void
}) {
  return (
    <div className="space-y-4">
      {blocks.map((block) => (
        <Card
          key={block.id}
          id={block.id}
          className={cn("p-4", selectedBlockId === block.id && "border-primary/50 ring-1 ring-primary/20")}
        >
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="min-w-0">
              <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                {block.role || block.block_type}
              </div>
              <h2 className="mt-1 text-sm font-semibold text-foreground">{block.title || "Untitled block"}</h2>
            </div>
            <Button size="sm" className="gap-1.5" disabled={pending || drafts[block.id] === block.text} onClick={() => onSave(block)}>
              <Save className="h-3.5 w-3.5" />
              Save
            </Button>
          </div>
          <Textarea
            value={drafts[block.id] ?? block.text}
            onChange={(event) => onChange(block.id, event.target.value)}
            onFocus={() => onSelectBlock(block.id)}
            onSelect={(event) => captureTextRangeSelection(block, event.currentTarget, onTextRangeSelect)}
            onKeyUp={(event) => captureTextRangeSelection(block, event.currentTarget, onTextRangeSelect)}
            onMouseUp={(event) => captureTextRangeSelection(block, event.currentTarget, onTextRangeSelect)}
            className="mt-3 min-h-32 resize-y text-sm leading-relaxed"
          />
        </Card>
      ))}

      <Card className="p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              markdown mode
            </div>
            <h2 className="mt-1 text-sm font-semibold text-foreground">AST Markdown round trip</h2>
            <p className="mt-1 text-xs text-muted-foreground">
              Markdown edits convert back into the canonical WorkProduct AST and keep review-needed state.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" className="gap-1.5 bg-transparent" disabled={pending} onClick={onLoadMarkdown}>
              <FileText className="h-3.5 w-3.5" />
              Load
            </Button>
            <Button size="sm" className="gap-1.5" disabled={pending || !markdownDraft.trim()} onClick={onApplyMarkdown}>
              <Save className="h-3.5 w-3.5" />
              Apply
            </Button>
          </div>
        </div>
        <Textarea
          value={markdownDraft}
          onChange={(event) => onMarkdownDraft(event.target.value)}
          placeholder="Load the current AST as Markdown or paste a Markdown draft."
          className="mt-3 min-h-56 font-mono text-xs leading-relaxed"
        />
      </Card>

      <Card className="border-dashed p-4">
        <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
          <Plus className="h-4 w-4 text-muted-foreground" />
          Add block
        </div>
        <input
          value={newBlockTitle}
          onChange={(event) => onNewBlockTitle(event.target.value)}
          placeholder="Block title"
          className="mt-3 w-full rounded-md border border-border bg-background px-3 py-2 text-xs focus:border-primary focus:outline-none"
        />
        <Textarea
          value={newBlockText}
          onChange={(event) => onNewBlockText(event.target.value)}
          placeholder="Block text"
          className="mt-2 min-h-24 text-sm"
        />
        <Button className="mt-3 gap-1.5" size="sm" disabled={pending} onClick={onAddBlock}>
          <Plus className="h-3.5 w-3.5" />
          Add
        </Button>
      </Card>
    </div>
  )
}

function QcPanel({
  findings,
  warnings,
  pending,
  onRun,
}: {
  findings: WorkProductFinding[]
  warnings: string[]
  pending: boolean
  onRun: () => void
}) {
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-foreground">QC findings</h2>
          <p className="mt-1 text-sm text-muted-foreground">AST validation and rule findings target stable document nodes.</p>
        </div>
        <Button size="sm" className="gap-1.5" disabled={pending} onClick={onRun}>
          <ShieldCheck className="h-3.5 w-3.5" />
          Run QC
        </Button>
      </div>
      {[...warnings.map((warning) => ({ id: warning, message: warning, severity: "warning", status: "open", suggested_fix: "Review before export." } as WorkProductFinding)), ...findings].map((finding) => (
        <Card key={finding.id || finding.finding_id} className="p-4">
          <div className="flex items-start justify-between gap-3">
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="outline" className="text-[10px] capitalize">
                  {finding.severity}
                </Badge>
                <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">{finding.status}</span>
              </div>
              <h3 className="mt-2 text-sm font-semibold text-foreground">{finding.message}</h3>
              <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{finding.suggested_fix || finding.explanation}</p>
            </div>
          </div>
        </Card>
      ))}
      {findings.length === 0 && warnings.length === 0 && (
        <Card className="p-4 text-sm text-muted-foreground">No open findings on the current payload.</Card>
      )}
    </div>
  )
}

function PreviewPanel({
  preview,
  markdown,
  pending,
  onLoad,
}: {
  preview: WorkProductPreviewResponse | null
  markdown: string | null
  pending: boolean
  onLoad: () => void
}) {
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-foreground">Preview</h2>
          <p className="mt-1 text-sm text-muted-foreground">Rendered from the canonical WorkProduct AST.</p>
        </div>
        <Button variant="outline" size="sm" className="gap-1.5 bg-transparent" disabled={pending} onClick={onLoad}>
          <Layers3 className="h-3.5 w-3.5" />
          Refresh
        </Button>
      </div>
      {preview ? (
        <Card className="overflow-hidden">
          <div className="border-b border-border bg-muted/30 px-4 py-2 text-[11px] text-muted-foreground">
            {preview.page_count} pages · {preview.review_label}
          </div>
          <div className="bg-background p-5 text-sm leading-relaxed text-foreground" dangerouslySetInnerHTML={{ __html: preview.html }} />
        </Card>
      ) : (
        <Card className="p-4 text-sm text-muted-foreground">Preview has not been generated yet.</Card>
      )}
      {markdown && (
        <Card className="p-4">
          <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Markdown projection</h3>
          <pre className="mt-3 max-h-72 overflow-auto whitespace-pre-wrap rounded-md bg-muted p-3 text-xs text-foreground">{markdown}</pre>
        </Card>
      )}
    </div>
  )
}

function ExportPanel({
  format,
  onFormat,
  exports,
  pending,
  onExport,
}: {
  format: string
  onFormat: (value: string) => void
  exports: WorkProductArtifact[]
  pending: boolean
  onExport: () => void
}) {
  return (
    <div className="space-y-4">
      <Card className="p-4">
        <div className="flex flex-wrap items-end gap-3">
          <div className="min-w-48">
            <label className="text-xs font-medium text-muted-foreground">Format</label>
            <select
              value={format}
              onChange={(event) => onFormat(event.target.value)}
              className="mt-1 w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-xs"
            >
              {["html", "markdown", "plain_text", "json", "pdf", "docx"].map((item) => (
                <option key={item} value={item}>
                  {item}
                </option>
              ))}
            </select>
          </div>
          <Button size="sm" className="gap-1.5" disabled={pending} onClick={onExport}>
            <Download className="h-3.5 w-3.5" />
            Export
          </Button>
        </div>
      </Card>
      <ul className="space-y-2">
        {exports.map((artifact) => (
          <li key={artifact.id}>
            <Card className="p-4">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline" className="text-[10px] uppercase">
                      {artifact.format}
                    </Badge>
                    <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                      {artifact.status}
                    </span>
                  </div>
                  <p className="mt-2 text-sm text-foreground">{artifact.profile} · {artifact.mode}</p>
                  {artifact.warnings.length > 0 && (
                    <p className="mt-1 text-xs text-warning">{artifact.warnings[0]}</p>
                  )}
                </div>
                {artifact.download_url && (
                  <Button asChild variant="outline" size="sm" className="bg-transparent">
                    <a href={artifact.download_url}>Download</a>
                  </Button>
                )}
              </div>
            </Card>
          </li>
        ))}
      </ul>
      {exports.length === 0 && <Card className="p-4 text-sm text-muted-foreground">No exports yet.</Card>}
    </div>
  )
}

function HistoryPanel({
  historyItems,
  snapshots,
  exports,
  aiAudit,
  pending,
  onLoad,
}: {
  historyItems: ChangeSet[]
  snapshots: VersionSnapshot[]
  exports: WorkProductArtifact[]
  aiAudit: AIEditAudit[]
  pending: boolean
  onLoad: () => void
}) {
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-foreground">Case History</h2>
          <p className="mt-1 text-sm text-muted-foreground">Version changes, snapshots, exports, and AI audit records.</p>
        </div>
        <Button variant="outline" size="sm" className="gap-1.5 bg-transparent" disabled={pending} onClick={onLoad}>
          <History className="h-3.5 w-3.5" />
          Refresh
        </Button>
      </div>
      <div className="grid gap-2 sm:grid-cols-4">
        <Metric label="Changes" value={historyItems.length} />
        <Metric label="Snapshots" value={snapshots.length} />
        <Metric label="Exports" value={exports.length} />
        <Metric label="AI audit" value={aiAudit.length} />
      </div>
      <ul className="space-y-2">
        {historyItems.map((item) => (
          <li key={item.id}>
            <Card className="p-4">
              <div className="flex items-start gap-3">
                <GitBranch className="mt-0.5 h-4 w-4 text-muted-foreground" />
                <div className="min-w-0">
                  <h3 className="text-sm font-semibold text-foreground">{item.title}</h3>
                  <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{item.summary}</p>
                  <div className="mt-2 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                    {item.source} · {item.created_at}
                  </div>
                </div>
              </div>
            </Card>
          </li>
        ))}
      </ul>
      {historyItems.length === 0 && <Card className="p-4 text-sm text-muted-foreground">No history loaded yet.</Card>}
    </div>
  )
}

function Inspector({
  workProduct,
  matter,
  selectedBlock,
  selectedTextRange,
  openFindings,
  onLinkSupport,
  onLinkTextRange,
  onUpdateSupportRelation,
  onRemoveSupport,
  onRunCommand,
  pending,
}: {
  workProduct: WorkProduct
  matter: Matter
  selectedBlock: WorkProductBlock | null
  selectedTextRange: SelectedTextRange | null
  openFindings: WorkProductFinding[]
  onLinkSupport: (input: {
    targetType: string
    targetId: string
    relation?: string
    citation?: string
    canonicalId?: string
    quote?: string
  }) => void
  onLinkTextRange: (input: {
    targetType: string
    targetId: string
    relation?: string
    citation?: string
    canonicalId?: string
    exhibitLabel?: string
    documentId?: string
    pageRange?: string
  }) => void
  onUpdateSupportRelation: (anchorId: string, relation: string) => void
  onRemoveSupport: (anchor: WorkProductAnchor) => void
  onRunCommand: (command: string, targetId?: string) => void
  pending: boolean
}) {
  const [relation, setRelation] = useState("supports")
  const [factId, setFactId] = useState(matter.facts[0]?.id ?? "")
  const [evidenceId, setEvidenceId] = useState(matter.evidence[0]?.evidence_id ?? "")
  const [documentId, setDocumentId] = useState(matter.documents[0]?.id ?? "")
  const [authorityText, setAuthorityText] = useState("")
  const selectedBlockKey = selectedBlock?.block_id || selectedBlock?.id || ""
  const selectedAnchors = useMemo(
    () =>
      selectedBlock
        ? workProduct.anchors.filter(
            (anchor) => anchor.block_id === selectedBlock.id || anchor.block_id === selectedBlock.block_id,
          )
        : [],
    [selectedBlock, workProduct.anchors],
  )
  const selectedRangeLinks = useMemo(
    () =>
      workProduct.document_ast.links.filter(
        (link) => link.source_block_id === selectedBlockKey && link.source_text_range,
      ),
    [selectedBlockKey, workProduct.document_ast.links],
  )
  const selectedRangeCitations = useMemo(
    () =>
      workProduct.document_ast.citations.filter(
        (citation) => citation.source_block_id === selectedBlockKey && citation.source_text_range,
      ),
    [selectedBlockKey, workProduct.document_ast.citations],
  )
  const selectedRangeExhibits = useMemo(
    () =>
      workProduct.document_ast.exhibits.filter(
        (exhibit) => exhibit.source_block_id === selectedBlockKey && exhibit.source_text_range,
      ),
    [selectedBlockKey, workProduct.document_ast.exhibits],
  )
  const selectedDocument = useMemo(
    () => matter.documents.find((document) => document.id === documentId || document.document_id === documentId),
    [documentId, matter.documents],
  )
  const astMetrics = useMemo(
    () => ({
      facts: workProduct.document_ast.links.filter((link) => link.target_type === "fact").length,
      evidence: workProduct.document_ast.links.filter((link) => link.target_type === "evidence").length,
      cites: workProduct.document_ast.citations.length,
      exhibits: workProduct.document_ast.exhibits.length,
    }),
    [workProduct.document_ast.citations.length, workProduct.document_ast.exhibits.length, workProduct.document_ast.links],
  )
  const factOptions = useMemo(
    () => matter.facts.map((fact) => ({ id: fact.id, label: fact.statement })),
    [matter.facts],
  )
  const evidenceOptions = useMemo(
    () =>
      matter.evidence.map((evidence) => ({
        id: evidence.evidence_id,
        label: evidence.quote || evidence.evidence_type || evidence.document_id,
      })),
    [matter.evidence],
  )
  const documentOptions = useMemo(
    () =>
      matter.documents.map((document) => ({
        id: document.id,
        label: document.exhibit_label
          ? `${document.exhibit_label}: ${document.title || document.filename}`
          : document.title || document.filename,
      })),
    [matter.documents],
  )
  const hasSelectedTextLinks =
    selectedRangeLinks.length + selectedRangeCitations.length + selectedRangeExhibits.length > 0

  return (
    <div className="space-y-4">
      <section>
        <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Inspector</h2>
        <div className="mt-3 rounded-md border border-border bg-background p-3">
          <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
            selected block
          </div>
          <div className="mt-1 text-sm font-medium text-foreground">
            {selectedBlock?.title || selectedBlock?.role || "No block selected"}
          </div>
          {selectedBlock && (
            <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">
              {selectedBlock.text || "Empty block."}
            </p>
          )}
        </div>
        <div className="mt-3 grid grid-cols-2 gap-2">
          <Metric label="Facts" value={astMetrics.facts} />
          <Metric label="Evidence" value={astMetrics.evidence} />
          <Metric label="Cites" value={astMetrics.cites} />
          <Metric label="Exhibits" value={astMetrics.exhibits} />
        </div>
      </section>

      <section>
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Support links</h3>
        <div className="mt-2 space-y-2">
          <select
            value={relation}
            onChange={(event) => setRelation(event.target.value)}
            className="h-8 w-full rounded-md border border-border bg-background px-2 font-mono text-xs"
          >
            {SUPPORT_RELATIONS.map((value) => (
              <option key={value} value={value}>
                {value.replace(/_/g, " ")}
              </option>
            ))}
          </select>

          <div className="rounded-md border border-border bg-background p-2">
            <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              selected text range
            </div>
            {selectedTextRange ? (
              <>
                <p className="mt-1 line-clamp-3 text-[11px] leading-relaxed text-foreground">
                  {selectedTextRange.quote}
                </p>
                <div className="mt-2 grid grid-cols-2 gap-1.5">
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 justify-center gap-1.5 bg-transparent text-[11px]"
                    disabled={pending || !factId}
                    onClick={() => onLinkTextRange({ targetType: "fact", targetId: factId, relation })}
                  >
                    <Link2 className="h-3 w-3" />
                    Fact
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 justify-center gap-1.5 bg-transparent text-[11px]"
                    disabled={pending || !evidenceId}
                    onClick={() => onLinkTextRange({ targetType: "evidence", targetId: evidenceId, relation })}
                  >
                    <Link2 className="h-3 w-3" />
                    Evidence
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 justify-center gap-1.5 bg-transparent text-[11px]"
                    disabled={pending || !documentId}
                    onClick={() =>
                      onLinkTextRange({
                        targetType: "document",
                        targetId: documentId,
                        relation,
                        exhibitLabel: selectedDocument?.exhibit_label || "Exhibit",
                        documentId,
                      })
                    }
                  >
                    <Link2 className="h-3 w-3" />
                    Exhibit
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 justify-center gap-1.5 bg-transparent text-[11px]"
                    disabled={pending || !authorityText.trim()}
                    onClick={() => {
                      const citation = authorityText.trim()
                      onLinkTextRange({
                        targetType: "authority",
                        targetId: citation,
                        citation,
                        canonicalId: citation,
                        relation: "cites",
                      })
                      setAuthorityText("")
                    }}
                  >
                    <BookOpen className="h-3 w-3" />
                    Citation
                  </Button>
                </div>
              </>
            ) : (
              <p className="mt-1 text-xs text-muted-foreground">Select text in a block to attach range-level support.</p>
            )}
          </div>

          <SupportPicker
            icon={ListChecks}
            label="Fact"
            value={factId}
            onValue={setFactId}
            options={factOptions}
            disabled={pending || !selectedBlock || matter.facts.length === 0}
            onLink={() => onLinkSupport({ targetType: "fact", targetId: factId, relation })}
          />

          <SupportPicker
            icon={Microscope}
            label="Evidence"
            value={evidenceId}
            onValue={setEvidenceId}
            options={evidenceOptions}
            disabled={pending || !selectedBlock || matter.evidence.length === 0}
            onLink={() => onLinkSupport({ targetType: "evidence", targetId: evidenceId, relation })}
          />

          <SupportPicker
            icon={FileText}
            label="Document"
            value={documentId}
            onValue={setDocumentId}
            options={documentOptions}
            disabled={pending || !selectedBlock || matter.documents.length === 0}
            onLink={() => onLinkSupport({ targetType: "document", targetId: documentId, relation })}
          />

          <div className="rounded-md border border-border bg-background p-2">
            <label className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
              <BookOpen className="h-3.5 w-3.5" />
              Authority
            </label>
            <input
              value={authorityText}
              onChange={(event) => setAuthorityText(event.target.value)}
              placeholder="ORS 90.320, ORCP 21, UTCR 2.010..."
              className="mt-2 h-8 w-full rounded-md border border-border bg-card px-2 text-xs focus:border-primary focus:outline-none"
            />
            <Button
              variant="outline"
              size="sm"
              className="mt-2 h-7 w-full justify-center gap-1.5 bg-transparent text-xs"
              disabled={pending || !selectedBlock || !authorityText.trim()}
              onClick={() => {
                const citation = authorityText.trim()
                onLinkSupport({
                  targetType: "authority",
                  targetId: citation,
                  citation,
                  canonicalId: citation,
                  relation,
                })
                setAuthorityText("")
              }}
            >
              <Link2 className="h-3.5 w-3.5" />
              Link authority
            </Button>
          </div>

          <div className="rounded-md border border-border bg-background p-2">
            <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              linked to this block
            </div>
            {selectedAnchors.length > 0 ? (
              <ul className="mt-2 space-y-2">
                {selectedAnchors.slice(0, 6).map((anchor) => {
                  const anchorId = supportAnchorId(anchor)
                  return (
                    <li key={anchorId} className="rounded-md border border-border bg-muted/40 p-2">
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <div className="flex flex-wrap items-center gap-1.5">
                            <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                              {supportTargetLabel(anchor.target_type)}
                            </span>
                            <span className="rounded bg-background px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                              {anchor.status || "linked"}
                            </span>
                          </div>
                          <p className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-foreground">
                            {supportPreview(matter, anchor)}
                          </p>
                        </div>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 shrink-0 px-2 text-[11px] text-muted-foreground"
                          disabled={pending}
                          onClick={() => onRemoveSupport(anchor)}
                        >
                          Remove
                        </Button>
                      </div>
                      <select
                        value={anchor.relation || "supports"}
                        onChange={(event) => onUpdateSupportRelation(anchorId, event.target.value)}
                        className="mt-2 h-7 w-full rounded-md border border-border bg-background px-2 font-mono text-[11px]"
                        disabled={pending}
                      >
                        {SUPPORT_RELATIONS.map((value) => (
                          <option key={value} value={value}>
                            {value.replace(/_/g, " ")}
                          </option>
                        ))}
                      </select>
                    </li>
                  )
                })}
              </ul>
            ) : (
              <p className="mt-2 text-xs text-muted-foreground">No support linked to the selected block yet.</p>
            )}
          </div>

          <div className="rounded-md border border-border bg-background p-2">
            <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              selected text links
            </div>
            {hasSelectedTextLinks ? (
              <ul className="mt-2 space-y-1.5">
                {selectedRangeLinks.map((link) => (
                  <li key={link.link_id} className="rounded bg-muted/50 px-2 py-1.5 text-[11px]">
                    <span className="font-mono uppercase text-muted-foreground">{link.target_type}</span>
                    <span className="mx-1 text-muted-foreground">·</span>
                    <span className="text-foreground">{rangeQuote(link.source_text_range)}</span>
                  </li>
                ))}
                {selectedRangeCitations.map((citation) => (
                  <li key={citation.citation_use_id} className="rounded bg-muted/50 px-2 py-1.5 text-[11px]">
                    <span className="font-mono uppercase text-muted-foreground">citation</span>
                    <span className="mx-1 text-muted-foreground">·</span>
                    <span className="text-foreground">{citation.raw_text}</span>
                  </li>
                ))}
                {selectedRangeExhibits.map((exhibit) => (
                  <li key={exhibit.exhibit_reference_id} className="rounded bg-muted/50 px-2 py-1.5 text-[11px]">
                    <span className="font-mono uppercase text-muted-foreground">exhibit</span>
                    <span className="mx-1 text-muted-foreground">·</span>
                    <span className="text-foreground">{exhibit.label}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="mt-2 text-xs text-muted-foreground">No selected text links on this block yet.</p>
            )}
          </div>
        </div>
      </section>

      <section>
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">QC</h3>
        <div className="mt-2 space-y-2">
          {openFindings.slice(0, 4).map((finding) => (
            <div key={finding.id} className="rounded-md border border-warning/20 bg-warning/10 p-2 text-xs text-warning">
              {finding.message}
            </div>
          ))}
          {openFindings.length === 0 && (
            <div className="rounded-md border border-border bg-background p-2 text-xs text-muted-foreground">
              No open findings in the current payload.
            </div>
          )}
        </div>
      </section>

      <section>
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Provider-free commands</h3>
        <div className="mt-2 grid gap-2">
          {["summarize_support", "find_missing_evidence", "find_missing_authority"].map((command) => (
            <Button
              key={command}
              variant="outline"
              size="sm"
              className="justify-start gap-1.5 bg-transparent text-xs"
              disabled={pending}
              onClick={() => onRunCommand(command, selectedBlock?.id)}
            >
              <Sparkles className="h-3.5 w-3.5" />
              {command.replace(/_/g, " ")}
            </Button>
          ))}
        </div>
      </section>
    </div>
  )
}

function StatusPill({ status }: { status: string }) {
  const Icon = status === "final" ? CheckCircle2 : status === "review" ? Clock : FileText
  return (
    <span className="inline-flex items-center gap-1 rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
      <Icon className="h-2.5 w-2.5" />
      {status}
    </span>
  )
}

function Metric({ label, value, warn = false }: { label: string; value: number; warn?: boolean }) {
  return (
    <div className="rounded-md border border-border bg-background px-2 py-1.5">
      <div className={cn("font-mono text-sm tabular-nums", warn && value > 0 ? "text-warning" : "text-foreground")}>
        {value}
      </div>
      <div className="mt-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">{label}</div>
    </div>
  )
}

function SupportPicker({
  icon: Icon,
  label,
  value,
  onValue,
  options,
  disabled,
  onLink,
}: {
  icon: typeof FileText
  label: string
  value: string
  onValue: (value: string) => void
  options: { id: string; label: string }[]
  disabled: boolean
  onLink: () => void
}) {
  return (
    <div className="rounded-md border border-border bg-background p-2">
      <label className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
        <Icon className="h-3.5 w-3.5" />
        {label}
      </label>
      <select
        value={value}
        onChange={(event) => onValue(event.target.value)}
        className="mt-2 h-8 w-full rounded-md border border-border bg-card px-2 text-xs"
        disabled={options.length === 0}
      >
        {options.length === 0 ? (
          <option value="">No {label.toLowerCase()} records</option>
        ) : (
          options.map((option) => (
            <option key={option.id} value={option.id}>
              {option.label.slice(0, 90)}
            </option>
          ))
        )}
      </select>
      <Button
        variant="outline"
        size="sm"
        className="mt-2 h-7 w-full justify-center gap-1.5 bg-transparent text-xs"
        disabled={disabled || !value}
        onClick={onLink}
      >
        <Link2 className="h-3.5 w-3.5" />
        Link {label.toLowerCase()}
      </Button>
    </div>
  )
}

function captureTextRangeSelection(
  block: WorkProductBlock,
  textarea: HTMLTextAreaElement,
  onTextRangeSelect: (range: SelectedTextRange | null) => void,
) {
  const selectionStart = textarea.selectionStart
  const selectionEnd = textarea.selectionEnd
  if (selectionEnd <= selectionStart) {
    onTextRangeSelect(null)
    return
  }
  const quote = textarea.value.slice(selectionStart, selectionEnd)
  if (!quote.trim()) {
    onTextRangeSelect(null)
    return
  }
  const startOffset = characterOffsetForSelection(textarea.value, selectionStart)
  onTextRangeSelect({
    blockId: block.block_id || block.id,
    startOffset,
    endOffset: startOffset + characterLength(quote),
    quote,
  })
}

function sameSelectedTextRange(left: SelectedTextRange | null, right: SelectedTextRange | null) {
  if (left === right) return true
  if (!left || !right) return false
  return (
    left.blockId === right.blockId &&
    left.startOffset === right.startOffset &&
    left.endOffset === right.endOffset &&
    left.quote === right.quote
  )
}

function characterOffsetForSelection(value: string, selectionOffset: number) {
  return characterLength(value.slice(0, selectionOffset))
}

function characterLength(value: string) {
  return Array.from(value).length
}

function supportAnchorId(anchor: WorkProductAnchor) {
  return anchor.anchor_id || anchor.id
}

function supportTargetLabel(targetType: string) {
  return targetType.replace(/_/g, " ")
}

function supportPreview(matter: Matter, anchor: WorkProductAnchor) {
  if (anchor.quote) return anchor.quote
  if (anchor.target_type === "fact") {
    const fact = matter.facts.find((item) => item.id === anchor.target_id || item.fact_id === anchor.target_id)
    return fact?.statement || fact?.text || anchor.target_id
  }
  if (["evidence", "document", "source_span"].includes(anchor.target_type)) {
    const evidence = matter.evidence.find((item) => item.evidence_id === anchor.target_id)
    const document = matter.documents.find((item) => item.id === anchor.target_id || item.document_id === anchor.target_id)
    const sourceSpan = matter.documents
      .flatMap((item) => item.source_spans ?? [])
      .find((span) => span.source_span_id === anchor.target_id || span.id === anchor.target_id)
    return evidence?.quote || sourceSpan?.quote || document?.title || document?.filename || anchor.target_id
  }
  return anchor.citation || anchor.canonical_id || anchor.target_id
}

function rangeQuote(range?: { quote?: string | null } | null) {
  return range?.quote || "selected text"
}

function blockDraftMap(blocks: WorkProductBlock[]) {
  return Object.fromEntries(blocks.map((block) => [block.id, block.text])) as Record<string, string>
}
