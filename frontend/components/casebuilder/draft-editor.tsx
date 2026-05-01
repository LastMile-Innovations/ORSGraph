"use client"

import { useMemo, useRef, useState } from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import {
  ArrowLeft,
  Sparkles,
  FileText,
  Quote,
  History,
  ListTree,
  ShieldCheck,
  CheckCircle2,
  XCircle,
  Wand2,
  Send,
  MessageSquare,
  Download,
  Save,
  Plus,
  ChevronRight,
  AlertTriangle,
  Lightbulb,
} from "lucide-react"
import type {
  Matter,
  Draft,
  DraftSection,
  DraftSuggestion,
  DraftCitation,
  DraftComment,
} from "@/lib/casebuilder/types"
import { matterDocumentHref, matterDraftHref, matterFactsHref } from "@/lib/casebuilder/routes"
import { citationCheckDraft, createWorkProduct, factCheckDraft, generateDraft, patchDraft, runWorkProductAiCommand } from "@/lib/casebuilder/api"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"

interface DraftEditorProps {
  matter: Matter
  draft: Draft
}

type RightTab = "sources" | "citecheck" | "outline" | "versions"

export function DraftEditor({ matter, draft: initialDraft }: DraftEditorProps) {
  const router = useRouter()
  const [draft, setDraft] = useState<Draft>(initialDraft)
  const [activeSection, setActiveSection] = useState<string>(initialDraft.sections[0]?.id ?? "")
  const [rightTab, setRightTab] = useState<RightTab>("sources")
  const [aiPrompt, setAiPrompt] = useState("")
  const [pendingPrompt, setPendingPrompt] = useState(false)
  const [actionPending, setActionPending] = useState(false)
  const [actionMessage, setActionMessage] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)

  const editorRef = useRef<HTMLDivElement>(null)

  const acceptSuggestion = (sectionId: string, suggestionId: string) => {
    setDraft((prev) => ({
      ...prev,
      sections: prev.sections.map((s) =>
        s.id !== sectionId
          ? s
          : {
              ...s,
              body: applySuggestionToBody(
                s.body,
                s.suggestions.find((sg) => sg.id === suggestionId),
              ),
              suggestions: s.suggestions.filter((sg) => sg.id !== suggestionId),
            },
      ),
    }))
  }

  const rejectSuggestion = (sectionId: string, suggestionId: string) => {
    setDraft((prev) => ({
      ...prev,
      sections: prev.sections.map((s) =>
        s.id !== sectionId
          ? s
          : { ...s, suggestions: s.suggestions.filter((sg) => sg.id !== suggestionId) },
      ),
    }))
  }

  const updateSectionBody = (sectionId: string, body: string) => {
    setDraft((prev) => ({
      ...prev,
      sections: prev.sections.map((section) =>
        section.id === sectionId ? { ...section, body } : section,
      ),
    }))
  }

  const runDraftAction = async (action: () => Promise<string | null>) => {
    setActionPending(true)
    setActionMessage(null)
    setActionError(null)
    const error = await action()
    setActionPending(false)
    if (error) setActionError(error)
  }

  const saveDraft = () =>
    runDraftAction(async () => {
      const result = await patchDraft(matter.id, draft.id, {
        title: draft.title,
        description: draft.description,
        status: draft.status,
        sections: draft.sections,
        paragraphs: draft.paragraphs,
      })
      if (!result.data) return result.error || "Draft could not be saved."
      setDraft(result.data)
      setActionMessage("Draft saved.")
      router.refresh()
      return null
    })

  const generateScaffold = () =>
    runDraftAction(async () => {
      const result = await generateDraft(matter.id, draft.id)
      if (!result.data?.result) return result.error || "Draft scaffold could not be generated."
      setDraft(result.data.result)
      setActiveSection(result.data.result.sections[0]?.id ?? "")
      setActionMessage(result.data.message)
      router.refresh()
      return null
    })

  const runSupportChecks = () =>
    runDraftAction(async () => {
      const [factResult, citationResult] = await Promise.all([
        factCheckDraft(matter.id, draft.id),
        citationCheckDraft(matter.id, draft.id),
      ])
      if (!factResult.data) return factResult.error || "Fact-check failed."
      if (!citationResult.data) return citationResult.error || "Citation-check failed."
      setRightTab("citecheck")
      setActionMessage(`${factResult.data.message} ${citationResult.data.message}`)
      router.refresh()
      return null
    })

  const handlePrompt = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!aiPrompt.trim()) return
    setPendingPrompt(true)
    setActionError(null)
    const prompt = aiPrompt.trim()
    const existingProduct =
      matter.work_products.find((product) => product.source_draft_id === draft.id) ??
      matter.work_products.find((product) => product.product_type === draft.draft_type || product.product_type === draft.kind)
    let workProductId = existingProduct?.id ?? existingProduct?.work_product_id
    if (!workProductId) {
      const created = await createWorkProduct(matter.id, {
        title: draft.title,
        product_type: draft.draft_type ?? draft.kind,
        source_draft_id: draft.id,
      })
      workProductId = created.data?.id ?? created.data?.work_product_id
      if (!workProductId) {
        setActionError(created.error || "Could not create a live work product for this draft command.")
        setPendingPrompt(false)
        return
      }
    }
    const result = await runWorkProductAiCommand(matter.id, workProductId, {
      command: "custom_prompt",
      target_id: activeSection,
      prompt,
    })
    if (result.data) {
      const firstBlock = result.data.result?.blocks?.find((block) => block.text?.trim())
      const newSuggestion: DraftSuggestion = {
        id: `sug-${Date.now()}`,
        kind: "rewrite",
        rationale: result.data.message || `Provider-free command response to: "${prompt}".`,
        original: activeSectionBody(draft.sections, activeSection).slice(0, 180),
        proposed: firstBlock?.text || activeSectionBody(draft.sections, activeSection),
        confidence: result.data.enabled ? 0.76 : 0.5,
        sources: firstBlock?.links?.slice(0, 4) ?? [],
      }
      setDraft((prev) => ({
        ...prev,
        sections: prev.sections.map((s) =>
          s.id !== activeSection
            ? s
            : { ...s, suggestions: [...s.suggestions, newSuggestion] },
        ),
      }))
      setAiPrompt("")
      setActionMessage(result.data.message)
    } else {
      setActionError(result.error || "Draft command failed.")
    }
    setPendingPrompt(false)
  }

  const allCitations = useMemo(
    () => draft.sections.flatMap((s) => s.citations),
    [draft.sections],
  )

  return (
    <TooltipProvider delayDuration={150}>
      <div className="flex flex-col">
        {/* Header */}
        <div className="border-b border-border bg-card px-6 py-3">
          <div className="flex items-center gap-3 text-xs text-muted-foreground">
            <Link
              href={matterDraftHref(matter.id)}
              className="flex items-center gap-1 hover:text-foreground"
            >
              <ArrowLeft className="h-3.5 w-3.5" />
              Drafts
            </Link>
            <ChevronRight className="h-3 w-3" />
            <span className="truncate text-foreground">{draft.title}</span>
          </div>

          <div className="mt-2 flex items-start justify-between gap-3">
            <div className="min-w-0 flex-1">
              <h1 className="truncate text-lg font-semibold text-foreground text-balance">
                {draft.title}
              </h1>
              <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
                <Badge variant="outline" className="text-[10px] capitalize">
                  {draft.status}
                </Badge>
                <span>{draft.wordCount.toLocaleString()} words</span>
                <span>·</span>
                <span>{draft.sections.length} sections</span>
                <span>·</span>
                <span>{allCitations.length} citations</span>
                <span>·</span>
                <span className="font-mono">Last edit {draft.lastEdited}</span>
              </div>
            </div>

            <div className="flex shrink-0 items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                className="gap-1.5 bg-transparent"
                disabled={actionPending}
                onClick={runSupportChecks}
              >
                <ShieldCheck className="h-3.5 w-3.5" />
                Check support
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="gap-1.5 bg-transparent"
                disabled={actionPending}
                onClick={saveDraft}
              >
                <Save className="h-3.5 w-3.5" />
                Save
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="gap-1.5 bg-transparent"
                disabled={actionPending}
                onClick={generateScaffold}
              >
                <Wand2 className="h-3.5 w-3.5" />
                Scaffold
              </Button>
              <Button variant="outline" size="sm" className="gap-1.5 bg-transparent">
                <Download className="h-3.5 w-3.5" />
                Export
              </Button>
              <Button size="sm" className="gap-1.5">
                <Send className="h-3.5 w-3.5" />
                Mark final
              </Button>
            </div>
          </div>
          {(actionMessage || actionError) && (
            <div
              className={cn(
                "mt-2 rounded border px-3 py-2 text-xs",
                actionError
                  ? "border-destructive/30 bg-destructive/5 text-destructive"
                  : "border-primary/20 bg-primary/5 text-muted-foreground",
              )}
            >
              {actionError || actionMessage}
            </div>
          )}
        </div>

        {/* Three-pane layout */}
        <div className="grid grid-cols-1 lg:grid-cols-[240px_minmax(0,1fr)_400px]">
          {/* Outline */}
          <aside className="hidden border-r border-border bg-card lg:block">
            <div className="border-b border-border px-3 py-2.5">
              <p className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                Outline
              </p>
            </div>
            <ScrollArea className="h-[calc(100vh-220px)]">
              <ul className="px-2 py-2">
                {draft.sections.map((section, idx) => (
                  <li key={section.id}>
                    <button
                      onClick={() => {
                        setActiveSection(section.id)
                        const el = window.document.getElementById(`section-${section.id}`)
                        el?.scrollIntoView({ behavior: "smooth", block: "start" })
                      }}
                      className={cn(
                        "block w-full rounded px-2 py-1.5 text-left text-xs transition-colors",
                        activeSection === section.id
                          ? "bg-muted font-medium text-foreground"
                          : "text-muted-foreground hover:bg-muted/50 hover:text-foreground",
                      )}
                    >
                      <span className="font-mono text-[10px] text-muted-foreground">
                        {String(idx + 1).padStart(2, "0")}
                      </span>{" "}
                      {section.heading}
                      {section.suggestions.length > 0 && (
                        <span className="ml-1 inline-flex h-3.5 w-3.5 items-center justify-center rounded-full bg-amber-500/20 text-[8px] font-bold text-amber-700 dark:text-amber-300">
                          {section.suggestions.length}
                        </span>
                      )}
                    </button>
                  </li>
                ))}
              </ul>
              <div className="border-t border-border p-2">
                <Button variant="ghost" size="sm" className="w-full gap-1 text-[11px]">
                  <Plus className="h-3 w-3" />
                  Add section
                </Button>
              </div>
            </ScrollArea>
          </aside>

          {/* Editor */}
          <div className="flex flex-col bg-background">
            <ScrollArea className="h-[calc(100vh-220px)]">
              <article ref={editorRef} className="mx-auto max-w-3xl px-10 py-12">
                {draft.sections.map((section) => (
                  <SectionBlock
                    key={section.id}
                    section={section}
                    citations={section.citations}
                    matter={matter}
                    onAcceptSuggestion={(sid) => acceptSuggestion(section.id, sid)}
                    onRejectSuggestion={(sid) => rejectSuggestion(section.id, sid)}
                    onBodyChange={(body) => updateSectionBody(section.id, body)}
                    onFocus={() => setActiveSection(section.id)}
                  />
                ))}
              </article>
            </ScrollArea>

            {/* AI command bar */}
            <form
              onSubmit={handlePrompt}
              className="flex items-center gap-2 border-t border-border bg-card px-4 py-3"
            >
              <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-foreground text-background">
                <Sparkles className="h-3.5 w-3.5" />
              </div>
              <div className="relative flex-1">
                <Input
                  value={aiPrompt}
                  onChange={(e) => setAiPrompt(e.target.value)}
                  placeholder='Ask AI to edit this draft. e.g. "Tighten the negligence section" or "Add citation for fitness for habitation"'
                  className="h-9 border-border pl-3 pr-20 text-xs"
                  disabled={pendingPrompt}
                />
                <kbd className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 select-none rounded border border-border bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
                  ⌘K
                </kbd>
              </div>
              <Button type="submit" size="sm" disabled={pendingPrompt || !aiPrompt.trim()} className="gap-1">
                {pendingPrompt ? (
                  <>
                    <Wand2 className="h-3.5 w-3.5 animate-pulse" />
                    Thinking
                  </>
                ) : (
                  <>
                    <Send className="h-3.5 w-3.5" />
                    Generate
                  </>
                )}
              </Button>
            </form>
          </div>

          {/* Right panel */}
          <aside className="bg-card lg:border-l lg:border-border">
            <Tabs value={rightTab} onValueChange={(v) => setRightTab(v as RightTab)}>
              <div className="border-b border-border px-3 pt-3">
                <TabsList className="grid w-full grid-cols-4 bg-muted/40">
                  <TabsTrigger value="sources" className="gap-1 text-[11px]">
                    <Quote className="h-3 w-3" />
                    Sources
                  </TabsTrigger>
                  <TabsTrigger value="citecheck" className="gap-1 text-[11px]">
                    <ShieldCheck className="h-3 w-3" />
                    Check
                  </TabsTrigger>
                  <TabsTrigger value="outline" className="gap-1 text-[11px]">
                    <ListTree className="h-3 w-3" />
                    TOC
                  </TabsTrigger>
                  <TabsTrigger value="versions" className="gap-1 text-[11px]">
                    <History className="h-3 w-3" />
                    History
                  </TabsTrigger>
                </TabsList>
              </div>

              <ScrollArea className="h-[calc(100vh-272px)]">
                <TabsContent value="sources" className="m-0 p-4">
                  <SourcesPanel
                    citations={allCitations}
                    activeSectionId={activeSection}
                    sections={draft.sections}
                    matter={matter}
                  />
                </TabsContent>
                <TabsContent value="citecheck" className="m-0 p-4">
                  <CiteCheckPanel draft={draft} matter={matter} />
                </TabsContent>
                <TabsContent value="outline" className="m-0 p-4">
                  <OutlinePanel sections={draft.sections} active={activeSection} />
                </TabsContent>
                <TabsContent value="versions" className="m-0 p-4">
                  <VersionsPanel versions={draft.versions ?? []} />
                </TabsContent>
              </ScrollArea>
            </Tabs>
          </aside>
        </div>
      </div>
    </TooltipProvider>
  )
}

/* -------------------------------------------------------------------------- */
/*                                  Section                                   */
/* -------------------------------------------------------------------------- */

function applySuggestionToBody(body: string, suggestion?: DraftSuggestion): string {
  if (!suggestion) return body
  if (suggestion.kind === "rewrite" && suggestion.original) {
    return body.replace(suggestion.original, suggestion.proposed)
  }
  if (suggestion.kind === "insert") {
    return body + "\n\n" + suggestion.proposed
  }
  return body
}

function activeSectionBody(sections: DraftSection[], activeSection: string) {
  return sections.find((section) => section.id === activeSection)?.body ?? ""
}

interface SectionBlockProps {
  section: DraftSection
  citations: DraftCitation[]
  matter: Matter
  onAcceptSuggestion: (id: string) => void
  onRejectSuggestion: (id: string) => void
  onBodyChange: (body: string) => void
  onFocus: () => void
}

function SectionBlock({
  section,
  citations,
  matter,
  onAcceptSuggestion,
  onRejectSuggestion,
  onBodyChange,
  onFocus,
}: SectionBlockProps) {
  return (
    <section
      id={`section-${section.id}`}
      className="group relative mb-10"
      onFocus={onFocus}
      onClick={onFocus}
    >
      <header className="mb-4 flex items-baseline justify-between gap-3 border-b border-border pb-2">
        <h2 className="font-sans text-lg font-semibold tracking-tight text-foreground">
          {section.heading}
        </h2>
        {section.tone && (
          <span className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
            {section.tone}
          </span>
        )}
      </header>

      {/* Inline AI suggestions */}
      {section.suggestions.map((sg) => (
        <SuggestionBlock
          key={sg.id}
          suggestion={sg}
          onAccept={() => onAcceptSuggestion(sg.id)}
          onReject={() => onRejectSuggestion(sg.id)}
        />
      ))}

      <textarea
        value={section.body}
        onChange={(event) => onBodyChange(event.target.value)}
        className="min-h-52 w-full resize-y rounded border border-border bg-card px-4 py-3 font-serif text-[15px] leading-7 text-foreground focus:border-primary focus:outline-none"
      />

      {citations.length > 0 && (
        <div className="mt-3 space-y-3 font-serif text-[15px] leading-7 text-foreground">
          {section.body.split("\n\n").filter(Boolean).map((para, idx) => (
            <ParagraphWithCitations
              key={idx}
              text={para}
              citations={citations}
              matter={matter}
            />
          ))}
        </div>
      )}

      {/* Comments rendered in margin-style block */}
      {section.comments && section.comments.length > 0 && (
        <div className="mt-4 space-y-1.5 border-l-2 border-border pl-3">
          {section.comments.map((c) => (
            <CommentItem key={c.id} comment={c} />
          ))}
        </div>
      )}
    </section>
  )
}

function ParagraphWithCitations({
  text,
  citations,
  matter,
}: {
  text: string
  citations: DraftCitation[]
  matter: Matter
}) {
  // Tokenize text by {{cite:CIT-XXX}} markers.
  const parts = text.split(/(\{\{cite:[^}]+\}\})/g)
  const citationById = new Map(citations.map((c) => [c.id, c]))

  return (
    <p>
      {parts.map((part, i) => {
        const match = part.match(/^\{\{cite:([^}]+)\}\}$/)
        if (match) {
          const cite = citationById.get(match[1])
          if (!cite) return <span key={i} className="text-muted-foreground">[?]</span>
          return <CitationPill key={i} citation={cite} matter={matter} />
        }
        return <span key={i}>{part}</span>
      })}
    </p>
  )
}

function CitationPill({ citation, matter }: { citation: DraftCitation; matter: Matter }) {
  const sourceHref = citation.sourceKind === "document"
    ? matterDocumentHref(matter.id, citation.sourceId)
    : citation.sourceKind === "fact"
      ? matterFactsHref(matter.id, citation.sourceId)
      : citation.sourceKind === "statute"
        ? `/statutes/${citation.sourceId}`
        : `/sources/${citation.sourceId}`

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Link
          href={sourceHref}
          className={cn(
            "mx-0.5 inline-flex items-center gap-0.5 rounded border px-1 py-px font-sans align-baseline text-[10px] font-medium transition-colors",
            citation.verified
              ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 hover:bg-emerald-500/20 dark:text-emerald-300"
              : "border-amber-500/40 bg-amber-500/10 text-amber-700 hover:bg-amber-500/20 dark:text-amber-300",
          )}
        >
          {citation.verified ? (
            <CheckCircle2 className="h-2.5 w-2.5" />
          ) : (
            <AlertTriangle className="h-2.5 w-2.5" />
          )}
          {citation.shortLabel}
        </Link>
      </TooltipTrigger>
      <TooltipContent side="top" className="max-w-sm">
        <div className="space-y-1">
          <p className="font-mono text-[10px] uppercase tracking-wider opacity-70">
            {citation.sourceKind} · {citation.sourceId}
          </p>
          <p className="font-medium text-sm">{citation.fullLabel}</p>
          {citation.snippet && (
            <p className="text-xs italic opacity-90">{citation.snippet}</p>
          )}
        </div>
      </TooltipContent>
    </Tooltip>
  )
}

function SuggestionBlock({
  suggestion,
  onAccept,
  onReject,
}: {
  suggestion: DraftSuggestion
  onAccept: () => void
  onReject: () => void
}) {
  return (
    <Card className="mb-4 overflow-hidden border-foreground/30 bg-muted/30 font-sans">
      <div className="flex items-center justify-between border-b border-border bg-card px-3 py-2">
        <div className="flex items-center gap-2">
          <div className="flex h-5 w-5 items-center justify-center rounded bg-foreground text-background">
            <Sparkles className="h-3 w-3" />
          </div>
          <span className="text-xs font-semibold capitalize text-foreground">
            AI {suggestion.kind}
          </span>
          <Badge variant="outline" className="text-[10px]">
            {Math.round(suggestion.confidence * 100)}% confidence
          </Badge>
        </div>
        <div className="flex items-center gap-1">
          <Button
            size="sm"
            variant="ghost"
            onClick={onReject}
            className="h-7 gap-1 text-[11px] text-muted-foreground hover:text-foreground"
          >
            <XCircle className="h-3 w-3" />
            Reject
          </Button>
          <Button size="sm" onClick={onAccept} className="h-7 gap-1 text-[11px]">
            <CheckCircle2 className="h-3 w-3" />
            Accept
          </Button>
        </div>
      </div>

      <div className="space-y-2 px-3 py-3 text-xs">
        <p className="leading-relaxed text-muted-foreground">
          <Lightbulb className="mr-1 inline h-3 w-3" />
          {suggestion.rationale}
        </p>

        {suggestion.original && (
          <div className="rounded border border-rose-500/30 bg-rose-500/5 p-2 text-[12px] leading-relaxed text-rose-900/90 dark:text-rose-200/90">
            <p className="font-mono text-[10px] font-semibold uppercase tracking-wider opacity-70">
              − Remove
            </p>
            <p className="mt-1 line-through decoration-rose-500/60">{suggestion.original}</p>
          </div>
        )}

        <div className="rounded border border-emerald-500/30 bg-emerald-500/5 p-2 text-[12px] leading-relaxed text-emerald-900/90 dark:text-emerald-200/90">
          <p className="font-mono text-[10px] font-semibold uppercase tracking-wider opacity-70">
            + {suggestion.kind === "insert" ? "Insert" : "Replace with"}
          </p>
          <p className="mt-1">{suggestion.proposed}</p>
        </div>

        {suggestion.sources && suggestion.sources.length > 0 && (
          <div className="flex flex-wrap items-center gap-1 pt-1 text-[10px] text-muted-foreground">
            <span>Grounded in:</span>
            {suggestion.sources.map((source) => {
              const sourceId = typeof source === "string" ? source : source.id
              const sourceLabel = typeof source === "string" ? source : source.label
              return (
              <Badge key={sourceId} variant="outline" className="font-mono text-[9px]">
                {sourceLabel}
              </Badge>
              )
            })}
          </div>
        )}
      </div>
    </Card>
  )
}

function CommentItem({ comment }: { comment: DraftComment }) {
  return (
    <div className="text-[11px]">
      <div className="flex items-center gap-1.5">
        <MessageSquare className="h-3 w-3 text-muted-foreground" />
        <span className="font-medium text-foreground">{comment.author}</span>
        <span className="font-mono text-[10px] text-muted-foreground">{comment.timestamp}</span>
      </div>
      <p className="mt-0.5 leading-relaxed text-muted-foreground">{comment.body}</p>
    </div>
  )
}

/* -------------------------------------------------------------------------- */
/*                                Right panels                                */
/* -------------------------------------------------------------------------- */

function SourcesPanel({
  citations,
  activeSectionId,
  sections,
  matter,
}: {
  citations: DraftCitation[]
  activeSectionId: string
  sections: DraftSection[]
  matter: Matter
}) {
  const activeSection = sections.find((s) => s.id === activeSectionId)
  const sectionCites = activeSection?.citations ?? []
  const verified = sectionCites.filter((c) => c.verified).length

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
          In active section
        </h3>
        <p className="mt-1 text-xs text-foreground">
          {sectionCites.length} citations · {verified} verified
        </p>
      </div>
      {sectionCites.length === 0 ? (
        <p className="text-xs text-muted-foreground">No citations in this section yet.</p>
      ) : (
        <ul className="space-y-2">
          {sectionCites.map((c) => (
            <li key={c.id}>
              <CitationCard citation={c} matter={matter} />
            </li>
          ))}
        </ul>
      )}
      <div className="border-t border-border pt-4">
        <h3 className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
          All citations in draft
        </h3>
        <p className="mt-1 text-xs text-muted-foreground">{citations.length} total</p>
      </div>
    </div>
  )
}

function CitationCard({ citation, matter }: { citation: DraftCitation; matter: Matter }) {
  const sourceHref = citation.sourceKind === "document"
    ? matterDocumentHref(matter.id, citation.sourceId)
    : citation.sourceKind === "fact"
      ? matterFactsHref(matter.id, citation.sourceId)
      : citation.sourceKind === "statute"
        ? `/statutes/${citation.sourceId}`
        : `/sources/${citation.sourceId}`

  return (
    <Link
      href={sourceHref}
      className="block rounded-md border border-border bg-background p-2.5 text-xs transition-colors hover:border-foreground/20 hover:bg-muted/40"
    >
      <div className="flex items-start justify-between gap-2">
        <Badge variant="outline" className="text-[9px] capitalize">
          {citation.sourceKind}
        </Badge>
        {citation.verified ? (
          <CheckCircle2 className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
        ) : (
          <AlertTriangle className="h-3 w-3 text-amber-600 dark:text-amber-400" />
        )}
      </div>
      <p className="mt-1 font-medium leading-tight text-foreground">{citation.fullLabel}</p>
      {citation.snippet && (
        <p className="mt-1 line-clamp-2 italic text-muted-foreground">{citation.snippet}</p>
      )}
    </Link>
  )
}

function CiteCheckPanel({ draft, matter }: { draft: Draft; matter: Matter }) {
  const allCites = draft.sections.flatMap((s) => s.citations)
  const verified = allCites.filter((c) => c.verified)
  const unverified = allCites.filter((c) => !c.verified)
  const issues = draft.citeCheckIssues ?? []
  const factFindings = (matter.fact_check_findings ?? []).filter(
    (finding) => finding.draft_id === draft.id || finding.draft_id === draft.draft_id,
  )
  const citationFindings = (matter.citation_check_findings ?? []).filter(
    (finding) => finding.draft_id === draft.id || finding.draft_id === draft.draft_id,
  )
  const persistedFindingCount = factFindings.length + citationFindings.length

  return (
    <div className="space-y-4">
      <Card className="p-3">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium text-foreground">Status</span>
          {issues.length === 0 && unverified.length === 0 && persistedFindingCount === 0 ? (
            <Badge className="gap-1 bg-emerald-600/15 text-emerald-700 hover:bg-emerald-600/15 dark:text-emerald-300">
              <CheckCircle2 className="h-3 w-3" />
              Clean
            </Badge>
          ) : (
            <Badge variant="outline" className="gap-1 border-amber-500/40 text-amber-700 dark:text-amber-400">
              <AlertTriangle className="h-3 w-3" />
              {issues.length + unverified.length + persistedFindingCount} flag{issues.length + unverified.length + persistedFindingCount === 1 ? "" : "s"}
            </Badge>
          )}
        </div>
        <div className="mt-3 grid grid-cols-2 gap-2 text-xs">
          <div className="rounded border border-emerald-500/30 bg-emerald-500/5 p-2">
            <p className="text-[10px] uppercase text-emerald-700 dark:text-emerald-300">Verified</p>
            <p className="mt-0.5 font-mono text-lg font-semibold">{verified.length}</p>
          </div>
          <div className="rounded border border-amber-500/30 bg-amber-500/5 p-2">
            <p className="text-[10px] uppercase text-amber-700 dark:text-amber-400">Unverified</p>
            <p className="mt-0.5 font-mono text-lg font-semibold">{unverified.length}</p>
          </div>
        </div>
      </Card>

      {persistedFindingCount > 0 && (
        <div>
          <h3 className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
            Persisted findings
          </h3>
          <ul className="mt-2 space-y-2">
            {factFindings.map((finding) => (
              <li key={finding.finding_id} className="rounded border border-amber-500/30 bg-amber-500/5 p-2.5 text-xs">
                <div className="flex items-start gap-2">
                  <AlertTriangle className="mt-0.5 h-3 w-3 shrink-0 text-amber-600 dark:text-amber-400" />
                  <div>
                    <p className="font-medium text-foreground">{finding.finding_type}</p>
                    <p className="mt-0.5 leading-relaxed text-muted-foreground">{finding.message}</p>
                    {finding.paragraph_id && (
                      <p className="mt-1 font-mono text-[10px] text-muted-foreground">
                        {finding.paragraph_id}
                      </p>
                    )}
                  </div>
                </div>
              </li>
            ))}
            {citationFindings.map((finding) => (
              <li key={finding.finding_id} className="rounded border border-amber-500/30 bg-amber-500/5 p-2.5 text-xs">
                <div className="flex items-start gap-2">
                  <AlertTriangle className="mt-0.5 h-3 w-3 shrink-0 text-amber-600 dark:text-amber-400" />
                  <div>
                    <p className="font-medium text-foreground">{finding.finding_type}</p>
                    <p className="mt-0.5 leading-relaxed text-muted-foreground">{finding.message}</p>
                    <p className="mt-1 font-mono text-[10px] text-muted-foreground">
                      {finding.citation || "missing citation"}
                    </p>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}

      {issues.length > 0 && (
        <div>
          <h3 className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
            Issues
          </h3>
          <ul className="mt-2 space-y-2">
            {issues.map((issue) => (
              <li
                key={issue.id}
                className="rounded border border-amber-500/30 bg-amber-500/5 p-2.5 text-xs"
              >
                <div className="flex items-start gap-2">
                  <AlertTriangle className="mt-0.5 h-3 w-3 shrink-0 text-amber-600 dark:text-amber-400" />
                  <div>
                    <p className="font-medium text-foreground">{issue.title}</p>
                    <p className="mt-0.5 leading-relaxed text-muted-foreground">{issue.detail}</p>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}

      {unverified.length > 0 && (
        <div>
          <h3 className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
            Unverified citations
          </h3>
          <ul className="mt-2 space-y-1">
            {unverified.map((c) => (
              <li
                key={c.id}
                className="flex items-start gap-2 rounded border border-border bg-background p-2 text-xs"
              >
                <AlertTriangle className="mt-0.5 h-3 w-3 shrink-0 text-amber-600 dark:text-amber-400" />
                <div className="min-w-0 flex-1">
                  <p className="font-medium text-foreground">{c.fullLabel}</p>
                  <p className="mt-0.5 font-mono text-[10px] text-muted-foreground">{c.id}</p>
                </div>
                <Button size="sm" variant="ghost" className="h-6 text-[10px]">
                  Verify
                </Button>
              </li>
            ))}
          </ul>
        </div>
      )}

      <Button variant="outline" size="sm" className="w-full gap-1.5 bg-transparent">
        <ShieldCheck className="h-3.5 w-3.5" />
        Run full cite-check
      </Button>
    </div>
  )
}

function OutlinePanel({ sections, active }: { sections: DraftSection[]; active: string }) {
  return (
    <ol className="space-y-1.5">
      {sections.map((s, idx) => (
        <li key={s.id}>
          <a
            href={`#section-${s.id}`}
            className={cn(
              "block rounded px-2 py-1.5 text-xs",
              active === s.id
                ? "bg-muted font-medium text-foreground"
                : "text-muted-foreground hover:bg-muted/50",
            )}
          >
            <span className="mr-2 font-mono text-[10px]">{idx + 1}.</span>
            {s.heading}
          </a>
        </li>
      ))}
    </ol>
  )
}

function VersionsPanel({ versions }: { versions: NonNullable<Draft["versions"]> }) {
  if (versions.length === 0) {
    return <p className="text-xs text-muted-foreground">No version history yet.</p>
  }
  return (
    <ul className="space-y-2">
      {versions.map((v) => (
        <li
          key={v.id}
          className="rounded border border-border bg-background p-2.5 text-xs"
        >
          <div className="flex items-center justify-between">
            <span className="font-mono text-[10px] font-semibold text-foreground">
              {v.label}
            </span>
            <span className="font-mono text-[10px] text-muted-foreground">{v.timestamp}</span>
          </div>
          <p className="mt-1 leading-relaxed text-muted-foreground">{v.summary}</p>
          <p className="mt-1 text-[10px] text-foreground/70">by {v.author}</p>
        </li>
      ))}
    </ul>
  )
}

/* eslint-disable @typescript-eslint/no-unused-vars */
const _icon = FileText
