"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react"
import {
  AlertTriangle,
  Archive,
  BarChart3,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  FileText,
  Filter,
  Folder,
  FolderOpen,
  FolderUp,
  Grid2x2,
  List,
  Mic,
  MoveRight,
  Pencil,
  RefreshCcw,
  RotateCcw,
  Search,
  Upload,
  X,
} from "lucide-react"
import { cn } from "@/lib/utils"
import type {
  CaseBuilderEffectiveSettings,
  CaseDocument,
  DocumentType,
  MatterIndexSummary,
  MatterSummary,
  TranscriptionJobResponse,
} from "@/lib/casebuilder/types"
import {
  archiveDocument,
  createMatterIndexJob,
  createTranscription,
  getMatterSettingsState,
  getMatterIndexSummary,
  listTranscriptions,
  patchDocument,
  restoreDocument,
  syncTranscription,
} from "@/lib/casebuilder/api"
import { matterDocumentHref } from "@/lib/casebuilder/routes"
import {
  buildDocumentTree,
  buildUploadPreviewRows,
  documentIsMarkdownIndexable,
  documentIsArchived,
  documentIsMedia,
  documentIsViewOnly,
  documentLibraryPath,
  filterDocumentsBySelection,
  latestUploadBatchId,
  type DocumentTreeNode,
  type DocumentTreeSelection,
  type UploadPreviewRow,
} from "@/lib/casebuilder/document-tree"
import {
  dataTransferToUploadCandidates,
  filesToUploadCandidates,
  type UploadCandidate,
} from "@/lib/casebuilder/upload-folders"
import { ProcessingBadge } from "./badges"
import { uploadOptionsFromEffectiveSettings, useCaseBuilderUploads } from "./upload-provider"

const TYPE_LABEL: Record<DocumentType, string> = {
  complaint: "Complaint",
  answer: "Answer",
  motion: "Motion",
  order: "Order",
  evidence: "Evidence",
  contract: "Contract",
  lease: "Lease",
  email: "Email",
  letter: "Letter",
  notice: "Notice",
  medical: "Medical record",
  police: "Police report",
  agency_record: "Agency record",
  public_record: "Public record",
  spreadsheet: "Spreadsheet",
  photo: "Photo",
  screenshot: "Screenshot",
  audio_transcript: "Audio transcript",
  receipt: "Receipt",
  invoice: "Invoice",
  exhibit: "Exhibit",
  other: "Other",
}

interface Props {
  matter: MatterSummary
  documents: CaseDocument[]
}

export function DocumentLibrary({ matter, documents }: Props) {
  const router = useRouter()
  const { enqueueMatterUploads } = useCaseBuilderUploads()
  const fileInputRef = useRef<HTMLInputElement>(null)
  const folderInputRef = useRef<HTMLInputElement>(null)
  const [selection, setSelection] = useState<DocumentTreeSelection>({ kind: "all" })
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set())
  const [query, setQuery] = useState("")
  const [view, setView] = useState<"grid" | "list">("grid")
  const [processingFilter, setProcessingFilter] = useState<string>("All")
  const [storageFilter, setStorageFilter] = useState<string>("All")
  const [batchFilter, setBatchFilter] = useState<string>("All")
  const [duplicateOnly, setDuplicateOnly] = useState(false)
  const [uploadMessage, setUploadMessage] = useState<string | null>(null)
  const [pendingUploads, setPendingUploads] = useState<UploadCandidate[]>([])
  const [indexSummary, setIndexSummary] = useState<MatterIndexSummary | null>(null)
  const [indexMessage, setIndexMessage] = useState<string | null>(null)
  const [indexBusy, setIndexBusy] = useState(false)
  const [mediaTranscriptions, setMediaTranscriptions] = useState<Record<string, TranscriptionJobResponse[]>>({})
  const [mediaActionBusy, setMediaActionBusy] = useState<string | null>(null)
  const [mediaMessage, setMediaMessage] = useState<string | null>(null)
  const [selectedDocumentIds, setSelectedDocumentIds] = useState<Set<string>>(new Set())
  const [editingDocument, setEditingDocument] = useState<CaseDocument | null>(null)
  const [actionBusy, setActionBusy] = useState<string | null>(null)
  const [actionMessage, setActionMessage] = useState<string | null>(null)
  const [settings, setSettings] = useState<CaseBuilderEffectiveSettings | null>(null)

  const activeDocuments = useMemo(() => documents.filter((document) => !documentIsArchived(document)), [documents])
  const archivedDocuments = useMemo(() => documents.filter(documentIsArchived), [documents])
  const latestBatch = useMemo(() => latestUploadBatchId(documents), [documents])
  const tree = useMemo(() => buildDocumentTree(documents), [documents])
  const uploadPreviewRows = useMemo(() => buildUploadPreviewRows(pendingUploads, documents), [pendingUploads, documents])

  const duplicateGroups = useMemo(() => {
    const byHash = new Map<string, number>()
    for (const document of activeDocuments) {
      if (!document.file_hash) continue
      byHash.set(document.file_hash, (byHash.get(document.file_hash) ?? 0) + 1)
    }
    return Array.from(byHash.values()).filter((count) => count > 1).length
  }, [activeDocuments])

  const duplicateHashes = useMemo(() => {
    const byHash = new Map<string, number>()
    for (const document of activeDocuments) {
      if (!document.file_hash) continue
      byHash.set(document.file_hash, (byHash.get(document.file_hash) ?? 0) + 1)
    }
    return new Set(Array.from(byHash).filter(([, count]) => count > 1).map(([hash]) => hash))
  }, [activeDocuments])

  const processingOptions = useMemo(() => {
    return Array.from(new Set(documents.map((document) => document.processing_status))).sort()
  }, [documents])

  const storageOptions = useMemo(() => {
    return Array.from(new Set(documents.map((document) => document.storage_status ?? "stored"))).sort()
  }, [documents])

  const batchOptions = useMemo(() => {
    return Array.from(new Set(activeDocuments.map((document) => document.upload_batch_id).filter(Boolean) as string[])).sort()
  }, [activeDocuments])

  useEffect(() => {
    void refreshIndexSummary()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [matter.matter_id, documents.length])

  useEffect(() => {
    let cancelled = false
    async function loadSettings() {
      const result = await getMatterSettingsState(matter.matter_id)
      if (!cancelled) setSettings(result.data?.effective ?? null)
    }
    void loadSettings()
    return () => {
      cancelled = true
    }
  }, [matter.matter_id])

  const selectionDocuments = useMemo(() => {
    return filterDocumentsBySelection(documents, selection, latestBatch ?? undefined)
  }, [documents, latestBatch, selection])

  const filtered = useMemo(() => {
    return selectionDocuments.filter((document) => {
      if (processingFilter !== "All" && document.processing_status !== processingFilter) return false
      if (storageFilter !== "All" && (document.storage_status ?? "stored") !== storageFilter) return false
      if (batchFilter !== "All" && (document.upload_batch_id ?? "No batch") !== batchFilter) return false
      if (duplicateOnly && (!document.file_hash || !duplicateHashes.has(document.file_hash))) return false
      if (query.trim()) {
        const q = query.toLowerCase()
        const hay = `${document.filename} ${documentLibraryPath(document)} ${document.original_relative_path ?? ""} ${document.summary} ${document.parties_mentioned.join(" ")} ${document.entities_mentioned.join(" ")}`.toLowerCase()
        if (!hay.includes(q)) return false
      }
      return true
    })
  }, [batchFilter, duplicateHashes, duplicateOnly, processingFilter, query, selectionDocuments, storageFilter])

  const mediaDocuments = useMemo(() => activeDocuments.filter(documentIsMedia), [activeDocuments])

  useEffect(() => {
    if (selection.kind !== "media" || mediaDocuments.length === 0) return
    let cancelled = false
    async function loadMediaTranscriptions() {
      const pairs = await Promise.all(
        mediaDocuments.map(async (document) => {
          const result = await listTranscriptions(matter.matter_id, document.document_id)
          return [document.document_id, result.data ?? []] as const
        }),
      )
      if (!cancelled) {
        setMediaTranscriptions(Object.fromEntries(pairs))
      }
    }
    void loadMediaTranscriptions()
    return () => {
      cancelled = true
    }
  }, [selection.kind, matter.matter_id, mediaDocuments])

  async function refreshIndexSummary() {
    const result = await getMatterIndexSummary(matter.matter_id)
    if (result.data) setIndexSummary(result.data)
  }

  function queueUploads(candidates: UploadCandidate[]) {
    if (candidates.length === 0) return
    setUploadMessage(null)
    setPendingUploads(candidates)
  }

  async function queueDroppedFiles(event: React.DragEvent<HTMLElement>) {
    event.preventDefault()
    try {
      const candidates = await dataTransferToUploadCandidates(event.dataTransfer)
      queueUploads(candidates)
    } catch (error) {
      setUploadMessage(error instanceof Error ? error.message : String(error))
    }
  }

  async function indexDocuments(documentIds?: string[]) {
    setIndexBusy(true)
    setIndexMessage(null)
    const result = await createMatterIndexJob(matter.matter_id, {
      document_ids: documentIds && documentIds.length > 0 ? documentIds : undefined,
    })
    setIndexBusy(false)
    if (!result.data) {
      setIndexMessage(result.error || "Index job failed to start.")
      return null
    }
    setIndexMessage(`Index job started for ${result.data.requested} document${result.data.requested === 1 ? "" : "s"}.`)
    return result.data
  }

  async function uploadCandidates(candidates: UploadCandidate[]) {
    if (candidates.length === 0) return

    setUploadMessage(null)
    setIndexMessage(null)
    setPendingUploads([])
    const uploadBatchId = enqueueMatterUploads(matter.matter_id, candidates, {
      label: candidates.some((item) => item.relativePath.includes("/")) ? "Folder upload" : "File upload",
      ...uploadOptionsFromEffectiveSettings(settings),
    })
    if (!uploadBatchId) return
    setBatchFilter(uploadBatchId)
    setExpandedPaths((current) => expandUploadAncestors(current, candidates))
    setSelection({ kind: "recent" })
    setUploadMessage("Upload started. You can leave this page while files store and index.")
  }

  async function saveDocumentMetadata(document: CaseDocument, values: {
    title: string
    libraryPath: string
    documentType: string
    confidentiality: string
    isExhibit: boolean
    exhibitLabel: string
    dateObserved: string
  }) {
    setActionBusy(`${document.document_id}:patch`)
    setActionMessage(null)
    const result = await patchDocument(matter.matter_id, document.document_id, {
      title: values.title,
      library_path: values.libraryPath,
      document_type: values.documentType,
      confidentiality: values.confidentiality,
      is_exhibit: values.isExhibit,
      exhibit_label: values.exhibitLabel.trim() || null,
      date_observed: values.dateObserved.trim() || null,
    })
    setActionBusy(null)
    if (!result.data) {
      setActionMessage(result.error || "Document update failed.")
      return
    }
    const updatedDocument = result.data
    setEditingDocument(null)
    setActionMessage("Document metadata updated.")
    setExpandedPaths((current) => expandPath(current, documentLibraryPath(updatedDocument)))
    router.refresh()
  }

  async function archiveOne(document: CaseDocument) {
    setActionBusy(`${document.document_id}:archive`)
    setActionMessage(null)
    const result = await archiveDocument(matter.matter_id, document.document_id, { reason: "Archived from document library" })
    setActionBusy(null)
    if (!result.data) {
      setActionMessage(result.error || "Archive failed.")
      return
    }
    setSelectedDocumentIds((current) => withoutId(current, document.document_id))
    setActionMessage("Document archived. Source object preserved.")
    router.refresh()
  }

  async function restoreOne(document: CaseDocument) {
    setActionBusy(`${document.document_id}:restore`)
    setActionMessage(null)
    const result = await restoreDocument(matter.matter_id, document.document_id)
    setActionBusy(null)
    if (!result.data) {
      setActionMessage(result.error || "Restore failed.")
      return
    }
    setActionMessage("Document restored to active files.")
    router.refresh()
  }

  async function bulkArchive() {
    const targets = documents.filter((document) => selectedDocumentIds.has(document.document_id) && !documentIsArchived(document))
    for (const document of targets) {
      await archiveDocument(matter.matter_id, document.document_id, { reason: "Bulk archived from document library" })
    }
    setSelectedDocumentIds(new Set())
    setActionMessage(`${targets.length} document${targets.length === 1 ? "" : "s"} archived.`)
    router.refresh()
  }

  async function bulkRestore() {
    const targets = documents.filter((document) => selectedDocumentIds.has(document.document_id) && documentIsArchived(document))
    for (const document of targets) {
      await restoreDocument(matter.matter_id, document.document_id)
    }
    setSelectedDocumentIds(new Set())
    setActionMessage(`${targets.length} document${targets.length === 1 ? "" : "s"} restored.`)
    router.refresh()
  }

  async function bulkMove() {
    const folder = window.prompt("Move selected documents to folder path", selection.kind === "folder" ? selection.path : "Uploads")
    if (!folder) return
    const normalizedFolder = folder.replace(/\\/g, "/").replace(/^\/+|\/+$/g, "")
    if (!normalizedFolder.trim()) return
    const targets = documents.filter((document) => selectedDocumentIds.has(document.document_id) && !documentIsArchived(document))
    for (const document of targets) {
      const filename = documentLibraryPath(document).split("/").pop() || document.filename
      await patchDocument(matter.matter_id, document.document_id, {
        library_path: `${normalizedFolder}/${filename}`,
      })
    }
    setExpandedPaths((current) => expandPath(current, `${normalizedFolder}/placeholder`))
    setSelectedDocumentIds(new Set())
    setActionMessage(`${targets.length} document${targets.length === 1 ? "" : "s"} moved.`)
    router.refresh()
  }

  async function startMediaTranscription(document: CaseDocument, force = false) {
    setMediaActionBusy(`${document.document_id}:transcribe`)
    setMediaMessage(null)
    const result = await createTranscription(matter.matter_id, document.document_id, {
      force,
      redact_pii: settings?.transcript_redact_pii ?? true,
      speaker_labels: settings?.transcript_speaker_labels ?? true,
      prompt_preset: settings?.transcript_prompt_preset || null,
      remove_audio_tags: (settings?.transcript_remove_audio_tags ?? true) ? "all" : null,
    })
    setMediaActionBusy(null)
    if (!result.data) {
      setMediaMessage(result.error || "Transcription request failed.")
      return
    }
    const transcription = result.data
    setMediaTranscriptions((current) => ({
      ...current,
      [document.document_id]: replaceLatestTranscription(current[document.document_id] ?? [], transcription),
    }))
    setMediaMessage(transcription.warnings[0] ?? "Transcript job updated.")
    router.refresh()
  }

  async function syncMediaTranscription(document: CaseDocument, transcription: TranscriptionJobResponse) {
    setMediaActionBusy(`${document.document_id}:sync`)
    setMediaMessage(null)
    const result = await syncTranscription(matter.matter_id, document.document_id, transcription.job.transcription_job_id)
    setMediaActionBusy(null)
    if (!result.data) {
      setMediaMessage(result.error || "Transcript sync failed.")
      return
    }
    const synced = result.data
    setMediaTranscriptions((current) => ({
      ...current,
      [document.document_id]: replaceLatestTranscription(current[document.document_id] ?? [], synced),
    }))
    setMediaMessage(synced.warnings[0] ?? "Transcript status refreshed.")
    router.refresh()
  }

  function toggleSelected(documentId: string) {
    setSelectedDocumentIds((current) => {
      const next = new Set(current)
      if (next.has(documentId)) next.delete(documentId)
      else next.add(documentId)
      return next
    })
  }

  const selectedCount = selectedDocumentIds.size
  const activeSelectedCount = documents.filter((document) => selectedDocumentIds.has(document.document_id) && !documentIsArchived(document)).length
  const archivedSelectedCount = documents.filter((document) => selectedDocumentIds.has(document.document_id) && documentIsArchived(document)).length

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="border-b border-border bg-card px-6 py-4">
        <div className="flex items-end justify-between gap-3">
          <div>
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              CaseBuilder · documents
            </div>
            <h1 className="mt-1 text-base font-semibold text-foreground">Matter files</h1>
            <p className="mt-0.5 text-xs text-muted-foreground">
              {activeDocuments.length} active · {archivedDocuments.length} archived · {activeDocuments.reduce((s, d) => s + d.facts_extracted, 0)} facts extracted ·{" "}
              {activeDocuments.reduce((s, d) => s + d.contradictions_flagged, 0)} contradictions flagged
            </p>
          </div>
          <input
            ref={fileInputRef}
            type="file"
            multiple
            hidden
            onChange={(event) => {
              if (event.target.files) queueUploads(filesToUploadCandidates(event.target.files))
              event.currentTarget.value = ""
            }}
          />
          <input
            ref={folderInputRef}
            type="file"
            multiple
            hidden
            {...({ webkitdirectory: "", directory: "" } as Record<string, string>)}
            onChange={(event) => {
              if (event.target.files) queueUploads(filesToUploadCandidates(event.target.files))
              event.currentTarget.value = ""
            }}
          />
          <div className="flex items-center gap-2">
            <button
              onClick={() => folderInputRef.current?.click()}
              className="flex items-center gap-1.5 rounded border border-border bg-background px-3 py-1.5 font-mono text-xs uppercase tracking-wider text-foreground hover:bg-muted"
            >
              <FolderUp className="h-3.5 w-3.5" />
              folder
            </button>
            <button
              onClick={() => fileInputRef.current?.click()}
              className="flex items-center gap-1.5 rounded bg-primary px-3 py-1.5 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90"
            >
              <Upload className="h-3.5 w-3.5" />
              upload files
            </button>
          </div>
        </div>
        {(uploadMessage || actionMessage) && (
          <div className="mt-3 rounded border border-primary/20 bg-primary/5 px-3 py-2 text-xs text-muted-foreground">
            {uploadMessage || actionMessage}
          </div>
        )}
        <div className="mt-3 grid gap-2">
          <IndexConsole
            documents={activeDocuments}
            duplicateGroups={duplicateGroups}
            indexBusy={indexBusy}
            indexMessage={indexMessage}
            indexSummary={indexSummary}
            onIndex={() => void indexDocuments()}
          />
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2 border-b border-border px-6 py-2">
        <div className="flex flex-1 items-center gap-2 rounded border border-border bg-background px-2.5">
          <Search className="h-3.5 w-3.5 text-muted-foreground" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search filenames, folder paths, summaries, parties, entities..."
            className="flex-1 bg-transparent py-1.5 text-xs focus:outline-none"
          />
        </div>
        <select
          value={processingFilter}
          onChange={(event) => setProcessingFilter(event.target.value)}
          className="h-8 rounded border border-border bg-background px-2 font-mono text-[11px] uppercase tracking-wider text-muted-foreground"
        >
          <option value="All">all status</option>
          {processingOptions.map((status) => (
            <option key={status} value={status}>
              {status}
            </option>
          ))}
        </select>
        <select
          value={storageFilter}
          onChange={(event) => setStorageFilter(event.target.value)}
          className="h-8 rounded border border-border bg-background px-2 font-mono text-[11px] uppercase tracking-wider text-muted-foreground"
        >
          <option value="All">all storage</option>
          {storageOptions.map((status) => (
            <option key={status} value={status}>
              {status}
            </option>
          ))}
        </select>
        <select
          value={batchFilter}
          onChange={(event) => setBatchFilter(event.target.value)}
          className="h-8 rounded border border-border bg-background px-2 font-mono text-[11px] uppercase tracking-wider text-muted-foreground"
        >
          <option value="All">all batches</option>
          {batchOptions.map((batch) => (
            <option key={batch} value={batch}>
              {batch}
            </option>
          ))}
        </select>
        <button
          type="button"
          onClick={() => setDuplicateOnly((value) => !value)}
          className={cn(
            "h-8 rounded border px-2 font-mono text-[11px] uppercase tracking-wider",
            duplicateOnly ? "border-warning bg-warning/10 text-warning" : "border-border text-muted-foreground hover:bg-muted",
          )}
        >
          duplicates
        </button>
        <button
          onClick={() => setView("grid")}
          className={cn(
            "flex h-7 w-7 items-center justify-center rounded border",
            view === "grid" ? "border-primary text-primary" : "border-border text-muted-foreground",
          )}
          aria-label="Grid view"
        >
          <Grid2x2 className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={() => setView("list")}
          className={cn(
            "flex h-7 w-7 items-center justify-center rounded border",
            view === "list" ? "border-primary text-primary" : "border-border text-muted-foreground",
          )}
          aria-label="List view"
        >
          <List className="h-3.5 w-3.5" />
        </button>
      </div>

      {selectedCount > 0 && (
        <div className="flex flex-wrap items-center gap-2 border-b border-border bg-primary/5 px-6 py-2 text-xs">
          <span className="font-mono uppercase tracking-wider text-primary">{selectedCount} selected</span>
          <button
            type="button"
            onClick={() => void bulkMove()}
            disabled={activeSelectedCount === 0}
            className="inline-flex items-center gap-1 rounded border border-border bg-background px-2 py-1 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted disabled:opacity-50"
          >
            <MoveRight className="h-3 w-3" />
            move
          </button>
          <button
            type="button"
            onClick={() => void bulkArchive()}
            disabled={activeSelectedCount === 0}
            className="inline-flex items-center gap-1 rounded border border-border bg-background px-2 py-1 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted disabled:opacity-50"
          >
            <Archive className="h-3 w-3" />
            archive
          </button>
          <button
            type="button"
            onClick={() => void bulkRestore()}
            disabled={archivedSelectedCount === 0}
            className="inline-flex items-center gap-1 rounded border border-border bg-background px-2 py-1 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted disabled:opacity-50"
          >
            <RotateCcw className="h-3 w-3" />
            restore
          </button>
          <button
            type="button"
            onClick={() => setSelectedDocumentIds(new Set())}
            className="inline-flex items-center gap-1 rounded border border-border bg-background px-2 py-1 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted"
          >
            clear
          </button>
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        <DocumentTreePanel
          activeCount={activeDocuments.length}
          archivedCount={archivedDocuments.length}
          duplicateCount={duplicateGroups}
          expandedPaths={expandedPaths}
          latestBatchCount={latestBatch ? activeDocuments.filter((document) => document.upload_batch_id === latestBatch).length : 0}
          mediaCount={mediaDocuments.length}
          selection={selection}
          tree={tree}
          onSelect={setSelection}
          onToggleExpanded={(path) => {
            setExpandedPaths((current) => {
              const next = new Set(current)
              if (next.has(path)) next.delete(path)
              else next.add(path)
              return next
            })
          }}
        />

        <div
          className="flex-1 overflow-y-auto p-4 scrollbar-thin"
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => void queueDroppedFiles(event)}
        >
          {selection.kind === "media" ? (
            <MediaQueue
              busy={mediaActionBusy}
              documents={filtered}
              matter={matter}
              message={mediaMessage}
              transcriptions={mediaTranscriptions}
              onStart={startMediaTranscription}
              onSync={syncMediaTranscription}
            />
          ) : filtered.length === 0 ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
              <Filter className="h-8 w-8 text-muted-foreground" />
              <div className="text-sm font-medium">No documents match</div>
              <p className="max-w-md text-xs text-muted-foreground">
                Try clearing filters, choosing another tree node, or uploading a file or folder.
              </p>
            </div>
          ) : view === "grid" ? (
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
              {filtered.map((document) => (
                <DocCard
                  key={document.document_id}
                  actionBusy={actionBusy}
                  doc={document}
                  matter={matter}
                  selected={selectedDocumentIds.has(document.document_id)}
                  onArchive={archiveOne}
                  onEdit={setEditingDocument}
                  onIndex={(doc) => void indexDocuments([doc.document_id])}
                  onRestore={restoreOne}
                  onStartTranscription={startMediaTranscription}
                  onToggleSelected={toggleSelected}
                />
              ))}
            </div>
          ) : (
            <DocList
              actionBusy={actionBusy}
              docs={filtered}
              matter={matter}
              selectedDocumentIds={selectedDocumentIds}
              onArchive={archiveOne}
              onEdit={setEditingDocument}
              onIndex={(doc) => void indexDocuments([doc.document_id])}
              onRestore={restoreOne}
              onStartTranscription={startMediaTranscription}
              onToggleSelected={toggleSelected}
            />
          )}
        </div>
      </div>

      {pendingUploads.length > 0 && (
        <UploadPreviewDialog
          rows={uploadPreviewRows}
          uploading={false}
          onCancel={() => setPendingUploads([])}
          onRemove={(index) => setPendingUploads((current) => current.filter((_, itemIndex) => itemIndex !== index))}
          onUpload={() => void uploadCandidates(pendingUploads)}
        />
      )}

      {editingDocument && (
        <DocumentEditDialog
          busy={actionBusy === `${editingDocument.document_id}:patch`}
          document={editingDocument}
          onCancel={() => setEditingDocument(null)}
          onSave={(values) => void saveDocumentMetadata(editingDocument, values)}
        />
      )}
    </div>
  )
}

function IndexConsole({
  documents,
  duplicateGroups,
  indexBusy,
  indexMessage,
  indexSummary,
  onIndex,
}: {
  documents: CaseDocument[]
  duplicateGroups: number
  indexBusy: boolean
  indexMessage: string | null
  indexSummary: MatterIndexSummary | null
  onIndex: () => void
}) {
  const markdownIndexable = documents.filter((document) => documentIsMarkdownIndexable(document)).length
  const viewOnly = indexSummary?.processing_status_counts?.find((item) => item.status === "view_only")?.count
    ?? documents.filter((document) => documentIsViewOnly(document)).length
  return (
    <div className="rounded border border-border bg-background p-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          <BarChart3 className="h-3.5 w-3.5" />
          index console
        </div>
        <button
          type="button"
          onClick={onIndex}
          disabled={indexBusy || (indexSummary?.extractable_pending_documents ?? 0) === 0}
          className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
        >
          <RefreshCcw className={cn("h-3 w-3", indexBusy && "animate-spin")} />
          reindex pending
        </button>
      </div>
      <div className="grid grid-cols-3 gap-2 text-center md:grid-cols-9">
        <IndexTile label="active" value={indexSummary?.active_documents ?? documents.length} />
        <IndexTile label="archived" value={indexSummary?.archived_documents ?? 0} />
        <IndexTile label="indexed" value={indexSummary?.indexed_documents ?? documents.filter((d) => d.facts_extracted > 0 || (d.source_spans?.length ?? 0) > 0).length} />
        <IndexTile label="pending" value={indexSummary?.pending_documents ?? documents.filter((d) => d.processing_status === "queued").length} />
        <IndexTile label="extractable" value={indexSummary?.extractable_pending_documents ?? 0} />
        <IndexTile label="markdown" value={markdownIndexable} />
        <IndexTile label="view only" value={viewOnly} />
        <IndexTile label="failed" value={indexSummary?.failed_documents ?? documents.filter((d) => d.processing_status === "failed").length} tone="bad" />
        <IndexTile label="duplicates" value={indexSummary?.duplicate_groups.length ?? duplicateGroups} tone={duplicateGroups ? "warn" : undefined} />
      </div>
      {(indexMessage || indexSummary?.recent_ingestion_runs[0]) && (
        <div className="mt-2 truncate font-mono text-[10px] text-muted-foreground">
          {indexMessage ??
            `last run ${indexSummary?.recent_ingestion_runs[0]?.stage ?? "stored"} · ${indexSummary?.recent_ingestion_runs[0]?.status ?? "queued"}`}
        </div>
      )}
    </div>
  )
}

function DocumentTreePanel({
  activeCount,
  archivedCount,
  duplicateCount,
  expandedPaths,
  latestBatchCount,
  mediaCount,
  selection,
  tree,
  onSelect,
  onToggleExpanded,
}: {
  activeCount: number
  archivedCount: number
  duplicateCount: number
  expandedPaths: Set<string>
  latestBatchCount: number
  mediaCount: number
  selection: DocumentTreeSelection
  tree: DocumentTreeNode
  onSelect: (selection: DocumentTreeSelection) => void
  onToggleExpanded: (path: string) => void
}) {
  return (
    <aside className="w-64 flex-shrink-0 overflow-y-auto border-r border-border bg-background">
      <div className="px-3 py-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">matter files</div>
      <div className="space-y-px px-1 pb-3">
        <TreeSpecialItem label="All active" count={activeCount} icon={FolderOpen} active={selection.kind === "all"} onClick={() => onSelect({ kind: "all" })} />
        <TreeSpecialItem label="Recent uploads" count={latestBatchCount} icon={Upload} active={selection.kind === "recent"} onClick={() => onSelect({ kind: "recent" })} />
        <TreeSpecialItem label="Duplicates" count={duplicateCount} icon={AlertTriangle} active={selection.kind === "duplicates"} onClick={() => onSelect({ kind: "duplicates" })} />
        <TreeSpecialItem label="Media queue" count={mediaCount} icon={Mic} active={selection.kind === "media"} onClick={() => onSelect({ kind: "media" })} />
        <TreeSpecialItem label="Archive" count={archivedCount} icon={Archive} active={selection.kind === "archive"} onClick={() => onSelect({ kind: "archive" })} />
      </div>
      <div className="border-t border-border px-3 py-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">folders</div>
      <div className="space-y-px px-1 pb-3">
        {tree.children.length === 0 ? (
          <div className="px-2 py-2 text-xs text-muted-foreground">No active folders yet.</div>
        ) : (
          tree.children.map((node) => (
            <TreeNodeItem
              key={node.path}
              expandedPaths={expandedPaths}
              node={node}
              selection={selection}
              onSelect={onSelect}
              onToggleExpanded={onToggleExpanded}
            />
          ))
        )}
      </div>
    </aside>
  )
}

function TreeSpecialItem({
  active,
  count,
  icon: Icon,
  label,
  onClick,
}: {
  active: boolean
  count: number
  icon: typeof Folder
  label: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full items-center justify-between rounded px-2 py-1.5 text-left text-xs transition-colors",
        active ? "bg-primary/10 text-primary" : "text-foreground hover:bg-muted",
      )}
    >
      <span className="flex min-w-0 items-center gap-1.5">
        <Icon className="h-3.5 w-3.5 shrink-0" />
        <span className="truncate">{label}</span>
      </span>
      <span className="font-mono text-[10px] tabular-nums text-muted-foreground">{count}</span>
    </button>
  )
}

function TreeNodeItem({
  expandedPaths,
  node,
  selection,
  onSelect,
  onToggleExpanded,
}: {
  expandedPaths: Set<string>
  node: DocumentTreeNode
  selection: DocumentTreeSelection
  onSelect: (selection: DocumentTreeSelection) => void
  onToggleExpanded: (path: string) => void
}) {
  const expanded = expandedPaths.has(node.path)
  const active = selection.kind === "folder" && selection.path === node.path
  return (
    <div>
      <div className="flex items-center">
        <button
          type="button"
          onClick={() => onToggleExpanded(node.path)}
          className="flex h-7 w-6 items-center justify-center text-muted-foreground hover:text-foreground"
          aria-label={expanded ? `Collapse ${node.name}` : `Expand ${node.name}`}
        >
          {node.children.length > 0 ? expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" /> : <span className="h-3.5 w-3.5" />}
        </button>
        <button
          type="button"
          onClick={() => onSelect({ kind: "folder", path: node.path })}
          className={cn(
            "flex min-w-0 flex-1 items-center justify-between rounded px-1.5 py-1 text-left text-xs",
            active ? "bg-primary/10 text-primary" : "text-foreground hover:bg-muted",
          )}
          style={{ paddingLeft: `${Math.max(0, node.depth) * 10 + 6}px` }}
        >
          <span className="flex min-w-0 items-center gap-1.5">
            <Folder className="h-3.5 w-3.5 shrink-0" />
            <span className="truncate" title={node.path}>{node.name}</span>
          </span>
          <span className="font-mono text-[10px] tabular-nums text-muted-foreground">{node.counts.active}</span>
        </button>
      </div>
      {expanded && node.children.map((child) => (
        <TreeNodeItem
          key={child.path}
          expandedPaths={expandedPaths}
          node={child}
          selection={selection}
          onSelect={onSelect}
          onToggleExpanded={onToggleExpanded}
        />
      ))}
    </div>
  )
}

function IndexTile({ label, value, tone }: { label: string; value: number; tone?: "warn" | "bad" }) {
  return (
    <div className="rounded border border-border bg-card px-2 py-1">
      <div className="font-mono text-[9px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div
        className={cn(
          "font-mono text-sm font-semibold tabular-nums",
          tone === "warn" ? "text-warning" : tone === "bad" ? "text-destructive" : "text-foreground",
        )}
      >
        {value}
      </div>
    </div>
  )
}

function MediaQueue({
  busy,
  documents,
  matter,
  message,
  transcriptions,
  onStart,
  onSync,
}: {
  busy: string | null
  documents: CaseDocument[]
  matter: MatterSummary
  message: string | null
  transcriptions: Record<string, TranscriptionJobResponse[]>
  onStart: (document: CaseDocument, force?: boolean) => void
  onSync: (document: CaseDocument, transcription: TranscriptionJobResponse) => void
}) {
  return (
    <div className="space-y-3">
      <div className="rounded border border-border bg-card p-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">media operations queue</div>
            <div className="mt-1 text-xs text-muted-foreground">
              {documents.length} media file{documents.length === 1 ? "" : "s"} stored for viewing. Transcription is disabled while Markdown-only indexing is enabled.
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              disabled
              className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground opacity-50"
            >
              <RefreshCcw className="h-3 w-3" />
              bulk sync pending
            </button>
            <button
              type="button"
              disabled
              className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground opacity-50"
            >
              <Mic className="h-3 w-3" />
              bulk retry pending
            </button>
          </div>
        </div>
        {message && <div className="mt-2 rounded border border-border bg-background px-3 py-2 text-xs text-muted-foreground">{message}</div>}
      </div>

      {documents.length === 0 ? (
        <div className="flex min-h-[320px] flex-col items-center justify-center rounded border border-dashed text-center text-sm text-muted-foreground">
          <Mic className="mb-3 h-8 w-8" />
          <div className="font-medium text-foreground">No media files match</div>
          <p className="mt-2 max-w-md text-xs">Upload audio or video, or clear filters to see transcript operations.</p>
        </div>
      ) : (
        <div className="overflow-x-auto rounded border border-border bg-card">
          <table className="w-full text-xs">
            <thead className="border-b border-border bg-muted/40 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              <tr>
                <th className="px-3 py-2 text-left">media</th>
                <th className="px-3 py-2 text-left">document status</th>
                <th className="px-3 py-2 text-left">latest transcript</th>
                <th className="px-3 py-2 text-left">provider</th>
                <th className="px-3 py-2 text-right">actions</th>
              </tr>
            </thead>
            <tbody>
              {documents.map((document) => {
                const latest = latestTranscription(transcriptions[document.document_id] ?? [])
                const status = latest?.job.status ?? "not_started"
                const failed = latest?.job.status === "failed" || latest?.job.status === "provider_disabled"
                const processingDisabled = documentIsViewOnly(document) || !documentIsMarkdownIndexable(document)
                const canStart = !latest
                const canRetry = Boolean(latest && (latest.job.retryable || latest.job.status === "failed" || latest.job.status === "provider_disabled"))
                const canReview = Boolean(latest && latest.segments.length > 0)
                const documentHref = matterDocumentHref(matter.matter_id, document.document_id)
                return (
                  <tr key={document.document_id} className="border-b border-border hover:bg-muted/20">
                    <td className="px-3 py-2">
                      <Link href={documentHref} className="block font-mono text-foreground hover:text-primary">
                        <span className="block truncate">{document.filename}</span>
                        <span className="block truncate text-[10px] text-muted-foreground">{documentLibraryPath(document)}</span>
                      </Link>
                    </td>
                    <td className="px-3 py-2">
                      <ProcessingBadge status={document.processing_status} />
                    </td>
                    <td className="px-3 py-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <TranscriptStatusPill status={status} />
                        {latest && <span className="font-mono text-[10px] text-muted-foreground">{latest.job.segment_count || latest.segments.length} seg</span>}
                      </div>
                      {failed && (
                        <div className="mt-1 max-w-md truncate text-[10px] text-destructive">
                          {latest?.job.error_message || (latest?.job.status === "provider_disabled" ? "provider disabled" : "provider error")}
                        </div>
                      )}
                    </td>
                    <td className="px-3 py-2 font-mono text-[11px] text-muted-foreground">
                      {latest ? `${latest.job.provider_mode}${latest.job.provider_status ? ` / ${latest.job.provider_status}` : ""}` : "none"}
                    </td>
                    <td className="px-3 py-2">
                      <div className="flex justify-end gap-1">
                        <button
                          type="button"
                          onClick={() => void onStart(document, canRetry)}
                          disabled={processingDisabled || !(canStart || canRetry) || busy === `${document.document_id}:transcribe`}
                          className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
                        >
                          <Mic className="h-3 w-3" />
                          {latest ? "retry" : "transcribe"}
                        </button>
                        <button
                          type="button"
                          onClick={() => latest && void onSync(document, latest)}
                          disabled={processingDisabled || !latest || busy === `${document.document_id}:sync`}
                          className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
                        >
                          <RefreshCcw className="h-3 w-3" />
                          sync
                        </button>
                        <Link
                          href={documentHref}
                          aria-disabled={processingDisabled || !canReview}
                          className={cn(
                            "inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:bg-muted",
                            (processingDisabled || !canReview) && "pointer-events-none opacity-50",
                          )}
                        >
                          <CheckCircle2 className="h-3 w-3" />
                          review
                        </Link>
                        <Link
                          href={documentHref}
                          className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:bg-muted"
                        >
                          open
                        </Link>
                      </div>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}

function DocCard({
  actionBusy,
  doc,
  matter,
  selected,
  onArchive,
  onEdit,
  onIndex,
  onRestore,
  onStartTranscription,
  onToggleSelected,
}: DocumentActionProps & { selected: boolean }) {
  const archived = documentIsArchived(doc)
  return (
    <article className="group flex flex-col gap-2 rounded border border-border bg-card p-3 hover:border-primary/40">
      <div className="flex items-start gap-2">
        <input
          type="checkbox"
          checked={selected}
          onChange={() => onToggleSelected(doc.document_id)}
          className="mt-2 h-3.5 w-3.5 rounded border-border"
          aria-label={`Select ${doc.filename}`}
        />
        <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded bg-primary/10 text-primary">
          <FileText className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1">
          <Link href={matterDocumentHref(matter.matter_id, doc.document_id)} className="block truncate font-mono text-[11px] text-foreground hover:text-primary">
            {doc.filename}
          </Link>
          <div className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground">{documentLibraryPath(doc)}</div>
          <div className="mt-0.5 flex items-center gap-2 font-mono text-[10px] tabular-nums uppercase tracking-wider text-muted-foreground">
            <span>{TYPE_LABEL[doc.document_type] ?? doc.document_type}</span>
            <span>·</span>
            <span>{doc.pages}p</span>
            <span>·</span>
            <span>{(doc.bytes / 1024).toFixed(0)} KB</span>
          </div>
        </div>
        {doc.is_exhibit && doc.exhibit_label && (
          <span className="rounded bg-accent/15 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider text-accent">
            {doc.exhibit_label}
          </span>
        )}
      </div>

      <p className="line-clamp-3 text-[11px] leading-relaxed text-muted-foreground">{doc.summary}</p>

      <div className="grid grid-cols-3 gap-1 text-center">
        <Tile label="facts" value={doc.facts_extracted} />
        <Tile label="cites" value={doc.citations_found} />
        <Tile label="flags" value={doc.contradictions_flagged} tone={doc.contradictions_flagged > 0 ? "warn" : undefined} />
      </div>

      <div className="flex flex-wrap items-center gap-1 border-t border-border pt-2">
        <ProcessingBadge status={doc.processing_status} />
        <StoragePill status={doc.storage_status} />
        <DocumentActions
          actionBusy={actionBusy}
          doc={doc}
          matter={matter}
          onArchive={onArchive}
          onEdit={onEdit}
          onIndex={onIndex}
          onRestore={onRestore}
          onStartTranscription={onStartTranscription}
          onToggleSelected={onToggleSelected}
        />
        {archived && <span className="ml-auto font-mono text-[10px] uppercase tracking-wider text-warning">archived</span>}
      </div>
    </article>
  )
}

interface DocumentActionProps {
  actionBusy: string | null
  doc: CaseDocument
  matter: MatterSummary
  onArchive: (document: CaseDocument) => void
  onEdit: (document: CaseDocument) => void
  onIndex: (document: CaseDocument) => void
  onRestore: (document: CaseDocument) => void
  onStartTranscription: (document: CaseDocument, force?: boolean) => void
  onToggleSelected: (documentId: string) => void
}

function DocumentActions({
  actionBusy,
  doc,
  matter,
  onArchive,
  onEdit,
  onIndex,
  onRestore,
  onStartTranscription,
}: DocumentActionProps) {
  const archived = documentIsArchived(doc)
  return (
    <div className="ml-auto flex flex-wrap items-center justify-end gap-1">
      <Link
        href={matterDocumentHref(matter.matter_id, doc.document_id)}
        className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted"
      >
        open
      </Link>
      {!archived && (
        <>
          <button
            type="button"
            onClick={() => onEdit(doc)}
            className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted"
          >
            <Pencil className="h-3 w-3" />
            manage
          </button>
          <button
            type="button"
            onClick={() => void onIndex(doc)}
            className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted"
          >
            <RefreshCcw className="h-3 w-3" />
            index
          </button>
          {documentIsMedia(doc) && (
            <button
              type="button"
              onClick={() => void onStartTranscription(doc, false)}
              className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted"
            >
              <Mic className="h-3 w-3" />
              transcribe
            </button>
          )}
          <button
            type="button"
            onClick={() => void onArchive(doc)}
            disabled={actionBusy === `${doc.document_id}:archive`}
            className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted disabled:opacity-50"
            title="Archive preserves the stored source object and metadata."
          >
            <Archive className="h-3 w-3" />
            archive
          </button>
        </>
      )}
      {archived && (
        <button
          type="button"
          onClick={() => void onRestore(doc)}
          disabled={actionBusy === `${doc.document_id}:restore`}
          className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted disabled:opacity-50"
        >
          <RotateCcw className="h-3 w-3" />
          restore
        </button>
      )}
    </div>
  )
}

function Tile({ label, value, tone }: { label: string; value: number; tone?: "warn" }) {
  return (
    <div className={cn("rounded border border-border bg-background py-1", tone === "warn" && "border-warning/40 bg-warning/5")}>
      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div className={cn("font-mono text-sm font-semibold tabular-nums", tone === "warn" ? "text-warning" : "text-foreground")}>
        {value}
      </div>
    </div>
  )
}

function DocList({
  actionBusy,
  docs,
  matter,
  selectedDocumentIds,
  onArchive,
  onEdit,
  onIndex,
  onRestore,
  onStartTranscription,
  onToggleSelected,
}: Omit<DocumentActionProps, "doc"> & { docs: CaseDocument[]; selectedDocumentIds: Set<string> }) {
  return (
    <div className="overflow-x-auto rounded border border-border bg-card">
      <table className="w-full text-xs">
        <thead className="border-b border-border bg-muted/40 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
          <tr>
            <th className="w-8 px-3 py-2 text-left">sel</th>
            <th className="px-3 py-2 text-left">filename</th>
            <th className="px-3 py-2 text-left">tree path</th>
            <th className="px-3 py-2 text-left">type</th>
            <th className="px-3 py-2 text-right">facts</th>
            <th className="px-3 py-2 text-right">flags</th>
            <th className="px-3 py-2 text-left">status</th>
            <th className="px-3 py-2 text-left">storage</th>
            <th className="px-3 py-2 text-right">actions</th>
          </tr>
        </thead>
        <tbody>
          {docs.map((doc) => (
            <tr key={doc.document_id} className="border-b border-border hover:bg-muted/20">
              <td className="px-3 py-2">
                <input
                  type="checkbox"
                  checked={selectedDocumentIds.has(doc.document_id)}
                  onChange={() => onToggleSelected(doc.document_id)}
                  aria-label={`Select ${doc.filename}`}
                />
              </td>
              <td className="px-3 py-2">
                <Link href={matterDocumentHref(matter.matter_id, doc.document_id)} className="flex items-center gap-2 font-mono text-foreground hover:text-primary">
                  <FileText className="h-3.5 w-3.5 text-muted-foreground" />
                  <span className="min-w-0">
                    <span className="block truncate">{doc.filename}</span>
                    {doc.is_exhibit && doc.exhibit_label && (
                      <span className="rounded bg-accent/15 px-1 font-mono text-[10px] uppercase text-accent">{doc.exhibit_label}</span>
                    )}
                  </span>
                </Link>
              </td>
              <td className="max-w-sm px-3 py-2 font-mono text-[10px] text-muted-foreground">
                <span className="block truncate" title={documentLibraryPath(doc)}>{documentLibraryPath(doc)}</span>
              </td>
              <td className="px-3 py-2 font-mono text-[11px] uppercase tracking-wider text-muted-foreground">
                {TYPE_LABEL[doc.document_type] ?? doc.document_type}
              </td>
              <td className="px-3 py-2 text-right font-mono tabular-nums">{doc.facts_extracted}</td>
              <td className={cn("px-3 py-2 text-right font-mono tabular-nums", doc.contradictions_flagged > 0 ? "text-warning" : "text-muted-foreground")}>
                {doc.contradictions_flagged > 0 ? (
                  <span className="inline-flex items-center gap-1">
                    <AlertTriangle className="h-3 w-3" />
                    {doc.contradictions_flagged}
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 text-success">
                    <CheckCircle2 className="h-3 w-3" />0
                  </span>
                )}
              </td>
              <td className="px-3 py-2">
                <ProcessingBadge status={doc.processing_status} />
              </td>
              <td className="px-3 py-2">
                <StoragePill status={doc.storage_status} />
              </td>
              <td className="px-3 py-2">
                <DocumentActions
                  actionBusy={actionBusy}
                  doc={doc}
                  matter={matter}
                  onArchive={onArchive}
                  onEdit={onEdit}
                  onIndex={onIndex}
                  onRestore={onRestore}
                  onStartTranscription={onStartTranscription}
                  onToggleSelected={onToggleSelected}
                />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function UploadPreviewDialog({
  rows,
  uploading,
  onCancel,
  onRemove,
  onUpload,
}: {
  rows: UploadPreviewRow[]
  uploading: boolean
  onCancel: () => void
  onRemove: (index: number) => void
  onUpload: () => void
}) {
  const grouped = groupUploadRows(rows)
  const conflicts = rows.filter((row) => row.conflict !== "none").length
  const unsupported = rows.filter((row) => row.status === "unsupported").length
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 p-6 backdrop-blur-sm">
      <div className="flex max-h-[82vh] w-full max-w-4xl flex-col rounded border border-border bg-card shadow-xl">
        <div className="flex items-start justify-between gap-3 border-b border-border px-4 py-3">
          <div>
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">upload tree preview</div>
            <h2 className="mt-1 text-sm font-semibold text-foreground">{rows.length} item{rows.length === 1 ? "" : "s"} ready to add</h2>
            <p className="mt-1 text-xs text-muted-foreground">
              {grouped.length} folder group{grouped.length === 1 ? "" : "s"} · {conflicts} path conflict{conflicts === 1 ? "" : "s"} · {unsupported} unsupported candidate{unsupported === 1 ? "" : "s"}
            </p>
          </div>
          <button type="button" onClick={onCancel} className="rounded border border-border p-1 text-muted-foreground hover:bg-muted" aria-label="Close upload preview">
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-4">
          <div className="space-y-3">
            {grouped.map((group) => (
              <div key={group.folder} className="rounded border border-border bg-background">
                <div className="flex items-center justify-between border-b border-border px-3 py-2">
                  <div className="flex min-w-0 items-center gap-2 font-mono text-[11px] text-foreground">
                    <Folder className="h-3.5 w-3.5 text-primary" />
                    <span className="truncate">{group.folder}</span>
                  </div>
                  <span className="font-mono text-[10px] text-muted-foreground">{group.rows.length} files</span>
                </div>
                <div className="divide-y divide-border">
                  {group.rows.map(({ row, index }) => (
                    <div key={row.id} className="grid grid-cols-[1fr_auto_auto_auto] items-center gap-2 px-3 py-2 text-xs">
                      <span className="min-w-0 truncate font-mono" title={row.relativePath}>{row.relativePath}</span>
                      <UploadStatusPill status={row.status} />
                      <ConflictPill conflict={row.conflict} />
                      <button
                        type="button"
                        onClick={() => onRemove(index)}
                        className="rounded border border-border px-1.5 py-0.5 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted"
                      >
                        remove
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
        <div className="flex items-center justify-between gap-3 border-t border-border px-4 py-3">
          <p className="text-xs text-muted-foreground">Files are not uploaded until you confirm this preview.</p>
          <div className="flex items-center gap-2">
            <button type="button" onClick={onCancel} className="rounded border border-border px-3 py-1.5 font-mono text-xs uppercase text-muted-foreground hover:bg-muted">
              cancel
            </button>
            <button
              type="button"
              onClick={onUpload}
              disabled={uploading || rows.length === 0}
              className="rounded bg-primary px-3 py-1.5 font-mono text-xs uppercase text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              upload batch
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

function DocumentEditDialog({
  busy,
  document,
  onCancel,
  onSave,
}: {
  busy: boolean
  document: CaseDocument
  onCancel: () => void
  onSave: (values: {
    title: string
    libraryPath: string
    documentType: string
    confidentiality: string
    isExhibit: boolean
    exhibitLabel: string
    dateObserved: string
  }) => void
}) {
  const [title, setTitle] = useState(document.title || document.filename)
  const [libraryPath, setLibraryPath] = useState(documentLibraryPath(document))
  const [documentType, setDocumentType] = useState<DocumentType>(document.document_type)
  const [confidentiality, setConfidentiality] = useState(document.confidentiality || "private")
  const [isExhibit, setIsExhibit] = useState(document.is_exhibit)
  const [exhibitLabel, setExhibitLabel] = useState(document.exhibit_label || "")
  const [dateObserved, setDateObserved] = useState(document.date_observed || "")
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 p-6 backdrop-blur-sm">
      <div className="w-full max-w-xl rounded border border-border bg-card shadow-xl">
        <div className="flex items-start justify-between gap-3 border-b border-border px-4 py-3">
          <div>
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">manage document</div>
            <h2 className="mt-1 text-sm font-semibold text-foreground">{document.filename}</h2>
          </div>
          <button type="button" onClick={onCancel} className="rounded border border-border p-1 text-muted-foreground hover:bg-muted" aria-label="Close document manager">
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="grid gap-3 p-4">
          <Field label="title">
            <input value={title} onChange={(event) => setTitle(event.target.value)} className="h-9 rounded border border-border bg-background px-2 text-sm" />
          </Field>
          <Field label="tree path">
            <input value={libraryPath} onChange={(event) => setLibraryPath(event.target.value)} className="h-9 rounded border border-border bg-background px-2 font-mono text-xs" />
          </Field>
          <div className="grid gap-3 sm:grid-cols-2">
            <Field label="type">
              <select value={documentType} onChange={(event) => setDocumentType(event.target.value as DocumentType)} className="h-9 rounded border border-border bg-background px-2 text-sm">
                {Object.entries(TYPE_LABEL).map(([value, label]) => (
                  <option key={value} value={value}>{label}</option>
                ))}
              </select>
            </Field>
            <Field label="confidentiality">
              <select value={confidentiality} onChange={(event) => setConfidentiality(event.target.value)} className="h-9 rounded border border-border bg-background px-2 text-sm">
                <option value="private">Private</option>
                <option value="filed">Filed</option>
                <option value="public">Public</option>
                <option value="sealed">Sealed</option>
              </select>
            </Field>
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <Field label="date observed">
              <input value={dateObserved} onChange={(event) => setDateObserved(event.target.value)} className="h-9 rounded border border-border bg-background px-2 text-sm" placeholder="YYYY-MM-DD or note" />
            </Field>
            <label className="flex items-center gap-2 pt-6 text-xs text-muted-foreground">
              <input type="checkbox" checked={isExhibit} onChange={(event) => setIsExhibit(event.target.checked)} />
              mark as exhibit
            </label>
          </div>
          <Field label="exhibit label">
            <input value={exhibitLabel} onChange={(event) => setExhibitLabel(event.target.value)} className="h-9 rounded border border-border bg-background px-2 text-sm" />
          </Field>
        </div>
        <div className="flex items-center justify-end gap-2 border-t border-border px-4 py-3">
          <button type="button" onClick={onCancel} className="rounded border border-border px-3 py-1.5 font-mono text-xs uppercase text-muted-foreground hover:bg-muted">
            cancel
          </button>
          <button
            type="button"
            onClick={() => onSave({ title, libraryPath, documentType, confidentiality, isExhibit, exhibitLabel, dateObserved })}
            disabled={busy || !libraryPath.trim() || !title.trim()}
            className="rounded bg-primary px-3 py-1.5 font-mono text-xs uppercase text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            save
          </button>
        </div>
      </div>
    </div>
  )
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{label}</span>
      {children}
    </label>
  )
}

function StoragePill({ status }: { status?: CaseDocument["storage_status"] }) {
  const value = status ?? "stored"
  const cls =
    value === "stored"
      ? "bg-success/15 text-success"
      : value === "pending"
        ? "bg-primary/15 text-primary"
        : value === "failed" || value === "deleted"
          ? "bg-destructive/15 text-destructive"
          : "bg-muted text-muted-foreground"
  return (
    <span className={cn("rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider", cls)}>
      {value === "metadata_only" ? "metadata" : value}
    </span>
  )
}

function TranscriptStatusPill({ status }: { status: string }) {
  const cls =
    status === "processed"
      ? "bg-success/15 text-success"
      : status === "review_ready"
        ? "bg-primary/15 text-primary"
        : status === "failed" || status === "provider_disabled"
          ? "bg-destructive/15 text-destructive"
          : status === "not_started"
            ? "bg-muted text-muted-foreground"
            : "bg-warning/15 text-warning"
  return (
    <span className={cn("rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider", cls)}>
      {status === "provider_disabled" ? "disabled" : status === "not_started" ? "not started" : status}
    </span>
  )
}

function UploadStatusPill({ status }: { status: UploadPreviewRow["status"] }) {
  const cls =
    status === "ready"
      ? "bg-success/15 text-success"
      : status === "media"
        ? "bg-primary/15 text-primary"
        : status === "ocr"
          ? "bg-warning/15 text-warning"
          : "bg-destructive/15 text-destructive"
  return <span className={cn("rounded px-1.5 py-0.5 font-mono text-[10px] uppercase", cls)}>{status}</span>
}

function ConflictPill({ conflict }: { conflict: UploadPreviewRow["conflict"] }) {
  if (conflict === "none") {
    return <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase text-muted-foreground">new</span>
  }
  const label = conflict === "existing_path" ? "path exists" : "duplicate path"
  return <span className="rounded bg-warning/15 px-1.5 py-0.5 font-mono text-[10px] uppercase text-warning">{label}</span>
}

function latestTranscription(transcriptions: TranscriptionJobResponse[]) {
  if (!transcriptions.length) return null
  return [...transcriptions].sort((a, b) => b.job.created_at.localeCompare(a.job.created_at))[0]
}

function replaceLatestTranscription(
  transcriptions: TranscriptionJobResponse[],
  transcription: TranscriptionJobResponse,
) {
  return [
    ...transcriptions.filter((item) => item.job.transcription_job_id !== transcription.job.transcription_job_id),
    transcription,
  ].sort((a, b) => a.job.created_at.localeCompare(b.job.created_at))
}

function expandUploadAncestors(current: Set<string>, candidates: UploadCandidate[]) {
  const next = new Set(current)
  for (const candidate of candidates) {
    expandPath(next, candidate.relativePath)
  }
  return next
}

function expandPath(current: Set<string>, libraryPath: string) {
  const next = new Set(current)
  const parts = libraryPath.split("/").slice(0, -1)
  for (let index = 0; index < parts.length; index += 1) {
    next.add(parts.slice(0, index + 1).join("/"))
  }
  return next
}

function withoutId(current: Set<string>, id: string) {
  const next = new Set(current)
  next.delete(id)
  return next
}

function groupUploadRows(rows: UploadPreviewRow[]) {
  const map = new Map<string, Array<{ row: UploadPreviewRow; index: number }>>()
  rows.forEach((row, index) => {
    const group = map.get(row.folder) ?? []
    group.push({ row, index })
    map.set(row.folder, group)
  })
  return Array.from(map.entries())
    .sort(([a], [b]) => a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" }))
    .map(([folder, groupRows]) => ({ folder, rows: groupRows }))
}
