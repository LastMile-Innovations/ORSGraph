"use client"

import type { GraphLayoutName } from "./types"

const LAYOUTS: Array<{ value: GraphLayoutName; label: string }> = [
  { value: "force", label: "Force" },
  { value: "radial", label: "Radial" },
  { value: "hierarchical", label: "Hierarchy" },
  { value: "timeline", label: "Timeline" },
  { value: "embedding_projection", label: "Projection" },
]

export function LayoutSelector({
  value,
  onChange,
}: {
  value: GraphLayoutName
  onChange: (value: GraphLayoutName) => void
}) {
  return (
    <div className="grid grid-cols-2 gap-1">
      {LAYOUTS.map((layout) => (
        <button
          key={layout.value}
          type="button"
          onClick={() => onChange(layout.value)}
          className={`rounded border px-2 py-1.5 text-left font-mono text-[10px] uppercase tracking-wide ${
            value === layout.value
              ? "border-primary bg-primary/15 text-foreground"
              : "border-border bg-background text-muted-foreground hover:text-foreground"
          }`}
        >
          {layout.label}
        </button>
      ))}
    </div>
  )
}
