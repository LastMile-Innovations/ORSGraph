import { graphColorVar, graphNodeColorRole, type GraphColorRole } from "./constants"

const EDGE_LEGEND = [
  { label: "CITES", colorRole: "authority", dashed: false },
  { label: "EXPRESSES", colorRole: "evidence", dashed: false },
  { label: "DEFINES", colorRole: "info", dashed: false },
  { label: "HISTORY", colorRole: "warning", dashed: false },
  { label: "SIMILAR_TO", colorRole: "accent", dashed: true },
] satisfies Array<{
  label: string
  colorRole: GraphColorRole
  dashed: boolean
}>

export function GraphLegend({ compact = false }: { compact?: boolean }) {
  const nodes = ["LegalTextIdentity", "Provision", "Obligation", "Deadline", "Penalty", "Definition", "SourceNote"]
  return (
    <div className="rounded border border-border bg-card/95 p-3 shadow-sm backdrop-blur">
      <div className="mb-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground">Legend</div>
      <div className={compact ? "grid grid-cols-2 gap-2" : "space-y-3"}>
        <div className="space-y-1">
          {nodes.map((node) => (
            <div key={node} className="flex items-center gap-2 text-xs">
              <span className="h-2.5 w-2.5 rounded-full border border-background" style={{ backgroundColor: graphColorVar(graphNodeColorRole(node)) }} />
              <span className="truncate font-mono text-muted-foreground">{node}</span>
            </div>
          ))}
        </div>
        <div className="space-y-1">
          {EDGE_LEGEND.map((edge) => (
            <div key={edge.label} className="flex items-center gap-2 text-xs">
              <span
                className="h-0 w-5 border-t"
                style={{ borderColor: graphColorVar(edge.colorRole), borderTopStyle: edge.dashed ? "dashed" : "solid" }}
              />
              <span className="font-mono text-muted-foreground">{edge.label}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
