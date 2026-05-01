"use client"

import { useState } from "react"
import Link from "next/link"
import {
  Send,
  Sparkles,
  FileText,
  Quote,
  CheckCircle2,
  AlertTriangle,
  RotateCcw,
  Search,
  History,
  Plus,
  Filter,
} from "lucide-react"
import type { Matter, MatterChatMessage, MatterChatCitation } from "@/lib/casebuilder/types"
import { matterClaimsHref, matterDocumentHref, matterFactsHref } from "@/lib/casebuilder/routes"
import { askMatter } from "@/lib/casebuilder/api"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { cn } from "@/lib/utils"

interface AskMatterProps {
  matter: Matter
}

const STARTERS = [
  "Summarize the strongest claims and weakest defenses for this matter.",
  "What evidence do we have that the unit was uninhabitable?",
  "Draft a paragraph rebutting the affirmative defense of failure to mitigate.",
  "List all monetary amounts mentioned across the lease and ledger.",
  "What deadlines fall in the next 30 days, and what tasks are blocking?",
  "Compare the rent ledger with the tenant's payment records and flag inconsistencies.",
]

export function AskMatter({ matter }: AskMatterProps) {
  const [messages, setMessages] = useState<MatterChatMessage[]>(matter.chatHistory ?? [])
  const [input, setInput] = useState("")
  const [pending, setPending] = useState(false)
  const [scope, setScope] = useState<"all" | "documents" | "facts" | "claims">("all")

  const send = async (text: string) => {
    if (!text.trim()) return
    const userMsg: MatterChatMessage = {
      id: `msg-${Date.now()}`,
      role: "user",
      content: text,
      timestamp: new Date().toISOString(),
      citations: [],
    }
    setMessages((prev) => [...prev, userMsg])
    setInput("")
    setPending(true)

    const result = await askMatter(matter.id, { question: text, scope })
    if (result.data) {
      const reply: MatterChatMessage = {
        id: `msg-${Date.now() + 1}`,
        role: "assistant",
        content: result.data.answer,
        citations: mapAskCitations(result.data.citations),
        timestamp: new Date().toISOString(),
        confidence: 0.72,
        reasoning: [
          `Queried matter scope: ${scope}.`,
          `Matched ${result.data.related_documents.length} document sources and ${result.data.related_facts.length} facts.`,
          `Retrieved ${result.data.citations.filter((citation) => citation.kind === "statute").length} source-backed authority matches.`,
          ...result.data.warnings.slice(0, 2),
        ],
      }
      setMessages((prev) => [...prev, reply])
    } else {
      const reply: MatterChatMessage = {
        id: `msg-${Date.now() + 1}`,
        role: "assistant",
        content: result.error || "Matter ask is unavailable right now.",
        citations: [],
        timestamp: new Date().toISOString(),
        reasoning: ["The live matter ask endpoint returned an error."],
      }
      setMessages((prev) => [...prev, reply])
    }
    setPending(false)
  }

  return (
    <div className="flex flex-col">
      <div className="border-b border-border bg-background px-6 py-4">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <h1 className="flex items-center gap-2 text-xl font-semibold tracking-tight text-foreground">
              <Sparkles className="h-5 w-5" />
              Ask {matter.shortName}
            </h1>
            <p className="mt-1 text-sm text-muted-foreground">
              A research assistant grounded in the documents, facts, and authorities of this
              matter only.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5 bg-transparent"
              onClick={() => setMessages([])}
            >
              <RotateCcw className="h-3.5 w-3.5" />
              New thread
            </Button>
          </div>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Filter className="h-3 w-3" />
            Scope:
          </span>
          {(["all", "documents", "facts", "claims"] as const).map((s) => (
            <button
              key={s}
              onClick={() => setScope(s)}
              className={cn(
                "rounded-full border px-2.5 py-1 text-[11px] font-medium transition-colors capitalize",
                scope === s
                  ? "border-foreground bg-foreground text-background"
                  : "border-border bg-background text-muted-foreground hover:bg-muted",
              )}
            >
              {s === "all" ? "Whole matter" : s}
            </button>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_300px]">
        {/* Conversation */}
        <div className="flex flex-col">
          <ScrollArea className="h-[calc(100vh-280px)]">
            <div className="mx-auto max-w-3xl space-y-6 px-6 py-8">
              {messages.length === 0 ? (
                <EmptyState matter={matter} onPick={(t) => send(t)} />
              ) : (
                messages.map((msg) => (
                  <MessageBlock key={msg.id} message={msg} matter={matter} />
                ))
              )}
              {pending && (
                <div className="flex items-start gap-3">
                  <div className="flex h-8 w-8 items-center justify-center rounded-full bg-foreground text-background">
                    <Sparkles className="h-4 w-4 animate-pulse" />
                  </div>
                  <div className="rounded-lg border border-border bg-card px-3 py-2 text-xs text-muted-foreground">
                    Searching documents and synthesizing a grounded answer...
                  </div>
                </div>
              )}
            </div>
          </ScrollArea>

          {/* Input */}
          <form
            onSubmit={(e) => {
              e.preventDefault()
              send(input)
            }}
            className="border-t border-border bg-card px-4 py-3"
          >
            <div className="mx-auto flex max-w-3xl items-center gap-2">
              <div className="relative flex-1">
                <Search className="absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
                <Input
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  placeholder="Ask anything about this matter..."
                  className="h-10 pl-9 text-sm"
                  disabled={pending}
                />
              </div>
              <Button type="submit" disabled={!input.trim() || pending} className="gap-1">
                <Send className="h-3.5 w-3.5" />
                Ask
              </Button>
            </div>
          </form>
        </div>

        {/* Right rail */}
        <aside className="hidden border-l border-border bg-card lg:block">
          <ScrollArea className="h-[calc(100vh-220px)]">
            <div className="space-y-5 p-4">
              <div>
                <h3 className="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  <Sparkles className="h-3 w-3" />
                  Suggested prompts
                </h3>
                <ul className="mt-2 space-y-1">
                  {STARTERS.slice(0, 4).map((s) => (
                    <li key={s}>
                      <button
                        onClick={() => send(s)}
                        className="block w-full rounded border border-border bg-background px-2.5 py-1.5 text-left text-[11px] leading-snug text-foreground transition-colors hover:border-foreground/30 hover:bg-muted/40"
                      >
                        {s}
                      </button>
                    </li>
                  ))}
                </ul>
              </div>

              <div>
                <h3 className="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  <FileText className="h-3 w-3" />
                  Indexed sources
                </h3>
                <Card className="mt-2 p-3">
                  <ul className="space-y-1.5 text-xs">
                    <SourceStat label="Documents" count={matter.documents.length} />
                    <SourceStat label="Facts" count={matter.facts.length} />
                    <SourceStat label="Claims" count={matter.claims.length} />
                    <SourceStat
                      label="Chunks"
                      count={matter.documents.reduce((s, d) => s + d.chunks.length, 0)}
                    />
                  </ul>
                </Card>
              </div>

              <div>
                <h3 className="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  <History className="h-3 w-3" />
                  Recent threads
                </h3>
                <ul className="mt-2 space-y-1">
                  {(matter.recentThreads ?? []).map((t) => (
                    <li key={t.id}>
                      <button className="block w-full rounded px-2 py-1.5 text-left text-[11px] hover:bg-muted/50">
                        <p className="line-clamp-1 font-medium text-foreground">{t.title}</p>
                        <p className="font-mono text-[10px] text-muted-foreground">
                          {t.lastMessageAt}
                        </p>
                      </button>
                    </li>
                  ))}
                </ul>
                <Button variant="ghost" size="sm" className="mt-1 w-full gap-1 text-[11px]">
                  <Plus className="h-3 w-3" />
                  New thread
                </Button>
              </div>
            </div>
          </ScrollArea>
        </aside>
      </div>
    </div>
  )
}

function MessageBlock({ message, matter }: { message: MatterChatMessage; matter: Matter }) {
  if (message.role === "user") {
    return (
      <div className="flex justify-end">
        <div className="max-w-[75%] rounded-lg bg-foreground px-3.5 py-2.5 text-sm text-background">
          {message.content}
        </div>
      </div>
    )
  }

  return (
    <div className="flex items-start gap-3">
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-foreground text-background">
        <Sparkles className="h-4 w-4" />
      </div>
      <div className="min-w-0 flex-1 space-y-3">
        <div className="rounded-lg border border-border bg-card px-4 py-3 text-sm leading-relaxed text-foreground">
          <p className="whitespace-pre-wrap">{message.content}</p>
          {typeof message.confidence === "number" && (
            <div className="mt-3 flex items-center justify-between border-t border-border pt-2 text-[11px]">
              <Badge variant="outline" className="gap-1">
                <CheckCircle2 className="h-3 w-3" />
                {Math.round(message.confidence * 100)}% confidence
              </Badge>
              <span className="font-mono text-muted-foreground">{message.timestamp.slice(11, 16)}</span>
            </div>
          )}
        </div>

        {/* Citations */}
        {message.citations && message.citations.length > 0 && (
          <CitationsBlock citations={message.citations} matter={matter} />
        )}

        {/* Reasoning */}
        {message.reasoning && message.reasoning.length > 0 && (
          <details className="text-xs">
            <summary className="cursor-pointer text-[11px] font-medium text-muted-foreground hover:text-foreground">
              How I answered ({message.reasoning.length} steps)
            </summary>
            <ol className="mt-2 space-y-1 border-l-2 border-border pl-3 text-muted-foreground">
              {message.reasoning.map((step, i) => (
                <li key={i} className="leading-relaxed">
                  <span className="mr-2 font-mono text-[10px] opacity-70">{i + 1}.</span>
                  {step}
                </li>
              ))}
            </ol>
          </details>
        )}
      </div>
    </div>
  )
}

function CitationsBlock({
  citations,
  matter,
}: {
  citations: MatterChatCitation[]
  matter: Matter
}) {
  return (
    <div>
      <p className="mb-1.5 flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
        <Quote className="h-3 w-3" />
        Sources ({citations.length})
      </p>
      <ul className="space-y-1.5">
        {citations.map((c) => {
          const href = c.kind === "document"
            ? matterDocumentHref(matter.id, c.refId ?? c.sourceId, c.chunkId)
            : c.kind === "fact"
              ? matterFactsHref(matter.id, c.refId ?? c.sourceId)
              : c.kind === "claim"
                ? matterClaimsHref(matter.id, c.refId ?? c.sourceId)
                : c.kind === "statute"
                  ? `/statutes/${c.refId}`
                  : `/sources/${c.refId}`
          return (
            <li key={c.id}>
              <Link
                href={href}
                className="flex items-start gap-2 rounded-md border border-border bg-background p-2.5 text-xs transition-colors hover:border-foreground/30 hover:bg-muted/30"
              >
                <span className="mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded bg-muted font-mono text-[9px] font-semibold text-foreground">
                  {c.indexLabel}
                </span>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-1.5">
                    <Badge variant="outline" className="text-[9px] capitalize">
                      {c.kind}
                    </Badge>
                    <span className="truncate font-medium text-foreground">{c.title}</span>
                  </div>
                  {c.snippet && (
                    <p className="mt-1 line-clamp-2 italic text-muted-foreground">
                      &quot;{c.snippet}&quot;
                    </p>
                  )}
                </div>
              </Link>
            </li>
          )
        })}
      </ul>
    </div>
  )
}

function SourceStat({ label, count }: { label: string; count: number }) {
  return (
    <li className="flex items-center justify-between">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-mono font-semibold text-foreground">{count}</span>
    </li>
  )
}

function EmptyState({
  matter,
  onPick,
}: {
  matter: Matter
  onPick: (text: string) => void
}) {
  return (
    <div className="space-y-6 py-8 text-center">
      <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-muted">
        <Sparkles className="h-6 w-6 text-foreground" />
      </div>
      <div>
        <h2 className="text-lg font-semibold text-foreground">
          Ask anything about {matter.shortName}
        </h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Every answer is grounded in this matter&apos;s indexed documents, facts, and
          authorities.
        </p>
      </div>
      <ul className="mx-auto grid max-w-xl grid-cols-1 gap-2 text-left">
        {STARTERS.map((s) => (
          <li key={s}>
            <button
              onClick={() => onPick(s)}
              className="group flex w-full items-start gap-3 rounded-md border border-border bg-card px-3 py-2.5 transition-colors hover:border-foreground/30 hover:bg-muted/40"
            >
              <Sparkles className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground group-hover:text-foreground" />
              <span className="text-xs leading-relaxed text-foreground">{s}</span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  )
}

function mapAskCitations(citations: Array<{ citation_id: string; kind: string; source_id: string; title: string; snippet?: string | null }>): MatterChatCitation[] {
  return citations.map((citation, index) => {
    const kind = citation.kind === "document" || citation.kind === "fact" || citation.kind === "claim" || citation.kind === "statute" || citation.kind === "rule" || citation.kind === "source"
      ? citation.kind
      : "source"
    return {
      id: citation.citation_id,
      indexLabel: String(index + 1),
      kind,
      refId: citation.source_id,
      sourceId: citation.source_id,
      sourceKind: kind === "document" || kind === "fact" || kind === "statute" || kind === "rule" ? kind : "statute",
      shortLabel: citation.title,
      fullLabel: citation.title,
      title: citation.title,
      snippet: citation.snippet ?? undefined,
    }
  })
}

/* eslint-disable-next-line @typescript-eslint/no-unused-vars */
const _icon = AlertTriangle
