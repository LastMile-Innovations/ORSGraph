"use client"

import { Slider } from "@/components/ui/slider"

export function SimilarityThresholdSlider({
  value,
  onChange,
}: {
  value: number
  onChange: (value: number) => void
}) {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <span className="font-mono text-[11px] uppercase tracking-wide text-muted-foreground">
          Similarity
        </span>
        <span className="font-mono text-xs tabular-nums">{value.toFixed(2)}</span>
      </div>
      <Slider
        min={0.5}
        max={0.95}
        step={0.01}
        value={[value]}
        onValueChange={(next) => onChange(next[0] ?? value)}
      />
    </div>
  )
}
