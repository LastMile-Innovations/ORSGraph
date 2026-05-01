export type AdminJobKind =
  | "crawl"
  | "parse"
  | "qc"
  | "seed_neo4j"
  | "materialize_neo4j"
  | "embed_neo4j"
  | "source_ingest"
  | "combine_graph"

export type AdminJobStatus =
  | "queued"
  | "running"
  | "cancel_requested"
  | "succeeded"
  | "failed"
  | "cancelled"

export interface AdminJobParams {
  source_id?: string
  priority?: string
  out_dir?: string
  graph_dir?: string
  edition_year?: number
  max_chapters?: number
  chapters?: string
  session_key?: string
  fetch_only?: boolean
  skip_citation_resolution?: boolean
  dry_run?: boolean
  embed?: boolean
  create_vector_indexes?: boolean
}

export interface AdminJobProgress {
  phase?: string
  stdout_lines: number
  stderr_lines: number
  event_count: number
}

export interface AdminJob {
  job_id: string
  kind: AdminJobKind
  status: AdminJobStatus
  params: AdminJobParams
  command: string[]
  command_display: string
  is_read_only: boolean
  created_at_ms: number
  started_at_ms?: number
  finished_at_ms?: number
  exit_code?: number
  message?: string
  output_paths: Record<string, string>
  progress: AdminJobProgress
}

export interface AdminJobEvent {
  event_id: string
  job_id: string
  timestamp_ms: number
  level: string
  message: string
  stream?: string
}

export interface AdminJobDetail {
  job: AdminJob
  allow_kill: boolean
  recent_events: AdminJobEvent[]
  stdout_tail: string[]
  stderr_tail: string[]
}

export interface AdminOverview {
  enabled: boolean
  allow_kill: boolean
  active_job?: AdminJob | null
  recent_jobs: AdminJob[]
  job_counts: Record<string, number>
  paths: {
    jobs_dir: string
    data_dir: string
    graph_dir: string
  }
  sources: {
    registry_sources: number
    source_dirs: number
    source_artifacts: number
    source_bytes: number
  }
  graph: {
    jsonl_files: number
    rows: number
    bytes: number
  }
  indexing: {
    vector_enabled: boolean
    vector_search_enabled: boolean
    vector_index: string
    vector_dimension: number
    embedding_model: string
  }
  health: {
    api: string
    neo4j: string
    version: string
  }
}

export interface AdminLogResponse {
  job_id: string
  stream: string
  lines: string[]
}

export interface AdminSourceLocalStatus {
  source_dir_exists: boolean
  source_artifacts: number
  source_bytes: number
  graph_files: number
  graph_rows: number
  qc_status?: string | null
  last_finished_at?: string | null
}

export interface AdminSourceRegistryEntry {
  source_id: string
  name: string
  owner: string
  jurisdiction: string
  source_type: string
  access: string
  official_status: string
  connector_status: string
  priority: string
  source_url: string
  docs_url: string
  graph_nodes_created: string[]
  graph_edges_created: string[]
  local: AdminSourceLocalStatus
}

export interface AdminSourceRegistryResponse {
  sources: AdminSourceRegistryEntry[]
  totals: {
    sources: number
    p0_sources: number
    local_source_dirs: number
    local_artifacts: number
    local_bytes: number
  }
}

export interface AdminSourceDetail {
  source: AdminSourceRegistryEntry
  stats?: unknown
  qc_report?: unknown
  graph_files: Array<{ file: string; rows: number; bytes: number }>
  raw_artifacts: Array<{
    file: string
    bytes: number
    content_type?: string | null
    raw_hash?: string | null
    status?: string | null
    skipped?: boolean | null
  }>
}

const API_BASE_URL = process.env.NEXT_PUBLIC_ORS_API_BASE_URL || "http://localhost:8080/api/v1"
const API_KEY = process.env.NEXT_PUBLIC_ORS_API_KEY

async function fetchAdmin<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
  const headers = new Headers(options.headers)
  if (!headers.has("Content-Type") && typeof options.body === "string") {
    headers.set("Content-Type", "application/json")
  }
  if (API_KEY && !headers.has("x-api-key")) {
    headers.set("x-api-key", API_KEY)
  }

  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    cache: "no-store",
    ...options,
    headers,
  })

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: `Admin API error: ${response.status}` }))
    throw new Error(error.error || `Admin API error: ${response.status}`)
  }

  return response.json()
}

export function getAdminOverview() {
  return fetchAdmin<AdminOverview>("/admin/overview")
}

export function listAdminSources(params: { priority?: string; connector_status?: string } = {}) {
  const search = new URLSearchParams()
  if (params.priority) search.set("priority", params.priority)
  if (params.connector_status) search.set("connector_status", params.connector_status)
  const query = search.toString()
  return fetchAdmin<AdminSourceRegistryResponse>(`/admin/sources${query ? `?${query}` : ""}`)
}

export function getAdminSource(sourceId: string) {
  return fetchAdmin<AdminSourceDetail>(`/admin/sources/${encodeURIComponent(sourceId)}`)
}

export function listAdminJobs(params: { status?: AdminJobStatus; kind?: AdminJobKind; limit?: number; offset?: number } = {}) {
  const search = new URLSearchParams()
  if (params.status) search.set("status", params.status)
  if (params.kind) search.set("kind", params.kind)
  if (params.limit) search.set("limit", String(params.limit))
  if (params.offset) search.set("offset", String(params.offset))
  const query = search.toString()
  return fetchAdmin<AdminJob[]>(`/admin/jobs${query ? `?${query}` : ""}`)
}

export function getAdminJobDetail(jobId: string) {
  return fetchAdmin<AdminJobDetail>(`/admin/jobs/${encodeURIComponent(jobId)}`)
}

export function getAdminJobLogs(jobId: string, stream: "stdout" | "stderr", tail = 200) {
  const search = new URLSearchParams({ stream, tail: String(tail) })
  return fetchAdmin<AdminLogResponse>(`/admin/jobs/${encodeURIComponent(jobId)}/logs?${search}`)
}

export function startAdminJob(kind: AdminJobKind, params: AdminJobParams = {}) {
  return fetchAdmin<AdminJobDetail>("/admin/jobs", {
    method: "POST",
    body: JSON.stringify({ kind, params }),
  })
}

export function cancelAdminJob(jobId: string) {
  return fetchAdmin<AdminJobDetail>(`/admin/jobs/${encodeURIComponent(jobId)}/cancel`, {
    method: "POST",
    body: "{}",
  })
}

export function killAdminJob(jobId: string) {
  return fetchAdmin<AdminJobDetail>(`/admin/jobs/${encodeURIComponent(jobId)}/kill`, {
    method: "POST",
    body: "{}",
  })
}

export function isTerminalJob(status: AdminJobStatus) {
  return status === "succeeded" || status === "failed" || status === "cancelled"
}
