import "server-only"

import { cacheLife, cacheTag } from "next/cache"
import {
  getHomePageState,
  getProvisionInspectorDataState,
  getSourceDetailState,
  getStatuteIndexState,
  getStatutePageDataState,
  searchWithParamsState,
  type SearchParams,
  type SourceDetailResult,
  type StatuteIndexParams,
} from "./api"
import type { DataState } from "./data-state"
import { authorityCacheTags } from "./authority-hotset.mjs"

const AUTHORITY_RELEASE_ID = releaseIdFromHotsetBaseUrl(process.env.ORS_AUTHORITY_HOTSET_BASE_URL || "") || "release:unversioned"

export async function getCachedHomePageState() {
  "use cache"
  tagAuthorityRead("home")
  cacheLife("minutes")
  return getHomePageState()
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
  cacheLife("hours")
  return getSourceDetailState(sourceId)
}

export async function getCachedProvisionInspectorDataState(provisionId: string) {
  "use cache"
  tagAuthorityRead("provision", provisionId)
  cacheLife("hours")
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
