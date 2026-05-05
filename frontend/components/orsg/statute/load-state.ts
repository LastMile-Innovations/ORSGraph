import type { StatutePageResponse } from "@/lib/types"

export type StatuteLoadedState = {
  citations: boolean
  semantics: boolean
  chunks: boolean
  history: boolean
}

export function statuteLoadedStateFor(data: StatutePageResponse): StatuteLoadedState {
  return {
    citations: data.outbound_citations.length > 0 || data.inbound_citations.length > 0,
    semantics: data.definitions.length > 0 || data.deadlines.length > 0 || data.exceptions.length > 0 || data.penalties.length > 0,
    chunks: data.chunks.length > 0,
    history: Boolean(data.source_notes?.length),
  }
}

export function semanticCount(
  data: StatutePageResponse,
  loadedState: StatuteLoadedState | undefined,
  key: "definitions" | "exceptions" | "deadlines" | "penalties",
) {
  if (loadedState?.semantics) return data[key].length
  return data.summary_counts?.semantic_counts[key] ?? data[key].length
}

export function citationCount(
  data: StatutePageResponse,
  loadedState: StatuteLoadedState | undefined,
  key: "outbound" | "inbound",
) {
  if (loadedState?.citations) {
    return key === "outbound" ? data.outbound_citations.length : data.inbound_citations.length
  }
  return data.summary_counts?.citation_counts[key] ?? (key === "outbound" ? data.outbound_citations.length : data.inbound_citations.length)
}
