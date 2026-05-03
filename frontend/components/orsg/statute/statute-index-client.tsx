"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { useEffect, useMemo, useState } from "react"
import { BookOpen, ChevronLeft, ChevronRight, Search, X } from "lucide-react"
import type { StatuteIdentity } from "@/lib/types"
import type { DataSource } from "@/lib/data-state"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { StatusBadge } from "@/components/orsg/badges"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { cn } from "@/lib/utils"

interface StatuteIndexClientProps {
  statutes: StatuteIdentity[]
  total: number
  limit: number
  offset: number
  query?: string
  chapter?: string
  status?: string
  dataSource?: DataSource
  dataError?: string
}

const STATUS_OPTIONS = [
  { value: "all", label: "All statuses" },
  { value: "active", label: "Active" },
  { value: "amended", label: "Amended" },
  { value: "repealed", label: "Repealed" },
  { value: "renumbered", label: "Renumbered" },
]

const LIMIT_OPTIONS = [30, 60, 120]

export function StatuteIndexClient({
  statutes,
  total,
  limit,
  offset,
  query = "",
  chapter = "",
  status = "all",
  dataSource = "live",
  dataError,
}: StatuteIndexClientProps) {
  const router = useRouter()
  const [draftQuery, setDraftQuery] = useState(query)
  const [draftChapter, setDraftChapter] = useState(chapter)
  const [draftStatus, setDraftStatus] = useState(status)
  const [draftLimit, setDraftLimit] = useState(String(limit))

  useEffect(() => {
    setDraftQuery(query)
    setDraftChapter(chapter)
    setDraftStatus(status)
    setDraftLimit(String(limit))
  }, [chapter, limit, query, status])

  const pageStart = total === 0 ? 0 : offset + 1
  const pageEnd = Math.min(offset + statutes.length, total)
  const canPageBack = offset > 0
  const canPageForward = offset + limit < total
  const grouped = useMemo(() => {
    return statutes.reduce<Record<string, StatuteIdentity[]>>((acc, statute) => {
      const key = statute.chapter || "unknown"
      acc[key] = acc[key] ?? []
      acc[key].push(statute)
      return acc
    }, {})
  }, [statutes])

  const pushDirectoryUrl = (next: { offset?: number; limit?: number } = {}) => {
    const directHref = exactStatuteHref(draftQuery)
    if (directHref) {
      router.push(directHref)
      return
    }

    router.push(buildHref({
      q: draftQuery,
      chapter: draftChapter,
      status: draftStatus,
      limit: next.limit ?? Number(draftLimit),
      offset: next.offset ?? 0,
    }))
  }

  const clearFilters = () => {
    setDraftQuery("")
    setDraftChapter("")
    setDraftStatus("all")
    setDraftLimit("60")
    router.push("/statutes")
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <DataStateBanner source={dataSource} error={dataError} label="Statute index data" />
      <header className="border-b border-border bg-card px-4 py-5 sm:px-6">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div className="min-w-0">
            <div className="mb-2 flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
              <BookOpen className="h-3.5 w-3.5 text-primary" />
              statute directory
              <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                2025
              </span>
            </div>
            <h1 className="text-2xl font-semibold tracking-normal text-foreground">Oregon Revised Statutes</h1>
            <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
              {pageStart}-{pageEnd} of {total} indexed sections
            </p>
          </div>

          <form
            className="grid gap-2 sm:grid-cols-[minmax(220px,1fr)_120px_150px_92px_auto_auto]"
            onSubmit={(event) => {
              event.preventDefault()
              pushDirectoryUrl()
            }}
          >
            <div className="relative">
              <Search className="pointer-events-none absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
              <Input
                value={draftQuery}
                onChange={(event) => setDraftQuery(event.target.value)}
                placeholder="Open ORS 90.320 or filter title"
                className="pl-8"
                aria-label="Citation, title, or canonical ID"
              />
            </div>
            <Input
              value={draftChapter}
              onChange={(event) => setDraftChapter(event.target.value)}
              placeholder="Chapter"
              className="font-mono"
            />
            <select
              value={draftStatus}
              onChange={(event) => setDraftStatus(event.target.value)}
              className="h-9 rounded border border-input bg-background px-3 text-sm text-foreground focus:border-primary focus:outline-none"
              aria-label="Filter by status"
            >
              {STATUS_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>{option.label}</option>
              ))}
            </select>
            <select
              value={draftLimit}
              onChange={(event) => {
                setDraftLimit(event.target.value)
                router.push(buildHref({
                  q: draftQuery,
                  chapter: draftChapter,
                  status: draftStatus,
                  limit: Number(event.target.value),
                  offset: 0,
                }))
              }}
              className="h-9 rounded border border-input bg-background px-3 text-sm text-foreground focus:border-primary focus:outline-none"
              aria-label="Rows per page"
            >
              {LIMIT_OPTIONS.map((option) => (
                <option key={option} value={option}>{option}</option>
              ))}
            </select>
            <Button type="submit" size="sm" className="h-9 gap-1.5">
              <Search className="h-3.5 w-3.5" />
              Open / filter
            </Button>
            <Button type="button" variant="outline" size="sm" className="h-9 gap-1.5" onClick={clearFilters}>
              <X className="h-3.5 w-3.5" />
              Clear
            </Button>
          </form>
        </div>
      </header>

      <div className="flex-1 overflow-y-auto bg-background scrollbar-thin">
        {statutes.length === 0 ? (
          <div className="flex min-h-[320px] items-center justify-center px-6 text-center text-sm text-muted-foreground">
            No statutes match the current filters.
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-px bg-border md:grid-cols-2 2xl:grid-cols-3">
            {Object.entries(grouped).map(([chapterKey, items]) => (
              <section key={chapterKey} className="bg-card">
                <div className="sticky top-0 z-10 flex items-center justify-between border-b border-border bg-card/95 px-4 py-2 backdrop-blur">
                  <h2 className="font-mono text-xs uppercase tracking-wider text-muted-foreground">
                    Chapter {chapterKey}
                  </h2>
                  <span className="font-mono text-[10px] tabular-nums text-muted-foreground">{items.length}</span>
                </div>
                <ul className="divide-y divide-border">
                  {items.map((statute) => (
                    <li key={statute.canonical_id}>
                      <Link
                        href={`/statutes/${encodeURIComponent(statute.canonical_id)}`}
                        className="group block px-4 py-3 transition-colors hover:bg-muted/60 focus-visible:bg-muted/60 focus-visible:outline-none"
                      >
                        <div className="flex min-w-0 items-start justify-between gap-3">
                          <div className="min-w-0">
                            <div className="flex flex-wrap items-center gap-2">
                              <span className="font-mono text-sm font-medium text-primary">{statute.citation}</span>
                              <StatusBadge status={statute.status} />
                            </div>
                            <p className="mt-1 line-clamp-2 text-sm leading-snug text-foreground">{statute.title}</p>
                          </div>
                          <ChevronRight className="mt-1 h-4 w-4 flex-none text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-primary" />
                        </div>
                      </Link>
                    </li>
                  ))}
                </ul>
              </section>
            ))}
            <DirectoryGuidePanel />
          </div>
        )}
      </div>

      <footer className="flex flex-col gap-2 border-t border-border bg-card px-4 py-3 sm:flex-row sm:items-center sm:justify-between sm:px-6">
        <div className="font-mono text-[11px] uppercase tracking-wide text-muted-foreground">
          Showing <span className="text-foreground">{pageStart}-{pageEnd}</span> of <span className="text-foreground">{total}</span>
        </div>
        <div className="flex items-center gap-2">
          <Button asChild variant="outline" size="sm" className={cn("h-8 gap-1.5", !canPageBack && "pointer-events-none opacity-50")}>
            <Link href={buildHref({ q: query, chapter, status, limit, offset: Math.max(0, offset - limit) })} aria-disabled={!canPageBack}>
              <ChevronLeft className="h-3.5 w-3.5" />
              Previous
            </Link>
          </Button>
          <Button asChild variant="outline" size="sm" className={cn("h-8 gap-1.5", !canPageForward && "pointer-events-none opacity-50")}>
            <Link href={buildHref({ q: query, chapter, status, limit, offset: offset + limit })} aria-disabled={!canPageForward}>
              Next
              <ChevronRight className="h-3.5 w-3.5" />
            </Link>
          </Button>
        </div>
      </footer>
    </div>
  )
}

function DirectoryGuidePanel() {
  return (
    <section className="hidden min-h-[320px] bg-card px-5 py-5 text-sm text-muted-foreground md:block">
      <div className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        reader opens here
      </div>
      <h2 className="text-base font-semibold text-foreground">Select a statute to open the research view.</h2>
      <p className="mt-2 leading-6">
        Exact citations jump straight to the statute. Directory filters keep the URL as the source of truth, so back,
        forward, and shared links preserve the same result set.
      </p>
      <div className="mt-4 rounded border border-border bg-background/60 p-3 font-mono text-[11px] text-muted-foreground">
        Try ORS 90.320, a title phrase, or a chapter number.
      </div>
    </section>
  )
}

function buildHref(input: { q?: string; chapter?: string; status?: string; limit?: number; offset?: number }) {
  const params = new URLSearchParams()
  if (input.q?.trim()) params.set("q", input.q.trim())
  if (input.chapter?.trim()) params.set("chapter", input.chapter.trim())
  if (input.status && input.status !== "all") params.set("status", input.status)
  if (input.limit && input.limit !== 60) params.set("limit", String(input.limit))
  if (input.offset && input.offset > 0) params.set("offset", String(input.offset))
  const query = params.toString()
  return query ? `/statutes?${query}` : "/statutes"
}

function exactStatuteHref(value: string) {
  const trimmed = value.trim()
  if (!trimmed) return null
  if (/^or:ors:\d{1,3}[A-Z]?\.\d{3}(?:\([A-Za-z0-9]+\))*$/i.test(trimmed)) {
    return `/statutes/${encodeURIComponent(trimmed.replace(/^or:ors:/i, "or:ors:"))}`
  }
  const match = trimmed.match(/^(?:ORS\s*)?(\d{1,3}[A-Z]?\.\d{3}(?:\([A-Za-z0-9]+\))*)$/i)
  if (!match) return null
  return `/statutes/${encodeURIComponent(`or:ors:${normalizeSection(match[1])}`)}`
}

function normalizeSection(section: string) {
  return section.replace(/^(\d{1,3})([a-z])?\./i, (_match, chapter: string, suffix?: string) => {
    return `${chapter}${suffix ? suffix.toUpperCase() : ""}.`
  })
}
