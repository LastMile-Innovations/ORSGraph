import { 
  SearchResponse, 
  SuggestResult, 
  DirectOpenResponse,
  HomePageData,
  SystemHealth,
  GraphInsightCard,
  FeaturedStatute,
  StatutePageResponse,
  Provision,
  ProvisionInspectorData,
  AskAnswer,
  StatuteIdentity,
} from './types';

import { 
  mockHomePageData, 
  mockSystemHealth, 
  mockGraphInsights, 
  mockFeaturedStatutes,
  statuteIndex,
  recentItems,
  savedSearches,
  getStatuteByCanonicalId,
  getProvisionById,
  askAnswer as mockAskAnswer,
} from './mock-data';
import type { GraphNeighborhoodParams, GraphViewerResponse } from '@/components/graph/types';
import {
  classifyFallbackSource,
  dataErrorMessage,
  type DataSource,
  type DataState,
} from "./data-state";

const API_BASE_URL = process.env.NEXT_PUBLIC_ORS_API_BASE_URL || 'http://localhost:8080/api/v1';
const reportedFallbacks = new Set<string>();

export type SearchMode = 'auto' | 'hybrid' | 'keyword' | 'semantic' | 'citation'

export interface SearchParams {
  q: string
  type?: string
  mode?: SearchMode | string
  limit?: number
  offset?: number
  chapter?: string
  status?: string
  semantic_type?: string
  current_only?: boolean
  source_backed?: boolean
  has_citations?: boolean
  has_deadlines?: boolean
  has_penalties?: boolean
  needs_review?: boolean
}

async function fetchApi<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;
  const response = await fetch(url, {
    cache: 'no-store',
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(error.error || `API error: ${response.status}`);
  }

  return response.json();
}

function reportFallback(endpoint: string, error: unknown) {
  if (reportedFallbacks.has(endpoint)) return;
  reportedFallbacks.add(endpoint);
  console.info(`[ORSGraph] ${endpoint} unavailable; using fallback path (${dataErrorMessage(error)})`);
}

function fallbackState<T>(
  endpoint: string,
  data: T,
  error: unknown,
  source: DataSource = classifyFallbackSource(error),
): DataState<T> {
  reportFallback(endpoint, error);
  return { source, data, error: dataErrorMessage(error) };
}

// Health
export async function healthCheck() {
  return fetchApi<{ ok: boolean; service: string; neo4j: string; version: string }>('/health');
}

// Home Page
export async function getHomePageData(): Promise<HomePageData> {
  return (await getHomePageState()).data;
}

export async function getHomePageState(): Promise<DataState<HomePageData>> {
  try {
    return { source: "live", data: await fetchApi<HomePageData>('/home') };
  } catch (error) {
    const source = classifyFallbackSource(error);
    return fallbackState(
      "/home",
      {
        ...mockHomePageData,
        health: {
          ...mockHomePageData.health,
          api: source === "offline" ? "offline" : "mock",
          lastCheckedAt: new Date().toISOString(),
        },
      },
      error,
      source,
    );
  }
}

export async function getHealth(): Promise<SystemHealth> {
  return (await getHealthState()).data;
}

export async function getHealthState(): Promise<DataState<SystemHealth>> {
  try {
    return { source: "live", data: await fetchApi<SystemHealth>('/health') };
  } catch (error) {
    const source = classifyFallbackSource(error);
    return fallbackState(
      "/health",
      {
        ...mockSystemHealth,
        api: source === "offline" ? "offline" : "mock",
        lastCheckedAt: new Date().toISOString(),
      },
      error,
      source,
    );
  }
}

export async function getGraphInsights(): Promise<GraphInsightCard[]> {
  return (await getGraphInsightsState()).data;
}

export async function getGraphInsightsState(): Promise<DataState<GraphInsightCard[]>> {
  try {
    return { source: "live", data: await fetchApi<GraphInsightCard[]>('/analytics/home') };
  } catch (error) {
    return fallbackState("/analytics/home", mockGraphInsights, error);
  }
}

export async function getFeaturedStatutes(): Promise<FeaturedStatute[]> {
  return (await getFeaturedStatutesState()).data;
}

export async function getFeaturedStatutesState(): Promise<DataState<FeaturedStatute[]>> {
  try {
    return { source: "live", data: await fetchApi<FeaturedStatute[]>('/featured-statutes') };
  } catch (error) {
    return fallbackState("/featured-statutes", mockFeaturedStatutes, error);
  }
}

export async function openSearch(query: string): Promise<DirectOpenResponse> {
  const params = new URLSearchParams({ q: query });
  return fetchApi<DirectOpenResponse>(`/search/open?${params}`);
}

// Stats
export async function getStats() {
  return fetchApi<{
    nodes: number;
    relationships: number;
    chapters: number;
    sections: number;
    provisions: number;
    chunks: number;
    citations: number;
    cites_edges: number;
    semantic_nodes: number;
    last_seeded_at: string | null;
  }>('/stats');
}

interface StatuteIndexApiItem {
  canonical_id: string
  citation: string
  title: string | null
  chapter: string
  status: string
  edition_year: number
}

interface StatuteIndexApiResponse {
  items: StatuteIndexApiItem[]
  total: number
  limit: number
  offset: number
}

export interface SidebarStatute {
  canonical_id: string
  citation: string
  title: string
  chapter: string
  status: StatuteIdentity["status"]
  edition_year: number
  saved_at?: string | null
  opened_at?: string | null
}

export interface SidebarChapter {
  chapter: string
  label: string
  count: number
  items: SidebarStatute[]
}

export interface SidebarSavedSearch {
  saved_search_id: string
  query: string
  results: number
  created_at: string
  updated_at: string
}

export interface SidebarMatter {
  matter_id: string
  name: string
  status: string
  updated_at: string
  open_task_count: number
}

export interface SidebarData {
  corpus: {
    jurisdiction: string
    corpus: string
    edition_year: number
    total_statutes: number
    chapters: SidebarChapter[]
  }
  saved_searches: SidebarSavedSearch[]
  saved_statutes: SidebarStatute[]
  recent_statutes: SidebarStatute[]
  active_matter?: SidebarMatter | null
  updated_at: string
}

export async function getStatuteIndex(paramsInput: { limit?: number; offset?: number; chapter?: string } = {}): Promise<StatuteIdentity[]> {
  return (await getStatuteIndexState(paramsInput)).data;
}

export async function getStatuteIndexState(paramsInput: { limit?: number; offset?: number; chapter?: string } = {}): Promise<DataState<StatuteIdentity[]>> {
  try {
    const params = new URLSearchParams({
      limit: String(paramsInput.limit ?? 1000),
      offset: String(paramsInput.offset ?? 0),
    })
    if (paramsInput.chapter) params.set("chapter", paramsInput.chapter)

    const response = await fetchApi<StatuteIndexApiResponse>(`/statutes?${params}`)
    return { source: "live", data: response.items.map((item) => ({
      canonical_id: item.canonical_id,
      citation: item.citation,
      title: item.title ?? item.citation,
      jurisdiction: "Oregon",
      corpus: "ORS",
      chapter: item.chapter,
      status: normalizeLegalStatus(item.status),
      edition: item.edition_year,
    })) }
  } catch (error) {
    return fallbackState("/statutes", statuteIndex, error)
  }
}

export async function getSidebarState(): Promise<DataState<SidebarData>> {
  try {
    const response = await fetchApi<SidebarData>("/sidebar")
    return { source: "live", data: normalizeSidebarData(response) }
  } catch (error) {
    return fallbackState("/sidebar", buildFallbackSidebarData(), error)
  }
}

export async function saveSidebarSearch(input: { query: string; results?: number }): Promise<SidebarSavedSearch> {
  return fetchApi<SidebarSavedSearch>("/sidebar/saved-searches", {
    method: "POST",
    body: JSON.stringify(input),
  })
}

export async function deleteSidebarSearch(savedSearchId: string): Promise<{ deleted: boolean }> {
  return fetchApi<{ deleted: boolean }>(`/sidebar/saved-searches/${encodeURIComponent(savedSearchId)}`, {
    method: "DELETE",
  })
}

export async function saveSidebarStatute(canonicalId: string): Promise<SidebarStatute> {
  return fetchApi<SidebarStatute>("/sidebar/saved-statutes", {
    method: "POST",
    body: JSON.stringify({ canonical_id: canonicalId }),
  }).then(normalizeSidebarStatute)
}

export async function deleteSidebarStatute(statuteId: string): Promise<{ deleted: boolean }> {
  return fetchApi<{ deleted: boolean }>(`/sidebar/saved-statutes/${encodeURIComponent(statuteId)}`, {
    method: "DELETE",
  })
}

export async function recordSidebarRecentStatute(canonicalId: string): Promise<SidebarStatute> {
  return fetchApi<SidebarStatute>("/sidebar/recent-statutes", {
    method: "POST",
    body: JSON.stringify({ canonical_id: canonicalId }),
  }).then(normalizeSidebarStatute)
}

// Search
export async function search(
  query: string, 
  type: string = 'all', 
  mode: string = 'auto',
  limit: number = 20, 
  offset: number = 0
): Promise<SearchResponse> {
  return searchWithParams({ q: query, type, mode, limit, offset });
}

export async function searchWithParams(paramsInput: SearchParams): Promise<SearchResponse> {
  const params = new URLSearchParams({ 
    q: paramsInput.q,
    type: paramsInput.type || 'all',
    mode: paramsInput.mode || 'auto',
    limit: String(paramsInput.limit ?? 20),
    offset: String(paramsInput.offset ?? 0),
  });

  const optionalStringParams = [
    'chapter',
    'status',
    'semantic_type',
  ] as const

  for (const key of optionalStringParams) {
    const value = paramsInput[key]
    if (value && value !== 'all') params.set(key, value)
  }

  const optionalBooleanParams = [
    'current_only',
    'source_backed',
    'has_citations',
    'has_deadlines',
    'has_penalties',
    'needs_review',
  ] as const

  for (const key of optionalBooleanParams) {
    if (paramsInput[key] !== undefined) params.set(key, String(paramsInput[key]))
  }

  return fetchApi<SearchResponse>(`/search?${params}`);
}

export const getSearchResults = search;

export async function searchWithParamsState(paramsInput: SearchParams): Promise<DataState<SearchResponse | undefined>> {
  try {
    return { source: "live", data: await searchWithParams(paramsInput) };
  } catch (error) {
    reportFallback("/search", error);
    return { source: classifyFallbackSource(error), data: undefined, error: dataErrorMessage(error) };
  }
}

export async function searchSuggest(query: string, limit: number = 10): Promise<SuggestResult[]> {
  const params = new URLSearchParams({ q: query, limit: limit.toString() });
  return fetchApi<SuggestResult[]>(`/search/suggest?${params}`);
}

export async function directOpen(query: string): Promise<DirectOpenResponse> {
  const params = new URLSearchParams({ q: query });
  return fetchApi<DirectOpenResponse>(`/search/open?${params}`);
}

// Statute
export async function getStatute(citation: string) {
  return fetchApi<any>(`/statutes/${encodeURIComponent(citation)}`);
}

export async function getStatutePageData(citationOrCanonicalId: string): Promise<StatutePageResponse | null> {
  return (await getStatutePageDataState(citationOrCanonicalId)).data;
}

export async function getStatutePageDataState(citationOrCanonicalId: string): Promise<DataState<StatutePageResponse | null>> {
  try {
    const [detail, provisions, citations, semantics, history] = await Promise.all([
      getStatute(citationOrCanonicalId),
      getProvisions(citationOrCanonicalId),
      getCitations(citationOrCanonicalId),
      getSemantics(citationOrCanonicalId),
      getHistory(citationOrCanonicalId),
    ])

    return { source: "live", data: mapStatutePage(detail, provisions, citations, semantics, history) }
  } catch (error) {
    const fallback = getStatuteByCanonicalId(citationOrCanonicalId)
    return fallbackState(`/statutes/${citationOrCanonicalId}`, fallback, error, fallback ? classifyFallbackSource(error) : "error")
  }
}

export async function getProvisions(citation: string) {
  return fetchApi<{
    citation: string;
    provisions: Array<{
      provision_id: string;
      display_citation: string;
      local_path: string[];
      depth: number;
      text: string;
      children: any[];
    }>;
  }>(`/statutes/${encodeURIComponent(citation)}/provisions`);
}

export async function getCitations(citation: string) {
  return fetchApi<{
    citation: string;
    outbound: Array<{
      target_canonical_id: string | null;
      target_citation: string;
      context_snippet: string;
      source_provision: string;
      resolved: boolean;
    }>;
    inbound: Array<{
      target_canonical_id: string | null;
      target_citation: string;
      context_snippet: string;
      source_provision: string;
      resolved: boolean;
    }>;
    unresolved: Array<{
      target_canonical_id: string | null;
      target_citation: string;
      context_snippet: string;
      source_provision: string;
      resolved: boolean;
    }>;
  }>(`/statutes/${encodeURIComponent(citation)}/citations`);
}

export async function getSemantics(citation: string) {
  return fetchApi<{
    citation: string;
    obligations: Array<{ text: string; source_provision: string }>;
    exceptions: Array<{ text: string; source_provision: string }>;
    deadlines: Array<{
      description: string;
      duration: string;
      trigger: string;
      source_provision: string;
    }>;
    penalties: Array<{ text: string; source_provision: string }>;
    definitions: Array<{
      term: string;
      text: string;
      source_provision: string;
      scope: string;
    }>;
  }>(`/statutes/${encodeURIComponent(citation)}/semantics`);
}

export async function getHistory(citation: string) {
  return fetchApi<{
    citation: string;
    source_notes: string[];
    amendments: Array<{
      amendment_id: string;
      description: string;
      effective_date: string;
    }>;
    session_laws: Array<{
      session_law_id: string;
      citation: string;
      description: string;
    }>;
    status_events: Array<{
      event_id: string;
      event_type: string;
      date: string;
      description: string;
    }>;
  }>(`/statutes/${encodeURIComponent(citation)}/history`);
}

// Graph
export async function getNeighborhood(id: string, depth: number = 1, limit: number = 100) {
  return getGraphNeighborhood({ id, depth, limit });
}

export async function getGraphNeighborhood(input: GraphNeighborhoodParams): Promise<GraphViewerResponse> {
  const params = new URLSearchParams({
    depth: String(input.depth ?? 1),
    limit: String(input.limit ?? 100),
    mode: input.mode ?? 'legal',
  });

  if (input.id) params.set('id', input.id);
  if (input.citation) params.set('citation', input.citation);
  if (input.relationshipTypes?.length) params.set('relationshipTypes', input.relationshipTypes.join(','));
  if (input.nodeTypes?.length) params.set('nodeTypes', input.nodeTypes.join(','));
  if (input.includeChunks !== undefined) params.set('includeChunks', String(input.includeChunks));
  if (input.includeSimilarity !== undefined) params.set('includeSimilarity', String(input.includeSimilarity));
  if (input.similarityThreshold !== undefined) params.set('similarityThreshold', String(input.similarityThreshold));

  return fetchApi<GraphViewerResponse>(`/graph/neighborhood?${params}`);
}

// QC
export async function getQCSummary() {
  return fetchApi<{
    node_counts_by_label: Array<{ label: string; count: number }>;
    relationship_counts_by_type: Array<{ rel_type: string; count: number }>;
    orphan_counts: { provisions: number; chunks: number; citations: number };
    duplicate_counts: { legal_text_identities: number; provisions: number; cites_relationships: number };
    embedding_readiness: { total_chunks: number; embedded_chunks: number; coverage: number };
    cites_coverage: { total_citations: number; resolved_citations: number; coverage: number };
    last_qc_status: string | null;
  }>('/qc/summary');
}

export async function ask(question: string, mode: string = "research"): Promise<AskAnswer> {
  return fetchApi<AskAnswer>('/ask', {
    method: 'POST',
    body: JSON.stringify({ question, mode }),
  });
}

export async function askWithFallback(question: string, mode: string = "research"): Promise<AskAnswer> {
  return (await askWithFallbackState(question, mode)).data
}

export async function askWithFallbackState(question: string, mode: string = "research"): Promise<DataState<AskAnswer>> {
  try {
    return { source: "live", data: await ask(question, mode) }
  } catch (error) {
    return fallbackState("/ask", { ...mockAskAnswer, question }, error)
  }
}

export async function getProvisionInspectorData(provisionId: string): Promise<ProvisionInspectorData | null> {
  return (await getProvisionInspectorDataState(provisionId)).data
}

export async function getProvisionInspectorDataState(provisionId: string): Promise<DataState<ProvisionInspectorData | null>> {
  try {
    const response = await fetchApi<any>(`/provisions/${encodeURIComponent(provisionId)}`)
    return { source: "live", data: mapProvisionInspector(response) }
  } catch (error) {
    const fallback = getProvisionById(provisionId)
    return fallbackState(`/provisions/${provisionId}`, fallback, error, fallback ? classifyFallbackSource(error) : "error")
  }
}

function mapStatutePage(detail: any, provisionsResponse: any, citations: any, semantics: any, history: any): StatutePageResponse {
  const identity = {
    canonical_id: detail.identity.canonical_id,
    citation: detail.identity.citation,
    title: detail.identity.title ?? detail.identity.citation,
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: detail.identity.chapter,
    status: normalizeLegalStatus(detail.identity.status),
    edition: detail.source_document?.edition_year ?? 2025,
  }

  const currentVersion = {
    ...detail.current_version,
    source_documents: [detail.source_document?.source_id].filter(Boolean),
  }

  const provisions: Provision[] = (provisionsResponse.provisions ?? []).map((p: any) => mapProvision(p))
  const outbound = (citations.outbound ?? []).map((c: any) => ({
    target_canonical_id: c.target_canonical_id,
    target_citation: c.target_citation,
    context_snippet: c.context_snippet,
    source_provision: c.source_provision,
    resolved: Boolean(c.resolved),
  }))
  const inbound = (citations.inbound ?? []).map((c: any) => ({
    source_canonical_id: c.target_canonical_id ?? "",
    source_citation: c.target_citation,
    source_title: c.target_citation,
    source_provision: c.source_provision,
    context_snippet: c.context_snippet,
  }))

  return {
    identity,
    current_version: currentVersion,
    versions: [currentVersion],
    provisions,
    chunks: [],
    definitions: (semantics.definitions ?? []).map((d: any, index: number) => ({
      definition_id: `definition:${index}`,
      term: d.term,
      text: d.text,
      source_provision: d.source_provision,
      scope: d.scope ?? identity.citation,
    })),
    exceptions: (semantics.exceptions ?? []).map((e: any, index: number) => ({
      exception_id: `exception:${index}`,
      text: e.text,
      applies_to_provision: e.source_provision,
      source_provision: e.source_provision,
    })),
    deadlines: (semantics.deadlines ?? []).map((d: any, index: number) => ({
      deadline_id: `deadline:${index}`,
      description: d.description,
      duration: d.duration,
      trigger: d.trigger,
      source_provision: d.source_provision,
    })),
    penalties: (semantics.penalties ?? []).map((p: any, index: number) => ({
      penalty_id: `penalty:${index}`,
      description: p.text,
      category: "administrative",
      source_provision: p.source_provision,
    })),
    outbound_citations: outbound,
    inbound_citations: inbound,
    source_documents: [{
      source_id: detail.source_document?.source_id ?? "",
      url: detail.source_document?.url ?? "",
      retrieved_at: "",
      raw_hash: "",
      normalized_hash: "",
      edition_year: detail.source_document?.edition_year ?? 2025,
      parser_profile: "orsgraph-api",
      parser_warnings: [],
    }],
    qc: {
      status: history.source_notes?.length ? "warning" : "pass",
      passed_checks: history.source_notes?.length ? 1 : 2,
      total_checks: 2,
      notes: (history.source_notes ?? []).map((message: string, index: number) => ({
        note_id: `source-note:${index}`,
        level: "info",
        category: "source",
        message,
        related_id: detail.identity.canonical_id,
      })),
    },
  }
}

function mapProvision(p: any): Provision {
  return {
    provision_id: p.provision_id,
    display_citation: p.display_citation,
    provision_type: provisionTypeForDepth(p.depth),
    parent_id: null,
    text: p.text ?? "",
    text_preview: previewText(p.text ?? "", 180),
    signals: [],
    cites_count: 0,
    cited_by_count: 0,
    chunk_count: 0,
    qc_status: "pass",
    status: "active",
    children: (p.children ?? []).map((child: any) => mapProvision(child)),
  }
}

function mapProvisionInspector(response: any): ProvisionInspectorData {
  return {
    parent_statute: {
      canonical_id: response.parent_statute.canonical_id,
      citation: response.parent_statute.citation,
      title: response.parent_statute.title ?? response.parent_statute.citation,
      chapter: response.parent_statute.chapter,
      status: normalizeLegalStatus(response.parent_statute.status),
      edition: response.parent_statute.edition_year ?? 2025,
    },
    provision: mapProvisionDetail(response.provision),
    ancestors: response.ancestors ?? [],
    children: (response.children ?? []).map((p: any) => mapProvisionDetail(p)),
    siblings: response.siblings ?? [],
    chunks: response.chunks ?? [],
    outbound_citations: response.outbound_citations ?? [],
    inbound_citations: (response.inbound_citations ?? []).map((c: any) => ({
      source_canonical_id: c.target_canonical_id ?? "",
      source_citation: c.target_citation,
      source_title: c.target_citation,
      source_provision: c.source_provision,
      context_snippet: c.context_snippet,
    })),
    definitions: response.definitions ?? [],
    exceptions: response.exceptions ?? [],
    deadlines: response.deadlines ?? [],
    qc_notes: response.qc_notes ?? [],
  }
}

function mapProvisionDetail(p: any): Provision {
  return {
    provision_id: p.provision_id,
    display_citation: p.display_citation,
    provision_type: normalizeProvisionType(p.provision_type),
    parent_id: p.parent_id ?? null,
    text: p.text ?? "",
    text_preview: p.text_preview ?? previewText(p.text ?? "", 180),
    signals: p.signals ?? [],
    cites_count: p.cites_count ?? 0,
    cited_by_count: p.cited_by_count ?? 0,
    chunk_count: p.chunk_count ?? 0,
    qc_status: p.qc_status ?? "pass",
    status: normalizeLegalStatus(p.status),
    children: [],
  }
}

function normalizeSidebarData(data: SidebarData): SidebarData {
  return {
    ...data,
    corpus: {
      ...data.corpus,
      chapters: (data.corpus?.chapters ?? []).map((chapter) => ({
        ...chapter,
        count: Number(chapter.count ?? chapter.items?.length ?? 0),
        items: (chapter.items ?? []).map(normalizeSidebarStatute),
      })),
    },
    saved_searches: (data.saved_searches ?? []).map((search) => ({
      ...search,
      results: Number(search.results ?? 0),
    })),
    saved_statutes: (data.saved_statutes ?? []).map(normalizeSidebarStatute),
    recent_statutes: (data.recent_statutes ?? []).map(normalizeSidebarStatute),
    active_matter: data.active_matter ?? null,
  }
}

function normalizeSidebarStatute(item: SidebarStatute): SidebarStatute {
  return {
    canonical_id: item.canonical_id,
    citation: item.citation,
    title: item.title || item.citation,
    chapter: item.chapter,
    status: normalizeLegalStatus(item.status),
    edition_year: Number(item.edition_year ?? 2025),
    saved_at: item.saved_at ?? null,
    opened_at: item.opened_at ?? null,
  }
}

function buildFallbackSidebarData(): SidebarData {
  const chapters = statuteIndex.reduce<Map<string, SidebarChapter>>((acc, statute) => {
    const chapter = acc.get(statute.chapter) ?? {
      chapter: statute.chapter,
      label: `Chapter ${statute.chapter}`,
      count: 0,
      items: [],
    }
    chapter.count += 1
    if (chapter.items.length < 8) {
      chapter.items.push(sidebarStatuteFromIdentity(statute))
    }
    acc.set(statute.chapter, chapter)
    return acc
  }, new Map())

  return {
    corpus: {
      jurisdiction: "Oregon",
      corpus: "ORS",
      edition_year: 2025,
      total_statutes: statuteIndex.length,
      chapters: Array.from(chapters.values()),
    },
    saved_searches: savedSearches.map((search) => ({
      saved_search_id: search.id,
      query: search.query,
      results: search.results,
      created_at: "",
      updated_at: "",
    })),
    saved_statutes: recentItems.slice(0, 3).map(sidebarStatuteFromRecent),
    recent_statutes: recentItems.map(sidebarStatuteFromRecent),
    active_matter: null,
    updated_at: new Date().toISOString(),
  }
}

function sidebarStatuteFromRecent(item: { canonical_id: string; citation: string; title: string }): SidebarStatute {
  const identity = statuteIndex.find((statute) => statute.canonical_id === item.canonical_id)
  return identity
    ? sidebarStatuteFromIdentity(identity)
    : {
        canonical_id: item.canonical_id,
        citation: item.citation,
        title: item.title,
        chapter: "",
        status: "active",
        edition_year: 2025,
      }
}

function sidebarStatuteFromIdentity(item: StatuteIdentity): SidebarStatute {
  return {
    canonical_id: item.canonical_id,
    citation: item.citation,
    title: item.title,
    chapter: item.chapter,
    status: normalizeLegalStatus(item.status),
    edition_year: item.edition,
  }
}

function normalizeLegalStatus(status?: string) {
  const value = (status ?? "active").toLowerCase()
  return ["active", "repealed", "renumbered", "amended"].includes(value) ? value as any : "active"
}

function normalizeProvisionType(type?: string) {
  const value = (type ?? "section").toLowerCase()
  return ["section", "subsection", "paragraph", "subparagraph", "clause"].includes(value) ? value as any : "section"
}

function provisionTypeForDepth(depth?: number) {
  if ((depth ?? 0) <= 0) return "section"
  if (depth === 1) return "subsection"
  if (depth === 2) return "paragraph"
  if (depth === 3) return "subparagraph"
  return "clause"
}

function previewText(text: string, max: number) {
  return text.length > max ? `${text.slice(0, max).trim()}...` : text
}
