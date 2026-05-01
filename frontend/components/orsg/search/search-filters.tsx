"use client"

import { Checkbox } from "@/components/ui/checkbox"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { cn } from "@/lib/utils"
import { Filter, Hash, RotateCcw } from "lucide-react"
import type { ReactNode } from "react"

export const RESULT_TYPES = [
  { id: "all", label: "All" },
  { id: "statute", label: "Statutes" },
  { id: "provision", label: "Provisions" },
  { id: "definition", label: "Definitions" },
  { id: "obligation", label: "Obligations" },
  { id: "exception", label: "Exceptions" },
  { id: "deadline", label: "Deadlines" },
  { id: "penalty", label: "Penalties" },
  { id: "remedy", label: "Remedies" },
  { id: "requirednotice", label: "Notices" },
  { id: "taxrule", label: "Tax Rules" },
  { id: "moneyamount", label: "Money Amounts" },
  { id: "ratelimit", label: "Rate Limits" },
  { id: "legalactor", label: "Actors" },
  { id: "legalaction", label: "Actions" },
  { id: "sourcenote", label: "Source Notes" },
  { id: "temporaleffect", label: "Temporal Effects" },
  { id: "sessionlaw", label: "Session Laws" },
  { id: "amendment", label: "Amendments" },
  { id: "chunk", label: "Chunks" },
] as const

export const SEMANTIC_FILTERS = [
  { id: "all", label: "Any semantic signal" },
  { id: "Definition", label: "Definitions" },
  { id: "Obligation", label: "Obligations" },
  { id: "Deadline", label: "Deadlines" },
  { id: "Penalty", label: "Penalties" },
  { id: "RequiredNotice", label: "Notices" },
  { id: "Exception", label: "Exceptions" },
  { id: "Remedy", label: "Remedies" },
  { id: "TemporalEffect", label: "Currentness" },
  { id: "TaxRule", label: "Tax rules" },
] as const

export type SearchFiltersState = {
  chapter: string
  status: string
  semantic_type: string
  current_only: boolean
  source_backed: boolean
  has_citations: boolean
  has_deadlines: boolean
  has_penalties: boolean
  needs_review: boolean
}

export const DEFAULT_FILTERS: SearchFiltersState = {
  chapter: "",
  status: "all",
  semantic_type: "all",
  current_only: false,
  source_backed: false,
  has_citations: false,
  has_deadlines: false,
  has_penalties: false,
  needs_review: false,
}

interface SearchFiltersProps {
  currentType: string
  onTypeChange: (type: string) => void
  filters: SearchFiltersState
  onFiltersChange: (filters: SearchFiltersState) => void
  counts?: Record<string, number>
  statusCounts?: Record<string, number>
  semanticCounts?: Record<string, number>
  className?: string
}

export function SearchFilters({
  currentType,
  onTypeChange,
  filters,
  onFiltersChange,
  counts,
  statusCounts,
  semanticCounts,
  className,
}: SearchFiltersProps) {
  const setFilter = <K extends keyof SearchFiltersState>(key: K, value: SearchFiltersState[K]) => {
    onFiltersChange({ ...filters, [key]: value })
  }

  const clearFilters = () => {
    onTypeChange("all")
    onFiltersChange(DEFAULT_FILTERS)
  }

  return (
    <aside className={cn("hidden w-64 flex-none flex-col border-r border-border bg-card lg:flex", className)}>
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <div className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
          <Filter className="h-3 w-3" />
          filters
        </div>
        <button
          onClick={clearFilters}
          className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
          title="Reset filters"
        >
          <RotateCcw className="h-3.5 w-3.5" />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-2 scrollbar-thin">
        <Section title="Candidate type">
          <div className="space-y-0.5">
            {RESULT_TYPES.map((t) => (
              <button
                key={t.id}
                onClick={() => onTypeChange(t.id)}
                className={cn(
                  "flex w-full items-center justify-between rounded px-2 py-1.5 text-xs transition-colors",
                  currentType === t.id
                    ? "bg-primary/10 text-primary"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                <span>{t.label}</span>
                {counts && <span className="font-mono tabular-nums">{counts[t.id] || 0}</span>}
              </button>
            ))}
          </div>
        </Section>

        <Section title="Authority">
          <label className="block">
            <span className="mb-1 flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
              <Hash className="h-3 w-3" />
              chapter
            </span>
            <Input
              value={filters.chapter}
              onChange={(event) =>
                setFilter("chapter", event.target.value.replace(/[^0-9A-Za-z]/g, "").toUpperCase())
              }
              placeholder="90 or 419B"
              maxLength={4}
              className="h-8 font-mono text-xs"
            />
          </label>

          <Select value={filters.status} onValueChange={(value) => setFilter("status", value)}>
            <SelectTrigger className="mt-2 h-8 w-full text-xs">
              <SelectValue placeholder="Status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">Any status</SelectItem>
              <SelectItem value="active">Active ({statusCounts?.active || 0})</SelectItem>
              <SelectItem value="repealed">Repealed ({statusCounts?.repealed || 0})</SelectItem>
              <SelectItem value="renumbered">Renumbered ({statusCounts?.renumbered || 0})</SelectItem>
              <SelectItem value="stale">Stale ({statusCounts?.stale || 0})</SelectItem>
            </SelectContent>
          </Select>
        </Section>

        <Section title="Graph meaning">
          <Select
            value={filters.semantic_type}
            onValueChange={(value) => setFilter("semantic_type", value)}
          >
            <SelectTrigger className="h-8 w-full text-xs">
              <SelectValue placeholder="Semantic signal" />
            </SelectTrigger>
            <SelectContent>
              {SEMANTIC_FILTERS.map((filter) => (
                <SelectItem key={filter.id} value={filter.id}>
                  {filter.label}
                  {filter.id !== "all" && semanticCounts?.[filter.id] !== undefined
                    ? ` (${semanticCounts[filter.id]})`
                    : ""}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Section>

        <Section title="Required signals">
          <ToggleRow
            label="Current edition"
            checked={filters.current_only}
            onCheckedChange={(checked) => setFilter("current_only", checked)}
          />
          <ToggleRow
            label="Source-backed"
            checked={filters.source_backed}
            onCheckedChange={(checked) => setFilter("source_backed", checked)}
          />
          <ToggleRow
            label="Has citations"
            checked={filters.has_citations}
            onCheckedChange={(checked) => setFilter("has_citations", checked)}
          />
          <ToggleRow
            label="Has deadlines"
            checked={filters.has_deadlines}
            onCheckedChange={(checked) => setFilter("has_deadlines", checked)}
          />
          <ToggleRow
            label="Has penalties"
            checked={filters.has_penalties}
            onCheckedChange={(checked) => setFilter("has_penalties", checked)}
          />
          <ToggleRow
            label="Needs review"
            checked={filters.needs_review}
            onCheckedChange={(checked) => setFilter("needs_review", checked)}
          />
        </Section>
      </div>
    </aside>
  )
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mb-4">
      <h2 className="mb-1.5 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        {title}
      </h2>
      {children}
    </section>
  )
}

function ToggleRow({
  label,
  checked,
  onCheckedChange,
}: {
  label: string
  checked: boolean
  onCheckedChange: (checked: boolean) => void
}) {
  return (
    <label className="flex cursor-pointer items-center gap-2 rounded px-1.5 py-1 text-xs text-muted-foreground hover:bg-muted hover:text-foreground">
      <Checkbox
        checked={checked}
        onCheckedChange={(value) => onCheckedChange(value === true)}
      />
      <span>{label}</span>
    </label>
  )
}
