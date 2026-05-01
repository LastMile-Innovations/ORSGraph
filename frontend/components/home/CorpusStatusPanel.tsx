import { CorpusStatus } from "@/lib/types"
import { MetricTile } from "./MetricTile"

export function CorpusStatusPanel({ corpus }: { corpus: CorpusStatus }) {
  const c = corpus.counts
  const citationCoverage = `${formatPercent(corpus.citations.coveragePercent)}%`
  const embeddingCoverage = `${formatPercent(corpus.embeddings.coveragePercent)}%`

  return (
    <section className="mb-12">
      <div className="mb-4 flex flex-wrap items-end justify-between gap-2">
        <h2 className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">corpus status</h2>
        <span className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          {corpus.source} / {corpus.editionYear}
        </span>
      </div>
      <div className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-4">
        <MetricTile label="Sections" value={c.sections} state="ok" href="/statutes" />
        <MetricTile label="Versions" value={c.versions} state="ok" />
        <MetricTile label="Provisions" value={c.provisions} state="ok" />
        <MetricTile label="Retrieval Chunks" value={c.retrievalChunks} state="ok" />
        <MetricTile label="Citation Coverage" value={citationCoverage} state={corpus.citations.coveragePercent >= 80 ? "ok" : "warning"} helper={`${corpus.citations.resolved.toLocaleString()} resolved`} />
        <MetricTile label="Embedding Coverage" value={embeddingCoverage} state={corpus.embeddings.status === "complete" ? "ok" : "warning"} helper={corpus.embeddings.status.replace("_", " ")} />
        <MetricTile label="Citation Mentions" value={c.citationMentions} state="ok" />
        <MetricTile 
          label="CITES Edges" 
          value={c.citesEdges} 
          state={c.citesEdges < c.citationMentions ? "warning" : "ok"} 
          helper="fast traversal citation edges"
        />
        <MetricTile label="Semantic Nodes" value={c.semanticNodes} state="ok" />
        <MetricTile label="Neo4j Nodes" value={c.neo4jNodes} state="ok" />
        <MetricTile label="Neo4j Relationships" value={c.neo4jRelationships} state="ok" />
        <MetricTile 
          label="Unresolved Citations" 
          value={corpus.citations.unresolved} 
          state={corpus.citations.unresolved > 0 ? "warning" : "ok"} 
        />
        <MetricTile 
          label="QC Status" 
          value={corpus.qcStatus.toUpperCase()} 
          state={corpus.qcStatus === "pass" ? "ok" : corpus.qcStatus === "warning" ? "warning" : "error"} 
        />
      </div>
    </section>
  )
}

function formatPercent(value: number) {
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(value)
}
