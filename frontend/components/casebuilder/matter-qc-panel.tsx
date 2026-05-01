"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { useMemo, useState } from "react"
import { AlertTriangle, CheckCircle2, ClipboardList, Lightbulb, ShieldCheck } from "lucide-react"
import type { IssueSpotResponse, Matter, QcRun, QcSuggestedTask } from "@/lib/casebuilder/types"
import { createTask, runMatterQc, spotIssues } from "@/lib/casebuilder/api"
import { matterClaimsHref, matterDocumentHref, matterFactsHref, matterWorkProductHref } from "@/lib/casebuilder/routes"
import { cn } from "@/lib/utils"

export function MatterQcPanel({ matter }: { matter: Matter }) {
  const router = useRouter()
  const [qcRun, setQcRun] = useState<QcRun | null>(null)
  const [issues, setIssues] = useState<IssueSpotResponse | null>(null)
  const [pending, setPending] = useState<"qc" | "issues" | "task" | null>(null)
  const [message, setMessage] = useState<string | null>(null)

  const missingElements = matter.claims.flatMap((claim) => claim.elements.filter((element) => element.status === "missing"))
  const openFactFindings = matter.fact_check_findings.filter((finding) => finding.status === "open")
  const openCitationFindings = matter.citation_check_findings.filter((finding) => finding.status === "open")
  const sentenceFindings = (qcRun?.work_product_sentences ?? []).filter((sentence) => sentence.support_status !== "supported" || sentence.finding_ids.length > 0)
  const counts = useMemo(() => {
    const evidenceGaps = qcRun?.evidence_gaps.length ?? missingElements.length
    const authorityGaps = qcRun?.authority_gaps.length ?? matter.claims.filter((claim) => (claim.authorities?.length ?? 0) === 0).length
    const contradictions = qcRun?.contradictions.length ?? matter.evidence.filter((evidence) => evidence.contradicts_fact_ids.length > 0).length
    const findings = (qcRun?.fact_findings.length ?? openFactFindings.length) + (qcRun?.citation_findings.length ?? openCitationFindings.length) + (qcRun?.work_product_findings.length ?? 0) + sentenceFindings.length
    return { evidenceGaps, authorityGaps, contradictions, findings }
  }, [matter, missingElements.length, openCitationFindings.length, openFactFindings.length, qcRun, sentenceFindings.length])

  async function onRunQc() {
    setPending("qc")
    setMessage(null)
    const result = await runMatterQc(matter.id)
    setPending(null)
    if (!result.data) {
      setMessage(result.error || "Matter QC failed.")
      return
    }
    setQcRun(result.data)
    setMessage(`Matter QC complete: ${result.data.evidence_gaps.length} evidence gaps, ${result.data.authority_gaps.length} authority gaps.`)
  }

  async function onSpotIssues() {
    setPending("issues")
    setMessage(null)
    const result = await spotIssues(matter.id, { limit: 12 })
    setPending(null)
    if (!result.data) {
      setMessage(result.error || "Issue spotting failed.")
      return
    }
    setIssues(result.data)
    setMessage(`Issue spotting found ${result.data.suggestions.length} deterministic suggestion(s).`)
  }

  async function onCreateTask(task: QcSuggestedTask) {
    setPending("task")
    const result = await createTask(matter.id, {
      title: task.title,
      description: task.description || undefined,
      status: task.status || "todo",
      priority: task.priority || "med",
      due_date: task.due_date || undefined,
      related_claim_ids: task.related_claim_ids,
      related_document_ids: task.related_document_ids,
      related_deadline_id: task.related_deadline_id || undefined,
      source: task.source || "qc_run",
    })
    setPending(null)
    if (!result.data) {
      setMessage(result.error || "Could not create remediation task.")
      return
    }
    setMessage("Remediation task created.")
    router.refresh()
  }

  return (
    <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
      <header className="border-b border-border bg-card px-6 py-5">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <ShieldCheck className="h-3.5 w-3.5 text-primary" />
              case qc
            </div>
            <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Risk Dashboard</h1>
            <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
              Run deterministic matter-level checks for support gaps, missing authority, contradictions, and work-product findings.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button type="button" onClick={onRunQc} disabled={pending !== null} className="rounded bg-primary px-3 py-2 font-mono text-xs uppercase tracking-wider text-primary-foreground disabled:opacity-60">
              {pending === "qc" ? "running" : "run qc"}
            </button>
            <button type="button" onClick={onSpotIssues} disabled={pending !== null} className="rounded border border-border bg-background px-3 py-2 font-mono text-xs uppercase tracking-wider text-muted-foreground hover:border-primary/40 hover:text-primary disabled:opacity-60">
              {pending === "issues" ? "spotting" : "spot issues"}
            </button>
          </div>
        </div>
        {message && <div className="mt-3 rounded border border-border bg-background px-3 py-2 text-xs text-muted-foreground">{message}</div>}
      </header>

      <main className="space-y-4 px-6 py-6">
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-4">
          <Metric label="evidence gaps" value={counts.evidenceGaps} urgent={counts.evidenceGaps > 0} />
          <Metric label="authority gaps" value={counts.authorityGaps} urgent={counts.authorityGaps > 0} />
          <Metric label="contradictions" value={counts.contradictions} urgent={counts.contradictions > 0} />
          <Metric label="open findings" value={counts.findings} urgent={counts.findings > 0} />
        </div>

        {qcRun && (
          <section className="rounded border border-border bg-card">
            <SectionHeader title="latest qc run" subtitle={`${qcRun.mode} · ${new Date(qcRun.generated_at).toLocaleString()}`} />
            <div className="grid gap-3 p-4 lg:grid-cols-3">
              <FindingList title="Evidence gaps" items={qcRun.evidence_gaps.map((gap) => ({ id: gap.id, title: gap.title, body: gap.message, href: linkForTarget(matter.id, gap.target_type, gap.target_id) }))} />
              <FindingList title="Authority gaps" items={qcRun.authority_gaps.map((gap) => ({ id: gap.id, title: gap.title, body: gap.message, href: linkForTarget(matter.id, gap.target_type, gap.target_id) }))} />
              <FindingList title="Contradictions" items={qcRun.contradictions.map((item) => ({ id: item.id, title: item.title, body: item.message, href: item.fact_ids[0] ? matterFactsHref(matter.id, item.fact_ids[0]) : undefined }))} />
            </div>
            {sentenceFindings.length > 0 && (
              <div className="border-t border-border p-4">
                <h3 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">sentence support</h3>
                <div className="mt-2 grid gap-2 lg:grid-cols-2">
                  {sentenceFindings.slice(0, 12).map((sentence) => (
                    <Link
                      key={sentence.id}
                      href={matterWorkProductHref(matter.id, sentence.work_product_id, "editor")}
                      className="rounded border border-border bg-background p-3 text-xs hover:border-primary/40"
                    >
                      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                        {sentence.support_status} · {sentence.finding_ids.length} finding(s)
                      </div>
                      <p className="mt-1 line-clamp-3 text-foreground">{sentence.text}</p>
                    </Link>
                  ))}
                </div>
              </div>
            )}
            {qcRun.suggested_tasks.length > 0 && (
              <div className="border-t border-border p-4">
                <h3 className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                  <ClipboardList className="h-3.5 w-3.5" />
                  remediation tasks
                </h3>
                <div className="mt-2 grid gap-2 md:grid-cols-2">
                  {qcRun.suggested_tasks.map((task, index) => (
                    <button
                      key={`${task.title}-${index}`}
                      type="button"
                      onClick={() => onCreateTask(task)}
                      disabled={pending !== null}
                      className="rounded border border-border bg-background p-3 text-left text-xs hover:border-primary/40 disabled:opacity-60"
                    >
                      <div className="font-medium text-foreground">{task.title}</div>
                      {task.description && <div className="mt-1 line-clamp-2 text-muted-foreground">{task.description}</div>}
                    </button>
                  ))}
                </div>
              </div>
            )}
          </section>
        )}

        {issues && (
          <section className="rounded border border-border bg-card">
            <SectionHeader title="issue suggestions" subtitle={`${issues.suggestions.length} deterministic suggestion(s)`} />
            <div className="divide-y divide-border">
              {issues.suggestions.map((issue) => (
                <article key={issue.id} className="p-4">
                  <div className="flex items-start gap-3">
                    <Lightbulb className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
                    <div className="min-w-0">
                      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                        {issue.issue_type} · {Math.round(issue.confidence * 100)}%
                      </div>
                      <h3 className="mt-1 text-sm font-semibold text-foreground">{issue.title}</h3>
                      <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{issue.summary}</p>
                      <p className="mt-2 text-xs text-foreground">{issue.recommended_action}</p>
                      <div className="mt-2 flex flex-wrap gap-1.5">
                        {issue.fact_ids.map((factId) => <TargetLink key={factId} href={matterFactsHref(matter.id, factId)} label={factId} />)}
                        {issue.document_ids.map((documentId) => <TargetLink key={documentId} href={matterDocumentHref(matter.id, documentId)} label={documentId} />)}
                      </div>
                    </div>
                  </div>
                </article>
              ))}
            </div>
          </section>
        )}

        <section className="rounded border border-border bg-card">
          <SectionHeader title="persisted draft findings" subtitle={`${openFactFindings.length + openCitationFindings.length} open`} />
          {openFactFindings.length + openCitationFindings.length === 0 ? (
            <div className="p-4 text-sm text-muted-foreground">No persisted support or citation findings are open.</div>
          ) : (
            <div className="divide-y divide-border">
              {openFactFindings.map((finding) => <PersistedFinding key={finding.finding_id} kind={finding.finding_type} message={finding.message} anchor={finding.paragraph_id ?? finding.draft_id} />)}
              {openCitationFindings.map((finding) => <PersistedFinding key={finding.finding_id} kind={finding.finding_type} message={finding.message} anchor={finding.citation || finding.draft_id} />)}
            </div>
          )}
        </section>
      </main>
    </div>
  )
}

function linkForTarget(matterId: string, targetType: string, targetId: string) {
  if (targetType === "fact") return matterFactsHref(matterId, targetId)
  if (targetType === "claim" || targetType === "element") return matterClaimsHref(matterId, targetId)
  if (targetType === "document") return matterDocumentHref(matterId, targetId)
  if (targetType === "work_product") return matterWorkProductHref(matterId, targetId, "editor")
  return undefined
}

function TargetLink({ href, label }: { href: string; label: string }) {
  return <Link href={href} className="rounded border border-border px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground hover:border-primary/40 hover:text-primary">{label}</Link>
}

function SectionHeader({ title, subtitle }: { title: string; subtitle: string }) {
  return (
    <div className="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
      <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">{title}</h2>
      <span className="text-xs text-muted-foreground">{subtitle}</span>
    </div>
  )
}

function FindingList({ title, items }: { title: string; items: Array<{ id: string; title: string; body: string; href?: string }> }) {
  return (
    <div className="rounded border border-border bg-background">
      <div className="border-b border-border px-3 py-2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{title}</div>
      {items.length === 0 ? (
        <div className="p-3 text-xs text-muted-foreground">None found.</div>
      ) : (
        <div className="divide-y divide-border">
          {items.map((item) => (
            <Link key={item.id} href={item.href || "#"} className="block p-3 text-xs hover:bg-muted/40">
              <div className="font-medium text-foreground">{item.title}</div>
              <div className="mt-1 line-clamp-3 text-muted-foreground">{item.body}</div>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}

function PersistedFinding({ kind, message, anchor }: { kind: string; message: string; anchor?: string | null }) {
  return (
    <article className="flex items-start gap-3 p-4">
      <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-warning" />
      <div className="min-w-0">
        <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{kind}</div>
        <p className="mt-1 text-sm text-foreground">{message}</p>
        {anchor && <p className="mt-1 font-mono text-[10px] text-muted-foreground">{anchor}</p>}
      </div>
    </article>
  )
}

function Metric({ label, value, urgent = false }: { label: string; value: number; urgent?: boolean }) {
  const Icon = urgent ? AlertTriangle : CheckCircle2
  return (
    <section className="rounded border border-border bg-card p-4">
      <div className="flex items-center justify-between gap-3">
        <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
        <Icon className={urgent ? "h-4 w-4 text-warning" : "h-4 w-4 text-success"} />
      </div>
      <div className={cn("mt-2 font-mono text-2xl font-semibold tabular-nums", urgent ? "text-warning" : "text-foreground")}>{value}</div>
    </section>
  )
}
