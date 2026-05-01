export const CASEBUILDER_ROOT = "/casebuilder"
export const CASEBUILDER_MATTERS_ROOT = `${CASEBUILDER_ROOT}/matters`

export function casebuilderHomeHref() {
  return CASEBUILDER_ROOT
}

export function newMatterHref(intent?: "fight" | "build" | string) {
  return intent ? `${CASEBUILDER_ROOT}/new?intent=${encodeURIComponent(intent)}` : `${CASEBUILDER_ROOT}/new`
}

export function matterHref(matterId: string, section?: string) {
  const base = `${CASEBUILDER_MATTERS_ROOT}/${encodeMatterSlug(matterId)}`
  return section ? `${base}/${section.replace(/^\/+/, "")}` : base
}

export function matterDocumentHref(matterId: string, documentId: string, hash?: string) {
  return withHash(`${matterHref(matterId, "documents")}/${encodeMatterId(documentId)}`, hash)
}

export function matterDraftHref(matterId: string, draftId?: string) {
  const base = matterHref(matterId, "drafts")
  return draftId ? `${base}/${encodeMatterId(draftId)}` : base
}

export type WorkProductWorkspaceSection = "editor" | "qc" | "preview" | "export" | "history"

export function matterWorkProductsHref(matterId: string) {
  return matterHref(matterId, "work-products")
}

export function newWorkProductHref(matterId: string, productType?: string) {
  const href = `${matterWorkProductsHref(matterId)}/new`
  return productType ? `${href}?type=${encodeURIComponent(productType)}` : href
}

export function matterWorkProductHref(
  matterId: string,
  workProductId: string,
  section?: WorkProductWorkspaceSection,
  target?: { type?: string; id?: string },
) {
  const base = `${matterWorkProductsHref(matterId)}/${encodeMatterId(workProductId)}`
  const href = section ? `${base}/${section}` : base
  const params = new URLSearchParams()
  if (target?.type) params.set("targetType", target.type)
  const query = params.toString()
  return withHash(query ? `${href}?${query}` : href, target?.id)
}

export type ComplaintWorkspaceSection =
  | "editor"
  | "outline"
  | "claims"
  | "evidence"
  | "qc"
  | "preview"
  | "export"
  | "history"

export function matterComplaintHref(
  matterId: string,
  section?: ComplaintWorkspaceSection,
  target?: { type?: string; id?: string; returnTo?: string },
) {
  const base = matterHref(matterId, "complaint")
  const href = section ? `${base}/${section}` : base
  const params = new URLSearchParams()
  if (target?.type) params.set("targetType", target.type)
  if (target?.returnTo) params.set("returnTo", target.returnTo)
  const query = params.toString()
  return withHash(query ? `${href}?${query}` : href, target?.id)
}

export function matterFactsHref(matterId: string, factId?: string) {
  return withHash(matterHref(matterId, "facts"), factId)
}

export function matterClaimsHref(matterId: string, claimId?: string) {
  return withHash(matterHref(matterId, "claims"), claimId)
}

export function encodeMatterId(value: string) {
  return encodeURIComponent(value)
}

export function encodeMatterSlug(matterId: string) {
  const decoded = safeDecode(matterId)
  const slug = decoded.startsWith("matter:") ? decoded.slice("matter:".length) : decoded
  return encodeURIComponent(slug)
}

export function decodeMatterRouteId(routeId: string) {
  const decoded = safeDecode(routeId)
  return decoded.startsWith("matter:") ? decoded : `matter:${decoded}`
}

function safeDecode(value: string) {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
}

function withHash(href: string, hash?: string) {
  return hash ? `${href}#${encodeURIComponent(hash)}` : href
}
