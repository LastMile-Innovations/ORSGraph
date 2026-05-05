import { GraphViewer } from "@/components/graph/GraphViewer"
import type { GraphMode } from "@/components/graph/types"

type GraphPageSearchParams = {
  focus?: string | string[]
  mode?: string | string[]
}

type GraphPageProps = Omit<PageProps<"/graph">, "searchParams"> & {
  searchParams: Promise<GraphPageSearchParams>
}

const GRAPH_MODES: GraphMode[] = [
  "legal",
  "citation",
  "semantic",
  "history",
  "embedding_similarity",
  "hybrid",
  "chapter_atlas",
  "search_result_graph",
]

export default async function GraphPage({
  searchParams,
}: GraphPageProps) {
  const params = await searchParams
  const initialFocus = firstValue(params.focus)
  const initialMode = parseGraphMode(firstValue(params.mode))

  return <GraphViewer initialFocus={initialFocus} initialMode={initialMode} />
}

function firstValue(value: string | string[] | undefined) {
  if (Array.isArray(value)) return value[0]
  return value
}

function parseGraphMode(value: string | undefined): GraphMode | undefined {
  return GRAPH_MODES.includes(value as GraphMode) ? value as GraphMode : undefined
}
