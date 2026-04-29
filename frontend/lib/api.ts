import { 
  SearchResponse, 
  SuggestResult, 
  DirectOpenResponse,
  SearchResult,
  HomePageData,
  SystemHealth,
  GraphInsightCard,
  FeaturedStatute,
} from './types';

import { 
  mockHomePageData, 
  mockSystemHealth, 
  mockGraphInsights, 
  mockFeaturedStatutes 
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
    orphan_counts: { chunks: number; citations: number };
    duplicate_counts: { provisions: number };
    embedding_readiness: { total_chunks: number; embedded_chunks: number; coverage: number };
    cites_coverage: { total_citations: number; resolved_citations: number; coverage: number };
    last_qc_status: string | null;
  }>('/qc/summary');
}

// Ask (stub)
export async function ask(question: string) {
  return fetchApi<{ error: string }>('/ask', {
    method: 'POST',
    body: JSON.stringify({ question }),
  });
}
