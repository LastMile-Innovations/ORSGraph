import { CorpusStatus } from "@/lib/types"
import { MetricTile } from "./MetricTile"

export function CorpusStatusPanel({ corpus }: { corpus: CorpusStatus }) {
  const c = corpus.counts

  return (
    <section className="mb-16">
      <h2 className="text-xl font-semibold text-zinc-100 mb-6">Live Corpus Status</h2>
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
        <MetricTile label="Sections" value={c.sections} state="ok" />
        <MetricTile label="Versions" value={c.versions} state="ok" />
        <MetricTile label="Provisions" value={c.provisions} state="ok" />
        <MetricTile label="Retrieval Chunks" value={c.retrievalChunks} state="ok" />
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
        <MetricTile label="Resolved Citations" value={corpus.citations.resolved} state="ok" />
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
