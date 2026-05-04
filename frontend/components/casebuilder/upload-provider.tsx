"use client"

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
  abortFileUpload,
  completeFileUpload,
  createFileUpload,
  createFileUploadParts,
  createMatterIndexJob,
  getMatterIndexJob,
  importDocumentComplaint,
  listFileUploadParts,
  putSignedUploadFile,
  type CompletedUploadPart,
  type FileUploadIntent,
  type FileUploadPartIntent,
} from "@/lib/casebuilder/api"
import { isMarkdownIndexableFile } from "@/lib/casebuilder/document-tree"
import { matterComplaintHref, matterHref } from "@/lib/casebuilder/routes"
import { createUploadBatchId, normalizeUploadCandidate, type UploadCandidate } from "@/lib/casebuilder/upload-folders"
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
  uploadId?: string
  uploadMode?: "single" | "multipart" | string
  multipartPartSizeBytes?: number | null
  multipartTotalParts?: number | null
  multipartUploadedParts?: CompletedUploadPart[]
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
const MULTIPART_CONCURRENCY = 3
const MULTIPART_RETRIES = 2

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
    [applyIndexJobToRows, router, updateBatch],
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
    [pollIndexJob, updateBatch],
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
      let activeUploadId = row.uploadId

      try {
        updateRow(row.id, { status: "preparing", message: "Preparing signed upload", error: undefined })
        const existingMultipartIntent =
          row.uploadMode === "multipart" && row.uploadId && row.documentId
            ? ({
                upload_id: row.uploadId,
                document_id: row.documentId,
                mode: "multipart",
                method: "",
                url: "",
                expires_at: "",
                headers: {},
                part_size_bytes: row.multipartPartSizeBytes ?? null,
                total_parts: row.multipartTotalParts ?? null,
                parts: [],
              } as FileUploadIntent)
            : null
        const intent = existingMultipartIntent
          ? { data: existingMultipartIntent, error: null }
          : await createFileUpload(row.matterId, {
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
        activeUploadId = intent.data.upload_id
        if (canceledRowsRef.current.has(row.id)) throw new DOMException("Upload canceled", "AbortError")

        updateRow(row.id, {
          status: "uploading",
          documentId: intent.data.document_id,
          uploadId: intent.data.upload_id,
          uploadMode: intent.data.mode,
          multipartPartSizeBytes: intent.data.part_size_bytes ?? null,
          multipartTotalParts: intent.data.total_parts ?? null,
          message: "Uploading to private storage",
          uploadedBytes: 0,
          uploadSpeedBps: 0,
          uploadStartedAt: Date.now(),
          uploadUpdatedAt: Date.now(),
        })
        const onProgress = (progress: { loaded: number; speedBps?: number | null; elapsedMs: number }) => {
          const now = Date.now()
          updateRow(row.id, {
            uploadedBytes: Math.min(progress.loaded, file.size),
            uploadSpeedBps: progress.speedBps ?? null,
            uploadStartedAt: now - progress.elapsedMs,
            uploadUpdatedAt: now,
          })
        }
        const completedParts =
          intent.data.mode === "multipart"
            ? await putMultipartUploadFile(row.matterId, intent.data, file, {
                signal: controller.signal,
                onProgress,
                onUploadedParts: (parts) => updateRow(row.id, { multipartUploadedParts: parts }),
              })
            : null
        let etag: string | null | undefined = null
        if (!completedParts) {
          const put = await putSignedUploadFile(intent.data, file, {
            signal: controller.signal,
            onProgress,
          })
          if (!put.data) throw new Error(put.error || "Signed upload failed.")
          etag = put.data.etag
        }
        if (canceledRowsRef.current.has(row.id)) throw new DOMException("Upload canceled", "AbortError")

        updateRow(row.id, { status: "stored", message: "Finalizing document" })
        const completed = await completeFileUpload(row.matterId, intent.data.upload_id, {
          document_id: intent.data.document_id,
          etag,
          bytes: file.size,
          parts: completedParts ?? undefined,
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
        if (canceled && activeUploadId) {
          void abortFileUpload(row.matterId, activeUploadId)
        }
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
    [router, startIndexJobForBatch, updateBatch, uploadOneRow],
  )

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
      const normalizedCandidates = candidates.map((candidate) => normalizeUploadCandidate(candidate))
      const newCandidates: UploadCandidate[] = []
      const resumedBatchIds: string[] = []
      for (const candidate of normalizedCandidates) {
        const existing = rowsRef.current.find(
          (row) =>
            row.matterId === matterId &&
            row.uploadMode === "multipart" &&
            row.uploadId &&
            row.documentId &&
            !row.file &&
            row.status === "failed" &&
            row.relativePath === candidate.relativePath &&
            row.bytes === candidate.file.size,
        )
        const existingBatch = existing
          ? batchesRef.current.find((candidateBatch) => candidateBatch.id === existing.batchId)
          : null
        if (!existing || !existingBatch) {
          newCandidates.push(candidate)
          continue
        }
        const resumedRow: CaseBuilderUploadRow = {
          ...existing,
          file: candidate.file,
          status: "queued",
          message: "Queued",
          error: undefined,
          uploadedBytes: 0,
          uploadSpeedBps: null,
        }
        updateBatch(existingBatch.id, { status: "queued", message: "Resuming upload" })
        updateRow(existing.id, resumedRow)
        resumedBatchIds.push(existingBatch.id)
        void processBatch(existingBatch, [resumedRow], processingOptions)
      }
      if (newCandidates.length === 0) {
        setCollapsed(false)
        return resumedBatchIds[0] ?? null
      }
      const nextRows = newCandidates.map((candidate, index): CaseBuilderUploadRow => ({
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
    [processBatch, updateBatch, updateRow],
  )

  const cancelRow = useCallback(
    (rowId: string) => {
      const row = rowsRef.current.find((candidate) => candidate.id === rowId)
      canceledRowsRef.current.add(rowId)
      controllersRef.current.get(rowId)?.abort()
      if (row?.uploadId && ["preparing", "uploading", "stored"].includes(row.status)) {
        void abortFileUpload(row.matterId, row.uploadId)
      }
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
      const preserveMultipart = row.uploadMode === "multipart" && row.uploadId && row.documentId
      updateRow(rowId, {
        status: "queued",
        message: "Queued",
        documentId: preserveMultipart ? row.documentId : undefined,
        uploadId: preserveMultipart ? row.uploadId : undefined,
        uploadMode: preserveMultipart ? row.uploadMode : undefined,
        multipartPartSizeBytes: preserveMultipart ? row.multipartPartSizeBytes : undefined,
        multipartTotalParts: preserveMultipart ? row.multipartTotalParts : undefined,
        multipartUploadedParts: preserveMultipart ? row.multipartUploadedParts : undefined,
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
      rows: activeRows.map((row) => {
        const persisted = { ...row }
        delete persisted.file
        return persisted
      }),
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

async function putMultipartUploadFile(
  matterId: string,
  intent: FileUploadIntent,
  file: File,
  options: {
    signal: AbortSignal
    onProgress: (progress: { loaded: number; speedBps?: number | null; elapsedMs: number }) => void
    onUploadedParts?: (parts: CompletedUploadPart[]) => void
  },
): Promise<CompletedUploadPart[]> {
  const partSize = intent.part_size_bytes ?? 0
  const totalParts = intent.total_parts ?? (partSize > 0 ? Math.ceil(file.size / partSize) : 0)
  if (!partSize || !totalParts) {
    throw new Error("Multipart upload intent is missing part sizing.")
  }

  const startedAt = performance.now()
  const completedParts = new Map<number, CompletedUploadPart>()
  const loadedByPart = new Map<number, number>()
  const intentByPart = new Map<number, FileUploadPartIntent>(
    (intent.parts ?? []).map((part) => [part.part_number, part]),
  )

  const listed = await listFileUploadParts(matterId, intent.upload_id)
  if (!listed.data) throw new Error(listed.error || "Could not recover multipart upload state.")
  for (const part of listed.data.uploaded_parts) {
    completedParts.set(part.part_number, part)
    loadedByPart.set(part.part_number, multipartPartLength(file.size, partSize, part.part_number))
  }

  function emitProgress() {
    const loaded = Array.from(loadedByPart.values()).reduce((sum, value) => sum + value, 0)
    const elapsedMs = Math.max(performance.now() - startedAt, 1)
    const speedBps = loaded > 0 ? loaded / (elapsedMs / 1000) : 0
    options.onProgress({ loaded, speedBps, elapsedMs })
    options.onUploadedParts?.(sortedCompletedParts(completedParts))
  }

  async function getPartIntent(partNumber: number) {
    const cached = intentByPart.get(partNumber)
    if (cached) return cached
    const refreshed = await createFileUploadParts(matterId, intent.upload_id, [partNumber])
    if (!refreshed.data) throw new Error(refreshed.error || `Could not sign upload part ${partNumber}.`)
    for (const part of refreshed.data.uploaded_parts) {
      completedParts.set(part.part_number, part)
      loadedByPart.set(part.part_number, multipartPartLength(file.size, partSize, part.part_number))
    }
    for (const part of refreshed.data.parts) {
      intentByPart.set(part.part_number, part)
    }
    const signed = intentByPart.get(partNumber)
    if (!signed) throw new Error(`Upload part ${partNumber} was not signed.`)
    return signed
  }

  async function uploadPart(partNumber: number) {
    if (completedParts.has(partNumber)) return
    const start = (partNumber - 1) * partSize
    const end = Math.min(start + partSize, file.size)
    const blob = file.slice(start, end, file.type)
    for (let attempt = 0; attempt <= MULTIPART_RETRIES; attempt += 1) {
      if (options.signal.aborted) throw new DOMException("Upload canceled", "AbortError")
      try {
        const signed = await getPartIntent(partNumber)
        const put = await putSignedUploadFile(signed, blob, {
          signal: options.signal,
          onProgress: (progress) => {
            loadedByPart.set(partNumber, progress.loaded)
            emitProgress()
          },
        })
        if (!put.data) throw new Error(put.error || `Upload part ${partNumber} failed.`)
        if (!put.data.etag) throw new Error(`Upload part ${partNumber} did not return an ETag.`)
        const completed = { part_number: partNumber, etag: put.data.etag }
        completedParts.set(partNumber, completed)
        loadedByPart.set(partNumber, blob.size)
        emitProgress()
        return
      } catch (error) {
        if (isAbortError(error) || options.signal.aborted || attempt >= MULTIPART_RETRIES) throw error
        intentByPart.delete(partNumber)
        await sleep(500 * (attempt + 1))
      }
    }
  }

  const pending = Array.from({ length: totalParts }, (_, index) => index + 1).filter(
    (partNumber) => !completedParts.has(partNumber),
  )
  let cursor = 0
  async function worker() {
    for (;;) {
      const partNumber = pending[cursor]
      cursor += 1
      if (!partNumber) return
      await uploadPart(partNumber)
    }
  }

  emitProgress()
  await Promise.all(Array.from({ length: Math.min(MULTIPART_CONCURRENCY, pending.length) }, worker))
  const completed = sortedCompletedParts(completedParts)
  if (completed.length !== totalParts) {
    throw new Error(`Multipart upload completed ${completed.length} of ${totalParts} parts.`)
  }
  return completed
}

function sortedCompletedParts(parts: Map<number, CompletedUploadPart>) {
  return Array.from(parts.values()).sort((left, right) => left.part_number - right.part_number)
}

function multipartPartLength(totalBytes: number, partSize: number, partNumber: number) {
  const start = (partNumber - 1) * partSize
  return Math.max(0, Math.min(partSize, totalBytes - start))
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
