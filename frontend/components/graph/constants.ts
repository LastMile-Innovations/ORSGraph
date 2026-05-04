import type { GraphMode } from "./types"

export const GRAPH_MODES: Array<{ value: GraphMode; label: string }> = [
  { value: "legal", label: "Legal" },
  { value: "citation", label: "Citations" },
  { value: "semantic", label: "Meaning" },
  { value: "history", label: "History" },
  { value: "hybrid", label: "Hybrid" },
  { value: "embedding_similarity", label: "Similarity" },
]

export const RELATIONSHIP_FAMILIES = {
  hierarchy: ["HAS_VERSION", "VERSION_OF", "CONTAINS", "PART_OF_VERSION", "HAS_PARENT", "NEXT", "PREVIOUS"],
  citations: [
    "CITES",
    "CITES_VERSION",
    "CITES_PROVISION",
    "CITES_CHAPTER",
    "CITES_RANGE",
    "CITES_EXTERNAL",
    "MENTIONS_CITATION",
    "RESOLVES_TO",
    "RESOLVES_TO_VERSION",
    "RESOLVES_TO_PROVISION",
    "RESOLVES_TO_CHAPTER",
    "RESOLVES_TO_EXTERNAL",
    "RESOLVES_TO_RANGE_START",
    "RESOLVES_TO_RANGE_END",
  ],
  semantics: ["EXPRESSES", "SUPPORTED_BY", "IMPOSED_ON", "REQUIRES_ACTION", "SUBJECT_TO"],
  definitions: ["DEFINES", "HAS_SCOPE"],
  deadlines: ["HAS_DEADLINE"],
  penalties: ["VIOLATION_PENALIZED_BY"],
  notices: ["REQUIRES_NOTICE"],
  history: ["HAS_STATUS_EVENT", "HAS_TEMPORAL_EFFECT", "HAS_LINEAGE_EVENT", "FORMERLY", "RENUMBERED_TO", "REPEALED_BY", "MENTIONS_SESSION_LAW", "ENACTS", "AFFECTS", "AFFECTS_VERSION"],
  provenance: ["HAS_SOURCE_NOTE", "DERIVED_FROM"],
  retrieval: ["HAS_CHUNK", "CHUNK_OF", "PART_OF_CHUNK"],
  similarity: ["SIMILAR_TO"],
} as const

export const NODE_FAMILIES = {
  statutes: ["LegalTextIdentity"],
  versions: ["LegalTextVersion", "ChapterVersion"],
  provisions: ["Provision"],
  chunks: ["RetrievalChunk"],
  definitions: ["Definition", "DefinedTerm", "DefinitionScope"],
  duties: ["LegalSemanticNode", "Obligation", "Power", "Permission", "Prohibition"],
  deadlines: ["Deadline"],
  penalties: ["Penalty"],
  remedies: ["Remedy"],
  notices: ["RequiredNotice", "FormText"],
  actors: ["LegalActor", "LegalAction"],
  history: ["SourceNote", "StatusEvent", "TemporalEffect", "LineageEvent", "SessionLaw", "Amendment"],
  diagnostics: ["ParserDiagnostic"],
} as const

export const GRAPH_COLOR_TOKENS = {
  authority: "--primary",
  authorityStrong: "--primary",
  accent: "--accent",
  info: "--info",
  success: "--success",
  warning: "--warning",
  destructive: "--destructive",
  foreground: "--foreground",
  background: "--background",
  neutral: "--muted-foreground",
  document: "--muted-foreground",
  evidence: "--success",
  deadline: "--warning",
  draftInsert: "--success",
  draftDelete: "--destructive",
} as const

export type GraphColorRole = keyof typeof GRAPH_COLOR_TOKENS

export const NODE_COLOR_ROLES: Record<string, GraphColorRole> = {
  LegalTextIdentity: "authority",
  LegalTextVersion: "authorityStrong",
  ChapterVersion: "info",
  Provision: "evidence",
  RetrievalChunk: "document",
  CitationMention: "neutral",
  ExternalLegalCitation: "neutral",
  Definition: "authority",
  DefinedTerm: "info",
  DefinitionScope: "document",
  Obligation: "success",
  Power: "accent",
  Permission: "info",
  Prohibition: "destructive",
  Deadline: "deadline",
  Penalty: "destructive",
  Exception: "warning",
  Remedy: "draftDelete",
  RequiredNotice: "warning",
  FormText: "deadline",
  LegalActor: "foreground",
  LegalAction: "draftInsert",
  SourceNote: "document",
  StatusEvent: "warning",
  TemporalEffect: "warning",
  LineageEvent: "draftDelete",
  SessionLaw: "authority",
  Amendment: "info",
  ParserDiagnostic: "destructive",
}

export function graphColorVar(role: GraphColorRole) {
  return `var(${GRAPH_COLOR_TOKENS[role]})`
}

export function graphNodeColorRole(type: string): GraphColorRole {
  return NODE_COLOR_ROLES[type] ?? "neutral"
}
