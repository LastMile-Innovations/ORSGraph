import { notFound } from "next/navigation"
import { AlertTriangle, CheckCircle2, ShieldCheck } from "lucide-react"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { getMatterState } from "@/lib/casebuilder/api"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function MatterQcPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  const unsupportedParagraphs = matter.drafts.flatMap((draft) =>
    draft.paragraphs.filter((paragraph) => paragraph.factcheck_status !== "supported"),
  )
  const missingElements = matter.claims.flatMap((claim) =>
    claim.elements.filter((element) => element.status === "missing"),
  )
  const openFactFindings = matter.fact_check_findings.filter((finding) => finding.status === "open")
  const openCitationFindings = matter.citation_check_findings.filter((finding) => finding.status === "open")

  return (
    <MatterShell matter={matter} activeSection="qc" dataState={matterState}>
      <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
        <header className="border-b border-border bg-card px-6 py-5">
          <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <ShieldCheck className="h-3.5 w-3.5 text-primary" />
            case qc
          </div>
          <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Risk Dashboard</h1>
          <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
            V0 support checks for unsupported allegations, missing elements, citation gaps, deadline risks, and contradictions.
          </p>
        </header>

        <main className="space-y-4 px-6 py-6">
          <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-4">
            <Metric label="unsupported draft paragraphs" value={unsupportedParagraphs.length} urgent={unsupportedParagraphs.length > 0} />
            <Metric label="missing elements" value={missingElements.length} urgent={missingElements.length > 0} />
            <Metric label="open findings" value={openFactFindings.length + openCitationFindings.length} urgent={openFactFindings.length + openCitationFindings.length > 0} />
            <Metric label="open tasks" value={matter.tasks.filter((task) => task.status !== "done").length} />
          </div>

          <section className="rounded border border-border bg-card">
            <div className="border-b border-border px-4 py-3">
              <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                persisted findings
              </h2>
            </div>
            {openFactFindings.length + openCitationFindings.length === 0 ? (
              <div className="p-4 text-sm text-muted-foreground">
                No persisted support or citation findings are open.
              </div>
            ) : (
              <div className="divide-y divide-border">
                {openFactFindings.map((finding) => (
                  <FindingRow
                    key={finding.finding_id}
                    kind={finding.finding_type}
                    message={finding.message}
                    anchor={finding.paragraph_id ?? finding.draft_id}
                  />
                ))}
                {openCitationFindings.map((finding) => (
                  <FindingRow
                    key={finding.finding_id}
                    kind={finding.finding_type}
                    message={finding.message}
                    anchor={finding.citation || finding.draft_id}
                  />
                ))}
              </div>
            )}
          </section>
        </main>
      </div>
    </MatterShell>
  )
}

function FindingRow({ kind, message, anchor }: { kind: string; message: string; anchor?: string | null }) {
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
      <div className={urgent ? "mt-2 font-mono text-2xl font-semibold text-warning" : "mt-2 font-mono text-2xl font-semibold text-foreground"}>
        {value}
      </div>
    </section>
  )
}
