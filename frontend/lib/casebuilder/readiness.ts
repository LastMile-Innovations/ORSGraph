import type { Matter, TimelineEvent, TimelineSuggestion } from "./types"

export interface MatterReadiness {
  supportedFacts: number
  reviewFacts: number
  pendingTimelineSuggestions: number
  authorityLinks: number
  claimElements: number
  draftArtifacts: number
  hasIndexedSources: boolean
  hasTriggerDates: boolean
  deadlineMissing: string[]
  draftingMissing: string[]
  exportMissing: string[]
  setupGaps: Array<{ title: string; body: string; href: string }>
}

export function getMatterReadiness(matter: Matter): MatterReadiness {
  const supportedFacts = matter.facts.filter((fact) => fact.status === "supported" || fact.status === "admitted").length
  const reviewFacts = matter.facts.filter((fact) => fact.status === "proposed" || fact.needs_verification || fact.confidence < 0.7).length
  const pendingTimelineSuggestions = matter.timeline_suggestions.filter((item) => item.status === "suggested" || item.status === "needs_attention").length
  const authorityLinks = matter.claims.reduce((sum, claim) => sum + (claim.authorities?.length ?? 0), 0)
  const claimElements = matter.claims.reduce((sum, claim) => sum + claim.elements.length, 0)
  const draftArtifacts = matter.work_products.length + matter.drafts.filter((draft) => draft.kind !== "complaint").length
  const hasIndexedSources = matter.documents.some((document) => document.chunks.length > 0) || matter.facts.length > 0
  const hasTriggerDates =
    matter.deadlines.length > 0 ||
    matter.timeline.some((event) => hasValidDate(event)) ||
    matter.timeline_suggestions.some((suggestion) => hasValidDate(suggestion)) ||
    matter.facts.some((fact) => isValidDateValue(fact.date))

  const court = matter.court.trim().toLowerCase()
  const deadlineMissing = [
    !matter.case_number ? "case number" : null,
    !court || court === "unassigned" ? "assigned court" : null,
    !hasTriggerDates ? "trigger dates or source events" : null,
    matter.documents.length === 0 && matter.facts.length === 0 ? "source documents or reviewed facts" : null,
  ].filter(Boolean) as string[]

  const draftingMissing = [
    matter.parties.length === 0 ? "parties" : null,
    matter.claims.length === 0 ? "claims or defenses" : null,
    supportedFacts === 0 ? "reviewed/supported facts" : null,
    authorityLinks === 0 ? "linked authorities" : null,
  ].filter(Boolean) as string[]

  const exportMissing = [
    draftArtifacts === 0 ? "drafts or work products" : null,
    supportedFacts === 0 ? "reviewed facts" : null,
  ].filter(Boolean) as string[]

  const setupGaps = [
    reviewFacts > 0
      ? {
          title: `Review ${reviewFacts} extracted fact${reviewFacts === 1 ? "" : "s"}`,
          body: "Approve, edit, or reject extracted facts before using them for claims, deadlines, QC, or drafting.",
          href: "facts",
        }
      : null,
    pendingTimelineSuggestions > 0
      ? {
          title: `Review ${pendingTimelineSuggestions} timeline suggestion${pendingTimelineSuggestions === 1 ? "" : "s"}`,
          body: "Promote accurate suggestions to timeline events and repair invalid or uncertain dates.",
          href: "timeline",
        }
      : null,
    matter.claims.length === 0
      ? {
          title: "Create claims or defenses",
          body: "The evidence matrix, authorities, drafts, and QC checks need legal theories before they can evaluate coverage.",
          href: "claims",
        }
      : null,
    matter.claims.length > 0 && authorityLinks === 0
      ? {
          title: "Link authorities to legal theories",
          body: "Claims exist, but none have source-backed authority attached yet.",
          href: "authorities",
        }
      : null,
    matter.deadlines.length === 0
      ? {
          title: "Add or compute deadlines after preflight",
          body: "Deadline surfaces should stay in setup mode until court, case, and trigger-date inputs are visible.",
          href: "deadlines",
        }
      : null,
  ].filter(Boolean) as MatterReadiness["setupGaps"]

  return {
    supportedFacts,
    reviewFacts,
    pendingTimelineSuggestions,
    authorityLinks,
    claimElements,
    draftArtifacts,
    hasIndexedSources,
    hasTriggerDates,
    deadlineMissing,
    draftingMissing,
    exportMissing,
    setupGaps,
  }
}

export function isValidDateValue(value?: string | null) {
  if (!value?.trim()) return false
  const date = new Date(value)
  return Number.isFinite(date.getTime())
}

function hasValidDate(item: TimelineEvent | TimelineSuggestion) {
  return isValidDateValue(item.date)
}
