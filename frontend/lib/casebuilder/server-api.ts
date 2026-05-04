import "server-only"

import { headers } from "next/headers"
import { cache } from "react"
import {
  getComplaintState as getComplaintStateBase,
  getDocumentWorkspace as getDocumentWorkspaceBase,
  getMatterGraphState as getMatterGraphStateBase,
  getMatterState as getMatterStateBase,
  getMatterSummariesState as getMatterSummariesStateBase,
  getWorkProductState as getWorkProductStateBase,
  getWorkProductsState as getWorkProductsStateBase,
  type CaseBuilderRequestOptions,
  type GetWorkProductsOptions,
} from "./api"

const requestOptionsForCookie = cache((cookie: string | null): CaseBuilderRequestOptions => {
  return cookie ? { headers: { cookie } } : {}
})

export async function caseBuilderRequestOptions(): Promise<CaseBuilderRequestOptions> {
  const cookie = (await headers()).get("cookie")
  return requestOptionsForCookie(cookie)
}

export async function getMatterSummariesState() {
  return getMatterSummariesStateBase(await caseBuilderRequestOptions())
}

export async function getMatterState(id: string) {
  return getMatterStateBase(id, await caseBuilderRequestOptions())
}

export async function getMatterGraphState(matterId: string) {
  return getMatterGraphStateBase(matterId, await caseBuilderRequestOptions())
}

export async function getDocumentWorkspace(matterId: string, documentId: string) {
  return getDocumentWorkspaceBase(matterId, documentId, await caseBuilderRequestOptions())
}

export async function getWorkProductsState(
  matterId: string,
  options: GetWorkProductsOptions = {},
) {
  return getWorkProductsStateBase(matterId, { ...options, request: await caseBuilderRequestOptions() })
}

export async function getWorkProductState(
  matterId: string,
  workProductId?: string,
  options: GetWorkProductsOptions = {},
) {
  return getWorkProductStateBase(matterId, workProductId, { ...options, request: await caseBuilderRequestOptions() })
}

export async function getComplaintState(matterId: string, complaintId?: string) {
  return getComplaintStateBase(matterId, complaintId, await caseBuilderRequestOptions())
}
