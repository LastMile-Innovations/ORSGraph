"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { useMemo, useRef, useState } from "react"
import {
  AlertTriangle,
  CheckCircle2,
  FileText,
  Filter,
  Folder,
  Grid2x2,
  List,
  Search,
  Sparkles,
  Upload,
} from "lucide-react"
import { cn } from "@/lib/utils"
import type { CaseDocument, DocumentType, MatterSummary } from "@/lib/casebuilder/types"
import { extractDocument, uploadBinaryFile } from "@/lib/casebuilder/api"
import { matterDocumentHref } from "@/lib/casebuilder/routes"
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

export function DocumentLibrary({ matter, documents }: Props) {
  const router = useRouter()
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [folder, setFolder] = useState<string>("All")
  const [query, setQuery] = useState("")
  const [view, setView] = useState<"grid" | "list">("grid")
  const [uploading, setUploading] = useState(false)
  const [uploadMessage, setUploadMessage] = useState<string | null>(null)
  const [uploadError, setUploadError] = useState<string | null>(null)

  const folderCounts = useMemo(() => {
    const map: Record<string, number> = { All: documents.length }
    for (const f of FOLDERS) map[f] = 0
    for (const d of documents) map[d.folder] = (map[d.folder] ?? 0) + 1
    return map
  }, [documents])

  const filtered = useMemo(() => {
    return documents.filter((d) => {
      if (folder !== "All" && d.folder !== folder) return false
      if (query.trim()) {
        const q = query.toLowerCase()
        const hay = `${d.filename} ${d.summary} ${d.parties_mentioned.join(" ")} ${d.entities_mentioned.join(" ")}`.toLowerCase()
        if (!hay.includes(q)) return false
      }
      return true
    })
  }, [documents, folder, query])

  async function uploadFiles(files: FileList | File[]) {
    const selected = Array.from(files)
    if (selected.length === 0) return

    setUploading(true)
    setUploadMessage(null)
    setUploadError(null)

    let stored = 0
    let extracted = 0
    let binaryStored = 0
    const failures: string[] = []

    for (const file of selected) {
      try {
        const textLike = isTextLikeFile(file)
        const mimeType = file.type || guessMimeType(file.name)
        const documentType = guessDocumentType(file.name, mimeType)
        const result = await uploadBinaryFile(matter.matter_id, file, {
          document_type: documentType,
          folder: "Uploads",
          confidentiality: "private",
        })

        if (!result.data) {
          failures.push(`${file.name}: ${result.error || "upload failed"}`)
          continue
        }

        stored += 1
        if (textLike) {
          const extraction = await extractDocument(matter.matter_id, result.data.document_id)
          if (extraction.data?.status === "processed") extracted += 1
        } else if (result.data.storage_status === "stored") {
          binaryStored += 1
        }
      } catch (error) {
        failures.push(`${file.name}: ${error instanceof Error ? error.message : String(error)}`)
      }
    }

    setUploading(false)
    if (failures.length > 0) {
      setUploadError(failures.join(" | "))
    }
    setUploadMessage(
      `${stored} uploaded${extracted ? `, ${extracted} extracted` : ""}${binaryStored ? `, ${binaryStored} stored privately` : ""}.`,
    )
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
          <button
            onClick={() => fileInputRef.current?.click()}
            disabled={uploading}
            className="flex items-center gap-1.5 rounded bg-primary px-3 py-1.5 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
          >
            <Upload className="h-3.5 w-3.5" />
            {uploading ? "uploading" : "upload files"}
          </button>
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
          </div>
        )}
      </div>

      {/* Toolbar */}
      <div className="flex flex-wrap items-center gap-2 border-b border-border px-6 py-2">
        <div className="flex flex-1 items-center gap-2 rounded border border-border bg-background px-2.5">
          <Search className="h-3.5 w-3.5 text-muted-foreground" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search filenames, summaries, parties, entities…"
            className="flex-1 bg-transparent py-1.5 text-xs focus:outline-none"
          />
        </div>
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
            {FOLDERS.map((f) => (
              <FolderItem
                key={f}
                name={f}
                count={folderCounts[f]}
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
            <KV label="queued" value={documents.filter((d) => d.processing_status === "queued").length} cls="text-muted-foreground" />
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
        </aside>

        {/* Grid */}
        <div
          className="flex-1 overflow-y-auto p-4 scrollbar-thin"
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => {
            event.preventDefault()
            void uploadFiles(event.dataTransfer.files)
          }}
        >
          {filtered.length === 0 ? (
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
                  {d.filename}
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

function isTextLikeFile(file: File) {
  return file.type.startsWith("text/") || /\.(txt|md|csv|html?|json|log)$/i.test(file.name)
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
