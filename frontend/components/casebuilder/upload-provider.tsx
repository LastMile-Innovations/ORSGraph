"use client"

/* eslint-disable react-hooks/exhaustive-deps */

import Link from "next/link"
import { useRouter } from "next/navigation"
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react"
import {
  AlertCircle,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  FileText,
  FolderUp,
  Loader2,
  RefreshCcw,
  X,
} from "lucide-react"
import { cn } from "@/lib/utils"
import {
  completeFileUpload,
  createFileUpload,
  createMatterIndexJob,
  getMatterIndexJob,
  importDocumentComplaint,
  putSignedUploadFile,
} from "@/lib/casebuilder/api"
import { isMarkdownIndexableFile } from "@/lib/casebuilder/document-tree"
import { matterComplaintHref, matterHref } from "@/lib/casebuilder/routes"
import { createUploadBatchId, type UploadCandidate } from "@/lib/casebuilder/upload-folders"
import type { CaseBuilderEffectiveSettings, DocumentType, MatterIndexJob } from "@/lib/casebuilder/types"

export type CaseBuilderUploadRowStatus =
  | "queued"
  | "preparing"
  | "uploading"
  | "stored"
  | "view_only"
  | "indexing"
  | "indexed"
  | "failed"
  | "canceled"

export interface CaseBuilderUploadRow {
  id: string
  batchId: string
  matterId: string
  file?: File
  relativePath: string
  folder: string
  status: CaseBuilderUploadRowStatus
  message: string
  bytes: number
  uploadedBytes?: number
  uploadSpeedBps?: number | null
  uploadStartedAt?: number
  uploadUpdatedAt?: number
  documentId?: string
  indexJobId?: string
  error?: string
  importedComplaintHref?: string
}

export interface CaseBuilderUploadBatch {
  id: string
  matterId: string
  uploadBatchId: string
  label: string
  status: "queued" | "uploading" | "indexing" | "done" | "failed" | "canceled"
  createdAt: number
  rowIds: string[]
  autoIndex?: boolean
  importComplaints?: boolean
  defaultConfidentiality?: string
  defaultDocumentType?: DocumentType
  indexJobId?: string
  message?: string
}

export interface EnqueueMatterUploadsOptions {
  label?: string
  uploadBatchId?: string
  autoIndex?: boolean
  importComplaints?: boolean
  defaultConfidentiality?: string
  defaultDocumentType?: DocumentType
}

interface MatterUploadProcessingOptions {
  autoIndex: boolean
  importComplaints: boolean
  defaultConfidentiality: string
  defaultDocumentType: DocumentType
}

export function uploadOptionsFromEffectiveSettings(
  settings?: CaseBuilderEffectiveSettings | null,
): EnqueueMatterUploadsOptions {
  if (!settings) return {}
  return {
    autoIndex: settings.auto_index_uploads,
    importComplaints: settings.auto_import_complaints,
    defaultConfidentiality: settings.default_confidentiality,
    defaultDocumentType: settings.default_document_type,
  }
}

interface EnqueueMatterIntakeInput extends EnqueueMatterUploadsOptions {
  storyText?: string
}

interface UploadContextValue {
  batches: CaseBuilderUploadBatch[]
  rows: CaseBuilderUploadRow[]
  activeCount: number
  enqueueMatterUploads: (
    matterId: string,
    candidates: UploadCandidate[],
    options?: EnqueueMatterUploadsOptions,
  ) => string | null
  enqueueMatterIntake: (
    matterId: string,
    candidates: UploadCandidate[],
    options?: EnqueueMatterIntakeInput,
  ) => string | null
  cancelRow: (rowId: string) => void
  retryRow: (rowId: string) => void
  dismissBatch: (batchId: string) => void
}

const CaseBuilderUploadContext = createContext<UploadContextValue | null>(null)
const UPLOAD_SNAPSHOT_KEY = "casebuilder.uploads.v1"

interface PersistedUploadSnapshot {
  batches: CaseBuilderUploadBatch[]
  rows: Omit<CaseBuilderUploadRow, "file">[]
}

function normalizeUploadProcessingOptions(options: EnqueueMatterUploadsOptions = {}): MatterUploadProcessingOptions {
  return {
    autoIndex: options.autoIndex ?? true,
    importComplaints: options.importComplaints ?? true,
    defaultConfidentiality: options.defaultConfidentiality?.trim() || "private",
    defaultDocumentType: options.defaultDocumentType ?? "evidence",
  }
}

export function CaseBuilderUploadProvider({ children }: { children: ReactNode }) {
  const router = useRouter()
  const [batches, setBatches] = useState<CaseBuilderUploadBatch[]>([])
  const [rows, setRows] = useState<CaseBuilderUploadRow[]>([])
  const [collapsed, setCollapsed] = useState(true)
  const [storageReady, setStorageReady] = useState(false)
  const batchesRef = useRef<CaseBuilderUploadBatch[]>([])
  const rowsRef = useRef<CaseBuilderUploadRow[]>([])
  const controllersRef = useRef(new Map<string, AbortController>())
  const canceledRowsRef = useRef(new Set<string>())

  useEffect(() => {
    batchesRef.current = batches
  }, [batches])

  useEffect(() => {
    rowsRef.current = rows
  }, [rows])

  const updateRow = useCallback((rowId: string, patch: Partial<CaseBuilderUploadRow>) => {
    setRows((current) => current.map((row) => (row.id === rowId ? { ...row, ...patch } : row)))
  }, [])

  const updateBatch = useCallback((batchId: string, patch: Partial<CaseBuilderUploadBatch>) => {
    setBatches((current) => current.map((batch) => (batch.id === batchId ? { ...batch, ...patch } : batch)))
  }, [])

  const enqueueRows = useCallback(
    (
      matterId: string,
      candidates: UploadCandidate[],
      options: EnqueueMatterUploadsOptions = {},
    ) => {
      if (candidates.length === 0) return null
      const uploadBatchId =
        options.uploadBatchId ??
        createUploadBatchId(candidates.some((candidate) => candidate.relativePath.includes("/")) ? "folder" : "batch")
      const batchId = uploadBatchId
      const processingOptions = normalizeUploadProcessingOptions(options)
      const nextRows = candidates.map((candidate, index): CaseBuilderUploadRow => ({
        id: `${batchId}:row:${index}`,
        batchId,
        matterId,
        file: candidate.file,
        relativePath: candidate.relativePath,
        folder: candidate.folder,
        status: "queued",
        message: "Queued",
        bytes: candidate.file.size,
        uploadedBytes: 0,
        uploadSpeedBps: null,
      }))
      const batch: CaseBuilderUploadBatch = {
        id: batchId,
        matterId,
        uploadBatchId,
        label: options.label ?? (uploadBatchId.startsWith("folder") ? "Folder upload" : "File upload"),
        status: "queued",
        createdAt: Date.now(),
        rowIds: nextRows.map((row) => row.id),
        autoIndex: processingOptions.autoIndex,
        importComplaints: processingOptions.importComplaints,
        defaultConfidentiality: processingOptions.defaultConfidentiality,
        defaultDocumentType: processingOptions.defaultDocumentType,
      }
      setBatches((current) => [batch, ...current])
      setRows((current) => [...nextRows, ...current])
      setCollapsed(false)
      void processBatch(batch, nextRows, processingOptions)
      return batchId
    },
    [],
  )

  const processBatch = useCallback(
    async (
      batch: CaseBuilderUploadBatch,
      batchRows: CaseBuilderUploadRow[],
      options: MatterUploadProcessingOptions,
    ) => {
      updateBatch(batch.id, { status: "uploading", message: "Uploading" })
      const indexableDocumentIds: string[] = []
      let failed = 0
      let stored = 0

      for (const row of batchRows) {
        if (canceledRowsRef.current.has(row.id)) continue
        const result = await uploadOneRow(row, batch.uploadBatchId, options)
        if (!result.ok) {
          failed += 1
          continue
        }
        stored += 1
        if (result.markdownIndexable && result.documentId) indexableDocumentIds.push(result.documentId)
      }

      router.refresh()

      if (indexableDocumentIds.length > 0 && options.autoIndex) {
        await startIndexJobForBatch(batch, indexableDocumentIds)
      } else {
        updateBatch(batch.id, {
          status: failed > 0 ? "failed" : "done",
          message: `${stored} stored${failed ? `, ${failed} failed` : ""}.`,
        })
      }
    },
    [router, updateBatch],
  )

  const uploadOneRow = useCallback(
    async (
      row: CaseBuilderUploadRow,
      uploadBatchId: string,
      options: MatterUploadProcessingOptions,
    ): Promise<{ ok: boolean; documentId?: string; markdownIndexable?: boolean }> => {
      const controller = new AbortController()
      controllersRef.current.set(row.id, controller)
      const file = row.file
      if (!file) {
        const message = "File selection was lost after refresh. Select this file or folder again to retry."
        updateRow(row.id, { status: "failed", message, error: message })
        return { ok: false }
      }
      const mimeType = file.type || guessMimeType(file.name)
      const documentType = guessDocumentType(file.name, mimeType, options.defaultDocumentType)
      const markdownIndexable = isMarkdownIndexableFile(file.name, mimeType)

      try {
        updateRow(row.id, { status: "preparing", message: "Preparing signed upload", error: undefined })
        const intent = await createFileUpload(row.matterId, {
          filename: file.name,
          mime_type: mimeType,
          bytes: file.size,
          document_type: documentType,
          folder: row.folder,
          confidentiality: options.defaultConfidentiality,
          relative_path: row.relativePath,
          upload_batch_id: uploadBatchId,
        })
        if (!intent.data) throw new Error(intent.error || "Could not create signed upload.")
        if (canceledRowsRef.current.has(row.id)) throw new DOMException("Upload canceled", "AbortError")

        updateRow(row.id, {
          status: "uploading",
          documentId: intent.data.document_id,
          message: "Uploading to private storage",
          uploadedBytes: 0,
          uploadSpeedBps: 0,
          uploadStartedAt: Date.now(),
          uploadUpdatedAt: Date.now(),
        })
        const put = await putSignedUploadFile(intent.data, file, {
          signal: controller.signal,
          onProgress: (progress) => {
            const now = Date.now()
            updateRow(row.id, {
              uploadedBytes: Math.min(progress.loaded, file.size),
              uploadSpeedBps: progress.speedBps,
              uploadStartedAt: now - progress.elapsedMs,
              uploadUpdatedAt: now,
            })
          },
        })
        if (!put.data) throw new Error(put.error || "Signed upload failed.")
        if (canceledRowsRef.current.has(row.id)) throw new DOMException("Upload canceled", "AbortError")

        updateRow(row.id, { status: "stored", message: "Finalizing document" })
        const completed = await completeFileUpload(row.matterId, intent.data.upload_id, {
          document_id: intent.data.document_id,
          etag: put.data.etag,
          bytes: file.size,
        })
        if (!completed.data) throw new Error(completed.error || "Could not finalize upload.")

        let importedComplaintHref: string | undefined
        if (options.importComplaints && markdownIndexable && shouldImportAsComplaint(file.name, documentType)) {
          const imported = await importDocumentComplaint(row.matterId, completed.data.document_id, {
            force: true,
            mode: "structured_import",
          })
          const complaint = imported.data?.imported[0]?.complaint
          if (complaint) {
            importedComplaintHref = matterComplaintHref(row.matterId, "editor", {
              type: "complaint",
              id: complaint.complaint_id,
            })
          }
        }

        updateRow(row.id, {
          status: markdownIndexable ? "stored" : "view_only",
          documentId: completed.data.document_id,
          importedComplaintHref,
          uploadedBytes: file.size,
          uploadUpdatedAt: Date.now(),
          message: markdownIndexable ? "Stored; queued for indexing" : "Stored privately; view-only",
        })
        return { ok: true, documentId: completed.data.document_id, markdownIndexable }
      } catch (error) {
        const canceled = isAbortError(error) || canceledRowsRef.current.has(row.id)
        updateRow(row.id, {
          status: canceled ? "canceled" : "failed",
          message: canceled ? "Canceled" : errorMessage(error),
          error: canceled ? undefined : errorMessage(error),
        })
        return { ok: false }
      } finally {
        controllersRef.current.delete(row.id)
      }
    },
    [updateRow],
  )

  const startIndexJobForBatch = useCallback(
    async (batch: CaseBuilderUploadBatch, documentIds: string[]) => {
      updateBatch(batch.id, { status: "indexing", message: "Indexing" })
      setRows((current) =>
        current.map((row) =>
          row.batchId === batch.id && row.documentId && documentIds.includes(row.documentId)
            ? { ...row, status: "indexing", message: "Indexing" }
            : row,
        ),
      )

      const job = await createMatterIndexJob(batch.matterId, {
        document_ids: documentIds,
        upload_batch_id: batch.uploadBatchId,
      })
      if (!job.data) {
        const message = job.error || "Could not start index job."
        updateBatch(batch.id, { status: "failed", message })
        setRows((current) =>
          current.map((row) =>
            row.batchId === batch.id && row.documentId && documentIds.includes(row.documentId)
              ? { ...row, status: "failed", message, error: message }
              : row,
          ),
        )
        return
      }

      updateBatch(batch.id, { indexJobId: job.data.index_job_id })
      setRows((current) =>
        current.map((row) =>
          row.batchId === batch.id && row.documentId && documentIds.includes(row.documentId)
            ? { ...row, indexJobId: job.data?.index_job_id }
            : row,
        ),
      )
      await pollIndexJob(batch.id, batch.matterId, job.data.index_job_id)
    },
    [updateBatch],
  )

  const pollIndexJob = useCallback(
    async (batchId: string, matterId: string, jobId: string) => {
      let latest: MatterIndexJob | null = null
      for (let attempt = 0; attempt < 120; attempt += 1) {
        await sleep(attempt === 0 ? 400 : 1500)
        const result = await getMatterIndexJob(matterId, jobId)
        if (!result.data) {
          updateBatch(batchId, { status: "failed", message: result.error || "Index job status unavailable." })
          return
        }
        latest = result.data
        applyIndexJobToRows(batchId, latest)
        if (!["queued", "running"].includes(latest.status)) break
      }

      if (!latest) {
        return
      }
      router.refresh()
      updateBatch(batchId, {
        status: latest.status === "failed" ? "failed" : "done",
        message: `${latest.processed} indexed${latest.skipped ? `, ${latest.skipped} skipped` : ""}${latest.failed ? `, ${latest.failed} failed` : ""}.`,
      })
    },
    [router, updateBatch],
  )

  const applyIndexJobToRows = useCallback((batchId: string, job: MatterIndexJob) => {
    const byDocument = new Map(job.results.map((result) => [result.document_id, result]))
    setRows((current) =>
      current.map((row) => {
        if (row.batchId !== batchId || !row.documentId) return row
        const result = byDocument.get(row.documentId)
        if (!result) return ["queued", "running"].includes(job.status) && row.status === "indexing" ? row : row
        return {
          ...row,
          status: result.status === "indexed" ? "indexed" : result.status === "failed" ? "failed" : "stored",
          message: result.message,
          error: result.status === "failed" ? result.message : undefined,
        }
      }),
    )
  }, [])

  const cancelRow = useCallback(
    (rowId: string) => {
      canceledRowsRef.current.add(rowId)
      controllersRef.current.get(rowId)?.abort()
      updateRow(rowId, { status: "canceled", message: "Canceled", error: undefined })
    },
    [updateRow],
  )

  const retryRow = useCallback(
    (rowId: string) => {
      const row = rowsRef.current.find((candidate) => candidate.id === rowId)
      const batch = row ? batchesRef.current.find((candidate) => candidate.id === row.batchId) : null
      if (!row || !batch) return
      if (!row.file) {
        const message = "File selection was lost after refresh. Select this file or folder again to retry."
        updateRow(rowId, { status: "failed", message, error: message })
        return
      }
      canceledRowsRef.current.delete(rowId)
      updateRow(rowId, {
        status: "queued",
        message: "Queued",
        documentId: undefined,
        indexJobId: undefined,
        uploadedBytes: 0,
        uploadSpeedBps: null,
        uploadStartedAt: undefined,
        uploadUpdatedAt: undefined,
        error: undefined,
      })
      updateBatch(batch.id, { status: "uploading", message: "Retrying" })
      void processBatch(batch, [{ ...row, status: "queued", message: "Queued" }], normalizeUploadProcessingOptions(batch))
    },
    [processBatch, updateBatch, updateRow],
  )

  const dismissBatch = useCallback((batchId: string) => {
    setBatches((current) => current.filter((batch) => batch.id !== batchId))
    setRows((current) => current.filter((row) => row.batchId !== batchId))
  }, [])

  useEffect(() => {
    const snapshot = readPersistedUploadSnapshot()
    if (snapshot) {
      const recoveredRows = snapshot.rows.map(recoverPersistedRow)
      const recoveredRowIds = new Set(recoveredRows.map((row) => row.id))
      const recoveredBatches = snapshot.batches
        .map((batch) => ({
          ...batch,
          rowIds: batch.rowIds.filter((rowId) => recoveredRowIds.has(rowId)),
        }))
        .filter((batch) => batch.rowIds.length > 0)
        .map((batch) => recoverPersistedBatch(batch, recoveredRows))
      setRows(recoveredRows)
      setBatches(recoveredBatches)
      if (recoveredBatches.length > 0) setCollapsed(false)
      for (const batch of recoveredBatches) {
        const indexJobId = batch.indexJobId ?? recoveredRows.find((row) => row.batchId === batch.id)?.indexJobId
        if (indexJobId && batch.status === "indexing") {
          void pollIndexJob(batch.id, batch.matterId, indexJobId)
        }
      }
    }
    setStorageReady(true)
  }, [pollIndexJob])

  useEffect(() => {
    if (!storageReady) return
    writePersistedUploadSnapshot(batches, rows)
  }, [batches, rows, storageReady])

  const activeCount = rows.filter((row) => ["queued", "preparing", "uploading", "indexing"].includes(row.status)).length
  const value = useMemo<UploadContextValue>(
    () => ({
      batches,
      rows,
      activeCount,
      enqueueMatterUploads: enqueueRows,
      enqueueMatterIntake: (matterId, candidates, options = {}) => {
        const intakeCandidates = [...candidates]
        const story = options.storyText?.trim()
        if (story) {
          const file = new File([story], "case-narrative.md", { type: "text/markdown" })
          intakeCandidates.unshift({ file, relativePath: "Intake/case-narrative.md", folder: "Intake" })
        }
        return enqueueRows(matterId, intakeCandidates, {
          label: options.label ?? "Matter intake",
          uploadBatchId: options.uploadBatchId,
          autoIndex: options.autoIndex,
          importComplaints: options.importComplaints,
          defaultConfidentiality: options.defaultConfidentiality,
          defaultDocumentType: options.defaultDocumentType,
        })
      },
      cancelRow,
      retryRow,
      dismissBatch,
    }),
    [activeCount, batches, cancelRow, dismissBatch, enqueueRows, retryRow, rows],
  )

  return (
    <CaseBuilderUploadContext.Provider value={value}>
      {children}
      <UploadTray
        batches={batches}
        collapsed={collapsed}
        rows={rows}
        onCancelRow={cancelRow}
        onDismissBatch={dismissBatch}
        onRetryRow={retryRow}
        onToggle={() => setCollapsed((value) => !value)}
      />
    </CaseBuilderUploadContext.Provider>
  )
}

export function useCaseBuilderUploads() {
  const context = useContext(CaseBuilderUploadContext)
  if (!context) {
    throw new Error("useCaseBuilderUploads must be used within CaseBuilderUploadProvider")
  }
  return context
}

function UploadTray({
  batches,
  collapsed,
  rows,
  onCancelRow,
  onDismissBatch,
  onRetryRow,
  onToggle,
}: {
  batches: CaseBuilderUploadBatch[]
  collapsed: boolean
  rows: CaseBuilderUploadRow[]
  onCancelRow: (rowId: string) => void
  onDismissBatch: (batchId: string) => void
  onRetryRow: (rowId: string) => void
  onToggle: () => void
}) {
  if (batches.length === 0) return null
  const activeCount = rows.filter((row) => ["queued", "preparing", "uploading", "indexing"].includes(row.status)).length
  const failedCount = rows.filter((row) => row.status === "failed").length
  return (
    <div className="fixed bottom-4 right-4 z-50 w-[min(28rem,calc(100vw-2rem))] rounded border border-border bg-card shadow-2xl">
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full items-center justify-between gap-3 border-b border-border px-3 py-2 text-left"
      >
        <span className="flex min-w-0 items-center gap-2">
          {activeCount > 0 ? (
            <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin text-primary" />
          ) : failedCount > 0 ? (
            <AlertCircle className="h-3.5 w-3.5 shrink-0 text-destructive" />
          ) : (
            <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-success" />
          )}
          <span className="truncate font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            uploads {activeCount > 0 ? `${activeCount} active` : failedCount > 0 ? `${failedCount} failed` : "complete"}
          </span>
        </span>
        {collapsed ? <ChevronUp className="h-3.5 w-3.5" /> : <ChevronDown className="h-3.5 w-3.5" />}
      </button>
      {!collapsed && (
        <div className="max-h-96 overflow-y-auto p-2 scrollbar-thin">
          {batches.map((batch) => {
            const batchRows = rows.filter((row) => row.batchId === batch.id)
            return (
              <div key={batch.id} className="mb-2 rounded border border-border bg-background p-2 last:mb-0">
                <div className="mb-2 flex items-center justify-between gap-2">
                  <div className="min-w-0">
                    <div className="truncate text-xs font-medium text-foreground">{batch.label}</div>
                    <div className="font-mono text-[10px] text-muted-foreground">
                      {batchRows.length} item{batchRows.length === 1 ? "" : "s"} · {batch.status}
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <Link
                      href={matterHref(batch.matterId, "documents")}
                      className="inline-flex h-7 w-7 items-center justify-center rounded border border-border text-muted-foreground hover:bg-muted hover:text-foreground"
                      title="Open documents"
                    >
                      <FolderUp className="h-3.5 w-3.5" />
                    </Link>
                    {!["queued", "uploading", "indexing"].includes(batch.status) && (
                      <button
                        type="button"
                        onClick={() => onDismissBatch(batch.id)}
                        className="inline-flex h-7 w-7 items-center justify-center rounded border border-border text-muted-foreground hover:bg-muted hover:text-foreground"
                        title="Dismiss"
                      >
                        <X className="h-3.5 w-3.5" />
                      </button>
                    )}
                  </div>
                </div>
                <div className="space-y-1">
                  {batchRows.map((row) => {
                    const uploaded = rowUploadedBytes(row)
                    const percent = uploadPercent(row)
                    return (
                      <div key={row.id} className="rounded border border-border px-2 py-1.5 text-[11px]">
                        <div className="flex items-center gap-2">
                          <span className={cn("h-2 w-2 shrink-0 rounded-full", uploadDotClass(row.status))} />
                          <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                          <span className="min-w-0 flex-1 truncate font-mono" title={row.relativePath}>
                            {row.relativePath}
                          </span>
                          <span className="shrink-0 text-muted-foreground">{row.status}</span>
                          {["queued", "preparing", "uploading"].includes(row.status) && (
                            <button
                              type="button"
                              onClick={() => onCancelRow(row.id)}
                              className="shrink-0 rounded border border-border px-1.5 py-0.5 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted"
                            >
                              cancel
                            </button>
                          )}
                          {row.status === "failed" && (
                            <button
                              type="button"
                              onClick={() => onRetryRow(row.id)}
                              className="inline-flex shrink-0 items-center gap-1 rounded border border-border px-1.5 py-0.5 font-mono text-[10px] uppercase text-muted-foreground hover:bg-muted"
                            >
                              <RefreshCcw className="h-3 w-3" />
                              retry
                            </button>
                          )}
                          {row.importedComplaintHref && (
                            <Link
                              href={row.importedComplaintHref}
                              className="shrink-0 rounded border border-primary/25 px-1.5 py-0.5 font-mono text-[10px] uppercase text-primary hover:bg-primary/10"
                            >
                              complaint
                            </Link>
                          )}
                        </div>
                        {row.bytes > 0 && ["uploading", "stored", "view_only", "indexing", "indexed"].includes(row.status) && (
                          <div className="mt-1.5 flex items-center gap-2 pl-5">
                            <div className="h-1.5 min-w-0 flex-1 overflow-hidden rounded bg-muted">
                              <div className="h-full rounded bg-primary transition-[width]" style={{ width: `${percent}%` }} />
                            </div>
                            <span className="shrink-0 font-mono text-[10px] text-muted-foreground">
                              {formatBytes(uploaded)} / {formatBytes(row.bytes)}
                              {row.status === "uploading" ? ` · ${formatSpeed(row.uploadSpeedBps)}` : ""}
                            </span>
                          </div>
                        )}
                      </div>
                    )
                  })}
                </div>
                {batch.message && <div className="mt-2 truncate font-mono text-[10px] text-muted-foreground">{batch.message}</div>}
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}

function readPersistedUploadSnapshot(): PersistedUploadSnapshot | null {
  try {
    const raw = window.sessionStorage.getItem(UPLOAD_SNAPSHOT_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as PersistedUploadSnapshot
    if (!Array.isArray(parsed.batches) || !Array.isArray(parsed.rows)) return null
    return parsed
  } catch {
    return null
  }
}

function writePersistedUploadSnapshot(batches: CaseBuilderUploadBatch[], rows: CaseBuilderUploadRow[]) {
  try {
    const activeRows = rows.filter((row) =>
      ["queued", "preparing", "uploading", "indexing", "failed"].includes(row.status),
    )
    if (activeRows.length === 0) {
      window.sessionStorage.removeItem(UPLOAD_SNAPSHOT_KEY)
      return
    }
    const activeRowIds = new Set(activeRows.map((row) => row.id))
    const snapshot: PersistedUploadSnapshot = {
      batches: batches
        .map((batch) => ({
          ...batch,
          rowIds: batch.rowIds.filter((rowId) => activeRowIds.has(rowId)),
        }))
        .filter((batch) => batch.rowIds.length > 0),
      rows: activeRows.map(({ file: _file, ...row }) => row),
    }
    window.sessionStorage.setItem(UPLOAD_SNAPSHOT_KEY, JSON.stringify(snapshot))
  } catch {
    // Session storage is a convenience for refresh recovery; uploads still work without it.
  }
}

function recoverPersistedRow(row: Omit<CaseBuilderUploadRow, "file">): CaseBuilderUploadRow {
  if (row.status === "indexing" && row.documentId && row.indexJobId) {
    return { ...row, message: "Indexing continues in the background." }
  }
  if (["queued", "preparing", "uploading", "indexing"].includes(row.status)) {
    const message = "Upload interrupted by refresh. Select this file or folder again to retry."
    return { ...row, status: "failed", message, error: message }
  }
  return {
    ...row,
    status: row.status === "failed" ? "failed" : row.status,
    message: row.message || "Retry needed.",
  }
}

function recoverPersistedBatch(batch: CaseBuilderUploadBatch, rows: CaseBuilderUploadRow[]): CaseBuilderUploadBatch {
  const batchRows = rows.filter((row) => row.batchId === batch.id)
  if (batchRows.some((row) => row.status === "indexing" && row.indexJobId)) {
    return { ...batch, status: "indexing", message: "Indexing continues in the background." }
  }
  if (batchRows.some((row) => row.status === "failed")) {
    return { ...batch, status: "failed", message: "Upload interrupted by refresh." }
  }
  return batch
}

function rowUploadedBytes(row: CaseBuilderUploadRow) {
  if (["stored", "view_only", "indexing", "indexed"].includes(row.status)) return row.bytes
  return Math.max(0, Math.min(row.uploadedBytes ?? 0, row.bytes))
}

function uploadPercent(row: CaseBuilderUploadRow) {
  if (row.bytes <= 0) return 0
  return Math.max(0, Math.min(100, Math.round((rowUploadedBytes(row) / row.bytes) * 100)))
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${trimNumber(bytes / 1024)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${trimNumber(bytes / 1024 / 1024)} MB`
  return `${trimNumber(bytes / 1024 / 1024 / 1024)} GB`
}

function formatSpeed(speedBps?: number | null) {
  if (!speedBps || speedBps <= 0) return "starting"
  return `${formatBytes(speedBps)}/s`
}

function trimNumber(value: number) {
  return value >= 10 ? value.toFixed(0) : value.toFixed(1)
}

function uploadDotClass(status: CaseBuilderUploadRowStatus) {
  switch (status) {
    case "indexed":
      return "bg-success"
    case "stored":
    case "view_only":
      return "bg-warning"
    case "failed":
      return "bg-destructive"
    case "queued":
    case "preparing":
    case "uploading":
    case "indexing":
      return "bg-primary animate-pulse"
    case "canceled":
      return "bg-muted-foreground/40"
  }
}

function guessMimeType(filename: string) {
  if (/\.(md|markdown)$/i.test(filename)) return "text/markdown"
  if (/\.csv$/i.test(filename)) return "text/csv"
  if (/\.html?$/i.test(filename)) return "text/html"
  if (/\.json$/i.test(filename)) return "application/json"
  if (/\.pdf$/i.test(filename)) return "application/pdf"
  if (/\.docx?$/i.test(filename)) return "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
  if (/\.(png|jpe?g|gif|webp|heic)$/i.test(filename)) return "image/*"
  if (/\.(mp3|m4a|wav|aac|flac)$/i.test(filename)) return "audio/*"
  if (/\.(mp4|mov|m4v|webm)$/i.test(filename)) return "video/*"
  return "application/octet-stream"
}

function guessDocumentType(filename: string, mimeType: string, fallback: DocumentType = "evidence"): DocumentType {
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
  return fallback
}

function shouldImportAsComplaint(filename: string, documentType: string) {
  return documentType === "complaint" || /complaint|pleading|petition/i.test(filename)
}

function isAbortError(error: unknown) {
  return error instanceof DOMException
    ? error.name === "AbortError"
    : error instanceof Error && error.name === "AbortError"
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}

function sleep(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}
