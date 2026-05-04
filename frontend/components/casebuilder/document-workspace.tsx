"use client"

import { useMemo, useRef, useState, type Dispatch, type ReactNode, type RefObject, type SetStateAction } from "react"
import Image from "next/image"
import Link from "next/link"
import { useRouter } from "next/navigation"
import {
  ArrowLeft,
  CalendarClock,
  Captions,
  CheckCircle2,
  Download,
  FileText,
  GitGraphIcon,
  Highlighter,
  Link2,
  MessageSquare,
  Mic,
  Network,
  PanelRight,
  PlusCircle,
  RefreshCw,
  Save,
  Search,
  ScrollText,
  Shield,
  Sparkles,
  Tags,
  Users,
} from "lucide-react"
import type {
  CaseBuilderEffectiveSettings,
  DocumentAnnotation,
  CaseBuilderEmbeddingSearchResult,
  MarkdownAstNode,
  DocumentWorkspace as DocumentWorkspaceState,
  Matter,
  TranscriptionJob,
  TranscriptSegment,
  TranscriptionJobResponse,
} from "@/lib/casebuilder/types"
import {
  type CreateTranscriptionInput,
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
  runDocumentEmbeddings,
  saveDocumentText,
  searchMatterEmbeddings,
  suggestTimeline,
  syncTranscription,
} from "@/lib/casebuilder/api"
import { matterHref, matterTimelineHref, matterWorkProductHref } from "@/lib/casebuilder/routes"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import { Switch } from "@/components/ui/switch"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Textarea } from "@/components/ui/textarea"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { ProcessingBadge } from "./badges"
import { cn } from "@/lib/utils"

interface DocumentWorkspaceProps {
  matter: Matter
  workspace: DocumentWorkspaceState
  settings?: CaseBuilderEffectiveSettings | null
}

type WorkspaceTab = "links" | "annotations" | "provenance" | "markdown_graph" | "speakers" | "privacy"
type SelectedTextRange = { start: number; end: number; quote: string }
type TranscriptView = "redacted" | "raw"
type TranscriptSegmentDrafts = Record<string, Partial<Record<TranscriptView, string>>>
type SpeakerMode = "auto" | "exact" | "range"
type PromptMode = "default" | "preset" | "custom" | "keyterms"

type TranscriptionSettings = {
  redactPii: boolean
  speakerLabels: boolean
  speakerMode: SpeakerMode
  speakersExpected: string
  minSpeakersExpected: string
  maxSpeakersExpected: string
  promptMode: PromptMode
  promptPreset: string
  prompt: string
  keyterms: string
  wordSearchTerms: string
  removeAudioTags: boolean
}

const promptPresetOptions = [
  { value: "verbatim_multilingual", label: "Verbatim multilingual" },
  { value: "unclear_masked", label: "Unclear as [masked]" },
  { value: "unclear", label: "Unclear as [unclear]" },
  { value: "legal", label: "Legal" },
  { value: "medical", label: "Medical" },
  { value: "financial", label: "Financial" },
  { value: "technical", label: "Technical" },
  { value: "code_switching", label: "Code switching" },
  { value: "customer_support", label: "Customer support" },
]

const defaultTranscriptionSettings: TranscriptionSettings = {
  redactPii: true,
  speakerLabels: true,
  speakerMode: "auto",
  speakersExpected: "",
  minSpeakersExpected: "",
  maxSpeakersExpected: "",
  promptMode: "default",
  promptPreset: "unclear",
  prompt: "",
  keyterms: "",
  wordSearchTerms: "",
  removeAudioTags: true,
}

function transcriptViewFromSettings(settings?: CaseBuilderEffectiveSettings | null): TranscriptView {
  return settings?.transcript_default_view === "raw" ? "raw" : "redacted"
}

function transcriptionSettingsFromEffectiveSettings(settings?: CaseBuilderEffectiveSettings | null): TranscriptionSettings {
  const promptPreset = settings?.transcript_prompt_preset?.trim() || defaultTranscriptionSettings.promptPreset
  return {
    ...defaultTranscriptionSettings,
    redactPii: settings?.transcript_redact_pii ?? defaultTranscriptionSettings.redactPii,
    speakerLabels: settings?.transcript_speaker_labels ?? defaultTranscriptionSettings.speakerLabels,
    promptMode: settings?.transcript_prompt_preset ? "preset" : defaultTranscriptionSettings.promptMode,
    promptPreset,
    removeAudioTags: settings?.transcript_remove_audio_tags ?? defaultTranscriptionSettings.removeAudioTags,
  }
}

export function DocumentWorkspace({ matter, workspace: initialWorkspace, settings }: DocumentWorkspaceProps) {
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
  const [transcriptView, setTranscriptView] = useState<TranscriptView>(() => transcriptViewFromSettings(settings))
  const [transcriptSegmentDrafts, setTranscriptSegmentDrafts] = useState<TranscriptSegmentDrafts>({})
  const [timelineDateDrafts, setTimelineDateDrafts] = useState<Record<string, string>>({})
  const [transcriptionSettings, setTranscriptionSettings] = useState<TranscriptionSettings>(() =>
    transcriptionSettingsFromEffectiveSettings(settings),
  )
  const reviewInFlightRef = useRef(false)
  const reviewGenerationRef = useRef(0)
  const textAreaRef = useRef<HTMLTextAreaElement | null>(null)

  const document = workspace.document
  const filename = document.filename.toLowerCase()
  const mime = (document.mime_type ?? "").toLowerCase()
  const isPdf = filename.endsWith(".pdf") || mime === "application/pdf"
  const isMarkdown = filename.endsWith(".md") || filename.endsWith(".markdown") || mime === "text/markdown"
  const isImage = mime.startsWith("image/") || /\.(png|jpe?g|gif|webp|heic|tiff?)$/.test(filename)
  const isMedia = mime.startsWith("audio/") || mime.startsWith("video/") || /\.(mp3|m4a|wav|mp4|mov|webm)$/.test(filename)
  const canEdit = capabilityEnabled(workspace, "edit")
  const canPromote = capabilityEnabled(workspace, "promote")
  const canAnnotate = capabilityEnabled(workspace, "annotate")
  const canExtract = capabilityEnabled(workspace, "extract")
  const contentUrl = workspace.content_url ?? null
  const dirty = textDraft !== (workspace.text_content ?? "")
  const canSave = canEdit && dirty && busy !== "save"
  const activeTranscription = useMemo(
    () => latestTranscription(workspace.transcriptions),
    [workspace.transcriptions],
  )
  const selectedSegment = activeTranscription?.segments.find((segment) => segment.segment_id === selectedSegmentId) ?? null

  function selectMarkdownNode(node: MarkdownAstNode) {
    const start = node.char_start ?? node.byte_start
    const end = node.char_end ?? node.byte_end
    if (start == null || end == null || end <= start) return
    const quote = textDraft.slice(start, end)
    setSelectedTextRange({ start, end, quote })
    const syncEditorSelection = () => {
      const textArea = textAreaRef.current
      if (!textArea) return
      textArea.focus()
      textArea.setSelectionRange(start, end)
      const line = textDraft.slice(0, start).split("\n").length
      textArea.scrollTop = Math.max(0, (line - 4) * 24)
    }
    if (typeof requestAnimationFrame === "function") {
      requestAnimationFrame(syncEditorSelection)
    } else {
      syncEditorSelection()
    }
  }

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

  function onSegmentDraftChange(segmentId: string, view: TranscriptView, text: string) {
    setTranscriptSegmentDrafts((current) => ({
      ...current,
      [segmentId]: {
        ...current[segmentId],
        [view]: text,
      },
    }))
  }

  function clearSegmentDraft(segmentId: string, view: TranscriptView) {
    setTranscriptSegmentDrafts((current) => {
      const segmentDraft = current[segmentId]
      if (!segmentDraft || segmentDraft[view] === undefined) return current
      const nextSegmentDraft = { ...segmentDraft }
      delete nextSegmentDraft[view]
      const next = { ...current }
      if (Object.keys(nextSegmentDraft).length) {
        next[segmentId] = nextSegmentDraft
      } else {
        delete next[segmentId]
      }
      return next
    })
  }

  function clearFlushedSegmentDrafts(drafts: TranscriptSegmentDrafts) {
    setTranscriptSegmentDrafts((current) => {
      const next = { ...current }
      for (const [segmentId, segmentDraft] of Object.entries(drafts)) {
        const nextSegmentDraft = { ...next[segmentId] }
        for (const view of Object.keys(segmentDraft) as TranscriptView[]) {
          delete nextSegmentDraft[view]
        }
        if (Object.keys(nextSegmentDraft).length) {
          next[segmentId] = nextSegmentDraft
        } else {
          delete next[segmentId]
        }
      }
      return next
    })
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
    if (!canExtract) return
    await runAction(
      "extract",
      () => extractDocument(matter.id, document.document_id),
      (data) => {
        setWorkspace((current) => ({
          ...current,
          document: data.document,
          source_spans: data.source_spans,
          markdown_ast_document: data.markdown_ast_document,
          markdown_ast_nodes: data.markdown_ast_nodes,
          markdown_semantic_units: data.markdown_semantic_units,
          text_chunks: data.text_chunks,
          evidence_spans: data.evidence_spans,
          entity_mentions: data.entity_mentions,
          entities: data.entities,
          search_index_records: data.search_index_records,
          embedding_runs: data.embedding_run ? [data.embedding_run, ...current.embedding_runs] : current.embedding_runs,
          proposed_facts: data.proposed_facts,
          timeline_suggestions: data.timeline_suggestions,
          text_content: data.document.extracted_text ?? current.text_content,
        }))
        setTextDraft(data.document.extracted_text ?? textDraft)
        setMessage(data.message)
        router.refresh()
      },
    )
  }

  async function onRunEmbeddings() {
    await runAction(
      "embeddings",
      () => runDocumentEmbeddings(matter.id, document.document_id),
      (run) => {
        setWorkspace((current) => ({
          ...current,
          embedding_runs: [run, ...current.embedding_runs.filter((item) => item.embedding_run_id !== run.embedding_run_id)],
        }))
        setMessage(`Embedding run ${run.status}: ${run.embedded_count} record${run.embedded_count === 1 ? "" : "s"}.`)
        router.refresh()
      },
    )
  }

  async function onSuggestTimeline() {
    if (!canExtract) return
    await runAction(
      "timeline suggestions",
      () => suggestTimeline(matter.id, { document_ids: [document.document_id], limit: 50 }),
      (data) => {
        const first = data.suggestions[0]
        const providerMode = data.agent_run?.provider_mode ?? data.mode
        setMessage(`${data.suggestions.length} timeline suggestion${data.suggestions.length === 1 ? "" : "s"} ready for review (${providerMode}).`)
        router.push(
          matterTimelineHref(matter.id, {
            suggestionId: first?.suggestion_id,
            status: first ? "suggested" : undefined,
            sourceType: first?.source_type,
            agentRunId: first?.agent_run_id ?? data.agent_run?.agent_run_id,
          }),
        )
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
      () => createTranscription(matter.id, document.document_id, buildCreateTranscriptionInput(transcriptionSettings)),
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

  async function flushTranscriptSegmentDrafts(
    transcription: TranscriptionJobResponse,
    drafts: TranscriptSegmentDrafts,
  ): Promise<TranscriptionJobResponse | null> {
    const patches: Array<{
      segment: TranscriptSegment
      view: TranscriptView
      text: string
      patch: { text?: string; redacted_text?: string; review_status: "edited" }
    }> = []

    for (const segment of transcription.segments) {
      const draft = drafts[segment.segment_id]
      if (!draft) continue
      if (draft.redacted !== undefined && draft.redacted !== segmentText(segment, "redacted")) {
        patches.push({
          segment,
          view: "redacted",
          text: draft.redacted,
          patch: { redacted_text: draft.redacted, review_status: "edited" },
        })
      }
      if (draft.raw !== undefined && draft.raw !== segmentText(segment, "raw")) {
        patches.push({
          segment,
          view: "raw",
          text: draft.raw,
          patch: { text: draft.raw, review_status: "edited" },
        })
      }
    }

    if (!patches.length) {
      clearFlushedSegmentDrafts(drafts)
      return transcription
    }

    let latest = transcription
    const flushedDrafts: TranscriptSegmentDrafts = {}
    for (const item of patches) {
      const result = await patchTranscriptSegment(
        matter.id,
        document.document_id,
        transcription.job.transcription_job_id,
        item.segment.segment_id,
        item.patch,
      )
      if (!result.data) {
        setError(result.error || "Transcript segment save failed.")
        return null
      }
      latest = result.data
      flushedDrafts[item.segment.segment_id] = {
        ...flushedDrafts[item.segment.segment_id],
        [item.view]: item.text,
      }
    }
    replaceTranscription(latest)
    clearFlushedSegmentDrafts(flushedDrafts)
    return latest
  }

  async function onPatchSegment(segment: TranscriptSegment, text: string, view: TranscriptView) {
    if (!activeTranscription || text === segmentText(segment, view)) {
      clearSegmentDraft(segment.segment_id, view)
      return
    }
    const patch =
      view === "redacted"
        ? { redacted_text: text, review_status: "edited" }
        : { text, review_status: "edited" }
    const startedReviewGeneration = reviewGenerationRef.current
    setBusy("segment")
    setMessage(null)
    setError(null)
    const result = await patchTranscriptSegment(
      matter.id,
      document.document_id,
      activeTranscription.job.transcription_job_id,
      segment.segment_id,
      patch,
    )
    setBusy((current) => (current === "segment" ? null : current))
    if (!result.data) {
      setError(result.error || "Transcript segment save failed.")
      return
    }
    if (!reviewInFlightRef.current && startedReviewGeneration === reviewGenerationRef.current) {
      replaceTranscription(result.data)
    }
    clearSegmentDraft(segment.segment_id, view)
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
    if (
      transcriptView === "raw" &&
      !window.confirm("Commit the raw transcript for review/export? Redacted review is the default case-use surface.")
    ) {
      setTranscriptView("redacted")
      setMessage("Raw review skipped. Redacted transcript remains the default review surface.")
      return
    }
    const reviewView: TranscriptView = transcriptView === "raw" ? "raw" : "redacted"
    const draftsSnapshot = transcriptSegmentDrafts
    const reviewedText = transcriptText(activeTranscription.segments, reviewView, draftsSnapshot)
    reviewGenerationRef.current += 1
    reviewInFlightRef.current = true
    setBusy("review transcript")
    setMessage(null)
    setError(null)
    try {
      const syncedTranscription = await flushTranscriptSegmentDrafts(activeTranscription, draftsSnapshot)
      if (!syncedTranscription) {
        return
      }
      const result = await reviewTranscription(matter.id, document.document_id, syncedTranscription.job.transcription_job_id, {
        reviewed_text: reviewedText,
        review_surface: reviewView,
        status: "approved",
      })
      if (!result.data) {
        setError(result.error || "review transcript failed.")
        return
      }
      replaceTranscription(result.data)
      setWorkspace((current) => ({ ...current, document: { ...current.document, processing_status: "processed", status: "processed" } }))
      setMessage("Transcript reviewed and committed.")
      router.refresh()
    } finally {
      reviewInFlightRef.current = false
      setBusy((current) => (current === "review transcript" ? null : current))
    }
  }

  async function onCreateSegmentAnnotation(segment: TranscriptSegment) {
    const quote = segmentDraftText(segment, transcriptView, transcriptSegmentDrafts)
    await runAction(
      "annotation",
      () =>
        createDocumentAnnotation(matter.id, document.document_id, {
          annotation_type: "note",
          label: `Transcript ${segment.ordinal}`,
          note: quote,
          text_range: {
            time_start_ms: segment.time_start_ms,
            time_end_ms: segment.time_end_ms,
            speaker_label: segment.speaker_label,
            quote,
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
    const quote = segmentDraftText(segment, transcriptView, transcriptSegmentDrafts)
    await runAction(
      "fact",
      () =>
        createFact(matter.id, {
          statement: quote,
          status: "alleged",
          confidence: segment.confidence,
          source_document_ids: [document.document_id],
          source_span_ids: segment.source_span_id ? [segment.source_span_id] : [],
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
    const quote = segmentDraftText(segment, transcriptView, transcriptSegmentDrafts)
    await runAction(
      "evidence",
      () =>
        createEvidence(matter.id, {
          document_id: document.document_id,
          source_span: segment.source_span_id ?? undefined,
          quote,
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

  async function onCreateSegmentTimeline(segment: TranscriptSegment, suppliedDate?: string) {
    if (!(activeTranscription?.job.status === "processed" || segment.review_status === "approved")) return
    const eventDate = (suppliedDate || document.date_observed || "").trim()
    if (!eventDate) {
      setMessage("Add an event date for this transcript segment before creating a timeline event.")
      return
    }
    const description = segmentDraftText(segment, transcriptView, transcriptSegmentDrafts)
    await runAction(
      "timeline",
      () =>
        createTimelineEvent(matter.id, {
          date: eventDate,
          title: `Transcript segment ${segment.ordinal}`,
          description,
          kind: "other",
          source_document_id: document.document_id,
          source_span_ids: segment.source_span_id ? [segment.source_span_id] : [],
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
    if (!canPromote) return
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
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden bg-background">
        <header className="shrink-0 border-b bg-card px-4 py-3">
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
              {isMedia ? (
                <>
                  <IconButton label="Transcribe media" onClick={onStartTranscription} disabled={!canExtract || busy === "transcribe"}>
                    <Mic className="h-4 w-4" />
                  </IconButton>
                  <IconButton label="Sync transcript" onClick={onSyncTranscription} disabled={!canExtract || !activeTranscription || busy === "sync transcript"}>
                    <RefreshCw className="h-4 w-4" />
                  </IconButton>
                  <IconButton
                    label={transcriptView === "raw" ? "Review raw transcript" : "Review redacted transcript"}
                    onClick={onReviewTranscript}
                    disabled={!canExtract || !activeTranscription || activeTranscription.segments.length === 0 || busy === "review transcript"}
                  >
                    <CheckCircle2 className="h-4 w-4" />
                  </IconButton>
                </>
              ) : (
                <IconButton label="Extract text" onClick={onExtract} disabled={!canExtract || busy === "extract"}>
                  <Sparkles className="h-4 w-4" />
                </IconButton>
              )}
              <IconButton label="Suggest timeline" onClick={onSuggestTimeline} disabled={!canExtract || busy === "timeline suggestions"}>
                <CalendarClock className="h-4 w-4" />
              </IconButton>
              {canEdit && (
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

        <div className="grid min-h-0 flex-1 grid-cols-1 grid-rows-[minmax(0,1fr)_minmax(18rem,40vh)] overflow-hidden lg:grid-cols-[minmax(0,1fr)_22rem] lg:grid-rows-[minmax(0,1fr)] xl:grid-cols-[minmax(0,1fr)_24rem]">
          <main className="min-h-0 overflow-hidden border-r bg-background">
            <DocumentCenterPane
              canEdit={canEdit}
              canExtract={canExtract}
              contentUrl={contentUrl}
              documentTitle={document.title}
              isImage={isImage}
              isMarkdown={isMarkdown}
              isMedia={isMedia}
              isPdf={isPdf}
              activeTranscription={activeTranscription}
              busy={busy}
              selectedSegmentId={selectedSegmentId}
              timelineDateDrafts={timelineDateDrafts}
              transcriptSegmentDrafts={transcriptSegmentDrafts}
              transcriptReviewed={Boolean(activeTranscription?.job.status === "processed")}
              transcriptionSettings={transcriptionSettings}
              transcriptView={transcriptView}
              textDraft={textDraft}
              textAreaRef={textAreaRef}
              workspace={workspace}
              onCreateAnnotation={onCreateSegmentAnnotation}
              onCreateEvidence={onCreateSegmentEvidence}
              onCreateFact={onCreateSegmentFact}
              onCreateTimeline={onCreateSegmentTimeline}
              onSegmentDraftChange={onSegmentDraftChange}
              onPatchSegment={onPatchSegment}
              onReviewTranscription={onReviewTranscript}
              onSelectSegment={setSelectedSegmentId}
              onStartTranscription={onStartTranscription}
              onTimelineDateDraftChange={(segmentId, value) => {
                setTimelineDateDrafts((current) => ({ ...current, [segmentId]: value }))
              }}
              onTranscriptionSettingsChange={setTranscriptionSettings}
              onSyncTranscription={onSyncTranscription}
              onTextChange={setTextDraft}
              onTextSelection={setSelectedTextRange}
              onSave={onSave}
            />
          </main>

          <aside className="min-h-0 overflow-hidden bg-card">
            <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as WorkspaceTab)} className="flex h-full min-h-0 flex-col">
              <div className="border-b px-3 pt-3">
                <TabsList className={cn("grid w-full", isMedia ? "grid-cols-6" : "grid-cols-4")}>
                  <TabsTrigger value="links" aria-label="Links"><Link2 className="h-3.5 w-3.5" /></TabsTrigger>
                  <TabsTrigger value="annotations" aria-label="Annotations"><Highlighter className="h-3.5 w-3.5" /></TabsTrigger>
                  <TabsTrigger value="provenance" aria-label="Provenance"><PanelRight className="h-3.5 w-3.5" /></TabsTrigger>
                  <TabsTrigger value="markdown_graph" aria-label="Markdown graph"><GitGraphIcon className="h-3.5 w-3.5" /></TabsTrigger>
                  {isMedia && <TabsTrigger value="speakers" aria-label="Speakers"><Users className="h-3.5 w-3.5" /></TabsTrigger>}
                  {isMedia && <TabsTrigger value="privacy" aria-label="Privacy"><Shield className="h-3.5 w-3.5" /></TabsTrigger>}
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
                <TabsContent value="markdown_graph" className="m-0 space-y-4 p-4">
                  <MarkdownGraphPanel
                    matterId={matter.id}
                    workspace={workspace}
                    selectedTextRange={selectedTextRange}
                    onSelectNode={selectMarkdownNode}
                    onRunEmbeddings={onRunEmbeddings}
                  />
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
                        <KeyValue label="Redacted Audio" value={activeTranscription?.redacted_audio_version?.document_version_id ?? "none"} />
                        <KeyValue label="Prompt" value={transcriptionPromptSummary(activeTranscription?.job)} />
                        <KeyValue label="Speakers" value={transcriptionSpeakerSummary(activeTranscription?.job)} />
                        <KeyValue label="Word Search" value={activeTranscription?.job.word_search_terms.length ? `${activeTranscription.job.word_search_terms.length} terms` : "none"} />
                        <KeyValue label="Audio Tags" value={activeTranscription?.job.remove_audio_tags ?? "kept"} />
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
  canExtract,
  contentUrl,
  documentTitle,
  isImage,
  isMarkdown,
  isMedia,
  isPdf,
  selectedSegmentId,
  timelineDateDrafts,
  transcriptSegmentDrafts,
  transcriptReviewed,
  transcriptionSettings,
  transcriptView,
  textDraft,
  textAreaRef,
  workspace,
  onCreateAnnotation,
  onCreateEvidence,
  onCreateFact,
  onCreateTimeline,
  onSegmentDraftChange,
  onPatchSegment,
  onReviewTranscription,
  onSelectSegment,
  onStartTranscription,
  onTimelineDateDraftChange,
  onTranscriptionSettingsChange,
  onSyncTranscription,
  onTextChange,
  onTextSelection,
  onSave,
}: {
  activeTranscription: TranscriptionJobResponse | null
  busy: string | null
  canEdit: boolean
  canExtract: boolean
  contentUrl: string | null
  documentTitle: string
  isImage: boolean
  isMarkdown: boolean
  isMedia: boolean
  isPdf: boolean
  selectedSegmentId: string | null
  timelineDateDrafts: Record<string, string>
  transcriptSegmentDrafts: TranscriptSegmentDrafts
  transcriptReviewed: boolean
  transcriptionSettings: TranscriptionSettings
  transcriptView: TranscriptView
  textDraft: string
  textAreaRef: RefObject<HTMLTextAreaElement | null>
  workspace: DocumentWorkspaceState
  onCreateAnnotation: (segment: TranscriptSegment) => void
  onCreateEvidence: (segment: TranscriptSegment) => void
  onCreateFact: (segment: TranscriptSegment) => void
  onCreateTimeline: (segment: TranscriptSegment, eventDate?: string) => void
  onSegmentDraftChange: (segmentId: string, view: TranscriptView, text: string) => void
  onPatchSegment: (segment: TranscriptSegment, text: string, view: TranscriptView) => void
  onReviewTranscription: () => void
  onSelectSegment: (segmentId: string | null) => void
  onStartTranscription: () => void
  onTimelineDateDraftChange: (segmentId: string, value: string) => void
  onTranscriptionSettingsChange: Dispatch<SetStateAction<TranscriptionSettings>>
  onSyncTranscription: () => void
  onTextChange: (value: string) => void
  onTextSelection: (range: SelectedTextRange | null) => void
  onSave: () => void
}) {
  if (isPdf && contentUrl) {
    return <iframe title={documentTitle} src={`${contentUrl}#view=FitH`} className="h-full min-h-0 w-full bg-background" />
  }
  if (isImage && contentUrl) {
    return (
      <div className="flex h-full min-h-0 items-center justify-center overflow-hidden bg-muted/30 p-6">
        <div className="relative h-full min-h-0 w-full max-w-5xl">
          <Image
            src={contentUrl}
            alt={documentTitle}
            fill
            unoptimized
            sizes="(max-width: 1024px) 100vw, 1024px"
            className="rounded-md border object-contain"
          />
        </div>
      </div>
    )
  }
  if (isMedia && contentUrl) {
    return (
      <MediaTranscriptPane
        activeTranscription={activeTranscription}
        busy={busy}
        canExtract={canExtract}
        contentUrl={contentUrl}
        documentTitle={documentTitle}
        selectedSegmentId={selectedSegmentId}
        timelineDateDrafts={timelineDateDrafts}
        transcriptSegmentDrafts={transcriptSegmentDrafts}
        transcriptReviewed={transcriptReviewed}
        transcriptionSettings={transcriptionSettings}
        transcriptView={transcriptView}
        workspace={workspace}
        onCreateAnnotation={onCreateAnnotation}
        onCreateEvidence={onCreateEvidence}
        onCreateFact={onCreateFact}
        onCreateTimeline={onCreateTimeline}
        onSegmentDraftChange={onSegmentDraftChange}
        onPatchSegment={onPatchSegment}
        onReviewTranscription={onReviewTranscription}
        onSelectSegment={onSelectSegment}
        onStartTranscription={onStartTranscription}
        onTimelineDateDraftChange={onTimelineDateDraftChange}
        onTranscriptionSettingsChange={onTranscriptionSettingsChange}
        onSyncTranscription={onSyncTranscription}
      />
    )
  }
  if (isMarkdown || (canEdit && textDraft)) {
    return (
      <div className="flex h-full min-h-0 flex-col overflow-hidden">
        <div className="flex items-center justify-between border-b px-4 py-2 text-xs text-muted-foreground">
          <span>{isMarkdown ? "Markdown source" : "Text source"}</span>
          <Badge variant={canEdit ? "default" : "outline"}>{canEdit ? "Editable" : "Read only"}</Badge>
        </div>
        <Textarea
          ref={textAreaRef}
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
    <div className="flex h-full min-h-0 items-center justify-center overflow-hidden p-8">
      <div className="max-w-md text-center text-sm text-muted-foreground">
        <FileText className="mx-auto mb-3 h-10 w-10" />
        <div className="font-medium text-foreground">Stored source</div>
        <p className="mt-2">This file is stored privately and available for viewing or annotation. Markdown-only indexing is enabled, so extraction is skipped for this source.</p>
        {contentUrl && (
          <a
            href={contentUrl}
            target="_blank"
            rel="noreferrer"
            className="mt-4 inline-flex rounded border border-border px-3 py-1.5 font-mono text-xs uppercase tracking-wider text-foreground hover:bg-muted"
          >
            Open source
          </a>
        )}
      </div>
    </div>
  )
}

function MediaTranscriptPane({
  activeTranscription,
  busy,
  canExtract,
  contentUrl,
  documentTitle,
  selectedSegmentId,
  timelineDateDrafts,
  transcriptSegmentDrafts,
  transcriptReviewed,
  transcriptionSettings,
  transcriptView,
  workspace,
  onCreateAnnotation,
  onCreateEvidence,
  onCreateFact,
  onCreateTimeline,
  onSegmentDraftChange,
  onPatchSegment,
  onReviewTranscription,
  onSelectSegment,
  onStartTranscription,
  onTimelineDateDraftChange,
  onTranscriptionSettingsChange,
  onSyncTranscription,
}: {
  activeTranscription: TranscriptionJobResponse | null
  busy: string | null
  canExtract: boolean
  contentUrl: string
  documentTitle: string
  selectedSegmentId: string | null
  timelineDateDrafts: Record<string, string>
  transcriptSegmentDrafts: TranscriptSegmentDrafts
  transcriptReviewed: boolean
  transcriptionSettings: TranscriptionSettings
  transcriptView: TranscriptView
  workspace: DocumentWorkspaceState
  onCreateAnnotation: (segment: TranscriptSegment) => void
  onCreateEvidence: (segment: TranscriptSegment) => void
  onCreateFact: (segment: TranscriptSegment) => void
  onCreateTimeline: (segment: TranscriptSegment, eventDate?: string) => void
  onSegmentDraftChange: (segmentId: string, view: TranscriptView, text: string) => void
  onPatchSegment: (segment: TranscriptSegment, text: string, view: TranscriptView) => void
  onReviewTranscription: () => void
  onSelectSegment: (segmentId: string | null) => void
  onStartTranscription: () => void
  onTimelineDateDraftChange: (segmentId: string, value: string) => void
  onTranscriptionSettingsChange: Dispatch<SetStateAction<TranscriptionSettings>>
  onSyncTranscription: () => void
}) {
  const audioRef = useRef<HTMLAudioElement | null>(null)
  const videoRef = useRef<HTMLVideoElement | null>(null)
  const isAudio = workspace.document.mime_type?.startsWith("audio/")
  const segments = activeTranscription?.segments ?? []
  const canCreateCaseItems = canExtract && activeTranscription?.job.status === "processed"

  function jump(segment: TranscriptSegment) {
    onSelectSegment(segment.segment_id)
    const media = audioRef.current ?? videoRef.current
    if (media) {
      media.currentTime = segment.time_start_ms / 1000
      void media.play().catch(() => undefined)
    }
  }

  return (
    <div className="grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)] overflow-hidden bg-background">
      <div className="space-y-3 overflow-y-auto border-b bg-muted/30 p-4">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-center">
          <div className="min-w-0 flex-1">
            {isAudio ? (
              <audio ref={audioRef} controls src={contentUrl} className="w-full" />
            ) : (
              <video ref={videoRef} controls src={contentUrl} className="max-h-[280px] w-full rounded-md border bg-black object-contain" />
            )}
          </div>
          <div className="flex shrink-0 flex-wrap gap-2">
            <Button size="sm" onClick={onStartTranscription} disabled={!canExtract || busy === "transcribe"}>
              <Mic className="mr-2 h-4 w-4" />
              Transcribe
            </Button>
            <Button size="sm" variant="outline" onClick={onSyncTranscription} disabled={!canExtract || !activeTranscription || busy === "sync transcript"}>
              <RefreshCw className="mr-2 h-4 w-4" />
              Sync
            </Button>
            <Button size="sm" variant="outline" onClick={onReviewTranscription} disabled={!canExtract || !activeTranscription || segments.length === 0 || busy === "review transcript"}>
              <CheckCircle2 className="mr-2 h-4 w-4" />
              {transcriptView === "raw" ? "Review raw" : "Review redacted"}
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
            {transcriptionSettingsSummary(activeTranscription.job).map((item) => (
              <span key={item}>{item}</span>
            ))}
          </div>
        )}
        <TranscriptionSettingsPanel
          disabled={busy === "transcribe"}
          settings={transcriptionSettings}
          onChange={onTranscriptionSettingsChange}
        />
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
          {activeTranscription?.job.status === "failed" && (
            <div className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">
              {activeTranscription.job.error_message || "Transcript provider failed. Retry transcription after checking provider settings."}
            </div>
          )}
          {segments.map((segment) => {
            const selected = segment.segment_id === selectedSegmentId
            const segmentReviewed = canCreateCaseItems || segment.review_status === "approved" || transcriptReviewed
            const timelineDate = timelineDateDrafts[segment.segment_id] ?? workspace.document.date_observed ?? ""
            const draftText = segmentDraftText(segment, transcriptView, transcriptSegmentDrafts)
            return (
              <section key={segment.segment_id} className={cn("rounded-md border bg-card p-3", selected && "border-primary ring-1 ring-primary")}>
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <button className="text-left text-xs font-medium text-foreground" onClick={() => jump(segment)}>
                    {segmentSpeaker(segment)} · {formatMs(segment.time_start_ms)}-{formatMs(segment.time_end_ms)}
                  </button>
                  <div className="flex flex-wrap items-center gap-1">
                    {segment.paragraph_ordinal ? <Badge variant="secondary">P{segment.paragraph_ordinal}</Badge> : null}
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
                    <div className="flex items-center gap-1">
                      <Input
                        aria-label={`Timeline date for segment ${segment.ordinal}`}
                        type="date"
                        value={timelineDate}
                        onChange={(event) => onTimelineDateDraftChange(segment.segment_id, event.target.value)}
                        disabled={!segmentReviewed}
                        className="h-8 w-[9.25rem] font-mono text-[11px]"
                      />
                      <IconButton label="Create timeline event" onClick={() => onCreateTimeline(segment, timelineDate)} disabled={!segmentReviewed}>
                        <ScrollText className="h-4 w-4" />
                      </IconButton>
                    </div>
                  </div>
                </div>
                <Textarea
                  aria-label={`Transcript segment ${segment.ordinal}`}
                  value={draftText}
                  onChange={(event) => onSegmentDraftChange(segment.segment_id, transcriptView, event.target.value)}
                  onFocus={() => onSelectSegment(segment.segment_id)}
                  onBlur={(event) => {
                    onPatchSegment(segment, event.currentTarget.value, transcriptView)
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

function TranscriptionSettingsPanel({
  disabled,
  settings,
  onChange,
}: {
  disabled: boolean
  settings: TranscriptionSettings
  onChange: Dispatch<SetStateAction<TranscriptionSettings>>
}) {
  const update = (patch: Partial<TranscriptionSettings>) => {
    onChange((current) => ({ ...current, ...patch }))
  }

  return (
    <div className="grid gap-3 rounded-md border bg-background/80 p-3 text-xs lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
      <div className="space-y-3">
        <div className="grid grid-cols-3 gap-2">
          <SwitchField
            checked={settings.redactPii}
            disabled={disabled}
            label="PII"
            onCheckedChange={(checked) => update({ redactPii: checked })}
          />
          <SwitchField
            checked={settings.speakerLabels}
            disabled={disabled}
            label="Speakers"
            onCheckedChange={(checked) => update({ speakerLabels: checked })}
          />
          <SwitchField
            checked={settings.removeAudioTags}
            disabled={disabled}
            label="Tags"
            onCheckedChange={(checked) => update({ removeAudioTags: checked })}
          />
        </div>

        <div className="grid gap-2 sm:grid-cols-[140px_minmax(0,1fr)]">
          <Select
            value={settings.speakerMode}
            onValueChange={(value) => update({ speakerMode: value as SpeakerMode })}
            disabled={disabled || !settings.speakerLabels}
          >
            <SelectTrigger className="h-8"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="auto">Auto speakers</SelectItem>
              <SelectItem value="exact">Exact count</SelectItem>
              <SelectItem value="range">Count range</SelectItem>
            </SelectContent>
          </Select>

          {settings.speakerMode === "exact" ? (
            <Input
              value={settings.speakersExpected}
              onChange={(event) => update({ speakersExpected: event.target.value })}
              inputMode="numeric"
              placeholder="Speakers expected"
              disabled={disabled || !settings.speakerLabels}
              className="h-8"
            />
          ) : settings.speakerMode === "range" ? (
            <div className="grid grid-cols-2 gap-2">
              <Input
                value={settings.minSpeakersExpected}
                onChange={(event) => update({ minSpeakersExpected: event.target.value })}
                inputMode="numeric"
                placeholder="Min"
                disabled={disabled || !settings.speakerLabels}
                className="h-8"
              />
              <Input
                value={settings.maxSpeakersExpected}
                onChange={(event) => update({ maxSpeakersExpected: event.target.value })}
                inputMode="numeric"
                placeholder="Max"
                disabled={disabled || !settings.speakerLabels}
                className="h-8"
              />
            </div>
          ) : (
            <div className="flex h-8 items-center rounded-md border bg-muted/30 px-3 text-muted-foreground">
              Provider clustering
            </div>
          )}
        </div>
      </div>

      <div className="space-y-2">
        <div className="grid gap-2 sm:grid-cols-[150px_minmax(0,1fr)]">
          <Select
            value={settings.promptMode}
            onValueChange={(value) => update({ promptMode: value as PromptMode })}
            disabled={disabled}
          >
            <SelectTrigger className="h-8"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="default">Default prompt</SelectItem>
              <SelectItem value="preset">Preset prompt</SelectItem>
              <SelectItem value="custom">Custom prompt</SelectItem>
              <SelectItem value="keyterms">Keyterms</SelectItem>
            </SelectContent>
          </Select>

          {settings.promptMode === "preset" ? (
            <Select
              value={settings.promptPreset}
              onValueChange={(value) => update({ promptPreset: value })}
              disabled={disabled}
            >
              <SelectTrigger className="h-8"><SelectValue /></SelectTrigger>
              <SelectContent>
                {promptPresetOptions.map((option) => (
                  <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          ) : (
            <Input
              value={settings.wordSearchTerms}
              onChange={(event) => update({ wordSearchTerms: event.target.value })}
              placeholder="Word search terms"
              disabled={disabled}
              className="h-8"
            />
          )}
        </div>

        {settings.promptMode === "custom" && (
          <Textarea
            value={settings.prompt}
            onChange={(event) => update({ prompt: event.target.value })}
            placeholder="Prompt"
            rows={3}
            disabled={disabled}
            className="min-h-20 resize-y"
          />
        )}

        {settings.promptMode === "keyterms" && (
          <Textarea
            value={settings.keyterms}
            onChange={(event) => update({ keyterms: event.target.value })}
            placeholder="Keyterms"
            rows={3}
            disabled={disabled}
            className="min-h-20 resize-y"
          />
        )}

        {settings.promptMode === "preset" && (
          <Input
            value={settings.wordSearchTerms}
            onChange={(event) => update({ wordSearchTerms: event.target.value })}
            placeholder="Word search terms"
            disabled={disabled}
            className="h-8"
          />
        )}
      </div>
    </div>
  )
}

function SwitchField({
  checked,
  disabled,
  label,
  onCheckedChange,
}: {
  checked: boolean
  disabled: boolean
  label: string
  onCheckedChange: (checked: boolean) => void
}) {
  return (
    <Label className="flex h-8 items-center justify-between rounded-md border bg-muted/20 px-2 text-xs">
      <span>{label}</span>
      <Switch checked={checked} disabled={disabled} onCheckedChange={onCheckedChange} />
    </Label>
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
        <Button aria-label={label} variant="outline" size="icon" disabled={disabled} onClick={onClick}>
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}

function MarkdownGraphPanel({
  matterId,
  workspace,
  selectedTextRange,
  onSelectNode,
  onRunEmbeddings,
}: {
  matterId: string
  workspace: DocumentWorkspaceState
  selectedTextRange: SelectedTextRange | null
  onSelectNode: (node: MarkdownAstNode) => void
  onRunEmbeddings: () => Promise<void>
}) {
  const [embeddingQuery, setEmbeddingQuery] = useState("")
  const [embeddingResults, setEmbeddingResults] = useState<CaseBuilderEmbeddingSearchResult[]>([])
  const [embeddingSearchBusy, setEmbeddingSearchBusy] = useState(false)
  const [embeddingSearchError, setEmbeddingSearchError] = useState<string | null>(null)
  const astDocument = workspace.markdown_ast_document
  const outlineNodes = workspace.markdown_ast_nodes
    .filter((node) => node.node_kind === "heading" || (node.depth <= 2 && node.node_kind !== "text"))
    .slice(0, 32)
  const textNodesById = new Map(workspace.markdown_ast_nodes.map((node) => [node.markdown_ast_node_id, node]))
  const embeddingRecordsByTarget = new Map(
    workspace.embedding_records.map((record) => [record.target_id, record]),
  )
  const latestEmbeddingRun = workspace.embedding_runs[0] ?? null
  const coverage = workspace.embedding_coverage

  async function onSearchEmbeddings() {
    const query = embeddingQuery.trim()
    if (!query) return
    setEmbeddingSearchBusy(true)
    setEmbeddingSearchError(null)
    const result = await searchMatterEmbeddings(matterId, {
      query,
      document_ids: [workspace.document.document_id],
      limit: 8,
    })
    setEmbeddingSearchBusy(false)
    if (!result.data) {
      setEmbeddingSearchError(result.error ?? "Embedding search failed.")
      return
    }
    setEmbeddingResults(result.data.results)
    if (result.data.warnings.length) {
      setEmbeddingSearchError(result.data.warnings.join(" "))
    }
  }

  return (
    <>
      <InspectorSection title="Markdown Graph" icon={<GitGraphIcon className="h-4 w-4" />}>
        <div className="grid grid-cols-2 gap-2 text-xs">
          <GraphMetric label="AST nodes" value={String(workspace.markdown_ast_nodes.length)} />
          <GraphMetric label="Semantic units" value={String(workspace.markdown_semantic_units.length)} />
          <GraphMetric label="Entities" value={String(workspace.entities.length)} />
          <GraphMetric label="Spans" value={String(workspace.source_spans.length)} />
        </div>
        <div className="mt-3 space-y-1 text-xs text-muted-foreground">
          <KeyValue label="Parser" value={astDocument?.parser_version ?? "none"} />
          <KeyValue label="Schema" value={astDocument?.graph_schema_version ?? "none"} />
          <KeyValue label="Root" value={astDocument?.root_node_id ?? "none"} />
        </div>
      </InspectorSection>

      <InspectorSection title="Embeddings" icon={<Sparkles className="h-4 w-4" />}>
        <div className="grid grid-cols-2 gap-2 text-xs">
          <GraphMetric label="Current" value={String(coverage.current_count)} />
          <GraphMetric label="Chunks" value={String(coverage.chunk_embedded)} />
          <GraphMetric label="Units" value={String(coverage.semantic_unit_embedded)} />
          <GraphMetric label="Stale" value={String(coverage.stale_count)} />
        </div>
        <div className="mt-3 space-y-1 text-xs text-muted-foreground">
          <KeyValue label="Model" value={coverage.model ?? "voyage-4-large"} />
          <KeyValue label="Index" value={coverage.vector_index_name ?? "casebuilder_markdown_embedding_1024"} />
          <KeyValue label="Latest run" value={latestEmbeddingRun ? `${latestEmbeddingRun.status} / ${latestEmbeddingRun.stage}` : "none"} />
        </div>
        <div className="mt-3 flex gap-2">
          <Button size="sm" variant="outline" className="flex-1" onClick={onRunEmbeddings}>
            <RefreshCw className="mr-2 h-3.5 w-3.5" />
            Embed
          </Button>
        </div>
      </InspectorSection>

      <InspectorSection title="Semantic Search" icon={<Search className="h-4 w-4" />}>
        <div className="flex gap-2">
          <Input
            type="search"
            value={embeddingQuery}
            onChange={(event) => setEmbeddingQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") void onSearchEmbeddings()
            }}
            placeholder="Search Markdown graph"
          />
          <Button size="icon" variant="outline" onClick={() => void onSearchEmbeddings()} disabled={embeddingSearchBusy}>
            <Search className="h-4 w-4" />
          </Button>
        </div>
        {embeddingSearchError && <div className="mt-2 text-xs text-destructive">{embeddingSearchError}</div>}
        {embeddingResults.length ? (
          <div className="mt-3 space-y-2">
            {embeddingResults.map((result) => (
              <button
                key={result.embedding_record.embedding_record_id}
                type="button"
                onClick={() => {
                  const firstNode = result.markdown_ast_node_ids
                    .map((id) => textNodesById.get(id))
                    .find(Boolean)
                  if (firstNode) onSelectNode(firstNode)
                }}
                className="w-full rounded-md border px-3 py-2 text-left text-xs hover:border-primary/40 hover:bg-muted/40"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="line-clamp-1 font-medium">{result.embedding_record.target_label}</span>
                  <Badge variant={result.stale ? "outline" : "default"}>{Math.round(result.score * 100)}%</Badge>
                </div>
                {result.text_excerpt && <div className="mt-1 line-clamp-2 text-muted-foreground">{result.text_excerpt}</div>}
              </button>
            ))}
          </div>
        ) : (
          <EmptyLine text="No embedding search results yet." />
        )}
      </InspectorSection>

      <InspectorSection title="Semantic Units" icon={<Network className="h-4 w-4" />}>
        {workspace.markdown_semantic_units.length ? (
          <div className="space-y-2">
            {workspace.markdown_semantic_units.slice(0, 24).map((unit) => (
              <button
                key={unit.semantic_unit_id}
                type="button"
                onClick={() => {
                  const firstNode = unit.markdown_ast_node_ids
                    .map((id) => textNodesById.get(id))
                    .find(Boolean)
                  if (firstNode) onSelectNode(firstNode)
                }}
                className="w-full rounded-md border px-3 py-2 text-left text-xs hover:border-primary/40 hover:bg-muted/40"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="line-clamp-1 font-medium">{unit.canonical_label}</span>
                  <Badge variant="outline">{unit.semantic_role}</Badge>
                </div>
                <div className="mt-2 flex flex-wrap gap-1">
                  <Badge variant="outline">{unit.unit_kind}</Badge>
                  {embeddingRecordsByTarget.has(unit.semantic_unit_id) && (
                    <Badge variant={embeddingRecordsByTarget.get(unit.semantic_unit_id)?.stale ? "outline" : "default"}>embedded</Badge>
                  )}
                  {unit.entity_mention_ids.length > 0 && <Badge variant="outline">{unit.entity_mention_ids.length} mentions</Badge>}
                  {unit.citation_texts.length > 0 && <Badge variant="outline">citations</Badge>}
                  {unit.date_texts.length > 0 && <Badge variant="outline">dates</Badge>}
                  {unit.money_texts.length > 0 && <Badge variant="outline">money</Badge>}
                </div>
              </button>
            ))}
          </div>
        ) : (
          <EmptyLine text="No semantic units yet." />
        )}
      </InspectorSection>

      <InspectorSection title="Outline" icon={<FileText className="h-4 w-4" />}>
        {outlineNodes.length ? (
          <div className="space-y-2">
            {outlineNodes.map((node) => (
              <button
                key={node.markdown_ast_node_id}
                type="button"
                onClick={() => onSelectNode(node)}
                className="w-full rounded-md border px-3 py-2 text-left text-xs hover:border-primary/40 hover:bg-muted/40"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="line-clamp-1 font-medium">{node.text_excerpt || node.structure_path || node.tag}</span>
                  <Badge variant="outline">{node.node_kind}</Badge>
                </div>
                {node.text_chunk_ids.some((id) => embeddingRecordsByTarget.has(id)) && (
                  <div className="mt-1">
                    <Badge variant="default">embedded chunk</Badge>
                  </div>
                )}
                <div className="mt-1 font-mono text-[10px] text-muted-foreground">
                  {node.char_start != null && node.char_end != null ? `${node.char_start}-${node.char_end}` : node.markdown_ast_node_id}
                </div>
              </button>
            ))}
          </div>
        ) : (
          <EmptyLine text="No Markdown AST nodes yet." />
        )}
      </InspectorSection>

      <InspectorSection title="Entities" icon={<Users className="h-4 w-4" />}>
        {workspace.entities.length ? (
          <div className="space-y-2">
            {workspace.entities.slice(0, 24).map((entity) => (
              <div key={entity.entity_id} className="rounded-md border px-3 py-2 text-xs">
                <div className="flex items-center justify-between gap-2">
                  <span className="line-clamp-1 font-medium">{entity.canonical_name}</span>
                  <Badge variant={entity.review_status === "approved" ? "default" : "outline"}>{entity.entity_type}</Badge>
                </div>
                <div className="mt-2 flex flex-wrap gap-1">
                  <Badge variant="outline">{entity.review_status}</Badge>
                  <Badge variant="outline">{entity.mention_ids.length} mentions</Badge>
                  {entity.party_match_ids.length > 0 && <Badge variant="outline">party candidate</Badge>}
                </div>
              </div>
            ))}
          </div>
        ) : (
          <EmptyLine text="No reviewable entities yet." />
        )}
      </InspectorSection>

      <InspectorSection title="Facts And Timeline" icon={<CalendarClock className="h-4 w-4" />}>
        <div className="space-y-3">
          {workspace.proposed_facts.slice(0, 8).map((fact) => (
            <GraphReviewCard
              key={fact.fact_id}
              label="Fact"
              title={fact.statement}
              ids={fact.markdown_ast_node_ids ?? []}
              textNodesById={textNodesById}
              onSelectNode={onSelectNode}
            />
          ))}
          {workspace.timeline_suggestions.slice(0, 8).map((suggestion) => (
            <GraphReviewCard
              key={suggestion.suggestion_id}
              label={suggestion.date_text}
              title={suggestion.title}
              ids={suggestion.markdown_ast_node_ids}
              textNodesById={textNodesById}
              onSelectNode={onSelectNode}
            />
          ))}
          {!workspace.proposed_facts.length && !workspace.timeline_suggestions.length && (
            <EmptyLine text="No proposed facts or timeline suggestions yet." />
          )}
        </div>
      </InspectorSection>

      {selectedTextRange && (
        <InspectorSection title="Selected Source" icon={<Highlighter className="h-4 w-4" />}>
          <div className="rounded-md border bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
            <div className="font-mono text-[10px]">{selectedTextRange.start}-{selectedTextRange.end}</div>
            <div className="mt-1 line-clamp-3">{selectedTextRange.quote}</div>
          </div>
        </InspectorSection>
      )}
    </>
  )
}

function GraphMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border bg-background px-3 py-2">
      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div className="mt-1 text-base font-semibold text-foreground">{value}</div>
    </div>
  )
}

function GraphReviewCard({
  ids,
  label,
  onSelectNode,
  textNodesById,
  title,
}: {
  ids: string[]
  label: string
  onSelectNode: (node: MarkdownAstNode) => void
  textNodesById: Map<string, MarkdownAstNode>
  title: string
}) {
  const firstNode = ids.map((id) => textNodesById.get(id)).find(Boolean)
  return (
    <div className="rounded-md border px-3 py-2 text-xs">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <Badge variant="outline">{label}</Badge>
          <div className="mt-2 line-clamp-3 leading-5">{title}</div>
        </div>
        {firstNode && (
          <Button size="sm" variant="outline" onClick={() => onSelectNode(firstNode)} className="shrink-0">
            Jump
          </Button>
        )}
      </div>
      <div className="mt-2 font-mono text-[10px] text-muted-foreground">
        {ids.length ? `${ids.length} AST link${ids.length === 1 ? "" : "s"}` : "no AST link"}
      </div>
    </div>
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

function segmentDraftText(segment: TranscriptSegment, view: TranscriptView, drafts: TranscriptSegmentDrafts) {
  return drafts[segment.segment_id]?.[view] ?? segmentText(segment, view)
}

function segmentSpeaker(segment: TranscriptSegment) {
  return segment.speaker_name || segment.speaker_label || (segment.channel ? `Channel ${segment.channel}` : "Speaker")
}

function transcriptText(segments: TranscriptSegment[], view: TranscriptView, drafts: TranscriptSegmentDrafts = {}) {
  const paragraphs: string[] = []
  let currentParagraph: number | null | undefined = null
  let currentLines: string[] = []
  for (const segment of segments) {
    if (currentLines.length && segment.paragraph_ordinal && segment.paragraph_ordinal !== currentParagraph) {
      paragraphs.push(currentLines.join("\n"))
      currentLines = []
    }
    currentParagraph = segment.paragraph_ordinal
    currentLines.push(`${segmentSpeaker(segment)}: ${segmentDraftText(segment, view, drafts)}`)
  }
  if (currentLines.length) paragraphs.push(currentLines.join("\n"))
  return paragraphs.join("\n\n")
}

function buildCreateTranscriptionInput(settings: TranscriptionSettings): CreateTranscriptionInput {
  const wordSearchTerms = listFromText(settings.wordSearchTerms)
  const input: CreateTranscriptionInput = {
    redact_pii: settings.redactPii,
    speaker_labels: settings.speakerLabels,
    remove_audio_tags: settings.removeAudioTags ? "all" : null,
  }

  if (settings.speakerLabels && settings.speakerMode === "exact") {
    const speakersExpected = positiveInteger(settings.speakersExpected)
    if (speakersExpected) input.speakers_expected = speakersExpected
  }

  if (settings.speakerLabels && settings.speakerMode === "range") {
    const minSpeakersExpected = positiveInteger(settings.minSpeakersExpected)
    const maxSpeakersExpected = positiveInteger(settings.maxSpeakersExpected)
    if (minSpeakersExpected || maxSpeakersExpected) {
      input.speaker_options = {
        min_speakers_expected: minSpeakersExpected,
        max_speakers_expected: maxSpeakersExpected,
      }
    }
  }

  if (wordSearchTerms.length) {
    input.word_search_terms = wordSearchTerms
  }

  if (settings.promptMode === "preset") {
    input.prompt_preset = settings.promptPreset
  } else if (settings.promptMode === "custom") {
    const prompt = settings.prompt.trim()
    if (prompt) input.prompt = prompt
  } else if (settings.promptMode === "keyterms") {
    const keyterms = listFromText(settings.keyterms)
    if (keyterms.length) input.keyterms_prompt = keyterms
  }

  return input
}

function positiveInteger(value: string) {
  const parsed = Number.parseInt(value.trim(), 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null
}

function listFromText(value: string) {
  return Array.from(
    new Set(
      value
        .split(/[\n,]+/)
        .map((item) => item.trim())
        .filter(Boolean),
    ),
  )
}

function transcriptionSettingsSummary(job: TranscriptionJob) {
  const items: string[] = []
  if (job.speakers_expected) {
    items.push(`${job.speakers_expected} expected`)
  } else if (job.speaker_options) {
    const min = job.speaker_options.min_speakers_expected ?? "?"
    const max = job.speaker_options.max_speakers_expected ?? "?"
    items.push(`${min}-${max} expected`)
  }
  if (job.prompt_preset) items.push(`prompt ${job.prompt_preset}`)
  if (job.prompt && !job.prompt_preset) items.push("custom prompt")
  if (job.keyterms_prompt.length) items.push(`${job.keyterms_prompt.length} keyterm(s)`)
  if (job.word_search_terms.length) items.push(`${job.word_search_terms.length} search term(s)`)
  if (job.remove_audio_tags) items.push("tags removed")
  return items
}

function transcriptionPromptSummary(job?: TranscriptionJob | null) {
  if (!job) return "none"
  if (job.prompt_preset) return job.prompt_preset
  if (job.prompt) return "custom"
  if (job.keyterms_prompt.length) return `${job.keyterms_prompt.length} keyterms`
  return "default"
}

function transcriptionSpeakerSummary(job?: TranscriptionJob | null) {
  if (!job) return "none"
  if (job.speakers_expected) return `${job.speakers_expected} expected`
  if (job.speaker_options) {
    const min = job.speaker_options.min_speakers_expected ?? "?"
    const max = job.speaker_options.max_speakers_expected ?? "?"
    return `${min}-${max} expected`
  }
  return "auto"
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
