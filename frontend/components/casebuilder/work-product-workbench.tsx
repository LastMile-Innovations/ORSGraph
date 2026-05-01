"use client"

import { useEffect, useMemo, useState } from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import {
  AlertTriangle,
  ArrowLeft,
  CheckCircle2,
  ChevronRight,
  Clock,
  Download,
  FileText,
  GitBranch,
  History,
  Layers3,
  ListChecks,
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
  WorkProductArtifact,
  WorkProductBlock,
  WorkProductFinding,
  WorkProductPreviewResponse,
} from "@/lib/casebuilder/types"
import {
  createWorkProductBlock,
  exportWorkProduct,
  getWorkProductAiAudit,
  getWorkProductExportHistory,
  getWorkProductHistory,
  getWorkProductSnapshots,
  patchWorkProductBlock,
  previewWorkProduct,
  runWorkProductAiCommand,
  runWorkProductQc,
  validateWorkProductAst,
  workProductAstToMarkdown,
} from "@/lib/casebuilder/api"
import {
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

const WORKSPACE_LINKS: { section: WorkProductWorkspaceSection | "overview"; label: string; icon: typeof FileText }[] = [
  { section: "overview", label: "Overview", icon: FileText },
  { section: "editor", label: "Editor", icon: ListChecks },
  { section: "qc", label: "QC", icon: ShieldCheck },
  { section: "preview", label: "Preview", icon: Layers3 },
  { section: "export", label: "Export", icon: Download },
  { section: "history", label: "History", icon: History },
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
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)

  useEffect(() => {
    setWorkProduct(initialWorkProduct)
    setBlockDrafts(blockDraftMap(initialWorkProduct.blocks))
    setExports(initialWorkProduct.artifacts)
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

  const openFindings = workProduct.findings.filter((finding) => finding.status === "open")
  const selectedWarnings = [
    ...workProduct.document_ast.citations.filter((citation) => citation.status !== "resolved").map((citation) => `Citation needs review: ${citation.raw_text}`),
    ...workProduct.document_ast.exhibits.filter((exhibit) => exhibit.status !== "attached").map((exhibit) => `Exhibit needs review: ${exhibit.label}`),
  ].slice(0, 4)

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
      setMessage(previewResult.data.review_label)
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
                  className="block rounded-md px-2 py-1.5 text-xs text-muted-foreground hover:bg-muted hover:text-foreground"
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
                pending={pending}
                onChange={(blockId, text) => setBlockDrafts((current) => ({ ...current, [blockId]: text }))}
                onSave={saveBlock}
                newBlockTitle={newBlockTitle}
                newBlockText={newBlockText}
                onNewBlockTitle={setNewBlockTitle}
                onNewBlockText={setNewBlockText}
                onAddBlock={addBlock}
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
              openFindings={openFindings}
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
  pending,
  onChange,
  onSave,
  newBlockTitle,
  newBlockText,
  onNewBlockTitle,
  onNewBlockText,
  onAddBlock,
}: {
  blocks: WorkProductBlock[]
  drafts: Record<string, string>
  pending: boolean
  onChange: (blockId: string, text: string) => void
  onSave: (block: WorkProductBlock) => void
  newBlockTitle: string
  newBlockText: string
  onNewBlockTitle: (value: string) => void
  onNewBlockText: (value: string) => void
  onAddBlock: () => void
}) {
  return (
    <div className="space-y-4">
      {blocks.map((block) => (
        <Card key={block.id} id={block.id} className="p-4">
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
            className="mt-3 min-h-32 resize-y text-sm leading-relaxed"
          />
        </Card>
      ))}

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
  openFindings,
  onRunCommand,
  pending,
}: {
  workProduct: WorkProduct
  openFindings: WorkProductFinding[]
  onRunCommand: (command: string, targetId?: string) => void
  pending: boolean
}) {
  const firstBlock = workProduct.blocks[0]
  return (
    <div className="space-y-4">
      <section>
        <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Inspector</h2>
        <div className="mt-3 grid grid-cols-2 gap-2">
          <Metric label="Facts" value={workProduct.document_ast.links.filter((link) => link.target_type === "fact").length} />
          <Metric label="Evidence" value={workProduct.document_ast.links.filter((link) => link.target_type === "evidence").length} />
          <Metric label="Cites" value={workProduct.document_ast.citations.length} />
          <Metric label="Exhibits" value={workProduct.document_ast.exhibits.length} />
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
              onClick={() => onRunCommand(command, firstBlock?.id)}
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

function blockDraftMap(blocks: WorkProductBlock[]) {
  return Object.fromEntries(blocks.map((block) => [block.id, block.text])) as Record<string, string>
}
