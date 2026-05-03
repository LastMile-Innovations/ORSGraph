"use client"

import { useState } from "react"
import type { StatutePageResponse } from "@/lib/types"
import Link from "next/link"
import { useRouter } from "next/navigation"
import { authorityBadges, authorityReason } from "@/lib/authority-taxonomy"
import { StatusBadge, QCBadge } from "@/components/orsg/badges"
import { Button } from "@/components/ui/button"
import { attachAuthority, getMatterSummariesState, type LoadState } from "@/lib/casebuilder/api"
import type { MatterSummary } from "@/lib/casebuilder/types"
import { saveSidebarStatute } from "@/lib/api"
import { Star, MessageSquare, FolderPlus, ExternalLink } from "lucide-react"

export function StatuteHeader({
  data,
  inspectorAction,
}: {
  data: StatutePageResponse
  inspectorAction?: React.ReactNode
}) {
  const { identity, current_version, qc, inbound_citations, outbound_citations } = data
  const router = useRouter()
  const counts = data.summary_counts
  const authorityMeta = {
    ...identity,
    authority_family:
      identity.authority_family
      || (identity.corpus === "us:constitution" ? "USCONST" : undefined)
      || (identity.corpus === "or:constitution" ? "ORCONST" : undefined)
      || (identity.corpus === "or:ors" ? "ORS" : undefined),
    primary_law:
      identity.primary_law ?? (identity.corpus === "us:constitution" || identity.corpus === "or:constitution" || identity.corpus === "or:ors"),
  }
  const [statusMessage, setStatusMessage] = useState<string | null>(null)
  const [matterState, setMatterState] = useState<LoadState<MatterSummary[]> | null>(null)
  const [selectedMatter, setSelectedMatter] = useState("")

  async function saveStatute() {
    setStatusMessage(null)
    try {
      await saveSidebarStatute(identity.canonical_id)
      router.refresh()
      setStatusMessage("Saved.")
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : "Save failed.")
    }
  }

  async function loadMatters() {
    if (matterState) return
    const next = await getMatterSummariesState()
    setMatterState(next)
    setSelectedMatter(next.data[0]?.matter_id ?? "")
  }

  async function addToMatter() {
    if (!selectedMatter) return
    const result = await attachAuthority(selectedMatter, {
      target_type: "matter",
      target_id: selectedMatter,
      citation: identity.citation,
      canonical_id: identity.canonical_id,
      reason: `Added from ${identity.citation}`,
    })
    setStatusMessage(result.data?.attached ? "Added to matter." : result.error || "Add to matter failed.")
  }

  return (
    <header className="border-b border-border bg-card">
      <div className="px-6 pt-5 pb-3">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <span>{identity.jurisdiction}</span>
              <span className="text-border">/</span>
              <span>{identity.corpus}</span>
              <span className="text-border">/</span>
              <span>chapter {identity.chapter}</span>
              <span className="text-border">/</span>
              <span>edition {identity.edition}</span>
            </div>
            <div className="mt-1 flex flex-col gap-1 lg:flex-row lg:items-baseline lg:gap-3">
              <h1 className="font-mono text-2xl font-semibold tracking-tight text-foreground">
                {identity.citation}
              </h1>
              <h2 className="text-lg text-foreground lg:text-xl">{identity.title}</h2>
            </div>
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <StatusBadge status={identity.status} />
              <QCBadge status={qc.status} />
              {authorityBadges(authorityMeta).map((badge) => (
                <span
                  key={badge}
                  className="inline-flex items-center rounded border border-primary/20 bg-primary/5 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-primary"
                >
                  {badge}
                </span>
              ))}
              <span className="inline-flex items-center rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                effective {current_version.effective_date}
              </span>
              <span className="inline-flex items-center rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                {authorityReason(authorityMeta)}
              </span>
              <span className="inline-flex items-center rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                {data.versions.length} versions
              </span>
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {inspectorAction}
            <Button variant="outline" size="sm" className="h-8 gap-1.5 bg-transparent" onClick={saveStatute}>
              <Star className="h-3.5 w-3.5" />
              Save
            </Button>
            <Button variant="outline" size="sm" className="h-8 gap-1.5 bg-transparent" onClick={loadMatters}>
              <FolderPlus className="h-3.5 w-3.5" />
              Add to matter
            </Button>
            <Button asChild size="sm" className="h-8 gap-1.5">
              <Link href={`/ask?q=${encodeURIComponent(`Explain ${identity.citation}: ${identity.title}`)}`}>
                <MessageSquare className="h-3.5 w-3.5" />
                Ask about this
              </Link>
            </Button>
          </div>
        </div>
        {(matterState || statusMessage) && (
          <div className="mt-3 flex flex-wrap items-center gap-2 text-xs">
            {matterState && (
              <>
                <select
                  value={selectedMatter}
                  onChange={(event) => setSelectedMatter(event.target.value)}
                  className="h-8 rounded border border-border bg-background px-2 font-mono"
                >
                  {matterState.data.map((matter) => (
                    <option key={matter.matter_id} value={matter.matter_id}>
                      {matter.name}
                    </option>
                  ))}
                </select>
                <Button variant="outline" size="sm" className="h-8 bg-transparent" disabled={!selectedMatter} onClick={addToMatter}>
                  Attach authority
                </Button>
              </>
            )}
            {statusMessage && <span className="text-muted-foreground">{statusMessage}</span>}
          </div>
        )}

        {/* Quick metrics row */}
        <div className="mt-4 flex flex-wrap items-center gap-x-6 gap-y-1.5 border-t border-border pt-3 font-mono text-[11px] tabular-nums">
          <Metric label="provisions" value={counts?.provision_count ?? countProvisions(data)} />
          <Metric label="cites" value={counts?.citation_counts.outbound ?? outbound_citations.length} accent />
          <Metric label="cited by" value={counts?.citation_counts.inbound ?? inbound_citations.length} accent />
          <Metric label="definitions" value={counts?.semantic_counts.definitions ?? data.definitions.length} />
          <Metric label="exceptions" value={counts?.semantic_counts.exceptions ?? data.exceptions.length} />
          <Metric label="deadlines" value={counts?.semantic_counts.deadlines ?? data.deadlines.length} />
          <Metric label="penalties" value={counts?.semantic_counts.penalties ?? data.penalties.length} />
          <Metric label="chunks" value={data.chunks.length} />
          <Metric label="qc" value={`${qc.passed_checks}/${qc.total_checks}`} warn={qc.status === "warning"} fail={qc.status === "fail"} />
          {data.source_documents[0]?.url && (
            <a
              href={data.source_documents[0]?.url}
              target="_blank"
              rel="noreferrer"
              className="ml-auto flex items-center gap-1 text-muted-foreground hover:text-primary"
            >
              <ExternalLink className="h-3 w-3" />
              official source
            </a>
          )}
        </div>
      </div>
    </header>
  )
}

function Metric({
  label,
  value,
  accent,
  warn,
  fail,
}: {
  label: string
  value: string | number
  accent?: boolean
  warn?: boolean
  fail?: boolean
}) {
  const valueCls = fail
    ? "text-destructive"
    : warn
      ? "text-warning"
      : accent
        ? "text-accent"
        : "text-foreground"
  return (
    <div className="flex items-baseline gap-1.5">
      <span className="text-muted-foreground uppercase tracking-wide">{label}</span>
      <span className={`font-semibold ${valueCls}`}>{value}</span>
    </div>
  )
}

function countProvisions(data: StatutePageResponse): number {
  let count = 0
  function walk(p: any) {
    count++
    if (p.children) p.children.forEach(walk)
  }
  data.provisions.forEach(walk)
  return count
}
