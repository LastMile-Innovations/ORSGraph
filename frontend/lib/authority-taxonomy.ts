export const AUTHORITY_LEVELS = {
  USCONST: 100,
  ORCONST: 91,
  FederalStatute: 92,
  StateConstitution: 91,
  StateStatute: 90,
  FederalRule: 84,
  StateRule: 80,
  LocalRule: 75,
  OfficialCommentary: 65,
  CaseLaw: 60,
  Secondary: 30,
} as const

export const AUTHORITY_LADDER = [
  "Constitution",
  "Federal statutes/rules",
  "State constitution/statutes",
  "Court rules",
  "Local overlays",
  "Commentary/cases",
] as const

type AuthorityLike = {
  authority_family?: string | null
  authority_level?: number | null
  authority_tier?: string | null
  source_role?: string | null
  primary_law?: boolean | null
  official_commentary?: boolean | null
  controlling_weight?: number | null
  kind?: string | null
}

export function authorityBadges(authority: AuthorityLike) {
  const family = authority.authority_family?.toUpperCase()
  const role = authority.source_role
  const badges: string[] = []

  if (authority.primary_law || role === "primary_law") badges.push("Primary Law")
  if (family === "USCONST") badges.push("Federal Constitution")
  if (family === "ORCONST") badges.push("Oregon Constitution")
  if (authority.official_commentary || role === "official_commentary") badges.push("Official Analysis")
  if ((authority.controlling_weight ?? 0) >= 3.5) badges.push("Controlling")
  if (role === "official_commentary" || family === "CONAN") badges.push("Interprets")
  if (authority.kind?.toLowerCase().includes("external")) badges.push("External Case")

  return [...new Set(badges)]
}

export function authorityReason(authority: AuthorityLike) {
  const family = authority.authority_family?.toUpperCase()
  if (family === "USCONST") return "controlling constitutional text"
  if (authority.source_role === "official_commentary" || authority.official_commentary) {
    return "official annotation, not controlling by itself"
  }
  if (family === "ORCONST") return "controlling Oregon constitutional text"
  if (family === "CONAN") return "official interpretation, not controlling by itself"
  if (authority.source_role === "primary_law" || authority.primary_law) return "primary legal authority"
  if (authority.source_role === "case_law") return "case-law authority"
  return "supporting legal source"
}

export function formatAuthorityTier(value?: string | null) {
  if (!value) return undefined
  return value.replace(/_/g, " ")
}
