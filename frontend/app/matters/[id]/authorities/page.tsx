import Link from "next/link"
import { notFound } from "next/navigation"
import { BookOpen, CheckCircle2, ExternalLink, FileText, Scale, ShieldCheck } from "lucide-react"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { AuthoritySearchPanel } from "@/components/casebuilder/authority-search-panel"
import { getMatterState } from "@/lib/casebuilder/api"
import { matterClaimsHref, matterDraftHref } from "@/lib/casebuilder/routes"
import type { Matter } from "@/lib/casebuilder/types"
import { cn } from "@/lib/utils"

interface PageProps {
  params: Promise<{ id: string }>
}

type AuthorityRow = {
  citation: string
  canonicalId: string
  reason?: string
  claimIds: string[]
  defenseIds: string[]
  draftSections: string[]
}

export default async function AuthoritiesPage({ params }: PageProps) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  const authorities = collectAuthorities(matter)
  const highCoverage = authorities.filter((authority) => authority.claimIds.length + authority.defenseIds.length > 1)

  return (
    <MatterShell matter={matter} activeSection="authorities" dataState={matterState}>
      <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
        <header className="border-b border-border bg-card px-6 py-5">
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div>
              <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                <BookOpen className="h-3 w-3 text-primary" />
                authority layer
              </div>
              <h1 className="mt-1 text-xl font-semibold tracking-tight text-foreground">
                Authorities
              </h1>
              <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
                Statutes, rules, and procedural authorities linked to claims, defenses, and draft sections.
              </p>
            </div>
            <div className="rounded border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
              Linked authorities are from the matter graph; search results come from live ORSGraph.
            </div>
          </div>

          <div className="mt-5 grid grid-cols-2 gap-px overflow-hidden rounded border border-border bg-border md:grid-cols-4">
            <Metric label="authorities" value={authorities.length} />
            <Metric label="claim links" value={authorities.reduce((sum, item) => sum + item.claimIds.length, 0)} />
            <Metric label="defense links" value={authorities.reduce((sum, item) => sum + item.defenseIds.length, 0)} />
            <Metric label="high coverage" value={highCoverage.length} accent="text-success" />
          </div>
        </header>

        <main className="px-6 py-6">
          <div className="grid grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1fr)_320px]">
            <div className="space-y-4">
              <AuthoritySearchPanel matter={matter} />
              <section className="overflow-hidden rounded border border-border bg-card">
                <div className="border-b border-border px-4 py-3">
                  <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                    linked authorities
                  </h2>
                </div>
                <div className="divide-y divide-border">
                  {authorities.map((authority) => (
                    <article key={authority.canonicalId} className="p-4">
                      <div className="flex flex-wrap items-start justify-between gap-3">
                        <div className="min-w-0">
                          <Link
                            href={`/statutes/${authority.canonicalId}`}
                            className="inline-flex items-center gap-1.5 font-mono text-sm font-semibold text-primary hover:underline"
                          >
                            {authority.citation}
                            <ExternalLink className="h-3 w-3" />
                          </Link>
                          {authority.reason && (
                            <p className="mt-1 max-w-3xl text-sm leading-relaxed text-foreground">
                              {authority.reason}
                            </p>
                          )}
                        </div>
                        <span className="rounded bg-success/15 px-2 py-0.5 font-mono text-[10px] uppercase tracking-wide text-success">
                          resolved
                        </span>
                      </div>

                      <div className="mt-3 flex flex-wrap gap-2">
                        {authority.claimIds.map((claimId) => (
                          <LinkPill key={claimId} href={matterClaimsHref(matter.id, claimId)} icon={Scale}>
                            {claimId}
                          </LinkPill>
                        ))}
                        {authority.defenseIds.map((defenseId) => (
                          <LinkPill key={defenseId} href={matterClaimsHref(matter.id, defenseId)} icon={ShieldCheck}>
                            {defenseId}
                          </LinkPill>
                        ))}
                        {authority.draftSections.map((sectionId) => (
                          <LinkPill key={sectionId} href={matterDraftHref(matter.id)} icon={FileText}>
                            {sectionId}
                          </LinkPill>
                        ))}
                      </div>
                    </article>
                  ))}
                </div>
              </section>
            </div>

            <aside className="space-y-4">
              <section className="rounded border border-border bg-card p-4">
                <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                  coverage
                </h2>
                <div className="mt-3 space-y-2">
                  {highCoverage.map((authority) => (
                    <div key={authority.canonicalId} className="rounded border border-border bg-background p-3">
                      <div className="flex items-center gap-2">
                        <CheckCircle2 className="h-3.5 w-3.5 text-success" />
                        <span className="font-mono text-xs text-foreground">{authority.citation}</span>
                      </div>
                      <p className="mt-1 text-xs text-muted-foreground">
                        Used across {authority.claimIds.length + authority.defenseIds.length} legal theories.
                      </p>
                    </div>
                  ))}
                </div>
              </section>

              <section className="rounded border border-border bg-card p-4">
                <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
                  endpoint contract
                </h2>
                <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                  Production data should come from a matter authorities endpoint that returns canonical IDs,
                  currentness, resolved status, linked theories, and pinpoint citations.
                </p>
              </section>
            </aside>
          </div>
        </main>
      </div>
    </MatterShell>
  )
}

function collectAuthorities(matter: Matter): AuthorityRow[] {
  const rows = new Map<string, AuthorityRow>()
  const upsert = (citation: string, canonicalId: string, patch: Partial<AuthorityRow>) => {
    const existing = rows.get(canonicalId) ?? {
      citation,
      canonicalId,
      claimIds: [],
      defenseIds: [],
      draftSections: [],
    }
    rows.set(canonicalId, {
      ...existing,
      reason: existing.reason ?? patch.reason,
      claimIds: [...new Set([...existing.claimIds, ...(patch.claimIds ?? [])])],
      defenseIds: [...new Set([...existing.defenseIds, ...(patch.defenseIds ?? [])])],
      draftSections: [...new Set([...existing.draftSections, ...(patch.draftSections ?? [])])],
    })
  }

  for (const claim of matter.claims) {
    for (const authority of claim.authorities ?? []) {
      upsert(authority.citation, authority.canonical_id, {
        reason: authority.reason,
        claimIds: claim.kind === "defense" ? [] : [claim.id],
        defenseIds: claim.kind === "defense" ? [claim.id] : [],
      })
    }
  }
  for (const draft of matter.drafts) {
    for (const section of draft.sections) {
      for (const citation of section.citations) {
        if (citation.sourceKind === "statute") {
          upsert(citation.shortLabel, citation.sourceId, { draftSections: [section.id] })
        }
      }
    }
  }

  return [...rows.values()].sort((a, b) => a.citation.localeCompare(b.citation))
}

function Metric({ label, value, accent = "text-foreground" }: { label: string; value: number; accent?: string }) {
  return (
    <div className="bg-card px-4 py-3">
      <div className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div className={cn("mt-0.5 font-mono text-lg font-semibold tabular-nums", accent)}>
        {value.toLocaleString()}
      </div>
    </div>
  )
}

function LinkPill({
  href,
  icon: Icon,
  children,
}: {
  href: string
  icon: typeof Scale
  children: React.ReactNode
}) {
  return (
    <Link
      href={href}
      className="inline-flex items-center gap-1 rounded border border-border bg-background px-2 py-1 font-mono text-[10px] text-muted-foreground hover:border-primary/40 hover:text-primary"
    >
      <Icon className="h-3 w-3" />
      {children}
    </Link>
  )
}
