"use client"

import type { GraphMode } from "./types"
import { GRAPH_MODES } from "./constants"

export function GraphModeSelector({
  value,
  onChange,
}: {
  value: GraphMode
  onChange: (mode: GraphMode) => void
}) {
  return (
    <div className="flex flex-wrap gap-1">
      {GRAPH_MODES.map((mode) => (
        <button
          key={mode.value}
          type="button"
          onClick={() => onChange(mode.value)}
          className={`rounded border px-2.5 py-1.5 font-mono text-[11px] uppercase tracking-wide transition-colors ${
            value === mode.value
              ? "border-primary bg-primary text-primary-foreground"
              : "border-border bg-background text-muted-foreground hover:border-primary/50 hover:text-foreground"
          }`}
        >
          {mode.label}
        </button>
      ))}
    </div>
  )
}
