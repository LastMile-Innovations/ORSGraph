import "server-only"

import { cacheLife, cacheTag } from "next/cache"
import {
  getHomePageState,
  getStatuteIndexState,
  getStatutePageDataState,
  searchWithParamsState,
  type SearchParams,
  type StatuteIndexParams,
} from "./api"
import { authorityCacheTags } from "./authority-hotset.mjs"

const AUTHORITY_RELEASE_ID = releaseIdFromHotsetBaseUrl(process.env.ORS_AUTHORITY_HOTSET_BASE_URL || "") || "release:unversioned"

export async function getCachedHomePageState() {
  "use cache"
  tagAuthorityRead("home")
  cacheLife("hours")
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
