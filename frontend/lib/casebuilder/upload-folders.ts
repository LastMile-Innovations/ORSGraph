export interface UploadCandidate {
  file: File
  relativePath: string
  folder: string
}

export function createUploadBatchId(prefix = "batch"): string {
  const random = Math.random().toString(36).slice(2, 8)
  return `${prefix}:${Date.now().toString(36)}:${random}`
}

export function normalizeUploadRelativePath(file: File, explicitPath?: string): string {
  const webkitPath = (file as File & { webkitRelativePath?: string }).webkitRelativePath
  const raw = explicitPath || webkitPath || file.name
  const normalized = raw
    .replace(/\\/g, "/")
    .split("/")
    .map((segment) => segment.trim())
    .filter((segment) => segment && segment !== "." && segment !== ".." && !/[\u0000-\u001f\u007f]/.test(segment))
    .join("/")
  return normalized || file.name
}

export function folderFromRelativePath(relativePath: string, fallback = "Uploads"): string {
  const first = relativePath.split("/").find(Boolean)
  if (!first || first === relativePath) return fallback
  return first
}

export function filesToUploadCandidates(files: FileList | File[], fallbackFolder = "Uploads"): UploadCandidate[] {
  return Array.from(files).map((file) => {
    const relativePath = normalizeUploadRelativePath(file)
    return {
      file,
      relativePath,
      folder: folderFromRelativePath(relativePath, fallbackFolder),
    }
  })
}

export async function dataTransferToUploadCandidates(dataTransfer: DataTransfer): Promise<UploadCandidate[]> {
  const items = Array.from(dataTransfer.items ?? [])
  const entries = items
    .map((item) => (item as DataTransferItem & { webkitGetAsEntry?: () => unknown }).webkitGetAsEntry?.())
    .filter(Boolean)

  if (entries.length === 0) {
    return filesToUploadCandidates(dataTransfer.files)
  }

  const candidates: UploadCandidate[] = []
  for (const entry of entries) {
    candidates.push(...(await walkEntry(entry, "")))
  }
  return candidates
}

async function walkEntry(entry: unknown, parentPath: string): Promise<UploadCandidate[]> {
  const item = entry as {
    isFile?: boolean
    isDirectory?: boolean
    name?: string
    file?: (success: (file: File) => void, failure: (error: unknown) => void) => void
    createReader?: () => { readEntries: (success: (entries: unknown[]) => void, failure: (error: unknown) => void) => void }
  }
  const name = item.name ?? ""
  const relativePath = parentPath ? `${parentPath}/${name}` : name

  if (item.isFile && item.file) {
    const file = await new Promise<File>((resolve, reject) => item.file?.(resolve, reject))
    const normalized = normalizeUploadRelativePath(file, relativePath)
    return [{ file, relativePath: normalized, folder: folderFromRelativePath(normalized) }]
  }

  if (item.isDirectory && item.createReader) {
    const reader = item.createReader()
    const children = await readAllEntries(reader)
    const nested = await Promise.all(children.map((child) => walkEntry(child, relativePath)))
    return nested.flat()
  }

  return []
}

async function readAllEntries(reader: {
  readEntries: (success: (entries: unknown[]) => void, failure: (error: unknown) => void) => void
}): Promise<unknown[]> {
  const all: unknown[] = []
  for (;;) {
    const batch = await new Promise<unknown[]>((resolve, reject) => reader.readEntries(resolve, reject))
    if (batch.length === 0) return all
    all.push(...batch)
  }
}
