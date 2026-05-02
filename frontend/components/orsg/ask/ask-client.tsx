"use client"

import { useState } from "react"
import Link from "next/link"
import type { AskAnswer } from "@/lib/types"
import type { DataSource } from "@/lib/data-state"
import { askWithFallbackState } from "@/lib/api"
import { cn } from "@/lib/utils"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import {
  Sparkles,
  Send,
  Type,
  ShieldAlert,
  Clock,
  AlertTriangle,
  BookOpen,
  Hash,
  ChevronRight,
  Activity,
} from "lucide-react"
import { ChunkTypeBadge, TrustBadge } from "@/components/orsg/badges"

const MODES = [
  { id: "research", label: "Legal research" },
  { id: "plain", label: "Plain English" },
  { id: "complaint", label: "Complaint defense" },
  { id: "compliance", label: "Compliance" },
  { id: "drafting", label: "Drafting support" },
] as const

const EXAMPLE_QUESTIONS = [
  "What Oregon laws define district attorney duties?",
  "What are the security deposit deadlines under ORS chapter 90?",
  "Which provisions control habitability repair duties?",
]

interface Props {
  initialQuery: string
  initialAnswer: AskAnswer | null
  initialDataSource?: DataSource
  initialDataError?: string
}

export function AskClient({
  initialQuery,
  initialAnswer,
  initialDataSource = "live",
  initialDataError,
}: Props) {
  const [q, setQ] = useState(initialQuery)
  const [mode, setMode] = useState("research")
  const [answer, setAnswer] = useState<AskAnswer | null>(initialAnswer)
  const [dataSource, setDataSource] = useState<DataSource>(initialDataSource)
  const [dataError, setDataError] = useState<string | undefined>(initialDataError)
  const [loading, setLoading] = useState(false)

  async function submitQuestion(nextQuestion = q) {
    const question = nextQuestion.trim()
    if (!question || loading) return
    setQ(question)
    setLoading(true)
    try {
      const next = await askWithFallbackState(question, mode)
      setAnswer(next.data)
      setDataSource(next.source)
      setDataError(next.error)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <DataStateBanner source={dataSource} error={dataError} label="Ask response" />
      {/* Question bar */}
      <header className="border-b border-border bg-card px-4 py-4 sm:px-6">
        <div className="mx-auto max-w-5xl">
          <div className="mb-2 flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <Sparkles className="h-3 w-3 text-primary" />
            ask ORSGraph
          </div>
          <h1 className="text-2xl font-semibold tracking-normal text-foreground">
            Ask a source-grounded legal question.
          </h1>
          <p className="mt-1 max-w-3xl text-sm leading-6 text-muted-foreground">
            Answers stay tied to parsed provisions, retrieved chunks, definitions, caveats, and QC notes.
          </p>
          <div className="mt-4 flex items-start gap-2 rounded-md border border-border bg-background p-2 shadow-sm focus-within:border-primary">
            <Sparkles className="mt-2 h-4 w-4 flex-none text-primary" />
            <textarea
              value={q}
              onChange={(e) => setQ(e.target.value)}
              onKeyDown={(event) => {
                if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
                  event.preventDefault()
                  submitQuestion()
                }
              }}
              rows={2}
              placeholder="Ask anything grounded in source-backed authority..."
              className="flex-1 resize-none bg-transparent py-1.5 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
            />
            <button
              onClick={() => submitQuestion()}
              disabled={loading || q.trim().length === 0}
              className="mt-1 flex h-8 items-center gap-1 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
            >
              <Send className="h-3 w-3" />
              {loading ? "asking" : "answer"}
            </button>
          </div>

          <div className="mt-3 flex items-center gap-1 overflow-x-auto scrollbar-thin">
            <span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
              mode:
            </span>
            {MODES.map((m) => (
              <button
                key={m.id}
                onClick={() => setMode(m.id)}
                className={cn(
                  "whitespace-nowrap rounded px-2.5 py-1 font-mono text-[11px] uppercase tracking-wide transition-colors",
                  mode === m.id
                    ? "bg-primary/15 text-primary"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                {m.label}
              </button>
            ))}
          </div>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        {/* Answer panel */}
        <div className="flex-1 overflow-y-auto scrollbar-thin">
          {answer ? (
            <div className="mx-auto max-w-3xl px-6 py-8">
              {/* Question echo */}
              <div className="mb-6 rounded border border-border bg-card p-4">
                <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                  question
                </div>
                <p className="mt-1 text-base text-foreground">{answer.question}</p>
              </div>

              {/* Short answer */}
              <Section title="Short answer" trust="generated">
                <p className="text-base leading-relaxed text-foreground">{answer.short_answer}</p>
              </Section>

              {/* Controlling law */}
              <Section title="Controlling law" trust="parsed">
                <ul className="space-y-2">
                  {answer.controlling_law.map((c) => (
                    <li
                      key={c.canonical_id}
                      className="flex items-start gap-3 rounded border border-border bg-card p-3"
                    >
                      <BookOpen className="mt-0.5 h-3.5 w-3.5 flex-none text-primary" />
                      <div className="min-w-0 flex-1">
                        <Link
                          href={`/statutes/${c.canonical_id}`}
                          className="font-mono text-sm font-semibold text-primary hover:underline"
                        >
                          {c.citation}
                        </Link>
                        <p className="text-sm text-muted-foreground">{c.reason}</p>
                      </div>
                      <ChevronRight className="h-4 w-4 text-muted-foreground" />
                    </li>
                  ))}
                </ul>
              </Section>

              {/* Relevant provisions */}
              <Section title="Relevant provisions" trust="parsed">
                <ul className="divide-y divide-border overflow-hidden rounded border border-border bg-card">
                  {answer.relevant_provisions.map((p) => (
                    <li key={p.provision_id} className="px-3 py-2.5 hover:bg-muted/30">
                      <Link
                        href={`/provisions/${encodeURIComponent(p.provision_id)}`}
                        className="font-mono text-sm font-medium text-primary hover:underline"
                      >
                        {p.citation}
                      </Link>
                      <p className="mt-0.5 line-clamp-2 text-sm text-foreground">
                        {p.text_preview}
                      </p>
                    </li>
                  ))}
                </ul>
              </Section>

              {/* Definitions / exceptions / deadlines */}
              <div className="mb-6 grid grid-cols-1 gap-3 lg:grid-cols-3">
                <MiniSection icon={Type} title="Definitions" accent="text-chart-1">
                  {answer.definitions.map((d, i) => (
                    <div key={i} className="text-xs">
                      <span className="font-serif italic text-foreground">"{d.term}"</span>
                      <p className="mt-0.5 text-muted-foreground">{d.text}</p>
                      <span className="mt-0.5 block font-mono text-[10px] text-primary">
                        {d.source}
                      </span>
                    </div>
                  ))}
                </MiniSection>

                <MiniSection icon={ShieldAlert} title="Exceptions" accent="text-warning">
                  {answer.exceptions.map((e, i) => (
                    <div key={i} className="text-xs">
                      <p className="text-foreground">{e.text}</p>
                      <span className="mt-0.5 block font-mono text-[10px] text-primary">
                        {e.source}
                      </span>
                    </div>
                  ))}
                </MiniSection>

                <MiniSection icon={Clock} title="Deadlines" accent="text-chart-3">
                  {answer.deadlines.map((d, i) => (
                    <div key={i} className="text-xs">
                      <div className="flex items-baseline gap-1.5">
                        <span className="font-mono text-sm font-semibold text-chart-3">
                          {d.duration}
                        </span>
                        <span className="text-foreground">{d.description}</span>
                      </div>
                      <span className="mt-0.5 block font-mono text-[10px] text-primary">
                        {d.source}
                      </span>
                    </div>
                  ))}
                </MiniSection>
              </div>

              {/* Citations */}
              <Section title="Citations" trust="parsed">
                <div className="flex flex-wrap gap-1.5">
                  {answer.citations.map((c) => (
                    <Link
                      key={c}
                      href={`/statutes/or:ors:${c.replace("ORS ", "")}`}
                      className="inline-flex items-center gap-1 rounded border border-border bg-card px-2 py-1 font-mono text-xs text-foreground hover:border-primary hover:text-primary"
                    >
                      <Hash className="h-3 w-3 text-muted-foreground" />
                      {c}
                    </Link>
                  ))}
                </div>
              </Section>

              {/* Caveats */}
              {answer.caveats.length > 0 && (
                <Section title="Caveats / unknowns">
                  <ul className="space-y-1.5">
                    {answer.caveats.map((c, i) => (
                      <li
                        key={i}
                        className="flex items-start gap-2 rounded border border-warning/30 bg-warning/5 p-2.5 text-sm text-foreground"
                      >
                        <AlertTriangle className="mt-0.5 h-3.5 w-3.5 flex-none text-warning" />
                        <span>{c}</span>
                      </li>
                    ))}
                  </ul>
                </Section>
              )}
            </div>
          ) : (
            <AskEmptyState onAsk={submitQuestion} loading={loading} />
          )}
        </div>

        {/* Right inspector: source pack + retrieved chunks */}
        {answer && (
          <aside className="hidden w-80 flex-col overflow-hidden border-l border-border bg-card xl:flex">
            <div className="overflow-y-auto scrollbar-thin">
              <header className="border-b border-border px-4 py-3">
                <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                  source pack
                </h2>
                <p className="mt-1 text-sm text-foreground">
                  Provisions, definitions, and chunks used to build this answer.
                </p>
              </header>

              <Panel title="Retrieved chunks" count={answer.retrieved_chunks.length}>
                <ul className="space-y-2">
                  {answer.retrieved_chunks.map((c) => (
                    <li key={c.chunk_id} className="rounded border border-border bg-background p-2">
                      <div className="flex items-center justify-between gap-2">
                        <ChunkTypeBadge type={c.chunk_type} />
                        <span className="font-mono text-[10px] tabular-nums text-foreground">
                          {c.score.toFixed(2)}
                        </span>
                      </div>
                      <div className="mt-1 font-mono text-[10px] text-muted-foreground">
                        {c.chunk_id}
                      </div>
                      <p className="mt-1 line-clamp-3 text-xs text-foreground">{c.preview}</p>
                    </li>
                  ))}
                </ul>
              </Panel>

              <Panel title="Provisions used" count={answer.relevant_provisions.length}>
                <ul className="space-y-1">
                  {answer.relevant_provisions.map((p) => (
                    <li key={p.provision_id} className="font-mono text-xs">
                      <Link
                        href={`/provisions/${encodeURIComponent(p.provision_id)}`}
                        className="text-primary hover:underline"
                      >
                        {p.citation}
                      </Link>
                    </li>
                  ))}
                </ul>
              </Panel>

              <Panel title="QC notes" count={answer.qc_notes.length}>
                {answer.qc_notes.length === 0 ? (
                  <p className="text-xs text-muted-foreground">All sources passed QC.</p>
                ) : (
                  <ul className="space-y-1.5">
                    {answer.qc_notes.map((n, i) => (
                      <li
                        key={i}
                        className="flex items-start gap-1.5 text-xs text-muted-foreground"
                      >
                        <AlertTriangle className="mt-0.5 h-3 w-3 flex-none text-warning" />
                        <span>{n}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </Panel>

              <div className="border-t border-border px-4 py-3 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                <div className="mb-1 flex items-center gap-1.5">
                  <Activity className="h-3 w-3 text-primary" />
                  grounding policy
                </div>
                <ul className="space-y-0.5 normal-case">
                  <li>Answer drawn from Provision and LegalTextVersion nodes.</li>
                  <li>No generated summary used as primary source.</li>
                  <li>Each claim links to a parsed provision.</li>
                </ul>
              </div>
            </div>
          </aside>
        )}
      </div>
    </div>
  )
}

function AskEmptyState({
  onAsk,
  loading,
}: {
  onAsk: (question: string) => void
  loading: boolean
}) {
  return (
    <div className="mx-auto flex min-h-full max-w-3xl flex-col justify-center px-6 py-10">
      <div className="rounded border border-border bg-card p-5">
        <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          <Sparkles className="h-3.5 w-3.5 text-primary" />
          source-grounded questions
        </div>
        <div className="mt-3 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">examples</div>
        <div className="mt-4 grid gap-2">
          {EXAMPLE_QUESTIONS.map((question) => (
            <button
              key={question}
              type="button"
              disabled={loading}
              onClick={() => onAsk(question)}
              className="flex items-center justify-between gap-3 rounded border border-border bg-background px-3 py-2 text-left text-sm transition-colors hover:border-primary/50 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            >
              <span>{question}</span>
              <Send className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}

function Section({
  title,
  trust,
  children,
}: {
  title: string
  trust?: "official" | "parsed" | "extracted" | "generated" | "user_draft"
  children: React.ReactNode
}) {
  return (
    <section className="mb-6">
      <header className="mb-2 flex items-center justify-between">
        <h3 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          {title}
        </h3>
        {trust && <TrustBadge level={trust} />}
      </header>
      {children}
    </section>
  )
}

function MiniSection({
  icon: Icon,
  title,
  accent,
  children,
}: {
  icon: typeof Type
  title: string
  accent: string
  children: React.ReactNode
}) {
  return (
    <div className="rounded border border-border bg-card p-3">
      <div className="mb-2 flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        <Icon className={`h-3 w-3 ${accent}`} />
        {title}
      </div>
      <div className="space-y-2">{children}</div>
    </div>
  )
}

function Panel({
  title,
  count,
  children,
}: {
  title: string
  count: number
  children: React.ReactNode
}) {
  return (
    <section className="border-b border-border px-4 py-3">
      <h3 className="mb-2 flex items-center justify-between font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        <span>{title}</span>
        <span className="tabular-nums">{count}</span>
      </h3>
      {children}
    </section>
  )
}
