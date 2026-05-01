"use client"

import { Search, ArrowRight } from "lucide-react"

const SUGGESTED_SEARCHES = [
  "ORS 90.300",
  "chapter 90 habitability",
  "ORS 90.320 to 90.330",
  "definition of dwelling unit",
  "security deposit deadline",
  "landlord notice",
]

interface SearchEmptyStateProps {
  onSelectSuggestion: (suggestion: string) => void
}

export function SearchEmptyState({ onSelectSuggestion }: SearchEmptyStateProps) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center p-12 text-center">
      <div className="rounded-full bg-muted p-6 mb-6">
        <Search className="h-12 w-12 text-muted-foreground" />
      </div>
      <h2 className="text-xl font-semibold text-foreground mb-2">Search the ORS Graph</h2>
      <p className="text-muted-foreground max-w-md mb-8">
        Start with a citation, legal concept, actor, deadline, or penalty to explore the Oregon Revised Statutes.
      </p>

      <div className="w-full max-w-2xl">
        <div className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground mb-4">
          Suggested searches
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {SUGGESTED_SEARCHES.map((s) => (
            <button
              key={s}
              onClick={() => onSelectSuggestion(s)}
              className="flex items-center justify-between rounded-lg border border-border bg-card p-4 text-left hover:border-primary/50 hover:bg-primary/5 transition-all group"
            >
              <span className="text-sm font-medium">{s}</span>
              <ArrowRight className="h-4 w-4 text-muted-foreground group-hover:text-primary group-hover:translate-x-1 transition-all" />
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}
