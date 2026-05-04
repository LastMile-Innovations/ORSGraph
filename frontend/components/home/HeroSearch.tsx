"use client"

import { useState } from "react"
import { useRouter } from "next/navigation"
import { openSearch } from "@/lib/api"
import { toSafeInternalHref } from "@/lib/navigation-safety"
import { Loader2, Search } from "lucide-react"

export function HeroSearch() {
  const [query, setQuery] = useState("")
  const [isLoading, setIsLoading] = useState(false)
  const router = useRouter()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!query.trim()) return

    setIsLoading(true)
    try {
      const isCitationLike = /^[A-Za-z]+\s*\d+/i.test(query) || /^\d+\.\d+/i.test(query) || /^chapter\s*\d+/i.test(query)
      
      if (isCitationLike) {
        const response = await openSearch(query)
        const href = toSafeInternalHref(response.href)
        if (response.matched && href) {
          router.push(href)
          return
        }
      }
      
      router.push(`/search?q=${encodeURIComponent(query)}`)
    } catch {
      router.push(`/search?q=${encodeURIComponent(query)}`)
    } finally {
      setIsLoading(false)
    }
  }

  const handleChipClick = (chip: string) => {
    setQuery(chip)
  }

  const chips = [
    "ORS 90.300",
    "landlord notice",
    "security deposit deadline",
    "civil penalty",
    "operative date",
    "Department of Revenue tax"
  ]

  return (
    <div className="mt-7 w-full max-w-3xl">
      <form onSubmit={handleSubmit} className="rounded-md border border-border bg-background p-2 shadow-sm">
        <label className="sr-only" htmlFor="home-search">
          Search ORSGraph
        </label>
        <div className="flex flex-col gap-2 sm:flex-row">
          <div className="flex min-h-11 min-w-0 flex-1 items-center gap-2 rounded border border-input bg-card px-3 focus-within:border-primary">
            <Search className="h-4 w-4 shrink-0 text-muted-foreground" />
            <input
              id="home-search"
              type="text"
              className="min-w-0 flex-1 bg-transparent py-2 text-base text-foreground outline-none placeholder:text-muted-foreground"
              placeholder="Search ORS citations, duties, deadlines, definitions..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              disabled={isLoading}
              autoComplete="off"
            />
          </div>
          <button
            type="submit"
            disabled={isLoading || !query.trim()}
            className="inline-flex min-h-11 items-center justify-center gap-2 rounded bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50 sm:w-32"
          >
            {isLoading ? <Loader2 className="h-4 w-4 animate-spin" /> : <Search className="h-4 w-4" />}
            Search
          </button>
        </div>
      </form>

      <div className="mt-3 flex flex-wrap gap-2">
        {chips.map((chip) => (
          <button
            key={chip}
            type="button"
            onClick={() => handleChipClick(chip)}
            className="rounded border border-border bg-card px-2.5 py-1.5 font-mono text-[11px] text-muted-foreground transition-colors hover:border-primary/50 hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
          >
            {chip}
          </button>
        ))}
      </div>
    </div>
  )
}
