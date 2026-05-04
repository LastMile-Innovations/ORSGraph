import type { CaseDocument } from "./types"
import type { UploadCandidate } from "./upload-folders"

export type DocumentTreeSelectionKind = "all" | "recent" | "duplicates" | "media" | "archive" | "folder"

export interface DocumentTreeSelection {
  kind: DocumentTreeSelectionKind
  path?: string
}

export interface DocumentTreeCounts {
  total: number
  active: number
  archived: number
  indexed: number
  pending: number
  failed: number
  media: number
}

export interface DocumentTreeNode {
  id: string
  name: string
  path: string
  depth: number
  documents: CaseDocument[]
  children: DocumentTreeNode[]
  counts: DocumentTreeCounts
}

export interface UploadPreviewRow {
  id: string
  candidate: UploadCandidate
  relativePath: string
  folder: string
  inferredType: string
  status: "ready" | "media" | "ocr" | "unsupported"
  conflict: "none" | "existing_path" | "duplicate_in_batch"
}

export function documentIsArchived(document: CaseDocument): boolean {
  return Boolean(document.archived_at || document.deleted_at || document.storage_status === "deleted")
}

export function documentLibraryPath(document: CaseDocument): string {
  const explicit = normalizeClientLibraryPath(document.library_path || document.original_relative_path || "")
  if (explicit) return explicit
  const filename = normalizePathSegment(document.filename || document.document_id)
  const folder = normalizePathSegment(document.folder || "Uploads")
  return `${folder || "Uploads"}/${filename || document.document_id}`
}

export function buildDocumentTree(documents: CaseDocument[]): DocumentTreeNode {
  const root = createTreeNode("Matter Files", "", -1)
  for (const document of documents.filter((item) => !documentIsArchived(item))) {
    const path = documentLibraryPath(document)
    const folderSegments = path.split("/").slice(0, -1)
    let node = root
    for (const [index, segment] of folderSegments.entries()) {
      const folderPath = folderSegments.slice(0, index + 1).join("/")
      let child = node.children.find((item) => item.path === folderPath)
      if (!child) {
        child = createTreeNode(segment, folderPath, index)
        node.children.push(child)
      }
      node = child
    }
    node.documents.push(document)
  }
  sortTree(root)
  rollupCounts(root)
  return root
}

export function filterDocumentsBySelection(
  documents: CaseDocument[],
  selection: DocumentTreeSelection,
  latestBatchId = latestUploadBatchId(documents),
): CaseDocument[] {
  const active = documents.filter((document) => !documentIsArchived(document))
  if (selection.kind === "archive") {
    return documents.filter(documentIsArchived)
  }
  if (selection.kind === "recent") {
    return latestBatchId ? active.filter((document) => document.upload_batch_id === latestBatchId) : []
  }
  if (selection.kind === "duplicates") {
    const hashes = duplicateHashes(active)
    return active.filter((document) => document.file_hash && hashes.has(document.file_hash))
  }
  if (selection.kind === "media") {
    return active.filter(documentIsMedia)
  }
  if (selection.kind === "folder" && selection.path) {
    const prefix = `${selection.path}/`
    return active.filter((document) => documentLibraryPath(document).startsWith(prefix))
  }
  return active
}

export function latestUploadBatchId(documents: CaseDocument[]): string | null {
  const withBatch = documents
    .filter((document) => document.upload_batch_id && !documentIsArchived(document))
    .sort((a, b) => (b.uploaded_at || "").localeCompare(a.uploaded_at || ""))
  return withBatch[0]?.upload_batch_id ?? null
}

export function duplicateHashes(documents: CaseDocument[]): Set<string> {
  const counts = new Map<string, number>()
  for (const document of documents) {
    if (!document.file_hash) continue
    counts.set(document.file_hash, (counts.get(document.file_hash) ?? 0) + 1)
  }
  return new Set(Array.from(counts).filter(([, count]) => count > 1).map(([hash]) => hash))
}

export function buildUploadPreviewRows(
  candidates: UploadCandidate[],
  existingDocuments: CaseDocument[],
): UploadPreviewRow[] {
  const existingPaths = new Set(
    existingDocuments
      .filter((document) => !documentIsArchived(document))
      .map((document) => documentLibraryPath(document).toLowerCase()),
  )
  const batchPathCounts = new Map<string, number>()
  for (const candidate of candidates) {
    const path = normalizeClientLibraryPath(candidate.relativePath) || candidate.file.name
    const key = path.toLowerCase()
    batchPathCounts.set(key, (batchPathCounts.get(key) ?? 0) + 1)
  }
  return candidates.map((candidate, index) => {
    const relativePath = normalizeClientLibraryPath(candidate.relativePath) || candidate.file.name
    const key = relativePath.toLowerCase()
    return {
      id: `${key}:${index}`,
      candidate,
      relativePath,
      folder: relativePath.split("/").slice(0, -1).join("/") || candidate.folder || "Uploads",
      inferredType: inferUploadDocumentType(candidate.file),
      status: inferUploadStatus(candidate.file),
      conflict: existingPaths.has(key)
        ? "existing_path"
        : (batchPathCounts.get(key) ?? 0) > 1
          ? "duplicate_in_batch"
          : "none",
    }
  })
}

export function normalizeClientLibraryPath(value: string): string {
  return value
    .replace(/\\/g, "/")
    .split("/")
    .map((segment) => normalizePathSegment(segment))
    .filter(Boolean)
    .join("/")
}

export function documentIsMedia(document: Pick<CaseDocument, "filename" | "mime_type" | "processing_status">): boolean {
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

function createTreeNode(name: string, path: string, depth: number): DocumentTreeNode {
  return {
    id: path || "root",
    name,
    path,
    depth,
    documents: [],
    children: [],
    counts: emptyCounts(),
  }
}

function emptyCounts(): DocumentTreeCounts {
  return { total: 0, active: 0, archived: 0, indexed: 0, pending: 0, failed: 0, media: 0 }
}

function rollupCounts(node: DocumentTreeNode): DocumentTreeCounts {
  const counts = emptyCounts()
  for (const document of node.documents) {
    counts.total += 1
    counts.active += 1
    if (document.processing_status === "processed") counts.indexed += 1
    if (document.processing_status === "failed") counts.failed += 1
    if (!["processed", "failed", "unsupported", "ocr_required", "transcription_deferred", "view_only"].includes(document.processing_status)) {
      counts.pending += 1
    }
    if (documentIsMedia(document)) counts.media += 1
  }
  for (const child of node.children) {
    const childCounts = rollupCounts(child)
    counts.total += childCounts.total
    counts.active += childCounts.active
    counts.archived += childCounts.archived
    counts.indexed += childCounts.indexed
    counts.pending += childCounts.pending
    counts.failed += childCounts.failed
    counts.media += childCounts.media
  }
  node.counts = counts
  return counts
}

function sortTree(node: DocumentTreeNode) {
  node.children.sort((a, b) => a.path.localeCompare(b.path, undefined, { numeric: true, sensitivity: "base" }))
  node.documents.sort((a, b) => documentLibraryPath(a).localeCompare(documentLibraryPath(b), undefined, { numeric: true, sensitivity: "base" }))
  for (const child of node.children) sortTree(child)
}

function inferUploadDocumentType(file: File): string {
  const name = file.name.toLowerCase()
  const mimeType = file.type.toLowerCase()
  if (name.includes("complaint")) return "complaint"
  if (name.includes("answer")) return "answer"
  if (name.includes("motion")) return "motion"
  if (name.includes("notice")) return "notice"
  if (name.includes("lease")) return "lease"
  if (name.includes("contract")) return "contract"
  if (name.includes("receipt")) return "receipt"
  if (name.includes("invoice")) return "invoice"
  if (/\.csv$/i.test(name)) return "spreadsheet"
  if (mimeType.startsWith("image/")) return "photo"
  return "evidence"
}

function inferUploadStatus(file: File): UploadPreviewRow["status"] {
  const name = file.name.toLowerCase()
  const mimeType = file.type.toLowerCase()
  if (mimeType.startsWith("audio/") || mimeType.startsWith("video/") || /\.(mp3|m4a|wav|aac|flac|mp4|mov|m4v|webm)$/i.test(name)) {
    return "media"
  }
  if (mimeType.startsWith("image/") || /\.(png|jpe?g|gif|webp|heic)$/i.test(name)) {
    return "ocr"
  }
  if (/\.(exe|dmg|pkg|app|bin|zip|7z|rar)$/i.test(name)) {
    return "unsupported"
  }
  return "ready"
}

function normalizePathSegment(value: string): string {
  const trimmed = value.trim()
  if (!trimmed || trimmed === "." || trimmed === ".." || /[\u0000-\u001f\u007f]/.test(trimmed)) return ""
  return trimmed
}
