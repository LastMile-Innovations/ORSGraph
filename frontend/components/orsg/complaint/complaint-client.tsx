"use client"

import { useState, useMemo } from "react"
import Link from "next/link"
import type { ComplaintAnalysis, ComplaintParty, ResponseType, RiskLevel } from "@/lib/types"
import { createMatter, extractDocument, importDocumentComplaint, uploadTextFile } from "@/lib/casebuilder/api"
import type { ComplaintDraft } from "@/lib/casebuilder/types"
import { cn } from "@/lib/utils"
import {
  AlertTriangle,
  ArrowRight,
  Briefcase,
  Calendar,
  CheckCircle2,
  ChevronRight,
  ClipboardList,
  FileText,
  Gavel,
  Scale,
  Shield,
  Upload,
  Users,
} from "lucide-react"

const STEPS = [
  { id: "upload", label: "Upload", icon: Upload },
  { id: "extract", label: "Parties & Claims", icon: Users },
  { id: "respond", label: "Admit / Deny", icon: ClipboardList },
  { id: "deadlines", label: "Deadlines", icon: Calendar },
  { id: "options", label: "Response Options", icon: Shield },
  { id: "draft", label: "Draft Answer", icon: FileText },
] as const

type StepId = (typeof STEPS)[number]["id"]

const RISK_CLS: Record<RiskLevel, string> = {
  high: "bg-destructive/15 text-destructive",
  medium: "bg-warning/15 text-warning",
  low: "bg-success/15 text-success",
}

const RESPONSE_META: Record<ResponseType, { label: string; cls: string }> = {
  admit: { label: "Admit", cls: "bg-success/15 text-success" },
  deny: { label: "Deny", cls: "bg-destructive/15 text-destructive" },
  deny_in_part: { label: "Deny in part", cls: "bg-warning/15 text-warning" },
  lack_knowledge: { label: "Lack knowledge", cls: "bg-muted text-muted-foreground" },
  legal_conclusion: { label: "Legal conclusion", cls: "bg-accent/20 text-accent-foreground" },
  needs_review: { label: "Needs review", cls: "bg-primary/15 text-primary" },
}

export function ComplaintWorkflowClient() {
  const [title, setTitle] = useState("Complaint analysis")
  const [text, setText] = useState("")
  const [pending, setPending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [analysis, setAnalysis] = useState<ComplaintAnalysis | null>(null)

  async function analyze(event: React.FormEvent) {
    event.preventDefault()
    if (!text.trim()) return
    setPending(true)
    setError(null)
    try {
      const matter = await createMatter({
        name: title || "Complaint analysis",
        matter_type: "complaint_analysis",
        user_role: "defendant",
        jurisdiction: "Oregon",
      })
      if (!matter.data) throw new Error(matter.error || "Matter could not be created.")
      const document = await uploadTextFile(matter.data.id, {
        filename: `${title || "complaint"}.txt`,
        text,
        mime_type: "text/plain",
        document_type: "complaint",
        folder: "pleadings",
      })
      if (!document.data) throw new Error(document.error || "Complaint could not be uploaded.")
      await extractDocument(matter.data.id, document.data.id)
      const imported = await importDocumentComplaint(matter.data.id, document.data.id, { title, force: true })
      const complaint = imported.data?.imported[0]?.complaint
      if (!complaint) throw new Error(imported.error || imported.data?.warnings[0] || "Complaint import produced no complaint draft.")
      setAnalysis(analysisFromComplaint(complaint, title, text))
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Complaint analysis failed.")
    } finally {
      setPending(false)
    }
  }

  if (analysis) return <ComplaintClient analysis={analysis} />

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-1 flex-col gap-4 p-6">
      <div>
        <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          <Briefcase className="h-3 w-3" />
          complaint analyzer / live
        </div>
        <h1 className="mt-1 text-xl font-semibold">Complaint Analyzer</h1>
      </div>
      <form onSubmit={analyze} className="space-y-3">
        <input
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          className="h-9 w-full rounded border border-border bg-card px-3 text-sm"
          placeholder="Matter or complaint title"
        />
        <textarea
          value={text}
          onChange={(event) => setText(event.target.value)}
          className="min-h-[420px] w-full resize-y rounded border border-border bg-card px-3 py-2 text-sm leading-6"
          placeholder="Paste complaint text..."
        />
        {error && <div className="rounded border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">{error}</div>}
        <button
          type="submit"
          disabled={pending || !text.trim()}
          className="rounded bg-primary px-4 py-2 font-mono text-xs uppercase tracking-wider text-primary-foreground disabled:opacity-50"
        >
          {pending ? "Analyzing" : "Analyze complaint"}
        </button>
      </form>
    </div>
  )
}

export function ComplaintClient({ analysis }: { analysis: ComplaintAnalysis }) {
  const [step, setStep] = useState<StepId>("extract")

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {/* Header */}
      <div className="border-b border-border bg-card px-6 py-4">
        <div className="flex flex-col items-start justify-between gap-3 lg:flex-row lg:items-end">
          <div className="min-w-0">
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <Briefcase className="h-3 w-3" />
              complaint analyzer
            </div>
            <h1 className="mt-1 line-clamp-1 text-base font-semibold leading-tight">
              {analysis.case_number} — {analysis.parties[0].name} v. {analysis.parties[2].name}
            </h1>
            <div className="mt-1 flex flex-wrap items-center gap-3 font-mono text-[10px] tabular-nums text-muted-foreground">
              <span>{analysis.court}</span>
              <span className="text-border">|</span>
              <span>served {analysis.service_date}</span>
              <span className="text-border">|</span>
              <span>role: {analysis.user_role}</span>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Stat label="claims" value={analysis.claims.length} />
            <Stat
              label="critical deadlines"
              value={analysis.deadlines.filter((d) => d.severity === "critical").length}
              tone="fail"
            />
            <Stat label="defenses" value={analysis.defense_candidates.length} tone="success" />
          </div>
        </div>
      </div>

      {/* Stepper */}
      <div className="border-b border-border bg-background px-6 py-3 overflow-x-auto">
        <div className="flex items-center gap-1 min-w-max">
          {STEPS.map((s, i) => {
            const Icon = s.icon
            const active = step === s.id
            return (
              <div key={s.id} className="flex items-center gap-1">
                <button
                  onClick={() => setStep(s.id)}
                  className={cn(
                    "flex items-center gap-2 rounded px-3 py-1.5 font-mono text-[11px] uppercase tracking-wider transition-colors",
                    active
                      ? "bg-primary/10 text-primary"
                      : "text-muted-foreground hover:bg-muted hover:text-foreground",
                  )}
                >
                  <span
                    className={cn(
                      "flex h-5 w-5 items-center justify-center rounded-full font-mono text-[10px] tabular-nums",
                      active ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground",
                    )}
                  >
                    {i + 1}
                  </span>
                  <Icon className="h-3.5 w-3.5" />
                  <span>{s.label}</span>
                </button>
                {i < STEPS.length - 1 && <ChevronRight className="h-3 w-3 text-border" />}
              </div>
            )
          })}
        </div>
      </div>

      {/* Step content */}
      <div className="flex-1 overflow-y-auto bg-background">
        {step === "upload" && <UploadStep analysis={analysis} onNext={() => setStep("extract")} />}
        {step === "extract" && <ExtractStep analysis={analysis} onNext={() => setStep("respond")} />}
        {step === "respond" && <RespondStep analysis={analysis} onNext={() => setStep("deadlines")} />}
        {step === "deadlines" && <DeadlinesStep analysis={analysis} onNext={() => setStep("options")} />}
        {step === "options" && <OptionsStep analysis={analysis} onNext={() => setStep("draft")} />}
        {step === "draft" && <DraftStep analysis={analysis} />}
      </div>
    </div>
  )
}

function analysisFromComplaint(complaint: ComplaintDraft, title: string, sourceText: string): ComplaintAnalysis {
  const parties: ComplaintParty[] = complaint.parties.map((party) => ({
    party_id: party.party_id,
    name: party.name,
    role: party.role === "defendant" || party.role === "plaintiff" ? party.role : "third_party",
    type: party.party_type === "entity" || party.party_type === "government" ? party.party_type : "individual",
  }))
  const claims = complaint.counts.map((count, index) => ({
    claim_id: count.count_id,
    count_label: `Count ${index + 1}`,
    title: count.title,
    cause_of_action: count.legal_theory,
    required_elements: count.element_ids.map((elementId) => ({
      element_id: elementId,
      text: elementId,
      alleged: true,
      proven: false,
      authority: count.authorities[0]?.citation ?? "review required",
    })),
    alleged_facts: count.fact_ids,
    missing_facts: count.weaknesses,
    potential_defenses: count.weaknesses.map((weakness) => ({ name: weakness, authority: "case review", viability: "medium" as RiskLevel })),
    relevant_law: count.authorities.map((authority) => ({
      citation: authority.citation,
      canonical_id: authority.canonical_id,
      reason: authority.reason ?? "Imported complaint authority",
    })),
    risk_level: (count.health === "blocked" ? "high" : count.health === "needs_review" ? "medium" : "low") as RiskLevel,
  }))
  return {
    complaint_id: complaint.complaint_id,
    filename: `${title || complaint.title}.txt`,
    uploaded_at: complaint.created_at || new Date().toISOString(),
    court: complaint.caption.court_name || "Unassigned court",
    case_number: complaint.caption.case_number ?? "Unassigned",
    user_role: "defendant",
    service_date: new Date().toISOString().slice(0, 10),
    summary: complaint.paragraphs.slice(0, 3).map((paragraph) => paragraph.text).join(" "),
    parties,
    claims,
    allegations: complaint.paragraphs.map((paragraph) => ({
      allegation_id: paragraph.paragraph_id,
      paragraph: paragraph.number,
      text: paragraph.text,
      suggested_response: paragraph.review_status === "supported" ? "admit" : "needs_review",
      reason: paragraph.review_status,
      evidence_needed: paragraph.fact_ids,
    })),
    deadlines: [],
    defense_candidates: claims.flatMap((claim) => claim.potential_defenses.map((defense) => ({
      name: defense.name,
      authority: defense.authority,
      rationale: "Imported complaint weakness.",
      viability: defense.viability,
    }))),
    motion_candidates: [],
    counterclaim_candidates: [],
    evidence_checklist: complaint.paragraphs.slice(0, 8).map((paragraph) => ({
      item: `Evidence for paragraph ${paragraph.number}`,
      obtained: paragraph.evidence_uses.length > 0,
      needed_for: paragraph.text.slice(0, 80),
    })),
    draft_answer_preview: sourceText.slice(0, 1200),
  }
}

// ===== Steps =====

function UploadStep({ analysis, onNext }: { analysis: ComplaintAnalysis; onNext: () => void }) {
  const [selectedFile, setSelectedFile] = useState<File | null>(null)

  return (
    <div className="mx-auto max-w-3xl space-y-4 p-6">
      <Section title="upload complaint">
        <div className="rounded border-2 border-dashed border-border bg-card p-10 text-center">
          <Upload className="mx-auto h-8 w-8 text-muted-foreground" />
          <h3 className="mt-3 text-sm font-medium">Drop complaint PDF or paste text</h3>
          <p className="mt-1 text-xs text-muted-foreground">
            ORSGraph extracts parties, claims, allegations, deadlines, and runs each citation through the graph.
          </p>
          <label className="mt-4 inline-flex cursor-pointer rounded bg-primary px-4 py-1.5 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90">
            select file
            <input
              type="file"
              className="sr-only"
              accept=".pdf,.doc,.docx,.txt"
              onChange={(event) => setSelectedFile(event.target.files?.[0] ?? null)}
            />
          </label>
          {selectedFile && (
            <div className="mx-auto mt-3 max-w-md rounded border border-border bg-background px-3 py-2 text-left font-mono text-xs">
              <div className="truncate text-foreground">{selectedFile.name}</div>
              <div className="mt-0.5 text-[10px] tabular-nums text-muted-foreground">
                {(selectedFile.size / 1024).toFixed(1)} KB selected
              </div>
            </div>
          )}
        </div>

        <div className="mt-4 rounded border border-border bg-card p-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <FileText className="h-4 w-4 text-primary" />
                <span className="font-mono text-sm">{analysis.filename}</span>
                <span className="rounded bg-success/15 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-success">
                  parsed
                </span>
              </div>
              <div className="mt-1 font-mono text-[10px] tabular-nums text-muted-foreground">
                uploaded {new Date(analysis.uploaded_at).toLocaleString()} · {analysis.allegations.length}{" "}
                allegations · {analysis.claims.length} claims
              </div>
            </div>
            <button
              onClick={onNext}
              className="flex items-center gap-1 rounded border border-border px-3 py-1.5 font-mono text-xs uppercase tracking-wider hover:border-primary hover:text-primary"
            >
              continue
              <ArrowRight className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      </Section>
    </div>
  )
}

function ExtractStep({ analysis, onNext }: { analysis: ComplaintAnalysis; onNext: () => void }) {
  const [activeClaim, setActiveClaim] = useState(analysis.claims[0].claim_id)
  const claim = analysis.claims.find((c) => c.claim_id === activeClaim)!

  return (
    <div className="grid grid-cols-1 gap-0 lg:grid-cols-[280px_1fr]">
      <aside className="border-b border-border lg:border-b-0 lg:border-r">
        <div className="p-4">
          <Section title="parties">
            <div className="space-y-1">
              {analysis.parties.map((p) => (
                <div key={p.party_id} className="flex items-start gap-2 rounded p-2 hover:bg-muted">
                  <div
                    className={cn(
                      "mt-1 h-1.5 w-1.5 flex-shrink-0 rounded-full",
                      p.role === "plaintiff" ? "bg-chart-1" : "bg-chart-3",
                    )}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="line-clamp-1 text-xs text-foreground">{p.name}</div>
                    <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                      {p.role} · {p.type}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </Section>

          <Section title="claims">
            <div className="space-y-1">
              {analysis.claims.map((c) => (
                <button
                  key={c.claim_id}
                  onClick={() => setActiveClaim(c.claim_id)}
                  className={cn(
                    "flex w-full flex-col items-start rounded border p-2 text-left transition-colors",
                    c.claim_id === activeClaim
                      ? "border-primary bg-primary/5"
                      : "border-border hover:border-primary/40",
                  )}
                >
                  <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                    {c.count_label}
                    <span className={cn("rounded px-1 py-0 text-[9px]", RISK_CLS[c.risk_level])}>
                      {c.risk_level}
                    </span>
                  </div>
                  <div className="mt-0.5 line-clamp-2 text-xs text-foreground">{c.title}</div>
                </button>
              ))}
            </div>
          </Section>
        </div>
      </aside>

      <div className="p-6">
        <div className="mb-4 rounded border border-border bg-card p-4">
          <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">summary</div>
          <p className="mt-1 text-sm leading-relaxed text-foreground">{analysis.summary}</p>
        </div>

        <ClaimDetail claim={claim} />

        <div className="mt-4 flex justify-end">
          <button
            onClick={onNext}
            className="flex items-center gap-1 rounded bg-primary px-4 py-1.5 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90"
          >
            continue to admit/deny
            <ArrowRight className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
    </div>
  )
}

function ClaimDetail({ claim }: { claim: ComplaintAnalysis["claims"][number] }) {
  return (
    <div className="space-y-4">
      <div className="rounded border border-border bg-card p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              {claim.count_label}
            </div>
            <h2 className="mt-1 text-base font-semibold">{claim.title}</h2>
          </div>
          <span
            className={cn(
              "rounded px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider",
              RISK_CLS[claim.risk_level],
            )}
          >
            risk: {claim.risk_level}
          </span>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card title="required elements" icon={Scale}>
          <div className="space-y-2">
            {claim.required_elements.map((el) => (
              <div key={el.element_id} className="flex items-start gap-2 rounded border border-border bg-background p-2">
                <div
                  className={cn(
                    "mt-1 h-2 w-2 flex-shrink-0 rounded-full",
                    el.proven ? "bg-success" : el.alleged ? "bg-warning" : "bg-destructive",
                  )}
                />
                <div className="min-w-0 flex-1">
                  <div className="text-xs text-foreground">{el.text}</div>
                  <div className="mt-0.5 font-mono text-[10px] tabular-nums text-muted-foreground">
                    {el.authority}
                  </div>
                </div>
                <span className="font-mono text-[9px] uppercase tracking-wider text-muted-foreground">
                  {el.proven ? "proven" : el.alleged ? "alleged" : "missing"}
                </span>
              </div>
            ))}
          </div>
        </Card>

        <Card title="potential defenses" icon={Shield}>
          <div className="space-y-2">
            {claim.potential_defenses.map((d, i) => (
              <div key={i} className="rounded border border-border bg-background p-2">
                <div className="flex items-start justify-between gap-2">
                  <div className="text-xs text-foreground">{d.name}</div>
                  <span
                    className={cn(
                      "flex-shrink-0 rounded px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-wider",
                      RISK_CLS[d.viability],
                    )}
                  >
                    {d.viability}
                  </span>
                </div>
                <div className="mt-0.5 font-mono text-[10px] text-muted-foreground">{d.authority}</div>
              </div>
            ))}
          </div>
        </Card>

        <Card title="alleged facts" icon={CheckCircle2}>
          <ul className="space-y-1 pl-1">
            {claim.alleged_facts.map((f, i) => (
              <li key={i} className="flex gap-2 text-xs text-foreground">
                <span className="text-success">+</span>
                {f}
              </li>
            ))}
          </ul>
        </Card>

        <Card title="missing facts" icon={AlertTriangle} tone="warning">
          <ul className="space-y-1 pl-1">
            {claim.missing_facts.map((f, i) => (
              <li key={i} className="flex gap-2 text-xs text-foreground">
                <span className="text-warning">?</span>
                {f}
              </li>
            ))}
          </ul>
        </Card>

        <Card title="relevant law" icon={Gavel} className="lg:col-span-2">
          <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
            {claim.relevant_law.map((l, i) => (
              <Link
                key={i}
                href={`/statutes/${l.canonical_id}`}
                className="rounded border border-border bg-background p-2 hover:border-primary/40"
              >
                <div className="font-mono text-xs text-primary">{l.citation}</div>
                <div className="mt-0.5 text-[11px] text-muted-foreground">{l.reason}</div>
              </Link>
            ))}
          </div>
        </Card>
      </div>
    </div>
  )
}

function RespondStep({ analysis, onNext }: { analysis: ComplaintAnalysis; onNext: () => void }) {
  const [responses, setResponses] = useState<Record<string, ResponseType>>(
    Object.fromEntries(analysis.allegations.map((a) => [a.allegation_id, a.suggested_response])),
  )

  const counts = useMemo(() => {
    const c: Record<ResponseType, number> = {
      admit: 0,
      deny: 0,
      deny_in_part: 0,
      lack_knowledge: 0,
      legal_conclusion: 0,
      needs_review: 0,
    }
    for (const v of Object.values(responses)) c[v]++
    return c
  }, [responses])

  return (
    <div className="p-6">
      <div className="mb-4 flex flex-wrap items-center gap-2">
        {(Object.keys(RESPONSE_META) as ResponseType[]).map((k) => (
          <div
            key={k}
            className={cn(
              "flex items-center gap-1.5 rounded border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-wider",
            )}
          >
            <span className={cn("rounded px-1.5 py-0.5", RESPONSE_META[k].cls)}>{RESPONSE_META[k].label}</span>
            <span className="tabular-nums">{counts[k]}</span>
          </div>
        ))}
      </div>

      <div className="overflow-x-auto rounded border border-border bg-card">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-border bg-muted/40 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              <th className="px-3 py-2 text-left w-12">¶</th>
              <th className="px-3 py-2 text-left">allegation</th>
              <th className="px-3 py-2 text-left w-44">suggested</th>
              <th className="px-3 py-2 text-left">reason</th>
              <th className="px-3 py-2 text-left">evidence needed</th>
            </tr>
          </thead>
          <tbody>
            {analysis.allegations.map((a) => (
              <tr key={a.allegation_id} className="border-b border-border align-top">
                <td className="px-3 py-3 font-mono text-[11px] tabular-nums text-muted-foreground">
                  {a.paragraph}
                </td>
                <td className="px-3 py-3">
                  <p className="font-serif leading-snug text-foreground">{a.text}</p>
                </td>
                <td className="px-3 py-3">
                  <select
                    value={responses[a.allegation_id]}
                    onChange={(e) =>
                      setResponses({
                        ...responses,
                        [a.allegation_id]: e.target.value as ResponseType,
                      })
                    }
                    className="w-full rounded border border-border bg-background px-2 py-1 font-mono text-[11px]"
                  >
                    {(Object.keys(RESPONSE_META) as ResponseType[]).map((k) => (
                      <option key={k} value={k}>
                        {RESPONSE_META[k].label}
                      </option>
                    ))}
                  </select>
                </td>
                <td className="px-3 py-3 text-[11px] text-muted-foreground">{a.reason}</td>
                <td className="px-3 py-3">
                  {a.evidence_needed.length > 0 ? (
                    <ul className="space-y-0.5">
                      {a.evidence_needed.map((e, i) => (
                        <li key={i} className="font-mono text-[10px] text-muted-foreground">
                          • {e}
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <span className="font-mono text-[10px] text-muted-foreground">—</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="mt-4 flex justify-end">
        <button
          onClick={onNext}
          className="flex items-center gap-1 rounded bg-primary px-4 py-1.5 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90"
        >
          continue to deadlines
          <ArrowRight className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  )
}

function DeadlinesStep({ analysis, onNext }: { analysis: ComplaintAnalysis; onNext: () => void }) {
  return (
    <div className="mx-auto max-w-4xl p-6">
      <div className="space-y-3">
        {analysis.deadlines.map((d) => {
          const tone =
            d.severity === "critical"
              ? "border-destructive/40 bg-destructive/5"
              : d.severity === "warning"
              ? "border-warning/40 bg-warning/5"
              : "border-border bg-card"
          const dot =
            d.severity === "critical" ? "bg-destructive" : d.severity === "warning" ? "bg-warning" : "bg-muted-foreground"
          return (
            <div key={d.deadline_id} className={cn("rounded border p-4", tone)}>
              <div className="flex items-start justify-between gap-3">
                <div className="flex items-start gap-3">
                  <div className={cn("mt-1.5 h-2 w-2 flex-shrink-0 rounded-full", dot)} />
                  <div>
                    <div className="text-sm font-medium text-foreground">{d.description}</div>
                    <div className="mt-0.5 font-mono text-[11px] tabular-nums text-muted-foreground">
                      due {d.due_date} · {d.source_citation}
                    </div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="font-mono text-2xl tabular-nums text-foreground">{d.days_remaining}</div>
                  <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                    days left
                  </div>
                </div>
              </div>
            </div>
          )
        })}
      </div>

      <div className="mt-4 flex justify-end">
        <button
          onClick={onNext}
          className="flex items-center gap-1 rounded bg-primary px-4 py-1.5 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90"
        >
          continue to response options
          <ArrowRight className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  )
}

function OptionsStep({ analysis, onNext }: { analysis: ComplaintAnalysis; onNext: () => void }) {
  return (
    <div className="grid grid-cols-1 gap-6 p-6 lg:grid-cols-2">
      <Card title="defense candidates" icon={Shield}>
        <div className="space-y-2">
          {analysis.defense_candidates.map((d, i) => (
            <div key={i} className="rounded border border-border bg-background p-3">
              <div className="flex items-start justify-between gap-2">
                <div className="text-sm font-medium text-foreground">{d.name}</div>
                <span
                  className={cn(
                    "flex-shrink-0 rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider",
                    RISK_CLS[d.viability],
                  )}
                >
                  {d.viability}
                </span>
              </div>
              <div className="mt-1 font-mono text-[10px] text-muted-foreground">{d.authority}</div>
              <p className="mt-1 text-[12px] leading-snug text-muted-foreground">{d.rationale}</p>
            </div>
          ))}
        </div>
      </Card>

      <Card title="motion candidates" icon={Gavel}>
        <div className="space-y-2">
          {analysis.motion_candidates.map((m, i) => (
            <div key={i} className="rounded border border-border bg-background p-3">
              <div className="text-sm font-medium text-foreground">{m.name}</div>
              <div className="mt-1 font-mono text-[10px] text-muted-foreground">{m.authority}</div>
              <p className="mt-1 text-[12px] leading-snug text-muted-foreground">{m.basis}</p>
            </div>
          ))}
        </div>
      </Card>

      <Card title="evidence checklist" icon={ClipboardList} className="lg:col-span-2">
        <div className="grid grid-cols-1 gap-1.5 md:grid-cols-2">
          {analysis.evidence_checklist.map((e, i) => (
            <div
              key={i}
              className="flex items-center justify-between gap-3 rounded border border-border bg-background px-3 py-2"
            >
              <div className="flex items-center gap-2">
                <div
                  className={cn(
                    "flex h-4 w-4 flex-shrink-0 items-center justify-center rounded border",
                    e.obtained
                      ? "border-success bg-success/15 text-success"
                      : "border-border text-muted-foreground",
                  )}
                >
                  {e.obtained && <CheckCircle2 className="h-3 w-3" />}
                </div>
                <span className="text-xs text-foreground">{e.item}</span>
              </div>
              <span className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                {e.needed_for}
              </span>
            </div>
          ))}
        </div>
      </Card>

      <div className="lg:col-span-2 flex justify-end">
        <button
          onClick={onNext}
          className="flex items-center gap-1 rounded bg-primary px-4 py-1.5 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90"
        >
          generate draft answer
          <ArrowRight className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  )
}

function DraftStep({ analysis }: { analysis: ComplaintAnalysis }) {
  return (
    <div className="mx-auto max-w-4xl p-6">
      <div className="mb-3 flex items-center justify-between">
        <div>
          <h2 className="text-base font-semibold">Draft Answer Preview</h2>
          <p className="text-xs text-muted-foreground">
            Generated from your admit/deny selections, defense candidates, and applicable authority.
          </p>
        </div>
        <Link
          href="/draft"
          className="flex items-center gap-1 rounded border border-border px-3 py-1.5 font-mono text-xs uppercase tracking-wider hover:border-primary hover:text-primary"
        >
          open in drafting studio
          <ArrowRight className="h-3.5 w-3.5" />
        </Link>
      </div>

      <pre className="overflow-x-auto rounded border border-border bg-card p-6 font-mono text-[12px] leading-relaxed text-foreground whitespace-pre-wrap">
        {analysis.draft_answer_preview}
      </pre>
    </div>
  )
}

// ===== Helpers =====

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="mb-4">
      <h2 className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{title}</h2>
      {children}
    </div>
  )
}

function Card({
  title,
  icon: Icon,
  children,
  tone,
  className,
}: {
  title: string
  icon: typeof Scale
  children: React.ReactNode
  tone?: "warning" | "fail"
  className?: string
}) {
  return (
    <div className={cn("rounded border border-border bg-card", className)}>
      <div className="flex items-center gap-2 border-b border-border px-3 py-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        <Icon className={cn("h-3 w-3", tone === "warning" && "text-warning", tone === "fail" && "text-destructive")} />
        {title}
      </div>
      <div className="p-3">{children}</div>
    </div>
  )
}

function Stat({
  label,
  value,
  tone,
}: {
  label: string
  value: number | string
  tone?: "success" | "warning" | "fail"
}) {
  return (
    <div className="rounded border border-border bg-card px-3 py-1.5">
      <div className="font-mono text-[9px] uppercase tracking-widest text-muted-foreground">{label}</div>
      <div
        className={cn(
          "font-mono text-base tabular-nums",
          tone === "success" && "text-success",
          tone === "warning" && "text-warning",
          tone === "fail" && "text-destructive",
        )}
      >
        {value}
      </div>
    </div>
  )
}
