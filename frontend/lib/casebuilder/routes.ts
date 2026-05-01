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
