export type GraphMode =
  | "legal"
  | "citation"
  | "semantic"
  | "history"
  | "embedding_similarity"
  | "hybrid"
  | "chapter_atlas"
  | "search_result_graph"

export type GraphLayoutName = "force" | "radial" | "hierarchical" | "timeline" | "embedding_projection"

export type GraphNode = {
  id: string
  label: string
  type: string
  labels: string[]
  citation?: string | null
  title?: string | null
  chapter?: string | null
  status?: string | null
  textSnippet?: string | null
  size?: number | null
  color?: string | null
  score?: number | null
  similarityScore?: number | null
  confidence?: number | null
  sourceBacked?: boolean | null
  metrics?: {
    degree?: number | null
    inDegree?: number | null
    outDegree?: number | null
    pagerank?: number | null
    semanticCount?: number | null
    citationCount?: number | null
  } | null
  href?: string | null
}

export type GraphEdge = {
  id: string
  source: string
  target: string
  type: string
  label?: string | null
  kind: "legal" | "semantic_similarity" | "provenance" | "history" | "retrieval" | string
  weight?: number | null
  confidence?: number | null
  similarityScore?: number | null
  sourceBacked?: boolean | null
  style?: {
    dashed?: boolean
    width?: number
    color?: string
  } | null
}

export type GraphViewerResponse = {
  center?: GraphNode | null
  nodes: GraphNode[]
  edges: GraphEdge[]
  layout?: {
    name: GraphLayoutName | string
  } | null
  stats: {
    nodeCount: number
    edgeCount: number
    truncated: boolean
    warnings: string[]
  }
}

export type GraphNeighborhoodParams = {
  id?: string
  citation?: string
  depth?: number
  limit?: number
  mode?: GraphMode
  relationshipTypes?: string[]
  nodeTypes?: string[]
  includeChunks?: boolean
  includeSimilarity?: boolean
  similarityThreshold?: number
}

export type GraphFullParams = {
  limit?: number
  edgeLimit?: number
  relationshipTypes?: string[]
  nodeTypes?: string[]
  includeChunks?: boolean
  includeSimilarity?: boolean
  similarityThreshold?: number
}

export type GraphViewScope = "neighborhood" | "full"
