"use client"

import { Checkbox } from "@/components/ui/checkbox"
import { NODE_FAMILIES, RELATIONSHIP_FAMILIES } from "./constants"

export type GraphFilters = {
  relationshipFamilies: Set<string>
  nodeFamilies: Set<string>
  includeChunks: boolean
}

export function GraphFilterPanel({
  filters,
  onChange,
}: {
  filters: GraphFilters
  onChange: (filters: GraphFilters) => void
}) {
  function toggleFamily(kind: "relationshipFamilies" | "nodeFamilies", value: string) {
    const next = new Set(filters[kind])
    if (next.has(value)) next.delete(value)
    else next.add(value)
    onChange({ ...filters, [kind]: next })
  }

  return (
    <div className="space-y-5">
      <FilterSection
        title="Relationship families"
        values={Object.keys(RELATIONSHIP_FAMILIES)}
        enabled={filters.relationshipFamilies}
        onToggle={(value) => toggleFamily("relationshipFamilies", value)}
      />
      <FilterSection
        title="Node families"
        values={Object.keys(NODE_FAMILIES)}
        enabled={filters.nodeFamilies}
        onToggle={(value) => toggleFamily("nodeFamilies", value)}
      />
      <label className="flex items-center gap-2 rounded border border-border bg-background p-2 text-xs">
        <Checkbox
          checked={filters.includeChunks}
          onCheckedChange={(checked) => onChange({ ...filters, includeChunks: checked === true })}
        />
        <span className="font-mono uppercase tracking-wide text-muted-foreground">Show chunks</span>
      </label>
    </div>
  )
}

function FilterSection({
  title,
  values,
  enabled,
  onToggle,
}: {
  title: string
  values: string[]
  enabled: Set<string>
  onToggle: (value: string) => void
}) {
  return (
    <section>
      <div className="mb-2 font-mono text-[11px] uppercase tracking-wide text-muted-foreground">{title}</div>
      <div className="grid grid-cols-1 gap-1">
        {values.map((value) => (
          <label
            key={value}
            className="flex items-center gap-2 rounded border border-border bg-background/70 px-2 py-1.5 text-xs"
          >
            <Checkbox checked={enabled.has(value)} onCheckedChange={() => onToggle(value)} />
            <span className="font-mono uppercase tracking-wide text-foreground">{value.replace("_", " ")}</span>
          </label>
        ))}
      </div>
    </section>
  )
}
