import "server-only"

import { cacheLife, cacheTag } from "next/cache"
import {
  getHomePageState,
  getProvisionInspectorDataState,
  getSourceDetailState,
  getSourcesState,
  getStatuteIndexState,
  getStatutePageDataState,
  searchWithParamsState,
  type SearchParams,
  type SourceIndexParams,
  type SourceDetailResult,
  type StatuteIndexParams,
} from "./api"
import type { DataState } from "./data-state"
import { authorityCacheTags } from "./authority-hotset.mjs"

const AUTHORITY_RELEASE_ID = releaseIdFromHotsetBaseUrl(process.env.ORS_AUTHORITY_HOTSET_BASE_URL || "") || "release:unversioned"
const AUTHORITY_CACHE_MODE = "memory-with-authority-hotset"

export function authorityServerCacheMode() {
  return AUTHORITY_CACHE_MODE
}

export async function getCachedHomePageState() {
  "use cache"
  tagAuthorityRead("home")
  cacheLife("authorityShell")
  return getHomePageState()
}

export async function getCachedSourcesState(params: SourceIndexParams = {}) {
  return getCachedSourcesStateForParams(params)
}

async function getCachedSourcesStateForParams(params: SourceIndexParams) {
  "use cache"
  tagAuthorityRead("sources", stableAuthorityParamsKey(params))
  cacheLife("authorityShell")
  return getSourcesState(params)
}

export async function getCachedStatuteIndexState(params: StatuteIndexParams = {}) {
  return getStatuteIndexState(params)
}

export async function getCachedStatutePageDataState(citationOrCanonicalId: string) {
  return getStatutePageDataState(citationOrCanonicalId)
}

export async function getCachedSearchWithParamsState(params: SearchParams) {
  return searchWithParamsState(params)
}

export async function getCachedSourceDetailState(sourceId: string): Promise<DataState<SourceDetailResult | null>> {
  "use cache"
  tagAuthorityRead("source", sourceId)
  cacheLife("authorityDetail")
  return getSourceDetailState(sourceId)
}

export async function getCachedProvisionInspectorDataState(provisionId: string) {
  "use cache"
  tagAuthorityRead("provision", provisionId)
  cacheLife("authorityDetail")
  return getProvisionInspectorDataState(provisionId)
}

function tagAuthorityRead(...keys: string[]) {
  for (const tag of authorityCacheTags(AUTHORITY_RELEASE_ID, keys)) {
    cacheTag(tag)
  }
}

function releaseIdFromHotsetBaseUrl(baseUrl: string) {
  const segment = baseUrl.replace(/\/$/, "").split("/").filter(Boolean).at(-1) || ""
  try {
    return decodeURIComponent(segment)
  } catch {
    return segment
  }
}

function stableAuthorityParamsKey(params: object) {
  const entries = Object.entries(params as Record<string, unknown>)
    .filter(([, value]) => value !== undefined && value !== null && value !== "" && value !== "all")
    .sort(([left], [right]) => left.localeCompare(right))

  if (entries.length === 0) return "default"
  return entries
    .map(([key, value]) => `${key}=${Array.isArray(value) ? value.join(",") : String(value)}`)
    .join("&")
}
