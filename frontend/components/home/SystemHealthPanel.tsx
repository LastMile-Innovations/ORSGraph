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
    { label: "Last checked", value: formatDate(health.lastCheckedAt) },
    { label: "Last QC run", value: formatDate(corpus.lastQcRun) },
  ]

  return (
    <section className="mb-12">
      <h2 className="mb-4 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">system health and provenance</h2>
      <div className="rounded-md border border-border bg-card p-4 font-mono text-sm">
        <div className="grid grid-cols-1 gap-x-8 gap-y-3 md:grid-cols-2">
          {items.map((item, idx) => (
            <div key={idx} className="flex items-center justify-between gap-3 border-b border-border pb-2">
              <span className="text-muted-foreground">{item.label}</span>
              <span className={`font-medium ${
                item.state === "ok" ? "text-success" :
                item.state === "warning" ? "text-warning" :
                item.state === "error" ? "text-destructive" :
                "text-foreground"
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

function formatDate(value?: string) {
  if (!value) return "not reported"
  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(value))
}
