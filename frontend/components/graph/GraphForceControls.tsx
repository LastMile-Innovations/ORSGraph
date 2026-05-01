"use client"

import { Slider } from "@/components/ui/slider"

export type GraphForces = {
  repulsion: number
  cluster: number
  labelDensity: number
  depth: number
}

export function GraphForceControls({
  forces,
  onChange,
}: {
  forces: GraphForces
  onChange: (forces: GraphForces) => void
}) {
  return (
    <div className="space-y-4">
      <ForceSlider label="Repulsion" value={forces.repulsion} onChange={(repulsion) => onChange({ ...forces, repulsion })} />
      <ForceSlider label="Cluster" value={forces.cluster} onChange={(cluster) => onChange({ ...forces, cluster })} />
      <ForceSlider label="Labels" value={forces.labelDensity} onChange={(labelDensity) => onChange({ ...forces, labelDensity })} />
      <div className="grid grid-cols-2 gap-1">
        {[1, 2].map((depth) => (
          <button
            key={depth}
            type="button"
            onClick={() => onChange({ ...forces, depth })}
            className={`rounded border px-2 py-1.5 font-mono text-[11px] uppercase tracking-wide ${
              forces.depth === depth ? "border-primary bg-primary/15 text-foreground" : "border-border bg-background text-muted-foreground"
            }`}
          >
            Depth {depth}
          </button>
        ))}
      </div>
    </div>
  )
}

function ForceSlider({ label, value, onChange }: { label: string; value: number; onChange: (value: number) => void }) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <span className="font-mono text-[11px] uppercase tracking-wide text-muted-foreground">{label}</span>
        <span className="font-mono text-xs tabular-nums">{value}</span>
      </div>
      <Slider min={0} max={100} step={5} value={[value]} onValueChange={(next) => onChange(next[0] ?? value)} />
    </div>
  )
}
