"use client"

import { useMemo, useRef, useState, type ReactNode } from "react"
import Image from "next/image"
import Link from "next/link"
import { useRouter } from "next/navigation"
import {
  ArrowLeft,
  Captions,
  CheckCircle2,
  Download,
  FileText,
  Highlighter,
  Link2,
  MessageSquare,
  Mic,
  PanelRight,
  PlusCircle,
  RefreshCw,
  Save,
  ScrollText,
  Shield,
  Sparkles,
  Tags,
  Users,
} from "lucide-react"
import type {
  DocumentAnnotation,
  DocumentWorkspace as DocumentWorkspaceState,
  Matter,
  TranscriptSegment,
  TranscriptionJobResponse,
} from "@/lib/casebuilder/types"
import {
  createEvidence,
  createFact,
  createTimelineEvent,
  createDocumentAnnotation,
  createTranscription,
  createDocumentDownloadUrl,
  extractDocument,
  patchTranscriptSegment,
  patchTranscriptSpeaker,
  promoteDocumentWorkProduct,
  reviewTranscription,
  saveDocumentText,
  syncTranscription,
} from "@/lib/casebuilder/api"
import { matterHref, matterWorkProductHref } from "@/lib/casebuilder/routes"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Textarea } from "@/components/ui/textarea"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { ProcessingBadge } from "./badges"
import { cn } from "@/lib/utils"

interface DocumentWorkspaceProps {
  matter: Matter
  workspace: DocumentWorkspaceState
}

type WorkspaceTab = "links" | "annotations" | "provenance" | "speakers" | "privacy"
type SelectedTextRange = { start: number; end: number; quote: string }
type TranscriptView = "redacted" | "raw"

export function DocumentWorkspace({ matter, workspace: initialWorkspace }: DocumentWorkspaceProps) {
  const router = useRouter()
  const [workspace, setWorkspace] = useState(initialWorkspace)
  const [textDraft, setTextDraft] = useState(initialWorkspace.text_content ?? "")
  const [activeTab, setActiveTab] = useState<WorkspaceTab>("links")
  const [busy, setBusy] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [annotationType, setAnnotationType] = useState("note")
  const [annotationLabel, setAnnotationLabel] = useState("")
  const [annotationNote, setAnnotationNote] = useState("")
  const [annotationPage, setAnnotationPage] = useState("")
  const [selectedTextRange, setSelectedTextRange] = useState<SelectedTextRange | null>(null)
  const [selectedSegmentId, setSelectedSegmentId] = useState<string | null>(null)
  const [transcriptView, setTranscriptView] = useState<TranscriptView>("redacted")

  const document = workspace.document
  const filename = document.filename.toLowerCase()
  const mime = (document.mime_type ?? "").toLowerCase()
  const isPdf = filename.endsWith(".pdf") || mime === "application/pdf"
  const isDocx = filename.endsWith(".docx")
  const isMarkdown = filename.endsWith(".md") || filename.endsWith(".markdown") || mime === "text/markdown"
  const isImage = mime.startsWith("image/") || /\.(png|jpe?g|gif|webp|heic|tiff?)$/.test(filename)
  const isMedia = mime.startsWith("audio/") || mime.startsWith("video/") || /\.(mp3|m4a|wav|mp4|mov|webm)$/.test(filename)
  const canEdit = capabilityEnabled(workspace, "edit")
  const canPromote = capabilityEnabled(workspace, "promote")
  const canAnnotate = capabilityEnabled(workspace, "annotate")
  const contentUrl = workspace.content_url ?? null
  const dirty = textDraft !== (workspace.text_content ?? "")
  const canSave = canEdit && dirty && busy !== "save"
  const activeTranscription = useMemo(
    () => latestTranscription(workspace.transcriptions),
    [workspace.transcriptions],
  )
  const selectedSegment = activeTranscription?.segments.find((segment) => segment.segment_id === selectedSegmentId) ?? null

  const links = useMemo(
    () => [
      ...(document.linked_claim_ids ?? []).map((id) => ({ label: "Claim", id })),
      ...workspace.source_spans.slice(0, 12).map((span) => ({ label: `Span ${span.page ?? 1}`, id: span.source_span_id })),
    ],
    [document.linked_claim_ids, workspace.source_spans],
  )

  async function runAction<T>(
    label: string,
    action: () => Promise<{ data: T | null; error?: string }>,
    onSuccess: (data: T) => void,
  ) {
    setBusy(label)
    setMessage(null)
    setError(null)
    const result = await action()
    setBusy(null)
    if (!result.data) {
      setError(result.error || `${label} failed.`)
      return
    }
    onSuccess(result.data)
  }

  async function onDownload() {
    if (contentUrl) {
      window.open(contentUrl, "_blank", "noopener,noreferrer")
      return
    }
    await runAction(
      "download",
      () => createDocumentDownloadUrl(matter.id, document.document_id),
      (data) => window.open(data.url, "_blank", "noopener,noreferrer"),
    )
  }

  async function onExtract() {
    await runAction(
      "extract",
      () => extractDocument(matter.id, document.document_id),
      (data) => {
        setWorkspace((current) => ({
          ...current,
          document: data.document,
          source_spans: data.source_spans,
          text_content: data.document.extracted_text ?? current.text_content,
        }))
        setTextDraft(data.document.extracted_text ?? textDraft)
        setMessage(data.message)
        router.refresh()
      },
    )
  }

  function replaceTranscription(transcription: TranscriptionJobResponse) {
    setWorkspace((current) => ({
      ...current,
      document: transcription.job.status === "processed"
        ? {
            ...current.document,
            processing_status: "processed",
            status: "processed",
          }
        : current.document,
      transcriptions: [
        ...current.transcriptions.filter(
          (item) => item.job.transcription_job_id !== transcription.job.transcription_job_id,
        ),
        transcription,
      ].sort((a, b) => a.job.created_at.localeCompare(b.job.created_at)),
    }))
    setSelectedSegmentId(transcription.segments[0]?.segment_id ?? null)
  }

  async function onStartTranscription() {
    await runAction(
      "transcribe",
      () => createTranscription(matter.id, document.document_id, { redact_pii: true, speaker_labels: true }),
      (data) => {
        replaceTranscription(data)
        setMessage(data.warnings[0] ?? "Transcription job updated.")
        router.refresh()
      },
    )
  }

  async function onSyncTranscription() {
    if (!activeTranscription) return
    await runAction(
      "sync transcript",
      () => syncTranscription(matter.id, document.document_id, activeTranscription.job.transcription_job_id),
      (data) => {
        replaceTranscription(data)
        setMessage(data.warnings[0] ?? "Transcript status refreshed.")
        router.refresh()
      },
    )
  }

  async function onPatchSegment(segment: TranscriptSegment, text: string) {
    if (!activeTranscription || text === segment.text) return
    await runAction(
      "segment",
      () =>
        patchTranscriptSegment(matter.id, document.document_id, activeTranscription.job.transcription_job_id, segment.segment_id, {
          text,
          review_status: "edited",
        }),
      replaceTranscription,
    )
  }

  async function onPatchSpeaker(speakerId: string, displayName: string) {
    if (!activeTranscription) return
    await runAction(
      "speaker",
      () =>
        patchTranscriptSpeaker(matter.id, document.document_id, activeTranscription.job.transcription_job_id, speakerId, {
          display_name: displayName || null,
        }),
      replaceTranscription,
    )
  }

  async function onReviewTranscript() {
    if (!activeTranscription) return
    await runAction(
      "review transcript",
      () =>
        reviewTranscription(matter.id, document.document_id, activeTranscription.job.transcription_job_id, {
          reviewed_text: transcriptText(activeTranscription.segments, false),
          status: "approved",
        }),
      (data) => {
        replaceTranscription(data)
        setWorkspace((current) => ({ ...current, document: { ...current.document, processing_status: "processed", status: "processed" } }))
        setMessage("Transcript reviewed and committed.")
        router.refresh()
      },
    )
  }

  async function onCreateSegmentAnnotation(segment: TranscriptSegment) {
    await runAction(
      "annotation",
      () =>
        createDocumentAnnotation(matter.id, document.document_id, {
          annotation_type: "note",
          label: `Transcript ${segment.ordinal}`,
          note: segmentText(segment, transcriptView),
          text_range: {
            time_start_ms: segment.time_start_ms,
            time_end_ms: segment.time_end_ms,
            speaker_label: segment.speaker_label,
            quote: segmentText(segment, transcriptView),
          },
        }),
      (annotation) => {
        setWorkspace((current) => ({ ...current, annotations: [...current.annotations, annotation] }))
        setActiveTab("annotations")
        setMessage("Transcript annotation saved.")
      },
    )
  }

  async function onCreateSegmentFact(segment: TranscriptSegment) {
    if (!(activeTranscription?.job.status === "processed" || segment.review_status === "approved")) return
    await runAction(
      "fact",
      () =>
        createFact(matter.id, {
          statement: segment.text,
          status: "alleged",
          confidence: segment.confidence,
          source_document_ids: [document.document_id],
          notes: `Transcript segment ${segment.ordinal} (${formatMs(segment.time_start_ms)}-${formatMs(segment.time_end_ms)}).`,
        }),
      () => {
        setMessage("Fact created from reviewed transcript segment.")
        router.refresh()
      },
    )
  }

  async function onCreateSegmentEvidence(segment: TranscriptSegment) {
    if (!(activeTranscription?.job.status === "processed" || segment.review_status === "approved")) return
    await runAction(
      "evidence",
      () =>
        createEvidence(matter.id, {
          document_id: document.document_id,
          source_span: segment.source_span_id ?? undefined,
          quote: segment.text,
          evidence_type: "audio_transcript",
          confidence: segment.confidence,
          strength: "medium",
        }),
      () => {
        setMessage("Evidence created from reviewed transcript segment.")
        router.refresh()
      },
    )
  }

  async function onCreateSegmentTimeline(segment: TranscriptSegment) {
    if (!(activeTranscription?.job.status === "processed" || segment.review_status === "approved")) return
    await runAction(
      "timeline",
      () =>
        createTimelineEvent(matter.id, {
          date: document.date_observed ?? new Date().toISOString().slice(0, 10),
          title: `Transcript segment ${segment.ordinal}`,
          description: segment.text,
          kind: "discovery",
          source_document_id: document.document_id,
        }),
      () => {
        setMessage("Timeline event created from reviewed transcript segment.")
        router.refresh()
      },
    )
  }

  async function onSave() {
    if (!canSave) return
    await runAction(
      "save",
      () => saveDocumentText(matter.id, document.document_id, { text: textDraft }),
      (data) => {
        setWorkspace((current) => ({
          ...current,
          document: data.document,
          current_version: data.document_version,
          text_content: data.document.extracted_text ?? textDraft,
          warnings: data.warnings,
        }))
        setMessage(data.warnings[0] ?? "Document saved.")
        router.refresh()
      },
    )
  }

  async function onPromote() {
    await runAction(
      "promote",
      () => promoteDocumentWorkProduct(matter.id, document.document_id, { title: document.title, product_type: "memo" }),
      (data) => {
        setMessage(data.warnings[0] ?? "Promoted to work product.")
        router.push(matterWorkProductHref(matter.id, data.work_product.work_product_id))
      },
    )
  }

  async function onCreateAnnotation() {
    if (!canAnnotate) return
    await runAction(
      "annotation",
      () =>
        createDocumentAnnotation(matter.id, document.document_id, {
          annotation_type: annotationType,
          label: annotationLabel || undefined,
          note: annotationNote || undefined,
          page_range: annotationPage ? { page: Number(annotationPage) } : undefined,
          text_range: selectedTextRange
            ? {
                char_start: selectedTextRange.start,
                char_end: selectedTextRange.end,
                quote: selectedTextRange.quote,
              }
            : undefined,
        }),
      (annotation) => {
        setWorkspace((current) => ({ ...current, annotations: [...current.annotations, annotation] }))
        setAnnotationLabel("")
        setAnnotationNote("")
        setAnnotationPage("")
        setSelectedTextRange(null)
        setActiveTab("annotations")
        setMessage("Annotation saved.")
      },
    )
  }

  return (
    <TooltipProvider delayDuration={200}>
      <div className="flex min-h-0 flex-1 flex-col bg-background">
        <header className="border-b bg-card px-4 py-3">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Link href={matterHref(matter.id, "documents")} className="inline-flex items-center gap-1 hover:text-foreground">
              <ArrowLeft className="h-3.5 w-3.5" />
              Documents
            </Link>
          </div>
          <div className="mt-2 flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <FileText className="h-5 w-5 shrink-0 text-muted-foreground" />
                <h1 className="truncate text-xl font-semibold">{document.title}</h1>
              </div>
              <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <Badge variant="outline" className="font-mono text-[10px]">{document.document_type}</Badge>
                <ProcessingBadge status={document.processing_status} />
                {dirty && <Badge variant="secondary">Unsaved</Badge>}
                <span>{document.fileSize}</span>
                <span>{document.storage_status ?? "stored"}</span>
                {workspace.current_version && <span className="font-mono">{workspace.current_version.role}</span>}
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <IconButton label="Download source" onClick={onDownload} disabled={busy === "download"}>
                <Download className="h-4 w-4" />
              </IconButton>
              <IconButton label="Extract text" onClick={onExtract} disabled={busy === "extract"}>
                <Sparkles className="h-4 w-4" />
              </IconButton>
              {(isDocx || isMarkdown || canEdit) && (
                <IconButton label="Save text" onClick={onSave} disabled={!canSave}>
                  <Save className="h-4 w-4" />
                </IconButton>
              )}
              <IconButton label="Promote to work product" onClick={onPromote} disabled={!canPromote || busy === "promote"}>
                <ScrollText className="h-4 w-4" />
              </IconButton>
            </div>
          </div>
          {(message || error || workspace.warnings.length > 0) && (
            <div
              className={cn(
                "mt-3 rounded-md border px-3 py-2 text-xs",
                error ? "border-destructive/30 bg-destructive/5 text-destructive" : "border-border bg-muted/40 text-muted-foreground",
              )}
            >
              {error || message || workspace.warnings[0]}
            </div>
          )}
        </header>

        <div className="grid min-h-0 flex-1 grid-cols-1 lg:grid-cols-[minmax(0,1fr)_360px]">
          <main className="min-h-0 border-r bg-background">
            <DocumentCenterPane
              canEdit={canEdit}
              contentUrl={contentUrl}
              documentTitle={document.title}
              isDocx={isDocx}
              isImage={isImage}
              isMarkdown={isMarkdown}
              isMedia={isMedia}
              isPdf={isPdf}
              activeTranscription={activeTranscription}
              busy={busy}
              selectedSegmentId={selectedSegmentId}
              transcriptReviewed={Boolean(activeTranscription?.job.status === "processed")}
              transcriptView={transcriptView}
              textDraft={textDraft}
              workspace={workspace}
              onCreateAnnotation={onCreateSegmentAnnotation}
              onCreateEvidence={onCreateSegmentEvidence}
              onCreateFact={onCreateSegmentFact}
              onCreateTimeline={onCreateSegmentTimeline}
              onPatchSegment={onPatchSegment}
              onReviewTranscription={onReviewTranscript}
              onSelectSegment={setSelectedSegmentId}
              onStartTranscription={onStartTranscription}
              onSyncTranscription={onSyncTranscription}
              onTextChange={setTextDraft}
              onTextSelection={setSelectedTextRange}
              onSave={onSave}
            />
          </main>

          <aside className="min-h-0 bg-card">
            <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as WorkspaceTab)} className="flex h-full flex-col">
              <div className="border-b px-3 pt-3">
                <TabsList className={cn("grid w-full", isMedia ? "grid-cols-5" : "grid-cols-3")}>
                  <TabsTrigger value="links"><Link2 className="h-3.5 w-3.5" /></TabsTrigger>
                  <TabsTrigger value="annotations"><Highlighter className="h-3.5 w-3.5" /></TabsTrigger>
                  <TabsTrigger value="provenance"><PanelRight className="h-3.5 w-3.5" /></TabsTrigger>
                  {isMedia && <TabsTrigger value="speakers"><Users className="h-3.5 w-3.5" /></TabsTrigger>}
                  {isMedia && <TabsTrigger value="privacy"><Shield className="h-3.5 w-3.5" /></TabsTrigger>}
                </TabsList>
              </div>
              <ScrollArea className="min-h-0 flex-1">
                <TabsContent value="links" className="m-0 space-y-4 p-4">
                  <InspectorSection title="Case Links" icon={<Link2 className="h-4 w-4" />}>
                    {links.length ? (
                      <div className="space-y-2">
                        {links.map((link) => (
                          <div key={`${link.label}:${link.id}`} className="rounded-md border px-3 py-2 text-xs">
                            <div className="font-medium">{link.label}</div>
                            <div className="mt-1 break-all font-mono text-muted-foreground">{link.id}</div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <EmptyLine text="No linked claims or spans yet." />
                    )}
                  </InspectorSection>
                  <InspectorSection title="Capabilities" icon={<Tags className="h-4 w-4" />}>
                    <div className="space-y-2">
                      {workspace.capabilities.map((capability) => (
                        <div key={capability.capability} className="flex items-center justify-between gap-2 text-xs">
                          <span className="capitalize">{capability.capability}</span>
                          <Badge variant={capability.enabled ? "default" : "outline"} className="max-w-44 truncate">
                            {capability.mode}
                          </Badge>
                        </div>
                      ))}
                    </div>
                  </InspectorSection>
                </TabsContent>

                <TabsContent value="annotations" className="m-0 space-y-4 p-4">
                  <InspectorSection title="New Annotation" icon={<MessageSquare className="h-4 w-4" />}>
                    <div className="space-y-3">
                      <Select value={annotationType} onValueChange={setAnnotationType}>
                        <SelectTrigger><SelectValue /></SelectTrigger>
                        <SelectContent>
                          <SelectItem value="note">Note</SelectItem>
                          <SelectItem value="highlight">Highlight</SelectItem>
                          <SelectItem value="redaction">Redaction</SelectItem>
                          <SelectItem value="exhibit_label">Exhibit label</SelectItem>
                          <SelectItem value="fact_link">Fact link</SelectItem>
                          <SelectItem value="citation">Citation</SelectItem>
                          <SelectItem value="issue">Issue</SelectItem>
                        </SelectContent>
                      </Select>
                      <Input value={annotationLabel} onChange={(event) => setAnnotationLabel(event.target.value)} placeholder="Label" />
                      <Input value={annotationPage} onChange={(event) => setAnnotationPage(event.target.value)} inputMode="numeric" placeholder="Page" />
                      <Textarea value={annotationNote} onChange={(event) => setAnnotationNote(event.target.value)} placeholder="Note" rows={4} />
                      {selectedTextRange && (
                        <div className="rounded-md border bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                          <div className="line-clamp-2">{selectedTextRange.quote}</div>
                        </div>
                      )}
                      <Button className="w-full" onClick={onCreateAnnotation} disabled={!canAnnotate || busy === "annotation"}>
                        Save annotation
                      </Button>
                    </div>
                  </InspectorSection>
                  <InspectorSection title="Sidecar Annotations" icon={<Highlighter className="h-4 w-4" />}>
                    <AnnotationList annotations={workspace.annotations} />
                  </InspectorSection>
                </TabsContent>

                <TabsContent value="provenance" className="m-0 space-y-4 p-4">
                  <InspectorSection title="Source" icon={<FileText className="h-4 w-4" />}>
                    <KeyValue label="Document ID" value={document.document_id} />
                    <KeyValue label="Version ID" value={workspace.current_version?.document_version_id ?? document.current_version_id ?? "none"} />
                    <KeyValue label="Object" value={document.object_blob_id ?? "none"} />
                    <KeyValue label="Hash" value={document.file_hash ?? "pending"} />
                    {activeTranscription && (
                      <>
                        <KeyValue label="Transcript" value={activeTranscription.job.transcription_job_id} />
                        <KeyValue label="Provider" value={activeTranscription.job.provider_status ?? activeTranscription.job.provider_mode} />
                      </>
                    )}
                  </InspectorSection>
                  {workspace.docx_manifest && (
                    <InspectorSection title="DOCX Package" icon={<ScrollText className="h-4 w-4" />}>
                      <KeyValue label="Entries" value={String(workspace.docx_manifest.entry_count)} />
                      <KeyValue label="Text parts" value={String(workspace.docx_manifest.text_part_count)} />
                      <KeyValue label="Editable" value={workspace.docx_manifest.editable ? "yes" : "review"} />
                      {workspace.docx_manifest.unsupported_features.length > 0 && (
                        <div className="mt-2 flex flex-wrap gap-1">
                          {workspace.docx_manifest.unsupported_features.map((feature) => (
                            <Badge key={feature} variant="outline">{feature}</Badge>
                          ))}
                        </div>
                      )}
                    </InspectorSection>
                  )}
                </TabsContent>
                {isMedia && (
                  <TabsContent value="speakers" className="m-0 space-y-4 p-4">
                    <InspectorSection title="Speakers" icon={<Users className="h-4 w-4" />}>
                      {activeTranscription?.speakers.length ? (
                        <div className="space-y-3">
                          {activeTranscription.speakers.map((speaker) => (
                            <div key={speaker.speaker_id} className="rounded-md border px-3 py-2">
                              <div className="mb-2 flex items-center justify-between gap-2 text-xs">
                                <span className="font-mono">{speaker.speaker_label}</span>
                                <Badge variant="outline">{speaker.segment_count}</Badge>
                              </div>
                              <Input
                                defaultValue={speaker.display_name ?? ""}
                                placeholder="Speaker name"
                                onBlur={(event) => onPatchSpeaker(speaker.speaker_id, event.currentTarget.value)}
                              />
                            </div>
                          ))}
                        </div>
                      ) : (
                        <EmptyLine text="No diarized speakers yet." />
                      )}
                    </InspectorSection>
                  </TabsContent>
                )}
                {isMedia && (
                  <TabsContent value="privacy" className="m-0 space-y-4 p-4">
                    <InspectorSection title="Transcript View" icon={<Shield className="h-4 w-4" />}>
                      <div className="grid grid-cols-2 gap-2">
                        <Button size="sm" variant={transcriptView === "redacted" ? "default" : "outline"} onClick={() => setTranscriptView("redacted")}>
                          Redacted
                        </Button>
                        <Button size="sm" variant={transcriptView === "raw" ? "default" : "outline"} onClick={() => setTranscriptView("raw")}>
                          Raw
                        </Button>
                      </div>
                      <div className="mt-3 space-y-2 text-xs text-muted-foreground">
                        <KeyValue label="Raw" value={activeTranscription?.raw_artifact_version?.document_version_id ?? "none"} />
                        <KeyValue label="Redacted" value={activeTranscription?.redacted_artifact_version?.document_version_id ?? "none"} />
                        <KeyValue label="Reviewed" value={activeTranscription?.reviewed_document_version?.document_version_id ?? "none"} />
                      </div>
                    </InspectorSection>
                    <InspectorSection title="Selected Segment" icon={<Captions className="h-4 w-4" />}>
                      {selectedSegment ? (
                        <div className="space-y-2 text-xs">
                          <KeyValue label="Segment" value={selectedSegment.segment_id} />
                          <KeyValue label="Time" value={`${formatMs(selectedSegment.time_start_ms)}-${formatMs(selectedSegment.time_end_ms)}`} />
                          <KeyValue label="Span" value={selectedSegment.source_span_id ?? "pending"} />
                        </div>
                      ) : (
                        <EmptyLine text="Select a segment to inspect timestamps." />
                      )}
                    </InspectorSection>
                  </TabsContent>
                )}
              </ScrollArea>
            </Tabs>
          </aside>
        </div>
      </div>
    </TooltipProvider>
  )
}

function DocumentCenterPane({
  activeTranscription,
  busy,
  canEdit,
  contentUrl,
  documentTitle,
  isDocx,
  isImage,
  isMarkdown,
  isMedia,
  isPdf,
  selectedSegmentId,
  transcriptReviewed,
  transcriptView,
  textDraft,
  workspace,
  onCreateAnnotation,
  onCreateEvidence,
  onCreateFact,
  onCreateTimeline,
  onPatchSegment,
  onReviewTranscription,
  onSelectSegment,
  onStartTranscription,
  onSyncTranscription,
  onTextChange,
  onTextSelection,
  onSave,
}: {
  activeTranscription: TranscriptionJobResponse | null
  busy: string | null
  canEdit: boolean
  contentUrl: string | null
  documentTitle: string
  isDocx: boolean
  isImage: boolean
  isMarkdown: boolean
  isMedia: boolean
  isPdf: boolean
  selectedSegmentId: string | null
  transcriptReviewed: boolean
  transcriptView: TranscriptView
  textDraft: string
  workspace: DocumentWorkspaceState
  onCreateAnnotation: (segment: TranscriptSegment) => void
  onCreateEvidence: (segment: TranscriptSegment) => void
  onCreateFact: (segment: TranscriptSegment) => void
  onCreateTimeline: (segment: TranscriptSegment) => void
  onPatchSegment: (segment: TranscriptSegment, text: string) => void
  onReviewTranscription: () => void
  onSelectSegment: (segmentId: string | null) => void
  onStartTranscription: () => void
  onSyncTranscription: () => void
  onTextChange: (value: string) => void
  onTextSelection: (range: SelectedTextRange | null) => void
  onSave: () => void
}) {
  if (isPdf && contentUrl) {
    return <iframe title={documentTitle} src={`${contentUrl}#view=FitH`} className="h-full min-h-[640px] w-full bg-background" />
  }
  if (isImage && contentUrl) {
    return (
      <div className="flex h-full min-h-[640px] items-center justify-center bg-muted/30 p-6">
        <div className="relative h-full min-h-[520px] w-full max-w-5xl">
          <Image src={contentUrl} alt={documentTitle} fill unoptimized sizes="100vw" className="rounded-md border object-contain" />
        </div>
      </div>
    )
  }
  if (isMedia && contentUrl) {
    return (
      <MediaTranscriptPane
        activeTranscription={activeTranscription}
        busy={busy}
        contentUrl={contentUrl}
        documentTitle={documentTitle}
        selectedSegmentId={selectedSegmentId}
        transcriptReviewed={transcriptReviewed}
        transcriptView={transcriptView}
        workspace={workspace}
        onCreateAnnotation={onCreateAnnotation}
        onCreateEvidence={onCreateEvidence}
        onCreateFact={onCreateFact}
        onCreateTimeline={onCreateTimeline}
        onPatchSegment={onPatchSegment}
        onReviewTranscription={onReviewTranscription}
        onSelectSegment={onSelectSegment}
        onStartTranscription={onStartTranscription}
        onSyncTranscription={onSyncTranscription}
      />
    )
  }
  if (isDocx || isMarkdown || textDraft) {
    return (
      <div className="flex h-full min-h-[640px] flex-col">
        <div className="flex items-center justify-between border-b px-4 py-2 text-xs text-muted-foreground">
          <span>{isDocx ? "DOCX OOXML text map" : isMarkdown ? "Markdown source" : "Text source"}</span>
          <Badge variant={canEdit ? "default" : "outline"}>{canEdit ? "Editable" : "Read only"}</Badge>
        </div>
        <Textarea
          value={textDraft}
          onChange={(event) => onTextChange(event.target.value)}
          onSelect={(event) => {
            const target = event.currentTarget
            const start = target.selectionStart
            const end = target.selectionEnd
            onTextSelection(end > start ? { start, end, quote: textDraft.slice(start, end) } : null)
          }}
          onKeyDown={(event) => {
            if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
              event.preventDefault()
              onSave()
            }
          }}
          readOnly={!canEdit}
          className="min-h-0 flex-1 resize-none rounded-none border-0 bg-background p-6 font-mono text-sm leading-6 shadow-none focus-visible:ring-0"
        />
      </div>
    )
  }
  return (
    <div className="flex h-full min-h-[640px] items-center justify-center p-8">
      <div className="max-w-md text-center text-sm text-muted-foreground">
        <FileText className="mx-auto mb-3 h-10 w-10" />
        <div className="font-medium text-foreground">Unsupported preview</div>
        <p className="mt-2">The original file is stored and can still be annotated, extracted when supported, or downloaded.</p>
      </div>
    </div>
  )
}

function MediaTranscriptPane({
  activeTranscription,
  busy,
  contentUrl,
  documentTitle,
  selectedSegmentId,
  transcriptReviewed,
  transcriptView,
  workspace,
  onCreateAnnotation,
  onCreateEvidence,
  onCreateFact,
  onCreateTimeline,
  onPatchSegment,
  onReviewTranscription,
  onSelectSegment,
  onStartTranscription,
  onSyncTranscription,
}: {
  activeTranscription: TranscriptionJobResponse | null
  busy: string | null
  contentUrl: string
  documentTitle: string
  selectedSegmentId: string | null
  transcriptReviewed: boolean
  transcriptView: TranscriptView
  workspace: DocumentWorkspaceState
  onCreateAnnotation: (segment: TranscriptSegment) => void
  onCreateEvidence: (segment: TranscriptSegment) => void
  onCreateFact: (segment: TranscriptSegment) => void
  onCreateTimeline: (segment: TranscriptSegment) => void
  onPatchSegment: (segment: TranscriptSegment, text: string) => void
  onReviewTranscription: () => void
  onSelectSegment: (segmentId: string | null) => void
  onStartTranscription: () => void
  onSyncTranscription: () => void
}) {
  const audioRef = useRef<HTMLAudioElement | null>(null)
  const videoRef = useRef<HTMLVideoElement | null>(null)
  const isAudio = workspace.document.mime_type?.startsWith("audio/")
  const segments = activeTranscription?.segments ?? []
  const canCreateCaseItems = activeTranscription?.job.status === "processed"

  function jump(segment: TranscriptSegment) {
    onSelectSegment(segment.segment_id)
    const media = audioRef.current ?? videoRef.current
    if (media) {
      media.currentTime = segment.time_start_ms / 1000
      void media.play().catch(() => undefined)
    }
  }

  return (
    <div className="grid h-full min-h-[640px] grid-rows-[auto_minmax(0,1fr)] bg-background">
      <div className="border-b bg-muted/30 p-4">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-center">
          <div className="min-w-0 flex-1">
            {isAudio ? (
              <audio ref={audioRef} controls src={contentUrl} className="w-full" />
            ) : (
              <video ref={videoRef} controls src={contentUrl} className="max-h-[280px] w-full rounded-md border bg-black object-contain" />
            )}
          </div>
          <div className="flex shrink-0 flex-wrap gap-2">
            <Button size="sm" onClick={onStartTranscription} disabled={busy === "transcribe"}>
              <Mic className="mr-2 h-4 w-4" />
              Transcribe
            </Button>
            <Button size="sm" variant="outline" onClick={onSyncTranscription} disabled={!activeTranscription || busy === "sync transcript"}>
              <RefreshCw className="mr-2 h-4 w-4" />
              Sync
            </Button>
            <Button size="sm" variant="outline" onClick={onReviewTranscription} disabled={!activeTranscription || segments.length === 0 || busy === "review transcript"}>
              <CheckCircle2 className="mr-2 h-4 w-4" />
              Review
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => activeTranscription?.caption_vtt && downloadText(`${documentTitle}.vtt`, activeTranscription.caption_vtt, "text/vtt")}
              disabled={!activeTranscription?.caption_vtt}
            >
              <Captions className="mr-2 h-4 w-4" />
              VTT
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => activeTranscription?.caption_srt && downloadText(`${documentTitle}.srt`, activeTranscription.caption_srt, "application/x-subrip")}
              disabled={!activeTranscription?.caption_srt}
            >
              <Captions className="mr-2 h-4 w-4" />
              SRT
            </Button>
          </div>
        </div>
        {activeTranscription && (
          <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <Badge variant={activeTranscription.job.status === "failed" ? "destructive" : "outline"}>{activeTranscription.job.status}</Badge>
            <span>{activeTranscription.job.provider_mode}</span>
            <span>{activeTranscription.job.segment_count || segments.length} segment(s)</span>
            <span>{activeTranscription.job.speaker_count} speaker(s)</span>
            <span>{transcriptView === "redacted" ? "redacted view" : "raw review view"}</span>
          </div>
        )}
      </div>
      <ScrollArea className="min-h-0">
        <div className="space-y-3 p-4">
          {!activeTranscription && (
            <div className="rounded-md border border-dashed p-8 text-center text-sm text-muted-foreground">
              <Mic className="mx-auto mb-3 h-9 w-9" />
              <div className="font-medium text-foreground">No transcript job yet</div>
              <p className="mt-2">Start transcription from this uploaded media file.</p>
            </div>
          )}
          {activeTranscription?.job.status === "provider_disabled" && (
            <div className="rounded-md border border-warning/30 bg-warning/5 px-3 py-2 text-xs text-warning">
              AssemblyAI is disabled or missing an API key on the API server.
            </div>
          )}
          {segments.map((segment) => {
            const selected = segment.segment_id === selectedSegmentId
            const segmentReviewed = canCreateCaseItems || segment.review_status === "approved" || transcriptReviewed
            return (
              <section key={segment.segment_id} className={cn("rounded-md border bg-card p-3", selected && "border-primary ring-1 ring-primary")}>
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <button className="text-left text-xs font-medium text-foreground" onClick={() => jump(segment)}>
                    {segment.speaker_name || segment.speaker_label || "Speaker"} · {formatMs(segment.time_start_ms)}-{formatMs(segment.time_end_ms)}
                  </button>
                  <div className="flex flex-wrap items-center gap-1">
                    <Badge variant="outline">{segment.review_status}</Badge>
                    <IconButton label="Annotate segment" onClick={() => onCreateAnnotation(segment)}>
                      <Highlighter className="h-4 w-4" />
                    </IconButton>
                    <IconButton label="Create fact" onClick={() => onCreateFact(segment)} disabled={!segmentReviewed}>
                      <PlusCircle className="h-4 w-4" />
                    </IconButton>
                    <IconButton label="Create evidence" onClick={() => onCreateEvidence(segment)} disabled={!segmentReviewed}>
                      <Link2 className="h-4 w-4" />
                    </IconButton>
                    <IconButton label="Create timeline event" onClick={() => onCreateTimeline(segment)} disabled={!segmentReviewed}>
                      <ScrollText className="h-4 w-4" />
                    </IconButton>
                  </div>
                </div>
                <Textarea
                  key={`${segment.segment_id}:${transcriptView}`}
                  defaultValue={segmentText(segment, transcriptView)}
                  readOnly={transcriptView === "redacted"}
                  onFocus={() => onSelectSegment(segment.segment_id)}
                  onBlur={(event) => {
                    if (transcriptView === "raw") onPatchSegment(segment, event.currentTarget.value)
                  }}
                  rows={3}
                  className="mt-3 resize-y text-sm leading-6"
                />
              </section>
            )
          })}
        </div>
      </ScrollArea>
    </div>
  )
}

function IconButton({
  children,
  label,
  disabled,
  onClick,
}: {
  children: ReactNode
  label: string
  disabled?: boolean
  onClick: () => void
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant="outline" size="icon" disabled={disabled} onClick={onClick}>
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}

function InspectorSection({ children, icon, title }: { children: ReactNode; icon: ReactNode; title: string }) {
  return (
    <section>
      <div className="mb-2 flex items-center gap-2 text-sm font-medium">
        {icon}
        {title}
      </div>
      {children}
      <Separator className="mt-4" />
    </section>
  )
}

function AnnotationList({ annotations }: { annotations: DocumentAnnotation[] }) {
  if (!annotations.length) return <EmptyLine text="No annotations yet." />
  return (
    <div className="space-y-2">
      {annotations.map((annotation) => (
        <div key={annotation.annotation_id} className="rounded-md border px-3 py-2 text-xs">
          <div className="flex items-center justify-between gap-2">
            <span className="font-medium">{annotation.label}</span>
            <Badge variant="outline">{annotation.annotation_type}</Badge>
          </div>
          {annotation.note && <p className="mt-2 leading-5 text-muted-foreground">{annotation.note}</p>}
          <div className="mt-2 font-mono text-muted-foreground">
            {annotation.page_range?.page ? `page ${annotation.page_range.page}` : annotation.annotation_id}
          </div>
        </div>
      ))}
    </div>
  )
}

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[92px_minmax(0,1fr)] gap-2 py-1 text-xs">
      <span className="text-muted-foreground">{label}</span>
      <span className="break-all font-mono">{value}</span>
    </div>
  )
}

function EmptyLine({ text }: { text: string }) {
  return <div className="rounded-md border border-dashed px-3 py-4 text-center text-xs text-muted-foreground">{text}</div>
}

function capabilityEnabled(workspace: DocumentWorkspaceState, name: string) {
  return workspace.capabilities.some((capability) => capability.capability === name && capability.enabled)
}

function latestTranscription(transcriptions: TranscriptionJobResponse[]) {
  if (!transcriptions.length) return null
  return [...transcriptions].sort((a, b) => b.job.created_at.localeCompare(a.job.created_at))[0]
}

function segmentText(segment: TranscriptSegment, view: TranscriptView) {
  if (view === "redacted") return segment.redacted_text || segment.text
  return segment.text
}

function transcriptText(segments: TranscriptSegment[], redacted: boolean) {
  return segments
    .map((segment) => `${segment.speaker_name || segment.speaker_label || "Speaker"}: ${redacted ? segment.redacted_text || segment.text : segment.text}`)
    .join("\n")
}

function formatMs(ms: number) {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000))
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  return `${minutes}:${seconds.toString().padStart(2, "0")}`
}

function downloadText(filename: string, text: string, type: string) {
  const blob = new Blob([text], { type })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement("a")
  anchor.href = url
  anchor.download = filename
  anchor.click()
  URL.revokeObjectURL(url)
}
