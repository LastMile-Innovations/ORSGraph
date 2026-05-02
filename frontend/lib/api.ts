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
  SourceIndexEntry,
} from './types';

import { 
  mockHomePageData, 
  mockSystemHealth, 
  mockGraphInsights, 
  mockFeaturedStatutes,
  getProvisionById,
  askAnswer as mockAskAnswer,
} from './mock-data';
import { getSourceById as getDemoSourceById, sourceIndex as demoSourceIndex } from "./mock-sources";
import { orsApiBaseUrl } from "./ors-api-url";
import type { GraphFullParams, GraphNeighborhoodParams, GraphViewerResponse } from '@/components/graph/types';
import {
  classifyApiFailureSource,
  classifyFallbackSource,
  dataErrorMessage,
  DEMO_MODE,
  type DataSource,
  type DataState,
} from "./data-state";

const API_BASE_URL = orsApiBaseUrl();
const API_TIMEOUT_MS = Number(process.env.NEXT_PUBLIC_ORS_API_TIMEOUT_MS || 5000);
const reportedFallbacks = new Set<string>();

export class ApiRequestError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly endpoint: string,
  ) {
    super(message)
    this.name = "ApiRequestError"
  }
}

export type SearchMode = 'auto' | 'hybrid' | 'keyword' | 'semantic' | 'citation'

export interface SearchParams {
  q: string
  type?: string
  mode?: SearchMode | string
  limit?: number
  offset?: number
  authority_family?: string
  authority_tier?: string
  jurisdiction?: string
  source_role?: string
  chapter?: string
  status?: string
  semantic_type?: string
  current_only?: boolean
  source_backed?: boolean
  has_citations?: boolean
  has_deadlines?: boolean
  has_penalties?: boolean
  needs_review?: boolean
  primary_law?: boolean
  official_commentary?: boolean
}

async function fetchApi<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;
  const controller = new AbortController();
  let timedOut = false;
  const timeout = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, API_TIMEOUT_MS);
  const parentSignal = options.signal;
  const abortFromParent = () => controller.abort();

  if (parentSignal?.aborted) {
    controller.abort();
  } else {
    parentSignal?.addEventListener("abort", abortFromParent, { once: true });
  }

  try {
    const response = await fetch(url, {
      cache: 'no-store',
      ...options,
      signal: controller.signal,
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }));
      throw new ApiRequestError(error.error || `API error: ${response.status}`, response.status, endpoint);
    }

    return response.json();
  } catch (error) {
    if (timedOut && isAbortError(error)) {
      throw new Error(`API request timed out after ${Math.round(API_TIMEOUT_MS / 1000)}s`);
    }
    throw error;
  } finally {
    clearTimeout(timeout);
    parentSignal?.removeEventListener("abort", abortFromParent);
  }
}

function isAbortError(error: unknown) {
  return error instanceof DOMException
    ? error.name === "AbortError"
    : error instanceof Error && error.name === "AbortError";
}

function reportFallback(endpoint: string, error: unknown) {
  if (reportedFallbacks.has(endpoint)) return;
  reportedFallbacks.add(endpoint);
  console.info(`[ORSGraph] ${endpoint} unavailable; using fallback path (${dataErrorMessage(error)})`);
}

function reportApiFailure(endpoint: string, error: unknown) {
  if (reportedFallbacks.has(endpoint)) return;
  reportedFallbacks.add(endpoint);
  console.info(`[ORSGraph] ${endpoint} unavailable (${dataErrorMessage(error)})`);
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

function apiFailureState<T>(
  endpoint: string,
  data: T,
  error: unknown,
  source: DataSource = classifyApiFailureSource(error),
): DataState<T> {
  reportApiFailure(endpoint, error);
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

export interface StatuteIndexParams {
  q?: string
  limit?: number
  offset?: number
  chapter?: string
  status?: string
}

export interface StatuteIndexResult {
  items: StatuteIdentity[]
  total: number
  limit: number
  offset: number
}

export interface SourceIndexParams {
  q?: string
  status?: string
  edition_year?: number
  limit?: number
  offset?: number
}

export interface SourceIndexResult {
  items: SourceIndexEntry[]
  total: number
  limit: number
  offset: number
}

export interface SourceDetailResult {
  source: SourceIndexEntry
  related_sources: SourceIndexEntry[]
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

export async function getStatuteIndex(paramsInput: StatuteIndexParams = {}): Promise<StatuteIdentity[]> {
  return (await getStatuteIndexState(paramsInput)).data.items;
}

export async function getStatuteIndexState(paramsInput: StatuteIndexParams = {}): Promise<DataState<StatuteIndexResult>> {
  const limit = paramsInput.limit ?? 60
  const offset = paramsInput.offset ?? 0

  try {
    const params = new URLSearchParams({
      limit: String(limit),
      offset: String(offset),
    })
    if (paramsInput.q) params.set("q", paramsInput.q)
    if (paramsInput.chapter) params.set("chapter", paramsInput.chapter)
    if (paramsInput.status && paramsInput.status !== "all") params.set("status", paramsInput.status)

    const response = await fetchApi<StatuteIndexApiResponse>(`/statutes?${params}`)
    return {
      source: "live",
      data: {
        items: response.items.map(mapStatuteIndexItem),
        total: Number(response.total ?? response.items.length),
        limit: Number(response.limit ?? limit),
        offset: Number(response.offset ?? offset),
      },
    }
  } catch (error) {
    return apiFailureState("/statutes", { items: [], total: 0, limit, offset }, error)
  }
}

export async function getSourcesState(paramsInput: SourceIndexParams = {}): Promise<DataState<SourceIndexResult>> {
  try {
    const params = new URLSearchParams({
      limit: String(paramsInput.limit ?? 50),
      offset: String(paramsInput.offset ?? 0),
    })
    if (paramsInput.q) params.set("q", paramsInput.q)
    if (paramsInput.status && paramsInput.status !== "all") params.set("status", paramsInput.status)
    if (paramsInput.edition_year) params.set("edition_year", String(paramsInput.edition_year))

    const response = await fetchApi<SourceIndexResult>(`/sources?${params}`)
    return { source: response.items.length > 0 ? "live" : "empty", data: response }
  } catch (error) {
    if (DEMO_MODE) {
      return fallbackState(
        "/sources",
        {
          items: demoSourceIndex,
          total: demoSourceIndex.length,
          limit: paramsInput.limit ?? demoSourceIndex.length,
          offset: paramsInput.offset ?? 0,
        },
        error,
        "demo",
      )
    }
    return {
      source: classifyFallbackSource(error) === "offline" ? "offline" : "error",
      data: { items: [], total: 0, limit: paramsInput.limit ?? 50, offset: paramsInput.offset ?? 0 },
      error: dataErrorMessage(error),
    }
  }
}

export async function getSourceDetailState(sourceId: string): Promise<DataState<SourceDetailResult | null>> {
  try {
    return { source: "live", data: await fetchApi<SourceDetailResult>(`/sources/${encodeURIComponent(sourceId)}`) }
  } catch (error) {
    if (DEMO_MODE) {
      const source = getDemoSourceById(sourceId)
      return fallbackState(
        `/sources/${sourceId}`,
        source
          ? {
              source,
              related_sources: demoSourceIndex.filter((item) => item.source_id !== source.source_id).slice(0, 6),
            }
          : null,
        error,
        source ? "demo" : "error",
      )
    }
    return {
      source: classifyFallbackSource(error) === "offline" ? "offline" : "error",
      data: null,
      error: dataErrorMessage(error),
    }
  }
}

function mapStatuteIndexItem(item: StatuteIndexApiItem): StatuteIdentity {
  return {
    canonical_id: item.canonical_id,
    citation: item.citation,
    title: item.title ?? item.citation,
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: item.chapter,
    status: normalizeLegalStatus(item.status),
    edition: item.edition_year,
  }
}

export async function getSidebarState(): Promise<DataState<SidebarData | null>> {
  try {
    const response = await fetchApi<SidebarData>("/sidebar")
    return { source: "live", data: normalizeSidebarData(response) }
  } catch (error) {
    return apiFailureState("/sidebar", null, error)
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
    'authority_family',
    'authority_tier',
    'jurisdiction',
    'source_role',
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
    'primary_law',
    'official_commentary',
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

export async function searchSuggest(
  query: string,
  limit: number = 10,
  signal?: AbortSignal,
): Promise<SuggestResult[]> {
  const params = new URLSearchParams({ q: query, limit: limit.toString() });
  return fetchApi<SuggestResult[]>(`/search/suggest?${params}`, { signal });
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
    const page = await fetchApi<any>(`/statutes/${encodeURIComponent(citationOrCanonicalId)}/page`)
    return { source: "live", data: mapStatuteCompactPage(page) }
  } catch (error) {
    return apiFailureState(
      `/statutes/${citationOrCanonicalId}/page`,
      null,
      error,
      error instanceof ApiRequestError && error.status === 404 ? "empty" : classifyApiFailureSource(error),
    )
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

export async function getChunks(citation: string) {
  return fetchApi<{
    citation: string;
    chunks: Array<{
      chunk_id: string;
      chunk_type: string;
      source_kind: "statute" | "provision";
      source_id: string;
      text: string;
      embedding_policy: "primary" | "secondary" | "none";
      answer_policy: "preferred" | "supporting" | "context_only";
      search_weight: number;
      embedded: boolean;
      parser_confidence: number;
    }>;
  }>(`/statutes/${encodeURIComponent(citation)}/chunks`);
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

export async function getFullGraph(input: GraphFullParams = {}): Promise<GraphViewerResponse> {
  const params = new URLSearchParams();

  if (input.relationshipTypes?.length) params.set('relationshipTypes', input.relationshipTypes.join(','));
  if (input.nodeTypes?.length) params.set('nodeTypes', input.nodeTypes.join(','));
  if (input.includeChunks !== undefined) params.set('includeChunks', String(input.includeChunks));
  if (input.includeSimilarity !== undefined) params.set('includeSimilarity', String(input.includeSimilarity));
  if (input.similarityThreshold !== undefined) params.set('similarityThreshold', String(input.similarityThreshold));

  const query = params.toString();
  return fetchApi<GraphViewerResponse>(`/graph/full${query ? `?${query}` : ''}`);
}

export interface GraphPathResponse {
  from: string
  to: string
  paths: Array<{ node_ids: string[]; edge_ids: string[]; length: number }>
  nodes: GraphViewerResponse["nodes"]
  edges: GraphViewerResponse["edges"]
  stats: GraphViewerResponse["stats"]
}

export async function getGraphPath(input: { from: string; to: string; mode?: string; limit?: number }): Promise<GraphPathResponse> {
  const params = new URLSearchParams({
    from: input.from,
    to: input.to,
    mode: input.mode ?? "legal",
    limit: String(input.limit ?? 3),
  })
  return fetchApi<GraphPathResponse>(`/graph/path?${params}`)
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

export type QCSummary = Awaited<ReturnType<typeof getQCSummary>>

export async function runQCRun() {
  return fetchApi<{
    run_id: string
    status: string
    started_at: string
    completed_at: string
    summary: QCSummary
    warnings: string[]
  }>("/qc/runs", { method: "POST" })
}

export async function getQCReport(format: "json" | "csv" = "json") {
  return fetchApi<{
    report_id: string
    format: string
    mime_type: string
    generated_at: string
    summary: QCSummary
    content: string
  }>(`/qc/reports/latest?format=${encodeURIComponent(format)}`)
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

function mapStatuteCompactPage(page: any): StatutePageResponse {
  const sourceDocument = page.source_document ?? {}
  const identity = {
    canonical_id: page.identity?.canonical_id ?? "",
    citation: page.identity?.citation ?? "",
    title: page.identity?.title ?? page.identity?.citation ?? "",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: page.identity?.chapter ?? "",
    status: normalizeLegalStatus(page.identity?.status),
    edition: sourceDocument.edition_year ?? 2025,
  }
  const currentVersion = {
    version_id: page.current_version?.version_id ?? "",
    effective_date: page.current_version?.effective_date ?? "",
    end_date: page.current_version?.end_date ?? null,
    is_current: Boolean(page.current_version?.is_current),
    text: page.current_version?.text ?? "",
    source_documents: [sourceDocument.source_id].filter(Boolean),
  }
  const source_documents = [{
    source_id: sourceDocument.source_id ?? "",
    url: sourceDocument.url ?? "",
    retrieved_at: "",
    raw_hash: "",
    normalized_hash: "",
    edition_year: sourceDocument.edition_year ?? 2025,
    parser_profile: "orsgraph-api",
    parser_warnings: [],
  }]
  const citation_counts = {
    outbound: Number(page.citation_counts?.outbound ?? 0),
    inbound: Number(page.citation_counts?.inbound ?? 0),
  }
  const semantic_counts = {
    obligations: Number(page.semantic_counts?.obligations ?? 0),
    exceptions: Number(page.semantic_counts?.exceptions ?? 0),
    deadlines: Number(page.semantic_counts?.deadlines ?? 0),
    penalties: Number(page.semantic_counts?.penalties ?? 0),
    definitions: Number(page.semantic_counts?.definitions ?? 0),
  }
  const notes = (page.qc?.notes ?? []).map((note: any, index: number) => ({
    note_id: note.note_id ?? `source-note:${index}`,
    level: normalizeQCNoteLevel(note.level),
    category: note.category ?? "source",
    message: note.message ?? "",
    related_id: note.related_id ?? identity.canonical_id,
  }))

  return {
    identity,
    current_version: currentVersion,
    versions: [currentVersion],
    provisions: (page.provisions ?? []).map((p: any) => mapProvision(p)),
    chunks: [],
    definitions: [],
    exceptions: [],
    deadlines: [],
    penalties: [],
    outbound_citations: [],
    inbound_citations: [],
    source_documents,
    qc: {
      status: normalizeQCStatus(page.qc?.status),
      passed_checks: Number(page.qc?.passed_checks ?? (notes.length ? 1 : 2)),
      total_checks: Number(page.qc?.total_checks ?? 2),
      notes,
    },
    summary_counts: {
      provision_count: Number(page.provision_count ?? 0),
      citation_counts,
      semantic_counts,
    },
    source_notes: page.source_notes ?? [],
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

function normalizeLegalStatus(status?: string) {
  const value = (status ?? "active").toLowerCase()
  return ["active", "repealed", "renumbered", "amended"].includes(value) ? value as any : "active"
}

function normalizeQCStatus(status?: string) {
  const value = (status ?? "pass").toLowerCase()
  return ["pass", "warning", "fail"].includes(value) ? value as any : "pass"
}

function normalizeQCNoteLevel(level?: string) {
  const value = (level ?? "info").toLowerCase()
  return ["info", "warning", "fail"].includes(value) ? value as any : "info"
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
