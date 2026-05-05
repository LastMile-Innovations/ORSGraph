"use client"

import { useMemo, useRef, useState } from "react"
import { useRouter } from "next/navigation"
import { AlertTriangle, ChevronLeft, ChevronRight, Database, GitBranch, Search, Sparkles, X } from "lucide-react"
import type { SearchResponse, SuggestResult } from "@/lib/types"
import type { DataSource } from "@/lib/data-state"
import { SearchInput } from "./search-input"
import {
  AUTHORITY_FAMILIES,
  AUTHORITY_TIERS,
  DEFAULT_FILTERS,
  JURISDICTIONS,
  RESULT_TYPES,
  SEMANTIC_FILTERS,
  SOURCE_ROLES,
  SearchFilters,
  type SearchFiltersState,
} from "./search-filters"
import { SearchResultCard } from "./search-result-card"
import { SearchEmptyState } from "./search-empty-state"
import { SearchLoadingState } from "./search-loading-state"
import { directOpen, searchWithParamsState } from "@/lib/api"
import { toSafeInternalHref } from "@/lib/navigation-safety"
import { AUTHORITY_LADDER } from "@/lib/authority-taxonomy"
import { DataStateBanner } from "@/components/orsg/data-state-banner"
import { cn } from "@/lib/utils"

const MODES = [
  { id: "auto", label: "Auto" },
  { id: "hybrid", label: "Hybrid" },
  { id: "keyword", label: "Keyword" },
  { id: "semantic", label: "Semantic" },
  { id: "citation", label: "Citation" },
] as const

interface Props {
  initialQuery?: string
  initialMode?: string
  initialType?: string
  initialFilters?: SearchFiltersState
  response?: SearchResponse
  initialDataSource?: DataSource
  initialDataError?: string
}

export function SearchClient({
  initialQuery = "",
  initialMode = "auto",
  initialType = "all",
  initialFilters = DEFAULT_FILTERS,
  response: initialResponse,
  initialDataSource = "live",
  initialDataError,
}: Props) {
  const router = useRouter()
  const [q, setQ] = useState(initialQuery)
  const [mode, setMode] = useState(initialMode)
  const [resultTypeFilter, setResultTypeFilter] = useState(initialType)
  const [filters, setFilters] = useState<SearchFiltersState>(initialFilters)
  const [response, setResponse] = useState<SearchResponse | undefined>(initialResponse)
  const [limit, setLimit] = useState(initialResponse?.limit || 20)
  const [offset, setOffset] = useState(initialResponse?.offset || 0)
  const [isLoading, setIsLoading] = useState(false)
  const [hasSearched, setHasSearched] = useState(!!initialQuery)
  const [dataSource, setDataSource] = useState<DataSource>(initialDataSource)
  const [dataError, setDataError] = useState<string | undefined>(initialDataError)
  const [error, setError] = useState<string | undefined>(
    initialQuery && !initialResponse && initialDataSource !== "live"
      ? initialDataError ?? "Search API unavailable"
      : undefined,
  )
  const searchRequestRef = useRef(0)

  const performSearch = async ({
    query = q,
    searchMode = mode,
    typeFilter = resultTypeFilter,
    nextFilters = filters,
    nextLimit = limit,
    nextOffset = 0,
  }: {
    query?: string
    searchMode?: string
    typeFilter?: string
    nextFilters?: SearchFiltersState
    nextLimit?: number
    nextOffset?: number
  } = {}) => {
    const requestId = ++searchRequestRef.current
    const trimmed = query.trim()
    setOffset(nextOffset)

    if (!trimmed) {
      setResponse(undefined)
      setHasSearched(false)
      setError(undefined)
      setDataSource("live")
      setDataError(undefined)
      setIsLoading(false)
      router.replace("/search", { scroll: false })
      return
    }

    const params = {
      q: trimmed,
      type: typeFilter,
      mode: searchMode,
      limit: nextLimit,
      offset: nextOffset,
      ...normalizeFilters(nextFilters),
    }

    setIsLoading(true)
    setHasSearched(true)
    setError(undefined)
    router.replace(`/search?${toSearchParams(params)}`, { scroll: false })

    try {
      const res = await searchWithParamsState(params)
      if (requestId !== searchRequestRef.current) return
      setDataSource(res.source)
      setDataError(res.error)
      if (!res.data) {
        setError(res.error ?? "Search API unavailable")
        setResponse(undefined)
        return
      }
      setResponse(res.data)
      setMode(searchMode)
      setLimit(res.data.limit || nextLimit)
      setOffset(res.data.offset || nextOffset)
    } catch (searchError) {
      if (requestId !== searchRequestRef.current) return
      console.error("Search failed:", searchError)
      setError(searchError instanceof Error ? searchError.message : "Search failed")
      setResponse(undefined)
    } finally {
      if (requestId === searchRequestRef.current) {
        setIsLoading(false)
      }
    }
  }

  const handleModeChange = (newMode: string) => {
    setMode(newMode)
    if (q) performSearch({ searchMode: newMode, nextOffset: 0 })
  }

  const handleTypeFilterChange = (newType: string) => {
    setResultTypeFilter(newType)
    if (q) performSearch({ typeFilter: newType, nextOffset: 0 })
  }

  const handleFiltersChange = (nextFilters: SearchFiltersState) => {
    setFilters(nextFilters)
    if (q) performSearch({ nextFilters, nextOffset: 0 })
  }

  const handleLimitChange = (nextLimit: number) => {
    setLimit(nextLimit)
    if (q) performSearch({ nextLimit, nextOffset: 0 })
  }

  const submitSearch = async (query = q) => {
    const trimmed = query.trim()
    if (!trimmed) {
      await performSearch({ query: trimmed, nextOffset: 0 })
      return
    }

    if (isDirectOpenCandidate(trimmed, mode)) {
      const requestId = ++searchRequestRef.current
      setIsLoading(true)
      setHasSearched(true)
      setError(undefined)
      try {
        const opened = await directOpen(trimmed)
        if (requestId !== searchRequestRef.current) return
        const href = toSafeInternalHref(opened.href)
        if (opened.matched && href) {
          router.push(href)
          return
        }
      } catch (openError) {
        if (requestId !== searchRequestRef.current) return
        console.info("Direct open failed; falling back to search", openError)
      } finally {
        if (requestId === searchRequestRef.current) {
          setIsLoading(false)
        }
      }
    }

    await performSearch({ query: trimmed, nextOffset: 0 })
  }

  const handleSuggestionSelect = (suggestion: SuggestResult) => {
    setQ(suggestion.label)
    const href = toSafeInternalHref(suggestion.href)
    if (href && !href.startsWith("/search")) {
      router.push(href)
      return
    }
    performSearch({ query: suggestion.label, nextOffset: 0 })
  }

  const results = useMemo(() => response?.results ?? [], [response?.results])
  const counts = useMemo(() => {
    const fromFacets = response?.facets?.kinds || {}
    const fallback = results.reduce<Record<string, number>>((acc, result) => {
      const kind = result.kind ?? result.result_type ?? "result"
      acc[kind] = (acc[kind] || 0) + 1
      acc.all = (acc.all || 0) + 1
      return acc
    }, { all: 0 })
    return {
      ...fallback,
      ...fromFacets,
      all: response?.total ?? fallback.all ?? 0,
    }
  }, [response, results])

  const responseOffset = response?.offset ?? 0
  const responseLimit = response?.limit ?? limit
  const pageStart = response ? responseOffset + 1 : 0
  const pageEnd = response ? Math.min(responseOffset + response.results.length, response.total) : 0
  const canPageBack = !!response && responseOffset > 0
  const canPageForward = !!response && responseOffset + responseLimit < response.total
  const activeFilterChips = useMemo(
    () => buildActiveFilterChips(resultTypeFilter, filters),
    [resultTypeFilter, filters],
  )

  const clearActiveFilter = (filterId: string) => {
    if (filterId === "type") {
      setResultTypeFilter("all")
      if (q) performSearch({ typeFilter: "all", nextOffset: 0 })
      return
    }

    if (!(filterId in DEFAULT_FILTERS)) return
    const key = filterId as keyof SearchFiltersState
    const nextFilters = { ...filters, [key]: DEFAULT_FILTERS[key] } as SearchFiltersState
    setFilters(nextFilters)
    if (q) performSearch({ nextFilters, nextOffset: 0 })
  }

  const clearAllFilters = () => {
    setResultTypeFilter("all")
    setFilters(DEFAULT_FILTERS)
    if (q) performSearch({ typeFilter: "all", nextFilters: DEFAULT_FILTERS, nextOffset: 0 })
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <DataStateBanner source={dataSource} error={dataError} label="Search data" />
      <header className="border-b border-border bg-card px-4 py-4 sm:px-6">
        <div className="mb-4 flex flex-col gap-1">
          <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            <Search className="h-3.5 w-3.5 text-primary" />
            Source search
          </div>
          <div className="flex flex-col gap-1 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <h1 className="text-2xl font-semibold tracking-normal text-foreground">Find the authority, then inspect the trail.</h1>
              <p className="mt-1 max-w-3xl text-sm leading-6 text-muted-foreground">
                Search citations, titles, concepts, semantic nodes, and source-backed records from one filterable surface.
              </p>
            </div>
            {response && (
              <div className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                {pageStart}-{pageEnd} of {response.total.toLocaleString()}
              </div>
            )}
          </div>
        </div>
        <SearchInput
          value={q}
          onChange={setQ}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.defaultPrevented) submitSearch()
          }}
          onSelectSuggestion={handleSuggestionSelect}
          tookMs={response?.analysis?.timings.total_ms}
          totalResults={hasSearched && !isLoading ? response?.total : undefined}
        />

        <div className="mt-3 flex flex-wrap items-center gap-2">
          <div className="flex items-center gap-1 overflow-x-auto scrollbar-none">
            {MODES.map((m) => (
              <button
                key={m.id}
                onClick={() => handleModeChange(m.id)}
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

          <div className="ml-auto flex items-center gap-2">
            <select
              value={resultTypeFilter}
              onChange={(event) => handleTypeFilterChange(event.target.value)}
              className="h-7 rounded border border-border bg-background px-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground lg:hidden"
              aria-label="Candidate type"
            >
              {RESULT_TYPES.map((type) => (
                <option key={type.id} value={type.id}>
                  {type.label}
                </option>
              ))}
            </select>

            <select
              value={limit}
              onChange={(event) => handleLimitChange(Number(event.target.value))}
              className="h-7 rounded border border-border bg-background px-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground"
            >
              <option value={10}>10 results</option>
              <option value={20}>20 results</option>
              <option value={50}>50 results</option>
              <option value={100}>100 results</option>
            </select>
          </div>
        </div>

        {activeFilterChips.length > 0 && (
          <div className="mt-3 flex flex-wrap items-center gap-1.5">
            {activeFilterChips.map((filter) => (
              <button
                key={filter.id}
                onClick={() => clearActiveFilter(filter.id)}
                className="inline-flex h-7 items-center gap-1 rounded border border-border bg-background px-2 font-mono text-[10px] uppercase tracking-wide text-muted-foreground hover:border-primary/40 hover:text-primary"
                title={`Clear ${filter.label}`}
              >
                {filter.label}
                <X className="h-3 w-3" />
              </button>
            ))}
            <button
              onClick={clearAllFilters}
              className="h-7 rounded px-2 font-mono text-[10px] uppercase tracking-wide text-muted-foreground hover:bg-muted hover:text-foreground"
            >
              clear all
            </button>
          </div>
        )}
      </header>

      <div className="flex flex-1 overflow-hidden">
        <SearchFilters
          currentType={resultTypeFilter}
          onTypeChange={handleTypeFilterChange}
          filters={filters}
          onFiltersChange={handleFiltersChange}
          counts={counts}
          statusCounts={response?.facets?.statuses}
          semanticCounts={response?.facets?.semantic_types}
          className={cn(!hasSearched && "opacity-50 pointer-events-none")}
        />

        <div className="flex flex-1 flex-col overflow-hidden bg-background">
          {!hasSearched ? (
            <SearchEmptyState
              onSelectSuggestion={(suggestion) =>
                handleSuggestionSelect({
                  label: suggestion,
                  kind: "query",
                  href: `/search?q=${encodeURIComponent(suggestion)}`,
                  match_type: "none",
                  score: 0,
                })
              }
            />
          ) : isLoading ? (
            <SearchLoadingState />
          ) : error ? (
            <SearchError message={error} />
          ) : results.length > 0 ? (
            <div className="flex-1 overflow-y-auto scrollbar-thin">
              <SearchRunSummary response={response} query={q} pageStart={pageStart} pageEnd={pageEnd} />
              <div className="divide-y divide-border">
                {results.map((result) => (
                  <SearchResultCard key={`${result.kind}:${result.id}`} result={result} />
                ))}
              </div>
              <div className="flex items-center justify-between border-t border-border px-6 py-3">
                <div className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                  {pageStart}-{pageEnd} of {response?.total || 0}
                </div>
                <div className="flex items-center gap-1">
                  <button
                    disabled={!canPageBack}
                    onClick={() => performSearch({ nextOffset: Math.max(0, offset - limit) })}
                    className="rounded border border-border p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
                    title="Previous page"
                  >
                    <ChevronLeft className="h-4 w-4" />
                  </button>
                  <button
                    disabled={!canPageForward}
                    onClick={() => performSearch({ nextOffset: offset + limit })}
                    className="rounded border border-border p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
                    title="Next page"
                  >
                    <ChevronRight className="h-4 w-4" />
                  </button>
                </div>
              </div>
            </div>
          ) : (
            <div className="flex flex-1 flex-col items-center justify-center p-12 text-center">
              <p className="text-muted-foreground">No results found for &quot;{q}&quot;</p>
              <button
                onClick={() => {
                  setHasSearched(false)
                  setResponse(undefined)
                  setDataSource("live")
                  setDataError(undefined)
                  router.replace("/search", { scroll: false })
                }}
                className="mt-4 text-sm text-primary hover:underline"
              >
                Clear search and try suggestions
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

function SearchRunSummary({
  response,
  query,
  pageStart,
  pageEnd,
}: {
  response?: SearchResponse
  query: string
  pageStart: number
  pageEnd: number
}) {
  return (
    <div className="border-b border-border bg-muted/20 px-6 py-2">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
        <span>
          {pageStart}-{pageEnd} of {response?.total || 0} for &quot;{query}&quot;
        </span>
        {response?.analysis?.timings && <span>{response.analysis.timings.total_ms}ms</span>}
        {response?.analysis?.inferred_chapter && <span>chapter {response.analysis.inferred_chapter}</span>}
        {response?.analysis?.applied_filters && response.analysis.applied_filters.length > 0 && (
          <span>filters {response.analysis.applied_filters.join(", ")}</span>
        )}
        {response?.analysis?.residual_text && (
          <span>topic {response.analysis.residual_text}</span>
        )}
      </div>
      {response?.embeddings && !response.embeddings.enabled && (
        <div className="mt-2 flex items-center gap-1.5 text-xs text-warning">
          <AlertTriangle className="h-3.5 w-3.5" />
          <span>Semantic retrieval is not available for this run; results are limited to non-vector retrieval.</span>
        </div>
      )}
      {(response?.analysis?.intent || response?.embeddings || response?.retrieval || response?.rerank?.enabled) && (
        <details className="mt-2 text-[10px] text-muted-foreground">
          <summary className="cursor-pointer font-mono uppercase tracking-wide hover:text-foreground">Advanced retrieval details</summary>
          <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 font-mono uppercase tracking-wide">
            {response?.analysis?.intent && <span>intent {response.analysis.intent}</span>}
            {response?.analysis?.expansion_count ? <span>expanded {response.analysis.expansion_count}</span> : null}
            {response?.embeddings && (
              <span className="inline-flex items-center gap-1">
                <Sparkles className="h-3 w-3" />
                vectors {response.embeddings.enabled ? response.embeddings.model : "off"}
              </span>
            )}
            {response?.retrieval && (
              <span className="inline-flex items-center gap-1">
                <Database className="h-3 w-3" />
                exact {response.retrieval.exact_candidates} · text {response.retrieval.fulltext_candidates} · vector{" "}
                {response.retrieval.vector_candidates}
                {response.retrieval.capped_candidates !== undefined
                  ? ` · candidates ${response.retrieval.capped_candidates}`
                  : ""}
              </span>
            )}
            {response?.retrieval && (
              <span className="inline-flex items-center gap-1">
                <GitBranch className="h-3 w-3" />
                graph {response.retrieval.graph_expanded_candidates} · rerank{" "}
                {response.retrieval.reranked_candidates}
              </span>
            )}
            {response?.rerank?.enabled && <span>rerank {response.rerank.model || "enabled"}</span>}
            <span>{AUTHORITY_LADDER.join(" > ")}</span>
          </div>
        </details>
      )}
      {response?.warnings && response.warnings.length > 0 && (
        <div className="mt-2 flex flex-col gap-1">
          {response.warnings.map((warning) => (
            <div key={warning} className="flex items-center gap-1.5 text-xs text-warning">
              <AlertTriangle className="h-3.5 w-3.5" />
              <span>{warning}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

function SearchError({ message }: { message: string }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center p-12 text-center">
      <AlertTriangle className="mb-3 h-6 w-6 text-warning" />
      <p className="text-sm text-foreground">{message}</p>
    </div>
  )
}

function normalizeFilters(filters: SearchFiltersState) {
  return {
    authority_family: filters.authority_family !== "all" ? filters.authority_family : undefined,
    authority_tier: filters.authority_tier !== "all" ? filters.authority_tier : undefined,
    jurisdiction: filters.jurisdiction !== "all" ? filters.jurisdiction : undefined,
    source_role: filters.source_role !== "all" ? filters.source_role : undefined,
    chapter: filters.chapter || undefined,
    status: filters.status !== "all" ? filters.status : undefined,
    semantic_type: filters.semantic_type !== "all" ? filters.semantic_type : undefined,
    current_only: filters.current_only || undefined,
    source_backed: filters.source_backed || undefined,
    has_citations: filters.has_citations || undefined,
    has_deadlines: filters.has_deadlines || undefined,
    has_penalties: filters.has_penalties || undefined,
    needs_review: filters.needs_review || undefined,
    primary_law: filters.primary_law || undefined,
    official_commentary: filters.official_commentary || undefined,
  }
}

function buildActiveFilterChips(resultType: string, filters: SearchFiltersState) {
  const chips: { id: string; label: string }[] = []
  if (resultType !== "all") {
    const type = RESULT_TYPES.find((item) => item.id === resultType)
    chips.push({ id: "type", label: type?.label ?? resultType })
  }
  if (filters.authority_family !== "all") {
    const authority = AUTHORITY_FAMILIES.find((item) => item.id === filters.authority_family)
    chips.push({ id: "authority_family", label: authority?.label ?? filters.authority_family })
  }
  if (filters.authority_tier !== "all") {
    const tier = AUTHORITY_TIERS.find((item) => item.id === filters.authority_tier)
    chips.push({ id: "authority_tier", label: tier?.label ?? filters.authority_tier })
  }
  if (filters.jurisdiction !== "all") {
    const jurisdiction = JURISDICTIONS.find((item) => item.id === filters.jurisdiction)
    chips.push({ id: "jurisdiction", label: jurisdiction?.label ?? filters.jurisdiction })
  }
  if (filters.source_role !== "all") {
    const sourceRole = SOURCE_ROLES.find((item) => item.id === filters.source_role)
    chips.push({ id: "source_role", label: sourceRole?.label ?? filters.source_role })
  }
  if (filters.chapter) chips.push({ id: "chapter", label: `Chapter ${filters.chapter}` })
  if (filters.status !== "all") chips.push({ id: "status", label: `Status ${filters.status}` })
  if (filters.semantic_type !== "all") {
    const semantic = SEMANTIC_FILTERS.find((item) => item.id === filters.semantic_type)
    chips.push({ id: "semantic_type", label: semantic?.label ?? filters.semantic_type })
  }
  if (filters.current_only) chips.push({ id: "current_only", label: "Current" })
  if (filters.source_backed) chips.push({ id: "source_backed", label: "Source-backed" })
  if (filters.has_citations) chips.push({ id: "has_citations", label: "Citations" })
  if (filters.has_deadlines) chips.push({ id: "has_deadlines", label: "Deadlines" })
  if (filters.has_penalties) chips.push({ id: "has_penalties", label: "Penalties" })
  if (filters.needs_review) chips.push({ id: "needs_review", label: "Needs review" })
  if (filters.primary_law) chips.push({ id: "primary_law", label: "Primary law" })
  if (filters.official_commentary) chips.push({ id: "official_commentary", label: "Official commentary" })
  return chips
}

function isDirectOpenCandidate(query: string, mode: string) {
  if (mode !== "auto" && mode !== "citation") return false
  const trimmed = query.trim()
  if (/\s+(to|through)\s+|[–—-]/i.test(trimmed)) return false
  return /^(?:(?:ORS|UTCR)\s*)?\d{1,3}[A-Z]?\.\d{3}(?:\([A-Za-z0-9]+\))*$/i.test(trimmed)
    || /^U\.?\s*S\.?\s+Const\.?\s+(?:(?:art\.?)\s+(?:[IVXLCDM]+|\d+)(?:,\s*§+\s*\d+)?(?:,\s*cl\.?\s*\d+)?|(?:amend\.?)\s+(?:[IVXLCDM]+|\d+)(?:,\s*§+\s*\d+)?)$/i.test(trimmed)
    || /^(?:Amdt\d+|Art[IVXLCDM]+|Art\d+)[A-Za-z0-9.]+$/i.test(trimmed)
    || /^(?:First|Second|Third|Fourth|Fifth|Sixth|Seventh|Eighth|Ninth|Tenth|Eleventh|Twelfth|Thirteenth|Fourteenth|Fifteenth|Sixteenth|Seventeenth|Eighteenth|Nineteenth|Twentieth|Twenty-First|Twenty-Second|Twenty-Third|Twenty-Fourth|Twenty-Fifth|Twenty-Sixth|Twenty-Seventh)\s+Amendment$/i.test(trimmed)
    || /^Due Process Clause$/i.test(trimmed)
}

function toSearchParams(params: Record<string, string | number | boolean | undefined>) {
  const searchParams = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== "" && value !== false) {
      searchParams.set(key, String(value))
    }
  }
  return searchParams.toString()
}
