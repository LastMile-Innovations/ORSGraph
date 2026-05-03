"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { useEffect, useMemo, useRef, useState } from "react"
import {
  AlertTriangle,
  BarChart3,
  CheckCircle2,
  FileText,
  Filter,
  Folder,
  FolderUp,
  Grid2x2,
  List,
  Mic,
  RefreshCcw,
  Search,
  Sparkles,
  Upload,
} from "lucide-react"
import { cn } from "@/lib/utils"
import type { CaseDocument, DocumentType, MatterIndexSummary, MatterSummary, TranscriptionJobResponse } from "@/lib/casebuilder/types"
import {
  createTranscription,
  getMatterIndexSummary,
  importDocumentComplaint,
  listTranscriptions,
  runMatterIndex,
  syncTranscription,
  uploadBinaryFile,
} from "@/lib/casebuilder/api"
import { matterComplaintHref, matterDocumentHref } from "@/lib/casebuilder/routes"
import {
  createUploadBatchId,
  dataTransferToUploadCandidates,
  filesToUploadCandidates,
  type UploadCandidate,
} from "@/lib/casebuilder/upload-folders"
import { ProcessingBadge } from "./badges"

const FOLDERS = [
  "Pleadings",
  "Evidence",
  "Correspondence",
  "Contracts",
  "Notices",
  "Court Orders",
  "Public Records",
  "Research",
  "Drafts",
  "Inbox",
] as const

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

type UploadQueueStatus = "queued" | "uploading" | "indexing" | "indexed" | "stored" | "failed"

interface UploadQueueRow {
  id: string
  candidate: UploadCandidate
  status: UploadQueueStatus
  message: string
  documentId?: string
}

export function DocumentLibrary({ matter, documents }: Props) {
  const router = useRouter()
  const fileInputRef = useRef<HTMLInputElement>(null)
  const folderInputRef = useRef<HTMLInputElement>(null)
  const [folder, setFolder] = useState<string>("All")
  const [query, setQuery] = useState("")
  const [view, setView] = useState<"grid" | "list">("grid")
  const [processingFilter, setProcessingFilter] = useState<string>("All")
  const [storageFilter, setStorageFilter] = useState<string>("All")
  const [batchFilter, setBatchFilter] = useState<string>("All")
  const [duplicateOnly, setDuplicateOnly] = useState(false)
  const [uploading, setUploading] = useState(false)
  const [uploadMessage, setUploadMessage] = useState<string | null>(null)
  const [uploadError, setUploadError] = useState<string | null>(null)
  const [importedComplaints, setImportedComplaints] = useState<Array<{ title: string; href: string }>>([])
  const [uploadRows, setUploadRows] = useState<UploadQueueRow[]>([])
  const [indexSummary, setIndexSummary] = useState<MatterIndexSummary | null>(null)
  const [indexMessage, setIndexMessage] = useState<string | null>(null)
  const [indexBusy, setIndexBusy] = useState(false)
  const [mediaTranscriptions, setMediaTranscriptions] = useState<Record<string, TranscriptionJobResponse[]>>({})
  const [mediaActionBusy, setMediaActionBusy] = useState<string | null>(null)
  const [mediaMessage, setMediaMessage] = useState<string | null>(null)

  const folderCounts = useMemo(() => {
    const map: Record<string, number> = { All: documents.length }
    for (const f of FOLDERS) map[f] = 0
    for (const d of documents) map[d.folder] = (map[d.folder] ?? 0) + 1
    map["Media queue"] = documents.filter(isMediaDocument).length
    return map
  }, [documents])

  const duplicateGroups = useMemo(() => {
    const byHash = new Map<string, number>()
    for (const document of documents) {
      if (!document.file_hash) continue
      byHash.set(document.file_hash, (byHash.get(document.file_hash) ?? 0) + 1)
    }
    return Array.from(byHash.values()).filter((count) => count > 1).length
  }, [documents])

  const duplicateHashes = useMemo(() => {
    const byHash = new Map<string, number>()
    for (const document of documents) {
      if (!document.file_hash) continue
      byHash.set(document.file_hash, (byHash.get(document.file_hash) ?? 0) + 1)
    }
    return new Set(Array.from(byHash).filter(([, count]) => count > 1).map(([hash]) => hash))
  }, [documents])

  const folderNames = useMemo(() => {
    return Array.from(new Set([...FOLDERS, ...documents.map((document) => document.folder)])).filter(Boolean)
  }, [documents])

  const processingOptions = useMemo(() => {
    return Array.from(new Set(documents.map((document) => document.processing_status))).sort()
  }, [documents])

  const storageOptions = useMemo(() => {
    return Array.from(new Set(documents.map((document) => document.storage_status ?? "stored"))).sort()
  }, [documents])

  const batchOptions = useMemo(() => {
    return Array.from(new Set(documents.map((document) => document.upload_batch_id).filter(Boolean) as string[])).sort()
  }, [documents])

  useEffect(() => {
    void refreshIndexSummary()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [matter.matter_id, documents.length])

  const filtered = useMemo(() => {
    return documents.filter((d) => {
      if (folder === "Media queue") {
        if (!isMediaDocument(d)) return false
      } else if (folder !== "All" && d.folder !== folder) return false
      if (processingFilter !== "All" && d.processing_status !== processingFilter) return false
      if (storageFilter !== "All" && (d.storage_status ?? "stored") !== storageFilter) return false
      if (batchFilter !== "All" && (d.upload_batch_id ?? "No batch") !== batchFilter) return false
      if (duplicateOnly && (!d.file_hash || !duplicateHashes.has(d.file_hash))) return false
      if (query.trim()) {
        const q = query.toLowerCase()
        const hay = `${d.filename} ${d.original_relative_path ?? ""} ${d.summary} ${d.parties_mentioned.join(" ")} ${d.entities_mentioned.join(" ")}`.toLowerCase()
        if (!hay.includes(q)) return false
      }
      return true
    })
  }, [batchFilter, documents, duplicateHashes, duplicateOnly, folder, processingFilter, query, storageFilter])

  const mediaDocuments = useMemo(() => documents.filter(isMediaDocument), [documents])

  useEffect(() => {
    if (folder !== "Media queue" || mediaDocuments.length === 0) return
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
  }, [folder, matter.matter_id, mediaDocuments])

  async function refreshIndexSummary() {
    const result = await getMatterIndexSummary(matter.matter_id)
    if (result.data) setIndexSummary(result.data)
  }

  async function indexDocuments(documentIds?: string[]) {
    setIndexBusy(true)
    setIndexMessage(null)
    const result = await runMatterIndex(matter.matter_id, {
      document_ids: documentIds && documentIds.length > 0 ? documentIds : undefined,
    })
    setIndexBusy(false)
    if (!result.data) {
      setIndexMessage(result.error || "Index run failed.")
      return null
    }
    setIndexSummary(result.data.summary)
    setIndexMessage(
      `${result.data.processed} indexed${result.data.skipped ? `, ${result.data.skipped} skipped` : ""}${result.data.failed ? `, ${result.data.failed} failed` : ""}.`,
    )
    return result.data
  }

  async function uploadFiles(files: FileList | File[]) {
    await uploadCandidates(filesToUploadCandidates(files))
  }

  async function uploadCandidates(candidates: UploadCandidate[]) {
    if (candidates.length === 0) return

    setUploading(true)
    setUploadMessage(null)
    setUploadError(null)
    setIndexMessage(null)

    let stored = 0
    let binaryStored = 0
    let imported = 0
    const importedLinks: Array<{ title: string; href: string }> = []
    const failures: string[] = []
    const uploadedDocumentIds: string[] = []
    const uploadBatchId = createUploadBatchId(candidates.some((item) => item.relativePath.includes("/")) ? "folder" : "batch")
    setUploadRows(
      candidates.map((candidate, index) => ({
        id: `${uploadBatchId}:${index}`,
        candidate,
        status: "queued",
        message: candidate.relativePath,
      })),
    )

    for (const [index, candidate] of candidates.entries()) {
      const file = candidate.file
      const rowId = `${uploadBatchId}:${index}`
      try {
        const mimeType = file.type || guessMimeType(file.name)
        const documentType = guessDocumentType(file.name, mimeType)
        setUploadRows((rows) =>
          rows.map((row) => (row.id === rowId ? { ...row, status: "uploading", message: "Uploading" } : row)),
        )
        const result = await uploadBinaryFile(matter.matter_id, file, {
          document_type: documentType,
          folder: candidate.folder,
          confidentiality: "private",
          relative_path: candidate.relativePath,
          upload_batch_id: uploadBatchId,
        })

        if (!result.data) {
          failures.push(`${file.name}: ${result.error || "upload failed"}`)
          setUploadRows((rows) =>
            rows.map((row) =>
              row.id === rowId ? { ...row, status: "failed", message: result.error || "Upload failed" } : row,
            ),
          )
          continue
        }

        stored += 1
        uploadedDocumentIds.push(result.data.document_id)
        setUploadRows((rows) =>
          rows.map((row) =>
            row.id === rowId
              ? { ...row, status: "indexing", documentId: result.data?.document_id, message: "Queued for indexing" }
              : row,
          ),
        )
        if (result.data.storage_status === "stored") {
          binaryStored += 1
        }
        if (shouldImportAsComplaint(file.name, documentType)) {
          const importedComplaint = await importDocumentComplaint(matter.matter_id, result.data.document_id, {
            force: true,
            mode: "structured_import",
          })
          const complaint = importedComplaint.data?.imported[0]?.complaint
          if (complaint) {
            imported += 1
            importedLinks.push({
              title: complaint.title || file.name,
              href: matterComplaintHref(matter.matter_id, "editor", {
                type: "complaint",
                id: complaint.complaint_id,
              }),
            })
          } else if (importedComplaint.data?.skipped[0]) {
            failures.push(`${file.name}: ${importedComplaint.data.skipped[0].message}`)
          } else if (importedComplaint.error) {
            failures.push(`${file.name}: ${importedComplaint.error}`)
          }
        }
      } catch (error) {
        failures.push(`${file.name}: ${error instanceof Error ? error.message : String(error)}`)
        setUploadRows((rows) =>
          rows.map((row) =>
            row.id === rowId
              ? { ...row, status: "failed", message: error instanceof Error ? error.message : String(error) }
              : row,
          ),
        )
      }
    }

    const indexRun = uploadedDocumentIds.length ? await indexDocuments(uploadedDocumentIds) : null
    if (indexRun) {
      const byDocument = new Map(indexRun.results.map((result) => [result.document_id, result]))
      setUploadRows((rows) =>
        rows.map((row) => {
          const result = row.documentId ? byDocument.get(row.documentId) : null
          if (!result) return row.status === "failed" ? row : { ...row, status: "stored", message: "Stored privately" }
          return {
            ...row,
            status: result.status === "indexed" ? "indexed" : result.status === "failed" ? "failed" : "stored",
            message: result.message,
          }
        }),
      )
    }

    setUploading(false)
    setImportedComplaints(importedLinks)
    if (failures.length > 0) {
      setUploadError(failures.join(" | "))
    }
    setUploadMessage(
      `${stored} uploaded${indexRun?.processed ? `, ${indexRun.processed} indexed` : ""}${binaryStored ? `, ${binaryStored} stored privately` : ""}${imported ? `, ${imported} opened as structured complaint` : ""}.`,
    )
    router.refresh()
  }

  async function retryUpload(row: UploadQueueRow) {
    await uploadCandidates([row.candidate])
  }

  async function startMediaTranscription(document: CaseDocument, force = false) {
    setMediaActionBusy(`${document.document_id}:transcribe`)
    setMediaMessage(null)
    const result = await createTranscription(matter.matter_id, document.document_id, {
      force,
      redact_pii: true,
      speaker_labels: true,
      remove_audio_tags: "all",
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

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {/* Header */}
      <div className="border-b border-border bg-card px-6 py-4">
        <div className="flex items-end justify-between gap-3">
          <div>
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              CaseBuilder · documents
            </div>
            <h1 className="mt-1 text-base font-semibold text-foreground">Document library</h1>
            <p className="mt-0.5 text-xs text-muted-foreground">
              {documents.length} files · {documents.reduce((s, d) => s + d.facts_extracted, 0)} facts extracted ·{" "}
              {documents.reduce((s, d) => s + d.contradictions_flagged, 0)} contradictions flagged
            </p>
          </div>
          <input
            ref={fileInputRef}
            type="file"
            multiple
            hidden
            onChange={(event) => {
              if (event.target.files) void uploadFiles(event.target.files)
              event.currentTarget.value = ""
            }}
          />
          <input
            ref={folderInputRef}
            type="file"
            multiple
            hidden
            // Browser folder selection is still exposed through vendor-prefixed attributes.
            {...({ webkitdirectory: "", directory: "" } as Record<string, string>)}
            onChange={(event) => {
              if (event.target.files) void uploadFiles(event.target.files)
              event.currentTarget.value = ""
            }}
          />
          <div className="flex items-center gap-2">
            <button
              onClick={() => folderInputRef.current?.click()}
              disabled={uploading}
              className="flex items-center gap-1.5 rounded border border-border bg-background px-3 py-1.5 font-mono text-xs uppercase tracking-wider text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-60"
            >
              <FolderUp className="h-3.5 w-3.5" />
              folder
            </button>
            <button
              onClick={() => fileInputRef.current?.click()}
              disabled={uploading}
              className="flex items-center gap-1.5 rounded bg-primary px-3 py-1.5 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
            >
              <Upload className="h-3.5 w-3.5" />
              {uploading ? "uploading" : "upload files"}
            </button>
          </div>
        </div>
        {(uploadMessage || uploadError) && (
          <div
            className={cn(
              "mt-3 rounded border px-3 py-2 text-xs",
              uploadError
                ? "border-destructive/30 bg-destructive/5 text-destructive"
                : "border-primary/20 bg-primary/5 text-muted-foreground",
            )}
          >
            {uploadError || uploadMessage}
            {importedComplaints.length > 0 && (
              <div className="mt-2 flex flex-wrap gap-2">
                {importedComplaints.map((item) => (
                  <Link
                    key={item.href}
                    href={item.href}
                    title={item.title}
                    className="inline-flex items-center gap-1 rounded border border-primary/25 bg-background px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-primary hover:bg-primary/10"
                  >
                    <FileText className="h-3 w-3" />
                    opened as structured complaint
                  </Link>
                ))}
              </div>
            )}
          </div>
        )}
        <div className="mt-3 grid gap-2 lg:grid-cols-[1.3fr_1fr]">
          <div className="rounded border border-border bg-background p-3">
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                <BarChart3 className="h-3.5 w-3.5" />
                index console
              </div>
              <button
                type="button"
                onClick={() => void indexDocuments()}
                disabled={indexBusy || (indexSummary?.extractable_pending_documents ?? 0) === 0}
                className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
              >
                <RefreshCcw className={cn("h-3 w-3", indexBusy && "animate-spin")} />
                reindex pending
              </button>
            </div>
            <div className="grid grid-cols-3 gap-2 text-center md:grid-cols-6">
              <IndexTile label="indexed" value={indexSummary?.indexed_documents ?? documents.filter((d) => d.facts_extracted > 0 || (d.source_spans?.length ?? 0) > 0).length} />
              <IndexTile label="pending" value={indexSummary?.pending_documents ?? documents.filter((d) => d.processing_status === "queued").length} />
              <IndexTile label="extractable" value={indexSummary?.extractable_pending_documents ?? 0} />
              <IndexTile label="failed" value={indexSummary?.failed_documents ?? documents.filter((d) => d.processing_status === "failed").length} tone="bad" />
              <IndexTile label="ocr" value={indexSummary?.ocr_required_documents ?? documents.filter((d) => d.processing_status === "ocr_required").length} tone="warn" />
              <IndexTile label="duplicates" value={indexSummary?.duplicate_groups.length ?? duplicateGroups} tone={duplicateGroups ? "warn" : undefined} />
            </div>
            {(indexMessage || indexSummary?.recent_ingestion_runs[0]) && (
              <div className="mt-2 truncate font-mono text-[10px] text-muted-foreground">
                {indexMessage ??
                  `last run ${indexSummary?.recent_ingestion_runs[0]?.stage ?? "stored"} · ${indexSummary?.recent_ingestion_runs[0]?.status ?? "queued"}`}
              </div>
            )}
          </div>
          {uploadRows.length > 0 && (
            <div className="rounded border border-border bg-background p-3">
              <div className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                upload batch
              </div>
              <div className="max-h-28 space-y-1 overflow-y-auto pr-1">
                {uploadRows.map((row) => (
                  <div key={row.id} className="flex items-center gap-2 rounded border border-border px-2 py-1 text-[11px]">
                    <span className={cn("h-2 w-2 rounded-full", uploadStatusClass(row.status))} />
                    <span className="min-w-0 flex-1 truncate font-mono">{row.candidate.relativePath}</span>
                    <span className="shrink-0 text-muted-foreground">{row.status}</span>
                    {row.status === "failed" && (
                      <button
                        type="button"
                        onClick={() => void retryUpload(row)}
                        className="shrink-0 rounded border border-border px-1.5 py-0.5 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted"
                      >
                        retry
                      </button>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Toolbar */}
      <div className="flex flex-wrap items-center gap-2 border-b border-border px-6 py-2">
        <div className="flex flex-1 items-center gap-2 rounded border border-border bg-background px-2.5">
          <Search className="h-3.5 w-3.5 text-muted-foreground" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search filenames, folder paths, summaries, parties, entities…"
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
        <div className="flex items-center gap-1">
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
      </div>

      {/* Body: folders + grid */}
      <div className="flex flex-1 overflow-hidden">
        {/* Folders rail */}
        <aside className="w-52 flex-shrink-0 overflow-y-auto border-r border-border bg-background">
          <div className="px-3 py-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            folders
          </div>
          <div className="space-y-px px-1 pb-3">
            <FolderItem name="All" count={folderCounts.All} active={folder === "All"} onClick={() => setFolder("All")} />
            <FolderItem
              name="Media queue"
              count={folderCounts["Media queue"]}
              active={folder === "Media queue"}
              onClick={() => setFolder("Media queue")}
            />
            {folderNames.map((f) => (
              <FolderItem
                key={f}
                name={f}
                count={folderCounts[f] ?? 0}
                active={folder === f}
                onClick={() => setFolder(f)}
              />
            ))}
          </div>

          <div className="border-t border-border px-3 py-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            extraction status
          </div>
          <div className="space-y-1 px-3 pb-3 font-mono text-[11px]">
            <KV label="processed" value={documents.filter((d) => d.processing_status === "processed").length} cls="text-success" />
            <KV label="processing" value={documents.filter((d) => d.processing_status === "processing").length} cls="text-primary" />
            <KV label="review ready" value={documents.filter((d) => d.processing_status === "review_ready").length} cls="text-primary" />
            <KV label="queued" value={documents.filter((d) => d.processing_status === "queued").length} cls="text-muted-foreground" />
            <KV label="ocr needed" value={documents.filter((d) => d.processing_status === "ocr_required").length} cls="text-warning" />
            <KV label="transcribe" value={documents.filter((d) => d.processing_status === "transcription_deferred").length} cls="text-warning" />
            <KV label="unsupported" value={documents.filter((d) => d.processing_status === "unsupported").length} cls="text-warning" />
            <KV label="failed" value={documents.filter((d) => d.processing_status === "failed").length} cls="text-destructive" />
          </div>
          <div className="border-t border-border px-3 py-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            storage status
          </div>
          <div className="space-y-1 px-3 pb-3 font-mono text-[11px]">
            <KV label="stored" value={documents.filter((d) => d.storage_status === "stored").length} cls="text-success" />
            <KV label="pending" value={documents.filter((d) => d.storage_status === "pending").length} cls="text-primary" />
            <KV label="metadata" value={documents.filter((d) => d.storage_status === "metadata_only").length} cls="text-muted-foreground" />
            <KV label="failed" value={documents.filter((d) => d.storage_status === "failed").length} cls="text-destructive" />
          </div>
          <div className="border-t border-border px-3 py-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            duplicate groups
          </div>
          <div className="space-y-1 px-3 pb-3 font-mono text-[11px]">
            <KV label="exact hash" value={duplicateGroups} cls={duplicateGroups ? "text-warning" : "text-muted-foreground"} />
          </div>
        </aside>

        {/* Grid */}
        <div
          className="flex-1 overflow-y-auto p-4 scrollbar-thin"
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => {
            event.preventDefault()
            void dataTransferToUploadCandidates(event.dataTransfer).then(uploadCandidates)
          }}
        >
          {folder === "Media queue" ? (
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
                Try clearing your filter or upload files to start extracting facts and citations.
              </p>
            </div>
          ) : view === "grid" ? (
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
              {filtered.map((d) => (
                <DocCard key={d.document_id} doc={d} matter={matter} />
              ))}
            </div>
          ) : (
            <DocList docs={filtered} matter={matter} />
          )}
        </div>
      </div>
    </div>
  )
}

function FolderItem({
  name,
  count,
  active,
  onClick,
}: {
  name: string
  count: number
  active: boolean
  onClick: () => void
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "flex w-full items-center justify-between rounded px-2 py-1 text-left text-xs transition-colors",
        active ? "bg-primary/10 text-primary" : "text-foreground hover:bg-muted",
      )}
    >
      <span className="flex items-center gap-1.5">
        <Folder className="h-3 w-3" />
        {name}
      </span>
      <span className="font-mono text-[10px] tabular-nums text-muted-foreground">{count}</span>
    </button>
  )
}

function KV({ label, value, cls }: { label: string; value: number; cls: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="uppercase tracking-wider text-muted-foreground">{label}</span>
      <span className={cn("tabular-nums", cls)}>{value}</span>
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

function uploadStatusClass(status: UploadQueueStatus) {
  if (status === "indexed") return "bg-success"
  if (status === "failed") return "bg-destructive"
  if (status === "uploading" || status === "indexing") return "bg-primary"
  return "bg-muted-foreground"
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
              {documents.length} media file{documents.length === 1 ? "" : "s"} awaiting transcription, sync, or redacted review.
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
                const canStart = !latest
                const canRetry = Boolean(latest && (latest.job.retryable || latest.job.status === "failed" || latest.job.status === "provider_disabled"))
                const canReview = Boolean(latest && latest.segments.length > 0)
                const documentHref = matterDocumentHref(matter.matter_id, document.document_id)
                return (
                  <tr key={document.document_id} className="border-b border-border hover:bg-muted/20">
                    <td className="px-3 py-2">
                      <Link href={documentHref} className="block font-mono text-foreground hover:text-primary">
                        <span className="block truncate">{document.filename}</span>
                        {document.original_relative_path && document.original_relative_path !== document.filename && (
                          <span className="block truncate text-[10px] text-muted-foreground">{document.original_relative_path}</span>
                        )}
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
                          disabled={!(canStart || canRetry) || busy === `${document.document_id}:transcribe`}
                          className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
                        >
                          <Mic className="h-3 w-3" />
                          {latest ? "retry" : "transcribe"}
                        </button>
                        <button
                          type="button"
                          onClick={() => latest && void onSync(document, latest)}
                          disabled={!latest || busy === `${document.document_id}:sync`}
                          className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
                        >
                          <RefreshCcw className="h-3 w-3" />
                          sync
                        </button>
                        <Link
                          href={documentHref}
                          aria-disabled={!canReview}
                          className={cn(
                            "inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:bg-muted",
                            !canReview && "pointer-events-none opacity-50",
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

function DocCard({ doc, matter }: { doc: CaseDocument; matter: MatterSummary }) {
  return (
    <Link
      href={matterDocumentHref(matter.matter_id, doc.document_id)}
      className="group flex flex-col gap-2 rounded border border-border bg-card p-3 hover:border-primary/40"
    >
      <div className="flex items-start gap-2">
        <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded bg-primary/10 text-primary">
          <FileText className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate font-mono text-[11px] text-foreground">{doc.filename}</div>
          {doc.original_relative_path && doc.original_relative_path !== doc.filename && (
            <div className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground">{doc.original_relative_path}</div>
          )}
          <div className="mt-0.5 flex items-center gap-2 font-mono text-[10px] tabular-nums uppercase tracking-wider text-muted-foreground">
            <span>{TYPE_LABEL[doc.document_type]}</span>
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
        <Tile
          label="flags"
          value={doc.contradictions_flagged}
          tone={doc.contradictions_flagged > 0 ? "warn" : undefined}
        />
      </div>

      <div className="flex items-center justify-between border-t border-border pt-2">
        <ProcessingBadge status={doc.processing_status} />
        <StoragePill status={doc.storage_status} />
        <div className="flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground group-hover:text-primary">
          <Sparkles className="h-3 w-3" />
          inspect
        </div>
      </div>
    </Link>
  )
}

function Tile({ label, value, tone }: { label: string; value: number; tone?: "warn" }) {
  return (
    <div
      className={cn(
        "rounded border border-border bg-background py-1",
        tone === "warn" && "border-warning/40 bg-warning/5",
      )}
    >
      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div
        className={cn(
          "font-mono text-sm font-semibold tabular-nums",
          tone === "warn" ? "text-warning" : "text-foreground",
        )}
      >
        {value}
      </div>
    </div>
  )
}

function DocList({ docs, matter }: { docs: CaseDocument[]; matter: MatterSummary }) {
  return (
    <div className="overflow-x-auto rounded border border-border bg-card">
      <table className="w-full text-xs">
        <thead className="border-b border-border bg-muted/40 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
          <tr>
            <th className="px-3 py-2 text-left">filename</th>
            <th className="px-3 py-2 text-left">type</th>
            <th className="px-3 py-2 text-left">folder</th>
            <th className="px-3 py-2 text-left">date</th>
            <th className="px-3 py-2 text-right">pages</th>
            <th className="px-3 py-2 text-right">facts</th>
            <th className="px-3 py-2 text-right">cites</th>
            <th className="px-3 py-2 text-right">flags</th>
            <th className="px-3 py-2 text-left">status</th>
            <th className="px-3 py-2 text-left">storage</th>
          </tr>
        </thead>
        <tbody>
          {docs.map((d) => (
            <tr key={d.document_id} className="border-b border-border hover:bg-muted/20">
              <td className="px-3 py-2">
                <Link
                  href={matterDocumentHref(matter.matter_id, d.document_id)}
                  className="flex items-center gap-2 font-mono text-foreground hover:text-primary"
                >
                  <FileText className="h-3.5 w-3.5 text-muted-foreground" />
                  <span className="min-w-0">
                    <span className="block truncate">{d.filename}</span>
                    {d.original_relative_path && d.original_relative_path !== d.filename && (
                      <span className="block truncate text-[10px] text-muted-foreground">{d.original_relative_path}</span>
                    )}
                  </span>
                  {d.is_exhibit && d.exhibit_label && (
                    <span className="rounded bg-accent/15 px-1 font-mono text-[10px] uppercase text-accent">
                      {d.exhibit_label}
                    </span>
                  )}
                </Link>
              </td>
              <td className="px-3 py-2 font-mono text-[11px] uppercase tracking-wider text-muted-foreground">
                {TYPE_LABEL[d.document_type]}
              </td>
              <td className="px-3 py-2 text-muted-foreground">{d.folder}</td>
              <td className="px-3 py-2 font-mono text-[11px] tabular-nums text-muted-foreground">
                {d.date_observed ?? "—"}
              </td>
              <td className="px-3 py-2 text-right font-mono tabular-nums">{d.pages}</td>
              <td className="px-3 py-2 text-right font-mono tabular-nums">{d.facts_extracted}</td>
              <td className="px-3 py-2 text-right font-mono tabular-nums">{d.citations_found}</td>
              <td
                className={cn(
                  "px-3 py-2 text-right font-mono tabular-nums",
                  d.contradictions_flagged > 0 ? "text-warning" : "text-muted-foreground",
                )}
              >
                {d.contradictions_flagged > 0 ? (
                  <span className="inline-flex items-center gap-1">
                    <AlertTriangle className="h-3 w-3" />
                    {d.contradictions_flagged}
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 text-success">
                    <CheckCircle2 className="h-3 w-3" />0
                  </span>
                )}
              </td>
              <td className="px-3 py-2">
                <ProcessingBadge status={d.processing_status} />
              </td>
              <td className="px-3 py-2">
                <StoragePill status={d.storage_status} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
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

function guessMimeType(filename: string) {
  if (/\.csv$/i.test(filename)) return "text/csv"
  if (/\.html?$/i.test(filename)) return "text/html"
  if (/\.json$/i.test(filename)) return "application/json"
  if (/\.pdf$/i.test(filename)) return "application/pdf"
  if (/\.docx?$/i.test(filename)) {
    return "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
  }
  if (/\.(png|jpe?g|gif|webp|heic)$/i.test(filename)) return "image/*"
  if (/\.(mp3|m4a|wav|aac|flac)$/i.test(filename)) return "audio/*"
  if (/\.(mp4|mov|m4v|webm)$/i.test(filename)) return "video/*"
  return "application/octet-stream"
}

function guessDocumentType(filename: string, mimeType: string): DocumentType {
  const lower = filename.toLowerCase()
  if (lower.includes("complaint")) return "complaint"
  if (lower.includes("answer")) return "answer"
  if (lower.includes("motion")) return "motion"
  if (lower.includes("notice")) return "notice"
  if (lower.includes("lease")) return "lease"
  if (lower.includes("contract")) return "contract"
  if (lower.includes("receipt")) return "receipt"
  if (lower.includes("invoice")) return "invoice"
  if (/\.csv$/i.test(filename)) return "spreadsheet"
  if (mimeType.startsWith("image/") || /\.(png|jpe?g|gif|webp|heic)$/i.test(filename)) return "photo"
  return "evidence"
}

function isMediaDocument(document: { filename: string; mime_type?: string; processing_status?: string }) {
  const filename = document.filename.toLowerCase()
  const mimeType = (document.mime_type ?? "").toLowerCase()
  return (
    mimeType.startsWith("audio/") ||
    mimeType.startsWith("video/") ||
    /\.(mp3|m4a|wav|aac|flac|mp4|mov|m4v|webm)$/i.test(filename) ||
    document.processing_status === "transcription_deferred" ||
    document.processing_status === "review_ready"
  )
}

function shouldImportAsComplaint(filename: string, documentType: DocumentType) {
  return documentType === "complaint" || /complaint|pleading|petition/i.test(filename)
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
