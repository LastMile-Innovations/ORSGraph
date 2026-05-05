"use client"

import { useState } from "react"
import { useRouter } from "next/navigation"
import Link from "next/link"
import { BookOpen, Link2, Search } from "lucide-react"
import type { AuthorityTargetType, CaseAuthoritySearchItem, Matter } from "@/lib/casebuilder/types"
import { attachAuthority, searchAuthority } from "@/lib/casebuilder/api"
import { authorityBadges, authorityReason } from "@/lib/authority-taxonomy"

interface AuthoritySearchPanelProps {
  matter: Matter
}

export function AuthoritySearchPanel({ matter }: AuthoritySearchPanelProps) {
  const router = useRouter()
  const [query, setQuery] = useState("")
  const [results, setResults] = useState<CaseAuthoritySearchItem[]>([])
  const [warnings, setWarnings] = useState<string[]>([])
  const [searched, setSearched] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [targetType, setTargetType] = useState<AuthorityTargetType>("claim")
  const [targetId, setTargetId] = useState("")
  const [attachMessage, setAttachMessage] = useState<string | null>(null)
  const [attachingId, setAttachingId] = useState<string | null>(null)

  const targets = buildAuthorityTargets(matter, targetType)
  const hasTargets = targets.length > 0
  const selectedTarget = targetId || targets[0]?.id || ""

  async function onSearch() {
    if (!query.trim()) {
      setError("Enter a search query.")
      return
    }
    setLoading(true)
    setError(null)
    setWarnings([])
    const response = await searchAuthority(matter.id, query.trim(), 8)
    setLoading(false)
    setSearched(true)
    if (!response.data) {
      setResults([])
      setError(response.error || "Authority search failed.")
      return
    }
    setResults(response.data.results)
    setWarnings(response.data.warnings)
  }

  async function onAttach(result: CaseAuthoritySearchItem) {
    if (!selectedTarget) {
      setError("Choose a claim, element, or draft paragraph target first.")
      return
    }
    const citation = result.citation || result.title || result.id
    const canonicalId = result.canonical_id || result.id
    setAttachingId(result.id)
    setError(null)
    setAttachMessage(null)
    const response = await attachAuthority(matter.id, {
      target_type: targetType,
      target_id: selectedTarget,
      citation,
      canonical_id: canonicalId,
      reason: result.snippet,
    })
    setAttachingId(null)
    if (!response.data) {
      setError(response.error || "Authority could not be attached.")
      return
    }
    setAttachMessage(`Attached ${citation} to ${selectedTarget}.`)
    router.refresh()
  }

  return (
    <section className="overflow-hidden rounded border border-border bg-card">
      <div className="border-b border-border px-4 py-3">
        <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          authority search
        </h2>
        <div className="mt-3 flex gap-2">
          <div className="flex min-w-0 flex-1 items-center gap-2 rounded border border-border bg-background px-2.5">
            <Search className="h-3.5 w-3.5 text-muted-foreground" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void onSearch()
              }}
              placeholder="Search Constitution, CONAN, ORS, rules, definitions..."
              className="min-w-0 flex-1 bg-transparent py-2 text-xs focus:outline-none"
            />
          </div>
          <button
            type="button"
            onClick={onSearch}
            disabled={loading}
            className="rounded bg-primary px-3 py-2 font-mono text-xs uppercase tracking-wider text-primary-foreground hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {loading ? "searching" : "search"}
          </button>
        </div>
        <div className="mt-2 grid gap-2 md:grid-cols-[150px_minmax(0,1fr)]">
          <select
            value={targetType}
            onChange={(event) => {
              const next = event.target.value as AuthorityTargetType
              setTargetType(next)
              setTargetId("")
            }}
            className="rounded border border-border bg-background px-2.5 py-2 font-mono text-[11px]"
          >
            <option value="claim">claim</option>
            <option value="element">element</option>
            <option value="draft_paragraph">draft paragraph</option>
          </select>
          <select
            value={selectedTarget}
            onChange={(event) => setTargetId(event.target.value)}
            className="min-w-0 rounded border border-border bg-background px-2.5 py-2 text-xs"
          >
            {targets.length === 0 ? (
              <option value="">No targets available</option>
            ) : (
              targets.map((target) => (
                <option key={target.id} value={target.id}>
                  {target.label}
                </option>
              ))
            )}
          </select>
        </div>
        {!hasTargets && (
          <div className="mt-2 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
            Create a claim, element, or draft paragraph before attaching authorities. Search can still help you collect candidate law.
          </div>
        )}
        {error && <p className="mt-2 text-xs text-destructive">{error}</p>}
        {attachMessage && <p className="mt-2 text-xs text-muted-foreground">{attachMessage}</p>}
        {warnings.length > 0 && (
          <p className="mt-2 text-xs text-warning">{warnings.join(" ")}</p>
        )}
      </div>

      <div className="divide-y divide-border">
        {results.map((result) => (
          <article key={result.id} className="p-4">
            <div className="flex items-start gap-2">
              <BookOpen className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" />
              <div className="min-w-0 flex-1">
                <Link
                  href={result.href || `/statutes/${encodeURIComponent(result.canonical_id || result.id)}`}
                  className="font-mono text-sm font-semibold text-primary hover:underline"
                >
                  {result.citation || result.title || result.id}
                </Link>
                {result.title && (
                  <p className="mt-0.5 text-xs font-medium text-foreground">{result.title}</p>
                )}
                <div className="mt-1 flex flex-wrap gap-1">
                  {authorityBadges(result).map((badge) => (
                    <span
                      key={badge}
                      className="rounded border border-primary/20 bg-primary/5 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-primary"
                    >
                      {badge}
                    </span>
                  ))}
                </div>
                <p className="mt-1 line-clamp-3 text-xs leading-relaxed text-muted-foreground">
                  {result.snippet}
                </p>
                <p className="mt-2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                  {result.kind} · {authorityReason(result)} · score {result.score.toFixed(2)}
                </p>
                <button
                  type="button"
                  onClick={() => void onAttach(result)}
                  disabled={attachingId === result.id || !selectedTarget}
                  className="mt-3 inline-flex items-center gap-1 rounded border border-border bg-background px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
                >
                  <Link2 className="h-3 w-3" />
                  {attachingId === result.id ? "attaching" : "attach"}
                </button>
              </div>
            </div>
          </article>
        ))}
        {searched && results.length === 0 && !error && (
          <div className="p-4 text-xs text-muted-foreground">No matching authority found.</div>
        )}
      </div>
    </section>
  )
}

function buildAuthorityTargets(matter: Matter, targetType: AuthorityTargetType) {
  if (targetType === "claim") {
    return matter.claims.map((claim) => ({
      id: claim.id,
      label: `${claim.kind}: ${claim.title}`,
    }))
  }
  if (targetType === "element") {
    return matter.claims.flatMap((claim) =>
      claim.elements.map((element) => ({
        id: element.id,
        label: `${claim.title} - ${element.title}`,
      })),
    )
  }
  return matter.drafts.flatMap((draft) =>
    draft.paragraphs.map((paragraph) => ({
      id: paragraph.paragraph_id,
      label: `${draft.title} - ${paragraph.role} #${paragraph.index}`,
    })),
  )
}
