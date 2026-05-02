import { Shell } from "@/components/orsg/shell"
import { SearchClient } from "@/components/orsg/search/search-client"
import { getCachedSearchWithParamsState } from "@/lib/authority-server-cache"

type SearchPageParams = {
  q?: string
  type?: string
  mode?: string
  limit?: string
  offset?: string
  authority_family?: string
  authority_tier?: string
  jurisdiction?: string
  source_role?: string
  chapter?: string
  status?: string
  semantic_type?: string
  current_only?: string
  source_backed?: string
  has_citations?: string
  has_deadlines?: string
  has_penalties?: string
  needs_review?: string
  primary_law?: string
  official_commentary?: string
}

function boolParam(value?: string) {
  return value === "true"
}

function numberParam(value: string | undefined, fallback: number) {
  const parsed = Number(value)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback
}

export default async function SearchPage({
  searchParams,
}: {
  searchParams: Promise<SearchPageParams>
}) {
  const params = await searchParams
  const q = params.q || ""
  const initialMode = params.mode || "auto"
  const initialType = params.type || "all"
  const initialLimit = numberParam(params.limit, 20)
  const initialOffset = Math.max(0, Number(params.offset || 0) || 0)
  const initialFilters = {
    authority_family: params.authority_family || "all",
    authority_tier: params.authority_tier || "all",
    jurisdiction: params.jurisdiction || "all",
    source_role: params.source_role || "all",
    chapter: params.chapter || "",
    status: params.status || "all",
    semantic_type: params.semantic_type || "all",
    current_only: boolParam(params.current_only),
    source_backed: boolParam(params.source_backed),
    has_citations: boolParam(params.has_citations),
    has_deadlines: boolParam(params.has_deadlines),
    has_penalties: boolParam(params.has_penalties),
    needs_review: boolParam(params.needs_review),
    primary_law: boolParam(params.primary_law),
    official_commentary: boolParam(params.official_commentary),
  }

  const responseState = q
    ? await getCachedSearchWithParamsState({
        q,
        type: initialType,
        mode: initialMode,
        limit: initialLimit,
        offset: initialOffset,
        ...initialFilters,
      })
    : undefined

  return (
    <Shell>
      <SearchClient 
        initialQuery={q}
        initialMode={initialMode}
        initialType={initialType}
        initialFilters={initialFilters}
        response={responseState?.data}
        initialDataSource={responseState?.source}
        initialDataError={responseState?.error}
      />
    </Shell>
  )
}
