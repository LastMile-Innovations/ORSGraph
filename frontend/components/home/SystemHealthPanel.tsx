import { SystemHealth, CorpusStatus } from "@/lib/types"

export function SystemHealthPanel({ health, corpus }: { health: SystemHealth; corpus: CorpusStatus }) {
  const items = [
    { label: "Data source", value: `${corpus.source}, ${corpus.editionYear} edition` },
    { label: "API status", value: health.api, state: health.api === "connected" ? "ok" : "warning" },
    { label: "Neo4j status", value: health.neo4j, state: health.neo4j === "connected" ? "ok" : "error" },
    { label: "QC status", value: health.qc, state: health.qc === "pass" ? "ok" : "warning" },
    { label: "Graph materialization", value: health.graphMaterialization },
    { label: "Embeddings", value: health.embeddings },
    { label: "Rerank status", value: health.rerank },
  ]

  return (
    <section className="mb-16">
      <h2 className="text-xl font-semibold text-zinc-100 mb-6">System Health & Provenance</h2>
      <div className="bg-zinc-950 border border-zinc-800 rounded-xl p-6 font-mono text-sm">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-4">
          {items.map((item, idx) => (
            <div key={idx} className="flex justify-between items-center border-b border-zinc-800/50 pb-2">
              <span className="text-zinc-500">{item.label}</span>
              <span className={`font-medium ${
                item.state === "ok" ? "text-emerald-400" :
                item.state === "warning" ? "text-amber-400" :
                item.state === "error" ? "text-red-400" :
                "text-zinc-300"
              }`}>
                {item.value}
              </span>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
