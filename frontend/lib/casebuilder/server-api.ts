import "server-only"

import { headers } from "next/headers"
import { cache } from "react"
import {
  getCaseBuilderSettingsState as getCaseBuilderSettingsStateBase,
  getComplaintState as getComplaintStateBase,
  getDocumentWorkspace as getDocumentWorkspaceBase,
  getMatterSettingsState as getMatterSettingsStateBase,
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

const requestCookie = cache(async () => {
  return (await headers()).get("cookie")
})

const getMatterSummariesStateForCookie = cache((cookie: string | null) => {
  return getMatterSummariesStateBase(requestOptionsForCookie(cookie))
})

const getMatterStateForCookie = cache((id: string, cookie: string | null) => {
  return getMatterStateBase(id, requestOptionsForCookie(cookie))
})

const getCaseBuilderSettingsStateForCookie = cache((cookie: string | null) => {
  return getCaseBuilderSettingsStateBase(requestOptionsForCookie(cookie))
})

const getMatterSettingsStateForCookie = cache((matterId: string, cookie: string | null) => {
  return getMatterSettingsStateBase(matterId, requestOptionsForCookie(cookie))
})

const getMatterGraphStateForCookie = cache((matterId: string, cookie: string | null) => {
  return getMatterGraphStateBase(matterId, requestOptionsForCookie(cookie))
})

const getDocumentWorkspaceForCookie = cache((matterId: string, documentId: string, cookie: string | null) => {
  return getDocumentWorkspaceBase(matterId, documentId, requestOptionsForCookie(cookie))
})

const getWorkProductsStateForCookie = cache((matterId: string, includeDocumentAst: boolean | undefined, cookie: string | null) => {
  return getWorkProductsStateBase(matterId, {
    includeDocumentAst,
    request: requestOptionsForCookie(cookie),
  })
})

const getWorkProductStateForCookie = cache((
  matterId: string,
  workProductId: string | undefined,
  includeDocumentAst: boolean | undefined,
  cookie: string | null,
) => {
  return getWorkProductStateBase(matterId, workProductId, {
    includeDocumentAst,
    request: requestOptionsForCookie(cookie),
  })
})

const getComplaintStateForCookie = cache((matterId: string, complaintId: string | undefined, cookie: string | null) => {
  return getComplaintStateBase(matterId, complaintId, requestOptionsForCookie(cookie))
})

export async function caseBuilderRequestOptions(): Promise<CaseBuilderRequestOptions> {
  return requestOptionsForCookie(await requestCookie())
}

export async function getMatterSummariesState() {
  return getMatterSummariesStateForCookie(await requestCookie())
}

export async function getMatterState(id: string) {
  return getMatterStateForCookie(id, await requestCookie())
}

export async function getCaseBuilderSettingsState() {
  return getCaseBuilderSettingsStateForCookie(await requestCookie())
}

export async function getMatterSettingsState(matterId: string) {
  return getMatterSettingsStateForCookie(matterId, await requestCookie())
}

export async function getMatterGraphState(matterId: string) {
  return getMatterGraphStateForCookie(matterId, await requestCookie())
}

export async function getDocumentWorkspace(matterId: string, documentId: string) {
  return getDocumentWorkspaceForCookie(matterId, documentId, await requestCookie())
}

export async function getWorkProductsState(
  matterId: string,
  options: GetWorkProductsOptions = {},
) {
  return getWorkProductsStateForCookie(matterId, options.includeDocumentAst, await requestCookie())
}

export async function getWorkProductState(
  matterId: string,
  workProductId?: string,
  options: GetWorkProductsOptions = {},
) {
  return getWorkProductStateForCookie(matterId, workProductId, options.includeDocumentAst, await requestCookie())
}

export async function getComplaintState(matterId: string, complaintId?: string) {
  return getComplaintStateForCookie(matterId, complaintId, await requestCookie())
}
