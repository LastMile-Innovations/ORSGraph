import { 
  SearchResponse, 
  SuggestResult, 
  DirectOpenResponse,
  SearchResult,
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
  getStatuteByCanonicalId,
  getProvisionById,
  askAnswer as mockAskAnswer,
} from './mock-data';
import type { GraphNeighborhoodParams, GraphViewerResponse } from '@/components/graph/types';

const API_BASE_URL = process.env.NEXT_PUBLIC_ORS_API_BASE_URL || 'http://localhost:8080/api/v1';

export type SearchMode = 'hybrid' | 'keyword' | 'semantic' | 'citation'

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

// Health
export async function healthCheck() {
  return fetchApi<{ ok: boolean; service: string; neo4j: string; version: string }>('/health');
}

// Home Page
export async function getHomePageData(): Promise<HomePageData> {
  try {
    return await fetchApi<HomePageData>('/home');
  } catch (error) {
    console.warn("Failed to fetch /home, falling back to mock data", error);
    return mockHomePageData;
  }
}

export async function getHealth(): Promise<SystemHealth> {
  try {
    return await fetchApi<SystemHealth>('/health');
  } catch (error) {
    return mockSystemHealth;
  }
}

export async function getGraphInsights(): Promise<GraphInsightCard[]> {
  try {
    return await fetchApi<GraphInsightCard[]>('/analytics/home');
  } catch (error) {
    return mockGraphInsights;
  }
}

export async function getFeaturedStatutes(): Promise<FeaturedStatute[]> {
  try {
    return await fetchApi<FeaturedStatute[]>('/featured-statutes');
  } catch (error) {
    return mockFeaturedStatutes;
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

export async function getStatuteIndex(paramsInput: { limit?: number; offset?: number; chapter?: string } = {}): Promise<StatuteIdentity[]> {
  try {
    const params = new URLSearchParams({
      limit: String(paramsInput.limit ?? 1000),
      offset: String(paramsInput.offset ?? 0),
    })
    if (paramsInput.chapter) params.set("chapter", paramsInput.chapter)

    const response = await fetchApi<StatuteIndexApiResponse>(`/statutes?${params}`)
    return response.items.map((item) => ({
      canonical_id: item.canonical_id,
      citation: item.citation,
      title: item.title ?? item.citation,
      jurisdiction: "Oregon",
      corpus: "ORS",
      chapter: item.chapter,
      status: normalizeLegalStatus(item.status),
      edition: item.edition_year,
    }))
  } catch (error) {
    console.warn("Failed to fetch /statutes, falling back to mock index", error)
    return statuteIndex
  }
}

// Search
export async function search(
  query: string, 
  type: string = 'all', 
  mode: string = 'hybrid',
  limit: number = 20, 
  offset: number = 0
): Promise<SearchResponse> {
  return searchWithParams({ q: query, type, mode, limit, offset });
}

export async function searchWithParams(paramsInput: SearchParams): Promise<SearchResponse> {
  const params = new URLSearchParams({ 
    q: paramsInput.q,
    type: paramsInput.type || 'all',
    mode: paramsInput.mode || 'hybrid',
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
  try {
    const [detail, provisions, citations, semantics, history] = await Promise.all([
      getStatute(citationOrCanonicalId),
      getProvisions(citationOrCanonicalId),
      getCitations(citationOrCanonicalId),
      getSemantics(citationOrCanonicalId),
      getHistory(citationOrCanonicalId),
    ])

    return mapStatutePage(detail, provisions, citations, semantics, history)
  } catch (error) {
    console.warn("Failed to fetch statute detail, falling back to mock statute", error)
    return getStatuteByCanonicalId(citationOrCanonicalId)
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
  try {
    return await ask(question, mode)
  } catch (error) {
    console.warn("Failed to fetch /ask, falling back to mock answer", error)
    return { ...mockAskAnswer, question }
  }
}

export async function getProvisionInspectorData(provisionId: string): Promise<ProvisionInspectorData | null> {
  try {
    const response = await fetchApi<any>(`/provisions/${encodeURIComponent(provisionId)}`)
    return mapProvisionInspector(response)
  } catch (error) {
    console.warn("Failed to fetch provision detail, falling back to mock provision", error)
    return getProvisionById(provisionId)
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
