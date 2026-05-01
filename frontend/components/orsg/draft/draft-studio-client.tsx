"use client"

import { useMemo, useState } from "react"
import Link from "next/link"
import type { ComplaintAnalysis, ResponseType, RiskLevel } from "@/lib/types"
import { cn } from "@/lib/utils"
import {
  AlertTriangle,
  ArrowRight,
  BookOpen,
  CheckCircle2,
  ClipboardList,
  Download,
  FileText,
  Gavel,
  History,
  Link2,
  PanelRight,
  Save,
  Scale,
  ShieldCheck,
  Sparkles,
  Wand2,
} from "lucide-react"

type RightTab = "support" | "citations" | "qc"

const RESPONSE_META: Record<ResponseType, { label: string; className: string }> = {
  admit: { label: "Admit", className: "bg-success/15 text-success" },
  deny: { label: "Deny", className: "bg-destructive/15 text-destructive" },
  deny_in_part: { label: "Deny in part", className: "bg-warning/15 text-warning" },
  lack_knowledge: { label: "Lack knowledge", className: "bg-muted text-muted-foreground" },
  legal_conclusion: { label: "Legal conclusion", className: "bg-accent/20 text-accent-foreground" },
  needs_review: { label: "Needs review", className: "bg-primary/15 text-primary" },
}

const RISK_CLASS: Record<RiskLevel, string> = {
  high: "bg-destructive/15 text-destructive",
  medium: "bg-warning/15 text-warning",
  low: "bg-success/15 text-success",
}

const RIGHT_TABS: Array<{ id: RightTab; label: string; icon: typeof Link2 }> = [
  { id: "support", label: "Support", icon: Link2 },
  { id: "citations", label: "Cites", icon: BookOpen },
  { id: "qc", label: "QC", icon: ShieldCheck },
]

const RESPONSE_ORDER: ResponseType[] = [
  "admit",
  "deny",
  "deny_in_part",
  "lack_knowledge",
  "legal_conclusion",
  "needs_review",
]

export function DraftStudioClient({ analysis }: { analysis: ComplaintAnalysis }) {
  const [draftText, setDraftText] = useState(analysis.draft_answer_preview.trim())
  const [selectedAllegationId, setSelectedAllegationId] = useState(analysis.allegations[0]?.allegation_id ?? "")
  const [outlineSection, setOutlineSection] = useState("caption")
  const [rightTab, setRightTab] = useState<RightTab>("support")
  const [lastSavedAt, setLastSavedAt] = useState<string | null>(null)

  const selectedAllegation =
    analysis.allegations.find((allegation) => allegation.allegation_id === selectedAllegationId) ??
    analysis.allegations[0]

  const authorities = useMemo(() => {
    const rows = new Map<string, ComplaintAnalysis["claims"][number]["relevant_law"][number] & { claims: string[] }>()
    for (const claim of analysis.claims) {
      for (const law of claim.relevant_law) {
        const existing = rows.get(law.canonical_id)
        if (existing) {
          existing.claims.push(claim.count_label)
        } else {
          rows.set(law.canonical_id, { ...law, claims: [claim.count_label] })
        }
      }
    }
    return [...rows.values()]
  }, [analysis.claims])

  const responseCounts = useMemo(() => {
    const counts = Object.fromEntries(RESPONSE_ORDER.map((type) => [type, 0])) as Record<ResponseType, number>
    for (const allegation of analysis.allegations) counts[allegation.suggested_response] += 1
    return counts
  }, [analysis.allegations])

  const qcItems = useMemo(
    () =>
      analysis.allegations.filter(
        (allegation) =>
          allegation.suggested_response === "needs_review" ||
          allegation.suggested_response === "lack_knowledge" ||
          allegation.evidence_needed.length > 0,
      ),
    [analysis.allegations],
  )

  const wordCount = draftText.trim().split(/\s+/).filter(Boolean).length
  const primaryDeadline = analysis.deadlines[0]
  const plaintiffNames = analysis.parties.filter((party) => party.role === "plaintiff").map((party) => party.name)
  const defendantNames = analysis.parties.filter((party) => party.role === "defendant").map((party) => party.name)

  function saveDraft() {
    setLastSavedAt(
      new Intl.DateTimeFormat(undefined, {
        hour: "numeric",
        minute: "2-digit",
      }).format(new Date()),
    )
  }

  function exportDraft() {
    const blob = new Blob([draftText], { type: "text/plain;charset=utf-8" })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement("a")
    anchor.href = url
    anchor.download = `${analysis.case_number.replace(/[^a-z0-9]+/gi, "-").replace(/^-|-$/g, "") || "draft"}-answer.txt`
    document.body.append(anchor)
    anchor.click()
    anchor.remove()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <header className="border-b border-border bg-card px-4 py-3 sm:px-6">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-end xl:justify-between">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <FileText className="h-3 w-3 text-primary" />
              Draft studio
              <span className="text-border">/</span>
              <span>{analysis.case_number}</span>
            </div>
            <h1 className="mt-1 truncate text-lg font-semibold text-foreground">
              Answer, affirmative defenses, and jury demand
            </h1>
            <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
              <span>{analysis.court}</span>
              <span className="text-border">|</span>
              <span>{plaintiffNames[0]} v. {defendantNames[0]}</span>
              {primaryDeadline && (
                <>
                  <span className="text-border">|</span>
                  <span className="font-mono tabular-nums">{primaryDeadline.days_remaining} days left</span>
                </>
              )}
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <HeaderMetric label="words" value={wordCount.toLocaleString()} />
            <HeaderMetric label="paragraphs" value={analysis.allegations.length} />
            <HeaderMetric label="authorities" value={authorities.length} />
            {lastSavedAt && (
              <span className="inline-flex h-8 items-center rounded border border-success/30 bg-success/10 px-2.5 font-mono text-[10px] uppercase tracking-wider text-success">
                saved {lastSavedAt}
              </span>
            )}
            <Link
              href="/complaint"
              className="inline-flex h-8 items-center gap-1.5 rounded border border-border bg-background px-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:border-primary hover:text-primary"
            >
              <ClipboardList className="h-3.5 w-3.5" />
              complaint
            </Link>
            <button
              type="button"
              onClick={exportDraft}
              className="inline-flex h-8 items-center gap-1.5 rounded border border-border bg-background px-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:border-primary hover:text-primary"
            >
              <Download className="h-3.5 w-3.5" />
              export
            </button>
            <button
              type="button"
              onClick={saveDraft}
              className="inline-flex h-8 items-center gap-1.5 rounded bg-primary px-3 font-mono text-[10px] uppercase tracking-wider text-primary-foreground hover:bg-primary/90"
            >
              <Save className="h-3.5 w-3.5" />
              save
            </button>
          </div>
        </div>
      </header>

      <div className="grid min-h-0 flex-1 grid-cols-1 overflow-hidden lg:grid-cols-[17rem_minmax(0,1fr)] xl:grid-cols-[17rem_minmax(0,1fr)_23rem]">
        <aside className="hidden min-h-0 flex-col overflow-hidden border-r border-border bg-sidebar text-sidebar-foreground lg:flex">
          <div className="border-b border-sidebar-border p-3">
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">Outline</div>
            <div className="mt-2 space-y-1">
              <OutlineButton icon={Gavel} label="Caption" active={outlineSection === "caption"} onClick={() => setOutlineSection("caption")} />
              <OutlineButton icon={Scale} label="Responses" count={analysis.allegations.length} active={outlineSection === "responses"} onClick={() => setOutlineSection("responses")} />
              <OutlineButton icon={ShieldCheck} label="Affirmative defenses" count={analysis.defense_candidates.length} active={outlineSection === "defenses"} onClick={() => setOutlineSection("defenses")} />
              <OutlineButton icon={Sparkles} label="Prayer and jury demand" active={outlineSection === "prayer"} onClick={() => setOutlineSection("prayer")} />
              <OutlineButton icon={History} label="Certificate" active={outlineSection === "certificate"} onClick={() => setOutlineSection("certificate")} />
            </div>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto p-3 scrollbar-thin">
            <div className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              Responses
            </div>
            <div className="space-y-1">
              {analysis.allegations.map((allegation) => {
                const active = allegation.allegation_id === selectedAllegation?.allegation_id
                return (
                  <button
                    key={allegation.allegation_id}
                    onClick={() => setSelectedAllegationId(allegation.allegation_id)}
                    className={cn(
                      "w-full rounded border px-2 py-2 text-left text-xs transition-colors",
                      active
                        ? "border-primary bg-primary/10 text-primary"
                        : "border-sidebar-border bg-background/40 text-muted-foreground hover:border-primary/50 hover:text-foreground",
                    )}
                  >
                    <div className="flex items-center justify-between gap-2 font-mono text-[10px] uppercase tracking-wider">
                      <span>Paragraph {allegation.paragraph}</span>
                      <span className={cn("rounded px-1.5 py-0.5", RESPONSE_META[allegation.suggested_response].className)}>
                        {RESPONSE_META[allegation.suggested_response].label}
                      </span>
                    </div>
                    <div className="mt-1 line-clamp-2 leading-snug">{allegation.text}</div>
                  </button>
                )
              })}
            </div>
          </div>
        </aside>

        <main className="min-h-0 overflow-y-auto bg-background scrollbar-thin">
          <div className="mx-auto flex min-h-full max-w-5xl flex-col px-4 py-4 sm:px-6">
            <div className="mb-3 flex flex-col gap-2 rounded border border-border bg-card p-3 md:flex-row md:items-center md:justify-between">
              <div className="min-w-0">
                <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                  <Wand2 className="h-3 w-3 text-primary" />
                  Working draft
                </div>
                <div className="mt-1 truncate text-sm font-medium text-foreground">
                  {defendantNames.join(", ")}
                </div>
              </div>
              <div className="flex flex-wrap gap-1.5">
                {RESPONSE_ORDER.map((type) => (
                  <span
                    key={type}
                    className={cn("rounded px-2 py-1 font-mono text-[10px] uppercase tracking-wider", RESPONSE_META[type].className)}
                  >
                    {RESPONSE_META[type].label}: {responseCounts[type]}
                  </span>
                ))}
              </div>
            </div>

            <textarea
              value={draftText}
              onChange={(event) => setDraftText(event.target.value)}
              spellCheck
              className="min-h-[760px] w-full flex-1 resize-y rounded border border-border bg-card px-5 py-5 font-mono text-[12px] leading-6 text-foreground shadow-sm outline-none focus:border-primary focus:ring-2 focus:ring-primary/15"
              aria-label="Draft answer text"
            />
          </div>
        </main>

        <aside className="hidden min-h-0 flex-col overflow-hidden border-l border-border bg-card xl:flex">
          <div className="border-b border-border p-3">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                <PanelRight className="h-3 w-3" />
                Inspector
              </div>
              <Link href="/fact-check" className="text-[11px] text-primary hover:underline">
                Run checks
              </Link>
            </div>
            <div className="mt-3 grid grid-cols-3 gap-1 rounded border border-border bg-background p-1">
              {RIGHT_TABS.map((tab) => {
                const Icon = tab.icon
                const active = rightTab === tab.id
                return (
                  <button
                    key={tab.id}
                    onClick={() => setRightTab(tab.id)}
                    className={cn(
                      "inline-flex items-center justify-center gap-1 rounded px-2 py-1.5 font-mono text-[10px] uppercase tracking-wider transition-colors",
                      active ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground",
                    )}
                  >
                    <Icon className="h-3 w-3" />
                    {tab.label}
                  </button>
                )
              })}
            </div>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto p-3 scrollbar-thin">
            {rightTab === "support" && selectedAllegation && <SupportPanel analysis={analysis} selectedId={selectedAllegation.allegation_id} />}
            {rightTab === "citations" && <CitationPanel authorities={authorities} claims={analysis.claims} />}
            {rightTab === "qc" && <QcPanel items={qcItems} />}
          </div>
        </aside>
      </div>
    </div>
  )
}

function HeaderMetric({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded border border-border bg-background px-2.5 py-1.5">
      <div className="font-mono text-[9px] uppercase tracking-widest text-muted-foreground">{label}</div>
      <div className="font-mono text-sm tabular-nums text-foreground">{value}</div>
    </div>
  )
}

function OutlineButton({
  icon: Icon,
  label,
  count,
  active,
  onClick,
}: {
  icon: typeof FileText
  label: string
  count?: number
  active?: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full items-center justify-between rounded px-2 py-1.5 text-xs transition-colors",
        active ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-sidebar-accent hover:text-foreground",
      )}
    >
      <span className="flex min-w-0 items-center gap-2">
        <Icon className="h-3.5 w-3.5 shrink-0" />
        <span className="truncate">{label}</span>
      </span>
      {typeof count === "number" && (
        <span className="rounded bg-background px-1.5 py-0.5 font-mono text-[10px] tabular-nums text-muted-foreground">
          {count}
        </span>
      )}
    </button>
  )
}

function SupportPanel({ analysis, selectedId }: { analysis: ComplaintAnalysis; selectedId: string }) {
  const allegation = analysis.allegations.find((item) => item.allegation_id === selectedId) ?? analysis.allegations[0]
  if (!allegation) return null
  const meta = RESPONSE_META[allegation.suggested_response]

  return (
    <div className="space-y-3">
      <section className="rounded border border-border bg-background p-3">
        <div className="flex items-start justify-between gap-2">
          <div>
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              Paragraph {allegation.paragraph}
            </div>
            <p className="mt-2 text-sm leading-relaxed text-foreground">{allegation.text}</p>
          </div>
          <span className={cn("shrink-0 rounded px-2 py-1 font-mono text-[10px] uppercase tracking-wider", meta.className)}>
            {meta.label}
          </span>
        </div>
        <div className="mt-3 border-t border-border pt-3 text-xs text-muted-foreground">{allegation.reason}</div>
      </section>

      <section>
        <PanelTitle icon={ClipboardList} label="Evidence" />
        <div className="mt-2 space-y-2">
          {allegation.evidence_needed.length > 0 ? (
            allegation.evidence_needed.map((item) => (
              <div key={item} className="flex items-start gap-2 rounded border border-border bg-background p-2 text-xs">
                <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-warning" />
                <span>{item}</span>
              </div>
            ))
          ) : (
            <div className="flex items-center gap-2 rounded border border-border bg-background p-2 text-xs text-muted-foreground">
              <CheckCircle2 className="h-3.5 w-3.5 text-success" />
              No extra evidence flagged
            </div>
          )}
        </div>
      </section>

      <section>
        <PanelTitle icon={ShieldCheck} label="Defenses" />
        <div className="mt-2 space-y-2">
          {analysis.defense_candidates.slice(0, 4).map((defense) => (
            <div key={defense.name} className="rounded border border-border bg-background p-2">
              <div className="flex items-start justify-between gap-2">
                <div className="text-xs font-medium text-foreground">{defense.name}</div>
                <span className={cn("rounded px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-wider", RISK_CLASS[defense.viability])}>
                  {defense.viability}
                </span>
              </div>
              <div className="mt-1 font-mono text-[10px] text-muted-foreground">{defense.authority}</div>
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}

function CitationPanel({
  authorities,
  claims,
}: {
  authorities: Array<ComplaintAnalysis["claims"][number]["relevant_law"][number] & { claims: string[] }>
  claims: ComplaintAnalysis["claims"]
}) {
  return (
    <div className="space-y-3">
      <section>
        <PanelTitle icon={BookOpen} label="Authorities" />
        <div className="mt-2 space-y-2">
          {authorities.map((authority) => (
            <Link
              key={authority.canonical_id}
              href={`/statutes/${authority.canonical_id}`}
              className="block rounded border border-border bg-background p-2 hover:border-primary/50"
            >
              <div className="flex items-start justify-between gap-2">
                <div className="font-mono text-xs text-primary">{authority.citation}</div>
                <ArrowRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
              </div>
              <div className="mt-1 text-[11px] leading-snug text-muted-foreground">{authority.reason}</div>
              <div className="mt-2 flex flex-wrap gap-1">
                {authority.claims.map((claim) => (
                  <span key={claim} className="rounded bg-muted px-1.5 py-0.5 font-mono text-[9px] uppercase text-muted-foreground">
                    {claim}
                  </span>
                ))}
              </div>
            </Link>
          ))}
        </div>
      </section>

      <section>
        <PanelTitle icon={Scale} label="Claim map" />
        <div className="mt-2 space-y-2">
          {claims.map((claim) => (
            <div key={claim.claim_id} className="rounded border border-border bg-background p-2">
              <div className="flex items-center justify-between gap-2">
                <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{claim.count_label}</div>
                <span className={cn("rounded px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-wider", RISK_CLASS[claim.risk_level])}>
                  {claim.risk_level}
                </span>
              </div>
              <div className="mt-1 text-xs font-medium text-foreground">{claim.title}</div>
              <div className="mt-2 font-mono text-[10px] text-muted-foreground">
                {claim.required_elements.length} elements, {claim.relevant_law.length} citations
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}

function QcPanel({ items }: { items: ComplaintAnalysis["allegations"] }) {
  return (
    <div className="space-y-3">
      <section className="rounded border border-border bg-background p-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">Open review</div>
            <div className="mt-1 text-2xl font-semibold text-foreground">{items.length}</div>
          </div>
          <ShieldCheck className="h-8 w-8 text-primary" />
        </div>
      </section>

      <section>
        <PanelTitle icon={AlertTriangle} label="Review queue" />
        <div className="mt-2 space-y-2">
          {items.map((item) => (
            <div key={item.allegation_id} className="rounded border border-border bg-background p-2">
              <div className="flex items-start justify-between gap-2">
                <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                  Paragraph {item.paragraph}
                </div>
                <span className={cn("rounded px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-wider", RESPONSE_META[item.suggested_response].className)}>
                  {RESPONSE_META[item.suggested_response].label}
                </span>
              </div>
              <div className="mt-1 line-clamp-2 text-xs text-foreground">{item.text}</div>
              <div className="mt-2 text-[11px] leading-snug text-muted-foreground">{item.reason}</div>
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}

function PanelTitle({ icon: Icon, label }: { icon: typeof FileText; label: string }) {
  return (
    <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
      <Icon className="h-3 w-3" />
      {label}
    </div>
  )
}
